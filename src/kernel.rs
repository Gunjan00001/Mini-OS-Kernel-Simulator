//! The kernel simulator: owns every subsystem, hosts the system-call
//! interface, and keeps the centralized event log.

use std::collections::BTreeMap;
use std::fmt;

use crate::deadlock::{DeadlockError, ResourceState};
use crate::ipc::{Ipc, IpcError, Message};
use crate::memory::{MemError, MemoryManager};
use crate::paging::{PagedMemory, PagingError};
use crate::process::{Process, State};
use crate::scheduler::{self, Algorithm};

#[derive(Debug)]
pub enum KernelError {
    NotFound(u32),
    InvalidTransition(u32, State, State),
    Memory(MemError),
    Paging(PagingError),
    Ipc(IpcError),
    Deadlock(DeadlockError),
    Other(String),
}

impl fmt::Display for KernelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KernelError::NotFound(pid) => write!(f, "no such process (pid {pid})"),
            KernelError::InvalidTransition(pid, from, to) => {
                write!(f, "P{pid}: illegal state transition {} -> {}", from, to)
            }
            KernelError::Memory(e) => write!(f, "memory: {e}"),
            KernelError::Paging(e) => write!(f, "paging: {e}"),
            KernelError::Ipc(e) => write!(f, "ipc: {e}"),
            KernelError::Deadlock(e) => write!(f, "deadlock: {e}"),
            KernelError::Other(msg) => write!(f, "{msg}"),
        }
    }
}

impl From<MemError> for KernelError {
    fn from(e: MemError) -> Self {
        KernelError::Memory(e)
    }
}

impl From<PagingError> for KernelError {
    fn from(e: PagingError) -> Self {
        KernelError::Paging(e)
    }
}

impl From<IpcError> for KernelError {
    fn from(e: IpcError) -> Self {
        KernelError::Ipc(e)
    }
}

impl From<DeadlockError> for KernelError {
    fn from(e: DeadlockError) -> Self {
        KernelError::Deadlock(e)
    }
}

#[derive(Debug, Clone)]
pub struct Event {
    pub tick: u32,
    pub msg: String,
}

/// The kernel. Owns every subsystem; nothing here is global static state.
pub struct Kernel {
    pub tick: u32,
    pub next_pid: u32,
    /// Echo every event to stdout as it happens (enabled by the UI).
    pub echo_events: bool,
    pub processes: BTreeMap<u32, Process>,
    pub scheduler_algo: Algorithm,
    pub last_schedule: Option<scheduler::ScheduleResult>,
    pub memory: MemoryManager,
    pub paging: PagedMemory,
    pub ipc: Ipc,
    pub resources: ResourceState,
    events: Vec<Event>,
}

impl Kernel {
    /// `total_memory`: contiguous memory units; `page_size`/`frames`: paging pool.
    pub fn new(total_memory: u32, page_size: u32, frames: u32) -> Self {
        Kernel {
            tick: 0,
            next_pid: 1,
            echo_events: false,
            processes: BTreeMap::new(),
            scheduler_algo: Algorithm::Fcfs,
            last_schedule: None,
            memory: MemoryManager::new(total_memory),
            paging: PagedMemory::new(page_size, frames),
            ipc: Ipc::new(),
            resources: ResourceState::new(vec![10, 5, 7], 0),
            events: Vec::new(),
        }
    }

    // ---- event log ----

    /// Log an event; every event advances the simulation tick.
    pub fn log(&mut self, msg: impl Into<String>) {
        self.tick += 1;
        let msg = msg.into();
        if self.echo_events {
            println!("[{}] {}", self.tick, msg);
        }
        self.events.push(Event { tick: self.tick, msg });
    }

    pub fn events(&self) -> &[Event] {
        &self.events
    }

    pub fn print_events(&self) {
        for e in &self.events {
            println!("[{}] {}", e.tick, e.msg);
        }
    }

    // ---- internal helpers ----

    /// Validate and apply a state transition, then log it.
    fn set_state(&mut self, pid: u32, to: State) -> Result<(), KernelError> {
        let (name, from) = {
            let p = self.processes.get(&pid).ok_or(KernelError::NotFound(pid))?;
            (p.name.clone(), p.state)
        };
        self.processes
            .get_mut(&pid)
            .unwrap()
            .transition(to)
            .map_err(|_| KernelError::InvalidTransition(pid, from, to))?;
        self.log(format!("{} (P{}) moved {} -> {}", name, pid, from, to));
        Ok(())
    }

    fn require_pid(&self, pid: u32) -> Result<(), KernelError> {
        if self.processes.contains_key(&pid) {
            Ok(())
        } else {
            Err(KernelError::NotFound(pid))
        }
    }

    // ---- system calls ----

