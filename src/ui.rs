//! Terminal interface: main menu, per-module menus, dashboard.
//! Plain CLI (no Ratatui) -- same content, no extra dependency.

use std::io::{self, Write};

use crate::deadlock;
use crate::kernel::{Kernel, KernelError};
use crate::memory::FitStrategy;
use crate::paging::Replacement;
use crate::process::State;
use crate::scheduler::{self, Algorithm};
use crate::sync;

const BANNER: &str = "\
========================================
          MINI OS KERNEL SIMULATOR
========================================";

const MAIN_MENU: &str = "
1. Process Management
2. CPU Scheduling
3. Memory Management
4. Process Synchronization
5. Inter-Process Communication
6. Deadlock Management
7. System Call Interface
8. Kernel Dashboard
9. Exit";

// ---------- input helpers ----------

pub fn ask(prompt: &str) -> String {
    print!("{} ", prompt);
    io::stdout().flush().unwrap();
    let mut line = String::new();
    match io::stdin().read_line(&mut line) {
        // EOF (piped input exhausted / Ctrl+Z): no more commands can arrive,
        // so exit instead of retrying forever on an empty string.
        Ok(0) => {
            println!("\nInput closed (EOF). Exiting simulator.");
            std::process::exit(0);
        }
        Ok(_) => line.trim().to_string(),
        Err(e) => {
            eprintln!("\nFailed to read input: {e}");
            std::process::exit(1);
        }
    }
}

pub fn ask_u32(prompt: &str) -> u32 {
    loop {
        match ask(prompt).parse() {
            Ok(v) => return v,
            Err(_) => println!("Please enter a whole number."),
        }
    }
}

fn ask_u32_or(prompt: &str, default: u32) -> u32 {
    let s = ask(&format!("{prompt} [{}]:", default));
    s.parse().unwrap_or(default)
}

fn ask_vec_u32(prompt: &str, n: usize) -> Vec<u32> {
    loop {
        let line = ask(&format!("{prompt} ({} values, space separated):", n));
        let vals: Vec<u32> = line.split_whitespace().filter_map(|w| w.parse().ok()).collect();
        if vals.len() == n {
            return vals;
        }
        println!("Need exactly {} values.", n);
    }
}

/// Print an error and return None on failure.
fn report<T>(r: Result<T, KernelError>) -> Option<T> {
    match r {
        Ok(v) => Some(v),
        Err(e) => {
            println!("ERROR: {}", e);
            None
        }
    }
}

// ---------- main menu ----------

pub fn main_menu(kernel: &mut Kernel) {
    kernel.echo_events = true;
    println!("{}", BANNER);
    loop {
        println!("{}", MAIN_MENU);
        match ask("Select an option:").as_str() {
            "1" => process_menu(kernel),
            "2" => schedule_menu(kernel),
            "3" => memory_menu(kernel),
            "4" => sync_menu(),
            "5" => ipc_menu(kernel),
            "6" => deadlock_menu(kernel),
            "7" => syscall_menu(kernel),
            "8" => println!("\n{}\n", kernel.dashboard()),
            "9" | "q" | "Q" => break,
            other => println!("Unknown option: {}", other),
        }
    }
    println!("Kernel simulator shutting down. Final tick: {}", kernel.tick);
}

// ---------- 1. process management ----------

