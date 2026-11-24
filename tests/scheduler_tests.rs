use mini_os_kernel::process::Process;
use mini_os_kernel::scheduler::{self, Algorithm, ScheduleResult};

fn p(pid: u32, arrival: u32, burst: u32, priority: u32) -> Process {
    Process::new(pid, format!("P{}", pid), arrival, burst, priority, 0)
}

fn approx(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

fn slots(result: &ScheduleResult) -> Vec<(u32, u32, u32)> {
    result
        .slots
        .iter()
        .map(|s| (s.pid, s.start, s.end))
        .collect()
}

#[test]
fn fcfs_order_and_metrics() {
    let procs = vec![p(1, 0, 4, 1), p(2, 1, 3, 1), p(3, 2, 2, 1)];
    let r = scheduler::schedule(&procs, Algorithm::Fcfs);
    assert_eq!(slots(&r), vec![(1, 0, 4), (2, 4, 7), (3, 7, 9)]);
    let m = &r.metrics;
    assert!(approx(m.avg_wait, 8.0 / 3.0));
    assert!(approx(m.avg_turnaround, 17.0 / 3.0));
    assert!(approx(m.avg_response, 8.0 / 3.0));
    assert!(approx(m.cpu_util_pct, 100.0));
    assert!(approx(m.throughput, 3.0 / 9.0));
}

#[test]
fn fcfs_idles_until_first_arrival() {
    let procs = vec![p(1, 5, 2, 1), p(2, 6, 3, 1)];
    let r = scheduler::schedule(&procs, Algorithm::Fcfs);
    assert_eq!(slots(&r), vec![(1, 5, 7), (2, 7, 10)]);
    // CPU idle for 5 ticks: util = 5/10.
    assert!(approx(r.metrics.cpu_util_pct, 50.0));
}

#[test]
fn sjf_picks_shortest_among_arrived() {
    let procs = vec![p(1, 0, 4, 1), p(2, 1, 3, 1), p(3, 2, 2, 1)];
    let r = scheduler::schedule(&procs, Algorithm::Sjf);
    assert_eq!(slots(&r), vec![(1, 0, 4), (3, 4, 6), (2, 6, 9)]);
    assert!(approx(r.metrics.avg_wait, 7.0 / 3.0));
    assert!(approx(r.metrics.avg_turnaround, 16.0 / 3.0));
}

#[test]
fn srtf_preempts_on_shorter_arrival() {
    // Classic example: P1(0,8) P2(1,4) P3(2,9) P4(3,5).
    let procs = vec![p(1, 0, 8, 1), p(2, 1, 4, 1), p(3, 2, 9, 1), p(4, 3, 5, 1)];
    let r = scheduler::schedule(&procs, Algorithm::Srtf);
    assert_eq!(
        slots(&r),
        vec![(1, 0, 1), (2, 1, 5), (4, 5, 10), (1, 10, 17), (3, 17, 26)]
    );
    assert!(approx(r.metrics.avg_wait, 6.5));
    assert!(approx(r.metrics.avg_turnaround, 13.0));
}

#[test]
fn round_robin_respects_quantum() {
    let procs = vec![p(1, 0, 5, 1), p(2, 1, 3, 1)];
    let r = scheduler::schedule(&procs, Algorithm::RoundRobin { quantum: 2 });
    assert_eq!(
        slots(&r),
        vec![(1, 0, 2), (2, 2, 4), (1, 4, 6), (2, 6, 7), (1, 7, 8)]
    );
    assert!(approx(r.metrics.avg_wait, 3.0));
    assert!(approx(r.metrics.avg_turnaround, 7.0));
    assert!(approx(r.metrics.avg_response, 0.5)); // P1: 0, P2: 1
}

#[test]
fn round_robin_zero_quantum_does_not_hang() {
    let procs = vec![p(1, 0, 3, 1)];
    let r = scheduler::schedule(&procs, Algorithm::RoundRobin { quantum: 0 });
    // Quantum clamped to 1, still completes.
    assert_eq!(slots(&r), vec![(1, 0, 3)]);
}

#[test]
fn priority_runs_highest_priority_first() {
    // Lower value = higher priority.
    let procs = vec![p(1, 0, 4, 2), p(2, 0, 3, 1), p(3, 0, 1, 3)];
    let r = scheduler::schedule(&procs, Algorithm::Priority);
    assert_eq!(slots(&r), vec![(2, 0, 3), (1, 3, 7), (3, 7, 8)]);
    assert!(approx(r.metrics.avg_wait, 10.0 / 3.0)); // waits: 3, 0, 7
    assert!(approx(r.metrics.avg_turnaround, 6.0)); // turnarounds: 7, 3, 8
}

#[test]
fn empty_workload_gives_zero_metrics() {
    let r = scheduler::schedule(&[], Algorithm::Fcfs);
    assert!(r.slots.is_empty());
    assert!(approx(r.metrics.avg_wait, 0.0));
    assert!(approx(r.metrics.throughput, 0.0));
}

#[test]
fn zero_burst_processes_are_skipped() {
    let procs = vec![p(1, 0, 0, 1), p(2, 0, 3, 1)];
    let r = scheduler::schedule(&procs, Algorithm::Fcfs);
    assert_eq!(slots(&r), vec![(2, 0, 3)]);
    // Stats only cover schedulable processes.
    let stats = scheduler::process_stats(&r, &procs);
    assert_eq!(stats.len(), 1);
    assert_eq!(stats[0].pid, 2);
}

#[test]
fn gantt_renders_a_line_per_row() {
    let procs = vec![p(1, 0, 4, 1), p(2, 1, 3, 1)];
    let r = scheduler::schedule(&procs, Algorithm::Fcfs);
    let g = scheduler::gantt(&r, &procs);
    assert_eq!(g.lines().count(), 2);
    assert!(g.contains("P1"));
    assert!(g.contains("P2"));
}

#[test]
fn gantt_handles_an_empty_timeline() {
    let r = scheduler::schedule(&[], Algorithm::Fcfs);
    assert!(scheduler::gantt(&r, &[]).contains("empty timeline"));
}

#[test]
fn process_stats_are_consistent_with_slots() {
    let procs = vec![p(1, 0, 4, 1), p(2, 1, 3, 1), p(3, 2, 2, 1)];
    let r = scheduler::schedule(&procs, Algorithm::Fcfs);
    let stats = scheduler::process_stats(&r, &procs);
    let s2 = stats.iter().find(|s| s.pid == 2).unwrap();
    assert_eq!(s2.first_start, 4);
    assert_eq!(s2.finish, 7);
    assert_eq!(s2.turnaround, 6); // 7 - 1
    assert_eq!(s2.wait, 3); // 6 - 3
    assert_eq!(s2.response, 3);
}
