use crate::auth::session::SessionStore;
use crate::auth::password;
use crate::config::WebConfig;
use crate::probe::{ServiceProbe, VersionProbe, LogReader, SystemctlProbe, BinaryVersionProbe, JournalReader};
use crate::store::Store;
use serde::Serialize;
use std::collections::{HashMap, VecDeque};
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

#[derive(Debug, Clone, Serialize)]
pub struct AlertRecord {
    pub ts_ms: i64,
    pub rule: String,
    pub message: String,
}

#[derive(Clone)]
pub struct AppState {
    pub cfg: Arc<WebConfig>,
    pub store: Arc<Store>,
    pub sessions: Arc<SessionStore>,
    pub creds: Arc<Mutex<Creds>>,
    pub login_throttle: Arc<Mutex<Throttle>>,
    pub now: NowFn,
    /// In-memory time series for alert evaluation.
    pub live: Arc<Mutex<sentinel_agent::state::State>>,
    /// Alert rule engine (carries firing state between ticks).
    pub engine: Arc<Mutex<sentinel_agent::rules::Engine>>,
    /// Last ~100 fired alerts (newest at back, served newest-first).
    pub alerts: Arc<Mutex<VecDeque<AlertRecord>>>,
    /// Probe trait objects — real impls in production; fakes in tests.
    pub svc_probe: Arc<dyn ServiceProbe>,
    pub ver_probe: Arc<dyn VersionProbe>,
    pub logs: Arc<dyn LogReader>,
}

const DEFAULT_RULES_TOML: &str = include_str!("../../sentinel-agent/rules/default.toml");

/// Construct a real AppState from config and an env-supplied password.
/// `SENTINEL_ADMIN_PASSWORD` must be set. Full bootstrap (TOTP, etc.) is done in Task 8.
pub fn from_config(cfg: WebConfig, admin_pw: &str) -> anyhow::Result<AppState> {
    let pw_phc = password::hash(admin_pw)?;
    let store = Store::open(&cfg.db_path)?;
    let engine = sentinel_agent::rules::Engine::from_toml(DEFAULT_RULES_TOML)?;
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
        live: Arc::new(Mutex::new(sentinel_agent::state::State::new())),
        engine: Arc::new(Mutex::new(engine)),
        alerts: Arc::new(Mutex::new(VecDeque::new())),
        svc_probe: Arc::new(SystemctlProbe),
        ver_probe: Arc::new(BinaryVersionProbe),
        logs: Arc::new(JournalReader),
    })
}

/// Test-only helper — exposed unconditionally so integration tests in tests/ can use it.
/// Default probe fields use fakes that return safe canned data.
/// Do not call in production code.
pub fn test_state_with_password(pw: &str) -> AppState {
    let cfg = WebConfig::default();
    let store = Store::open(":memory:").unwrap();
    let engine = sentinel_agent::rules::Engine::from_toml(DEFAULT_RULES_TOML).unwrap();
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
        live: Arc::new(Mutex::new(sentinel_agent::state::State::new())),
        engine: Arc::new(Mutex::new(engine)),
        alerts: Arc::new(Mutex::new(VecDeque::new())),
        svc_probe: Arc::new(NoopServiceProbe),
        ver_probe: Arc::new(NoopVersionProbe),
        logs: Arc::new(NoopLogReader),
    }
}

// ---- Default no-op fakes for test_state_with_password ----

struct NoopServiceProbe;
impl ServiceProbe for NoopServiceProbe {
    fn is_active(&self, _unit: &str) -> bool { false }
}

struct NoopVersionProbe;
impl VersionProbe for NoopVersionProbe {
    fn version(&self, _binary: &str) -> Option<String> { None }
}

struct NoopLogReader;
impl LogReader for NoopLogReader {
    fn tail(&self, _unit: &str, _lines: usize) -> Vec<String> { vec![] }
}
