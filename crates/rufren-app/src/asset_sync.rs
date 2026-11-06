//! Explicit audio sync: per-take metadata/cues plus content-addressed original bytes.
use crate::repository::{Repository, Result};
use base64::Engine;
use rufren_core::*;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::rc::Rc;

const CHUNK: usize = 256 * 1024;
async fn bridge<T: DeserializeOwned>(request: Value) -> Result<T> {
    let script = format!(
        "const request = {request};\n{}",
        include_str!("asset_sync.js")
    );
    let result = dioxus::document::eval(&script)
        .await
        .map_err(|e| e.to_string())?;
    if let Some(error) = result.get("error").and_then(Value::as_str) {
        return Err(error.into());
    }
    serde_json::from_value(
        result
            .get("value")
            .cloned()
            .ok_or("Empty recording storage response")?,
    )
    .map_err(|e| e.to_string())
}
/// Read the exact attached original, never a selected or most recent recording.
/// The caller separately checks the returned take's passage/revision binding.
pub(crate) async fn read_original(
    asset_id: &str,
    expected_hash: &str,
    max_bytes: usize,
) -> Result<(AudioRecordMetadata, Vec<u8>)> {
    read_original_with(asset_id, expected_hash, max_bytes, bridge::<Value>).await
}

async fn read_original_with<F, Fut>(
    asset_id: &str,
    expected_hash: &str,
    max_bytes: usize,
    mut read: F,
) -> Result<(AudioRecordMetadata, Vec<u8>)>
where
    F: FnMut(Value) -> Fut,
    Fut: std::future::Future<Output = Result<Value>>,
{
    if !valid_audio_hash(expected_hash) || max_bytes == 0 {
        return Err("The recording attachment has an invalid hash or size limit".into());
    }
    let metadata: Option<AudioRecordMetadata> =
        serde_json::from_value(read(json!({"op":"get","id":asset_id})).await?)
            .map_err(|_| "The attached recording has invalid saved metadata")?;
    let metadata = metadata.ok_or("The attached original is not on this device. Sync recordings or restore its audio backup first.")?;
    metadata.validate().map_err(|e| e.to_string())?;
    if metadata.asset_id != asset_id || metadata.content_hash != expected_hash {
        return Err("The saved recording does not match this exact attachment".into());
    }
    let length =
        usize::try_from(metadata.byte_length).map_err(|_| "The original recording is too large")?;
    if length > max_bytes.min(MAX_AUDIO_BYTES) {
        return Err(format!(
            "This original recording exceeds the {}-byte limit",
            max_bytes.min(MAX_AUDIO_BYTES)
        ));
    }
    let mut bytes = Vec::with_capacity(length);
    let mut digest = Sha256::new();
    while bytes.len() < length {
        let requested = CHUNK.min(length - bytes.len());
        let value =
            read(json!({"op":"read","id":asset_id,"offset":bytes.len(),"length":requested}))
                .await?;
        let encoded = value
            .as_str()
            .ok_or("The recording bridge returned invalid bytes")?;
        if encoded.len() > requested.div_ceil(3) * 4 {
            return Err("The recording bridge exceeded the requested byte limit".into());
        }
        let chunk = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|_| "The recording bridge returned invalid base64 bytes")?;
        if chunk.len() != requested {
            return Err("The original recording ended before its declared byte length".into());
        }
        digest.update(&chunk);
        bytes.extend_from_slice(&chunk);
    }
    // Hashing a declared prefix is insufficient: appended bytes must also be rejected.
    let end = read(json!({"op":"read","id":asset_id,"offset":length,"length":1})).await?;
    if end.as_str() != Some("") {
        return Err("The original recording has bytes beyond its declared length".into());
    }
    if format!("{:x}", digest.finalize()) != expected_hash {
        return Err(
            "The original recording failed its SHA-256 check. Restore a verified audio backup."
                .into(),
        );
    }
    // A different tab may append timed cues while we await chunks, but it cannot
    // change this take's identity or its original byte/decoding metadata.
    let current: Option<AudioRecordMetadata> =
        serde_json::from_value(read(json!({"op":"get","id":asset_id})).await?)
            .map_err(|_| "The attached recording metadata changed while reading")?;
    let current = current.ok_or("The attached original disappeared while reading")?;
    let metadata = metadata.merge(&current).map_err(|e| e.to_string())?;
    Ok((metadata, bytes))
}

