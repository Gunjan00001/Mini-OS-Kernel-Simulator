//! Terminal interface: colorized menus, tables and dashboard.
//!
//! Colors are applied through `crate::theme` and automatically disappear when
//! stdout is not a terminal, so piped output and tests stay plain text.

use std::io::{self, IsTerminal, Write};

use crate::deadlock;
use crate::kernel::{Kernel, KernelError};
use crate::memory::FitStrategy;
use crate::paging::Replacement;
use crate::process::State;
use crate::scheduler::{self, Algorithm};
use crate::sync;
use crate::theme::{self, paint, BOLD, CYAN, DIM, GREEN, RED, YELLOW};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const INNER: usize = 54;

// ---------- chrome ----------

fn clear_screen() {
    if theme::interactive() {
        print!("\x1b[2J\x1b[H");
        let _ = io::stdout().flush();
    }
}

fn banner() {
    let edge = paint(&format!("╔{}╗", "═".repeat(INNER)), CYAN);
    let bottom = paint(&format!("╚{}╝", "═".repeat(INNER)), CYAN);
    let row = |text: &str, style: &str| {
        println!("║{}║", paint(&format!("{:<INNER$}", text, INNER = INNER), style));
    };
    println!("{edge}");
    let title = "  MINI OS KERNEL SIMULATOR".to_string();
    println!("║{}║", paint(&format!("{title:<INNER$}"), CYAN));
    row("  processes · scheduling · memory · sync", DIM);
    println!("║{}║", paint(&format!("  v{VERSION:<51}"), DIM));
    println!("{bottom}");
}

fn screen(title: &str, context: &str) {
    clear_screen();
    println!();
    println!("  {}", paint(&format!("── {title} "), &format!("{BOLD}{CYAN}")));
    println!("  {}", paint(context, DIM));
    println!();
}

fn options(items: &[(&str, &str, &str)]) {
    for (key, label, hint) in items {
        println!(
            "   {}. {:<30} {}",
            paint(key, &format!("{BOLD}{YELLOW}")),
            label,
            paint(hint, DIM)
        );
    }
    println!();
}

fn prompt(text: &str) -> String {
    paint(&format!("» {text}:"), &format!("{BOLD}{GREEN}"))
}

fn ok(msg: &str) {
    println!("  {} {}", paint("OK", &format!("{BOLD}{GREEN}")), msg);
}

fn info(msg: &str) {
    println!("  {}", msg);
}

fn pause() {
    if io::stdin().is_terminal() {
        ask(&paint("- press Enter to continue -", DIM));
    }
}

fn report<T>(r: Result<T, KernelError>) -> Option<T> {
    match r {
        Ok(v) => Some(v),
        Err(e) => {
            println!("  {} {}", paint("!!", &format!("{BOLD}{RED}")), paint(&e.to_string(), RED));
            None
        }
    }
}

fn event_style(msg: &str) -> &'static str {
    if msg.contains("created") || msg.contains("READY") {
        GREEN
    } else if msg.contains("terminated") {
        RED
    } else if msg.contains("CPU assigned") || msg.contains("Scheduler") {
        YELLOW
    } else if msg.contains("memory") || msg.contains("Memory") || msg.contains("page table") {
        CYAN
    } else if msg.contains("sent") || msg.contains("received") || msg.contains("woken") {
        "\x1b[35m"
    } else {
        ""
    }
}

fn print_event_log(k: &Kernel) {
    println!("  {}", paint("EVENT LOG", &format!("{BOLD}{CYAN}")));
    for e in k.events() {
        println!(
            "  {} {}",
            paint(&format!("[{:>3}]", e.tick), DIM),
            paint(&e.msg, event_style(&e.msg))
        );
    }
}

// ---------- input helpers ----------

pub fn ask(prompt_text: &str) -> String {
    print!("{} ", prompt_text);
    io::stdout().flush().unwrap();
    let mut line = String::new();
    match io::stdin().read_line(&mut line) {
        // EOF (piped input exhausted / Ctrl+Z): no more commands can arrive,
        // so exit instead of retrying forever on an empty string.
        Ok(0) => {
            println!("\nInput closed (EOF). Exiting simulator.");
            std::process::exit(0);
        }
        Ok(_) => {
            let input = line.trim().to_string();
            if !io::stdin().is_terminal() {
                // Scripted/piped input is not echoed by the terminal; echo it
                // so captured transcripts read like a real session.
                println!("{}", paint(&input, CYAN));
            }
            input
        }
        Err(e) => {
            eprintln!("\nFailed to read input: {e}");
            std::process::exit(1);
        }
    }
}

pub fn ask_u32(prompt_text: &str) -> u32 {
    loop {
        match ask(prompt_text).parse() {
            Ok(v) => return v,
            Err(_) => println!("  {}", paint("Please enter a whole number.", RED)),
        }
    }
}

