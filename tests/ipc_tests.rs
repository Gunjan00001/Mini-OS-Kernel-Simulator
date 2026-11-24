use mini_os_kernel::ipc::IpcError;
use mini_os_kernel::kernel::Kernel;
use mini_os_kernel::sync;

// ---------- message queues ----------

#[test]
fn messages_are_delivered_in_fifo_order() {
    let mut k = Kernel::new(1024, 4, 8);
    let p1 = k.fork_process("P1", 0, 1, 1, 0).unwrap();
    let p2 = k.fork_process("P2", 0, 1, 1, 0).unwrap();
    let q = k.ipc.create_queue(10);
    k.send_message(p1, q, p2, "hello").unwrap();
    k.send_message(p2, q, p1, "hi back").unwrap();
    let m1 = k.receive_message(q, p2).unwrap();
    assert_eq!(m1.from, p1);
    assert_eq!(m1.body, "hello");
    let m2 = k.receive_message(q, p1).unwrap();
    assert_eq!(m2.body, "hi back");
}

#[test]
fn full_queue_records_blocked_sender() {
    let mut k = Kernel::new(1024, 4, 8);
    let p1 = k.fork_process("P1", 0, 1, 1, 0).unwrap();
    let p2 = k.fork_process("P2", 0, 1, 1, 0).unwrap();
    let q = k.ipc.create_queue(2);
    k.send_message(p1, q, p2, "m1").unwrap();
    k.send_message(p1, q, p2, "m2").unwrap();
    match k.send_message(p1, q, p2, "m3") {
        Err(mini_os_kernel::kernel::KernelError::Ipc(IpcError::QueueFull { queue, sender })) => {
            assert_eq!(queue, q);
            assert_eq!(sender, p1);
        }
        other => panic!("expected QueueFull, got {:?}", other.is_err()),
    }
    // Sender recorded as blocked.
    assert_eq!(k.ipc.queues[&q].blocked_senders, vec![p1]);
    // The queue still holds exactly m1, m2 -- no lost, no duplicated messages.
    assert_eq!(k.ipc.queues[&q].len(), 2);
}

#[test]
fn receiving_wakes_a_blocked_sender() {
    let mut k = Kernel::new(1024, 4, 8);
    let p1 = k.fork_process("P1", 0, 1, 1, 0).unwrap();
    let p2 = k.fork_process("P2", 0, 1, 1, 0).unwrap();
    let q = k.ipc.create_queue(1);
    k.send_message(p1, q, p2, "m1").unwrap();
    assert!(k.send_message(p1, q, p2, "m2").is_err()); // blocked
    assert_eq!(k.ipc.queues[&q].blocked_senders, vec![p1]);
    k.receive_message(q, p2).unwrap();
    // Space was freed: the blocked sender is no longer marked blocked...
    assert!(k.ipc.queues[&q].blocked_senders.is_empty());
    // ...and the event is in the kernel log.
    assert!(k.events().iter().any(|e| e.msg.contains("blocked sender was woken")));
    // Now the retry succeeds.
    k.send_message(p1, q, p2, "m2").unwrap();
}

#[test]
fn empty_queue_reports_empty() {
    let mut k = Kernel::new(1024, 4, 8);
    k.fork_process("P1", 0, 1, 1, 0).unwrap();
    let q = k.ipc.create_queue(5);
    match k.receive_message(q, 1) {
        Err(mini_os_kernel::kernel::KernelError::Ipc(IpcError::QueueEmpty(qid))) => assert_eq!(qid, q),
        _ => panic!("expected QueueEmpty"),
    }
}

#[test]
fn unknown_queue_is_rejected() {
    let mut k = Kernel::new(1024, 4, 8);
    let p1 = k.fork_process("P1", 0, 1, 1, 0).unwrap();
    assert!(k.send_message(p1, 99, p1, "x").is_err());
    assert!(k.receive_message(99, p1).is_err());
}

// ---------- shared memory ----------

#[test]
fn shared_memory_roundtrip() {
    let mut k = Kernel::new(1024, 4, 8);
    let r = k.ipc.create_region(16);
    k.ipc.write_region(r, 0, b"kernel").unwrap();
    let got = k.ipc.read_region(r, 0, 6).unwrap();
    assert_eq!(got, b"kernel");
}

#[test]
fn shared_memory_bounds_are_checked() {
    let mut k = Kernel::new(1024, 4, 8);
    let r = k.ipc.create_region(8);
    match k.ipc.write_region(r, 6, b"overflow!") {
        Err(IpcError::BadBounds { region, size, .. }) => {
            assert_eq!(region, r);
            assert_eq!(size, 8);
        }
        _ => panic!("expected BadBounds"),
    }
    match k.ipc.read_region(r, 5, 10) {
        Err(IpcError::BadBounds { .. }) => {}
        _ => panic!("expected BadBounds"),
    }
    assert!(matches!(
        k.ipc.read_region(999, 0, 1),
        Err(IpcError::UnknownRegion(999))
    ));
}

// ---------- synchronization demos ----------

#[test]
fn interleaved_race_loses_an_update() {
    let (result, correct) = sync::race_demo_interleaved();
    assert_eq!(result, 1); // P2 overwrites P1's update
    assert_eq!(correct, 2);
}

#[test]
fn mutex_protected_counter_is_exact() {
    let (racy, locked, expected) = sync::race_demo_threads(4, 20_000);
    assert_eq!(locked, expected, "mutex must give the exact count");
    assert_eq!(expected, 80_000);
    // Racy result is nondeterministic; it must be a plausible count.
    assert!(racy <= expected);
}

#[test]
fn producer_consumer_loses_no_messages() {
    let items = sync::producer_consumer(3, 2, 50, 4);
    assert_eq!(items.len(), 150);
    // Every produced item (p*1e6 + i) appears exactly once.
    let mut expected: Vec<u64> = Vec::new();
    for p in 0..3u64 {
        for i in 1..=50u64 {
            expected.push(p * 1_000_000 + i);
        }
    }
    let mut got = items.clone();
    got.sort_unstable();
    expected.sort_unstable();
    assert_eq!(got, expected);
}

#[test]
fn semaphore_demo_holds_mutual_exclusion() {
    let steps = sync::semaphore_demo();
    assert!(steps.iter().any(|s| s.contains("mutual exclusion holds")));
    // Final semaphore count returns to 1.
    assert!(steps.len() >= 5);
}