#[derive(serde::Deserialize)]
struct HttpOutput {
    status: u16,
    body: String,
}
struct Transfer {
    base: String,
    token: String,
    relay: uuid::Uuid,
    client: reqwest::Client,
}
impl Transfer {
    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base, path)
    }
    async fn http(&self, method: &str, path: &str, body: Option<Value>) -> Result<HttpOutput> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = &self.client;
            bridge(json!({"op":"http","url":self.url(path),"method":method,"body":body,"token":self.token,"relay":self.relay})).await
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let method =
                reqwest::Method::from_bytes(method.as_bytes()).map_err(|e| e.to_string())?;
            let mut request = self
                .client
                .request(method, self.url(path))
                .bearer_auth(&self.token)
                .header("X-Rufren-Relay", self.relay.to_string());
            if let Some(body) = body {
                request = request.json(&body);
            }
            let mut response = request
                .send()
                .await
                .map_err(|e| format!("Recording sync could not reach your relay: {e}"))?;
            let status = response.status().as_u16();
            let mut bytes = Vec::new();
            while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
                if bytes.len() + chunk.len() > 300000 {
                    return Err("Recording metadata response exceeds its size limit".into());
                }
                bytes.extend_from_slice(&chunk);
            }
            Ok(HttpOutput {
                status,
                body: String::from_utf8(bytes)
                    .map_err(|_| "Invalid recording metadata response")?,
            })
        }
    }
    async fn upload(&self, record: &AudioRecordMetadata) -> Result<()> {
        #[cfg(target_arch = "wasm32")]
        {
            let result:HttpOutput=bridge(json!({"op":"http","url":self.url(&format!("/v1/assets/blobs/{}",record.content_hash)),"method":"PUT","upload_id":record.asset_id,"token":self.token,"relay":self.relay})).await?;
            accepted(result)?;
            Ok(())
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let (current, bytes) =
                read_original(&record.asset_id, &record.content_hash, MAX_AUDIO_BYTES).await?;
            record.merge(&current).map_err(|e| e.to_string())?;
            let response = self
                .client
                .put(self.url(&format!("/v1/assets/blobs/{}", record.content_hash)))
                .bearer_auth(&self.token)
                .header("X-Rufren-Relay", self.relay.to_string())
                .header("Content-Type", "application/octet-stream")
                .body(bytes)
                .send()
                .await
                .map_err(|e| e.to_string())?;
            if !response.status().is_success() {
                return Err(format!(
                    "Recording upload returned {}. Saved originals were kept; retry sync.",
                    response.status()
                ));
            }
            Ok(())
        }
    }
    async fn download(&self, record: &AudioRecordMetadata) -> Result<()> {
        #[cfg(target_arch = "wasm32")]
        {
            let _:AudioRecordMetadata=bridge(json!({"op":"download","url":self.url(&format!("/v1/assets/blobs/{}",record.content_hash)),"method":"GET","record":record,"token":self.token,"relay":self.relay})).await?;
            Ok(())
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut response = self
                .client
                .get(self.url(&format!("/v1/assets/blobs/{}", record.content_hash)))
                .bearer_auth(&self.token)
                .header("X-Rufren-Relay", self.relay.to_string())
                .send()
                .await
                .map_err(|e| e.to_string())?;
            if !response.status().is_success() {
                return Err(format!(
                    "Recording download returned {}. Sync it from its original device first.",
                    response.status()
                ));
            }
            if response
                .content_length()
                .is_some_and(|n| n != record.byte_length)
            {
                return Err("Recording download length does not match metadata".into());
            }
            let mut bytes = Vec::with_capacity(record.byte_length as usize);
            while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
                if bytes.len() + chunk.len() > record.byte_length as usize
                    || bytes.len() + chunk.len() > MAX_AUDIO_BYTES
                {
                    return Err("Recording download exceeds its declared size".into());
                }
                bytes.extend_from_slice(&chunk);
            }
            if bytes.len() != record.byte_length as usize
                || format!("{:x}", Sha256::digest(&bytes)) != record.content_hash
            {
                return Err("Downloaded recording failed its SHA-256 or length check. Existing recordings were kept.".into());
            }
            let _: bool = bridge(json!({"op":"begin","record":record})).await?;
            for (index, chunk) in bytes.chunks(CHUNK).enumerate() {
                let _:bool=bridge(json!({"op":"chunk","id":record.asset_id,"offset":index*CHUNK,"base64":base64::engine::general_purpose::STANDARD.encode(chunk)})).await?;
            }
            let _: AudioRecordMetadata =
                bridge(json!({"op":"finish","id":record.asset_id})).await?;
            Ok(())
        }
    }
}
fn accepted(response: HttpOutput) -> Result<String> {
    if !(200..300).contains(&response.status) {
        return Err(format!(
            "Recording sync returned {}: {}",
            response.status,
            response.body.chars().take(1024).collect::<String>()
        ));
    }
    Ok(response.body)
}

