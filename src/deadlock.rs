//! Deadlock management: Banker's Algorithm (avoidance) and deadlock
//! detection over a resource-allocation model.
//!
//! Resource vectors are indexed by resource type; matrices are `[pid][res]`.

use std::fmt;

/// Global resource state: totals, current allocations, declared maximum
/// claims, and outstanding requests (used by deadlock detection).
///
/// All matrices are `[pid][resource]`; every row has one entry per resource
/// type in `total`.
#[derive(Debug, Clone)]
pub struct ResourceState {
    /// Instances of each resource type in the system.
    pub total: Vec<u32>,
    /// `allocated[pid][res]`: instances currently held by each process.
    pub allocated: Vec<Vec<u32>>,
    /// `max[pid][res]`: the process's declared maximum claim.
    pub max: Vec<Vec<u32>>,
    /// `request[pid][res]`: outstanding requests used by detection.
    pub request: Vec<Vec<u32>>,
}

/// Errors returned by the Banker's/detection layer.
#[derive(Debug)]
pub enum DeadlockError {
    InvalidPid(usize),
    BadVector { expected: usize, got: usize },
    ExceedsClaim { pid: usize, requested: u32, remaining_claim: u32 },
    ExceedsAvailable { requested: u32, available: u32 },
    WouldBeUnsafe(usize),
    InconsistentState,
}

impl fmt::Display for DeadlockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeadlockError::InvalidPid(pid) => write!(f, "no such process index {pid}"),
            DeadlockError::BadVector { expected, got } => {
                write!(f, "expected a vector of {expected} values, got {got}")
            }
            DeadlockError::ExceedsClaim { pid, requested, remaining_claim } => write!(
                f,
                "P{pid}: request of {requested} exceeds its remaining claim ({remaining_claim})"
            ),
            DeadlockError::ExceedsAvailable { requested, available } => {
                write!(f, "request of {requested} exceeds available resources ({available})")
            }
            DeadlockError::WouldBeUnsafe(pid) => write!(
                f,
                "P{pid}: granting this request would leave the system in an UNSAFE state (request denied)"
            ),
            DeadlockError::InconsistentState => write!(f, "max claim below current allocation"),
        }
    }
}

impl ResourceState {
    /// New state with zeroed matrices for `num_processes` processes.
    pub fn new(total: Vec<u32>, num_processes: usize) -> Self {
        ResourceState {
            allocated: vec![vec![0; total.len()]; num_processes],
            max: vec![vec![0; total.len()]; num_processes],
            request: vec![vec![0; total.len()]; num_processes],
            total,
        }
    }

    /// Available = total - sum of allocations.
    pub fn available(&self) -> Vec<u32> {
        let mut avail = self.total.clone();
        for row in &self.allocated {
            for (r, a) in row.iter().enumerate() {
                avail[r] = avail[r].saturating_sub(*a);
            }
        }
        avail
    }

    /// Number of processes in the scenario.
    pub fn num_processes(&self) -> usize {
        self.allocated.len()
    }

    /// Number of resource types in the scenario.
    pub fn num_resources(&self) -> usize {
        self.total.len()
    }

    /// Set allocation and max for one process, validating consistency.
    pub fn set_process(&mut self, pid: usize, allocated: Vec<u32>, max: Vec<u32>) -> Result<(), DeadlockError> {
        if pid >= self.allocated.len() {
            return Err(DeadlockError::InvalidPid(pid));
        }
        if allocated.len() != self.num_resources() || max.len() != self.num_resources() {
            return Err(DeadlockError::BadVector { expected: self.num_resources(), got: allocated.len() });
        }
        if max.iter().zip(&allocated).any(|(m, a)| m < a) {
            return Err(DeadlockError::InconsistentState);
        }
        self.allocated[pid] = allocated;
        self.max[pid] = max;
        Ok(())
    }

    /// Replace a process's outstanding request vector.
    ///
    /// The request is data for detection only; `bankers` and `try_grant`
    /// use the remaining claim (`max - allocated`) instead.
    pub fn set_request(&mut self, pid: usize, request: Vec<u32>) -> Result<(), DeadlockError> {
        if pid >= self.request.len() {
            return Err(DeadlockError::InvalidPid(pid));
        }
        if request.len() != self.num_resources() {
            return Err(DeadlockError::BadVector { expected: self.num_resources(), got: request.len() });
        }
        self.request[pid] = request;
        Ok(())
    }
}

