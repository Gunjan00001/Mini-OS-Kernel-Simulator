//! Paging: page tables, logical -> physical address translation, and page
//! replacement (FIFO / LRU / Optimal).
//!
//! The frame pool is global (one physical memory shared by all processes),
//! which is the realistic model. Optimal replacement is only well-defined
//! when the full reference string is known in advance; use
//! [`simulate_replacement`] for that. Online translation with `Optimal`
//! falls back to LRU.

use std::collections::{BTreeMap, VecDeque};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Replacement {
    Fifo,
    Lru,
    Optimal,
}

impl Replacement {
    pub fn name(&self) -> &'static str {
        match self {
            Replacement::Fifo => "FIFO",
            Replacement::Lru => "LRU",
            Replacement::Optimal => "OPTIMAL",
        }
    }
}

#[derive(Debug)]
pub enum PagingError {
    UnknownProcess(u32),
    InvalidAccess { pid: u32, addr: u32, limit: u32 },
}

impl fmt::Display for PagingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PagingError::UnknownProcess(pid) => {
                write!(f, "P{pid} has no page table (attach one first)")
            }
            PagingError::InvalidAccess { pid, addr, limit } => write!(
                f,
                "P{pid}: invalid memory access at logical address {addr} (address space limit {limit})"
            ),
        }
    }
}

/// Per-process page table: page number -> resident frame.
struct PageTable {
    num_pages: u32,
    loaded: BTreeMap<u32, u32>,
}

struct FrameUse {
    pid: u32,
    page: u32,
}

pub struct PagedMemory {
    pub page_size: u32,
    pub total_frames: u32,
    frames: Vec<Option<FrameUse>>,
    tables: BTreeMap<u32, PageTable>,
    /// pid -> internal fragmentation (last-page waste) in bytes.
    page_waste: BTreeMap<u32, u32>,
    /// (pid, page) in load order, for FIFO victim selection.
    fifo: VecDeque<(u32, u32)>,
    /// (pid, page) -> last access stamp, for LRU victim selection.
    last_used: BTreeMap<(u32, u32), u64>,
    clock: u64,
    pub faults: u64,
    pub hits: u64,
}

impl PagedMemory {
    pub fn new(page_size: u32, total_frames: u32) -> Self {
        let frames = total_frames.max(1) as usize;
        PagedMemory {
            page_size: page_size.max(1),
            total_frames: frames as u32,
            frames: (0..frames).map(|_| None).collect(),
            tables: BTreeMap::new(),
            page_waste: BTreeMap::new(),
            fifo: VecDeque::new(),
            last_used: BTreeMap::new(),
            clock: 0,
            faults: 0,
            hits: 0,
        }
    }

    /// Create a page table for `pid` covering `size_bytes`. Re-attaching
    /// replaces the old table and frees its frames. Returns the page count.
    pub fn attach(&mut self, pid: u32, size_bytes: u32) -> u32 {
        self.detach(pid);
        let pages = size_bytes.div_ceil(self.page_size);
        self.tables.insert(pid, PageTable { num_pages: pages, loaded: BTreeMap::new() });
        self.page_waste.insert(pid, pages * self.page_size - size_bytes);
        pages
    }

    /// Remove the page table and release every frame held by `pid`.
    /// No-op if the pid has no table.
    pub fn detach(&mut self, pid: u32) {
        self.tables.remove(&pid);
        self.page_waste.remove(&pid);
        for f in self.frames.iter_mut() {
            if f.as_ref().is_some_and(|u| u.pid == pid) {
                *f = None;
            }
        }
        self.fifo.retain(|&(p, _)| p != pid);
        self.last_used.retain(|&(p, _), _| p != pid);
    }

    pub fn used_frames(&self) -> u32 {
        self.frames.iter().filter(|f| f.is_some()).count() as u32
    }

    pub fn internal_fragmentation(&self) -> u32 {
        self.page_waste.values().sum()
    }

    /// Translate a logical address for `pid`. On a miss, handles the page
    /// fault with the given replacement policy and counts faults/hits.
    /// Returns the physical address.
    pub fn translate(&mut self, pid: u32, addr: u32, algo: Replacement) -> Result<u32, PagingError> {
        let page_size = self.page_size;
        let page = addr / page_size;
        let offset = addr % page_size;
        {
            let table = self.tables.get(&pid).ok_or(PagingError::UnknownProcess(pid))?;
            if page >= table.num_pages {
                return Err(PagingError::InvalidAccess {
                    pid,
                    addr,
                    limit: table.num_pages * page_size,
                });
            }
        }
        self.clock += 1;
        let now = self.clock;

        if let Some(&frame) = self.tables[&pid].loaded.get(&page) {
            self.hits += 1;
            self.last_used.insert((pid, page), now);
            return Ok(frame * page_size + offset);
        }

        // Page fault: obtain a frame (free first, else evict).
        self.faults += 1;
        let frame = self.obtain_frame(algo);
        self.frames[frame as usize] = Some(FrameUse { pid, page });
        self.tables.get_mut(&pid).unwrap().loaded.insert(page, frame);
        self.fifo.push_back((pid, page));
        self.last_used.insert((pid, page), now);
        Ok(frame * page_size + offset)
    }

