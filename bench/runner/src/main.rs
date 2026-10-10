//! Builds and runs every benchmark, then writes `results/*.json` and `RESULTS.md`.
//!
//! ```text
//! cargo run --release -p runner -- [all|ssr|wasm|browser|report] [--quick] [--chrome PATH]
//! ```
//!
//! - `ssr`: server rendering time of the product page, one process per framework.
//! - `wasm`: builds the counter apps for the browser and measures the bundles.
//! - `browser`: loads each counter app in headless Chromium (needs `wasm` first).
//! - `report`: rewrites `RESULTS.md` from `results/*.json`.
//! - `all` (default): the four, in that order.
//!
//! `--quick` shortens every measurement (for trying the setup; the report says so).

mod browser;
mod environment;
mod frameworks;
mod report;
mod server;
mod ssr;
mod tools;
mod wasm;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

pub type Result<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

/// Paths and options shared by every step.
pub struct Context {
    /// The `bench/` directory (the workspace root).
    pub bench: PathBuf,
    /// Cargo's target directory.
    pub target: PathBuf,
    /// Where the counter apps are assembled (`<target>/bench-www/<id>/`).
    pub www: PathBuf,
    /// `bench/results/`.
    pub results: PathBuf,
    pub quick: bool,
    pub chrome: Option<PathBuf>,
}

impl Context {
    fn new(quick: bool, chrome: Option<PathBuf>) -> Self {
        let bench = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("runner is in bench/")
            .to_path_buf();
        let target = std::env::var_os("CARGO_TARGET_DIR")
            .map_or_else(|| bench.join("target"), PathBuf::from);
        Self {
            www: target.join("bench-www"),
            results: bench.join("results"),
            bench,
            target,
            quick,
            chrome,
        }
    }

    /// Writes `value` to `results/<name>.json`.
    pub fn save<T: serde::Serialize>(&self, name: &str, value: &T) -> Result<()> {
        std::fs::create_dir_all(&self.results)?;
        let path = self.results.join(format!("{name}.json"));
        std::fs::write(&path, serde_json::to_string_pretty(value)? + "\n")?;
        eprintln!("wrote {}", path.display());
        Ok(())
    }

    /// Reads `results/<name>.json`, if it exists.
    pub fn load<T: serde::de::DeserializeOwned>(&self, name: &str) -> Result<Option<T>> {
        let path = self.results.join(format!("{name}.json"));
        match std::fs::read_to_string(&path) {
            Ok(json) => Ok(Some(
                serde_json::from_str(&json).map_err(|e| format!("{}: {e}", path.display()))?,
            )),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}

fn main() -> ExitCode {
    let mut step = "all".to_string();
    let (mut quick, mut chrome) = (false, std::env::var_os("CHROME").map(PathBuf::from));
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--quick" => quick = true,
            "--chrome" => chrome = args.next().map(PathBuf::from),
            "all" | "ssr" | "wasm" | "browser" | "report" => step = arg,
            _ => {
                eprintln!("usage: runner [all|ssr|wasm|browser|report] [--quick] [--chrome PATH]");
                return ExitCode::FAILURE;
            }
        }
    }
    let ctx = Context::new(quick, chrome);
    match run(&ctx, &step) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(ctx: &Context, step: &str) -> Result<()> {
    let all = step == "all";
    if all || step != "report" {
        ctx.save("environment", &environment::collect(ctx))?;
    }
    if all || step == "ssr" {
        ctx.save("ssr", &ssr::run(ctx)?)?;
    }
    if all || step == "wasm" {
        ctx.save("wasm", &wasm::run(ctx)?)?;
    }
    if all || step == "browser" {
        let report = browser::run(ctx)?;
        ctx.save("browser", &report)?;
        // The browser version is only known once it ran.
        let mut env = environment::collect(ctx);
        env.chromium = Some(report.chromium.clone());
        ctx.save("environment", &env)?;
    }
    report::write(ctx)
}

/// Median of `values` (mean of the two middle values for an even count).
pub fn median(values: &[f64]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let n = sorted.len();
    match n {
        0 => f64::NAN,
        _ if n % 2 == 1 => sorted[n / 2],
        _ => (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0,
    }
}
