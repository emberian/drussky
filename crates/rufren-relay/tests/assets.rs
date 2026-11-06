use rufren_core::*;
use rufren_relay::assets::AssetStore;
use rufren_store::Store;
use sha2::{Digest, Sha256};
use uuid::Uuid;

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn record(id: &str, bytes: &[u8]) -> AudioRecordMetadata {
    AudioRecordMetadata {
        asset_id: id.into(),
        content_hash: hash(bytes),
        role: "attempt".into(),
        name: "Synthetic transport fixture".into(),
        passage_id: String::new(),
        revision_id: String::new(),
        duration_ms: 100,
        sample_rate: 48000,
        channels: 1,
        frames: 4800,
        byte_length: bytes.len() as u64,
        created_at_ms: 1,
        mime_type: "audio/wav".into(),
        interrupted: false,
        device: serde_json::json!({}),
        annotations: vec![],
    }
}
async fn start(root: &std::path::Path, quota: u64) -> (String, Uuid, tokio::task::JoinHandle<()>) {
    let store = Store::open(root.join("relay.sqlite")).unwrap();
    let relay = store.identity("relay").unwrap();
    let assets = AssetStore::open(root.join("assets"), quota).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            rufren_relay::router_with_services(
                store,
                "asset-test-key".into(),
                vec![],
                rufren_tutor::TutorService::disabled(),
                Some(assets),
            ),
        )
        .await
        .unwrap();
    });
    (base, relay, server)
}
fn request(
    client: &reqwest::Client,
    method: reqwest::Method,
    url: String,
    relay: Uuid,
) -> reqwest::RequestBuilder {
    client
        .request(method, url)
        .bearer_auth("asset-test-key")
        .header("X-Rufren-Relay", relay.to_string())
}

