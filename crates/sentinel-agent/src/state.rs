use std::collections::{HashMap, VecDeque};
use crate::parse::Snapshot;

const MAX_POINTS: usize = 2048;

/// Exponentially weighted moving average and variance.
#[derive(Debug, Clone)]
pub struct Ewma {
    alpha: f64,
    mean: f64,
    var: f64,
    initialized: bool,
}

impl Ewma {
    pub fn new(alpha: f64) -> Self {
        Self { alpha, mean: 0.0, var: 0.0, initialized: false }
    }

    pub fn observe(&mut self, x: f64) {
        if !self.initialized {
            self.mean = x;
            self.var = 0.0;
            self.initialized = true;
            return;
        }
        let diff = x - self.mean;
        let incr = self.alpha * diff;
        self.mean += incr;
        self.var = (1.0 - self.alpha) * (self.var + diff * incr);
    }

    pub fn mean(&self) -> f64 { self.mean }
    pub fn stddev(&self) -> f64 { self.var.max(0.0).sqrt() }
}

impl Default for Ewma {
    fn default() -> Self {
        Self::new(0.05)
    }
}

/// Time series for a single metric (ring buffer of recent data points).
#[derive(Debug, Default)]
pub struct Series {
    points: VecDeque<(i64, f64)>, // (ts_ms, value)
}

impl Series {
    pub fn new() -> Self {
        Self { points: VecDeque::new() }
    }

    pub fn push(&mut self, ts_ms: i64, value: f64) {
        self.points.push_back((ts_ms, value));
        if self.points.len() > MAX_POINTS {
            self.points.pop_front();
        }
    }

    pub fn last(&self) -> Option<f64> {
        self.points.back().map(|&(_, v)| v)
    }

    /// Rate in units/sec over points within the window. Counter reset → 0.
    pub fn rate(&self, window_ms: i64) -> Option<f64> {
        let (last_ts, last_v) = *self.points.back()?;
        let cutoff = last_ts - window_ms;
        let (first_ts, first_v) =
            *self.points.iter().find(|&&(t, _)| t >= cutoff)?;
        let dt_s = (last_ts - first_ts) as f64 / 1000.0;
        if dt_s <= 0.0 {
            return None;
        }
        // If counter decreased (reset), return 0
        if last_v < first_v {
            return Some(0.0);
        }
        let dv = last_v - first_v;
        Some((dv / dt_s).max(0.0))
    }

    /// How many ms the value has not changed (None if the value changed at the current time).
    pub fn stale_for_ms(&self, now_ms: i64) -> Option<i64> {
        let (_, last_v) = *self.points.back()?;
        let mut changed_at = self.points.back()?.0;
        for &(t, v) in self.points.iter().rev() {
            if (v - last_v).abs() > f64::EPSILON {
                break;
            }
            changed_at = t;
        }
        let stale_ms = now_ms - changed_at;
        // Return None if the value changed at the current time (0 staleness means it just changed)
        if stale_ms == 0 {
            None
        } else {
            Some(stale_ms)
        }
    }
}

/// Collection of tracked time series.
#[derive(Debug, Default)]
pub struct State {
    series: HashMap<String, Series>,
    baselines: HashMap<String, Ewma>,
}

impl State {
    pub fn new() -> Self {
        Self { series: HashMap::new(), baselines: HashMap::new() }
    }

    /// Update series with values from the snapshot for the given list of metric names.
    pub fn update(&mut self, snap: &Snapshot, names: &[&str]) {
        for &name in names {
            if let Some(v) = snap.value(name) {
                self.series
                    .entry(name.to_string())
                    .or_default()
                    .push(snap.scraped_at_ms, v);
            }
        }
    }

    pub fn series(&self, name: &str) -> Option<&Series> {
        self.series.get(name)
    }

    pub fn observe_baseline(&mut self, key: &str, x: f64) {
        self.baselines
            .entry(key.to_string())
            .or_insert_with(|| Ewma::new(0.05))
            .observe(x);
    }

    pub fn baseline(&self, key: &str) -> Option<(f64, f64)> {
        self.baselines.get(key).map(|e| (e.mean(), e.stddev()))
    }

    /// Directly push a value into a named series (for sources outside the :8889 snapshot, e.g. RPC).
    pub fn record(&mut self, name: &str, ts_ms: i64, value: f64) {
        self.series.entry(name.to_string()).or_default().push(ts_ms, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_per_second_over_window() {
        let mut s = Series::new();
        s.push(0, 100.0);
        s.push(10_000, 125.0); // +25 over 10s => 2.5/s
        assert_eq!(s.rate(60_000), Some(2.5));
    }

    #[test]
    fn counter_reset_clamps_to_zero() {
        let mut s = Series::new();
        s.push(0, 100.0);
        s.push(1_000, 5.0); // counter reset
        assert_eq!(s.rate(60_000), Some(0.0));
    }

    #[test]
    fn stale_when_value_unchanged() {
        let mut s = Series::new();
        s.push(0, 42.0);
        s.push(5_000, 42.0);
        // unchanged for 5s
        assert_eq!(s.stale_for_ms(5_000), Some(5_000));
    }

    #[test]
    fn not_stale_when_value_changes() {
        let mut s = Series::new();
        s.push(0, 42.0);
        s.push(5_000, 43.0);
        assert_eq!(s.stale_for_ms(5_000), None);
    }

    #[test]
    fn ewma_converges_to_constant_input() {
        let mut e = Ewma::new(0.3);
        for _ in 0..200 { e.observe(10.0); }
        assert!((e.mean() - 10.0).abs() < 1e-6);
        assert!(e.stddev() < 1e-3);
    }

    #[test]
    fn ewma_stddev_grows_with_variance() {
        let mut e = Ewma::new(0.3);
        for i in 0..200 { e.observe(if i % 2 == 0 { 0.0 } else { 20.0 }); }
        assert!(e.stddev() > 5.0, "stddev was {}", e.stddev());
    }

    #[test]
    fn record_pushes_named_series() {
        let mut st = State::new();
        st.record("rpc_block_number", 0, 100.0);
        st.record("rpc_block_number", 1000, 102.0);
        assert_eq!(st.series("rpc_block_number").and_then(|s| s.last()), Some(102.0));
    }
}