fn ask_u32_or(prompt_text: &str, default: u32) -> u32 {
    let s = ask(&format!("{prompt_text} [{}]", paint(&default.to_string(), YELLOW)));
    s.parse().unwrap_or(default)
}

fn ask_vec_u32(prompt_text: &str, n: usize) -> Vec<u32> {
    loop {
        let line = ask(&format!("{prompt_text} ({} values, space separated)", n));
        let vals: Vec<u32> = line.split_whitespace().filter_map(|w| w.parse().ok()).collect();
        if vals.len() == n {
            return vals;
        }
        println!("  {}", paint(&format!("Need exactly {} values.", n), RED));
    }
}

fn usage_bar(k: &Kernel) -> String {
    theme::bar(k.memory.used(), k.memory.total, 20)
}

// ---------- main menu ----------

pub fn main_menu(kernel: &mut Kernel) {
    kernel.echo_events = true;
    loop {
        clear_screen();
        banner();
        println!();
        println!(
            "  {}",
            paint(
                &format!(
                    "tick {}  ·  {} processes  ·  scheduler {}",
                    kernel.tick,
                    kernel.processes.len(),
                    kernel.scheduler_algo.label()
                ),
                DIM
            )
        );
        println!();
        options(&[
            ("1", "Process Management", "fork, exit, states, event log"),
            ("2", "CPU Scheduling", "five algorithms, Gantt, compare"),
            ("3", "Memory Management", "allocators, paging, replacement"),
            ("4", "Process Synchronization", "race demos, producer-consumer"),
            ("5", "Inter-Process Communication", "queues, shared memory"),
            ("6", "Deadlock Management", "Banker's algorithm, detection"),
            ("7", "System Call Interface", "kernel entry points"),
            ("8", "Kernel Dashboard", "live system overview"),
            ("9", "Exit", ""),
        ]);
        match ask(&prompt("Select an option")).as_str() {
            "1" => process_menu(kernel),
            "2" => schedule_menu(kernel),
            "3" => memory_menu(kernel),
            "4" => sync_menu(),
            "5" => ipc_menu(kernel),
            "6" => deadlock_menu(kernel),
            "7" => syscall_menu(kernel),
            "8" => {
                clear_screen();
                println!();
                println!("{}", kernel.dashboard());
                pause();
            }
            "9" | "q" | "Q" => break,
            other => println!("  {}", paint(&format!("Unknown option: {other}"), RED)),
        }
    }
    println!(
        "\n{}",
        paint(
            &format!("Kernel simulator shutting down. Final tick: {}", kernel.tick),
            DIM
        )
    );
}

// ---------- 1. process management ----------

fn process_menu(k: &mut Kernel) {
    loop {
        let ctx = format!(
            "tick {}  ·  {} processes  ·  mem {}  ·  {}",
            k.tick,
            k.processes.len(),
            usage_bar(k),
            paint(
                &format!("{} frames used", k.paging.used_frames()),
                DIM
            )
        );
        screen("PROCESS MANAGEMENT", &ctx);
        options(&[
            ("1", "Create process (fork)", "PID, name, arrival, burst, priority, memory"),
            ("2", "Terminate process (exit)", "frees its memory"),
            ("3", "Block process (wait)", "RUNNING -> WAITING"),
            ("4", "Wake process", "WAITING -> READY"),
            ("5", "Dispatch process", "READY -> RUNNING"),
            ("6", "List all processes", ""),
            ("7", "Show process details", ""),
            ("8", "Show event log", ""),
            ("9", "Back", ""),
        ]);
        match ask(&prompt("Select an option")).as_str() {
            "1" => {
                let name = ask("Process name");
                let arrival = ask_u32("Arrival time");
                let burst = ask_u32("Burst time");
                let priority = ask_u32_or("Priority (lower = higher)", 5);
                let mem = ask_u32("Memory requirement (0 = none)");
                if let Some(pid) = report(k.fork_process(&name, arrival, burst, priority, mem)) {
                    ok(&format!(
                        "created {} (PID: {})",
                        paint(&name, theme::pid_fg(pid)),
                        paint(&pid.to_string(), &format!("{BOLD}{YELLOW}"))
                    ));
                }
            }
            "2" => {
                let pid = ask_u32("PID to terminate");
                if report(k.exit_process(pid)).is_some() {
                    ok(&format!("P{} terminated", pid));
                }
            }
            "3" => {
                let pid = ask_u32("PID to block");
                report(k.wait_process(pid));
            }
            "4" => {
                let pid = ask_u32("PID to wake");
                report(k.wake_process(pid));
            }
            "5" => {
                let pid = ask_u32("PID to dispatch");
                report(k.dispatch_process(pid));
            }
            "6" => {
                list_processes(k);
                pause();
            }
            "7" => {
                let pid = ask_u32("PID to inspect");
                match k.processes.get(&pid) {
                    Some(p) => {
                        println!();
                        println!(
                            "  {}",
                            paint(&format!("PROCESS {}", p.name), &format!("{BOLD}{CYAN}"))
                        );
                        let row = |label: &str, value: String| {
                            println!("  {} {value}", paint(&format!("{label:<12}:"), CYAN));
                        };
                        row("PID", paint(&p.pid.to_string(), theme::pid_fg(p.pid)));
                        row("State", paint(&p.state.to_string(), theme::state_style(p.state)));
                        row("Arrival", p.arrival.to_string());
                        row("Burst", p.burst.to_string());
                        row("Remaining", p.remaining.to_string());
                        row("Priority", p.priority.to_string());
                        row("Memory", format!("{} units", p.mem_req));
                        row("CPU time", p.cpu_time.to_string());
                        row("Wait time", p.wait_time.to_string());
                        row("Turnaround", p.turnaround.to_string());
                    }
                    None => println!("  {}", paint(&format!("✗ no such process (pid {pid})"), RED)),
                }
                pause();
            }
            "8" => {
                print_event_log(k);
                pause();
            }
            "9" => break,
            other => println!("  {}", paint(&format!("Unknown option: {other}"), RED)),
        }
    }
}

