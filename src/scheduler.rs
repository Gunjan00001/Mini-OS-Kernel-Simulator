//! CPU scheduling algorithms, metrics, Gantt chart, and per-process stats.
//!
//! All algorithms are pure functions over a snapshot of processes:
//! `(processes, algorithm) -> ScheduleResult`. They advance in deterministic
//! ticks and never mutate the caller's process table, so the same workload
//! can be run through every algorithm and compared.

use crate::process::Process;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Algorithm {
    Fcfs,
    Sjf,
    Srtf,
    RoundRobin { quantum: u32 },
    Priority,
}

impl Algorithm {
    pub fn label(&self) -> String {
        match self {
            Algorithm::Fcfs => "FCFS".to_string(),
            Algorithm::Sjf => "SJF".to_string(),
            Algorithm::Srtf => "SRTF".to_string(),
            Algorithm::RoundRobin { quantum } => format!("ROUND ROBIN (q={quantum})"),
            Algorithm::Priority => "PRIORITY".to_string(),
        }
    }

    /// All algorithms with a default quantum, for the compare-all view.
    pub fn all(quantum: u32) -> [Algorithm; 5] {
        [
            Algorithm::Fcfs,
            Algorithm::Sjf,
            Algorithm::Srtf,
            Algorithm::RoundRobin { quantum },
            Algorithm::Priority,
        ]
    }
}

/// One CPU occupancy interval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Slot {
    pub pid: u32,
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Metrics {
    pub avg_wait: f64,
    pub avg_turnaround: f64,
    pub avg_response: f64,
    pub cpu_util_pct: f64,
    /// Processes completed per unit of time.
    pub throughput: f64,
}

#[derive(Debug, Clone)]
pub struct ScheduleResult {
    pub algo: Algorithm,
    pub slots: Vec<Slot>,
    pub metrics: Metrics,
}

/// Per-process schedule statistics.
#[derive(Debug, Clone, Copy)]
pub struct ProcStat {
    pub pid: u32,
    pub arrival: u32,
    pub burst: u32,
    pub first_start: u32,
    pub finish: u32,
    pub wait: u32,
    pub turnaround: u32,
    pub response: u32,
}

/// Internal simulation job: a mutable copy of the schedulable part of a process.
#[derive(Clone)]
struct Job {
    pid: u32,
    arrival: u32,
    burst: u32,
    priority: u32,
    remaining: u32,
}

/// Run `algo` over `processes`. Processes with `burst == 0` are skipped.
pub fn schedule(processes: &[Process], algo: Algorithm) -> ScheduleResult {
    let mut jobs: Vec<Job> = processes
        .iter()
        .filter(|p| p.burst > 0)
        .map(|p| Job {
            pid: p.pid,
            arrival: p.arrival,
            burst: p.burst,
            priority: p.priority,
            remaining: p.burst,
        })
        .collect();
    jobs.sort_by_key(|j| (j.arrival, j.pid));

    let slots = match algo {
        Algorithm::Fcfs => fcfs(&mut jobs),
        Algorithm::Sjf => non_preemptive(&mut jobs, |j| (j.burst, j.arrival, j.pid)),
        Algorithm::Srtf => srtf(&mut jobs),
        Algorithm::RoundRobin { quantum } => round_robin(&mut jobs, quantum.max(1)),
        Algorithm::Priority => non_preemptive(&mut jobs, |j| (j.priority, j.arrival, j.pid)),
    };
    let slots = merge_slots(slots);
    let metrics = compute_metrics(processes, &slots);
    ScheduleResult { algo, slots, metrics }
}

/// FCFS: run in arrival order, skipping ahead over idle time.
fn fcfs(jobs: &mut [Job]) -> Vec<Slot> {
    let mut t = 0;
    let mut slots = Vec::new();
    for j in jobs.iter_mut() {
        if t < j.arrival {
            t = j.arrival;
        }
        let end = t + j.burst;
        slots.push(Slot { pid: j.pid, start: t, end });
        t = end;
    }
    slots
}

/// Non-preemptive pick-one-and-run-to-completion loop. Used by SJF (order by
/// burst) and Priority (order by priority value).
fn non_preemptive<F>(jobs: &mut [Job], key: F) -> Vec<Slot>
where
    F: Fn(&Job) -> (u32, u32, u32),
{
    let mut t = 0;
    let mut done = 0usize;
    let n = jobs.len();
    let mut slots = Vec::new();
    while done < n {
        let pick = jobs
            .iter()
            .enumerate()
            .filter(|(_, j)| j.remaining > 0 && j.arrival <= t)
            .min_by_key(|(_, j)| key(j))
            .map(|(i, _)| i);
        let Some(i) = pick else {
            // Nothing has arrived yet: jump to the next arrival.
            t = jobs.iter().filter(|j| j.remaining > 0).map(|j| j.arrival).min().unwrap();
            continue;
        };
        let j = &mut jobs[i];
        let end = t + j.remaining;
        slots.push(Slot { pid: j.pid, start: t, end });
        j.remaining = 0;
        t = end;
        done += 1;
    }
    slots
}

