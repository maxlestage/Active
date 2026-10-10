//! The SSR timing loop, shared by every `ssr-*` binary so they are all measured the same way.

use std::hint::black_box;
use std::time::{Duration, Instant};

use crate::check::check_page;

/// Pages rendered per batch are chosen so that one batch lasts about this long.
const BATCH: Duration = Duration::from_millis(10);
/// The page number cycles through `1..=PAGES`, so the output changes on every iteration.
const PAGES: u64 = 1000;

/// Checks then times `render(i)`, which must return the complete HTML document of product
/// page `i`, and prints one JSON line on stdout:
///
/// `{"us_per_page":…,"p10":…,"p90":…,"batches":…,"batch_size":…,"pages":…,"bytes":…}`
///
/// Method: renders for `BENCH_WARMUP_MS` (default 1000) to warm up caches, the allocator and
/// the branch predictors, then measures batches of about 10 ms for `BENCH_MEASURE_MS`
/// (default 3000). `us_per_page` is the median of the batches; `p10`/`p90` show the spread.
/// Exits with status 1 if a page is wrong. With `BENCH_PRINT=1`, only prints page 1.
pub fn bench_ssr(mut render: impl FnMut(u32) -> String) {
    if std::env::var_os("BENCH_PRINT").is_some() {
        println!("{}", render(1));
        return;
    }
    for i in [1, 2, 499, 1000] {
        if let Err(error) = check_page(i, &render(i)) {
            eprintln!("wrong page {i}: {error}\n{}", render(i));
            std::process::exit(1);
        }
    }
    let bytes = render(1).len();
    let warmup = env_duration("BENCH_WARMUP_MS", 1000);
    let measure = env_duration("BENCH_MEASURE_MS", 3000);

    let mut n = 0u64;
    let mut next = move || {
        n += 1;
        (n % PAGES + 1) as u32
    };
    let mut sink = 0usize;

    let start = Instant::now();
    let mut warm = 0u32;
    while start.elapsed() < warmup {
        sink += black_box(render(next())).len();
        warm += 1;
    }
    let per_page = start.elapsed() / warm.max(1);
    let batch_size = (BATCH.as_nanos() / per_page.as_nanos().max(1)).clamp(1, 1_000_000) as u32;

    let mut samples = Vec::new();
    let start = Instant::now();
    while start.elapsed() < measure || samples.len() < 20 {
        let t = Instant::now();
        for _ in 0..batch_size {
            sink += black_box(render(next())).len();
        }
        samples.push(t.elapsed().as_secs_f64() * 1e6 / f64::from(batch_size));
    }
    black_box(sink);

    samples.sort_by(f64::total_cmp);
    let quantile = |q: f64| samples[((samples.len() - 1) as f64 * q).round() as usize];
    println!(
        r#"{{"us_per_page":{:.4},"p10":{:.4},"p90":{:.4},"batches":{},"batch_size":{},"pages":{},"bytes":{}}}"#,
        quantile(0.5),
        quantile(0.1),
        quantile(0.9),
        samples.len(),
        batch_size,
        samples.len() as u64 * u64::from(batch_size),
        bytes
    );
}

fn env_duration(name: &str, default_ms: u64) -> Duration {
    let ms = std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default_ms);
    Duration::from_millis(ms)
}
