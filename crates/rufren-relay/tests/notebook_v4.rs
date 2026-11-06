use rufren_core::*;
use rufren_store::Store;
use uuid::Uuid;

#[tokio::test]
async fn notebook_v4_http_refuses_v3_upload_and_download_before_mutation() {
    let mut store = Store::open(":memory:").unwrap();
    let old: Vec<_> = (1..=3)
        .map(|version| {
            let mut event = starter(Uuid::new_v4(), version as i64);
            event.version = version;
            event
        })
        .collect();
    store.append(&old).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            rufren_relay::router(store, "synthetic-key".into(), vec![]),
        )
        .await
        .unwrap()
    });
    let client = reqwest::Client::new();
    let rejected = starter(Uuid::new_v4(), 4);
    for events in [vec![], vec![rejected.clone()]] {
        let response = client
            .post(format!("{endpoint}/v1/sync"))
            .bearer_auth("synthetic-key")
            .json(&SyncRequest {
                version: 3,
                relay: None,
                after: 0,
                events,
            })
            .send()
            .await
            .unwrap();
        assert!(!response.status().is_success());
        let message = response.text().await.unwrap();
        assert!(message.contains("Unsupported data version 3"), "{message}");
        assert!(
            !message.contains("passage_revised"),
            "old clients receive no new facts"
        );
    }
    let current: SyncResponse = client
        .post(format!("{endpoint}/v1/sync"))
        .bearer_auth("synthetic-key")
        .json(&SyncRequest {
            version: 4,
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
    assert_eq!(current.version, 4);
    assert_eq!(current.events, old);
    assert_eq!(
        current.cursor, 3,
        "rejected uploads consumed no sequence/cursor"
    );
    assert!(!current.events.iter().any(|event| event.id == rejected.id));
    let health = client
        .get(format!("{endpoint}/health"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert_eq!(health, "rufren-relay/4");
    server.abort();
}

#[tokio::test]
async fn notebook_v4_http_exact_documents_and_independent_receipts_converge_collision_rolls_back() {
    let response = serde_json::from_str(include_str!(
        "../../../scripts/platform/fixtures/teacher-notebook-response.json"
    ))
    .unwrap();
    let document_id = teacher_document_id(&response);
    let first = Event::new(
        Uuid::new_v4(),
        10,
        Fact::TeacherDocumentRecorded(TeacherDocument {
            format_version: TEACHER_DOCUMENT_FORMAT_VERSION,
            document_id,
            response,
            receipt: TeacherDocumentReceipt::LiveReceived { received_at_ms: 10 },
            context_source_fact_ids: None,
        }),
    )
    .unwrap();
    let mut second = first.clone();
    second.id = Uuid::new_v4();
    second.device = Uuid::new_v4();
    second.recorded_at_ms = 20;
    if let Fact::TeacherDocumentRecorded(document) = &mut second.fact {
        document.receipt = TeacherDocumentReceipt::RecoveredLocalCache;
    }
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
        .unwrap()
    });
    let client = reqwest::Client::new();
    let mut a = Store::open(":memory:").unwrap();
    let mut b = Store::open(":memory:").unwrap();
    for (device, event) in [(&mut a, first.clone()), (&mut b, second.clone())] {
        device.append(&[event.clone()]).unwrap();
        let response: SyncResponse = client
            .post(&url)
            .bearer_auth("synthetic-key")
            .json(&SyncRequest {
                version: 4,
                relay: None,
                after: 0,
                events: vec![event],
            })
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .json()
            .await
            .unwrap();
        device
            .accept(
                &SyncBinding {
                    endpoint: url.clone(),
                    relay: response.relay,
                    cursor: response.cursor,
                },
                &response,
            )
            .unwrap();
    }
    let binding: SyncBinding = a.get("sync").unwrap().unwrap();
    let page: SyncResponse = client
        .post(&url)
        .bearer_auth("synthetic-key")
        .json(&SyncRequest {
            version: 4,
            relay: Some(binding.relay),
            after: binding.cursor,
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
    a.accept(
        &SyncBinding {
            endpoint: url.clone(),
            relay: page.relay,
            cursor: page.cursor,
        },
        &page,
    )
    .unwrap();
    assert_eq!(a.library().unwrap(), b.library().unwrap());
    assert_eq!(a.library().unwrap().teacher_documents().len(), 1);
    assert_eq!(
        a.library()
            .unwrap()
            .teacher_document_receipts(document_id)
            .len(),
        2
    );
    let mut collision = first.clone();
    collision.id = Uuid::new_v4();
    if let Fact::TeacherDocumentRecorded(document) = &mut collision.fact {
        document.response.content.explanation =
            "Changed under an immutable document identity".into();
    }
    let unrelated = starter(Uuid::new_v4(), 30);
    assert!(
        !client
            .post(&url)
            .bearer_auth("synthetic-key")
            .json(&SyncRequest {
                version: 4,
                relay: Some(binding.relay),
                after: 2,
                events: vec![unrelated, collision]
            })
            .send()
            .await
            .unwrap()
            .status()
            .is_success()
    );
    let unchanged: SyncResponse = client
        .post(&url)
        .bearer_auth("synthetic-key")
        .json(&SyncRequest {
            version: 4,
            relay: Some(binding.relay),
            after: 0,
            events: vec![first.clone(), second.clone()],
        })
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(unchanged.cursor, 2);
    assert_eq!(unchanged.events, vec![first, second]);
    server.abort();
}
