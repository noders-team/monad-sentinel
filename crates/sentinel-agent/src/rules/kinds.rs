use serde::Deserialize;
use crate::parse::Snapshot;
use crate::state::State;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Op { Lt, Le, Gt, Ge }

impl Op {
    pub fn cmp(self, a: f64, b: f64) -> bool {
        match self {
            Op::Lt => a < b,
            Op::Le => a <= b,
            Op::Gt => a > b,
            Op::Ge => a >= b,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RuleKind {
    Rate { metric: String, op: Op, threshold: f64, window_ms: i64 },
    Stale { metric: String },
    Threshold { metric: String, op: Op, value: f64 },
    Absent { metric: String },
    AdaptiveRate { metric: String, k: f64, window_ms: i64 },
    Freshness { max_age_ms: i64 },
    RateRatio { numerator: String, denominator: String, max_ratio: f64, min_denominator_rate: f64, window_ms: i64 },
}

impl RuleKind {
    pub fn active(&self, state: &State, snap: &Snapshot, now_ms: i64, rule_for_ms: i64) -> bool {
        match self {
            RuleKind::Threshold { metric, op, value } => {
                snap.value(metric).map_or(false, |v| op.cmp(v, *value))
            }
            RuleKind::Absent { metric } => snap.value(metric).is_none(),
            RuleKind::Stale { metric } => state
                .series(metric)
                .and_then(|s| s.stale_for_ms(now_ms))
                .map_or(false, |d| d >= rule_for_ms),
            RuleKind::Rate { metric, op, threshold, window_ms } => state
                .series(metric)
                .and_then(|s| s.rate(*window_ms))
                .map_or(false, |r| op.cmp(r, *threshold)),
            RuleKind::AdaptiveRate { metric, k, window_ms } => {
                let r = match state.series(metric).and_then(|s| s.rate(*window_ms)) {
                    Some(r) => r,
                    None => return false,
                };
                match state.baseline(metric) {
                    Some((mean, std)) => r > mean + k * std,
                    None => false, // baseline not yet warmed up
                }
            }
            RuleKind::Freshness { max_age_ms } => snap
                .freshest_ts_ms()
                .map_or(false, |ts| now_ms - ts > *max_age_ms),
            RuleKind::RateRatio { numerator, denominator, max_ratio, min_denominator_rate, window_ms } => {
                let den = match state.series(denominator).and_then(|s| s.rate(*window_ms)) {
                    Some(d) => d, None => return false,
                };
                let num = match state.series(numerator).and_then(|s| s.rate(*window_ms)) {
                    Some(n) => n, None => return false,
                };
                if den < *min_denominator_rate { return false; }
                num / den < *max_ratio
            }
        }
    }

    pub fn metric(&self) -> &str {
        match self {
            RuleKind::Rate { metric, .. }
            | RuleKind::Stale { metric }
            | RuleKind::Threshold { metric, .. }
            | RuleKind::Absent { metric }
            | RuleKind::AdaptiveRate { metric, .. } => metric,
            RuleKind::Freshness { .. } | RuleKind::RateRatio { .. } => "",
        }
    }

    /// All metrics that the rule reads from State (for tracked_metrics).
    pub fn tracked(&self) -> Vec<&str> {
        match self {
            RuleKind::RateRatio { numerator, denominator, .. } => vec![numerator, denominator],
            other => {
                let m = other.metric();
                if m.is_empty() { vec![] } else { vec![m] }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse;
    use crate::state::State;

    fn snap_with(name: &str, v: f64, ts: i64) -> crate::parse::Snapshot {
        parse(&format!("{name}{{j=\"x\"}} {v} {ts}"), ts)
    }

    #[test]
    fn threshold_gt_fires() {
        let k = RuleKind::Threshold { metric: "m".into(), op: Op::Gt, value: 800.0 };
        let snap = snap_with("m", 900.0, 0);
        let st = State::new();
        assert!(k.active(&st, &snap, 0, 0));
    }

    #[test]
    fn absent_fires_when_metric_missing() {
        let k = RuleKind::Absent { metric: "gone".into() };
        let snap = snap_with("present", 1.0, 0);
        let st = State::new();
        assert!(k.active(&st, &snap, 0, 0));
    }

    #[test]
    fn stale_fires_when_block_num_frozen() {
        let k = RuleKind::Stale { metric: "blk".into() };
        let mut st = State::new();
        st.update(&snap_with("blk", 100.0, 0), &["blk"]);
        st.update(&snap_with("blk", 100.0, 40_000), &["blk"]);
        let last = snap_with("blk", 100.0, 40_000);
        assert!(k.active(&st, &last, 40_000, 30_000));
    }

    #[test]
    fn rate_below_threshold_fires_for_commit_stall() {
        // commit_stall: rate < 1.5/s
        let k = RuleKind::Rate { metric: "c".into(), op: Op::Lt, threshold: 1.5, window_ms: 60_000 };
        let mut st = State::new();
        st.update(&snap_with("c", 0.0, 0), &["c"]);
        st.update(&snap_with("c", 10.0, 10_000), &["c"]); // 1.0/s < 1.5
        let last = snap_with("c", 10.0, 10_000);
        assert!(k.active(&st, &last, 10_000, 0));
    }

    #[test]
    fn freshness_fires_when_metrics_stale() {
        // freshest ts = 1000, now = 100_000 → age 99s > 60s max
        let snap = parse("m{j=\"x\"} 1 1000", 1000);
        let k = RuleKind::Freshness { max_age_ms: 60_000 };
        let st = State::new();
        assert!(k.active(&st, &snap, 100_000, 0));
    }

    #[test]
    fn freshness_quiet_when_metrics_fresh() {
        let snap = parse("m{j=\"x\"} 1 99_000", 99_000);
        let k = RuleKind::Freshness { max_age_ms: 60_000 };
        let st = State::new();
        assert!(!k.active(&st, &snap, 100_000, 0));
    }

    #[test]
    fn freshness_quiet_when_no_timestamps() {
        let snap = parse("m 1", 0);
        let k = RuleKind::Freshness { max_age_ms: 60_000 };
        let st = State::new();
        assert!(!k.active(&st, &snap, 100_000, 0));
    }

    #[test]
    fn rate_ratio_fires_when_voting_collapses() {
        // rounds advance 2.0/s, votes 0/s → ratio 0 < 0.5, den>=1 → fire
        let mut st = State::new();
        st.record("rounds", 0, 0.0);
        st.record("rounds", 10_000, 20.0); // 2.0/s
        st.record("votes", 0, 100.0);
        st.record("votes", 10_000, 100.0); // 0/s
        let k = RuleKind::RateRatio {
            numerator: "votes".into(), denominator: "rounds".into(),
            max_ratio: 0.5, min_denominator_rate: 1.0, window_ms: 60_000,
        };
        let snap = parse("x 1 0", 0);
        assert!(k.active(&st, &snap, 10_000, 0));
    }

    #[test]
    fn rate_ratio_quiet_when_voting_tracks_rounds() {
        let mut st = State::new();
        st.record("rounds", 0, 0.0); st.record("rounds", 10_000, 20.0); // 2.0/s
        st.record("votes", 0, 0.0);  st.record("votes", 10_000, 19.0);  // 1.9/s, ratio .95
        let k = RuleKind::RateRatio { numerator:"votes".into(), denominator:"rounds".into(), max_ratio:0.5, min_denominator_rate:1.0, window_ms:60_000 };
        let snap = parse("x 1 0", 0);
        assert!(!k.active(&st, &snap, 10_000, 0));
    }

    #[test]
    fn rate_ratio_quiet_when_chain_not_advancing() {
        let mut st = State::new();
        st.record("rounds", 0, 0.0); st.record("rounds", 10_000, 2.0); // 0.2/s < min 1.0
        st.record("votes", 0, 0.0);  st.record("votes", 10_000, 0.0);
        let k = RuleKind::RateRatio { numerator:"votes".into(), denominator:"rounds".into(), max_ratio:0.5, min_denominator_rate:1.0, window_ms:60_000 };
        let snap = parse("x 1 0", 0);
        assert!(!k.active(&st, &snap, 10_000, 0));
    }

    #[test]
    fn rate_ratio_tracked_returns_both() {
        let k = RuleKind::RateRatio { numerator:"a".into(), denominator:"b".into(), max_ratio:0.5, min_denominator_rate:1.0, window_ms:60_000 };
        assert_eq!(k.tracked(), vec!["a", "b"]);
    }
}
