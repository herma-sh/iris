use std::hint::black_box;
use std::mem::size_of;
use std::time::{Duration, Instant};

use iris_core::{Cell, Line, Scrollback, ScrollbackConfig, SearchConfig, SearchEngine};

const RETAINED_LINES: usize = 100_000;
const LINE_WIDTH: usize = 64;
const MATCH_INTERVAL: usize = 97;
const TARGET_MAX_MEMORY_MIB: f64 = 200.0;
const TARGET_MAX_SEARCH_MILLIS: f64 = 500.0;
const TARGET_MAX_NAVIGATION_STEP_MICROS: f64 = 500.0;
const THRESHOLD_ENFORCEMENT_ENV: &str = "IRIS_SCROLLBACK_BENCH_ASSERT";
const PUSH_WARMUP_RUNS: usize = 1;
const SEARCH_WARMUP_RUNS: usize = 5;
const MIN_PUSH_BENCH_TIME: Duration = Duration::from_millis(250);
const MIN_SEARCH_BENCH_TIME: Duration = Duration::from_millis(750);
const NAVIGATION_STEPS: usize = 128;

#[derive(Clone)]
struct LineTemplates {
    matching_line: Line,
    plain_line: Line,
}

struct BenchResult {
    iterations: u64,
    elapsed: Duration,
}

struct ScrollbackBenchMetrics {
    retained_mib: f64,
    search_millis: f64,
    navigation_step_micros: f64,
}

fn main() {
    println!("scrollback_throughput");
    println!("====================");

    let templates = LineTemplates {
        matching_line: Line::from_text(&build_line_text("needle "), false),
        plain_line: Line::from_text(&build_line_text("filler "), false),
    };

    let push_result = run_benchmark(PUSH_WARMUP_RUNS, MIN_PUSH_BENCH_TIME, || {
        let scrollback = build_scrollback(&templates);
        black_box(scrollback.len());
        black_box(scrollback.memory_bytes());
    });
    let lines_per_second =
        (RETAINED_LINES as f64 * push_result.iterations as f64) / push_result.elapsed.as_secs_f64();
    println!(
        "push_100k_lines_64_cols: {:.0} lines/s over {} iterations ({:.3}s)",
        lines_per_second,
        push_result.iterations,
        push_result.elapsed.as_secs_f64()
    );

    let scrollback = build_scrollback(&templates);
    let retained_mib = bytes_to_mib(scrollback.memory_bytes());
    let bytes_per_line = scrollback.memory_bytes() as f64 / scrollback.len().max(1) as f64;
    let dense_cell_mib = bytes_to_mib(
        RETAINED_LINES
            .saturating_mul(LINE_WIDTH)
            .saturating_mul(size_of::<Cell>()),
    );
    println!(
        "retained_memory_100k_lines: {:.2} MiB (~{:.0} bytes/line, dense-cell floor {:.2} MiB)",
        retained_mib, bytes_per_line, dense_cell_mib
    );

    let search_config = SearchConfig {
        pattern: "needle".to_owned(),
        case_sensitive: true,
        use_regex: false,
        whole_word: true,
        wrap: true,
    };
    let expected_matches = match_line_count(RETAINED_LINES, MATCH_INTERVAL);
    let baseline_matches = scrollback.search_with_config(&search_config);
    assert_eq!(baseline_matches.len(), expected_matches);

    let search_result = run_benchmark(SEARCH_WARMUP_RUNS, MIN_SEARCH_BENCH_TIME, || {
        let matches = scrollback.search_with_config(&search_config);
        black_box(matches.len());
    });
    let per_search_ms = per_iteration_millis(&search_result);
    println!(
        "search_100k_lines_whole_word: {:.2} ms/search over {} iterations ({:.3}s)",
        per_search_ms,
        search_result.iterations,
        search_result.elapsed.as_secs_f64()
    );

    let mut navigation_engine = SearchEngine::new();
    navigation_engine.set_pattern("needle");
    navigation_engine.set_whole_word(true);
    navigation_engine.set_wrap(false);
    let baseline_navigation_hits =
        navigate_forward_steps(&mut navigation_engine, &scrollback, NAVIGATION_STEPS);
    assert_eq!(baseline_navigation_hits, NAVIGATION_STEPS);

    let navigation_result = run_benchmark(SEARCH_WARMUP_RUNS, MIN_SEARCH_BENCH_TIME, || {
        let hits = navigate_forward_steps(&mut navigation_engine, &scrollback, NAVIGATION_STEPS);
        assert_eq!(
            hits, NAVIGATION_STEPS,
            "navigate_forward_steps returned fewer hits than NAVIGATION_STEPS during run_benchmark; navigation_engine or scrollback state changed unexpectedly"
        );
        black_box(hits);
    });
    let per_navigation_step_us = per_iteration_micros(&navigation_result) / NAVIGATION_STEPS as f64;
    println!(
        "search_navigation_forward_step: {:.2} us/step over {} iterations ({:.3}s)",
        per_navigation_step_us,
        navigation_result.iterations,
        navigation_result.elapsed.as_secs_f64()
    );

    println!(
        "targets: retained memory <= {:.0} MiB, search <= {:.0} ms/query, navigation <= {:.0} us/step",
        TARGET_MAX_MEMORY_MIB, TARGET_MAX_SEARCH_MILLIS, TARGET_MAX_NAVIGATION_STEP_MICROS
    );

    let metrics = ScrollbackBenchMetrics {
        retained_mib,
        search_millis: per_search_ms,
        navigation_step_micros: per_navigation_step_us,
    };
    enforce_thresholds_if_requested(&metrics);
}

