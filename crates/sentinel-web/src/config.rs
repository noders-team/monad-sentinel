use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServiceKind { Bft, Execution, Rpc, Other }

#[derive(Debug, Clone, Deserialize)]
pub struct ServiceCfg {
    pub name: String,
    pub unit: String,
    pub binary: String,
    pub kind: ServiceKind,
    #[serde(default)]
    pub github_repo: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct WebConfig {
    pub listen_addr: String,
    pub db_path: String,
    pub scrape_interval_ms: u64,
    pub metrics_url: String,
    pub dev_insecure_cookies: bool,
    pub services: Vec<ServiceCfg>,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            listen_addr: "127.0.0.1:8088".to_string(),
            db_path: "sentinel-web.sqlite".to_string(),
            scrape_interval_ms: 5000,
            metrics_url: "http://localhost:8889/metrics".to_string(),
            dev_insecure_cookies: false,
            services: vec![
                ServiceCfg { name: "BFT".into(), unit: "monad-bft.service".into(),
                    binary: "/usr/local/bin/monad-node".into(), kind: ServiceKind::Bft,
                    github_repo: Some("category-labs/monad-bft".into()) },
                ServiceCfg { name: "Execution".into(), unit: "monad-execution.service".into(),
                    binary: "/usr/local/bin/monad".into(), kind: ServiceKind::Execution,
                    github_repo: Some("category-labs/monad".into()) },
                ServiceCfg { name: "RPC".into(), unit: "monad-rpc.service".into(),
                    binary: "/usr/local/bin/monad-rpc".into(), kind: ServiceKind::Rpc,
                    github_repo: None },
            ],
        }
    }
}

impl WebConfig {
    pub fn from_toml(text: &str) -> anyhow::Result<WebConfig> {
        Ok(toml::from_str(text)?)
    }
    /// Units the ops layer is permitted to touch (the allowlist source of truth).
    pub fn allowed_units(&self) -> Vec<String> {
        self.services.iter().map(|s| s.unit.clone()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_node_facts() {
        let c = WebConfig::from_toml("").expect("parse");
        assert_eq!(c.listen_addr, "127.0.0.1:8088");
        assert!(!c.dev_insecure_cookies);
        let units: Vec<&str> = c.services.iter().map(|s| s.unit.as_str()).collect();
        assert_eq!(units, ["monad-bft.service", "monad-execution.service", "monad-rpc.service"]);
        let bft = c.services.iter().find(|s| s.kind == ServiceKind::Bft).unwrap();
        assert_eq!(bft.binary, "/usr/local/bin/monad-node");
        assert_eq!(bft.github_repo.as_deref(), Some("category-labs/monad-bft"));
    }

    #[test]
    fn override_listen_addr() {
        let c = WebConfig::from_toml("listen_addr = \"127.0.0.1:9000\"").expect("parse");
        assert_eq!(c.listen_addr, "127.0.0.1:9000");
    }
}