fn list_processes(k: &Kernel) {
    if k.processes.is_empty() {
        println!("  {}", paint("No processes yet. Create one first (option 1).", DIM));
        return;
    }
    println!(
        "  {}",
        paint(
            &format!(
                "{:<4} {:<12} {:<11} {:>4} {:>5} {:>4} {:>5} {:>5}",
                "PID", "NAME", "STATE", "ARR", "BURST", "REM", "PRIO", "MEM"
            ),
            &format!("{BOLD}{CYAN}")
        )
    );
    for p in k.processes.values() {
        println!(
            "  {:<4} {:<12} {:<11} {:>4} {:>5} {:>4} {:>5} {:>5}",
            paint(&p.pid.to_string(), theme::pid_fg(p.pid)),
            p.name,
            paint(&format!("{:<11}", p.state), theme::state_style(p.state)),
            p.arrival,
            p.burst,
            p.remaining,
            p.priority,
            p.mem_req
        );
    }
}

// ---------- 2. cpu scheduling ----------

fn schedule_menu(k: &mut Kernel) {
    const QUANTUM: u32 = 2;
    loop {
        screen(
            "CPU SCHEDULING",
            &format!("current policy: {}", paint(&k.scheduler_algo.label(), YELLOW)),
        );
        options(&[
            ("1", "FCFS", "non-preemptive, arrival order"),
            ("2", "SJF", "non-preemptive, shortest job first"),
            ("3", "SRTF", "preemptive, shortest remaining time"),
            ("4", "Round Robin", "preemptive, time quantum"),
            ("5", "Priority", "non-preemptive, lower value wins"),
            ("6", "Compare all algorithms", "same workload, side by side"),
            ("7", "Back", ""),
        ]);
        match ask(&prompt("Select an option")).as_str() {
            "1" => run_and_show(k, Algorithm::Fcfs),
            "2" => run_and_show(k, Algorithm::Sjf),
            "3" => run_and_show(k, Algorithm::Srtf),
            "4" => {
                let q = ask_u32_or("Time quantum", QUANTUM);
                run_and_show(k, Algorithm::RoundRobin { quantum: q });
            }
            "5" => run_and_show(k, Algorithm::Priority),
            "6" => compare_all(k, ask_u32_or("Time quantum for RR", QUANTUM)),
            "7" => break,
            other => println!("  {}", paint(&format!("Unknown option: {other}"), RED)),
        }
    }
}

fn run_and_show(k: &mut Kernel, algo: Algorithm) {
    if report(k.run_schedule(algo)).is_none() {
        return;
    }
    let result = k.last_schedule.as_ref().unwrap();
    let procs: Vec<_> = k.processes.values().cloned().collect();
    println!();
    println!("  {}", paint(&format!("=== {} ===", algo.label()), &format!("{BOLD}{CYAN}")));
    for line in scheduler::gantt(result, &procs).lines() {
        println!("  {line}");
    }
    print_legend(result, &procs);
    show_metrics(result);
    show_stats(result, &procs);
    pause();
}

fn print_legend(result: &scheduler::ScheduleResult, procs: &[crate::process::Process]) {
    let mut seen: Vec<u32> = Vec::new();
    for s in &result.slots {
        if !seen.contains(&s.pid) {
            seen.push(s.pid);
        }
    }
    let names: Vec<String> = seen
        .iter()
        .map(|pid| {
            let name = procs
                .iter()
                .find(|p| p.pid == *pid)
                .map(|p| p.name.clone())
                .unwrap_or_else(|| format!("P{pid}"));
            paint(&name, &format!("{}{}", BOLD, theme::pid_fg(*pid)))
        })
        .collect();
    println!("  {} {}", paint("legend:", DIM), names.join("  "));
}

