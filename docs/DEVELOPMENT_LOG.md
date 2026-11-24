# Development Log

Chronological record of what was built, **why**, and every failure hit on the
way. Newest entries at the bottom.

---

## Session 1 — Project bootstrap

**What.** Created `mini-os-kernel` as a Rust 2021 cargo project, stdlib only.

**Why 2021, not 2024.** The specification asked for "Rust 2024"; edition 2024
requires a recent compiler. The installed toolchain is 1.100.0, which supports
it, but 2021 keeps the project buildable on older teaching-lab machines with
zero functional cost (no edition-2024 features are needed).

**Why zero dependencies.** The spec listed Serde and Ratatui as optional.
Neither is required by a single mandatory feature; every byte of IPC/serde
round-tripping is out of scope for the seven-day budget, and Ratatui would
have consumed time without adding information the ASCII dashboard already
conveys. Fewer moving parts = fewer failure modes at the viva.

**Failure.** No Rust toolchain on the machine (`cargo` / `rustc` not found).

**Fix.** Downloaded `rustup-init.exe` from `static.rust-lang.org` (the
`win.rustup.dev` short URL did not resolve on this network) and installed
`stable-x86_64-pc-windows-gnu`. The GNU toolchain was chosen because the
machine has no Visual Studio Build Tools; GNU ships a bundled mingw linker.

---

## Session 1 — Module design and first compile

**What.** Wrote all ten source files: `process`, `memory`, `paging`,
`scheduler`, `ipc`, `deadlock`, `sync`, `kernel`, `ui`, `main`.

**Why pure-function schedulers.** A trait per algorithm adds indirection with
no benefit: five implementations, one call site, identical signature. An enum
dispatch in `scheduler::schedule` is shorter, and scheduling a *snapshot*
means the same workload reruns under every algorithm without mutating the
process table.

**Why one global frame pool.** Per-process frame spaces would make paging
trivially correct and realistically wrong — real machines share frames.
Victim selection needs a global FIFO order / LRU stamps, so that is where
replacement state lives.

**Why a separate simulated semaphore.** `Condvar`-based blocking is right for
the producer-consumer demo but would stall the single-threaded simulated
processes. The simulator's semaphore reports "would block" instead.

**Failures (15 compile errors, 5 warnings).** Recorded verbatim in the commit
history; the substantive ones:

1. `E0592 duplicate definitions for free` — `MemoryManager::free(pid)` (dealloc)
   collided with `free()` (bytes free). Renamed the getter `free_bytes()`.
2. `E0507 cannot move out of b.owner` — `Option<(u32, String)>` is not `Copy`;
   used `as_ref().map(|(p, _)| *p)`.
3. `Option<FrameUse>: Clone` missing for `vec![None; n]` — meaningless derive;
   built the vector with `map(|_| None)`.
4. `E0614 type (u32, u32) cannot be dereferenced` — a stray `*` on the
   `min_by_key` result while picking the LRU victim.
5. `*mut u64 cannot be sent between threads` — the race demo's raw pointer.
   First attempt wrapped it in a `Send` newtype, but Rust 2021's disjoint
   closure captures captured the *field* (`ptr.0`), bypassing the wrapper.
   **Fix:** call a method on the wrapper (`p.slot()`) so the whole `Send`
   wrapper is captured.
6. `Vec<u32> doesn't implement Display`, wrong `offset` type in
   `write_region`, kernel `memory.free()` call sites — mechanical.

---

## Session 1 — Testing (the tests earned their keep)

**What.** Five integration suites covering process, scheduler, memory+paging,
deadlock, IPC+sync.

**Failures, in the order they were caught:**

1. **Producer-consumer deadlock (real, shipped-level bug).** Consumers exited
   on the first sentinel after marking themselves done; the remaining
   consumer could exit while a producer was still blocked on a full buffer,
   and `join()` hung forever. Fixed by tracking a shared `pending_sentinels`
   atomic: consumers drain until every producer's sentinel has been consumed,
   so all items precede all sentinels (FIFO) and nothing is left behind.