/// Call only after notebook facts sync, and only when the learner opts into transferring audio.
pub async fn synchronize_audio(
    repo: Rc<Repository>,
    endpoint: String,
    token: String,
) -> Result<usize> {
    let base = endpoint.trim().trim_end_matches('/').to_owned();
    let url = reqwest::Url::parse(&base).map_err(|_| "Enter a full sync URL")?;
    let local = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
    if (url.scheme() != "https" && !(url.scheme() == "http" && local))
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("Use a private HTTPS relay URL, or HTTP on localhost for development".into());
    }
    let binding: SyncBinding = repo
        .get("sync")?
        .ok_or("Sync notebook facts before transferring recordings")?;
    if binding.endpoint != base || token.is_empty() {
        return Err("Recording sync must use your connected notebook relay and key".into());
    }
    #[cfg(not(target_arch = "wasm32"))]
    let client = crate::native_http::client(&base, std::time::Duration::from_secs(60))?;
    #[cfg(target_arch = "wasm32")]
    let client = reqwest::Client::builder()
        .build()
        .map_err(|e| e.to_string())?;
    let transfer = Transfer {
        base,
        token,
        relay: binding.relay,
        client,
    };
    let mut moved = 0;
    let mut after = String::new();
    // Upload original bytes before advertising the take; failed retries cannot create a dangling registry entry.
    for _ in 0..10000 {
        let page: AssetPage = bridge(json!({"op":"inventory","after":after})).await?;
        for record in page.records {
            record.validate().map_err(|e| e.to_string())?;
            let exists = transfer
                .http(
                    "HEAD",
                    &format!("/v1/assets/blobs/{}", record.content_hash),
                    None,
                )
                .await?;
            if exists.status == 404 {
                transfer.upload(&record).await?;
                moved += 1;
            } else {
                accepted(exists)?;
            }
            let json = accepted(
                transfer
                    .http(
                        "PUT",
                        &format!("/v1/assets/records/{}", record.asset_id),
                        Some(serde_json::to_value(&record).map_err(|e| e.to_string())?),
                    )
                    .await?,
            )?;
            let merged: AudioRecordMetadata =
                serde_json::from_str(&json).map_err(|e| e.to_string())?;
            let merged = record.merge(&merged).map_err(|e| e.to_string())?;
            let _: AudioRecordMetadata = bridge(json!({"op":"merge","record":merged})).await?;
        }
        match page.next_after {
            Some(next) if next > after => after = next,
            Some(_) => return Err("Recording storage returned an invalid page cursor".into()),
            None => break,
        }
    }
    after.clear();
    for _ in 0..10000 {
        let mut url =
            reqwest::Url::parse(&transfer.url("/v1/assets")).map_err(|e| e.to_string())?;
        url.query_pairs_mut().append_pair("after", &after);
        let path = format!("/v1/assets?{}", url.query().unwrap_or_default());
        let page: AssetPage =
            serde_json::from_str(&accepted(transfer.http("GET", &path, None).await?)?)
                .map_err(|e| e.to_string())?;
        if page.version != ASSET_TRANSPORT_VERSION || page.records.len() > ASSET_PAGE_SIZE {
            return Err("Unsupported recording page version or size".into());
        }
        let mut last = after.clone();
        for record in page.records {
            record.validate().map_err(|e| e.to_string())?;
            if record.asset_id <= last {
                return Err("Recording page IDs are not strictly ordered".into());
            }
            last = record.asset_id.clone();
            let exists: bool =
                bridge(json!({"op":"has","id":record.asset_id,"hash":record.content_hash})).await?;
            if !exists {
                transfer.download(&record).await?;
                moved += 1;
            } else {
                let _: AudioRecordMetadata = bridge(json!({"op":"merge","record":record})).await?;
            }
        }
        match page.next_after {
            Some(next) if next == last && next > after => after = next,
            Some(_) => return Err("Recording relay returned an invalid page cursor".into()),
            None => return Ok(moved),
        }
    }
    Err("Recording sync paused after many pages. Retry to continue safely.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        cell::RefCell,
        collections::BTreeMap,
        future::{Ready, ready},
        task::{Context, Poll, Waker},
    };

    fn run<F: std::future::Future>(future: F) -> F::Output {
        let mut future = std::pin::pin!(future);
        match future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
        {
            Poll::Ready(result) => result,
            Poll::Pending => panic!("The fake IDB bridge must complete synchronously"),
        }
    }
    fn metadata(id: &str, bytes: &[u8]) -> AudioRecordMetadata {
        AudioRecordMetadata {
            asset_id: id.into(),
            content_hash: format!("{:x}", Sha256::digest(bytes)),
            role: "attempt".into(),
            name: "Original byte reader fixture".into(),
            passage_id: String::new(),
            revision_id: String::new(),
            duration_ms: 100,
            sample_rate: 48_000,
            channels: 1,
            frames: 4_800,
            byte_length: bytes.len() as u64,
            created_at_ms: 1,
            mime_type: "audio/wav".into(),
            interrupted: false,
            device: json!({}),
            annotations: vec![],
        }
    }
    fn fixture(
        records: BTreeMap<String, (AudioRecordMetadata, Vec<u8>)>,
    ) -> (
        impl FnMut(Value) -> Ready<Result<Value>>,
        Rc<RefCell<Vec<Value>>>,
    ) {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let observed = calls.clone();
        (
            move |request: Value| {
                observed.borrow_mut().push(request.clone());
                let id = request["id"].as_str().unwrap();
                ready(match request["op"].as_str().unwrap() {
                    "get" => Ok(json!(records.get(id).map(|(record, _)| record))),
                    "read" => match records.get(id) {
                        Some((_, bytes)) => {
                            let offset = request["offset"].as_u64().unwrap() as usize;
                            let length = request["length"].as_u64().unwrap() as usize;
                            let start = offset.min(bytes.len());
                            let end = start.saturating_add(length).min(bytes.len());
                            Ok(json!(
                                base64::engine::general_purpose::STANDARD
                                    .encode(&bytes[start..end])
                            ))
                        }
                        None => Err("No such original recording".into()),
                    },
                    _ => panic!("Unexpected write/network operation in original reader"),
                })
            },
            calls,
        )
    }

    #[test]
    fn original_read_exact_id_crosses_chunk_boundary_without_latest_substitution() {
        let wanted = vec![17; CHUNK + 19];
        let record = metadata("wanted-take", &wanted);
        let mut latest = metadata("newest-other-take", b"different original");
        latest.created_at_ms = 999;
        let (read, calls) = fixture(BTreeMap::from([
            (record.asset_id.clone(), (record.clone(), wanted.clone())),
            (
                latest.asset_id.clone(),
                (latest, b"different original".to_vec()),
            ),
        ]));
        let (actual, bytes) = run(read_original_with(
            &record.asset_id,
            &record.content_hash,
            wanted.len(),
            read,
        ))
        .unwrap();
        assert_eq!(actual, record);
        assert_eq!(bytes, wanted);
        assert!(
            calls
                .borrow()
                .iter()
                .all(|request| request["id"] == "wanted-take")
        );
        assert_eq!(
            calls
                .borrow()
                .iter()
                .filter(|request| request["op"] == "read")
                .count(),
            3
        ); // two chunks + EOF
    }

    #[test]
    fn original_read_rejects_missing_wrong_identity_hash_and_caller_cap_before_bytes() {
        let bytes = b"wanted original".to_vec();
        let record = metadata("wanted-take", &bytes);
        for (id, hash, cap) in [
            ("missing-take", record.content_hash.clone(), bytes.len()),
            ("wanted-take", "0".repeat(64), bytes.len()),
            ("wanted-take", record.content_hash.clone(), bytes.len() - 1),
            ("wanted-take", record.content_hash.clone(), 0),
        ] {
            let (read, calls) = fixture(BTreeMap::from([(
                record.asset_id.clone(),
                (record.clone(), bytes.clone()),
            )]));
            assert!(run(read_original_with(id, &hash, cap, read)).is_err());
            assert!(calls.borrow().iter().all(|request| request["op"] != "read"));
        }
        let mut wrong = record.clone();
        wrong.asset_id = "another-take".into();
        let (read, calls) = fixture(BTreeMap::from([(record.asset_id.clone(), (wrong, bytes))]));
        assert!(
            run(read_original_with(
                &record.asset_id,
                &record.content_hash,
                100,
                read
            ))
            .is_err()
        );
        assert!(calls.borrow().iter().all(|request| request["op"] != "read"));
    }

    #[test]
    fn original_read_rejects_corruption_truncation_and_appended_bytes() {
        let bytes = b"the exact original".to_vec();
        let record = metadata("wanted-take", &bytes);
        let mut corrupt = bytes.clone();
        corrupt[0] ^= 1;
        let mut appended = bytes.clone();
        appended.push(42);
        for actual in [corrupt, bytes[..bytes.len() - 1].to_vec(), appended] {
            let (read, _) = fixture(BTreeMap::from([(
                record.asset_id.clone(),
                (record.clone(), actual),
            )]));
            assert!(
                run(read_original_with(
                    &record.asset_id,
                    &record.content_hash,
                    bytes.len(),
                    read
                ))
                .is_err()
            );
        }
    }

    #[test]
    fn original_read_rejects_identity_changes_during_await_but_keeps_new_cues() {
        let bytes = b"the exact original".to_vec();
        let record = metadata("wanted-take", &bytes);
        for change_identity in [true, false] {
            let (mut underlying, _) = fixture(BTreeMap::from([(
                record.asset_id.clone(),
                (record.clone(), bytes.clone()),
            )]));
            let mut gets = 0;
            let reader = move |request: Value| {
                let get = request["op"] == "get";
                let mut value = run(underlying(request)).unwrap();
                if get {
                    gets += 1;
                    if gets == 2 {
                        if change_identity {
                            value["revision_id"] = json!("11111111-1111-4111-8111-111111111111");
                        } else {
                            value["annotations"] = json!([{"start_seconds":0.01,"end_seconds":0.05,"note":"Cue added in another tab","created_at_ms":2}]);
                        }
                    }
                }
                ready(Ok(value))
            };
            let result = run(read_original_with(
                &record.asset_id,
                &record.content_hash,
                bytes.len(),
                reader,
            ));
            if change_identity {
                assert!(result.is_err());
            } else {
                assert_eq!(result.unwrap().0.annotations.len(), 1);
            }
        }
    }
}
