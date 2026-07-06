use clap::Parser;
use sentinel_web::auth::password;
use sentinel_web::auth::totp::{generate_secret_base32, Totp};
use sentinel_web::config::WebConfig;
use sentinel_web::state::from_config_with_creds;
use sentinel_web::store::Store;

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

    // Bootstrap credentials — idempotent.
    //
    // On first run (no creds row in the database):
    //   • SENTINEL_ADMIN_PASSWORD must be set — its hash is persisted.
    //   • A fresh TOTP secret is generated, persisted, and the enrollment banner is written
    //     ONCE to an owner-only file next to the database (never to stderr/journald).
    //
    // On subsequent starts:
    //   • Persisted credentials are loaded from the database; SENTINEL_ADMIN_PASSWORD is ignored.
    //   • The enrollment file is NOT written again (the secret has not changed).
    //   • This means a crash + Restart=on-failure does NOT invalidate the operator's 2FA enrollment.
    //
    // Password note: SENTINEL_ADMIN_PASSWORD is only consumed on first bootstrap. To change the
    // password after initial setup, use the admin password-change endpoint or wipe the creds row.
    let bootstrap_store = Store::open(&cfg.db_path)?;
    let (pw_phc, totp_secret) = match bootstrap_store.get_creds()? {
        Some((pw_phc, totp_secret)) => {
            // Credentials already persisted — reuse them without printing the enrollment URI.
            eprintln!("sentinel-web: credentials loaded from database (TOTP enrollment unchanged)");
            (pw_phc, totp_secret)
        }
        None => {
            // First run — require the password env var and generate a new TOTP secret.
            let admin_pw = std::env::var("SENTINEL_ADMIN_PASSWORD")
                .map_err(|_| anyhow::anyhow!("SENTINEL_ADMIN_PASSWORD must be set on first run"))?;
            let pw_phc = password::hash(&admin_pw)?;

            let rng_bytes: [u8; 20] = {
                use rand::RngExt;
                let mut b = [0u8; 20];
                rand::rng().fill(&mut b);
                b
            };
            let totp_secret = generate_secret_base32(rng_bytes);

            bootstrap_store.set_creds(&pw_phc, &totp_secret)?;

            // Verify the secret is decodable before printing (sanity check).
            let _totp = Totp::from_base32(&totp_secret)?;

            // Write the secret to an owner-only file instead of stderr: journald
            // retains stderr indefinitely, which would hand the second factor
            // to anyone with journal access.
            let enroll_path = std::path::Path::new(&cfg.db_path)
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or_else(|| std::path::Path::new("."))
                .join("totp-enroll.txt");
            sentinel_web::auth::totp::write_enrollment_file(&enroll_path, &totp_secret)?;
            eprintln!("=== TOTP ENROLLMENT ===");
            eprintln!(
                "Enrollment secret written to {} — scan it ONCE with your authenticator app, then delete the file.",
                enroll_path.display()
            );
            eprintln!("=======================");

            (pw_phc, totp_secret)
        }
    };
    // The bootstrap store is dropped here; from_config_with_creds opens a fresh connection.
    drop(bootstrap_store);

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

    let api_router = sentinel_web::app::build_router(state);
    let spa = tower_http::services::ServeDir::new(&cfg.frontend_dist)
        .fallback(tower_http::services::ServeFile::new(
            format!("{}/index.html", cfg.frontend_dist),
        ));
    // Re-apply the security headers at the outermost layer so the SPA
    // fallback (added after build_router's own layer) is covered as well.
    let app = api_router
        .fallback_service(spa)
        .layer(axum::middleware::from_fn(sentinel_web::app::security_headers));
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
