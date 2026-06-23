use serde::Deserialize;
use anyhow::Context;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    pub metrics_url: String,
    pub scrape_interval_ms: u64,
    pub rules_path: String,
    pub rpc_url: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            metrics_url: "http://localhost:8889/metrics".to_string(),
            scrape_interval_ms: 5000,
            rules_path: "rules/default.toml".to_string(),
            rpc_url: "http://localhost:8080".to_string(),
        }
    }
}

impl Config {
    pub fn from_toml(text: &str) -> anyhow::Result<Config> {
        let c: Config = toml::from_str(text)?;
        Ok(c)
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
            .context("SENTINEL_TELEGRAM_TOKEN не задан")?;
        let telegram_chat_id = std::env::var("SENTINEL_TELEGRAM_CHAT_ID")
            .context("SENTINEL_TELEGRAM_CHAT_ID не задан")?;
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
        // гарантируем отсутствие
        std::env::remove_var("SENTINEL_TELEGRAM_TOKEN");
        std::env::remove_var("SENTINEL_TELEGRAM_CHAT_ID");
        assert!(Secrets::from_env().is_err());
    }
}
