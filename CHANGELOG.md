# Changelog

All notable changes to this project. Format based on
[Keep a Changelog](https://keepachangelog.com/). Versioning: `major.minor.bugs`
— the third number counts bug-fix releases.

## [1.2.0] - 2026-11-25

The "make it not boring" release: the terminal UI is now colorized and much
more visual.

### Added
- `theme.rs`: zero-dependency color theme with Windows VT-mode enablement
  (two kernel32 calls), `NO_COLOR`/pipe detection, automatic fallback.
- **Per-pid colored Gantt charts** with legend; colored process states in
  tables; best-value highlighting in the algorithm comparison.
- Colorized memory map, paging frame table, dashboard with usage bar,
  deadlock verdicts, IPC queue capacity bars, sync results and event log.
- Box-drawn banner, `»` prompt, context strip on every menu
  (`tick · processes · scheduler · memory`), clear-screen on menu entry and
  "press Enter" pauses for long outputs (interactive only).
- Visual screenshot pipeline rewritten as ANSI → HTML → headless Chrome,
  producing the colorized PNGs in `docs/screenshots/`.

### Fixed
- Dashboard memory/paging sections lost line breaks in the first colorized
  build (format-string placeholder mismatch).
- Menu hint column collided with the longest option label.
- Screenshot renderer: GDI advance-width mismatches, PowerShell 5.1 UTF-8
  decoding, BOM-contaminated piped input, case-insensitive crop anchors and
  ANSI-prefixed anchor lines; all resolved and documented in the dev log.

## [1.1.1] - 2026-11-25

Screenshot release plus the bugs those screenshots exposed.

### Added
- Real terminal screenshots in `docs/screenshots/` (12 PNGs), generated from
  captured sessions by `scripts/capture-screenshots.ps1`.
- README screenshots section with the highlights plus a collapsible gallery.

### Fixed
- **kernel**: attaching a page table to a nonexistent pid is now rejected
  (`Kernel::attach_paging` validates through the process table).
- **deadlock**: state display printed debug-padded vectors
  (`[0               , 1, 0]`); rows are now compact `(0, 1, 0)`.
- **memory**: block labels repeated the pid (`P1 P1 0-300`).
- **ui**: the race demo claimed "updates lost" even when the nondeterministic
  race happened not to lose any; the note now reflects the actual run.
- **docs**: corrected the test count (68, not 76) across README, INSTALL,
  CHANGELOG and DEVELOPMENT_LOG.

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
  producer-consumer, plus the total test count rising 60 → 66.

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
