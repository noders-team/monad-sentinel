use sentinel_agent::{parse::parse, run_once};
use sentinel_agent::state::State;
use sentinel_agent::rules::Engine;
use sentinel_agent::notify::Notifier;
use crate::store::Store;
use std::sync::Mutex;

/// One evaluation cycle: parse the raw metrics text, record tracked metrics into the
/// in-memory `State` and the sqlite `Store`, then run alert evaluation via `run_once`.
///
/// Lock order: engine → live (always acquire engine first when both are needed).
pub fn tick(
    metrics_text: &str,
    now_ms: i64,
    live: &Mutex<State>,
    store: &Store,
    engine: &Mutex<Engine>,
    notifier: &dyn Notifier,
) {
    let snap = parse(metrics_text, now_ms);

    // Collect tracked metric names while holding engine lock; release before touching live.
    let tracked: Vec<String> = {
        let eng = engine.lock().unwrap();
        eng.tracked_metrics().iter().map(|s| s.to_string()).collect()
    };

    // Record tracked metrics into in-memory state and persistent store.
    {
        let mut st = live.lock().unwrap();
        for name in &tracked {
            if let Some(v) = snap.value(name) {
                st.record(name, now_ms, v);
                if let Err(e) = store.append_metric(name, now_ms, v) {
                    eprintln!("store append error: {e:#}");
                }
            }
        }
    }

    // Run alert evaluation: acquire engine first, then live (matches invariant).
    {
        let mut eng = engine.lock().unwrap();
        let st = live.lock().unwrap();
        let _ = run_once(&mut eng, &st, &snap, now_ms, notifier);
    }
}

const SCRAPE_TIMEOUT_MS: u64 = 10_000;

/// Spawns a background thread that scrapes `cfg.metrics_url` every `scrape_interval_ms`
/// and calls `tick`. Errors are logged; the loop never panics.
pub fn spawn_loop(
    state: crate::state::AppState,
    notifier: std::sync::Arc<dyn Notifier + Send + Sync>,
) {
    std::thread::spawn(move || {
        let scraper = sentinel_agent::scrape::Scraper::new(
            state.cfg.metrics_url.clone(),
            SCRAPE_TIMEOUT_MS,
        );
        loop {
            let interval = std::time::Duration::from_millis(state.cfg.scrape_interval_ms);
            match scraper.scrape() {
                Ok(text) => {
                    let now_ms = (state.now)();
                    tick(
                        &text,
                        now_ms,
                        &state.live,
                        &state.store,
                        &state.engine,
                        notifier.as_ref(),
                    );
                }
                Err(e) => {
                    eprintln!("poller scrape error: {e:#}");
                }
            }
            std::thread::sleep(interval);
        }
    });
}