    /// `fork_process()`: create a process, allocate its memory, move it to READY.
    #[allow(clippy::too_many_arguments)]
    pub fn fork_process(
        &mut self,
        name: &str,
        arrival: u32,
        burst: u32,
        priority: u32,
        mem_req: u32,
    ) -> Result<u32, KernelError> {
        let pid = self.next_pid;
        if mem_req > 0 {
            self.memory.allocate(pid, name, mem_req)?;
        }
        // Only consume the pid once creation can no longer fail.
        self.next_pid += 1;
        self.processes
            .insert(pid, Process::new(pid, name, arrival, burst, priority, mem_req));
        self.log(format!("Process {} created (pid {})", name, pid));
        self.set_state(pid, State::Ready)?;
        Ok(pid)
    }

    /// `exit_process()`: terminate a process and release its resources.
    /// Walks the legal transition graph to TERMINATED from any live state.
    pub fn exit_process(&mut self, pid: u32) -> Result<(), KernelError> {
        let (name, state) = {
            let p = self.processes.get(&pid).ok_or(KernelError::NotFound(pid))?;
            (p.name.clone(), p.state)
        };
        if state == State::Terminated {
            return Err(KernelError::InvalidTransition(pid, state, State::Terminated));
        }
        if state != State::Running {
            if state == State::New || state == State::Waiting {
                self.set_state(pid, State::Ready)?;
            }
            self.set_state(pid, State::Running)?; // dispatched before exiting
        }
        self.set_state(pid, State::Terminated)?;
        if let Ok(freed) = self.memory.free(pid) {
            self.log(format!("Freed {} bytes of memory from {}", freed, name));
        }
        self.paging.detach(pid);
        self.log(format!("Process {} (P{}) terminated", name, pid));
        Ok(())
    }

    /// `wait_process()`: RUNNING -> WAITING (blocked on a resource/event).
    pub fn wait_process(&mut self, pid: u32) -> Result<(), KernelError> {
        self.set_state(pid, State::Waiting)
    }

    /// Wake a blocked process: WAITING -> READY.
    pub fn wake_process(&mut self, pid: u32) -> Result<(), KernelError> {
        self.set_state(pid, State::Ready)
    }

    /// Dispatch a process to the CPU: READY -> RUNNING.
    pub fn dispatch_process(&mut self, pid: u32) -> Result<(), KernelError> {
        self.set_state(pid, State::Running)
    }

    /// `allocate_memory()`: give `pid` a contiguous block.
    pub fn allocate_memory(&mut self, pid: u32, size: u32) -> Result<u32, KernelError> {
        self.require_pid(pid)?;
        let name = self.processes[&pid].name.clone();
        let start = self.memory.allocate(pid, &name, size)?;
        self.log(format!("P{} requested memory: {} bytes allocated at {}", pid, size, start));
        Ok(start)
    }

    /// `free_memory()`: release the block owned by `pid`.
    pub fn free_memory(&mut self, pid: u32) -> Result<u32, KernelError> {
        self.require_pid(pid)?;
        let freed = self.memory.free(pid)?;
        self.log(format!("P{} freed {} bytes of memory", pid, freed));
        Ok(freed)
    }

    /// `send_message()`: enqueue a message to a bounded queue.
    pub fn send_message(&mut self, from: u32, queue: u32, to: u32, body: &str) -> Result<(), KernelError> {
        for pid in [from, to] {
            self.require_pid(pid)?;
        }
        self.ipc.send(queue, from, to, body)?;
        self.log(format!("P{} sent message to P{} via Q{}", from, to, queue));
        Ok(())
    }

    /// `receive_message()`: dequeue the oldest message from a queue.
    pub fn receive_message(&mut self, queue: u32, pid: u32) -> Result<Message, KernelError> {
        self.require_pid(pid)?;
        let blocked_before = self
            .ipc
            .queues
            .get(&queue)
            .map(|q| q.blocked_senders.len())
            .unwrap_or(0);
        let msg = self.ipc.receive(queue)?;
        self.log(format!("P{} received message from P{} via Q{}", pid, msg.from, queue));
        let blocked_after = self
            .ipc
            .queues
            .get(&queue)
            .map(|q| q.blocked_senders.len())
            .unwrap_or(0);
        if blocked_after < blocked_before {
            self.log(format!(
                "Queue Q{} had space: a blocked sender was woken ({} still blocked)",
                queue, blocked_after
            ));
        }
        Ok(msg)
    }

    /// Paging helper: attach a page table to an existing process.
    /// Validates the pid through the kernel before touching the paging pool.
    pub fn attach_paging(&mut self, pid: u32, size_bytes: u32) -> Result<u32, KernelError> {
        self.require_pid(pid)?;
        let pages = self.paging.attach(pid, size_bytes);
        self.log(format!(
            "P{}: page table attached ({} pages, page size {})",
            pid, pages, self.paging.page_size
        ));
        Ok(pages)
    }

    // ---- scheduling ----

