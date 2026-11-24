# Changelog

All notable changes to this project. Format based on
[Keep a Changelog](https://keepachangelog.com/). Versioning: `major.minor.bugs`
— the third number counts bug-fix releases.

## [1.1.0] - 2026-11-25

### Added
- npm package (`package.json`, `bin/mini-os-kernel.js`, `bin/install.js`) so the
  simulator installs with `npm install -g` / `npx`.
- One-command installers: `install.sh` (macOS/Linux) and `install.ps1`
  (Windows, GNU toolchain — no Visual Studio required).
- `Dockerfile` (multi-stage build) for containerized runs.
- `docs/INSTALL.md` — every installation path, registry publishing guide,
  troubleshooting.
- `docs/DEVELOPMENT_LOG.md` — chronological what/why log including failures.
- `LICENSE` (MIT).

### Changed
- README now leads with installation options.

## [1.0.1] - 2026-11-25

Bug-fix release from the post-1.0.0 audit. 8 fixes, each with a regression
test where testable.

### Fixed
- **npm**: the wrapper now locates cargo in `~/.cargo/bin` when PATH is stale
  (freshly installed Rust, terminal not restarted).
- **ui**: EOF on stdin (piped input, closed console) caused an infinite retry
  loop in `ask_u32`; the simulator now exits cleanly.
- **scheduler**: empty workload printed a malformed Gantt chart; now prints an
  explicit empty-timeline message.
- **kernel**: dashboard "CURRENT PROCESS" listed every RUNNING pid; reports a
  single current process.
- **ipc**: blocked senders were recorded but never cleared; a successful
  receive now wakes the longest-blocked sender and logs it.
- **deadlock**: `try_grant` underflowed (debug panic) when a max claim was below
  the allocation; now returns `InconsistentState`.
- **kernel**: a failed fork burned a pid; pids are consumed only on success.
- **sync**: producer-consumer deadlocked with `consumers = 0` or
  `capacity = 0`; degenerate parameters are clamped.
- **scheduler**: removed dead code (`name_map`).

### Added
- Regression tests: empty Gantt, single current process, blocked-sender wake,
  inconsistent deadlock state, failed-fork pid reuse, degenerate
  producer-consumer, plus the total test count rising 60 → 76.

## [1.0.0] - 2026-11-25

### Added
- Process management: PCB, five states with validated transitions, process
  table, event log.
- CPU scheduling: FCFS, SJF, SRTF, Round Robin, Priority; Gantt charts,
  average wait/turnaround/response, CPU utilization, throughput; algorithm
  comparison.
- Memory management: First/Best/Worst Fit contiguous allocation with
  coalescing; fragmentation reporting; ASCII memory map.
- Paging: page tables, logical→physical translation, FIFO/LRU/Optimal
  replacement, fault/hit counters, internal fragmentation.
- Synchronization: deterministic race demo, real-thread race demo with mutex
  fix, bounded producer-consumer (Mutex + Condvar), simulated semaphore.
- IPC: bounded message queues with blocked-sender tracking, shared memory
  regions with bounds checking.
- Deadlock: Banker's Algorithm with safe sequence, deadlock detection,
  safety-checked resource grants with rollback.
- System-call interface (fork/exit/wait/allocate/free/send/receive) routed
  through the kernel with validation and event logging.
- Kernel dashboard and terminal menus (plain CLI, zero dependencies).
- 60 tests covering every subsystem, including hand-computed schedules,
  Belady's reference string, and Silberschatz deadlock examples.
- `docs/architecture.md`, `docs/algorithms.md`.

[1.1.0]: https://github.com/Gunjan00001/Mini-OS-Kernel-Simulator/releases/tag/v1.1.0
[1.0.1]: https://github.com/Gunjan00001/Mini-OS-Kernel-Simulator/releases/tag/v1.0.1
[1.0.0]: https://github.com/Gunjan00001/Mini-OS-Kernel-Simulator/releases/tag/v1.0.0
