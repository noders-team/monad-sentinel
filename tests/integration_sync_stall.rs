use sentinel_agent::parse::parse;
use sentinel_agent::rules::Engine;
use sentinel_agent::state::State;
use sentinel_agent::notify::Notifier;
use std::cell::RefCell;

struct Mock(RefCell<Vec<String>>);
impl Notifier for Mock {
    fn send(&self, t: &str) -> anyhow::Result<()> {
        self.0.borrow_mut().push(t.to_string());
        Ok(())
    }
}

#[test]
fn frozen_block_height_fires_sync_stall() {
    let toml = std::fs::read_to_string("rules/default.toml").unwrap();
    let mut engine = Engine::from_toml(&toml).unwrap();
    let mut state = State::new();
    let tracked = engine.tracked_metrics().iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let tracked_refs: Vec<&str> = tracked.iter().map(|s| s.as_str()).collect();
    let mock = Mock(RefCell::new(vec![]));

    // block_num frozen at 100; ticks every 5s up to 40s
    let mut sent = 0;
    for t in (0..=40_000).step_by(5_000) {
        let text = format!(
            "monad_execution_ledger_block_num{{j=\"x\"}} 100 {t}\nmonad_total_uptime_us{{j=\"x\"}} {t} {t}",
            t = t
        );
        let snap = parse(&text, t);
        state.update(&snap, &tracked_refs);
        sent += sentinel_agent::run_once(&mut engine, &state, &snap, t, &mock);
    }
    assert!(
        mock.0.borrow().iter().any(|m| m.contains("sync_stall")),
        "sync_stall should fire when block height is frozen; sent={sent}"
    );
}