fn show_metrics(result: &scheduler::ScheduleResult) {
    let m = &result.metrics;
    let metric = |label: &str, value: String| format!("{} {}", paint(label, DIM), paint(&value, YELLOW));
    println!(
        "  {}   {}   {}   {}   {}",
        metric("wait", format!("{:.2}", m.avg_wait)),
        metric("turnaround", format!("{:.2}", m.avg_turnaround)),
        metric("response", format!("{:.2}", m.avg_response)),
        metric("CPU", format!("{:.1}%", m.cpu_util_pct)),
        metric("throughput", format!("{:.3}/unit", m.throughput)),
    );
}

fn show_stats(result: &scheduler::ScheduleResult, procs: &[crate::process::Process]) {
    println!(
        "  {}",
        paint(
            &format!(
                "{:<4} {:>5} {:>5} {:>6} {:>6} {:>6} {:>6} {:>6}",
                "PID", "ARR", "BURST", "START", "FIN", "WAIT", "TAT", "RESP"
            ),
            &format!("{BOLD}{CYAN}")
        )
    );
    for s in scheduler::process_stats(result, procs) {
        println!(
            "  {:<4} {:>5} {:>5} {:>6} {:>6} {:>6} {:>6} {:>6}",
            paint(&s.pid.to_string(), theme::pid_fg(s.pid)),
            s.arrival,
            s.burst,
            s.first_start,
            s.finish,
            paint(&s.wait.to_string(), YELLOW),
            s.turnaround,
            s.response
        );
    }
}

fn compare_all(k: &mut Kernel, quantum: u32) {
    let procs: Vec<crate::process::Process> = k
        .processes
        .values()
        .filter(|p| p.state != State::Terminated)
        .cloned()
        .collect();
    if procs.is_empty() {
        println!("  {}", paint("No processes to schedule. Create some first.", DIM));
        return;
    }
    let results: Vec<scheduler::ScheduleResult> = Algorithm::all(quantum)
        .iter()
        .map(|algo| scheduler::schedule(&procs, *algo))
        .collect();

    let min_of = |f: fn(&scheduler::Metrics) -> f64| {
        results
            .iter()
            .map(|r| f(&r.metrics))
            .fold(f64::INFINITY, f64::min)
    };
    let max_of = |f: fn(&scheduler::Metrics) -> f64| {
        results
            .iter()
            .map(|r| f(&r.metrics))
            .fold(f64::NEG_INFINITY, f64::max)
    };
    let best_wait = min_of(|m| m.avg_wait);
    let best_tat = min_of(|m| m.avg_turnaround);
    let best_resp = min_of(|m| m.avg_response);
    let best_util = max_of(|m| m.cpu_util_pct);
    let best_tp = max_of(|m| m.throughput);

    println!();
    println!(
        "  {}",
        paint(
            &format!("=== COMPARISON on {} processes ===", procs.len()),
            &format!("{BOLD}{CYAN}")
        )
    );
    println!(
        "  {}",
        paint(
            &format!(
                "{:<22} {:>9} {:>11} {:>9} {:>8} {:>11}",
                "ALGORITHM", "WAIT", "TURNAROUND", "RESPONSE", "UTIL%", "THROUGHPUT"
            ),
            &format!("{BOLD}{CYAN}")
        )
    );
    let cell = |value: f64, best: f64, width: usize| {
        let text = format!("{:>width$.2}", value, width = width);
        if (value - best).abs() < 1e-9 {
            paint(&text, &format!("{BOLD}{GREEN}"))
        } else {
            text
        }
    };
    for r in &results {
        let m = &r.metrics;
        println!(
            "  {} {} {} {} {} {}",
            paint(&format!("{:<22}", r.algo.label()), CYAN),
            cell(m.avg_wait, best_wait, 9),
            cell(m.avg_turnaround, best_tat, 11),
            cell(m.avg_response, best_resp, 9),
            cell(m.cpu_util_pct, best_util, 8),
            cell(m.throughput, best_tp, 11),
        );
    }
    println!("  {}", paint("green = best value for that metric", DIM));
    for r in &results {
        println!();
        println!("  {}", paint(&format!("--- {} ---", r.algo.label()), &format!("{BOLD}{CYAN}")));
        for line in scheduler::gantt(r, &procs).lines() {
            println!("  {line}");
        }
        print_legend(r, &procs);
    }
    pause();
}

// ---------- 3. memory management ----------

