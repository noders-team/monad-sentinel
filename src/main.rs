use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use clap::{Parser, Subcommand};
use sentinel_agent::config::{Config, Secrets};
use sentinel_agent::notify::telegram::Telegram;
use sentinel_agent::parse::parse;
use sentinel_agent::rpc::RpcClient;
use sentinel_agent::rules::Engine;
use sentinel_agent::scrape::Scraper;
use sentinel_agent::state::State;
use sentinel_agent::time::now_ms;

#[derive(Parser)]
#[command(name = "sentinel-agent")]
struct Cli {
    #[arg(long, default_value = "sentinel.toml")]
    config: PathBuf,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Запустить цикл алертинга
    Run,
    /// Одноразовая проверка (dry-run, без Telegram)
    Check,
}

fn load(config_path: &PathBuf) -> anyhow::Result<(Config, Engine)> {
    let cfg_text = match std::fs::read_to_string(config_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("warning: config {} not read ({e}); using defaults", config_path.display());
            String::new()
        }
    };
    let cfg = Config::from_toml(&cfg_text)?;
    let rules_text = std::fs::read_to_string(&cfg.rules_path)?;
    let engine = Engine::from_toml(&rules_text)?;
    Ok((cfg, engine))
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let (cfg, mut engine) = load(&cli.config)?;
    let scraper = Scraper::new(cfg.metrics_url.clone(), 8000);
    let rpc = RpcClient::new(cfg.rpc_url.clone(), 8000);

    match cli.cmd {
        Cmd::Check => {
            let body = scraper.scrape()?;
            let now = now_ms();
            let snap = parse(&body, now);
            let mut state = State::new();
            let tracked: Vec<String> =
                engine.tracked_metrics().iter().map(|s| s.to_string()).collect();
            let refs: Vec<&str> = tracked.iter().map(|s| s.as_str()).collect();
            state.update(&snap, &refs);
            match rpc.block_number() {
                Ok(bn) => state.record("rpc_block_number", now, bn as f64),
                Err(e) => eprintln!("rpc error: {e:#}"),
            }
            for fired in engine.evaluate(&state, &snap, now) {
                println!(
                    "[{:?}] {} -> {:?}: {}",
                    fired.severity, fired.rule_id, fired.transition, fired.message
                );
            }
            println!("check done; {} серий в снимке", snap.samples.len());
            Ok(())
        }
        Cmd::Run => {
            let secrets = Secrets::from_env()?; // fail-fast
            let notifier = Telegram::new(&secrets);
            let mut state = State::new();
            let tracked: Vec<String> =
                engine.tracked_metrics().iter().map(|s| s.to_string()).collect();
            let refs: Vec<&str> = tracked.iter().map(|s| s.as_str()).collect();

            let stop = Arc::new(AtomicBool::new(false));
            signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&stop))?;
            signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&stop))?;

            eprintln!("sentinel-agent: старт, scrape {} мс", cfg.scrape_interval_ms);
            while !stop.load(Ordering::Relaxed) {
                let now = now_ms();
                let snap = match scraper.scrape() {
                    Ok(body) => {
                        let snap = parse(&body, now);
                        state.update(&snap, &refs);
                        // прогрев адаптивных базлайнов по rate отслеживаемых метрик
                        for &m in &refs {
                            if let Some(r) = state.series(m).and_then(|s| s.rate(60_000)) {
                                state.observe_baseline(m, r);
                            }
                        }
                        snap
                    }
                    Err(e) => {
                        eprintln!("scrape error: {e:#}");
                        // пустой снимок → сработает absent-правило scrape_down
                        parse("", now)
                    }
                };
                // RPC poll runs every tick, independent of scrape outcome
                match rpc.block_number() {
                    Ok(bn) => state.record("rpc_block_number", now, bn as f64),
                    Err(e) => eprintln!("rpc error: {e:#}"),
                }
                let sent =
                    sentinel_agent::run_once(&mut engine, &state, &snap, now, &notifier);
                if sent > 0 {
                    eprintln!("отправлено нотификаций: {sent}");
                }
                std::thread::sleep(Duration::from_millis(cfg.scrape_interval_ms));
            }
            eprintln!("sentinel-agent: остановлен");
            Ok(())
        }
    }
}
