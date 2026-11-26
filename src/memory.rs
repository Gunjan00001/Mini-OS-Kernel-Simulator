//! Contiguous memory allocation: First Fit / Best Fit / Worst Fit over a
//! free-block list, with coalescing on free and an ASCII memory map.

use std::fmt;

/// Contiguous-allocation strategies exposed by the memory menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FitStrategy {
    /// First hole large enough (lowest address).
    First,
    /// Smallest hole large enough (leaves the least waste).
    Best,
    /// Largest hole (leaves the largest remainder).
    Worst,
}

impl FitStrategy {
    /// Uppercase label for screens, logs and the report.
    pub fn name(&self) -> &'static str {
        match self {
            FitStrategy::First => "FIRST FIT",
            FitStrategy::Best => "BEST FIT",
            FitStrategy::Worst => "WORST FIT",
        }
    }
}

/// One region of physical memory. `owner == None` means free (a hole).
#[derive(Debug, Clone)]
pub struct Block {
    /// First address in the region.
    pub start: u32,
    /// Number of contiguous units in the region.
    pub size: u32,
    /// `(pid, name)` when allocated, `None` when free.
    pub owner: Option<(u32, String)>,
}

/// Errors returned by the contiguous allocator.
#[derive(Debug)]
pub enum MemError {
    /// No single hole is large enough for the request.
    InsufficientMemory {
        /// Units requested.
        requested: u32,
        /// Total free units (may be split across holes).
        available: u32,
    },
    /// The pid holds no block to free.
    NoAllocation(u32),
    /// The pid already owns a block (one block per process).
    DuplicateOwner(u32),
    /// A zero-unit allocation is rejected.
    InvalidSize(u32),
}

impl fmt::Display for MemError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemError::InsufficientMemory { requested, available } => write!(
                f,
                "not enough memory: requested {requested}, only {available} free"
            ),
            MemError::NoAllocation(pid) => write!(f, "P{pid} holds no memory allocation"),
            MemError::DuplicateOwner(pid) => write!(f, "P{pid} already holds a memory block"),
            MemError::InvalidSize(size) => write!(f, "invalid allocation size: {size}"),
        }
    }
}

/// Contiguous allocator over one physical address space.
///
/// Invariant: `blocks` is sorted by `start`, covers `[0, total)` without gaps,
/// and adjacent free blocks are always coalesced. Allocation splits a hole;
/// freeing merges neighbouring holes.
pub struct MemoryManager {
    /// Size of the simulated physical memory, in abstract units.
    pub total: u32,
    /// Active allocation strategy; switchable at runtime.
    pub strategy: FitStrategy,
    blocks: Vec<Block>, // sorted by start
}

impl MemoryManager {
    /// Start with one free block covering the whole address space.
    pub fn new(total: u32) -> Self {
        MemoryManager {
            total,
            strategy: FitStrategy::First,
            blocks: vec![Block { start: 0, size: total, owner: None }],
        }
    }

    /// Allocate `size` units for `pid` using the active strategy.
    /// Returns the start address of the allocation.
    pub fn allocate(&mut self, pid: u32, name: &str, size: u32) -> Result<u32, MemError> {
        if size == 0 {
            return Err(MemError::InvalidSize(size));
        }
        // One contiguous block per process keeps the map easy to read and
        // mirrors the simplified allocator taught in the course.
        if self.blocks.iter().any(|b| b.owner.as_ref().map(|(p, _)| *p) == Some(pid)) {
            return Err(MemError::DuplicateOwner(pid));
        }
        // Candidate holes are collected first because the strategy choice
        // needs to compare them before mutating the block list.
        let candidates: Vec<usize> = self
            .blocks
            .iter()
            .enumerate()
            .filter(|(_, b)| b.owner.is_none() && b.size >= size)
            .map(|(i, _)| i)
            .collect();
        if candidates.is_empty() {
            return Err(MemError::InsufficientMemory { requested: size, available: self.free_bytes() });
        }
        let idx = match self.strategy {
            FitStrategy::First => candidates[0],
            FitStrategy::Best => *candidates
                .iter()
                .min_by_key(|&i| self.blocks[*i].size)
                .unwrap(),
            FitStrategy::Worst => *candidates
                .iter()
                .max_by_key(|&i| self.blocks[*i].size)
                .unwrap(),
        };
        let hole = &mut self.blocks[idx];
        let start = hole.start;
        let leftover = hole.size - size;
        hole.size = size;
        hole.owner = Some((pid, name.to_string()));
        // Split the hole: the allocation keeps the low address, the remainder
        // becomes the next (still sorted) free block.
        if leftover > 0 {
            self.blocks.insert(
                idx + 1,
                Block { start: start + size, size: leftover, owner: None },
            );
        }
        Ok(start)
    }

