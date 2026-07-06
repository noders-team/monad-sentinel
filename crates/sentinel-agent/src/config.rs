use serde::Deserialize;
use anyhow::Context;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    pub metrics_url: String,
    pub scrape_interval_ms: u64,
    /// Optional override; when absent the rules compiled into the binary are used.
    pub rules_path: Option<String>,
    pub rpc_url: String,
    pub scrape_timeout_ms: u64,
    pub rpc_timeout_ms: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            metrics_url: "http://localhost:8889/metrics".to_string(),
            scrape_interval_ms: 5000,
            rules_path: None,
            rpc_url: "http://localhost:8080".to_string(),
            scrape_timeout_ms: 8000,
            rpc_timeout_ms: 8000,
        }
    }
}

/// Default rule set compiled into the binary so a from-scratch install works
/// without shipping rules/ alongside it (sentinel-web embeds the same file).
pub const DEFAULT_RULES_TOML: &str = include_str!("../rules/default.toml");

impl Config {
    pub fn from_toml(text: &str) -> anyhow::Result<Config> {
        let c: Config = toml::from_str(text)?;
        Ok(c)
    }

    /// Rule definitions to feed the engine: an explicitly configured file must
    /// exist (no silent fallback), otherwise the embedded defaults are used.
    pub fn rules_text(&self) -> anyhow::Result<String> {
        match &self.rules_path {
            Some(path) => std::fs::read_to_string(path)
                .with_context(|| format!("failed to read rules file {path}")),
            None => Ok(DEFAULT_RULES_TOML.to_string()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Secrets {
    pub telegram_token: String,
    pub telegram_chat_id: String,
}

impl Secrets {
    pub fn from_env() -> anyhow::Result<Secrets> {
        let telegram_token = std::env::var("SENTINEL_TELEGRAM_TOKEN")
            .context("SENTINEL_TELEGRAM_TOKEN is not set")?;
        let telegram_chat_id = std::env::var("SENTINEL_TELEGRAM_CHAT_ID")
            .context("SENTINEL_TELEGRAM_CHAT_ID is not set")?;
        Ok(Secrets { telegram_token, telegram_chat_id })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_apply_for_empty_toml() {
        let c = Config::from_toml("").expect("parse");
        assert_eq!(c.metrics_url, "http://localhost:8889/metrics");
        assert_eq!(c.scrape_interval_ms, 5000);
    }

    #[test]
    fn rpc_url_has_default() {
        let c = Config::from_toml("").expect("parse");
        assert_eq!(c.rpc_url, "http://localhost:8080");
    }

    #[test]
    fn overrides_from_toml() {
        let c = Config::from_toml("scrape_interval_ms = 10000").expect("parse");
        assert_eq!(c.scrape_interval_ms, 10000);
    }

    #[test]
    fn secrets_fail_fast_when_missing() {
        // ensure the vars are absent
        std::env::remove_var("SENTINEL_TELEGRAM_TOKEN");
        std::env::remove_var("SENTINEL_TELEGRAM_CHAT_ID");
        assert!(Secrets::from_env().is_err());
    }

    #[test]
    fn rules_path_defaults_to_none() {
        let c = Config::from_toml("").expect("parse");
        assert!(c.rules_path.is_none());
    }

    #[test]
    fn rules_text_falls_back_to_embedded_defaults() {
        let c = Config::from_toml("").expect("parse");
        let text = c.rules_text().expect("embedded rules");
        assert!(text.contains("participation_loss"));
    }

    #[test]
    fn rules_text_errors_on_missing_explicit_file() {
        let c = Config::from_toml("rules_path = \"/nonexistent/sentinel-rules.toml\"")
            .expect("parse");
        let err = c.rules_text().expect_err("missing file must fail, not fall back");
        assert!(err.to_string().contains("/nonexistent/sentinel-rules.toml"));
    }

    #[test]
    fn rules_text_reads_explicit_file() {
        let dir = std::env::temp_dir();
        let path = dir.join("sentinel-agent-test-rules.toml");
        std::fs::write(&path, "# custom rules marker").expect("write tmp rules");
        let c = Config::from_toml(&format!("rules_path = \"{}\"", path.display()))
            .expect("parse");
        let text = c.rules_text().expect("read explicit file");
        assert!(text.contains("custom rules marker"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn timeout_defaults() {
        let c = Config::from_toml("").expect("parse");
        assert_eq!(c.scrape_timeout_ms, 8000);
        assert_eq!(c.rpc_timeout_ms, 8000);
    }

    #[test]
    fn timeout_overrides() {
        let c = Config::from_toml("scrape_timeout_ms = 3000\nrpc_timeout_ms = 4000").expect("parse");
        assert_eq!(c.scrape_timeout_ms, 3000);
        assert_eq!(c.rpc_timeout_ms, 4000);
    }
}
