//! Running Cargo and other tools, and reading crate versions from `Cargo.lock`.

use std::collections::HashMap;
use std::process::Command;

use crate::{Context, Result};

/// Runs `cargo <args>` in the bench workspace; Cargo's progress goes to the terminal.
pub fn cargo(ctx: &Context, args: &[&str]) -> Result<()> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    eprintln!("$ cargo {}", args.join(" "));
    let status = Command::new(cargo)
        .args(args)
        .current_dir(&ctx.bench)
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("cargo {} failed ({status})", args.join(" ")).into())
    }
}

/// First line of `program args…`'s output, if it runs.
pub fn version_of(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        .next()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
}

/// Version of every package in the workspace's `Cargo.lock`, e.g. `0.8.22`, or
/// `0.1.0 (local)` for a path dependency (a package without `source`).
pub fn locked_versions(ctx: &Context) -> Result<HashMap<String, String>> {
    let lock = std::fs::read_to_string(ctx.bench.join("Cargo.lock"))?;
    let mut versions = HashMap::new();
    for package in lock.split("[[package]]").skip(1) {
        let field = |key: &str| {
            package
                .lines()
                .find_map(|line| line.strip_prefix(key)?.strip_prefix(" = "))
                .map(|v| v.trim_matches('"').to_string())
        };
        if let (Some(name), Some(version)) = (field("name"), field("version")) {
            let local = if field("source").is_some() {
                ""
            } else {
                " (local)"
            };
            versions.insert(name, format!("{version}{local}"));
        }
    }
    Ok(versions)
}

/// `leptos 0.8.22 + leptos_meta 0.8.7`, from the locked versions of `crates`.
pub fn describe_versions(versions: &HashMap<String, String>, crates: &[&str]) -> String {
    crates
        .iter()
        .map(|c| format!("{c} {}", versions.get(*c).map_or("?", String::as_str)))
        .collect::<Vec<_>>()
        .join(" + ")
}