#[tokio::test]
async fn asset_http_verification_retry_distinct_takes_and_cue_union() {
    let directory = tempfile::tempdir().unwrap();
    let (base, relay, server) = start(directory.path(), 1024 * 1024).await;
    let client = reqwest::Client::new();
    let bytes = b"original encoded fixture";
    let digest = hash(bytes);
    let blob = format!("{base}/v1/assets/blobs/{digest}");
    assert_eq!(
        client
            .put(&blob)
            .body(bytes.to_vec())
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    assert_eq!(
        request(&client, reqwest::Method::PUT, blob.clone(), Uuid::new_v4())
            .body(bytes.to_vec())
            .send()
            .await
            .unwrap()
            .status(),
        409
    );
    assert_eq!(
        request(
            &client,
            reqwest::Method::PUT,
            format!("{base}/v1/assets/blobs/not-a-hash"),
            relay
        )
        .body(bytes.to_vec())
        .send()
        .await
        .unwrap()
        .status(),
        400
    );
    assert_eq!(
        request(&client, reqwest::Method::PUT, blob.clone(), relay)
            .body(b"corruption".to_vec())
            .send()
            .await
            .unwrap()
            .status(),
        409
    );
    assert!(
        !directory
            .path()
            .join("assets")
            .join(format!("{digest}.blob"))
            .exists()
    );
    for _ in 0..2 {
        assert_eq!(
            request(&client, reqwest::Method::PUT, blob.clone(), relay)
                .body(bytes.to_vec())
                .send()
                .await
                .unwrap()
                .status(),
            204
        );
    }
    let head = request(&client, reqwest::Method::HEAD, blob.clone(), relay)
        .send()
        .await
        .unwrap();
    assert_eq!(head.status(), 200);
    assert_eq!(head.headers()["content-length"], bytes.len().to_string());
    assert_eq!(
        &request(&client, reqwest::Method::GET, blob.clone(), relay)
            .send()
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap()[..],
        bytes
    );
    let mut first = record("take-a", bytes);
    let mut second = record("take-b", bytes);
    second.sample_rate = 44100;
    second.frames = 4410;
    for take in [&first, &second] {
        assert_eq!(
            request(
                &client,
                reqwest::Method::PUT,
                format!("{base}/v1/assets/records/{}", take.asset_id),
                relay
            )
            .json(take)
            .send()
            .await
            .unwrap()
            .status(),
            200
        );
    }
    let stale = first.clone();
    first.device = serde_json::json!({"source_mime_type":"audio/wav"});
    first.annotations.push(TimedAudioCue {
        start_seconds: 0.01,
        end_seconds: 0.04,
        note: "First device".into(),
        created_at_ms: 2,
    });
    let cue_url = format!("{base}/v1/assets/records/take-a");
    assert_eq!(
        request(&client, reqwest::Method::PUT, cue_url.clone(), relay)
            .json(&first)
            .send()
            .await
            .unwrap()
            .status(),
        200
    );
    let mut concurrent = stale;
    concurrent.annotations.push(TimedAudioCue {
        start_seconds: 0.04,
        end_seconds: 0.08,
        note: "Second device".into(),
        created_at_ms: 3,
    });
    for _ in 0..2 {
        let merged: AudioRecordMetadata =
            request(&client, reqwest::Method::PUT, cue_url.clone(), relay)
                .json(&concurrent)
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap();
        assert_eq!(merged.annotations.len(), 2);
    }
    concurrent.name = "Changing immutable identity".into();
    assert_eq!(
        request(&client, reqwest::Method::PUT, cue_url, relay)
            .json(&concurrent)
            .send()
            .await
            .unwrap()
            .status(),
        409
    );
    let page: AssetPage = request(
        &client,
        reqwest::Method::GET,
        format!("{base}/v1/assets"),
        relay,
    )
    .send()
    .await
    .unwrap()
    .json()
    .await
    .unwrap();
    assert_eq!(page.records.len(), 2);
    assert_eq!(page.records[0].annotations.len(), 2);
    assert_eq!(page.records[0].device["source_mime_type"], "audio/wav");
    assert_eq!(page.records[1].sample_rate, 44100);
    // Files changed outside the relay cannot silently play as verified originals.
    std::fs::write(
        directory
            .path()
            .join("assets")
            .join(format!("{digest}.blob")),
        b"damaged disk bytes",
    )
    .unwrap();
    assert_eq!(
        request(&client, reqwest::Method::GET, blob, relay)
            .send()
            .await
            .unwrap()
            .status(),
        409
    );
    server.abort();
}

#[tokio::test]
async fn asset_http_quota_bounds_and_metadata_require_committed_bytes() {
    let directory = tempfile::tempdir().unwrap();
    let (base, relay, server) = start(directory.path(), 8).await;
    let client = reqwest::Client::new();
    let first = b"12345678";
    let second = b"87654321";
    let take = record("missing-first", first);
    assert_eq!(
        request(
            &client,
            reqwest::Method::PUT,
            format!("{base}/v1/assets/records/{}", take.asset_id),
            relay
        )
        .json(&take)
        .send()
        .await
        .unwrap()
        .status(),
        404
    );
    assert_eq!(
        request(
            &client,
            reqwest::Method::PUT,
            format!("{base}/v1/assets/blobs/{}", hash(first)),
            relay
        )
        .body(first.to_vec())
        .send()
        .await
        .unwrap()
        .status(),
        204
    );
    assert_eq!(
        request(
            &client,
            reqwest::Method::PUT,
            format!("{base}/v1/assets/blobs/{}", hash(second)),
            relay
        )
        .body(second.to_vec())
        .send()
        .await
        .unwrap()
        .status(),
        507
    );
    let oversized = vec![0u8; MAX_AUDIO_BYTES + 1];
    let invalid_hash = hash(&oversized);
    assert_eq!(
        request(
            &client,
            reqwest::Method::PUT,
            format!("{base}/v1/assets/blobs/{invalid_hash}"),
            relay
        )
        .body(oversized)
        .send()
        .await
        .unwrap()
        .status(),
        413
    );
    assert_eq!(
        std::fs::read_dir(directory.path().join("assets"))
            .unwrap()
            .count(),
        1
    );
    let page: AssetPage = request(
        &client,
        reqwest::Method::GET,
        format!("{base}/v1/assets"),
        relay,
    )
    .send()
    .await
    .unwrap()
    .json()
    .await
    .unwrap();
    assert!(page.records.is_empty());
    server.abort();
}
