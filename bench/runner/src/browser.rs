//! Browser: time from navigation start to "counter interactive" in headless Chromium,
//! driven with chromiumoxide.
//!
//! No JavaScript is evaluated. Each app records `performance.now()` in `<body>` attributes
//! from its Rust code (`data-start` when its wasm starts, `data-ready` when the button is
//! interactive); they are read through DOM queries (`DOM.querySelector` and
//! `DOM.describeNode`). The click uses `Input.dispatchMouseEvent` at the button's position
//! (`DOM.getContentQuads`).

use std::time::{Duration, Instant};

use chromiumoxide::{Browser, BrowserConfig, Page};
use futures::StreamExt;
use serde::{Deserialize, Serialize};

use crate::frameworks::COUNTERS;
use crate::{Context, Result, median, server};

/// Network profiles: name and added latency (Chromium applies it to every request, so it is
/// the round-trip time). Bandwidth is not limited.
const PROFILES: [(&str, f64); 2] = [("localhost", 0.0), ("100 ms RTT", 100.0)];

/// How long a page may take to become interactive.
const TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Serialize, Deserialize)]
pub struct BrowserReport {
    pub quick: bool,
    pub chromium: String,
    /// Measured loads per framework and profile (after one warm-up round).
    pub loads: usize,
    pub results: Vec<BrowserResult>,
}

#[derive(Serialize, Deserialize)]
pub struct BrowserResult {
    pub id: String,
    pub name: String,
    pub mode: String,
    pub ready_when: String,
    pub profiles: Vec<ProfileResult>,
    pub click: Click,
}

#[derive(Serialize, Deserialize)]
pub struct ProfileResult {
    pub profile: String,
    pub latency_ms: f64,
    /// `data-ready` of each load: milliseconds from navigation start to interactive.
    pub ready_ms: Vec<f64>,
    /// `data-start` of each load: milliseconds from navigation start to the wasm start.
    pub start_ms: Vec<f64>,
    pub median_ready_ms: f64,
    pub median_start_ms: f64,
    /// Median of `ready - start`: the framework's own work (hydration or first render).
    pub median_start_to_ready_ms: f64,
}

/// One click on the button: the number must go from 3 to 4.
#[derive(Serialize, Deserialize)]
pub struct Click {
    pub ok: bool,
    pub before: String,
    pub after: String,
}

/// Timings recorded by one page load.
struct Load {
    start: f64,
    ready: f64,
}

pub fn run(ctx: &Context) -> Result<BrowserReport> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(run_async(ctx))
}

async fn run_async(ctx: &Context) -> Result<BrowserReport> {
    let loads = if ctx.quick { 3 } else { 21 };
    for app in COUNTERS {
        if !ctx.www.join(app.id).join("index.html").exists() {
            return Err(format!("{} is not built: run the `wasm` step first", app.id).into());
        }
    }

    // A fresh profile for every run.
    let profile = ctx.target.join("bench-chromium-profile");
    let _ = std::fs::remove_dir_all(&profile);
    let mut config = BrowserConfig::builder()
        .no_sandbox()
        .disable_cache()
        .user_data_dir(&profile)
        .window_size(1280, 800)
        .request_timeout(Duration::from_secs(60));
    if let Some(chrome) = &ctx.chrome {
        config = config.chrome_executable(chrome);
    }
    let (mut browser, mut handler) = Browser::launch(config.build()?).await?;
    let events = tokio::spawn(async move { while handler.next().await.is_some() {} });
    let chromium = browser.version().await?.product;
    eprintln!("browser: {chromium}");

    let sites = COUNTERS
        .iter()
        .map(|app| Ok(format!("http://{}/", server::serve(ctx.www.join(app.id))?)))
        .collect::<Result<Vec<String>>>()?;
    let page = browser.new_page("about:blank").await?;

    // timings[profile][app]: loads interleaved across apps (one load of each, then again),
    // so a slow moment of the machine does not hit a single framework.
    let mut timings: Vec<Vec<Vec<Load>>> = Vec::new();
    for (name, latency) in PROFILES {
        emulate_latency(&page, latency).await?;
        let mut per_app: Vec<Vec<Load>> = COUNTERS.iter().map(|_| Vec::new()).collect();
        // Round 0 warms up the browser and the server; it is not counted.
        for round in 0..=loads {
            for ((app, url), measured) in COUNTERS.iter().zip(&sites).zip(&mut per_app) {
                let timing = load(&page, url)
                    .await
                    .map_err(|e| format!("{} ({name}): {e}", app.id))?;
                if round > 0 {
                    measured.push(timing);
                }
            }
        }
        timings.push(per_app);
    }

    let mut results = Vec::new();
    for (index, (app, url)) in COUNTERS.iter().zip(&sites).enumerate() {
        let profiles = PROFILES
            .iter()
            .zip(&timings)
            .map(|((name, latency), per_app)| profile_result(name, *latency, &per_app[index]))
            .collect::<Vec<_>>();
        for p in &profiles {
            eprintln!(
                "  {} ({}): interactive after {:.1} ms",
                app.id, p.profile, p.median_ready_ms
            );
        }

        emulate_latency(&page, 0.0).await?;
        load(&page, url).await?;
        let click = click(&page).await?;
        let verdict = if click.ok { "ok" } else { "WRONG" };
        eprintln!(
            "  {}: click {} -> {} ({verdict})",
            app.id, click.before, click.after
        );

        results.push(BrowserResult {
            id: app.id.into(),
            name: app.name.into(),
            mode: app.mode.into(),
            ready_when: app.ready.into(),
            profiles,
            click,
        });
    }
    page.close().await?;

    browser.close().await?;
    browser.wait().await?;
    events.await?;
    Ok(BrowserReport {
        quick: ctx.quick,
        chromium,
        loads,
        results,
    })
}