fn build_scrollback(templates: &LineTemplates) -> Scrollback {
    let mut scrollback = Scrollback::new(ScrollbackConfig {
        max_lines: RETAINED_LINES,
        max_memory_bytes: None,
    });

    for line_index in 0..RETAINED_LINES {
        if line_index % MATCH_INTERVAL == 0 {
            scrollback.push(templates.matching_line.clone());
        } else {
            scrollback.push(templates.plain_line.clone());
        }
    }

    scrollback
}

fn build_line_text(prefix: &str) -> String {
    let mut text = String::with_capacity(LINE_WIDTH);
    text.push_str(prefix);
    while text.len() < LINE_WIDTH {
        text.push('x');
    }
    text.truncate(LINE_WIDTH);
    text
}

fn match_line_count(total_lines: usize, interval: usize) -> usize {
    if total_lines == 0 || interval == 0 {
        return 0;
    }

    ((total_lines - 1) / interval) + 1
}

fn run_benchmark<F>(warmup_runs: usize, min_bench_time: Duration, mut runner: F) -> BenchResult
where
    F: FnMut(),
{
    for _ in 0..warmup_runs {
        runner();
    }

    let start = Instant::now();
    let mut iterations = 0_u64;
    while start.elapsed() < min_bench_time {
        runner();
        iterations += 1;
    }

    BenchResult {
        iterations: iterations.max(1),
        elapsed: start.elapsed(),
    }
}

fn per_iteration_millis(result: &BenchResult) -> f64 {
    (result.elapsed.as_secs_f64() * 1000.0) / result.iterations as f64
}

fn per_iteration_micros(result: &BenchResult) -> f64 {
    (result.elapsed.as_secs_f64() * 1_000_000.0) / result.iterations as f64
}

fn bytes_to_mib(bytes: usize) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}

fn navigate_forward_steps(
    engine: &mut SearchEngine,
    scrollback: &Scrollback,
    steps: usize,
) -> usize {
    let mut start_line = 0_u64;
    let mut start_col = 0_usize;
    let mut hits = 0usize;
    for _ in 0..steps {
        let Some(next) = engine.search_forward(scrollback, start_line, start_col) else {
            break;
        };
        start_line = next.line_number;
        start_col = next.column;
        hits = hits.saturating_add(1);
    }
    hits
}

fn enforce_thresholds_if_requested(metrics: &ScrollbackBenchMetrics) {
    if std::env::var_os(THRESHOLD_ENFORCEMENT_ENV).is_none() {
        return;
    }

    if metrics.retained_mib > TARGET_MAX_MEMORY_MIB {
        panic!(
            "scrollback benchmark memory threshold exceeded: retained={:.2} MiB target<={:.2} MiB",
            metrics.retained_mib, TARGET_MAX_MEMORY_MIB
        );
    }

    if metrics.search_millis > TARGET_MAX_SEARCH_MILLIS {
        panic!(
            "scrollback benchmark search threshold exceeded: search={:.2} ms target<={:.2} ms",
            metrics.search_millis, TARGET_MAX_SEARCH_MILLIS
        );
    }

    if metrics.navigation_step_micros > TARGET_MAX_NAVIGATION_STEP_MICROS {
        panic!(
            "scrollback benchmark navigation threshold exceeded: navigation={:.2} us/step target<={:.2} us/step",
            metrics.navigation_step_micros, TARGET_MAX_NAVIGATION_STEP_MICROS
        );
    }

    println!(
        "threshold_check: pass (env {THRESHOLD_ENFORCEMENT_ENV} set; retained={:.2} MiB, search={:.2} ms, navigation={:.2} us/step)",
        metrics.retained_mib, metrics.search_millis, metrics.navigation_step_micros
    );
}
