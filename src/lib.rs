//! Mini OS Kernel Simulator.
//!
//! A discrete-event simulation of core OS kernel responsibilities:
//! process management, CPU scheduling, memory management, synchronization,
//! IPC, deadlock management, and a simulated system-call interface.
//!
//! Simulated processes are data structures advanced by ticks, not real
//! threads. Real threads are used only in `sync` where demonstrating
//! genuine concurrency is the point.

pub mod deadlock;
pub mod ipc;
pub mod kernel;
pub mod memory;
pub mod paging;
pub mod process;
pub mod scheduler;
pub mod sync;
pub mod ui;
