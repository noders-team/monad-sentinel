use std::time::Duration;

pub struct Scraper {
    url: String,
    timeout: Duration,
}

impl Scraper {
    pub fn new(url: String, timeout_ms: u64) -> Self {
        Self { url, timeout: Duration::from_millis(timeout_ms) }
    }

    pub fn scrape(&self) -> anyhow::Result<String> {
        let agent = ureq::AgentBuilder::new()
            .timeout(self.timeout)
            .build();
        let body = agent.get(&self.url).call()?.into_string()?;
        Ok(body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unreachable_host_returns_err_not_panic() {
        let s = Scraper::new("http://127.0.0.1:1/metrics".into(), 500);
        assert!(s.scrape().is_err());
    }
}
