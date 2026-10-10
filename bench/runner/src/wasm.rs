//! Browser bundles: builds each counter app for `wasm32-unknown-unknown` with the
//! `wasm-release` profile, runs `wasm-bindgen --target web`, measures the files and writes
//! the server HTML next to them, in `<target>/bench-www/<id>/`, for the browser step.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use flate2::Compression;
use flate2::write::GzEncoder;
use serde::{Deserialize, Serialize};

use crate::frameworks::COUNTERS;
use crate::tools::{cargo, describe_versions, locked_versions, version_of};
use crate::{Context, Result};

#[derive(Serialize, Deserialize)]
pub struct WasmReport {
    pub wasm_bindgen: String,
    pub results: Vec<WasmResult>,
}

#[derive(Serialize, Deserialize)]
pub struct WasmResult {
    pub id: String,
    pub name: String,
    pub versions: String,
    pub mode: String,
    pub wasm_bytes: u64,
    pub wasm_gzip: u64,
    /// Every JavaScript file wasm-bindgen generated (the glue and its snippets).
    pub js_files: usize,
    pub js_bytes: u64,
    /// Sum of each JavaScript file compressed on its own (one HTTP response each).
    pub js_gzip: u64,
    pub total_gzip: u64,
    /// The server-rendered page.
    pub html_bytes: u64,
}

pub fn run(ctx: &Context) -> Result<WasmReport> {
    let versions = locked_versions(ctx)?;
    let bindgen = check_wasm_bindgen(versions.get("wasm-bindgen").map(String::as_str))?;

    let mut results = Vec::new();
    for app in COUNTERS {
        cargo(
            ctx,
            &[
                "build",
                "-p",
                app.package,
                "--lib",
                "--target",
                "wasm32-unknown-unknown",
                "--profile",
                "wasm-release",
            ],
        )?;
        cargo(
            ctx,
            &[
                "build",
                "--release",
                "-p",
                app.package,
                "--bin",
                app.package,
            ],
        )?;

        let site = ctx.www.join(app.id);
        let pkg = site.join("pkg");
        if pkg.exists() {
            std::fs::remove_dir_all(&pkg)?;
        }
        std::fs::create_dir_all(&pkg)?;
        let module = ctx
            .target
            .join("wasm32-unknown-unknown/wasm-release")
            .join(format!("{}.wasm", app.lib));
        let status = Command::new("wasm-bindgen")
            .args(["--target", "web", "--no-typescript", "--out-dir"])
            .arg(&pkg)
            .arg(&module)
            .status()?;
        if !status.success() {
            return Err(format!("wasm-bindgen failed for {}", app.id).into());
        }

        let page = Command::new(ctx.target.join("release").join(app.package)).output()?;
        if !page.status.success() {
            return Err(format!("{} could not render its page", app.package).into());
        }
        std::fs::write(site.join("index.html"), &page.stdout)?;

        let wasm = std::fs::read(pkg.join(format!("{}_bg.wasm", app.lib)))?;
        let js: Vec<Vec<u8>> = files_with_extension(&pkg, "js")?
            .iter()
            .map(std::fs::read)
            .collect::<Result<_, _>>()?;
        let (wasm_gzip, js_gzip) = (
            gzip_len(&wasm)?,
            js.iter().map(|f| gzip_len(f)).sum::<Result<u64>>()?,
        );
        let result = WasmResult {
            id: app.id.into(),
            name: app.name.into(),
            versions: describe_versions(&versions, app.crates),
            mode: app.mode.into(),
            wasm_bytes: wasm.len() as u64,
            wasm_gzip,
            js_files: js.len(),
            js_bytes: js.iter().map(|f| f.len() as u64).sum(),
            js_gzip,
            total_gzip: wasm_gzip + js_gzip,
            html_bytes: page.stdout.len() as u64,
        };
        eprintln!(
            "  {}: {} B wasm, {} B gzip in total",
            app.id, result.wasm_bytes, result.total_gzip
        );
        results.push(result);
    }
    Ok(WasmReport {
        wasm_bindgen: bindgen,
        results,
    })
}

/// The wasm-bindgen CLI must be the exact version of the `wasm-bindgen` crate.
fn check_wasm_bindgen(locked: Option<&str>) -> Result<String> {
    let installed =
        version_of("wasm-bindgen", &["--version"]).ok_or("wasm-bindgen CLI not found")?;
    match locked {
        Some(locked) if !installed.ends_with(locked.trim_end_matches(" (local)")) => Err(format!(
            "{installed} installed, but the crate is {locked}: cargo install wasm-bindgen-cli --version {locked}"
        )
        .into()),
        _ => Ok(installed),
    }
}

/// Size of `bytes` compressed with gzip at level 9 (`gzip -9`).
fn gzip_len(bytes: &[u8]) -> Result<u64> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(bytes)?;
    Ok(encoder.finish()?.len() as u64)
}

/// Every file under `dir` (recursively) ending in `.{extension}`.
fn files_with_extension(dir: &Path, extension: &str) -> Result<Vec<PathBuf>> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            found.extend(files_with_extension(&path, extension)?);
        } else if path.extension().is_some_and(|e| e == extension) {
            found.push(path);
        }
    }
    found.sort();
    Ok(found)
}
