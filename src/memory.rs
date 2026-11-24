//! Contiguous memory allocation: First Fit / Best Fit / Worst Fit over a
//! free-block list, with coalescing on free and an ASCII memory map.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FitStrategy {
    First,
    Best,
    Worst,
}

impl FitStrategy {
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
    pub start: u32,
    pub size: u32,
    pub owner: Option<(u32, String)>, // (pid, name)
}

#[derive(Debug)]
pub enum MemError {
    InsufficientMemory { requested: u32, available: u32 },
    NoAllocation(u32),
    DuplicateOwner(u32),
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

pub struct MemoryManager {
    pub total: u32,
    pub strategy: FitStrategy,
    blocks: Vec<Block>, // sorted by start
}

impl MemoryManager {
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
        if self.blocks.iter().any(|b| b.owner.as_ref().map(|(p, _)| *p) == Some(pid)) {
            return Err(MemError::DuplicateOwner(pid));
        }
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

    pub fn used(&self) -> u32 {
        self.blocks.iter().filter(|b| b.owner.is_some()).map(|b| b.size).sum()
    }

    pub fn free_bytes(&self) -> u32 {
        self.total - self.used()
    }

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

    pub fn blocks(&self) -> &[Block] {
        &self.blocks
    }

    /// ASCII memory map, e.g.
    /// ```text
    /// PHYSICAL MEMORY (total 1024)
    /// +----------------------+
    /// | P1           0-300   |
    /// +----------------------+
    /// | FREE       300-1024  |
    /// +----------------------+
    /// ```
    pub fn display(&self) -> String {
        let mut s = format!("PHYSICAL MEMORY (total {})\n", self.total);
        for b in &self.blocks {
            let label = match &b.owner {
                Some((_pid, name)) => format!("{name} {}-{}", b.start, b.start + b.size),
                None => format!("FREE {}-{}", b.start, b.start + b.size),
            };
            let label = truncate(&label, 22);
            s.push_str(&format!("+----------------------+\n"));
            s.push_str(&format!("|{:<22}|\n", label));
        }
        s.push_str("+----------------------+\n");
        s
    }
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        s.chars().take(n - 1).collect::<String>() + "~"
    }
}