/// SRTF: preemptive shortest-remaining-time-first, one tick at a time.
fn srtf(jobs: &mut [Job]) -> Vec<Slot> {
    let mut t = 0;
    let mut done = 0usize;
    let n = jobs.len();
    let mut slots = Vec::new();
    while done < n {
        let pick = jobs
            .iter()
            .enumerate()
            .filter(|(_, j)| j.remaining > 0 && j.arrival <= t)
            .min_by_key(|(_, j)| (j.remaining, j.arrival, j.pid))
            .map(|(i, _)| i);
        let Some(i) = pick else {
            t = jobs.iter().filter(|j| j.remaining > 0).map(|j| j.arrival).min().unwrap();
            continue;
        };
        let j = &mut jobs[i];
        let start = t;
        j.remaining -= 1;
        t += 1;
        if j.remaining == 0 {
            done += 1;
        }
        slots.push(Slot { pid: j.pid, start, end: t });
    }
    slots
}

/// Round Robin with a time quantum.
fn round_robin(jobs: &mut [Job], quantum: u32) -> Vec<Slot> {
    let mut queue = std::collections::VecDeque::new();
    let mut next = 0usize; // next job (in arrival order) not yet enqueued
    let mut done = 0usize;
    let n = jobs.len();
    let mut t = 0;
    let mut slots = Vec::new();
    while done < n {
        while next < n && jobs[next].arrival <= t {
            queue.push_back(next);
            next += 1;
        }
        let Some(i) = queue.pop_front() else {
            // Idle: jump to the next arrival.
            t = jobs[next].arrival;
            continue;
        };
        let run = quantum.min(jobs[i].remaining);
        slots.push(Slot { pid: jobs[i].pid, start: t, end: t + run });
        jobs[i].remaining -= run;
        t += run;
        // Processes that arrived during this slice get in line before the
        // preempted one re-joins the queue.
        while next < n && jobs[next].arrival <= t {
            queue.push_back(next);
            next += 1;
        }
        if jobs[i].remaining > 0 {
            queue.push_back(i);
        } else {
            done += 1;
        }
    }
    slots
}

/// Coalesce adjacent slices of the same process (produced by the tick loop).
fn merge_slots(mut slots: Vec<Slot>) -> Vec<Slot> {
    slots.dedup_by(|a, b| {
        if a.pid == b.pid && a.start == b.end {
            b.end = a.end;
            true
        } else {
            false
        }
    });
    slots
}

fn compute_metrics(processes: &[Process], slots: &[Slot]) -> Metrics {
    let scheduled: Vec<&Process> = processes.iter().filter(|p| p.burst > 0).collect();
    let n = scheduled.len();
    if n == 0 {
        return Metrics {
            avg_wait: 0.0,
            avg_turnaround: 0.0,
            avg_response: 0.0,
            cpu_util_pct: 0.0,
            throughput: 0.0,
        };
    }
    let makespan = slots.last().map(|s| s.end).unwrap_or(0) as f64;
    let busy: u32 = slots.iter().map(|s| s.end - s.start).sum();
    let (mut wait, mut tat, mut resp) = (0.0, 0.0, 0.0);
    for p in scheduled {
        let mine: Vec<&Slot> = slots.iter().filter(|s| s.pid == p.pid).collect();
        let first = *mine.first().expect("scheduled process missing from timeline");
        let finish = mine.last().unwrap().end;
        tat += (finish - p.arrival) as f64;
        wait += (finish - p.arrival - p.burst) as f64;
        resp += (first.start - p.arrival) as f64;
    }
    Metrics {
        avg_wait: wait / n as f64,
        avg_turnaround: tat / n as f64,
        avg_response: resp / n as f64,
        cpu_util_pct: if makespan > 0.0 { busy as f64 / makespan * 100.0 } else { 0.0 },
        throughput: if makespan > 0.0 { n as f64 / makespan } else { 0.0 },
    }
}

/// Per-process stats derived from the timeline.
pub fn process_stats(result: &ScheduleResult, processes: &[Process]) -> Vec<ProcStat> {
    processes
        .iter()
        .filter(|p| p.burst > 0)
        .map(|p| {
            let mine: Vec<&Slot> = result.slots.iter().filter(|s| s.pid == p.pid).collect();
            let first = *mine.first().expect("scheduled process missing from timeline");
            let finish = mine.last().unwrap().end;
            ProcStat {
                pid: p.pid,
                arrival: p.arrival,
                burst: p.burst,
                first_start: first.start,
                finish,
                wait: finish - p.arrival - p.burst,
                turnaround: finish - p.arrival,
                response: first.start - p.arrival,
            }
        })
        .collect()
}

/// ASCII Gantt chart:
/// ```text
/// Time:  0    4    7    12
///        | P1 | P2 | P3 |
/// ```
pub fn gantt(result: &ScheduleResult, processes: &[Process]) -> String {
    if result.slots.is_empty() {
        return "(empty timeline: no schedulable processes)".to_string();
    }
    let name = |pid: u32| {
        processes
            .iter()
            .find(|p| p.pid == pid)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| format!("P{pid}"))
    };
    let mut top = String::from("Time:");
    let mut mid = String::from("     ");
    let mut last_end = 0u32;
    for slot in &result.slots {
        let label = format!(" {}", name(slot.pid));
        let w = label
            .len()
            .max(slot.start.to_string().len())
            .max(4);
        top.push_str(&format!("{:>w$} ", slot.start, w = w + 1));
        mid.push_str(&format!("|{:<w$}", label, w = w + 1));
        last_end = slot.end;
    }
    top.push_str(&format!("{:>3}", last_end));
    mid.push('|');
    format!("{}\n{}", top, mid)
}

