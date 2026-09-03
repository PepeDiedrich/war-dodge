use std::hint::black_box;
use std::time::Instant;
use termux_poller::{next_backoff, parse_duration};

fn bench(name: &str, iterations: u64, mut operation: impl FnMut()) {
    for _ in 0..10_000 {
        operation();
    }
    let start = Instant::now();
    for _ in 0..iterations {
        operation();
    }
    println!(
        "{name:24} {:>8.1} ns/op",
        start.elapsed().as_nanos() as f64 / iterations as f64
    );
}

fn main() {
    bench("parse_duration", 1_000_000, || {
        let _ = black_box(parse_duration(black_box("15m")).unwrap());
    });
    bench("backoff", 10_000_000, || {
        let _ = black_box(next_backoff(
            black_box(Some(std::time::Duration::from_secs(30))),
            std::time::Duration::from_secs(30),
            std::time::Duration::from_secs(900),
        ));
    });
}
