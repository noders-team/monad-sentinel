/// A single Prometheus metric sample.
#[derive(Debug, Clone)]
pub struct Sample {
    pub name: String,
    pub labels: Vec<(String, String)>,
    pub value: f64,
    pub ts_ms: Option<i64>,
}

/// Snapshot of all metrics from a single scrape.
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub samples: Vec<Sample>,
    pub scraped_at_ms: i64,
}

impl Snapshot {
    /// Value of the first metric with the given name.
    pub fn value(&self, name: &str) -> Option<f64> {
        self.samples.iter().find(|s| s.name == name).map(|s| s.value)
    }
    /// Sum of values of all metrics whose name starts with prefix.
    pub fn sum_prefix(&self, prefix: &str) -> f64 {
        self.samples
            .iter()
            .filter(|s| s.name.starts_with(prefix))
            .map(|s| s.value)
            .sum()
    }
    /// The most recent embedded timestamp among samples (for checking otel freshness).
    pub fn freshest_ts_ms(&self) -> Option<i64> {
        self.samples.iter().filter_map(|s| s.ts_ms).max()
    }
}

/// Minimal Prometheus exposition format parser.
/// Line format: `name{labels} value [timestamp]`. Lines starting with `#` are ignored.
pub fn parse(text: &str, scraped_at_ms: i64) -> Snapshot {
    let mut samples = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(sample) = parse_line(line) {
            samples.push(sample);
        }
    }
    Snapshot { samples, scraped_at_ms }
}

fn parse_line(line: &str) -> Option<Sample> {
    // Split name+labels from the tail (value [ts]).
    let (name_labels, rest) = if let Some(brace_end) = line.find('}') {
        let nl = &line[..=brace_end];
        let rest = line[brace_end + 1..].trim_start();
        (nl, rest)
    } else {
        // no labels: everything up to the first space is the name
        let sp = line.find(' ')?;
        (&line[..sp], line[sp..].trim_start())
    };

    let (name, labels) = split_name_labels(name_labels);
    let mut parts = rest.split_whitespace();
    let value: f64 = parts.next()?.parse().ok()?;
    let ts_ms: Option<i64> = parts.next().and_then(|t| t.parse().ok());

    Some(Sample { name: name.to_string(), labels, value, ts_ms })
}

fn split_name_labels(s: &str) -> (&str, Vec<(String, String)>) {
    if let Some(open) = s.find('{') {
        let name = &s[..open];
        let inner = &s[open + 1..s.len().saturating_sub(1)]; // without { }
        let labels = inner
            .split(',')
            .filter_map(|kv| {
                let (k, v) = kv.split_once('=')?;
                Some((k.trim().to_string(), v.trim().trim_matches('"').to_string()))
            })
            .collect();
        (name, labels)
    } else {
        (s, Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> &'static str {
        include_str!("../tests/fixtures/metrics_sample.txt")
    }

    #[test]
    fn parses_value_skipping_comments() {
        let snap = parse(fixture(), 1782098354704);
        assert_eq!(snap.value("monad_state_consensus_events_commit_block"), Some(838117.0));
    }

    #[test]
    fn parses_scientific_notation() {
        let snap = parse(fixture(), 0);
        assert_eq!(snap.value("monad_execution_ledger_block_num"), Some(39_794_841.0));
    }

    #[test]
    fn missing_metric_is_none() {
        let snap = parse(fixture(), 0);
        assert_eq!(snap.value("does_not_exist"), None);
    }

    #[test]
    fn sum_prefix_aggregates_family() {
        let snap = parse(fixture(), 0);
        assert_eq!(snap.sum_prefix("monad_state_validation_errors_"), 437_089.0);
    }

    #[test]
    fn freshest_ts_picks_max_timestamp() {
        let text = "a{j=\"x\"} 1 1000\nb{j=\"x\"} 2 5000\nc{j=\"x\"} 3 3000";
        let snap = parse(text, 0);
        assert_eq!(snap.freshest_ts_ms(), Some(5000));
    }

    #[test]
    fn freshest_ts_none_when_no_timestamps() {
        let snap = parse("a 1\nb 2", 0);
        assert_eq!(snap.freshest_ts_ms(), None);
    }
}
