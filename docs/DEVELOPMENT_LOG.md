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

**Result:** 66 tests green.

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

---

## Session 1 — Packaging verification (found more bugs)

**What.** Exercised every install path end-to-end instead of trusting the
files.

**Failures:**

1. **npm wrapper could not find cargo.** `spawnSync("cargo")` relies on PATH;
   this shell (and any terminal opened before rustup ran) has no
   `~/.cargo/bin`. The wrapper reported "build failed" although Rust was
   installed. **Fix:** `bin/cargo.js` probes `$CARGO`, PATH, then the rustup
   default location. Re-tagged v1.1.0 after the fix (release had not been
   published yet).
2. **npm 11 blocks install scripts by default.** `npm install -g` warns that
   the postinstall is "not yet covered by allowScripts" and skips it. The
   pre-build is therefore an optimisation, not a requirement — the wrapper
   builds on first run. Documented; no workaround needed.
3. **Git remote was empty** (new repo, no refs) — `git ls-remote` printed
   nothing with exit 0 before the first push. Expected, noted so it is not
   mistaken for an auth failure.

**Verification performed:**
- Clean clone from GitHub → `cargo test` → 66 green (68 today, after the
  screenshot-session tests).
- `npm pack` → `npm install -g ./mini-os-kernel-simulator-1.1.0.tgz` →
  `mini-os-kernel` runs on this machine (postinstall skipped by npm 11, the
  first-run build path was exercised).
- `node bin/mini-os-kernel.js` with piped input → builds and exits cleanly.
- Three annotated tags (`v1.0.0`, `v1.0.1`, `v1.1.0`) and three GitHub
  releases; `v1.1.0` ships a Windows binary.

**Result:** 38 commits, 3 tags, 3 releases, CI workflow testing Linux/Windows/
macOS on every push.

---

## Session 2 — Screenshots, docs, and the bugs they exposed (v1.1.1)

**What.** Captured 12 real terminal sessions and rendered them to PNGs with a
committed PowerShell script; added a README screenshot gallery; re-audited
the code while reviewing the captures.

**Why screenshots from transcripts, not mock-ups.** The simulator is a text
application; a pipeline that replays scripted input through the real binary
and renders the captured text is reproducible and cannot drift from actual
behaviour. `scripts/capture-screenshots.ps1` regenerates every image.

**Failures / bugs found while preparing screenshots:**

1. Page tables could be attached to nonexistent pids — the memory menu called
   `paging.attach` directly, bypassing kernel validation. **Fix:**
   `Kernel::attach_paging` validates through the process table (with test).
2. Deadlock matrix rows printed debug-padded vectors
   (`[0               , 1, 0]`, caused by `{:<16?}`). **Fix:** compact
   `(0, 1, 0)` formatting (with test).
3. Memory block labels repeated the pid (`P1 P1 0-300`). **Fix:** label by
   process name only.
4. The race demo printed "updates lost" even in runs where the
   nondeterministic race happened to lose nothing. **Fix:** the note reflects
   the actual outcome.
5. **Docs claimed 76 tests; there were 66 (68 now).** A hand-counting error
   propagated into README, INSTALL, CHANGELOG and this log. Corrected
   everywhere; the lesson is to copy counts from `cargo test` output, never
   from memory.
6. Screenshot tooling itself: the menu color rule matched "Deadlock
   Management" (PowerShell `-match` is case-insensitive) and the dashboard
   crop stopped at the first internal blank line. Fixed with a case-sensitive
   error pattern and an explicit closing-border marker.

**Verification.** 68 tests green; every rendered PNG inspected; raw
transcripts retained in `target/shots` (git-ignored) for auditability.

---

## Session 3 — Colorized UI + visual overhaul (v1.2.0)

**What.** Added a color theme, restyled every screen, and rewrote the
screenshot pipeline so the repo images show the colors.

**Why zero-dependency colors.** ANSI escapes work everywhere; the only
platform barrier is Windows conhost, fixed by enabling
`ENABLE_VIRTUAL_TERMINAL_PROCESSING` through two direct kernel32 calls (no
`windows-sys`, no `crossterm`). Colors turn themselves off when stdout is
not a terminal, so tests and pipes see the same plain text as before — all
68 existing tests kept passing without modification.

**Why a screenshot HTML pipeline.** Re-rendering ANSI into a bitmap with GDI
turned into a font-metrics fight: GDI+ `MeasureString` over-reports
(12.6px vs 7.7px actual advance), GDI `TextRenderer` pads measurements,
bold variants change per-glyph advances, and any mismatch misplaces the
right border of a box further on the line. Converting the transcript to HTML
spans and screenshotting with headless Chrome removes the hand-rolled layout
math entirely.

**Failures, in order (all in this session):**

1. First color build: the dashboard's MEMORY/PAGING sections printed
   `MEMORYTotal : ...` — a format-string placeholder count mismatch. Rewrote
   the template with one pair of arguments per line.