fn process_menu(k: &mut Kernel) {
    loop {
        println!("
--- Process Management ---
1. Create process (fork)
2. Terminate process (exit)
3. Block process (wait)
4. Wake process
5. Dispatch process to CPU
6. List all processes
7. Show process details
8. Show event log
9. Back");
        match ask("Select an option:").as_str() {
            "1" => {
                let name = ask("Process name:");
                let arrival = ask_u32("Arrival time:");
                let burst = ask_u32("Burst time:");
                let priority = ask_u32_or("Priority (lower = higher)", 5);
                let mem = ask_u32("Memory requirement (0 = none):");
                if let Some(pid) = report(k.fork_process(&name, arrival, burst, priority, mem)) {
                    println!("Created process P{} ({})", pid, name);
                }
            }
            "2" => {
                let pid = ask_u32("PID to terminate:");
                if report(k.exit_process(pid)).is_some() {
                    println!("P{} terminated.", pid);
                }
            }
            "3" => {
                let pid = ask_u32("PID to block:");
                report(k.wait_process(pid));
            }
            "4" => {
                let pid = ask_u32("PID to wake:");
                report(k.wake_process(pid));
            }
            "5" => {
                let pid = ask_u32("PID to dispatch:");
                report(k.dispatch_process(pid));
            }
            "6" => list_processes(k),
            "7" => {
                let pid = ask_u32("PID to inspect:");
                match k.processes.get(&pid) {
                    Some(p) => println!(
                        "PID {} | {} | state {} | arrival {} | burst {} | remaining {} | priority {} | mem {} | cpu {} | wait {} | turnaround {}",
                        p.pid, p.name, p.state, p.arrival, p.burst, p.remaining, p.priority,
                        p.mem_req, p.cpu_time, p.wait_time, p.turnaround
                    ),
                    None => println!("ERROR: no such process (pid {pid})"),
                }
            }
            "8" => {
                println!("--- EVENT LOG ---");
                k.print_events();
            }
            "9" => break,
            other => println!("Unknown option: {}", other),
        }
    }
}

fn list_processes(k: &Kernel) {
    if k.processes.is_empty() {
        println!("No processes. Create one first (option 1).");
        return;
    }
    println!(
        "{:<4} {:<10} {:<10} {:>4} {:>4} {:>4} {:>4} {:>5}",
        "PID", "NAME", "STATE", "ARR", "BURST", "REM", "PRIO", "MEM"
    );
    for p in k.processes.values() {
        println!(
            "{:<4} {:<10} {:<10} {:>4} {:>4} {:>4} {:>4} {:>5}",
            p.pid, p.name, p.state, p.arrival, p.burst, p.remaining, p.priority, p.mem_req
        );
    }
}

// ---------- 2. cpu scheduling ----------

fn schedule_menu(k: &mut Kernel) {
    loop {
        println!("
--- CPU Scheduling --- (current: {})
1. FCFS
2. SJF (non-preemptive)
3. SRTF (preemptive)
4. Round Robin
5. Priority
6. Compare all algorithms
7. Back", k.scheduler_algo.label());
        let quantum = 2;
        match ask("Select an option:").as_str() {
            "1" => run_and_show(k, Algorithm::Fcfs),
            "2" => run_and_show(k, Algorithm::Sjf),
            "3" => run_and_show(k, Algorithm::Srtf),
            "4" => {
                let q = ask_u32_or("Time quantum", quantum);
                run_and_show(k, Algorithm::RoundRobin { quantum: q });
            }
            "5" => run_and_show(k, Algorithm::Priority),
            "6" => compare_all(k, ask_u32_or("Time quantum for RR", quantum)),
            "7" => break,
            other => println!("Unknown option: {}", other),
        }
    }
}

fn run_and_show(k: &mut Kernel, algo: Algorithm) {
    if report(k.run_schedule(algo)).is_none() {
        return;
    }
    let result = k.last_schedule.as_ref().unwrap();
    let procs: Vec<_> = k.processes.values().cloned().collect();
    println!("\n=== {} ===", algo.label());
    println!("{}", scheduler::gantt(result, &procs));
    show_metrics(result);
    show_stats(result, &procs);
}

fn show_metrics(result: &scheduler::ScheduleResult) {
    let m = &result.metrics;
    println!(
        "avg wait {:.2} | avg turnaround {:.2} | avg response {:.2} | CPU util {:.1}% | throughput {:.3} proc/unit",
        m.avg_wait, m.avg_turnaround, m.avg_response, m.cpu_util_pct, m.throughput
    );
}

fn show_stats(result: &scheduler::ScheduleResult, procs: &[crate::process::Process]) {
    println!(
        "{:<4} {:>5} {:>5} {:>6} {:>6} {:>6} {:>6} {:>6}",
        "PID", "ARR", "BURST", "START", "FIN", "WAIT", "TAT", "RESP"
    );
    for s in scheduler::process_stats(result, procs) {
        println!(
            "{:<4} {:>5} {:>5} {:>6} {:>6} {:>6} {:>6} {:>6}",
            s.pid, s.arrival, s.burst, s.first_start, s.finish, s.wait, s.turnaround, s.response
        );
    }
}

fn compare_all(k: &mut Kernel, quantum: u32) {
    let procs: Vec<crate::process::Process> = k
        .processes
        .values()
        .filter(|p| p.state != State::Terminated)
        .cloned()
        .collect();
    if procs.is_empty() {
        println!("No processes to schedule. Create some first.");
        return;
    }
    println!("\n=== COMPARISON on {} processes ===", procs.len());
    println!(
        "{:<22} {:>9} {:>9} {:>9} {:>8} {:>10}",
        "ALGORITHM", "WAIT", "TURNAROUND", "RESPONSE", "UTIL%", "THROUGHPUT"
    );
    let mut results = Vec::new();
    for algo in Algorithm::all(quantum) {
        let r = scheduler::schedule(&procs, algo);
        let m = &r.metrics;
        println!(
            "{:<22} {:>9.2} {:>9.2} {:>9.2} {:>7.1}% {:>10.3}",
            algo.label(), m.avg_wait, m.avg_turnaround, m.avg_response, m.cpu_util_pct, m.throughput
        );
        results.push(r);
    }
    for r in &results {
        println!("\n--- {} ---", r.algo.label());
        println!("{}", scheduler::gantt(r, &procs));
    }
}

// ---------- 3. memory management ----------

fn memory_menu(k: &mut Kernel) {
    loop {
        println!("
--- Memory Management --- (strategy: {})
1. Allocate memory for process
2. Free memory of process
3. Show memory map
4. Set allocation strategy
5. Paging: attach page table to process
6. Paging: translate logical address
7. Paging: show frame table
8. Page replacement demo (FIFO / LRU / Optimal)
9. Back", k.memory.strategy.name());
        match ask("Select an option:").as_str() {
            "1" => {
                let pid = ask_u32("PID:");
                let size = ask_u32("Size (units):");
                if let Some(start) = report(k.allocate_memory(pid, size)) {
                    println!("Allocated {} units at address {}.", size, start);
                }
            }
            "2" => {
                let pid = ask_u32("PID:");
                if let Some(freed) = report(k.free_memory(pid)) {
                    println!("Freed {} units.", freed);
                }
            }
            "3" => {
                println!("{}", k.memory.display());
                println!(
                    "used {} | free {} | largest free hole {} | external fragmentation {}",
                    k.memory.used(),
                    k.memory.free_bytes(),
                    k.memory.largest_free_hole(),
                    k.memory.external_fragmentation()
                );
                println!("{}", k.paging.display());
            }
            "4" => {
                println!("1. First Fit  2. Best Fit  3. Worst Fit");
                k.memory.strategy = match ask("Strategy:").as_str() {
                    "2" => FitStrategy::Best,
                    "3" => FitStrategy::Worst,
                    _ => FitStrategy::First,
                };
                println!("Strategy set to {}.", k.memory.strategy.name());
            }
            "5" => {
                let pid = ask_u32("PID:");
                let size = ask_u32("Address space size (bytes):");
                let pages = k.paging.attach(pid, size);
                println!("P{}: page table created, {} pages (page size {}).", pid, pages, k.paging.page_size);
            }
            "6" => {
                let pid = ask_u32("PID:");
                let addr = ask_u32("Logical address:");
                let algo = pick_replacement();
                match k.paging.translate(pid, addr, algo) {
                    Ok(phys) => println!("Logical {} -> physical {} (faults {}, hits {})", addr, phys, k.paging.faults, k.paging.hits),
                    Err(e) => println!("ERROR: {}", KernelError::from(e)),
                }
            }
            "7" => println!("{}", k.paging.display()),
            "8" => {
                let frames = ask_u32_or("Number of frames", 3);
                let line = ask("Reference string (page numbers, space separated):");
                let reference: Vec<u32> = line.split_whitespace().filter_map(|w| w.parse().ok()).collect();
                if reference.is_empty() {
                    println!("Empty reference string.");
                    continue;
                }
                for algo in [Replacement::Fifo, Replacement::Lru, Replacement::Optimal] {
                    let (faults, hits) = crate::paging::simulate_replacement(&reference, frames as usize, algo);
                    println!("{:>8}: {} faults, {} hits", algo.name(), faults, hits);
                }
            }
            "9" => break,
            other => println!("Unknown option: {}", other),
        }
    }
}

fn pick_replacement() -> Replacement {
    match ask("Policy (1 = FIFO, 2 = LRU, 3 = Optimal):").as_str() {
        "2" => Replacement::Lru,
        "3" => Replacement::Optimal,
        _ => Replacement::Fifo,
    }
}

// ---------- 4. synchronization ----------

fn sync_menu() {
    loop {
        println!("
--- Process Synchronization ---
1. Race condition (deterministic interleaving demo)
2. Race condition (REAL threads, then mutex-fixed)
3. Producer-consumer (REAL threads, bounded buffer)
4. Semaphore walkthrough (simulated P/V)
5. Back");
        match ask("Select an option:").as_str() {
            "1" => {
                let (result, correct) = sync::race_demo_interleaved();
                println!(
                    "Two processes each run `counter += 1` on a shared counter starting at 0.
P2 LOADS before P1 STORES, so P1's update is lost.
Result: counter = {} (correct value would be {}).",
                    result, correct
                );
            }
            "2" => {
                let threads = ask_u32_or("Number of threads", 4);
                let iters = ask_u32_or("Increments per thread", 100_000);
                let (racy, locked, expected) = sync::race_demo_threads(threads, iters);
                println!(
                    "Expected      : {}
RACY result   : {}  <- data race, updates lost
Mutex result  : {}  <- always exact",
                    expected, racy, locked
                );
            }
            "3" => {
                let producers = ask_u32_or("Producers", 2);
                let consumers = ask_u32_or("Consumers", 2);
                let items = ask_u32_or("Items per producer", 5);
                let capacity = ask_u32_or("Buffer capacity", 3);
                let got = sync::producer_consumer(producers, consumers, items, capacity as usize);
                let expected = (producers * items) as usize;
                println!("Consumed {} items (expected {}): {:?}", got.len(), expected, got);
                if got.len() == expected {
                    println!("No lost messages: producer-consumer correctness verified.");
                }
            }
            "4" => {
                for step in sync::semaphore_demo() {
                    println!("{}", step);
                }
            }
            "5" => break,
            other => println!("Unknown option: {}", other),
        }
    }
}

// ---------- 5. ipc ----------

fn ipc_menu(k: &mut Kernel) {
    loop {
        println!("
--- Inter-Process Communication ---
1. Create message queue
2. Send message
3. Receive message
4. Show queues and shared memory
5. Create shared memory region
6. Write to shared memory
7. Read from shared memory
8. Back");
        match ask("Select an option:").as_str() {
            "1" => {
                let capacity = ask_u32("Queue capacity:");
                let id = k.ipc.create_queue(capacity as usize);
                println!("Created queue Q{} (capacity {}).", id, capacity);
            }
            "2" => {
                let from = ask_u32("Sender PID:");
                let queue = ask_u32("Queue id (e.g. 1):");
                let to = ask_u32("Receiver PID:");
                let body = ask("Message body:");
                if report(k.send_message(from, queue, to, &body)).is_some() {
                    println!("Message delivered to queue.");
                }
            }
            "3" => {
                let queue = ask_u32("Queue id:");
                let pid = ask_u32("Receiver PID:");
                if let Some(msg) = report(k.receive_message(queue, pid)) {
                    println!("P{} received from P{}: {:?}", pid, msg.from, msg.body);
                }
            }
            "4" => println!("{}", k.ipc.summary()),
            "5" => {
                let size = ask_u32("Region size (bytes):");
                let id = k.ipc.create_region(size as usize);
                println!("Created shared memory region R{} ({} bytes).", id, size);
            }
            "6" => {
                let region = ask_u32("Region id:");
                let offset = ask_u32("Offset:");
                let data = ask("Data to write:");
                match k.ipc.write_region(region, offset as usize, data.as_bytes()) {
                    Ok(()) => println!("Wrote {} bytes.", data.len()),
                    Err(e) => println!("ERROR: {}", KernelError::from(e)),
                }
            }
            "7" => {
                let region = ask_u32("Region id:");
                let offset = ask_u32("Offset:");
                let len = ask_u32("Length:");
                match k.ipc.read_region(region, offset as usize, len as usize) {
                    Ok(bytes) => println!("Read: {:?}", String::from_utf8_lossy(&bytes)),
                    Err(e) => println!("ERROR: {}", KernelError::from(e)),
                }
            }
            "8" => break,
            other => println!("Unknown option: {}", other),
        }
    }
}

// ---------- 6. deadlock ----------

fn deadlock_menu(k: &mut Kernel) {
    loop {
        println!("
--- Deadlock Management --- ({} processes, {} resource types)
1. Load demo scenario (safe, for Banker's)
2. Load demo scenario (deadlocked, for detection)
3. Build scenario manually
4. Show current state
5. Run Banker's Algorithm
6. Request resources (safety-checked grant)
7. Run deadlock detection
8. Back", k.resources.num_processes(), k.resources.num_resources());
        match ask("Select an option:").as_str() {
            "1" => {
                k.resources = demo_banker_scenario();
                println!("{}", deadlock::display_state(&k.resources));
                println!("Classic Banker's example: safe state.");
            }
            "2" => {
                k.resources = demo_deadlock_scenario();
                println!("{}", deadlock::display_state(&k.resources));
                println!("Classic detection example: P1, P2, P3, P4 (indices) are deadlocked.");
            }
            "3" => {
                let m = ask_u32("Number of resource types:") as usize;
                let n = ask_u32("Number of processes:") as usize;
                let total = ask_vec_u32("Total resources", m);
                let mut rs = deadlock::ResourceState::new(total, n);
                for i in 0..n {
                    let alloc = ask_vec_u32(&format!("P{} allocated", i), m);
                    let max = ask_vec_u32(&format!("P{} max claim", i), m);
                    if let Err(e) = rs.set_process(i, alloc, max) {
                        println!("ERROR: {}", KernelError::from(e));
                    }
                    let req_line = ask(&format!("P{} outstanding request (Enter for none):", i));
                    let req: Vec<u32> = req_line.split_whitespace().filter_map(|w| w.parse().ok()).collect();
                    if req.len() == m {
                        rs.set_request(i, req).ok();
                    }
                }
                k.resources = rs;
                println!("Scenario loaded.");
            }
            "4" => println!("{}", deadlock::display_state(&k.resources)),
            "5" => {
                let safety = deadlock::bankers(&k.resources);
                if safety.safe {
                    println!(
                        "SAFE STATE. Safe sequence: {:?}",
                        safety.sequence
                    );
                } else {
                    println!("UNSAFE STATE. No safe sequence exists.");
                }
            }
            "6" => {
                let pid = ask_u32("Process index:") as usize;
                let req = ask_vec_u32("Request vector", k.resources.num_resources());
                match deadlock::try_grant(&mut k.resources, pid, &req) {
                    Ok(()) => println!("Request granted (state remains safe). New state:
{}", deadlock::display_state(&k.resources)),
                    Err(e) => println!("Request DENIED: {}", KernelError::from(e)),
                }
            }
            "7" => {
                let deadlocked = deadlock::detect_deadlock(&k.resources);
                if deadlocked.is_empty() {
                    println!("No deadlock detected.");
                } else {
                    println!("DEADLOCK detected involving processes: {:?}", deadlocked);
                }
            }
            "8" => break,
            other => println!("Unknown option: {}", other),
        }
    }
}

/// Silberschatz Banker's example: 5 processes, 3 resource types.
fn demo_banker_scenario() -> deadlock::ResourceState {
    let mut rs = deadlock::ResourceState::new(vec![10, 5, 7], 5);
    let alloc = [
        [0, 1, 0],
        [2, 0, 0],
        [3, 0, 2],
        [2, 1, 1],
        [0, 0, 2],
    ];
    let max = [
        [7, 5, 3],
        [3, 2, 2],
        [9, 0, 2],
        [2, 2, 2],
        [4, 3, 3],
    ];
    for i in 0..5 {
        rs.set_process(i, alloc[i].to_vec(), max[i].to_vec()).unwrap();
    }
    rs
}

/// Silberschatz deadlock-detection example: available (0,0,0),
/// P1, P2, P3, P4 deadlocked.
fn demo_deadlock_scenario() -> deadlock::ResourceState {
    let mut rs = deadlock::ResourceState::new(vec![7, 2, 6], 5);
    let alloc = [
        [0, 1, 0],
        [2, 0, 0],
        [3, 0, 3],
        [2, 1, 1],
        [0, 0, 2],
    ];
    let request = [
        [0, 0, 0],
        [2, 0, 2],
        [0, 0, 1],
        [1, 0, 0],
        [0, 0, 2],
    ];
    for i in 0..5 {
        rs.set_process(i, alloc[i].to_vec(), alloc[i].to_vec()).unwrap();
        rs.set_request(i, request[i].to_vec()).unwrap();
    }
    rs
}

// ---------- 7. system calls ----------

fn syscall_menu(k: &mut Kernel) {
    loop {
        println!("
--- System Call Interface ---
Available calls: fork_process, exit_process, wait_process, allocate_memory,
                 free_memory, send_message, receive_message
(All calls route through the kernel, validate arguments, delegate to the
 matching subsystem, and log an event.)

1. fork_process      4. allocate_memory
2. exit_process      5. free_memory
3. wait_process      6. send_message / receive_message (option 5 of main menu)
9. Back");
        match ask("Select an option:").as_str() {
            "1" => {
                let name = ask("Process name:");
                let arrival = ask_u32("Arrival time:");
                let burst = ask_u32("Burst time:");
                let priority = ask_u32_or("Priority (lower = higher)", 5);
                let mem = ask_u32("Memory requirement (0 = none):");
                if let Some(pid) = report(k.fork_process(&name, arrival, burst, priority, mem)) {
                    println!("fork_process -> pid {}", pid);
                }
            }
            "2" => {
                let pid = ask_u32("PID:");
                if report(k.exit_process(pid)).is_some() {
                    println!("exit_process(P{}) -> ok", pid);
                }
            }
            "3" => {
                let pid = ask_u32("PID:");
                if report(k.wait_process(pid)).is_some() {
                    println!("wait_process(P{}) -> P{} blocked (WAITING)", pid, pid);
                }
            }
            "4" => {
                let pid = ask_u32("PID:");
                let size = ask_u32("Size (units):");
                if let Some(start) = report(k.allocate_memory(pid, size)) {
                    println!("allocate_memory(P{}, {}) -> address {}", pid, size, start);
                }
            }
            "5" => {
                let pid = ask_u32("PID:");
                if let Some(freed) = report(k.free_memory(pid)) {
                    println!("free_memory(P{}) -> freed {}", pid, freed);
                }
            }
            "9" => break,
            other => println!("Unknown option: {}", other),
        }
    }
}
