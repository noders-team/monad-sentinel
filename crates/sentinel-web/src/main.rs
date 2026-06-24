#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = sentinel_web::config::WebConfig::default();
    let addr = cfg.listen_addr.clone();

    let admin_pw = std::env::var("SENTINEL_ADMIN_PASSWORD")
        .map_err(|_| anyhow::anyhow!("SENTINEL_ADMIN_PASSWORD must be set"))?;

    let state = sentinel_web::state::from_config(cfg, &admin_pw)?;
    let app = sentinel_web::app::build_router(state);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    eprintln!("sentinel-web listening on http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}
