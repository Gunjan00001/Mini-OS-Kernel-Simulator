# Architecture

## Layering

```
                 USER INTERFACE (ui.rs)
                       |
                COMMAND INTERPRETER (menu dispatch)
                       |
                 KERNEL SIMULATOR (kernel.rs)
                       |
     +-----------+-----+-----+-----------+
     v           v           v           v
 PROCESS      SCHEDULER    MEMORY      PAGING
 (process.rs) (scheduler.rs) (memory.rs) (paging.rs)
     |
     +--------+------------------+
     v        v                  v
   IPC      DEADLOCK            SYNC
 (ipc.rs)  (deadlock.rs)     (sync.rs, demos only)
```

## Ownership model

One `Kernel` struct owns every subsystem. There is no global mutable state:

- `Kernel::processes: BTreeMap<u32, Process>` — the process table
- `Kernel::memory: MemoryManager` — contiguous allocator
- `Kernel::paging: PagedMemory` — page tables + global frame pool
- `Kernel::ipc: Ipc` — bounded queues + shared memory regions
- `Kernel::resources: ResourceState` — deadlock scenario state
- `Kernel::events: Vec<Event>` — the centralized event log

Errors surface through one type, `KernelError`, with `From` conversions from
each subsystem's error enum. Every public kernel operation returns
`Result` and logs an event; every event advances the simulation tick.

## System-call layer

System calls are methods on `Kernel` (`fork_process`, `exit_process`,
`wait_process`, `allocate_memory`, `free_memory`, `send_message`,
`receive_message`). Each one validates arguments (unknown pid, bad state,
insufficient memory, full queue), delegates to the subsystem, and logs.
This models the user-program → kernel boundary without pretending to be
real ring transitions.

## Simulated processes vs real threads

- **Simulated processes** are PCB structs. Time advances in discrete ticks;
  no sleeps, no threads. The scheduler runs on immutable snapshots, so the
  same workload can be re-scheduled under every algorithm and compared
  without mutating the process table.
- **Real threads** appear only in `sync.rs`: the data-race demo (raw pointer
  with no synchronization, joined before free) and the bounded
  producer-consumer pipeline (`Arc<Mutex<VecDeque>>` + `Condvar`). These
  demonstrate concurrency that only exists with real parallelism.
- The simulated semaphore reports "would block" instead of actually
  blocking, keeping the simulator deterministic.

## Design decisions

- **Schedulers are pure functions** `(&[Process], Algorithm) -> ScheduleResult`.
  No trait hierarchy, no shared mutable scheduler state. Each algorithm
  advances its own tick loop; `merge_slots` coalesces per-tick slices into
  Gantt intervals. Metrics derive from the timeline, so every algorithm is
  measured identically.
- **One global frame pool for paging** (realistic), with FIFO load-order
  deque and LRU stamps kept per (pid, page). Optimal replacement is computed
  offline via `simulate_replacement` over a known reference string; online
  translation with Optimal falls back to LRU (documented ceiling).
- **Contiguous allocator** keeps a start-sorted block list; free coalesces
  adjacent holes. Strategy is a field, switchable at runtime.
- **Banker's/detection** share one work-finish sweep; Banker's uses
  remaining claims, detection uses the request matrix. `try_grant` performs
  a tentative grant and rolls back if the resulting state is unsafe.
- **CLI over Ratatui**: same information, zero dependency risk.
