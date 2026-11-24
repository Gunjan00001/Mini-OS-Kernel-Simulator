use mini_os_kernel::deadlock::{self, DeadlockError, ResourceState};

/// Silberschatz classic Banker's example (safe).
fn banker_state() -> ResourceState {
    let mut rs = ResourceState::new(vec![10, 5, 7], 5);
    let alloc = [[0, 1, 0], [2, 0, 0], [3, 0, 2], [2, 1, 1], [0, 0, 2]];
    let max = [[7, 5, 3], [3, 2, 2], [9, 0, 2], [2, 2, 2], [4, 3, 3]];
    for i in 0..5 {
        rs.set_process(i, alloc[i].to_vec(), max[i].to_vec()).unwrap();
    }
    rs
}

/// Silberschatz deadlock-detection example: P1..P4 (indices 1-4) deadlocked.
fn deadlock_state() -> ResourceState {
    let mut rs = ResourceState::new(vec![7, 2, 6], 5);
    let alloc = [[0, 1, 0], [2, 0, 0], [3, 0, 3], [2, 1, 1], [0, 0, 2]];
    let request = [[0, 0, 0], [2, 0, 2], [0, 0, 1], [1, 0, 0], [0, 0, 2]];
    for i in 0..5 {
        rs.set_process(i, alloc[i].to_vec(), alloc[i].to_vec()).unwrap();
        rs.set_request(i, request[i].to_vec()).unwrap();
    }
    rs
}

#[test]
fn bankers_finds_safe_state() {
    let s = deadlock::bankers(&banker_state());
    assert!(s.safe);
    assert_eq!(s.sequence.len(), 5);
    // The sequence must contain every process exactly once.
    let mut seen = s.sequence.clone();
    seen.sort();
    assert_eq!(seen, vec![0, 1, 2, 3, 4]);
}

#[test]
fn bankers_rejects_unsafe_state() {
    // One resource type, total 3. P0 holds 2 (max 3), P1 holds 1 (max 3).
    // Available = 0; nobody can finish -> unsafe.
    let mut rs = ResourceState::new(vec![3], 2);
    rs.set_process(0, vec![2], vec![3]).unwrap();
    rs.set_process(1, vec![1], vec![3]).unwrap();
    let s = deadlock::bankers(&rs);
    assert!(!s.safe);
    assert!(s.sequence.is_empty());
}

#[test]
fn bankers_on_empty_state_is_trivially_safe() {
    let rs = ResourceState::new(vec![10, 5, 7], 0);
    assert!(deadlock::bankers(&rs).safe);
}

#[test]
fn detection_finds_the_deadlocked_processes() {
    let deadlocked = deadlock::detect_deadlock(&deadlock_state());
    assert_eq!(deadlocked, vec![1, 2, 3, 4]);
}

#[test]
fn detection_reports_no_deadlock_when_all_can_finish() {
    let mut rs = banker_state();
    for i in 0..5 {
        rs.set_request(i, vec![0, 0, 0]).unwrap();
    }
    assert!(deadlock::detect_deadlock(&rs).is_empty());
}

#[test]
fn safe_request_is_granted() {
    let mut rs = banker_state();
    // Classic safe request: P1 requests (1,0,2).
    deadlock::try_grant(&mut rs, 1, &[1, 0, 2]).unwrap();
    assert_eq!(rs.allocated[1], vec![3, 0, 2]);
    assert!(deadlock::bankers(&rs).safe);
}

#[test]
fn unsafe_request_is_denied_and_rolled_back() {
    let mut rs = banker_state();
    // P0 requests (7,4,3) = its entire remaining claim; available is (3,3,2),
    // so it exceeds availability anyway. Use a request that fits availability
    // but breaks safety: P4 requests (3,3,0) -> available (3,3,2) covers it,
    // but afterwards nothing can finish first except... (known unsafe case).
    let before = rs.clone();
    let err = deadlock::try_grant(&mut rs, 4, &[3, 3, 0]).unwrap_err();
    assert!(matches!(err, DeadlockError::WouldBeUnsafe(4)));
    assert_eq!(rs.allocated, before.allocated); // rolled back
    assert_eq!(rs.available(), before.available());
}

#[test]
fn request_exceeding_claim_or_availability_is_rejected() {
    let mut rs = banker_state();
    // Exceeds remaining claim: P1 max (3,2,2), allocated (2,0,0), claim (1,2,2).
    let e1 = deadlock::try_grant(&mut rs, 1, &[2, 0, 0]).unwrap_err();
    assert!(matches!(e1, DeadlockError::ExceedsClaim { .. }));
    // Exceeds availability: P0 claim (7,4,3), request (4,0,0) but available (3,3,2).
    let e2 = deadlock::try_grant(&mut rs, 0, &[4, 0, 0]).unwrap_err();
    assert!(matches!(e2, DeadlockError::ExceedsAvailable { .. }));
}

#[test]
fn bad_inputs_are_rejected() {
    let mut rs = banker_state();
    assert!(matches!(
        deadlock::try_grant(&mut rs, 9, &[0, 0, 0]),
        Err(DeadlockError::InvalidPid(9))
    ));
    assert!(matches!(
        deadlock::try_grant(&mut rs, 0, &[0, 0]),
        Err(DeadlockError::BadVector { .. })
    ));
    let mut bad = ResourceState::new(vec![3], 1);
    assert!(matches!(
        bad.set_process(0, vec![2], vec![1]),
        Err(DeadlockError::InconsistentState)
    ));
}

#[test]
fn available_is_total_minus_allocated() {
    let rs = banker_state();
    assert_eq!(rs.available(), vec![3, 3, 2]);
}
