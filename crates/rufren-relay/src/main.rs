use rufren_store::Store;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if let [flag, path] = arguments.as_slice() {
        if flag == "--check-db" {
            let store = Store::open(path)?;
            let mut after = String::new();
            let mut recordings = 0;
            loop {
                let page = store.audio_records(&after)?;
                recordings += page.records.len();
                match page.next_after {
                    Some(next) => after = next,
                    None => break,
                }
            }
            println!(
                "Verified {} immutable events and {} recording metadata records",
                store.library()?.events.len(),
                recordings
            );
            return Ok(());
        }
    }
    if !arguments.is_empty() {
        return Err("Usage: rufren-relay [--check-db PATH]".into());
    }
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
    let asset_path =
        std::env::var("RUFREN_RELAY_ASSETS").unwrap_or_else(|_| format!("{path}.assets"));
    let asset_quota = std::env::var("RUFREN_RELAY_ASSET_QUOTA_BYTES")
        .unwrap_or_else(|_| "536870912".into())
        .parse::<u64>()?;
    let assets = rufren_relay::assets::AssetStore::open(asset_path, asset_quota)
        .map_err(|_| "Could not open relay audio storage")?;
    axum::serve(
        listener,
        rufren_relay::router_with_ai_services(
            Store::open(path)?,
            token,
            origins,
            rufren_tutor::TutorService::from_env()?,
            Some(assets),
            rufren_speech::SpeechService::from_env()?,
            rufren_speech::RecognitionService::from_env()?,
        ),
    )
    .with_graceful_shutdown(async {
        let _ = tokio::signal::ctrl_c().await;
    })
    .await?;
    Ok(())
}
