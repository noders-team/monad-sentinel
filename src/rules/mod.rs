pub mod kinds;

use serde::Deserialize;
use crate::alert::{AlertState, Severity, Transition};
use crate::parse::Snapshot;
use crate::state::State;
use kinds::RuleKind;

#[derive(Debug, Deserialize)]
struct RuleSpec {
    id: String,
    severity: Severity,
    for_ms: i64,
    repeat_ms: i64,
    vdp_link: String,
    message: String,
    #[serde(default)]
    suppressed_by: Vec<String>,
    #[serde(flatten)]
    kind: RuleKind,
}

#[derive(Debug, Deserialize)]
struct RuleFile {
    rule: Vec<RuleSpec>,
}

pub struct Rule {
    pub id: String,
    pub kind: RuleKind,
    pub severity: Severity,
    pub for_ms: i64,
    pub vdp_link: String,
    pub message: String,
    pub suppressed_by: Vec<String>,
    state: AlertState,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Fired {
    pub rule_id: String,
    pub severity: Severity,
    pub transition: Transition,
    pub message: String,
    pub vdp_link: String,
}

pub struct Engine {
    rules: Vec<Rule>,
}

impl Engine {
    pub fn from_toml(text: &str) -> anyhow::Result<Engine> {
        let file: RuleFile = toml::from_str(text)?;
        let rules = file
            .rule
            .into_iter()
            .map(|s| {
                // For Stale and Freshness rules, for_ms is the staleness threshold embedded in the kind's
                // active() check; the AlertState pending window should be 0 (fire immediately
                // once the staleness duration has been reached).
                let alert_for_ms = match &s.kind {
                    kinds::RuleKind::Stale { .. } | kinds::RuleKind::Freshness { .. } => 0,
                    _ => s.for_ms,
                };
                Rule {
                    id: s.id,
                    kind: s.kind,
                    severity: s.severity,
                    for_ms: s.for_ms,
                    vdp_link: s.vdp_link,
                    message: s.message,
                    suppressed_by: s.suppressed_by,
                    state: AlertState::new(alert_for_ms, s.repeat_ms),
                }
            })
            .collect();
        Ok(Engine { rules })
    }

    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    pub fn tracked_metrics(&self) -> Vec<&str> {
        self.rules.iter().flat_map(|r| r.kind.tracked()).collect()
    }

