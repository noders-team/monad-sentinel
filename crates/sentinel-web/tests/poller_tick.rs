use std::sync::Mutex;
use sentinel_agent::state::State;
use sentinel_agent::rules::Engine;
use sentinel_agent::notify::Notifier;
use sentinel_web::store::Store;
use sentinel_web::poller::tick;

struct Noop;
impl Notifier for Noop { fn send(&self, _t: &str) -> anyhow::Result<()> { Ok(()) } }

#[test]
fn tick_records_tracked_metric_into_state_and_store() {
    let engine = Mutex::new(Engine::from_toml(
        &std::fs::read_to_string("../sentinel-agent/rules/default.toml").unwrap()).unwrap());
    let live = Mutex::new(State::new());
    let store = Store::open(":memory:").unwrap();
    let now = 10_000_i64;
    // monad_total_uptime_us is tracked by the "scrape_down" absent rule
    // and emits a value from the snapshot → will be recorded into state and store
    let text = format!("monad_total_uptime_us{{j=\"x\"}} {now} {now}", now = now);
    tick(&text, now, &live, &store, &engine, &Noop);
    // at least one tracked series should now exist in the store
    let names = engine.lock().unwrap().tracked_metrics().iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let any = names.iter().any(|n| !store.query_window(n, 0).unwrap().is_empty());
    assert!(any, "tick must persist at least one tracked metric to the store");
    // explicitly verify that monad_total_uptime_us persists
    assert!(!store.query_window("monad_total_uptime_us", 0).unwrap().is_empty(), "monad_total_uptime_us must be persisted to the store");
}
