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
fn participation_loss_fires_when_node_stops_voting() {
    let toml = std::fs::read_to_string("rules/default.toml").unwrap();
    let mut engine = Engine::from_toml(&toml).unwrap();
    let mut state = State::new();
    let mock = Mock(RefCell::new(vec![]));

    let tracked: Vec<String> = engine.tracked_metrics().iter().map(|s| s.to_string()).collect();
    let refs: Vec<&str> = tracked.iter().map(|s| s.as_str()).collect();

    // rounds advance 2.3/s via :8889 snapshot; votes FROZEN. Fresh otel ts each tick (so metrics_stale stays silent).
    let mut rounds = 1000.0;
    let votes = 1000.0; // frozen
    for now in (0..=120_000).step_by(5_000) {
        rounds += 11.5; // 2.3/s * 5s
        let text = format!(
            "monad_state_consensus_events_enter_new_round_qc{{j=\"x\"}} {rounds} {now}\n\
             monad_state_consensus_events_created_vote{{j=\"x\"}} {votes} {now}\n\
             monad_total_uptime_us{{j=\"x\"}} {now} {now}",
            rounds = rounds, votes = votes, now = now
        );
        let snap = parse(&text, now);
        state.update(&snap, &refs);
        sentinel_agent::run_once(&mut engine, &state, &snap, now, &mock);
    }
    let msgs = mock.0.borrow();
    assert!(
        msgs.iter().any(|m| m.lines().next().unwrap_or("").contains("participation_loss")),
        "participation_loss should fire when votes freeze while rounds advance"
    );
}
