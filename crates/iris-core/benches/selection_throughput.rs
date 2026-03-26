use std::hint::black_box;
use std::time::{Duration, Instant};

use iris_core::{Selection, SelectionKind};

const LINE_COUNT: usize = 1_000_000;
const TARGET_MAX_SCAN_MILLIS: f64 = 100.0;
const MIN_BENCH_TIME: Duration = Duration::from_millis(750);
const WARMUP_RUNS: usize = 5;

struct BenchResult {
    iterations: u64,
    elapsed: Duration,
}

fn main() {
    let mut selection = Selection::new(0, 0, SelectionKind::Simple);
    selection.extend(LINE_COUNT.saturating_sub(1), 79);
    selection.complete();

    println!("selection_throughput");
    println!("====================");

    let full_span_result = run_benchmark(|| {
        let selected_rows = scan_linear_rows(&selection);
        black_box(selected_rows);
    });
    let per_scan_millis = per_iteration_millis(&full_span_result);
    println!(
        "full_span_contains_1m_rows: {:.2} ms/scan over {} iterations ({:.3}s)",
        per_scan_millis,
        full_span_result.iterations,
        full_span_result.elapsed.as_secs_f64()
    );
    println!("target: full span scan <= {:.2} ms", TARGET_MAX_SCAN_MILLIS);
}

fn run_benchmark<F>(mut runner: F) -> BenchResult
where
    F: FnMut(),
{
    for _ in 0..WARMUP_RUNS {
        runner();
    }

    let start = Instant::now();
    let mut iterations = 0_u64;
    while start.elapsed() < MIN_BENCH_TIME {
        runner();
        iterations += 1;
    }

    BenchResult {
        iterations: iterations.max(1),
        elapsed: start.elapsed(),
    }
}

fn scan_linear_rows(selection: &Selection) -> usize {
    let mut selected_rows = 0usize;
    for row in 0..LINE_COUNT {
        if selection.contains(row, 40) {
            selected_rows += 1;
        }
    }
    selected_rows
}

fn per_iteration_millis(result: &BenchResult) -> f64 {
    (result.elapsed.as_secs_f64() * 1000.0) / result.iterations as f64
}
