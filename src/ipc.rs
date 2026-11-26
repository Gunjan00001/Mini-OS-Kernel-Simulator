//! Inter-process communication: bounded message queues and shared memory.

use std::collections::{BTreeMap, VecDeque};
use std::fmt;

/// One message in a bounded queue.
#[derive(Debug, Clone)]
pub struct Message {
    /// Sending process id.
    pub from: u32,
    /// Intended receiver process id.
    pub to: u32,
    /// Message payload.
    pub body: String,
}

/// Errors returned by queues and shared-memory regions.
#[derive(Debug)]
pub enum IpcError {
    UnknownQueue(u32),
    UnknownRegion(u32),
    QueueFull { queue: u32, sender: u32 },
    QueueEmpty(u32),
    BadBounds { region: u32, offset: usize, len: usize, size: usize },
}

impl fmt::Display for IpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IpcError::UnknownQueue(q) => write!(f, "no such message queue Q{q}"),
            IpcError::UnknownRegion(r) => write!(f, "no such shared memory region R{r}"),
            IpcError::QueueFull { queue, sender } => write!(
                f,
                "Q{queue} is full: P{sender} would block (recorded as blocked sender)"
            ),
            IpcError::QueueEmpty(q) => write!(f, "Q{q} is empty: receiver would block"),
            IpcError::BadBounds { region, offset, len, size } => write!(
                f,
                "R{region}: access [{offset}, {}) out of bounds (region size {size})",
                offset + len
            ),
        }
    }
}

/// Bounded FIFO message queue.
///
/// A send on a full queue is rejected *and* recorded in `blocked_senders`, so
/// the UI can show which process would have blocked. A successful receive
/// removes the oldest blocked sender.
#[derive(Debug)]
pub struct MessageQueue {
    pub capacity: usize,
    /// Processes that tried to send while the queue was full.
    pub blocked_senders: Vec<u32>,
    messages: VecDeque<Message>,
}

impl MessageQueue {
    fn new(capacity: usize) -> Self {
        MessageQueue {
            capacity: capacity.max(1),
            blocked_senders: Vec::new(),
            messages: VecDeque::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.messages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    pub fn messages(&self) -> &VecDeque<Message> {
        &self.messages
    }
}

/// IPC subsystem: message queues and shared-memory regions, keyed by id.
///
/// Queue and region ids share one counter so displays can use unambiguous
/// `Q1`/`R2` labels.
pub struct Ipc {
    pub queues: BTreeMap<u32, MessageQueue>,
    pub regions: BTreeMap<u32, Vec<u8>>,
    next_id: u32,
}

impl Default for Ipc {
    fn default() -> Self {
        Self::new()
    }
}

impl Ipc {
    /// Start with no queues or regions.
    pub fn new() -> Self {
        Ipc { queues: BTreeMap::new(), regions: BTreeMap::new(), next_id: 1 }
    }

    /// Monotonic id shared by queues and regions.
    fn alloc_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Create a bounded queue. Returns its id (Q1, Q2, ...).
    pub fn create_queue(&mut self, capacity: usize) -> u32 {
        let id = self.alloc_id();
        self.queues.insert(id, MessageQueue::new(capacity));
        id
    }

    /// Send a message. If the queue is full the sender is recorded as blocked
    /// and an error is returned (simulated blocking).
    pub fn send(&mut self, queue: u32, from: u32, to: u32, body: impl Into<String>) -> Result<(), IpcError> {
        let q = self.queues.get_mut(&queue).ok_or(IpcError::UnknownQueue(queue))?;
        if q.messages.len() >= q.capacity {
            q.blocked_senders.push(from);
            return Err(IpcError::QueueFull { queue, sender: from });
        }
        q.messages.push_back(Message { from, to, body: body.into() });
        Ok(())
    }

    /// Receive the oldest message (FIFO) from a queue. A successful receive
    /// frees a slot, so the longest-blocked sender is marked unblocked.
    pub fn receive(&mut self, queue: u32) -> Result<Message, IpcError> {
        let q = self.queues.get_mut(&queue).ok_or(IpcError::UnknownQueue(queue))?;
        let msg = q.messages.pop_front().ok_or(IpcError::QueueEmpty(queue))?;
        if !q.blocked_senders.is_empty() {
            q.blocked_senders.remove(0);
        }
        Ok(msg)
    }

    /// Create a shared memory region of `size` bytes. Returns its id (R1, ...).
    pub fn create_region(&mut self, size: usize) -> u32 {
        let id = self.alloc_id();
        self.regions.insert(id, vec![0; size]);
        id
    }

    /// Write `data` at `offset`. The whole write must fit in one region;
    /// partial writes are rejected rather than silently truncated.
    pub fn write_region(&mut self, region: u32, offset: usize, data: &[u8]) -> Result<(), IpcError> {
        let buf = self.regions.get_mut(&region).ok_or(IpcError::UnknownRegion(region))?;
        if offset.saturating_add(data.len()) > buf.len() {
            return Err(IpcError::BadBounds { region, offset, len: data.len(), size: buf.len() });
        }
        buf[offset..offset + data.len()].copy_from_slice(data);
        Ok(())
    }

    /// Copy `len` bytes starting at `offset` out of a region.
    pub fn read_region(&self, region: u32, offset: usize, len: usize) -> Result<Vec<u8>, IpcError> {
        let buf = self.regions.get(&region).ok_or(IpcError::UnknownRegion(region))?;
        if offset.saturating_add(len) > buf.len() {
            return Err(IpcError::BadBounds { region, offset, len, size: buf.len() });
        }
        Ok(buf[offset..offset + len].to_vec())
    }

    /// Colored summary of every queue for menus, with capacity bars.
    pub fn summary(&self) -> String {
        use crate::theme::{self, paint, BOLD, CYAN, DIM, RED};
        if self.queues.is_empty() && self.regions.is_empty() {
            return paint("No queues or shared memory regions yet.", DIM);
        }
        let mut s = String::new();
        for (id, q) in &self.queues {
            let blocked = if q.blocked_senders.is_empty() {
                paint("none", DIM)
            } else {
                paint(&format!("{:?}", q.blocked_senders), RED)
            };
            s.push_str(&format!(
                "{} {} {}/{} messages{}  blocked senders: {}\n",
                paint(&format!("Q{id}"), &format!("{BOLD}{CYAN}")),
                theme::bar(q.len() as u32, q.capacity as u32, 10),
                q.len(),
                q.capacity,
                "",
                blocked
            ));
            for (i, m) in q.messages().iter().enumerate() {
                s.push_str(&format!(
                    "  {} {} {} {}: {:?}\n",
                    paint(&format!("[{}]", i), DIM),
                    paint(&format!("P{}", m.from), theme::pid_fg(m.from)),
                    paint("->", DIM),
                    paint(&format!("P{}", m.to), theme::pid_fg(m.to)),
                    m.body
                ));
            }
        }
        for (id, r) in &self.regions {
            s.push_str(&format!(
                "{} {} {} bytes\n",
                paint(&format!("R{id}"), &format!("{BOLD}{CYAN}")),
                paint("shared memory,", DIM),
                r.len()
            ));
        }
        s
    }
}