/// Banker's Algorithm safety check. Returns whether the state is safe and a
/// valid safe sequence (empty if unsafe).
///
/// Sweep invariant: `work` holds the resources that could be available after
/// every already-finished process releases its allocation. A process can
/// finish when its remaining claim fits in `work`; repeating the sweep until
/// no more progress is the textbook O(n^2 * m) safety check.
pub fn bankers(state: &ResourceState) -> Safety {
    let (n, m) = (state.num_processes(), state.num_resources());
    let mut work = state.available();
    let mut finish = vec![false; n];
    let mut sequence = Vec::with_capacity(n);

    // Defensive: a max claim below the current allocation is undefined.
    for i in 0..n {
        for r in 0..m {
            if state.max[i][r] < state.allocated[i][r] {
                return Safety { safe: false, sequence: vec![] };
            }
        }
    }

    loop {
        let mut progress = false;
        for i in 0..n {
            if finish[i] {
                continue;
            }
            let need_ok = (0..m).all(|r| state.max[i][r] - state.allocated[i][r] <= work[r]);
            if need_ok {
                for r in 0..m {
                    work[r] += state.allocated[i][r];
                }
                finish[i] = true;
                sequence.push(i);
                progress = true;
            }
        }
        if !progress {
            break;
        }
    }
    Safety { safe: sequence.len() == n, sequence }
}

/// Verdict returned by [`bankers`].
#[derive(Debug, Clone)]
pub struct Safety {
    /// True when every process appears in `sequence`.
    pub safe: bool,
    /// Process indices in a valid safe order (empty when unsafe).
    pub sequence: Vec<usize>,
}

/// Deadlock detection: same work/finish sweep as Banker's but using the
/// current request matrix instead of remaining claims. Returns the indices
/// of deadlocked processes (those that can never finish).
pub fn detect_deadlock(state: &ResourceState) -> Vec<usize> {
    let (n, m) = (state.num_processes(), state.num_resources());
    let mut work = state.available();
    let mut finish = vec![false; n];
    loop {
        let mut progress = false;
        for i in 0..n {
            if finish[i] {
                continue;
            }
            if (0..m).all(|r| state.request[i][r] <= work[r]) {
                for r in 0..m {
                    work[r] += state.allocated[i][r];
                }
                finish[i] = true;
                progress = true;
            }
        }
        if !progress {
            break;
        }
    }
    (0..n).filter(|&i| !finish[i]).collect()
}

/// Grant a resource request to `pid` only if the resulting state stays safe
/// (Banker's avoidance). Rolls back and errors otherwise.
pub fn try_grant(state: &mut ResourceState, pid: usize, req: &[u32]) -> Result<(), DeadlockError> {
    if pid >= state.num_processes() {
        return Err(DeadlockError::InvalidPid(pid));
    }
    if req.len() != state.num_resources() {
        return Err(DeadlockError::BadVector { expected: state.num_resources(), got: req.len() });
    }
    // A max claim below the current allocation makes the claim undefined;
    // refuse instead of underflowing on `max - allocated`.
    if (0..state.num_resources()).any(|r| state.max[pid][r] < state.allocated[pid][r]) {
        return Err(DeadlockError::InconsistentState);
    }
    let avail = state.available();
    for (r, &x) in req.iter().enumerate() {
        let need = state.max[pid][r] - state.allocated[pid][r];
        if x > need {
            return Err(DeadlockError::ExceedsClaim { pid, requested: x, remaining_claim: need });
        }
        if x > avail[r] {
            return Err(DeadlockError::ExceedsAvailable { requested: x, available: avail[r] });
        }
    }
    // Tentative grant.
    for (r, &x) in req.iter().enumerate() {
        state.allocated[pid][r] += x;
    }
    if bankers(state).safe {
        Ok(())
    } else {
        // Roll back: the request would push the system into an unsafe state.
        for (r, &x) in req.iter().enumerate() {
            state.allocated[pid][r] -= x;
        }
        Err(DeadlockError::WouldBeUnsafe(pid))
    }
}

/// Compact vector formatting for the matrix display: `(0, 1, 0)`.
fn fmt_vec(v: &[u32]) -> String {
    let parts: Vec<String> = v.iter().map(u32::to_string).collect();
    format!("({})", parts.join(", "))
}

/// One-line-per-process dump for menus, tinted by column.
pub fn display_state(state: &ResourceState) -> String {
    use crate::theme::{paint, CYAN, DIM, YELLOW};
    let mut s = String::new();
    let avail = state.available();
    let total_display = format!("{:?}", state.total);
    s.push_str(&format!(
        "{} {}\n",
        paint("Total     :", CYAN),
        total_display
    ));
    s.push_str(&format!(
        "{} {}\n",
        paint("Available :", CYAN),
        paint(&format!("{:?}", avail), YELLOW)
    ));
    s.push_str(&paint("PID  ALLOCATED        MAX              REQUEST", &format!("{CYAN}\x1b[1m")));
    s.push('\n');
    for i in 0..state.num_processes() {
        s.push_str(&format!(
            "{} {:<16} {:<16} {}\n",
            paint(&format!("{:<4}", i), DIM),
            fmt_vec(&state.allocated[i]),
            fmt_vec(&state.max[i]),
            paint(
                &fmt_vec(&state.request[i]),
                if state.request[i].iter().any(|&x| x > 0) {
                    YELLOW
                } else {
                    DIM
                }
            )
        ));
    }
    s
}
