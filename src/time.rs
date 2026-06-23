use std::time::{SystemTime, UNIX_EPOCH};

/// Current time in milliseconds since the Unix epoch.
pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn now_ms_is_positive_and_recent() {
        let t = now_ms();
        assert!(t > 1_700_000_000_000, "expected ms timestamp, got {t}");
    }
}
