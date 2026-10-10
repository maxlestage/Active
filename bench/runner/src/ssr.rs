//! Server rendering: builds every `ssr-*` binary, runs each one in its own process several
//! times (interleaved, so a slow period of the machine does not hit one framework only) and
//! keeps the median.

use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::frameworks::SSR;
use crate::tools::{cargo, describe_versions, locked_versions};
use crate::{Context, Result, median};

#[derive(Serialize, Deserialize)]
pub struct SsrReport {
    pub quick: bool,
    pub runs: usize,
    pub warmup_ms: u64,
    pub measure_ms: u64,
    pub results: Vec<SsrResult>,
}

#[derive(Serialize, Deserialize)]
pub struct SsrResult {
    pub id: String,
    pub name: String,
    pub versions: String,
    pub how: String,
    /// Median over the runs of each run's median; `None` if the implementation failed.
    pub us_per_page: Option<f64>,
    /// Size of the rendered page 1.
    pub bytes: Option<usize>,
    pub runs: Vec<Run>,
    pub error: Option<String>,
}

/// What one `ssr-*` process prints (see `bench_common::bench_ssr`).
#[derive(Serialize, Deserialize, Clone)]
pub struct Run {
    pub us_per_page: f64,
    pub p10: f64,
    pub p90: f64,
    pub batches: u64,
    pub batch_size: u64,
    pub pages: u64,
    pub bytes: usize,
}

pub fn run(ctx: &Context) -> Result<SsrReport> {
    let (runs, warmup_ms, measure_ms) = if ctx.quick {
        (1, 200, 500)
    } else {
        (3, 1000, 3000)
    };
    let versions = locked_versions(ctx)?;

    // One package at a time: each binary gets exactly its own features.
    let mut packages: Vec<&str> = SSR.iter().map(|b| b.package).collect();
    packages.dedup();
    for package in &packages {
        cargo(ctx, &["build", "--release", "-p", package])?;
    }

    let mut results: Vec<SsrResult> = SSR
        .iter()
        .map(|b| SsrResult {
            id: b.id.into(),
            name: b.name.into(),
            versions: describe_versions(&versions, b.crates),
            how: b.how.into(),
            us_per_page: None,
            bytes: None,
            runs: Vec::new(),
            error: None,
        })
        .collect();

    for round in 1..=runs {
        for (bench, result) in SSR.iter().zip(&mut results) {
            if result.error.is_some() {
                continue;
            }
            eprintln!("ssr {} (run {round}/{runs})", bench.id);
            let binary = ctx.target.join("release").join(bench.package);
            let output = Command::new(&binary)
                .args(bench.args)
                .env("BENCH_WARMUP_MS", warmup_ms.to_string())
                .env("BENCH_MEASURE_MS", measure_ms.to_string())
                .output()?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                result.error = Some(stderr.lines().next().unwrap_or("failed").to_string());
                eprintln!("  FAILED: {stderr}");
                continue;
            }
            let line = String::from_utf8_lossy(&output.stdout);
            let run: Run = serde_json::from_str(line.trim())
                .map_err(|e| format!("{}: {e}: {line}", bench.id))?;
            eprintln!(
                "  {:.2} µs/page (p10 {:.2}, p90 {:.2})",
                run.us_per_page, run.p10, run.p90
            );
            result.runs.push(run);
        }
    }

    for result in &mut results {
        if result.error.is_none() && !result.runs.is_empty() {
            let medians: Vec<f64> = result.runs.iter().map(|r| r.us_per_page).collect();
            result.us_per_page = Some(median(&medians));
            result.bytes = result.runs.first().map(|r| r.bytes);
        }
    }
    Ok(SsrReport {
        quick: ctx.quick,
        runs,
        warmup_ms,
        measure_ms,
        results,
    })
}
