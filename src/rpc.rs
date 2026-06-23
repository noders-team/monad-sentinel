use std::time::Duration;
use anyhow::{anyhow, Context};

pub struct RpcClient {
    url: String,
    timeout: Duration,
}

impl RpcClient {
    pub fn new(url: String, timeout_ms: u64) -> Self {
        Self { url, timeout: Duration::from_millis(timeout_ms) }
    }

    /// Запрос eth_blockNumber; возвращает текущую высоту блока.
    pub fn block_number(&self) -> anyhow::Result<u64> {
        let agent = ureq::AgentBuilder::new().timeout(self.timeout).build();
        let body = agent
            .post(&self.url)
            .set("content-type", "application/json")
            .send_string("{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"eth_blockNumber\",\"params\":[]}")?
            .into_string()?;
        parse_block_hex(&body)
    }
}

/// Достаёт hex-результат eth_blockNumber из JSON-ответа без полного JSON-парсера.
pub fn parse_block_hex(body: &str) -> anyhow::Result<u64> {
    let key = "\"result\":\"0x";
    let start = body.find(key).ok_or_else(|| anyhow!("no result field in: {body}"))?;
    let rest = &body[start + key.len()..];
    let end = rest.find('"').ok_or_else(|| anyhow!("unterminated result"))?;
    let hex = &rest[..end];
    u64::from_str_radix(hex, 16).context("bad hex block number")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hex_block_number() {
        // pure parser helper, no network
        assert_eq!(parse_block_hex("{\"jsonrpc\":\"2.0\",\"result\":\"0x2604a4f\",\"id\":1}").unwrap(), 0x2604a4f);
    }

    #[test]
    fn errors_on_missing_result() {
        assert!(parse_block_hex("{\"jsonrpc\":\"2.0\",\"error\":{},\"id\":1}").is_err());
    }

    #[test]
    fn unreachable_host_errs() {
        let c = RpcClient::new("http://127.0.0.1:1".into(), 500);
        assert!(c.block_number().is_err());
    }
}
