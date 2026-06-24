use crate::auth::session::SessionStore;
use crate::auth::password;
use crate::config::WebConfig;
use crate::store::Store;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub type NowFn = Arc<dyn Fn() -> i64 + Send + Sync>;

#[derive(Clone)]
pub struct Creds {
    pub pw_phc: String,
    pub totp_secret_b32: String,
}

#[derive(Default)]
pub struct Throttle {
    /// username/ip → (fail_count, window_start_ms)
    pub fails: HashMap<String, (u32, i64)>,
}

#[derive(Clone)]
pub struct AppState {
    pub cfg: Arc<WebConfig>,
    pub store: Arc<Store>,
    pub sessions: Arc<SessionStore>,
    pub creds: Arc<Mutex<Creds>>,
    pub login_throttle: Arc<Mutex<Throttle>>,
    pub now: NowFn,
}

/// Construct a real AppState from config and an env-supplied password.
/// `SENTINEL_ADMIN_PASSWORD` must be set. Full bootstrap (TOTP, etc.) is done in Task 8.
pub fn from_config(cfg: WebConfig, admin_pw: &str) -> anyhow::Result<AppState> {
    let pw_phc = password::hash(admin_pw)?;
    let store = Store::open(&cfg.db_path)?;
    Ok(AppState {
        sessions: Arc::new(SessionStore::new()),
        creds: Arc::new(Mutex::new(Creds {
            pw_phc,
            totp_secret_b32: crate::auth::totp::generate_secret_base32([0u8; 20]),
        })),
        cfg: Arc::new(cfg),
        store: Arc::new(store),
        login_throttle: Arc::new(Mutex::new(Throttle::default())),
        now: Arc::new(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64
        }),
    })
}

/// Test-only helper — exposed unconditionally so integration tests in tests/ can use it.
/// Do not call in production code.
pub fn test_state_with_password(pw: &str) -> AppState {
    let cfg = WebConfig::default();
    let store = Store::open(":memory:").unwrap();
    AppState {
        cfg: Arc::new(cfg),
        store: Arc::new(store),
        sessions: Arc::new(SessionStore::new()),
        creds: Arc::new(Mutex::new(Creds {
            pw_phc: password::hash(pw).unwrap(),
            totp_secret_b32: crate::auth::totp::generate_secret_base32([7u8; 20]),
        })),
        login_throttle: Arc::new(Mutex::new(Throttle::default())),
        now: Arc::new(|| 0),
    }
}
