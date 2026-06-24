use rand::RngExt;
use std::collections::HashMap;
use std::sync::Mutex;

struct Session {
    actor: String,
    created_ms: i64,
    last_ms: i64,
    ttl_ms: i64,
}

pub struct SessionStore {
    inner: Mutex<HashMap<String, Session>>,
}

impl SessionStore {
    pub fn new() -> Self {
        SessionStore { inner: Mutex::new(HashMap::new()) }
    }

    pub fn create(&self, actor: &str, now_ms: i64, ttl_ms: i64) -> String {
        let mut buf = [0u8; 32];
        rand::rng().fill(&mut buf);
        let token = hex(&buf);
        self.inner.lock().unwrap().insert(
            token.clone(),
            Session {
                actor: actor.to_string(),
                created_ms: now_ms,
                last_ms: now_ms,
                ttl_ms,
            },
        );
        token
    }

    /// Returns the actor name if the token is valid, refreshing the idle timeout.
    /// Enforces both absolute expiry (created_ms + ttl_ms) and idle timeout (30 min).
    /// Removes expired tokens on access.
    pub fn validate(&self, token: &str, now_ms: i64) -> Option<String> {
        let mut map = self.inner.lock().unwrap();
        let s = map.get_mut(token)?;
        let idle_ms: i64 = 30 * 60 * 1_000;
        if now_ms > s.created_ms + s.ttl_ms || now_ms > s.last_ms + idle_ms {
            map.remove(token);
            return None;
        }
        s.last_ms = now_ms;
        Some(s.actor.clone())
    }

    pub fn remove(&self, token: &str) {
        self.inner.lock().unwrap().remove(token);
    }
}

fn hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for x in b {
        s.push_str(&format!("{x:02x}"));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn create_validate_expire() {
        let s = SessionStore::new();
        let tok = s.create("admin", 1_000, 10_000);
        assert_eq!(s.validate(&tok, 5_000).as_deref(), Some("admin"));
        // absolute expiry past created+ttl
        assert_eq!(s.validate(&tok, 999_999), None);
    }
    #[test]
    fn remove_invalidates() {
        let s = SessionStore::new();
        let tok = s.create("admin", 0, 10_000);
        s.remove(&tok);
        assert_eq!(s.validate(&tok, 1), None);
    }
}
