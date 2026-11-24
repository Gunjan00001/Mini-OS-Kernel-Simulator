use mini_os_kernel::kernel::Kernel;
use mini_os_kernel::process::{Process, State};

/// Build a process whose current state is `state` (via legal hops only).
fn at(state: State) -> Process {
    let mut p = Process::new(1, "P1", 0, 5, 1, 0);
    match state {
        State::New => {}
        State::Ready => {
            p.transition(State::Ready).unwrap();
        }
        State::Running => {
            p.transition(State::Ready).unwrap();
            p.transition(State::Running).unwrap();
        }
        State::Waiting => {
            p.transition(State::Ready).unwrap();
            p.transition(State::Running).unwrap();
            p.transition(State::Waiting).unwrap();
        }
        State::Terminated => unreachable!("use the dedicated terminated test"),
    }
    p
}

#[test]
fn creation_initializes_new_state() {
    let p = Process::new(1, "P1", 0, 5, 3, 100);
    assert_eq!(p.state, State::New);
    assert_eq!(p.remaining, 5);
    assert_eq!(p.pid, 1);
}

#[test]
fn legal_transitions() {
    let legal = [
        (State::New, State::Ready),
        (State::Ready, State::Running),
        (State::Running, State::Ready),
        (State::Running, State::Waiting),
        (State::Running, State::Terminated),
        (State::Waiting, State::Ready),
    ];
    for (from, to) in legal {
        assert!(
            at(from).transition(to).is_ok(),
            "{} -> {} should be legal",
            from,
            to
        );
    }
}

#[test]
fn illegal_transitions() {
    let illegal = [
        (State::New, State::Running),
        (State::New, State::Waiting),
        (State::New, State::Terminated),
        (State::Ready, State::New),
        (State::Ready, State::Waiting),
        (State::Ready, State::Terminated),
        (State::Running, State::New),
        (State::Waiting, State::Running),
        (State::Waiting, State::Waiting),
        (State::Waiting, State::Terminated),
    ];
    for (from, to) in illegal {
        assert!(
            !at(from).transition(to).is_ok(),
            "{} -> {} should be illegal",
            from,
            to
        );
    }
    // Nothing leaves TERMINATED.
    let mut p = at(State::Running);
    p.transition(State::Terminated).unwrap();
    for to in [State::New, State::Ready, State::Running, State::Waiting] {
        assert!(!p.transition(to).is_ok(), "TERMINATED -> {} should be illegal", to);
    }
}

#[test]
fn fork_creates_ready_process_and_allocates_memory() {
    let mut k = Kernel::new(1024, 4, 8);
    let pid = k.fork_process("init", 0, 5, 1, 300).unwrap();
    let p = k.processes.get(&pid).unwrap();
    assert_eq!(p.state, State::Ready);
    assert_eq!(p.name, "init");
    assert_eq!(k.memory.used(), 300);
    assert_eq!(k.next_pid, 2);
    // Events were logged.
    assert!(k.events().iter().any(|e| e.msg.contains("created")));
}

#[test]
fn fork_fails_when_memory_insufficient_and_leaves_no_process() {
    let mut k = Kernel::new(100, 4, 8);
    assert!(k.fork_process("big", 0, 5, 1, 500).is_err());
    assert!(k.processes.is_empty());
    assert_eq!(k.memory.used(), 0);
    // A zero-size request is invalid too.
    assert!(k.fork_process("zero", 0, 5, 1, 0).is_ok()); // 0 mem = no allocation
}

#[test]
fn failed_fork_does_not_consume_a_pid() {
    let mut k = Kernel::new(100, 4, 8);
    assert!(k.fork_process("big", 0, 5, 1, 500).is_err());
    // The next successful creation gets the lowest unused pid.
    let pid = k.fork_process("small", 0, 5, 1, 10).unwrap();
    assert_eq!(pid, 1);
}

#[test]
fn exit_terminates_and_frees_memory() {
    let mut k = Kernel::new(1024, 4, 8);
    let pid = k.fork_process("worker", 0, 5, 1, 256).unwrap();
    k.exit_process(pid).unwrap();
    assert_eq!(k.processes[&pid].state, State::Terminated);
    assert_eq!(k.memory.used(), 0);
    // Terminated processes cannot exit again.
    assert!(k.exit_process(pid).is_err());
}

#[test]
fn invalid_pid_errors() {
    let mut k = Kernel::new(1024, 4, 8);
    assert!(k.exit_process(42).is_err());
    assert!(k.wait_process(42).is_err());
    assert!(k.allocate_memory(42, 10).is_err());
    assert!(k.free_memory(42).is_err());
    assert!(k.receive_message(1, 42).is_err());
    assert!(k.send_message(42, 1, 1, "hi").is_err());
}

#[test]
fn invalid_state_transition_is_rejected() {
    let mut k = Kernel::new(1024, 4, 8);
    let pid = k.fork_process("p", 0, 5, 1, 0).unwrap(); // state: READY
    // WAITING is only reachable from RUNNING.
    assert!(matches!(
        k.wait_process(pid),
        Err(mini_os_kernel::kernel::KernelError::InvalidTransition(..))
    ));
    // But READY -> RUNNING -> WAITING works.
    k.dispatch_process(pid).unwrap();
    k.wait_process(pid).unwrap();
    assert_eq!(k.processes[&pid].state, State::Waiting);
    k.wake_process(pid).unwrap();
    assert_eq!(k.processes[&pid].state, State::Ready);
}

#[test]
fn dashboard_shows_a_single_current_process() {
    let mut k = Kernel::new(1024, 4, 8);
    let p1 = k.fork_process("a", 0, 1, 1, 0).unwrap();
    let p2 = k.fork_process("b", 0, 1, 1, 0).unwrap();
    k.dispatch_process(p1).unwrap();
    k.dispatch_process(p2).unwrap();
    let line = k
        .dashboard()
        .lines()
        .find(|l| l.starts_with("CURRENT PROCESS"))
        .unwrap()
        .to_string();
    assert_eq!(line.trim(), "CURRENT PROCESS  : P1");
    // Both still listed in the ready queue.
    let d = k.dashboard();
    assert!(d.contains("READY QUEUE      : -"));
}

#[test]
fn paging_attach_requires_an_existing_process() {
    let mut k = Kernel::new(1024, 4, 8);
    assert!(k.attach_paging(9, 16).is_err());
    let pid = k.fork_process("p", 0, 5, 1, 0).unwrap();
    assert_eq!(k.attach_paging(pid, 10).unwrap(), 3); // ceil(10 / 4)
    assert_eq!(k.paging.used_frames(), 0); // pages are loaded lazily
    // Exiting the process releases the page table.
    k.exit_process(pid).unwrap();
    assert!(k.paging.translate(pid, 0, mini_os_kernel::paging::Replacement::Fifo).is_err());
}

#[test]
fn every_syscall_advances_the_tick_and_logs() {
    let mut k = Kernel::new(1024, 4, 8);
    let before = k.tick;
    let pid = k.fork_process("p", 0, 5, 1, 0).unwrap();
    assert!(k.tick > before);
    assert!(k.events().len() >= 2);
    k.allocate_memory(pid, 100).unwrap();
    assert!(k.events().last().unwrap().msg.contains("allocated"));
}
