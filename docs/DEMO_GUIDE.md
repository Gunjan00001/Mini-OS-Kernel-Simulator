# Live Demo Guide for the Professor

A five minute walkthrough that shows every subsystem actually running.

## Before the demo

- Start from the repository root.
- Launch the simulator. Any of these works:

```sh
mini-os-kernel                # installed via npm on this machine
cargo run --release           # from a clone
npx mini-os-kernel-simulator  # npm wrapper without a global install
```

- Use Windows Terminal or any UTF-8 terminal at roughly 120 columns wide.
- Colors should appear automatically. If the terminal is old, the simulator
  falls back to plain text and still works.
- Backup: the generated screenshots in `docs/screenshots/` show the same
  screens. Keep `docs/presentation/MiniOSKernel_Simulator.pptx` open on a
  second window.

## The five minute script

Every answer below is a number you type at the menu prompt.

### 1. Processes (about 40 seconds)

Choose `1`, then `1` four times. Enter:

| Name | Arrival | Burst | Priority | Memory |
|---|---|---|---|---|
| P1 | 0 | 8 | 1 | 100 |
| P2 | 1 | 4 | 2 | 80 |
| P3 | 2 | 9 | 1 | 120 |
| P4 | 3 | 5 | 1 | 60 |

Then choose `6` to show the process table, and `8` to show the event log.

Talk track: "Each process has a PCB, and the kernel validates every state
change. NEW to READY, READY to RUNNING, RUNNING to WAITING or TERMINATED.
Illegal transitions are refused."

Try one refusal to prove validation: choose `3` and give pid `1` while P1 is
READY. The kernel refuses WAITING from READY.

Back with `9`, then `9` to the main menu.

### 2. Scheduling (about 90 seconds)

Choose `2`, then `3` to run SRTF.

Point at the colored Gantt chart, the legend, the metrics line and the per
process table. SRTF should produce average wait 6.50 and average turnaround
13.00 for this workload.

Run `2` (SJF), then `1` (FCFS) and let the numbers change. Finally choose
`6` to compare all five algorithms. The best value for each metric is shown
in green: SRTF wins wait and turnaround, Round Robin wins response time.

Talk track: "Schedulers are pure functions over a process snapshot, so the
same workload runs under all five policies without mutating the process
table."

### 3. Memory and paging (about 70 seconds)

Choose `3`, then `3` to show the memory map. The four processes own colored
blocks.

Choose `2` and free pid `2`. Show the freed hole in the map with `3`.

Choose `4` and switch to Best Fit, then allocate 150 units for pid `4` and
show the map again.

Choose `5`, attach a page table to pid `1` with size 20 bytes. Choose `6`,
translate logical address `5` with policy `2` (LRU). It faults on page 1 and
maps to a physical address. Translate `0` as well. Choose `7` to show the
frame table.

Finish with `8` and the reference string
`1 2 3 4 1 2 5 1 2 3 4 5` with 3 frames: FIFO 9 faults, LRU 10, Optimal 7.

Talk track: "Optimal needs the future, so it is computed offline over a known
string. The same string with 4 frames gives more FIFO faults, which is
Belady's anomaly."

### 4. Synchronization (about 40 seconds)

Choose `4`, then `2` for the real thread demo. Enter 4 threads and 100000
increments. The expected count is 400000. The racy result may come out short
because the data race is real; the mutex result is always exact. Run it
twice if the racy run happens to be exact.

Choose `3` to run the producer consumer pipeline and show that consumed
count equals expected count. No messages are lost.

### 5. IPC (about 40 seconds)

Choose `5`. Create a queue with capacity `2` using option `1`. Send three
messages as P1 to P2. The third send is refused and P1 is recorded as a
blocked sender. Choose `4` to show the queue with its capacity bar. Receive
one message with `3`, then show the queue again: the blocked sender has been
woken.

### 6. Deadlock (about 50 seconds)

Choose `6`. Option `1` loads the safe Banker's scenario. Option `5` runs the
algorithm and prints a safe sequence such as `[1, 3, 4, 0, 2]`.

Option `6`, request `1 0 2` for process index `1`. The request is granted and
the new state stays safe.

Option `2` loads the deadlocked scenario, and option `7` reports the
deadlocked processes `[1, 2, 3, 4]`.

### 7. Dashboard (about 20 seconds)

Back on the main menu, choose `8`. The dashboard shows the CPU status, the
scheduler, queues, memory usage bar, page fault counters and system time.
System time equals the number of logged kernel events.

Close with: "The repository has 68 automated tests, CI on three operating
systems, and zero external dependencies. The presentation and report are in
the docs folder."

## Likely professor questions

- **Why is this not a real kernel?** It never boots hardware. It models
  kernel decisions with deterministic ticks so results are reproducible and
  testable. Real threads are used only in the synchronization demos.
- **Where is the context switch?** The scheduler records CPU assignments and
  state moves in the event log. Run SRTF and open the event log to see
  `CPU assigned to P1` then `CPU assigned to P2`.
- **How do you know the numbers are right?** Hand computed timelines are
  asserted in tests: SRTF average wait 6.50, Belady counts 9, 10 and 7,
  Banker's safe sequence and the deadlock set `[1, 2, 3, 4]`.
- **What happens if memory runs out?** Fork fails before a pid is consumed
  and the error is reported. Allocation failure paths are tested.
- **What would you do with more time?** Save and load scenarios, an
  interactive resource graph and a full screen terminal dashboard.

## If the live demo fails

1. Show the matching screenshot on the presentation slides.
2. Rebuild quickly with `cargo run --release`.
3. As a last resort, `docker run it mini-os-kernel` from a built image.
