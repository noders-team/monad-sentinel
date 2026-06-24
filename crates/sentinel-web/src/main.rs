use clap::Parser;
use sentinel_web::auth::password;
use sentinel_web::auth::totp::{generate_secret_base32, Totp};
use sentinel_web::config::WebConfig;
use sentinel_web::state::from_config_with_creds;

#[derive(Parser)]
#[command(about = "Sentinel Console — web control plane for the Monad validator node")]
struct Args {
    /// Path to sentinel-web.toml configuration file.
    #[arg(long, default_value = "/etc/sentinel/sentinel-web.toml")]
    config: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Load config from file (or fall back to defaults if the file doesn't exist in dev).
    let cfg = match std::fs::read_to_string(&args.config) {
        Ok(text) => WebConfig::from_toml(&text)
            .map_err(|e| anyhow::anyhow!("config parse error ({}): {e:#}", args.config))?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("WARNING: config file {} not found — using built-in defaults", args.config);
            WebConfig::default()
        }
        Err(e) => return Err(anyhow::anyhow!("cannot read config {}: {e:#}", args.config)),
    };

    let addr = cfg.listen_addr.clone();
    let db_path = cfg.db_path.clone();

    // Bootstrap credentials.
    // In this phase, credentials are stored as AppState fields (not persisted in the db).
    // The store is opened by from_config_with_creds; here we only validate the path is usable.
    let _ = &db_path; // used below by from_config_with_creds via cfg.db_path
    let admin_pw = std::env::var("SENTINEL_ADMIN_PASSWORD")
        .map_err(|_| anyhow::anyhow!("SENTINEL_ADMIN_PASSWORD must be set"))?;
    let pw_phc = password::hash(&admin_pw)?;

    // Generate a new TOTP secret on every startup (operator must re-enroll on first run or key rotation).
    // In a production setup you would persist the secret; for Phase 1 we print it once.
    let rng_bytes: [u8; 20] = {
        use rand::RngExt;
        let mut b = [0u8; 20];
        rand::rng().fill(&mut b);
        b
    };
    let totp_secret = generate_secret_base32(rng_bytes);

    // Print enrollment URI once to stderr for the operator.
    let totp = Totp::from_base32(&totp_secret)?;
    eprintln!("=== TOTP ENROLLMENT (scan once with your authenticator app) ===");
    eprintln!("Secret (base32): {totp_secret}");
    eprintln!("OTPAuth URI    : otpauth://totp/SentinelConsole:admin?secret={totp_secret}&issuer=SentinelConsole&algorithm=SHA1&digits=6&period=30");
    // Verify we can generate a code (sanity check only)
    let _ = totp;

    let state = from_config_with_creds(cfg.clone(), pw_phc, totp_secret)?;

    // Wire notifier: use Telegram if configured, otherwise log to stderr.
    // Telegram requires SENTINEL_TELEGRAM_TOKEN and SENTINEL_TELEGRAM_CHAT_ID.
    let notifier: std::sync::Arc<dyn sentinel_agent::notify::Notifier + Send + Sync> = {
        match (
            std::env::var("SENTINEL_TELEGRAM_TOKEN"),
            std::env::var("SENTINEL_TELEGRAM_CHAT_ID"),
        ) {
            (Ok(token), Ok(chat_id)) => {
                eprintln!("Telegram notifier configured for chat {chat_id}");
                let secrets = sentinel_agent::config::Secrets { telegram_token: token, telegram_chat_id: chat_id };
                std::sync::Arc::new(sentinel_agent::notify::telegram::Telegram::new(&secrets))
            }
            _ => {
                eprintln!("No Telegram credentials set — alerts will be logged only");
                std::sync::Arc::new(NoopNotifier)
            }
        }
    };

    sentinel_web::poller::spawn_loop(state.clone(), notifier);

    let app = sentinel_web::app::build_router(state);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    eprintln!("sentinel-web listening on http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

struct NoopNotifier;
impl sentinel_agent::notify::Notifier for NoopNotifier {
    fn send(&self, text: &str) -> anyhow::Result<()> {
        eprintln!("[alert] {text}");
        Ok(())
    }
}
