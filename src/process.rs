//! Process representation, states, and state-transition validation.

use std::fmt;

/// Lifecycle states of a simulated process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    New,
    Ready,
    Running,
    Waiting,
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
#[derive(Debug, Clone)]
pub struct Process {
    pub pid: u32,
    pub name: String,
    pub state: State,
    pub arrival: u32,
    pub burst: u32,
    pub remaining: u32,
    /// Lower value = higher priority (like `nice`).
    pub priority: u32,
    pub mem_req: u32,
    /// Total CPU time executed so far (simulated runs do not modify this;
    /// the scheduler reports statistics separately).
    pub cpu_time: u32,
    pub wait_time: u32,
    pub turnaround: u32,
}

impl Process {
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