fn memory_menu(k: &mut Kernel) {
    loop {
        screen(
            "MEMORY MANAGEMENT",
            &format!(
                "strategy {}  ·  used {}  ·  free {}",
                paint(k.memory.strategy.name(), YELLOW),
                k.memory.used(),
                k.memory.free_bytes()
            ),
        );
        options(&[
            ("1", "Allocate memory", "contiguous block for a process"),
            ("2", "Free memory", "release a process's block"),
            ("3", "Show memory map", "blocks, fragmentation, frames"),
            ("4", "Set allocation strategy", "First / Best / Worst Fit"),
            ("5", "Paging: attach page table", ""),
            ("6", "Paging: translate address", "logical -> physical, faults"),
            ("7", "Paging: show frame table", ""),
            ("8", "Page replacement demo", "FIFO vs LRU vs Optimal"),
            ("9", "Back", ""),
        ]);
        match ask(&prompt("Select an option")).as_str() {
            "1" => {
                let pid = ask_u32("PID");
                let size = ask_u32("Size (units)");
                if let Some(start) = report(k.allocate_memory(pid, size)) {
                    ok(&format!("allocated {} units at address {}", size, start));
                }
            }
            "2" => {
                let pid = ask_u32("PID");
                if let Some(freed) = report(k.free_memory(pid)) {
                    ok(&format!("freed {} units", freed));
                }
            }
            "3" => {
                println!();
                println!("{}", k.memory.display());
                println!(
                    "  {} {}  {} {}  {} {}  {} {}",
                    paint("usage", DIM),
                    usage_bar(k),
                    paint("used", DIM),
                    paint(&k.memory.used().to_string(), YELLOW),
                    paint("free", DIM),
                    paint(&k.memory.free_bytes().to_string(), GREEN),
                    paint("largest hole", DIM),
                    k.memory.largest_free_hole(),
                );
                println!(
                    "  {} {}",
                    paint("external fragmentation", DIM),
                    paint(
                        &k.memory.external_fragmentation().to_string(),
                        if k.memory.external_fragmentation() > 0 { YELLOW } else { GREEN }
                    )
                );
                println!();
                println!("{}", k.paging.display());
                pause();
            }
            "4" => {
                println!();
                println!(
                    "  1. First Fit    2. Best Fit    3. Worst Fit"
                );
                k.memory.strategy = match ask(&prompt("Strategy")).as_str() {
                    "2" => FitStrategy::Best,
                    "3" => FitStrategy::Worst,
                    _ => FitStrategy::First,
                };
                ok(&format!("strategy set to {}", k.memory.strategy.name()));
            }
            "5" => {
                let pid = ask_u32("PID");
                let size = ask_u32("Address space size (bytes)");
                if let Some(pages) = report(k.attach_paging(pid, size)) {
                    ok(&format!(
                        "P{}: page table created, {} pages (page size {})",
                        pid, pages, k.paging.page_size
                    ));
                }
            }
            "6" => {
                let pid = ask_u32("PID");
                let addr = ask_u32("Logical address");
                let algo = pick_replacement();
                match k.paging.translate(pid, addr, algo) {
                    Ok(phys) => println!(
                        "  {} logical {} {} physical {}   {} {}  {} {}",
                        paint("»", GREEN),
                        paint(&addr.to_string(), YELLOW),
                        paint("->", DIM),
                        paint(&phys.to_string(), GREEN),
                        paint("faults", DIM),
                        paint(
                            &k.paging.faults.to_string(),
                            if k.paging.faults > 0 { YELLOW } else { DIM }
                        ),
                        paint("hits", DIM),
                        paint(&k.paging.hits.to_string(), GREEN)
                    ),
                    Err(e) => println!("  {} {}", paint("!!", &format!("{BOLD}{RED}")), paint(&KernelError::from(e).to_string(), RED)),
                }
            }
            "7" => {
                println!();
                println!("{}", k.paging.display());
                pause();
            }
            "8" => {
                let frames = ask_u32_or("Number of frames", 3);
                let line = ask("Reference string (page numbers, space separated)");
                let reference: Vec<u32> = line.split_whitespace().filter_map(|w| w.parse().ok()).collect();
                if reference.is_empty() {
                    println!("  {}", paint("Empty reference string.", RED));
                    continue;
                }
                println!();
                for (algo, style) in [
                    (Replacement::Fifo, YELLOW),
                    (Replacement::Lru, CYAN),
                    (Replacement::Optimal, GREEN),
                ] {
                    let (faults, hits) = crate::paging::simulate_replacement(&reference, frames as usize, algo);
                    println!(
                        "  {:>8}: {} {} {} {}",
                        paint(algo.name(), style),
                        paint(&faults.to_string(), &format!("{BOLD}{style}")),
                        paint("faults,", DIM),
                        paint(&hits.to_string(), DIM),
                        paint("hits", DIM)
                    );
                }
                println!("  {}", paint("(Optimal knows the future: offline simulation)", DIM));
                pause();
            }
            "9" => break,
            other => println!("  {}", paint(&format!("Unknown option: {other}"), RED)),
        }
    }
}

fn pick_replacement() -> Replacement {
    match ask(&prompt("Policy: 1 = FIFO, 2 = LRU, 3 = Optimal")).as_str() {
        "2" => Replacement::Lru,
        "3" => Replacement::Optimal,
        _ => Replacement::Fifo,
    }
}

