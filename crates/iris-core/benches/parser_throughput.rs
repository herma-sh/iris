use std::hint::black_box;
use std::time::{Duration, Instant};

use iris_core::{Parser, Terminal};

const TARGET_PLAIN_MIB_PER_SEC: f64 = 100.0;
const TARGET_CSI_SEQ_PER_SEC: f64 = 10_000_000.0;
const MIN_BENCH_TIME: Duration = Duration::from_millis(750);
const WARMUP_RUNS: usize = 5;

struct BenchResult {
    iterations: u64,
    elapsed: Duration,
}

fn main() {
    let plain_text = plain_text_fixture(1_000_000);
    let csi_stream = csi_stream_fixture(100_000);

    println!("parser_throughput");
    println!("=================");

    let plain_result = run_benchmark(&plain_text, |data| {
        let mut parser = Parser::new();
        let mut terminal = Terminal::new(24, 80).unwrap();
        parser.advance(&mut terminal, black_box(data)).unwrap();
        black_box(terminal);
    });
    let plain_mib_per_sec = throughput_mib_per_sec(plain_text.len(), &plain_result);
    println!(
        "plain_text_1mb: {:.2} MiB/s over {} iterations ({:.3}s)",
        plain_mib_per_sec,
        plain_result.iterations,
        plain_result.elapsed.as_secs_f64()
    );

    let csi_result = run_benchmark(&csi_stream, |data| {
        let mut parser = Parser::new();
        let mut terminal = Terminal::new(24, 80).unwrap();
        parser.advance(&mut terminal, black_box(data)).unwrap();
        black_box(terminal);
    });
    let csi_mib_per_sec = throughput_mib_per_sec(csi_stream.len(), &csi_result);
    let csi_seq_per_sec =
        (100_000.0 * csi_result.iterations as f64) / csi_result.elapsed.as_secs_f64();
    println!(
        "csi_stream_100k: {:.2} MiB/s, {:.2} seq/s over {} iterations ({:.3}s)",
        csi_mib_per_sec,
        csi_seq_per_sec,
        csi_result.iterations,
        csi_result.elapsed.as_secs_f64()
    );

    println!(
        "targets: plain_text >= {:.2} MiB/s, csi_stream >= {:.0} seq/s",
        TARGET_PLAIN_MIB_PER_SEC, TARGET_CSI_SEQ_PER_SEC
    );
}

fn run_benchmark<F>(data: &[u8], mut runner: F) -> BenchResult
where
    F: FnMut(&[u8]),
{
    for _ in 0..WARMUP_RUNS {
        runner(data);
    }

    let start = Instant::now();
    let mut iterations = 0_u64;
    while start.elapsed() < MIN_BENCH_TIME {
        runner(data);
        iterations += 1;
    }

    BenchResult {
        iterations: iterations.max(1),
        elapsed: start.elapsed(),
    }
}

fn throughput_mib_per_sec(bytes_per_iteration: usize, result: &BenchResult) -> f64 {
    let total_bytes = bytes_per_iteration as f64 * result.iterations as f64;
    (total_bytes / (1024.0 * 1024.0)) / result.elapsed.as_secs_f64()
}

fn plain_text_fixture(bytes: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(bytes);
    for index in 0..bytes {
        if index % 80 == 79 {
            data.push(b'\n');
        } else {
            data.push(b'a' + (index % 26) as u8);
        }
    }
    data
}

fn csi_stream_fixture(sequence_count: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(sequence_count * 18);
    for index in 0..sequence_count {
        let color = 30 + (index % 8) as u8;
        data.extend_from_slice(b"\x1b[");
        push_decimal(&mut data, color);
        data.extend_from_slice(b";1mX\x1b[0m");
    }
    data
}

fn push_decimal(buffer: &mut Vec<u8>, value: u8) {
    if value >= 100 {
        buffer.push(b'0' + (value / 100));
    }
    if value >= 10 {
        buffer.push(b'0' + ((value / 10) % 10));
    }
    buffer.push(b'0' + (value % 10));
}
