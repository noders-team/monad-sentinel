use sentinel_agent::{parse::parse, notify::{format_message, Notifier}};
use sentinel_agent::rules::{Engine, Fired};
use sentinel_agent::state::State;
use crate::store::Store;
use std::sync::Mutex;

/// One evaluation cycle: parse the raw metrics text, record tracked metrics into the
/// in-memory `State` and the sqlite `Store`, then run alert evaluation.
///
/// Returns the list of `Fired` transitions so the caller can push them into the alert buffer.
///
/// Lock order: engine → live (always acquire engine first when both are needed).
pub fn tick(
    metrics_text: &str,
    now_ms: i64,
    live: &Mutex<State>,
    store: &Store,
    engine: &Mutex<Engine>,
    notifier: &dyn Notifier,
) -> Vec<Fired> {
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
    let fired = {
        let mut eng = engine.lock().unwrap();
        let st = live.lock().unwrap();
        eng.evaluate(&st, &snap, now_ms)
    };

    // Notify for each transition.
    for f in &fired {
        let text = format_message(f);
        if let Err(e) = notifier.send(&text) {
            eprintln!("notify error: {e:#}");
        }
    }

    fired
}

const SCRAPE_TIMEOUT_MS: u64 = 10_000;
/// Maximum number of alert records to keep in the in-memory buffer.
const ALERTS_CAP: usize = 100;

/// Spawns a background thread that scrapes `cfg.metrics_url` every `scrape_interval_ms`,
/// calls `tick`, and pushes any fired alerts into `state.alerts`.
/// Errors are logged; the loop never panics.
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
                    let fired = tick(
                        &text,
                        now_ms,
                        &state.live,
                        &state.store,
                        &state.engine,
                        notifier.as_ref(),
                    );
                    // Push fired alerts into the in-memory buffer (cap ALERTS_CAP, newest at back).
                    if !fired.is_empty() {
                        let mut buf = state.alerts.lock().unwrap();
                        for f in fired {
                            if buf.len() >= ALERTS_CAP {
                                buf.pop_front();
                            }
                            buf.push_back(crate::state::AlertRecord {
                                ts_ms: now_ms,
                                rule: f.rule_id,
                                message: f.message,
                            });
                        }
                    }
                }
                Err(e) => {
                    eprintln!("poller scrape error: {e:#}");
                }
            }
            std::thread::sleep(interval);
        }
    });
}