    /// Release every unit owned by `pid`, coalescing adjacent free blocks.
    /// Returns the number of units freed.
    pub fn free(&mut self, pid: u32) -> Result<u32, MemError> {
        let idx = self
            .blocks
            .iter()
            .position(|b| b.owner.as_ref().map(|(p, _)| *p) == Some(pid))
            .ok_or(MemError::NoAllocation(pid))?;
        let freed = self.blocks[idx].size;
        self.blocks[idx].owner = None;
        self.coalesce();
        Ok(freed)
    }

    /// Merge every run of adjacent free blocks into one.
    fn coalesce(&mut self) {
        let mut merged: Vec<Block> = Vec::with_capacity(self.blocks.len());
        for b in self.blocks.drain(..) {
            if let Some(last) = merged.last_mut() {
                if last.owner.is_none() && b.owner.is_none() {
                    last.size += b.size;
                    continue;
                }
            }
            merged.push(b);
        }
        self.blocks = merged;
    }

    /// Total units currently owned by a process.
    pub fn used(&self) -> u32 {
        self.blocks.iter().filter(|b| b.owner.is_some()).map(|b| b.size).sum()
    }

    /// Total units not owned by any process.
    pub fn free_bytes(&self) -> u32 {
        self.total - self.used()
    }

    /// Size of the biggest single hole (useful for external fragmentation).
    pub fn largest_free_hole(&self) -> u32 {
        self.blocks
            .iter()
            .filter(|b| b.owner.is_none())
            .map(|b| b.size)
            .max()
            .unwrap_or(0)
    }

    /// External fragmentation: free memory unusable by the largest pending
    /// request, i.e. `free() - largest_free_hole()` (scattered across small holes).
    pub fn external_fragmentation(&self) -> u32 {
        self.free_bytes() - self.largest_free_hole()
    }

    /// Read-only view of the block list used by tests and displays.
    pub fn blocks(&self) -> &[Block] {
        &self.blocks
    }

    /// ASCII memory map, e.g.
    /// ```text
    /// PHYSICAL MEMORY (total 1024)
    /// +----------------------+
    /// |P1 0-300              |
    /// +----------------------+
    /// |FREE 300-1024         |
    /// +----------------------+
    /// ```
    /// Owner rows are tinted by pid; free rows are dim green.
    pub fn display(&self) -> String {
        use crate::theme::{paint, pid_fg, CYAN, DIM, GREEN};
        let mut s = format!(
            "{}\n",
            paint(
                &format!("PHYSICAL MEMORY (total {})", self.total),
                &format!("{CYAN}\x1b[1m")
            )
        );
        let border = paint("+----------------------+", DIM);
        for b in &self.blocks {
            let (text, style) = match &b.owner {
                Some((pid, name)) => (
                    truncate(&format!("{name} {}-{}", b.start, b.start + b.size), 22),
                    pid_fg(*pid),
                ),
                None => (
                    truncate(&format!("FREE {}-{}", b.start, b.start + b.size), 22),
                    GREEN,
                ),
            };
            s.push_str(&format!("{border}\n|{}|\n", paint(&format!("{:<22}", text), style)));
        }
        s.push_str(&border);
        s.push('\n');
        s
    }
}

/// Shorten a label to `n` display characters, marking truncation with `~`.
fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        s.chars().take(n - 1).collect::<String>() + "~"
    }
}
