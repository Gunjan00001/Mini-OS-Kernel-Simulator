//! Process synchronization: race-condition demonstrations, a bounded
//! producer-consumer pipeline (real threads with Mutex + Condvar), and a
//! simulated semaphore.
//!
//! The demos here use real OS threads where actual concurrency is the point.
//! The rest of the simulator advances simulated processes by ticks instead.

use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};

/// Deterministic interleaving demo: two processes both do `counter += 1`,
/// but P2 loads before P1 stores, so P1's update is lost.
/// Returns (result, correct value).
#[allow(unused_assignments)] // the lost store IS the demonstration
pub fn race_demo_interleaved() -> (u64, u64) {
    let mut counter = 0u64;
    let p1_reg = counter; // P1: LOAD  (reads 0)
    let p2_reg = counter; // P2: LOAD  (reads 0, before P1 stores!)
    counter = p1_reg + 1; // P1: STORE (writes 1)
    counter = p2_reg + 1; // P2: STORE (writes 1 -- P1's update is lost)
    (counter, 2)
}

/// Wrapper making a raw pointer shareable across threads for the race demo.
#[derive(Clone, Copy)]
struct SendPtr(*mut u64);
// SAFETY: the demo joins every thread before freeing the allocation.
unsafe impl Send for SendPtr {}

impl SendPtr {
    /// Method call (not field access) so the closure captures the whole
    /// wrapper, keeping the unsafe `Send` impl in effect.
    unsafe fn slot(self) -> &'static mut u64 {
        unsafe { &mut *self.0 }
    }
}

