//! Inter-process communication: bounded message queues and shared memory.

use std::collections::{BTreeMap, VecDeque};
use std::fmt;

#[derive(Debug, Clone)]
pub struct Message {
    pub from: u32,
    pub to: u32,
    pub body: String,
}

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
    pub fn new() -> Self {
        Ipc { queues: BTreeMap::new(), regions: BTreeMap::new(), next_id: 1 }
    }

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

    /// Receive the oldest message (FIFO) from a queue.
    pub fn receive(&mut self, queue: u32) -> Result<Message, IpcError> {
        let q = self.queues.get_mut(&queue).ok_or(IpcError::UnknownQueue(queue))?;
        q.messages.pop_front().ok_or(IpcError::QueueEmpty(queue))
    }

    /// Create a shared memory region of `size` bytes. Returns its id (R1, ...).
    pub fn create_region(&mut self, size: usize) -> u32 {
        let id = self.alloc_id();
        self.regions.insert(id, vec![0; size]);
        id
    }

    pub fn write_region(&mut self, region: u32, offset: usize, data: &[u8]) -> Result<(), IpcError> {
        let buf = self.regions.get_mut(&region).ok_or(IpcError::UnknownRegion(region))?;
        if offset.saturating_add(data.len()) > buf.len() {
            return Err(IpcError::BadBounds { region, offset, len: data.len(), size: buf.len() });
        }
        buf[offset..offset + data.len()].copy_from_slice(data);
        Ok(())
    }

    pub fn read_region(&self, region: u32, offset: usize, len: usize) -> Result<Vec<u8>, IpcError> {
        let buf = self.regions.get(&region).ok_or(IpcError::UnknownRegion(region))?;
        if offset.saturating_add(len) > buf.len() {
            return Err(IpcError::BadBounds { region, offset, len, size: buf.len() });
        }
        Ok(buf[offset..offset + len].to_vec())
    }

    /// One-line summary of every queue for menus.
    pub fn summary(&self) -> String {
        if self.queues.is_empty() && self.regions.is_empty() {
            return "No queues or shared memory regions yet.".to_string();
        }
        let mut s = String::new();
        for (id, q) in &self.queues {
            s.push_str(&format!(
                "Q{}: {}/{} messages, blocked senders: {:?}\n",
                id,
                q.len(),
                q.capacity,
                q.blocked_senders
            ));
            for (i, m) in q.messages().iter().enumerate() {
                s.push_str(&format!("  [{}] P{} -> P{}: {:?}\n", i, m.from, m.to, m.body));
            }
        }
        for (id, r) in &self.regions {
            s.push_str(&format!("R{}: shared memory, {} bytes\n", id, r.len()));
        }
        s
    }
}
