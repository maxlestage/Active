//! The machine and tools the results come from.

use serde::{Deserialize, Serialize};

use crate::Context;
use crate::tools::version_of;

#[derive(Serialize, Deserialize)]
pub struct Environment {
    /// UTC date of the run, `YYYY-MM-DD`.
    pub date: String,
    pub cpu: String,
    pub logical_cores: usize,
    pub memory_gib: Option<f64>,
    pub os: String,
    pub rustc: String,
    pub wasm_bindgen: String,
    /// Set by the browser step.
    pub chromium: Option<String>,
    pub quick: bool,
}

pub fn collect(ctx: &Context) -> Environment {
    let cpuinfo = std::fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    let cpu = cpuinfo
        .lines()
        .find_map(|line| {
            line.strip_prefix("model name")
                .and_then(|rest| rest.split_once(':'))
                .map(|(_, v)| v.trim().to_string())
        })
        .unwrap_or_else(|| "unknown".into());
    let meminfo = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let memory_gib = meminfo
        .lines()
        .find_map(|line| line.strip_prefix("MemTotal:"))
        .and_then(|rest| {
            rest.trim()
                .trim_end_matches("kB")
                .trim()
                .parse::<f64>()
                .ok()
        })
        .map(|kib| (kib / 1024.0 / 1024.0 * 10.0).round() / 10.0);
    let os = std::fs::read_to_string("/proc/sys/kernel/osrelease").map_or_else(
        |_| std::env::consts::OS.to_string(),
        |release| format!("{} {}", std::env::consts::OS, release.trim()),
    );
    Environment {
        date: today(),
        cpu,
        logical_cores: std::thread::available_parallelism().map_or(1, usize::from),
        memory_gib,
        os,
        rustc: version_of("rustc", &["-V"]).unwrap_or_else(|| "unknown".into()),
        wasm_bindgen: version_of("wasm-bindgen", &["--version"])
            .unwrap_or_else(|| "not installed".into()),
        chromium: ctx
            .load::<Environment>("environment")
            .ok()
            .flatten()
            .and_then(|previous| previous.chromium),
        quick: ctx.quick,
    }
}

/// Today's UTC date, from the system clock (days since 1970 to a civil date).
fn today() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let days = (secs / 86_400) as i64 + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let mp = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}
