#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let app = sentinel_web::app::build_router();
    let addr = "127.0.0.1:8088";
    let listener = tokio::net::TcpListener::bind(addr).await?;
    eprintln!("sentinel-web listening on http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}