// ---------- 4. synchronization ----------

fn sync_menu() {
    loop {
        screen(
            "PROCESS SYNCHRONIZATION",
            "real threads for the demos · simulated semaphore for logical processes",
        );
        options(&[
            ("1", "Race condition (interleaving)", "deterministic lost update"),
            ("2", "Race condition (real threads)", "racy vs mutex-fixed count"),
            ("3", "Producer-consumer", "bounded buffer, no lost messages"),
            ("4", "Semaphore walkthrough", "P/V, mutual exclusion"),
            ("5", "Back", ""),
        ]);
        match ask(&prompt("Select an option")).as_str() {
            "1" => {
                let (result, correct) = sync::race_demo_interleaved();
                println!();
                println!("  Two processes each run `counter += 1` on a shared counter starting at 0.");
                println!("  P2 LOADS before P1 STORES, so P1's update is lost.");
                println!(
                    "  Result: {}  (correct would be {})",
                    paint(&format!("counter = {result}"), &format!("{BOLD}{RED}")),
                    paint(&correct.to_string(), GREEN)
                );
            }
            "2" => {
                let threads = ask_u32_or("Number of threads", 4);
                let iters = ask_u32_or("Increments per thread", 100_000);
                let (racy, locked, expected) = sync::race_demo_threads(threads, iters);
                let lost = expected.saturating_sub(racy);
                let note = if lost == 0 {
                    paint("no lost updates this run (a data race is not guaranteed to manifest)", DIM)
                } else {
                    paint("updates lost to the data race", RED)
                };
                let racy_style = if lost == 0 { GREEN } else { &format!("{BOLD}{RED}") };
                println!();
                println!("  {} {}", paint("Expected    :", DIM), paint(&expected.to_string(), CYAN));
                println!(
                    "  {} {}  {}",
                    paint("RACY result :", DIM),
                    paint(&racy.to_string(), racy_style),
                    note
                );
                println!(
                    "  {} {}  {}",
                    paint("Mutex result:", DIM),
                    paint(&locked.to_string(), &format!("{BOLD}{GREEN}")),
                    paint("always exact", DIM)
                );
            }
            "3" => {
                let producers = ask_u32_or("Producers", 2);
                let consumers = ask_u32_or("Consumers", 2);
                let items = ask_u32_or("Items per producer", 5);
                let capacity = ask_u32_or("Buffer capacity", 3);
                let got = sync::producer_consumer(producers, consumers, items, capacity as usize);
                let expected = (producers * items) as usize;
                println!();
                println!(
                    "  Consumed {} items (expected {}): {:?}",
                    paint(&got.len().to_string(), if got.len() == expected { GREEN } else { RED }),
                    expected,
                    got
                );
                if got.len() == expected {
                    ok("no lost messages: producer-consumer correctness verified");
                }
            }
            "4" => {
                println!();
                for step in sync::semaphore_demo() {
                    let style = if step.contains("BLOCKED") {
                        RED
                    } else if step.contains("signal") {
                        CYAN
                    } else if step.contains("entered") {
                        GREEN
                    } else {
                        DIM
                    };
                    println!("  {}", paint(&step, style));
                }
            }
            "5" => break,
            other => println!("  {}", paint(&format!("Unknown option: {other}"), RED)),
        }
        pause();
    }
}

// ---------- 5. ipc ----------