    /// Run `algo` over a snapshot of live processes. The snapshot is cloned,
    /// so the same workload can be re-scheduled and compared; process table
    /// states are untouched.
    pub fn run_schedule(&mut self, algo: Algorithm) -> Result<(), KernelError> {
        let snapshot: Vec<Process> = self
            .processes
            .values()
            .filter(|p| p.state != State::Terminated)
            .cloned()
            .collect();
        if snapshot.is_empty() {
            return Err(KernelError::Other("no runnable processes to schedule".to_string()));
        }
        let result = scheduler::schedule(&snapshot, algo);
        self.scheduler_algo = algo;
        self.log(format!("Scheduler {} started on {} processes", algo.label(), snapshot.len()));
        let mut prev: Option<u32> = None;
        for slot in &result.slots {
            if prev != Some(slot.pid) {
                self.log(format!("CPU assigned to P{}", slot.pid));
            }
            prev = Some(slot.pid);
        }
        let m = &result.metrics;
        self.log(format!(
            "Scheduling done: avg wait {:.2}, avg turnaround {:.2}, CPU util {:.1}%",
            m.avg_wait, m.avg_turnaround, m.cpu_util_pct
        ));
        self.last_schedule = Some(result);
        Ok(())
    }

    // ---- dashboard ----

    pub fn dashboard(&self) -> String {
        use crate::theme::{self, paint, BOLD, CYAN, DIM, GREEN, RED, YELLOW};

        let pids_in = |st: State| -> Vec<String> {
            self.processes
                .values()
                .filter(|p| p.state == st)
                .map(|p| format!("P{}", p.pid))
                .collect()
        };
        let running = pids_in(State::Running);
        let ready = pids_in(State::Ready);
        let waiting = pids_in(State::Waiting);
        let total = self.processes.len();
        let active = self.processes.values().filter(|p| p.state != State::Terminated).count();
        let list = |v: &[String]| -> String {
            if v.is_empty() {
                paint("-", DIM)
            } else {
                let colored: Vec<String> = v
                    .iter()
                    .map(|pid| {
                        let n = pid.trim_start_matches('P');
                        paint(pid, theme::pid_fg(n.parse().unwrap_or(0)))
                    })
                    .collect();
                colored.join(", ")
            }
        };
        let (used, free) = (self.memory.used(), self.memory.free_bytes());
        let (faults, hits) = (self.paging.faults, self.paging.hits);
        let (faults_s, hits_s) = (
            paint(&faults.to_string(), if faults > 0 { RED } else { DIM }),
            paint(&hits.to_string(), GREEN),
        );
        let pct = if self.memory.total == 0 {
            0
        } else {
            used * 100 / self.memory.total
        };
        let border = paint("╔════════════════════════════════════════════════════════╗", CYAN);
        let bottom = paint("╚════════════════════════════════════════════════════════╝", CYAN);
        let title = paint(
            "║                  KERNEL DASHBOARD                      ║",
            CYAN,
        );
        let memory_header = paint("MEMORY", &format!("{BOLD}{CYAN}"));
        let paging_header = paint("PAGING", &format!("{BOLD}{CYAN}"));
        format!(
"{border}
{title}
{bottom}
{}{}
{}{}
{}{}
{}{}
{}{}

{}{}
{}{}

{}
{}{}
{}{}
{}{}
{}{}
{}{}

{}
{}{}
{}{}
{}{}

{}{}
{bottom}",
            paint("CPU STATUS       : ", CYAN),
            paint(
                if running.is_empty() { "IDLE" } else { "RUNNING" },
                if running.is_empty() { DIM } else { GREEN },
            ),
            paint("SCHEDULER        : ", CYAN),
            paint(&self.scheduler_algo.label(), YELLOW),
            paint("CURRENT PROCESS  : ", CYAN),
            // The state machine does not forbid two manual dispatches, but
            // the dashboard's "current process" slot is singular: show the
            // lowest-pid running process.
            match running.first() {
                Some(pid) => paint(pid, theme::pid_fg(pid.trim_start_matches('P').parse().unwrap_or(0))),
                None => paint("-", DIM),
            },
            paint("READY QUEUE      : ", CYAN),
            list(&ready),
            paint("WAITING PROCESSES: ", CYAN),
            list(&waiting),
            paint("TOTAL PROCESSES  : ", CYAN),
            paint(&total.to_string(), YELLOW),
            paint("ACTIVE PROCESSES : ", CYAN),
            paint(&active.to_string(), GREEN),
            memory_header,
            paint("Total            : ", CYAN),
            paint(&format!("{} units", self.memory.total), DIM),
            paint("Used             : ", CYAN),
            paint(&used.to_string(), YELLOW),
            paint("Free             : ", CYAN),
            paint(&free.to_string(), GREEN),
            paint("Largest free hole: ", CYAN),
            self.memory.largest_free_hole().to_string(),
            paint("Memory usage     : ", CYAN),
            format!("{} {}%", theme::bar(used, self.memory.total, 24), paint(&pct.to_string(), YELLOW)),
            paging_header,
            paint("Frames used      : ", CYAN),
            format!("{}/{}", self.paging.used_frames(), self.paging.total_frames),
            paint("Page faults      : ", CYAN),
            faults_s,
            paint("Page hits        : ", CYAN),
            hits_s,
            paint("SYSTEM TIME      : ", CYAN),
            paint(&self.tick.to_string(), &format!("{BOLD}{YELLOW}")),
        )
    }
}
