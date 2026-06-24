use sentinel_agent::parse::parse;
use sentinel_agent::rules::Engine;
use sentinel_agent::state::State;
use sentinel_agent::notify::Notifier;
use std::cell::RefCell;

struct Mock(RefCell<Vec<String>>);
impl Notifier for Mock {
    fn send(&self, t: &str) -> anyhow::Result<()> { self.0.borrow_mut().push(t.to_string()); Ok(()) }
}

// Simulates the loop's "always record rpc_block_number" behavior: RPC is down from
// the start, so we record a constant 0 each tick. rpc_block_stall must fire.
#[test]
fn rpc_down_from_cold_start_fires_rpc_block_stall() {
    let toml = std::fs::read_to_string("rules/default.toml").unwrap();
    let mut engine = Engine::from_toml(&toml).unwrap();
    let mut state = State::new();
    let mock = Mock(RefCell::new(vec![]));
    for now in (0..=120_000).step_by(5_000) {
        // metrics fresh so metrics_stale stays silent; rpc constant 0 (down)
        let snap = parse(&format!("monad_total_uptime_us{{j=\"x\"}} {now} {now}", now = now), now);
        state.record("rpc_block_number", now, 0.0);
        sentinel_agent::run_once(&mut engine, &state, &snap, now, &mock);
    }
    assert!(
        mock.0.borrow().iter().any(|m| m.lines().next().unwrap_or("").contains("rpc_block_stall")),
        "rpc_block_stall must fire when RPC is down from cold start (constant block number)"
    );
}