    pub fn evaluate(&mut self, state: &State, snap: &Snapshot, now_ms: i64) -> Vec<Fired> {
        // 1) raw activity per rule
        let active: Vec<bool> = self
            .rules
            .iter()
            .map(|r| r.kind.active(state, snap, now_ms, r.for_ms))
            .collect();
        // 2) set of currently-active rule ids (owned to avoid borrow conflict with iter_mut below)
        let active_ids: std::collections::HashSet<String> = self
            .rules
            .iter()
            .zip(&active)
            .filter(|(_, &a)| a)
            .map(|(r, _)| r.id.clone())
            .collect();
        // 3) effective activity (suppressed if any suppressor is active)
        let mut out = Vec::new();
        for (i, r) in self.rules.iter_mut().enumerate() {
            let suppressed = r.suppressed_by.iter().any(|s| active_ids.contains(s.as_str()));
            let effective = active[i] && !suppressed;
            if let Some(transition) = r.state.step(effective, now_ms) {
                out.push(Fired {
                    rule_id: r.id.clone(),
                    severity: r.severity,
                    transition,
                    message: r.message.clone(),
                    vdp_link: r.vdp_link.clone(),
                });
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse;
    use crate::state::State;

    const TOML: &str = r#"
[[rule]]
id = "sync_stall"
severity = "critical"
for_ms = 30000
repeat_ms = 900000
vdp_link = "uptime"
message = "Block height frozen"
kind = "stale"
metric = "monad_execution_ledger_block_num"

[[rule]]
id = "vote_delay_high"
severity = "warning"
for_ms = 0
repeat_ms = 3600000
vdp_link = "performance"
message = "Vote delay p99 high"
kind = "threshold"
metric = "monad_state_vote_delay_ready_after_timer_start_p99_ms"
op = "gt"
value = 800.0
"#;

    #[test]
    fn loads_rules_from_toml() {
        let e = Engine::from_toml(TOML).expect("parse");
        assert_eq!(e.rule_count(), 2);
    }

    #[test]
    fn threshold_rule_fires_immediately() {
        let mut e = Engine::from_toml(TOML).expect("parse");
        let snap = parse(
            "monad_state_vote_delay_ready_after_timer_start_p99_ms{j=\"x\"} 900 0",
            0,
        );
        let st = State::new();
        let fired = e.evaluate(&st, &snap, 0);
        assert!(fired.iter().any(|f| f.rule_id == "vote_delay_high"
            && f.transition == crate::alert::Transition::Firing));
    }

    #[test]
    fn invalid_kind_returns_err() {
        let bad = r#"
[[rule]]
id = "x"
severity = "warning"
for_ms = 0
repeat_ms = 1000
vdp_link = "test"
message = "m"
kind = "no_such_kind"
metric = "m"
"#;
        assert!(Engine::from_toml(bad).is_err());
    }

    #[test]
    fn metrics_stale_fires_on_old_timestamp() {
        let toml = r#"
[[rule]]
id = "metrics_stale"
severity = "critical"
for_ms = 0
repeat_ms = 900000
vdp_link = "uptime"
message = "stale"
kind = "freshness"
max_age_ms = 60000
"#;
        let mut e = Engine::from_toml(toml).expect("parse");
        let snap = crate::parse::parse("m{j=\"x\"} 1 1000", 1000);
        let st = crate::state::State::new();
        let fired = e.evaluate(&st, &snap, 100_000);
        assert!(fired.iter().any(|f| f.rule_id == "metrics_stale"
            && f.transition == crate::alert::Transition::Firing));
    }

    #[test]
    fn tracked_metrics_skips_freshness_empty_metric() {
        let toml = r#"
[[rule]]
id = "metrics_stale"
severity = "critical"
for_ms = 0
repeat_ms = 900000
vdp_link = "uptime"
message = "stale"
kind = "freshness"
max_age_ms = 60000
"#;
        let e = Engine::from_toml(toml).expect("parse");
        assert!(e.tracked_metrics().iter().all(|m| !m.is_empty()));
    }

    #[test]
    fn suppressed_rule_does_not_fire_when_suppressor_active() {
        // suppressor fires (threshold), target would fire (threshold) but is suppressed
        let toml = r#"
[[rule]]
id = "suppressor"
severity = "critical"
for_ms = 0
repeat_ms = 900000
vdp_link = "x"
message = "sup"
kind = "threshold"
metric = "s"
op = "gt"
value = 0.0

[[rule]]
id = "target"
severity = "warning"
for_ms = 0
repeat_ms = 900000
vdp_link = "x"
message = "tgt"
kind = "threshold"
metric = "t"
op = "gt"
value = 0.0
suppressed_by = ["suppressor"]
"#;
        let mut e = Engine::from_toml(toml).expect("parse");
        let snap = crate::parse::parse("s 1 0\nt 1 0", 0); // both active
        let st = crate::state::State::new();
        let fired = e.evaluate(&st, &snap, 0);
        assert!(fired.iter().any(|f| f.rule_id == "suppressor"));
        assert!(!fired.iter().any(|f| f.rule_id == "target"), "target must be suppressed");
    }

    #[test]
    fn target_fires_when_suppressor_inactive() {
        let toml = r#"
[[rule]]
id = "suppressor"
severity = "critical"
for_ms = 0
repeat_ms = 900000
vdp_link = "x"
message = "sup"
kind = "threshold"
metric = "s"
op = "gt"
value = 0.0

[[rule]]
id = "target"
severity = "warning"
for_ms = 0
repeat_ms = 900000
vdp_link = "x"
message = "tgt"
kind = "threshold"
metric = "t"
op = "gt"
value = 0.0
suppressed_by = ["suppressor"]
"#;
        let mut e = Engine::from_toml(toml).expect("parse");
        let snap = crate::parse::parse("t 1 0", 0); // only target active, suppressor metric absent
        let st = crate::state::State::new();
        let fired = e.evaluate(&st, &snap, 0);
        assert!(fired.iter().any(|f| f.rule_id == "target"));
    }

    #[test]
    fn participation_loss_fires_and_tracks_both_metrics() {
        let toml = r#"
[[rule]]
id = "participation_loss"
severity = "critical"
for_ms = 0
repeat_ms = 900000
vdp_link = "uptime"
message = "not voting"
kind = "rate_ratio"
numerator = "votes"
denominator = "rounds"
max_ratio = 0.5
min_denominator_rate = 1.0
window_ms = 60000
"#;
        let mut e = Engine::from_toml(toml).expect("parse");
        let tracked = e.tracked_metrics();
        assert!(tracked.contains(&"votes") && tracked.contains(&"rounds"));

        let mut st = crate::state::State::new();
        st.record("rounds", 0, 0.0); st.record("rounds", 10_000, 20.0);
        st.record("votes", 0, 5.0);  st.record("votes", 10_000, 5.0);
        let snap = crate::parse::parse("x 1 0", 0);
        let fired = e.evaluate(&st, &snap, 10_000);
        assert!(fired.iter().any(|f| f.rule_id == "participation_loss"
            && f.transition == crate::alert::Transition::Firing));
    }
}