fn profile_result(name: &str, latency: f64, loads: &[Load]) -> ProfileResult {
    let ready_ms: Vec<f64> = loads.iter().map(|l| l.ready).collect();
    let start_ms: Vec<f64> = loads.iter().map(|l| l.start).collect();
    let start_to_ready: Vec<f64> = loads.iter().map(|l| l.ready - l.start).collect();
    ProfileResult {
        profile: name.into(),
        latency_ms: latency,
        median_ready_ms: median(&ready_ms),
        median_start_ms: median(&start_ms),
        median_start_to_ready_ms: median(&start_to_ready),
        ready_ms,
        start_ms,
    }
}

/// Adds `latency` milliseconds to every request of `page`, without limiting bandwidth
/// (`Network.emulateNetworkConditions`; 0 turns it off).
async fn emulate_latency(page: &Page, latency: f64) -> Result<()> {
    // Deprecated in the protocol in favor of `emulateNetworkConditionsByRule` +
    // `overrideNetworkState`, but still supported by Chromium and simpler for one rule.
    #[allow(deprecated)]
    let params = chromiumoxide::cdp::browser_protocol::network::EmulateNetworkConditionsParams::new(
        false, latency, -1.0, -1.0,
    );
    page.execute(params).await?;
    Ok(())
}

/// Loads `url` from a blank page and waits until the app marks itself interactive.
async fn load(page: &Page, url: &str) -> Result<Load> {
    page.goto("about:blank").await?;
    // Returns after the `load` event; the app may become interactive before or after it.
    page.goto(url).await?;
    let deadline = Instant::now() + TIMEOUT;
    loop {
        if let Ok(body) = page.find_element("body[data-ready]").await {
            // Flat list: [name, value, name, value, …].
            let attributes = body.attributes().await?;
            let read = |name: &str| {
                attributes
                    .chunks(2)
                    .find(|pair| pair[0] == name)
                    .and_then(|pair| pair.get(1)?.parse::<f64>().ok())
                    .ok_or_else(|| format!("no numeric {name} on <body>"))
            };
            return Ok(Load {
                start: read("data-start")?,
                ready: read("data-ready")?,
            });
        }
        if Instant::now() > deadline {
            return Err("the page never set data-ready".into());
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}

/// Clicks `#inc` once and checks that `#count` goes from 3 to 4.
async fn click(page: &Page) -> Result<Click> {
    let before = count(page).await?;
    let point = page.find_element("#inc").await?.clickable_point().await?;
    page.click(point).await?;
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let after = count(page).await?;
        if after == "4" || Instant::now() > deadline {
            return Ok(Click {
                ok: before == "3" && after == "4",
                before,
                after,
            });
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// The text of `#count` (its text nodes, without hydration comments).
async fn count(page: &Page) -> Result<String> {
    let node = page.find_element("#count").await?.description().await?;
    const TEXT_NODE: i64 = 3;
    Ok(node
        .children
        .unwrap_or_default()
        .iter()
        .filter(|child| child.node_type == TEXT_NODE)
        .map(|child| child.node_value.as_str())
        .collect())
}
