use rufren_store::Store;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let token = std::env::var("RUFREN_SYNC_TOKEN")
        .map_err(|_| "Set RUFREN_SYNC_TOKEN to a random key of at least 32 characters")?;
    if token.len() < 32 {
        return Err("RUFREN_SYNC_TOKEN must contain at least 32 characters".into());
    }
    let path = std::env::var("RUFREN_RELAY_DB").unwrap_or_else(|_| "private/relay.sqlite".into());
    if let Some(parent) = std::path::Path::new(&path)
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    let origins = std::env::var("RUFREN_ALLOWED_ORIGINS")
        .unwrap_or_else(|_| "http://127.0.0.1:8080,http://localhost:8080".into())
        .split(',')
        .map(|s| s.trim().parse())
        .collect::<Result<Vec<_>, _>>()?;
    let address = std::env::var("RUFREN_RELAY_BIND").unwrap_or_else(|_| "127.0.0.1:8787".into());
    let listener = tokio::net::TcpListener::bind(&address).await?;
    println!("Rufren sync listening on {}", listener.local_addr()?);
    axum::serve(
        listener,
        rufren_relay::router(Store::open(path)?, token, origins),
    )
    .with_graceful_shutdown(async {
        let _ = tokio::signal::ctrl_c().await;
    })
    .await?;
    Ok(())
}
