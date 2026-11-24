# Algorithms

## CPU scheduling

All algorithms run a deterministic tick simulation over a snapshot of
processes and emit CPU occupancy `Slot`s, merged into Gantt intervals.

- **FCFS** — arrival order, idle time skipped by jumping the clock to the
  next arrival.
- **SJF (non-preemptive)** — at each completion, pick the arrived process
  with the smallest burst (ties: earliest arrival, then pid).
- **SRTF (preemptive)** — every tick, pick the arrived process with the
  smallest remaining time; a longer-running job is preempted the tick a
  shorter one arrives. Classic example: P1(0,8), P2(1,4), P3(2,9), P4(3,5)
  yields timeline P1(0-1) P2(1-5) P4(5-10) P1(10-17) P3(17-26), avg wait 6.5.
- **Round Robin** — FIFO ready queue, run one quantum, arriving processes
  enqueue before the preempted one re-joins. Quantum is clamped to ≥ 1.
- **Priority** — non-preemptive, lowest priority value first (nice-like).

Metrics computed from the timeline: completion = last slot end;
turnaround = completion − arrival; wait = turnaround − burst;
response = first start − arrival; CPU utilization = busy time / makespan;
throughput = completed processes / makespan.

Verified against hand-computed workloads in `tests/scheduler_tests.rs`.

## Memory: contiguous allocation

Free-block list sorted by start address.

- **First Fit** — first hole ≥ request.
- **Best Fit** — smallest sufficient hole.
- **Worst Fit** — largest hole.
- **Free** — mark the block free, coalesce adjacent holes. External
  fragmentation is reported as free memory minus the largest hole.

## Memory: paging and replacement

- Address translation: `page = addr / page_size`, `offset = addr % page_size`,
  physical = `frame * page_size + offset`. Access beyond the page count is an
  invalid access error.
- On a miss (page fault): take a free frame, else evict.
  - **FIFO** — evict the oldest-loaded page (global load-order deque).
  - **LRU** — evict the page with the smallest last-access stamp.
  - **Optimal (offline)** — evict the page whose next reference is farthest
    in the future (never-again counts as infinity). Implemented in
    `simulate_replacement` over a full reference string.
- Reference check (Belady's string `1 2 3 4 1 2 5 1 2 3 4 5`, 3 frames):
  FIFO 9 faults, LRU 10, Optimal 7 — asserted in tests.

## Synchronization

- **Race condition** — two interleaved load/store sequences lose one update;
  the real-thread demo shows the same phenomenon with live threads and then
  shows the Mutex version producing the exact count.
- **Producer-consumer** — bounded buffer: producers wait on a Condvar while
  the queue is at capacity, consumers wait while empty; sentinel values
  terminate consumers. Test verifies zero lost/duplicated messages.
- **Semaphore** — counting P/V; `wait` on zero reports "would block"
  (simulated processes must not stall real threads).

## Deadlock management

- **Banker's Algorithm (avoidance)** — work/finish sweep: process i can
  finish when `max_i − allocated_i ≤ work`; on finish it releases its
  allocation. If all processes can finish, the state is safe and the finish
  order is a safe sequence. `try_grant` grants a request only if the
  resulting state is still safe (rolls back otherwise) — requests that would
  enter an unsafe state are denied, which is the core of avoidance.
- **Deadlock detection** — same sweep, but using the current request matrix
  instead of remaining claims. Processes that can never finish are
  deadlocked (a cycle in the resource-allocation graph). The classic
  Silberschatz example yields deadlocked processes {1, 2, 3, 4}.