/// Real-thread race demo. Runs `threads` threads each incrementing a shared
/// counter `iters` times, first WITHOUT synchronization (a genuine data race,
/// results are nondeterministic and usually short of the expected value),
/// then WITH a Mutex (always exactly the expected value).
/// Returns (racy_result, locked_result, expected).
///
/// # Safety (demo)
/// The racy half intentionally accesses shared memory through a raw pointer
/// with no synchronization: the data race is the demonstration. The counter
/// is heap-allocated and only freed after all threads join.
pub fn race_demo_threads(threads: u32, iters: u32) -> (u64, u64, u64) {
    // -- racy --
    let ptr = SendPtr(Box::into_raw(Box::new(0u64)));
    let handles: Vec<_> = (0..threads)
        .map(|_| {
            let p = ptr;
            std::thread::spawn(move || {
                for _ in 0..iters {
                    // SAFETY: ptr is valid until joined below; the race is the point.
                    unsafe { *p.slot() += 1; } // load-modify-store, not atomic -> lost updates
                }
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }
    let racy = unsafe { *ptr.0 };
    unsafe { drop(Box::from_raw(ptr.0)) };

    // -- synchronized --
    let counter = Arc::new(Mutex::new(0u64));
    let handles: Vec<_> = (0..threads)
        .map(|_| {
            let c = Arc::clone(&counter);
            std::thread::spawn(move || {
                for _ in 0..iters {
                    *c.lock().unwrap() += 1;
                }
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }
    let locked = *counter.lock().unwrap();
    let expected = u64::from(threads) * u64::from(iters);
    (racy, locked, expected)
}

/// Bounded producer-consumer with real threads: `producers` threads each
/// produce `items_per_producer` items into a capacity-bounded queue and then
/// push a sentinel (`None`); consumers drain until every sentinel has been
/// consumed (tracked with a shared atomic), which guarantees every produced
/// item is collected before the last consumer exits. Returns the consumed
/// items in consumption order.
pub fn producer_consumer(
    producers: u32,
    consumers: u32,
    items_per_producer: u32,
    capacity: usize,
) -> Vec<u64> {
    use std::sync::atomic::{AtomicUsize, Ordering};

    let queue = Arc::new((Mutex::new(VecDeque::<Option<u64>>::new()), Condvar::new()));
    let results = Arc::new(Mutex::new(Vec::<u64>::new()));
    // Decremented each time a consumer pops one producer's sentinel.
    let pending_sentinels = Arc::new(AtomicUsize::new(producers as usize));
    let mut handles = Vec::new();

    for prod in 0..producers {
        let queue = Arc::clone(&queue);
        handles.push(std::thread::spawn(move || {
            for i in 1..=items_per_producer {
                let item = u64::from(prod) * 1_000_000 + u64::from(i);
                let (q, cv) = &*queue;
                let mut q = q.lock().unwrap();
                while q.len() == capacity {
                    q = cv.wait(q).unwrap(); // bounded buffer: block while full
                }
                q.push_back(Some(item));
                cv.notify_all();
            }
            let (q, cv) = &*queue;
            let mut q = q.lock().unwrap();
            while q.len() == capacity {
                q = cv.wait(q).unwrap();
            }
            q.push_back(None); // sentinel: this producer is done
            cv.notify_all();
        }));
    }

    for _ in 0..consumers {
        let queue = Arc::clone(&queue);
        let results = Arc::clone(&results);
        let pending = Arc::clone(&pending_sentinels);
        handles.push(std::thread::spawn(move || {
            let (q, cv) = &*queue;
            loop {
                let mut q = q.lock().unwrap();
                // Block only while work may still arrive; if every producer's
                // sentinel was consumed and the queue is empty, we are done.
                while q.is_empty() && pending.load(Ordering::SeqCst) > 0 {
                    q = cv.wait(q).unwrap();
                }
                if q.is_empty() {
                    break; // all sentinels consumed, nothing left to drain
                }
                match q.pop_front().unwrap() {
                    Some(item) => {
                        results.lock().unwrap().push(item);
                        cv.notify_all();
                    }
                    None => {
                        pending.fetch_sub(1, Ordering::SeqCst);
                        cv.notify_all(); // wake peers so they can exit too
                    }
                }
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }
    let items = results.lock().unwrap().clone();
    items
}

/// Simulated counting semaphore (P/V). `wait` on a zero count reports that
/// the caller would block instead of actually blocking -- simulated processes
/// must not stall real threads.
#[derive(Debug)]
pub struct SimSemaphore {
    count: i64,
}

impl SimSemaphore {
    pub fn new(initial: i64) -> Self {
        SimSemaphore { count: initial }
    }

    pub fn count(&self) -> i64 {
        self.count
    }

    pub fn wait(&mut self) -> Result<(), &'static str> {
        if self.count > 0 {
            self.count -= 1;
            Ok(())
        } else {
            Err("BLOCKED (semaphore count = 0)")
        }
    }

    pub fn signal(&mut self) {
        self.count += 1;
    }
}

/// Simulated mutex + semaphore walkthrough: P1 enters the critical section,
/// P2's wait fails and it blocks, P1 signals, P2 enters. Deterministic.
/// Returns the step log.
pub fn semaphore_demo() -> Vec<String> {
    let mut steps = Vec::new();
    let mut sem = SimSemaphore::new(1); // binary semaphore guarding the CS

    steps.push("semaphore initialized to 1 (binary, guards the critical section)".to_string());
    match sem.wait() {
        Ok(()) => steps.push("P1: wait() -> entered CS (count 1 -> 0)".to_string()),
        Err(e) => steps.push(format!("P1: wait() -> {e}")),
    }
    match sem.wait() {
        Ok(()) => steps.push("P2: wait() -> entered CS".to_string()),
        Err(e) => steps.push(format!("P2: wait() -> {e} (mutual exclusion holds)")),
    }
    sem.signal();
    steps.push("P1: signal() -> left CS (count 0 -> 1)".to_string());
    match sem.wait() {
        Ok(()) => steps.push("P2: wait() -> entered CS (count 1 -> 0)".to_string()),
        Err(e) => steps.push(format!("P2: wait() -> {e}")),
    }
    sem.signal();
    steps.push("P2: signal() -> left CS (count 1)".to_string());
    assert_eq!(sem.count(), 1);
    steps
}