fn ipc_menu(k: &mut Kernel) {
    loop {
        screen(
            "INTER-PROCESS COMMUNICATION",
            &format!(
                "{} queues  ·  {} shared regions  ·  {} processes",
                k.ipc.queues.len(),
                k.ipc.regions.len(),
                k.processes.len()
            ),
        );
        options(&[
            ("1", "Create message queue", "bounded capacity"),
            ("2", "Send message", "sender, queue, receiver, body"),
            ("3", "Receive message", "FIFO, wakes a blocked sender"),
            ("4", "Show queues and shared memory", ""),
            ("5", "Create shared memory region", ""),
            ("6", "Write to shared memory", ""),
            ("7", "Read from shared memory", ""),
            ("8", "Back", ""),
        ]);
        match ask(&prompt("Select an option")).as_str() {
            "1" => {
                let capacity = ask_u32("Queue capacity");
                let id = k.ipc.create_queue(capacity as usize);
                ok(&format!("created queue Q{} (capacity {})", id, capacity));
            }
            "2" => {
                let from = ask_u32("Sender PID");
                let queue = ask_u32("Queue id (e.g. 1)");
                let to = ask_u32("Receiver PID");
                let body = ask("Message body");
                if report(k.send_message(from, queue, to, &body)).is_some() {
                    ok("message delivered to queue");
                }
            }
            "3" => {
                let queue = ask_u32("Queue id");
                let pid = ask_u32("Receiver PID");
                if let Some(msg) = report(k.receive_message(queue, pid)) {
                    println!(
                        "  {} P{} {} P{}: {:?}",
                        paint("<-", GREEN),
                        paint(&pid.to_string(), theme::pid_fg(pid)),
                        paint("received from", DIM),
                        paint(&msg.from.to_string(), theme::pid_fg(msg.from)),
                        msg.body
                    );
                }
            }
            "4" => {
                println!();
                println!("{}", k.ipc.summary());
                pause();
            }
            "5" => {
                let size = ask_u32("Region size (bytes)");
                let id = k.ipc.create_region(size as usize);
                ok(&format!("created shared memory region R{} ({} bytes)", id, size));
            }
            "6" => {
                let region = ask_u32("Region id");
                let offset = ask_u32("Offset");
                let data = ask("Data to write");
                match k.ipc.write_region(region, offset as usize, data.as_bytes()) {
                    Ok(()) => ok(&format!("wrote {} bytes", data.len())),
                    Err(e) => println!(
                        "  {} {}",
                        paint("!!", &format!("{BOLD}{RED}")),
                        paint(&KernelError::from(e).to_string(), RED)
                    ),
                }
            }
            "7" => {
                let region = ask_u32("Region id");
                let offset = ask_u32("Offset");
                let len = ask_u32("Length");
                match k.ipc.read_region(region, offset as usize, len as usize) {
                    Ok(bytes) => println!(
                        "  {} {:?}",
                        paint("read:", CYAN),
                        String::from_utf8_lossy(&bytes)
                    ),
                    Err(e) => println!(
                        "  {} {}",
                        paint("!!", &format!("{BOLD}{RED}")),
                        paint(&KernelError::from(e).to_string(), RED)
                    ),
                }
            }
            "8" => break,
            other => println!("  {}", paint(&format!("Unknown option: {other}"), RED)),
        }
    }
}

// ---------- 6. deadlock ----------

fn deadlock_menu(k: &mut Kernel) {
    loop {
        screen(
            "DEADLOCK MANAGEMENT",
            &format!(
                "{} processes  ·  {} resource types  ·  available {:?}",
                k.resources.num_processes(),
                k.resources.num_resources(),
                k.resources.available()
            ),
        );
        options(&[
            ("1", "Load demo scenario", "safe state (Banker's example)"),
            ("2", "Load demo scenario", "deadlocked (detection example)"),
            ("3", "Build scenario manually", "totals, allocations, max, requests"),
            ("4", "Show current state", ""),
            ("5", "Run Banker's Algorithm", "safety check + safe sequence"),
            ("6", "Request resources", "granted only if state stays safe"),
            ("7", "Run deadlock detection", "find deadlocked processes"),
            ("8", "Back", ""),
        ]);
        match ask(&prompt("Select an option")).as_str() {
            "1" => {
                k.resources = demo_banker_scenario();
                println!();
                println!("{}", deadlock::display_state(&k.resources));
                ok("classic Banker's example loaded: safe state");
            }
            "2" => {
                k.resources = demo_deadlock_scenario();
                println!();
                println!("{}", deadlock::display_state(&k.resources));
                println!(
                    "  {}",
                    paint("classic detection example: P1, P2, P3, P4 are deadlocked", YELLOW)
                );
            }
            "3" => {
                let m = ask_u32("Number of resource types") as usize;
                let n = ask_u32("Number of processes") as usize;
                let total = ask_vec_u32("Total resources", m);
                let mut rs = deadlock::ResourceState::new(total, n);
                for i in 0..n {
                    let alloc = ask_vec_u32(&format!("P{i} allocated"), m);
                    let max = ask_vec_u32(&format!("P{i} max claim"), m);
                    if let Err(e) = rs.set_process(i, alloc, max) {
                        println!(
                            "  {} {}",
                            paint("!!", &format!("{BOLD}{RED}")),
                            paint(&KernelError::from(e).to_string(), RED)
                        );
                    }
                    let req_line = ask(&format!("P{i} outstanding request (Enter for none)"));
                    let req: Vec<u32> = req_line.split_whitespace().filter_map(|w| w.parse().ok()).collect();
                    if req.len() == m {
                        rs.set_request(i, req).ok();
                    }
                }
                k.resources = rs;
                ok("scenario loaded");
            }
            "4" => {
                println!();
                println!("{}", deadlock::display_state(&k.resources));
                pause();
            }
            "5" => {
                let safety = deadlock::bankers(&k.resources);
                println!();
                if safety.safe {
                    println!(
                        "  {} {}",
                        paint("SAFE STATE", &format!("{BOLD}{GREEN}")),
                        paint(&format!("safe sequence: {:?}", safety.sequence), YELLOW)
                    );
                } else {
                    println!(
                        "  {}",
                        paint("UNSAFE STATE: no safe sequence exists", &format!("{BOLD}{RED}"))
                    );
                }
            }
            "6" => {
                let pid = ask_u32("Process index") as usize;
                let req = ask_vec_u32("Request vector", k.resources.num_resources());
                match deadlock::try_grant(&mut k.resources, pid, &req) {
                    Ok(()) => {
                        ok("request granted (state remains safe)");
                        println!("{}", deadlock::display_state(&k.resources));
                    }
                    Err(e) => println!(
                        "  {} {}",
                        paint("✗ request DENIED:", &format!("{BOLD}{RED}")),
                        paint(&KernelError::from(e).to_string(), RED)
                    ),
                }
            }
            "7" => {
                let deadlocked = deadlock::detect_deadlock(&k.resources);
                println!();
                if deadlocked.is_empty() {
                    println!("  {}", paint("no deadlock detected", &format!("{BOLD}{GREEN}")));
                } else {
                    println!(
                        "  {} {}",
                        paint("DEADLOCK detected", &format!("{BOLD}{RED}")),
                        paint(&format!("processes: {:?}", deadlocked), RED)
                    );
                }
            }
            "8" => break,
            other => println!("  {}", paint(&format!("Unknown option: {other}"), RED)),
        }
    }
}

