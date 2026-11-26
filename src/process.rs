//! Process representation, states, and state-transition validation.

use std::fmt;

/// Lifecycle states of a simulated process.
///
/// The legal moves between these states are enforced by
/// [`State::can_transition_to`]; illegal moves are demo material, not bugs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// Created but not yet admitted to the ready queue.
    New,
    /// Runnable, waiting for the CPU.
    Ready,
    /// Currently dispatched on the (single) simulated CPU.
    Running,
    /// Blocked on a resource or event; cannot run until woken.
    Waiting,
    /// Finished; all resources have been released.
    Terminated,
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            State::New => "NEW",
            State::Ready => "READY",
            State::Running => "RUNNING",
            State::Waiting => "WAITING",
            State::Terminated => "TERMINATED",
        })
    }
}

impl State {
    /// The legal transition graph:
    /// `NEW -> READY -> RUNNING -> {READY | WAITING | TERMINATED}`, `WAITING -> READY`.
    pub fn can_transition_to(self, next: State) -> bool {
        matches!(
            (self, next),
            (State::New, State::Ready)
                | (State::Ready, State::Running)
                | (State::Running, State::Ready)
                | (State::Running, State::Waiting)
                | (State::Running, State::Terminated)
                | (State::Waiting, State::Ready)
        )
    }
}

/// Process Control Block for a simulated process.
///
/// The scheduler works on clones of these fields; scheduling a workload never
/// mutates the live process table (see [`crate::scheduler::schedule`]).
#[derive(Debug, Clone)]
pub struct Process {
    /// Kernel-assigned process id (`P1`, `P2`, ...).
    pub pid: u32,
    /// Human-readable name shown in tables and the Gantt chart.
    pub name: String,
    /// Current lifecycle state.
    pub state: State,
    /// Tick at which the process becomes schedulable.
    pub arrival: u32,
    /// Total CPU time the process needs, in ticks.
    pub burst: u32,
    /// CPU time left to run (initialised to `burst`).
    pub remaining: u32,
    /// Lower value = higher priority (like `nice`).
    pub priority: u32,
    /// Contiguous memory requested at fork time (`0` = no allocation).
    pub mem_req: u32,
    /// Total CPU time executed so far (simulated runs do not modify this;
    /// the scheduler reports statistics separately).
    pub cpu_time: u32,
    /// Accumulated waiting time, available for bookkeeping by callers.
    pub wait_time: u32,
    /// Completion time minus arrival time, available for bookkeeping.
    pub turnaround: u32,
}

impl Process {
    /// Build a PCB in [`State::New`] with `remaining == burst` and zeroed
    /// statistics.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        pid: u32,
        name: impl Into<String>,
        arrival: u32,
        burst: u32,
        priority: u32,
        mem_req: u32,
    ) -> Self {
        Process {
            pid,
            name: name.into(),
            state: State::New,
            arrival,
            burst,
            remaining: burst,
            priority,
            mem_req,
            cpu_time: 0,
            wait_time: 0,
            turnaround: 0,
        }
    }

    /// Attempt a state transition, enforcing the legal graph.
    pub fn transition(&mut self, next: State) -> Result<State, String> {
        if self.state.can_transition_to(next) {
            self.state = next;
            Ok(next)
        } else {
            Err(format!(
                "invalid transition {} -> {} for {}",
                self.state, next, self.name
            ))
        }
    }
}