    /// Find a free frame, or evict a victim page and return its frame.
    fn obtain_frame(&mut self, algo: Replacement) -> u32 {
        if let Some(i) = self.frames.iter().position(|f| f.is_none()) {
            return i as u32;
        }
        let (vpid, vpage) = match algo {
            Replacement::Fifo => *self.fifo.front().expect("no resident pages"),
            Replacement::Lru | Replacement::Optimal => {
                // ponytail: online Optimal needs future references; falls back
                // to LRU here. Use simulate_replacement() for true Optimal.
                let (vpid, vpage) = self
                    .last_used
                    .iter()
                    .min_by_key(|(_, &stamp)| stamp)
                    .map(|(k, _)| *k)
                    .expect("no resident pages");
                (vpid, vpage)
            }
        };
        let frame = self.tables[&vpid].loaded[&vpage];
        self.frames[frame as usize] = None;
        self.tables.get_mut(&vpid).unwrap().loaded.remove(&vpage);
        self.fifo.retain(|&k| k != (vpid, vpage));
        self.last_used.remove(&(vpid, vpage));
        frame
    }

    /// Colorized frame table.
    pub fn display(&self) -> String {
        use crate::theme::{paint, pid_fg, CYAN, DIM, GREEN};
        let mut s = format!(
            "{}\n",
            paint(
                &format!(
                    "PAGING (page size {}, {} frames, {} used)",
                    self.page_size,
                    self.total_frames,
                    self.used_frames()
                ),
                &format!("{CYAN}\x1b[1m")
            )
        );
        s.push_str(&paint("FRAME  CONTENT", &format!("{CYAN}\x1b[1m")));
        s.push('\n');
        for (i, f) in self.frames.iter().enumerate() {
            let content = match f {
                Some(u) => paint(&format!("P{} page {}", u.pid, u.page), pid_fg(u.pid)),
                None => paint("FREE", GREEN),
            };
            s.push_str(&format!("{}\n", format!("{:<7}{}", paint(&i.to_string(), DIM), content)));
        }
        s.push_str(&format!(
            "{} {}  {} {}\n",
            paint("Page faults:", CYAN),
            paint(
                &self.faults.to_string(),
                if self.faults > 0 { "\x1b[31m" } else { DIM }
            ),
            paint("Page hits:", CYAN),
            paint(&self.hits.to_string(), GREEN)
        ));
        s.push_str(&format!(
            "{} {}\n",
            paint("Internal fragmentation:", CYAN),
            self.internal_fragmentation()
        ));
        s
    }
}

/// Offline page-replacement simulation over a known reference string.
/// Returns (faults, hits). This is where true Optimal is computed.
pub fn simulate_replacement(reference: &[u32], num_frames: usize, algo: Replacement) -> (u64, u64) {
    let mut frames: Vec<u32> = Vec::new(); // resident pages, load order
    let mut last_used: BTreeMap<u32, usize> = BTreeMap::new();
    let (mut faults, mut hits) = (0u64, 0u64);
    for (t, &page) in reference.iter().enumerate() {
        if frames.contains(&page) {
            hits += 1;
            if algo == Replacement::Lru {
                last_used.insert(page, t);
            }
            continue;
        }
        faults += 1;
        if frames.len() < num_frames {
            frames.push(page);
        } else {
            match algo {
                Replacement::Fifo => {
                    // Evict the oldest loaded page; push keeps load order.
                    let evicted = frames.remove(0);
                    frames.push(page);
                    last_used.remove(&evicted);
                }
                Replacement::Lru => {
                    let victim = (0..frames.len())
                        .min_by_key(|&i| last_used.get(&frames[i]).copied().unwrap_or(0))
                        .unwrap();
                    let evicted = frames[victim];
                    frames[victim] = page;
                    last_used.remove(&evicted);
                }
                Replacement::Optimal => {
                    let victim = (0..frames.len())
                        .max_by_key(|&i| {
                            reference[t + 1..]
                                .iter()
                                .position(|&p| p == frames[i])
                                .map(|j| j + t + 1)
                                .unwrap_or(usize::MAX) // never used again -> best victim
                        })
                        .unwrap();
                    let evicted = frames[victim];
                    frames[victim] = page;
                    last_used.remove(&evicted);
                }
            }
        }
        last_used.insert(page, t);
    }
    (faults, hits)
}
