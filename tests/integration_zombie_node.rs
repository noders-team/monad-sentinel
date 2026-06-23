use sentinel_agent::parse::parse;
use sentinel_agent::rules::Engine;
use sentinel_agent::state::State;
use sentinel_agent::notify::Notifier;
use std::cell::RefCell;

struct Mock(RefCell<Vec<String>>);
impl Notifier for Mock {
    fn send(&self, t: &str) -> anyhow::Result<()> { self.0.borrow_mut().push(t.to_string()); Ok(()) }
}

#[test]
fn frozen_metrics_but_live_chain_fires_metrics_stale_not_rpc_stall() {
    let toml = std::fs::read_to_string("rules/default.toml").unwrap();
    let mut engine = Engine::from_toml(&toml).unwrap();
    let mut state = State::new();
    let tracked = engine.tracked_metrics().iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let tracked_refs: Vec<&str> = tracked.iter().map(|s| s.as_str()).collect();
    let mock = Mock(RefCell::new(vec![]));

    // metrics frozen: every snapshot carries the SAME embedded ts (60_000),
    // while wall-clock `now` advances and rpc_block_number keeps rising.
    // block_num is also frozen (500) to trigger sync_stall co-fire.
    let frozen_ts = 60_000i64;
    let frozen_block_num = 500.0;
    let mut rpc_block = 1000.0;
    for now in (60_000..=300_000).step_by(5_000) {
        let snap = parse(&format!(
            "monad_total_uptime_us{{j=\"x\"}} 1 {frozen_ts}\nmonad_execution_ledger_block_num{{j=\"x\"}} {frozen_block_num} {frozen_ts}",
            frozen_ts = frozen_ts,
            frozen_block_num = frozen_block_num as i64
        ), now);
        // node chain alive: rpc advances 2.5/s * 5s = 12.5 ~ 12 per tick
        rpc_block += 12.0;
        state.record("rpc_block_number", now, rpc_block);
        state.update(&snap, &tracked_refs);
        sentinel_agent::run_once(&mut engine, &state, &snap, now, &mock);
    }

    let msgs = mock.0.borrow();
    assert!(msgs.iter().any(|m| m.lines().next().unwrap_or("").contains("metrics_stale")), "metrics_stale should fire");
    assert!(!msgs.iter().any(|m| m.lines().next().unwrap_or("").contains("rpc_block_stall")), "rpc_block_stall must NOT fire when chain is live");
    // M0.5 expected co-fire: sync_stall fires because monad_execution_ledger_block_num is frozen.
    // This co-fire will be suppressed by M0.6 Correlation engine.
    // Operator disambiguates: if rpc_block_stall is SILENT, chain is alive and this is
    // metrics-pipeline failure (restart the node), not a chain stall.
    assert!(msgs.iter().any(|m| m.lines().next().unwrap_or("").contains("sync_stall")), "sync_stall should co-fire (frozen block_num); M0.6 Correlation will suppress");
}