/// Silberschatz Banker's example: 5 processes, 3 resource types.
fn demo_banker_scenario() -> deadlock::ResourceState {
    let mut rs = deadlock::ResourceState::new(vec![10, 5, 7], 5);
    let alloc = [
        [0, 1, 0],
        [2, 0, 0],
        [3, 0, 2],
        [2, 1, 1],
        [0, 0, 2],
    ];
    let max = [
        [7, 5, 3],
        [3, 2, 2],
        [9, 0, 2],
        [2, 2, 2],
        [4, 3, 3],
    ];
    for i in 0..5 {
        rs.set_process(i, alloc[i].to_vec(), max[i].to_vec()).unwrap();
    }
    rs
}

/// Silberschatz deadlock-detection example: available (0,0,0), P1..P4 deadlocked.
fn demo_deadlock_scenario() -> deadlock::ResourceState {
    let mut rs = deadlock::ResourceState::new(vec![7, 2, 6], 5);
    let alloc = [
        [0, 1, 0],
        [2, 0, 0],
        [3, 0, 3],
        [2, 1, 1],
        [0, 0, 2],
    ];
    let request = [
        [0, 0, 0],
        [2, 0, 2],
        [0, 0, 1],
        [1, 0, 0],
        [0, 0, 2],
    ];
    for i in 0..5 {
        rs.set_process(i, alloc[i].to_vec(), alloc[i].to_vec()).unwrap();
        rs.set_request(i, request[i].to_vec()).unwrap();
    }
    rs
}

// ---------- 7. system calls ----------

fn syscall_menu(k: &mut Kernel) {
    loop {
        screen(
            "SYSTEM CALL INTERFACE",
            "every call validates, delegates to a subsystem, and logs an event",
        );
        options(&[
            ("1", "fork_process", "create a process"),
            ("2", "exit_process", "terminate + free resources"),
            ("3", "wait_process", "block a process"),
            ("4", "allocate_memory", ""),
            ("5", "free_memory", ""),
            ("6", "send / receive message", "use IPC menu (option 5)"),
            ("9", "Back", ""),
        ]);
        match ask(&prompt("Select an option")).as_str() {
            "1" => {
                let name = ask("Process name");
                let arrival = ask_u32("Arrival time");
                let burst = ask_u32("Burst time");
                let priority = ask_u32_or("Priority (lower = higher)", 5);
                let mem = ask_u32("Memory requirement (0 = none)");
                if let Some(pid) = report(k.fork_process(&name, arrival, burst, priority, mem)) {
                    ok(&format!("fork_process -> pid {}", pid));
                }
            }
            "2" => {
                let pid = ask_u32("PID");
                if report(k.exit_process(pid)).is_some() {
                    ok(&format!("exit_process(P{pid}) -> ok"));
                }
            }
            "3" => {
                let pid = ask_u32("PID");
                if report(k.wait_process(pid)).is_some() {
                    ok(&format!("wait_process(P{pid}) -> P{pid} blocked (WAITING)"));
                }
            }
            "4" => {
                let pid = ask_u32("PID");
                let size = ask_u32("Size (units)");
                if let Some(start) = report(k.allocate_memory(pid, size)) {
                    ok(&format!("allocate_memory(P{pid}, {size}) -> address {start}"));
                }
            }
            "5" => {
                let pid = ask_u32("PID");
                if let Some(freed) = report(k.free_memory(pid)) {
                    ok(&format!("free_memory(P{pid}) -> freed {freed}"));
                }
            }
            "6" => info(&paint("Send/receive live in the IPC menu: main menu -> 5.", DIM)),
            "9" => break,
            other => println!("  {}", paint(&format!("Unknown option: {other}"), RED)),
        }
    }
}