2. Screenshot pipeline v2 (GDI): per-run x offsets drifted; measured char
   width was 12px while DrawString advanced 7.7px, pushing box borders
   ~50px right. Tried TextRenderer measurement (8.8px), then a fixed grid.
3. `Get-Content` mangled UTF-8 box characters (PowerShell 5.1 reads native
   output with the OEM codepage). Fixed by setting
   `[Console]::OutputEncoding = UTF8` in the capture script.
4. Setting `$OutputEncoding = UTF8` to fix #3 **prepended a BOM to piped
   stdin**, so the simulator read `\uFEFF9` and printed
   "Unknown option: 9". Removed that assignment; inputs are ASCII.
5. Crop anchors failed after colors were added: lines now start with ANSI
   escapes, and PowerShell `-match` is case-insensitive, so the anchor
   "KERNEL DASHBOARD" selected the menu item "8. Kernel Dashboard".
   Fixed by matching against ANSI-stripped text with `-cmatch`.
6. Box-drawing glyphs in **bold** spans have different advances, so the
   banner's right border drifted; borders are no longer bolded.
7. LCD subpixel rendering added color fringes to thin box lines; headless
   Chrome now runs with `--disable-lcd-text`.
8. The shade glyph `░` in progress bars falls back to a different font in
   some renderers and looks like noise; bars now use solid blocks + spaces.
9. Honestly-reported earlier mistake, still relevant here: the v1.1.1 docs
   claimed "76 tests" (actual 68). Counts in this session were verified with
   `Select-String -Pattern '^#\[test\]'`.

**Result:** 68 tests green, zero warnings, 12 colorized screenshots
regenerated, CI green on the push.

---

## Session 4 — Presentation, report and demo materials (v1.2.1)

**What.** Generated a 17 slide presentation (PPTX and PDF), an 11 page project
report (DOCX and PDF) and a live demo guide, all from the real screenshots
and measured results.

**How.** `scripts/make_docs.py` builds both documents with python-pptx and
python-docx; PDFs are exported through the installed PowerPoint and Word.
Keeping the generator committed means the documents can be rebuilt after any
UI change instead of drifting.

**Failure found while reviewing the report figures.** The paging frame table
rendered `0FREE` with no gap between the frame number and the content. The
format width was applied to the ANSI wrapped string, and the invisible escape
codes counted as characters, so no padding was produced. Fix: pad the number
first, then color it. This is the same class of bug as the earlier dashboard
line break regression: styling layered onto text must never participate in
width arithmetic.

**Verification.** All 68 tests still green, all 12 screenshots regenerated at
v1.2.0, both PDFs re-exported, slide deck inspected slide by slide after
PowerPoint export.

Documents live in `docs/presentation` (PPTX and PDF), `docs/report`
(DOCX and PDF) and `docs/DEMO_GUIDE.md`.


---

## Session 5 — Bug audit and launch video (v1.2.2)

**What.** Full audit pass (tests, clippy, edge cases) and a 22 second launch
video generated with Hyperframes from the real screenshots.

**Bug found.** The page replacement demo accepted a frame count of 0; the
offline simulator then tried to evict from an empty frame list and panicked.
Clamped to one frame and added a regression test that exercises FIFO, LRU and
Optimal, bringing the suite to 69. Also cleared all clippy warnings (nested
format strings, checked percentage arithmetic) and stopped dimming context
lines that already carry color accents.

**Brag video.** `brag-output/brag.mp4`: six scenes, 1920x1080 at 30fps, music
plus motion matched SFX, real UI frames from the SRTF Gantt, comparison table,
Banker's Algorithm and dashboard. Timing follows the bundled music cue sheet
(strong cues at 8.74 and 18.56 seconds). The poster frame is baked as frame 0.

**Verification.** 69 tests green, zero clippy warnings, video check passed
before rendering, poster and decoded frames inspected.

---

## Session 6 — Documentation and source-comment pass

**What.** Reviewed every module and added the comments a reviewer needs:
module headers, rustdoc on public structs/fields/functions, and inline
comments at the steps that encode an invariant — SRTF tie-breaking, hole
splitting/coalescing, FIFO load order, LRU stamps, victim bookkeeping,
blocked-sender wakeup and Banker's rollback. The integration tests now state
their scope and which textbook fixture they exercise. `README.md` gained a
"Source documentation" section and the `cargo doc` command;
`docs/architecture.md` gained a reading order.

**Why.** Comments explain *why* and which invariant holds, not what the code
already says. Anyone opening a module during review should understand the
model without tracing the entire call graph first.

**Docs refresh.** The report and slide deck now carry the course code
(24B41EC311) and team roll numbers; the report PDF was removed because it no
longer matched the edited DOCX. The Office files are the tracked sources and
PDFs are treated as export copies.

**Verification.** 73 tests green (four new edge-case tests: empty dashboard,
IPC summary, direct semaphore P/V, external fragmentation),
`cargo clippy --all-targets -- -D warnings` clean, `cargo doc --no-deps`
builds without warnings. No behaviour changed.