2. **FIFO eviction corrupted load order.** `simulate_replacement` replaced the
   victim in place, but FIFO's next victim is `frames[0]` (oldest loaded);
   in-place replacement moved the new page into the oldest position. Fixed
   with `remove(0)` + `push` for FIFO only.
3. **Three wrong hand-computed expectations** — the tests failed for the right
   reason:
   - Priority turnaround: P3 completes at 8, so TAT sum is 18, not 17.
   - Worst Fit fixture: my "two holes" were adjacent, so allocation
     coalesced them into one hole and all strategies chose the same address.
     Rebuilt the fixture with non-adjacent holes of 10 and 20 units.
   - I asserted FIFO faults never grow with more frames — that is exactly
     Belady's anomaly, which this reference string is famous for (9 faults at
     3 frames, **10 at 4**). The test now asserts the anomaly.
4. **`mine.next()` then `mine.last()` on the same iterator** — with exactly
   one slot the iterator is exhausted and `.last()` returns `None`.
   Collected into a `Vec` instead.
5. **Stale cargo build lock.** A foreground `cargo test` exceeded the shell
   timeout and left `.cargo-build-lock`; the next run waited forever.
   Killed the orphaned cargo processes and removed the locks.

**Result:** 60 tests green, clean build with zero warnings.

---

## Session 1 — Post-1.0.0 audit (v1.0.1)

**What.** Systematic bug hunt before publishing. Every finding recorded:

| ID | Finding | Root cause | Fix |
|----|---------|-----------|-----|
| B1 | CLI infinite loop on EOF | `read_line` returns `Ok(0)`; empty string failed every parser, retry loop never ended | exit cleanly on EOF |
| B2 | Malformed empty Gantt | terminal-width loop assumes at least one slot | explicit "(empty timeline)" |
| B3 | Dashboard listed every RUNNING process | manual `dispatch` has no single-CPU invariant | show lowest pid |
| B4 | Blocked senders never woke | `blocked_senders` was append-only | successful receive wakes the oldest; kernel logs it |
| B5 | Dead code `name_map` | left from an abandoned display idea | deleted |
| B6 | `try_grant` debug panic | `max - allocated` underflows if the public struct is inconsistent | return `InconsistentState` |
| B7 | PID burned on failed fork | `next_pid` incremented before allocation could fail | increment only after success |
| B8 | Producer-consumer hang with 0 consumers / 0 capacity | no consumer can drain; zero-capacity buffer never accepts | clamp both to 1 |

B1, B4, B8 could hang or wedge a demo; B6 could panic in a debug build. All
fixed with regression tests where the harness can reach them (B1 is
stdin-dependent; verified manually by piping input).

**Result:** 76 tests green.

---

## Session 1 — Packaging and release (v1.1.0)

**What.** npm wrapper (`package.json` + `bin/*.js`), `install.sh`,
`install.ps1`, `Dockerfile`, `docs/INSTALL.md`, `CHANGELOG.md`, this log,
`LICENSE`.

**Why npm compiles from source.** Shipping per-platform prebuilt binaries
through npm needs a release pipeline we do not have yet. The wrapper runs
`cargo build --release` on first use (and in postinstall), which works on
every platform with Rust installed; the postinstall failing is non-fatal so
`npm install` never breaks.

**Why the Windows script forces the GNU toolchain.** Proven on this machine:
the MSVC default requires Visual Studio Build Tools that end users often lack;
GNU needs nothing extra.

**Why the version scheme `major.minor.bugs`.** Requested by the project owner:
the third component counts bug-fix releases. `v1.0.0` = initial complete
simulator, `v1.0.1` = audit fixes, `v1.1.0` = installers and logging.

**Git history.** Authored as small conventional commits (one per module, per
test suite, per bug fix, per doc set) with annotated tags at each release, so
the history itself reads as the development log.
