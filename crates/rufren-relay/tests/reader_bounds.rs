//! Actual HTTP bounds for a shelf whose valid originals exceed one relay body.
//! Synthetic repeated text deliberately stresses transport, not Reader pedagogy.
use rufren_core::*;
use rufren_store::{SYNC_PAGE_BYTES, Store};
use std::collections::BTreeMap;
use std::time::Duration;

fn long_document(index: usize) -> Event {
    let mut definition: ReaderDefinition = serde_json::from_str(include_str!(
        "../../../scripts/platform/fixtures/reader-v7-definition.json"
    ))
    .unwrap();
    definition.id = format!("synthetic.reader.bounds.{index}");
    definition.original = "Я".repeat(32_000); // 64,000 UTF-8 bytes, below64KiB.
    definition.original_sha256 = reader_text_sha256(&definition.original);
    definition.chunks = (0..16)
        .map(|chunk| ReaderChunk {
            id: format!("chunk-{chunk}"),
            span: TextSpan {
                start: chunk * 4000,
                end: (chunk + 1) * 4000,
            },
            text: "Я".repeat(2000),
            stress: Some("Я".repeat(2000)),
            translation: Some("\\".repeat(2048)), // JSON escaping counts as bytes too.
            glosses: (0..3)
                .map(|_| ReaderNote {
                    span: TextSpan {
                        start: chunk * 4000,
                        end: chunk * 4000 + 2,
                    },
                    text: "Я".into(),
                    explanation: "\\".repeat(2048),
                })
                .collect(),
            grammar_notes: vec![ReaderNote {
                span: TextSpan {
                    start: chunk * 4000,
                    end: chunk * 4000 + 2,
                },
                text: "Я".into(),
                explanation: "\\".repeat(1800),
            }],
            prompts: vec![],
        })
        .collect();
    let event = reader_document_event(&definition).unwrap();
    assert!(
        (500_000..READER_MAX_DOCUMENT_BYTES).contains(&serde_json::to_vec(&event).unwrap().len())
    );
    event
}

#[tokio::test]
async fn long_reader_shelf_http_pages_retry_and_body_limit_preserve_every_original() {
    let events: Vec<_> = (0..20).map(long_document).collect();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/v1/sync", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            rufren_relay::router(
                Store::open(":memory:").unwrap(),
                "synthetic-key".into(),
                vec![],
            ),
        )
        .await
        .unwrap();
    });
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .unwrap();
    let count_only = SyncRequest {
        version: SYNC_VERSION,
        relay: None,
        after: 0,
        events: events.clone(),
    };
    let too_large = serde_json::to_vec(&count_only).unwrap();
    assert!(too_large.len() > 2 * 1024 * 1024 && events.len() < SYNC_BATCH);
    let refused = client
        .post(&url)
        .bearer_auth("synthetic-key")
        .header("Content-Type", "application/json")
        .body(too_large)
        .send()
        .await
        .unwrap();
    assert_eq!(refused.status(), reqwest::StatusCode::PAYLOAD_TOO_LARGE);
    let empty: SyncResponse = client
        .post(&url)
        .bearer_auth("synthetic-key")
        .json(&SyncRequest {
            version: SYNC_VERSION,
            relay: None,
            after: 0,
            events: vec![],
        })
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        empty.cursor, 0,
        "oversized request saved no prefix or cursor"
    );
    assert!(empty.events.is_empty());
    let relay = empty.relay;
    let mut batches: Vec<Vec<Event>> = Vec::new();
    let mut bytes = 0;
    for event in &events {
        let size = serde_json::to_vec(event).unwrap().len();
        if batches.last().is_none_or(|batch| batch.len() == SYNC_BATCH)
            || bytes + size > SYNC_PAGE_BYTES
        {
            batches.push(vec![]);
            bytes = 0;
        }
        bytes += size;
        batches.last_mut().unwrap().push(event.clone());
    }
    assert!(batches.len() > 1);
    let mut cursor = 0;
    for batch in &batches {
        let request = SyncRequest {
            version: SYNC_VERSION,
            relay: Some(relay),
            after: cursor,
            events: batch.clone(),
        };
        let body = serde_json::to_vec(&request).unwrap();
        assert!(body.len() > 900_000 && body.len() < 2 * 1024 * 1024);
        let bytes = client
            .post(&url)
            .bearer_auth("synthetic-key")
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .bytes()
            .await
            .unwrap();
        assert!(
            bytes.len() < 1_100_000,
            "response fits native/web streaming cap"
        );
        let response: SyncResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(response.accepted.len(), batch.len());
        assert_eq!(response.cursor - cursor, batch.len() as u64);
        cursor = response.cursor;
    }
    assert_eq!(cursor, events.len() as u64);
    let retry: SyncResponse = client
        .post(&url)
        .bearer_auth("synthetic-key")
        .json(&SyncRequest {
            version: SYNC_VERSION,
            relay: Some(relay),
            after: cursor,
            events: batches[0].clone(),
        })
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(retry.cursor, cursor);
    assert!(retry.events.is_empty());
    assert_eq!(retry.accepted.len(), batches[0].len());
    let mut after = 0;
    let mut downloaded = BTreeMap::new();
    let mut pages = 0;
    loop {
        let body = client
            .post(&url)
            .bearer_auth("synthetic-key")
            .json(&SyncRequest {
                version: SYNC_VERSION,
                relay: Some(relay),
                after,
                events: vec![],
            })
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .bytes()
            .await
            .unwrap();
        assert!(body.len() < 1_100_000);
        let page: SyncResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(page.relay, relay);
        assert!(page.cursor > after && page.events.len() < SYNC_BATCH);
        after = page.cursor;
        for event in page.events {
            assert!(
                downloaded.insert(event.id, event).is_none(),
                "no duplicate page event"
            );
        }
        pages += 1;
        if !page.has_more {
            break;
        }
    }
    assert!(pages > 1);
    assert_eq!(after, cursor);
    assert_eq!(
        downloaded,
        events.into_iter().map(|event| (event.id, event)).collect()
    );
    server.abort();
}
