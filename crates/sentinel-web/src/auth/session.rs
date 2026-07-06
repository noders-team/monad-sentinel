use rand::RngExt;
use std::collections::HashMap;
use std::sync::Mutex;

struct Session {
    actor: String,
    created_ms: i64,
    last_ms: i64,
    ttl_ms: i64,
}

/// Idle timeout: a session not presented for this long is dead.
const IDLE_MS: i64 = 30 * 60 * 1_000;

pub struct SessionStore {
    inner: Mutex<HashMap<String, Session>>,
}

impl Default for SessionStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionStore {
    pub fn new() -> Self {
        SessionStore { inner: Mutex::new(HashMap::new()) }
    }

    /// 256-bit CSPRNG token, hex-encoded. Also used standalone for the CSRF
    /// value, which is a pure double-submit token and is never stored.
    pub fn random_token() -> String {
        let mut buf = [0u8; 32];
        rand::rng().fill(&mut buf);
        hex(&buf)
    }

    pub fn create(&self, actor: &str, now_ms: i64, ttl_ms: i64) -> String {
        let token = Self::random_token();
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
        if now_ms > s.created_ms + s.ttl_ms || now_ms > s.last_ms + IDLE_MS {
            map.remove(token);
            return None;
        }
        s.last_ms = now_ms;
        Some(s.actor.clone())
    }

    pub fn remove(&self, token: &str) {
        self.inner.lock().unwrap().remove(token);
    }

    /// Drop every expired session. `validate` only prunes tokens that are
    /// presented again, so abandoned sessions need this periodic sweep to keep
    /// the store from growing for the lifetime of the process.
    pub fn sweep(&self, now_ms: i64) {
        self.inner.lock().unwrap().retain(|_, s| {
            now_ms <= s.created_ms + s.ttl_ms && now_ms <= s.last_ms + IDLE_MS
        });
    }

    pub fn len(&self) -> usize {
        self.inner.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
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

    #[test]
    fn random_token_does_not_grow_the_store() {
        let s = SessionStore::new();
        let t1 = SessionStore::random_token();
        let t2 = SessionStore::random_token();
        assert_ne!(t1, t2);
        assert_eq!(t1.len(), 64);
        assert_eq!(s.len(), 0);
    }

    #[test]
    fn sweep_removes_expired_sessions() {
        let s = SessionStore::new();
        let _live = s.create("admin", 1_000_000, 10_000_000);
        let _dead = s.create("admin", 0, 1_000); // absolute expiry long past
        assert_eq!(s.len(), 2);
        s.sweep(2_000_000);
        assert_eq!(s.len(), 1, "expired session must be swept");
    }
}
