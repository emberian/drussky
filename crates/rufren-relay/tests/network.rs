use rufren_core::*;
use rufren_store::Store;
use uuid::Uuid;

async fn exchange(client: &reqwest::Client, url: &str, store: &mut Store, events: Vec<Event>) {
    let binding = store.get::<SyncBinding>("sync").unwrap();
    let request = SyncRequest {
        version: SYNC_VERSION,
        relay: binding.as_ref().map(|b| b.relay),
        after: binding.map_or(0, |b| b.cursor),
        events,
    };
    let response: SyncResponse = client
        .post(url)
        .bearer_auth("test-key")
        .json(&request)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    store
        .accept(
            &SyncBinding {
                endpoint: url.into(),
                relay: response.relay,
                cursor: response.cursor,
            },
            &response,
        )
        .unwrap();
}

#[tokio::test]
async fn network_two_offline_devices_converge_and_unauthorized_writes_fail() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/v1/sync", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            rufren_relay::router(Store::open(":memory:").unwrap(), "test-key".into(), vec![]),
        )
        .await
        .unwrap()
    });
    let client = reqwest::Client::new();
    let mut a = Store::open(":memory:").unwrap();
    let mut b = Store::open(":memory:").unwrap();
    let base = starter(Uuid::new_v4(), 0);
    let bevent = starter(Uuid::new_v4(), 0);
    a.append(&[base.clone()]).unwrap();
    b.append(&[bevent.clone()]).unwrap();
    let request = SyncRequest {
        version: SYNC_VERSION,
        relay: None,
        after: 0,
        events: vec![base.clone()],
    };
    assert_eq!(
        client
            .post(&url)
            .json(&request)
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    exchange(&client, &url, &mut a, vec![base.clone()]).await;
    exchange(&client, &url, &mut b, vec![bevent]).await;
    exchange(&client, &url, &mut a, vec![base.clone()]).await;
    assert_eq!(a.library().unwrap(), b.library().unwrap());
    assert_eq!(a.library().unwrap().events.len(), 2);
    let relay = a.get::<SyncBinding>("sync").unwrap().unwrap().relay;
    assert_eq!(a.acknowledged(relay).unwrap().len(), 1);

    let revise = |title: &str| {
        let Fact::PassageRevised(mut revision) = base.fact.clone() else {
            unreachable!()
        };
        revision.passage.title = title.into();
        revision.parents = [base.id].into();
        Event::new(Uuid::new_v4(), 0, Fact::PassageRevised(revision)).unwrap()
    };
    let ae = revise("Wording from device A");
    let be = revise("Wording from device B");
    a.append(&[ae.clone()]).unwrap();
    b.append(&[be.clone()]).unwrap();
    exchange(&client, &url, &mut a, vec![ae]).await;
    exchange(&client, &url, &mut b, vec![be]).await;
    exchange(&client, &url, &mut a, vec![]).await;
    assert_eq!(a.library().unwrap(), b.library().unwrap());
    assert_eq!(a.library().unwrap().events.len(), 4);
    assert!(a.library().unwrap().heads().values().any(|v| v.len() == 2));
    server.abort();
}

#[tokio::test]
async fn network_auth_precedes_decode_and_old_protocol_cannot_write() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            rufren_relay::router(Store::open(":memory:").unwrap(), "test-key".into(), vec![]),
        )
        .await
        .unwrap();
    });
    let client = reqwest::Client::new();
    let sync = format!("{base}/v1/sync");
    assert_eq!(
        client
            .post(&sync)
            .header("Content-Type", "application/json")
            .body("{broken")
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    let old = SyncRequest {
        version: 1,
        relay: None,
        after: 0,
        events: vec![starter(Uuid::new_v4(), 0)],
    };
    assert_eq!(
        client
            .post(&sync)
            .bearer_auth("test-key")
            .json(&old)
            .send()
            .await
            .unwrap()
            .status(),
        409
    );
    let empty = SyncRequest {
        version: SYNC_VERSION,
        relay: None,
        after: 0,
        events: vec![],
    };
    let page: SyncResponse = client
        .post(&sync)
        .bearer_auth("test-key")
        .json(&empty)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(page.events.is_empty());
    assert_eq!(
        client
            .post(&sync)
            .bearer_auth("test-key")
            .header("Content-Type", "application/json")
            .body("x".repeat(2 * 1024 * 1024 + 1))
            .send()
            .await
            .unwrap()
            .status(),
        413
    );
    assert_eq!(
        client
            .get(format!("{base}/api/tutor/status"))
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    let status: serde_json::Value = client
        .get(format!("{base}/api/tutor/status"))
        .bearer_auth("test-key")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(status["enabled"], false);
    let tutor = serde_json::json!({"task":"explain", "text":"Привет", "learner_context":null});
    assert_eq!(
        client
            .post(format!("{base}/api/tutor/suggest"))
            .bearer_auth("test-key")
            .json(&tutor)
            .send()
            .await
            .unwrap()
            .status(),
        503
    );
    server.abort();
}

#[tokio::test]
async fn network_teacher_and_speech_require_auth_and_report_disabled() {
    let token = "teacher-test-private-key-not-a-real-key";
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let router = rufren_relay::router(Store::open(":memory:").unwrap(), token.into(), vec![]);
    let server = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let client = reqwest::Client::new();
    for path in ["/api/teacher/status", "/api/speech/status"] {
        assert_eq!(
            client
                .get(format!("{url}{path}"))
                .send()
                .await
                .unwrap()
                .status(),
            401
        );
        let status: serde_json::Value = client
            .get(format!("{url}{path}"))
            .bearer_auth(token)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(status["enabled"], false);
    }
    for path in ["/api/teacher/teach", "/api/speech/generate"] {
        assert_eq!(
            client
                .post(format!("{url}{path}"))
                .body("malformedJSON")
                .send()
                .await
                .unwrap()
                .status(),
            401
        );
    }
    let teacher = serde_json::json!({"mode":"teach_lesson","input":{"request_id":"request-1","lesson_id":"lesson-1","title":"Location","objective":"Teach location.","material":"Я в школе.","learner_context":null}});
    assert_eq!(
        client
            .post(format!("{url}/api/teacher/teach"))
            .bearer_auth(token)
            .json(&teacher)
            .send()
            .await
            .unwrap()
            .status(),
        503
    );
    let speech = serde_json::json!({"request_id":"request-1","revision_id":null,"text":"Привет."});
    assert_eq!(
        client
            .post(format!("{url}/api/speech/generate"))
            .bearer_auth(token)
            .json(&speech)
            .send()
            .await
            .unwrap()
            .status(),
        503
    );
    server.abort();
}

#[tokio::test]
async fn network_recognition_auth_disabled_bounds_and_exact_cancel() {
    let token = "recognition-test-key-not-a-real-credential";
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let router = rufren_relay::router(Store::open(":memory:").unwrap(), token.into(), vec![]);
    let server = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let client = reqwest::Client::new();
    assert_eq!(
        client
            .get(format!("{url}/health"))
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap(),
        format!("rufren-relay/{SYNC_VERSION}")
    );
    for path in ["/api/speech/transcribe", "/api/speech/transcribe/cancel"] {
        assert_eq!(
            client
                .post(format!("{url}{path}"))
                .body("not JSON")
                .send()
                .await
                .unwrap()
                .status(),
            401
        );
    }
    assert_eq!(
        client
            .get(format!("{url}/api/speech/recognition/status"))
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    let status: serde_json::Value = client
        .get(format!("{url}/api/speech/recognition/status"))
        .bearer_auth(token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(status["enabled"], false);
    let request = serde_json::json!({"request_id":"request-1","asset_id":"asset-1","attempt_id":null,"revision_id":null,"language":"ru","audio_sha256":"0".repeat(64),"wav_base64":"AA==","span":null});
    assert_eq!(
        client
            .post(format!("{url}/api/speech/transcribe"))
            .bearer_auth(token)
            .json(&request)
            .send()
            .await
            .unwrap()
            .status(),
        503
    );
    // The ASR route's explicit allowance must override the general 2 MiB cap.
    let mut above_general_limit = request.clone();
    above_general_limit["wav_base64"] = serde_json::Value::String("A".repeat(2_100_000));
    assert_eq!(
        client
            .post(format!("{url}/api/speech/transcribe"))
            .bearer_auth(token)
            .json(&above_general_limit)
            .send()
            .await
            .unwrap()
            .status(),
        503
    );
    let cancelled: serde_json::Value = client
        .post(format!("{url}/api/speech/transcribe/cancel"))
        .bearer_auth(token)
        .json(&serde_json::json!({"request_id":"request-1"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(cancelled["cancelled"], true);
    assert_eq!(cancelled["active"], false);
    // Cancel may arrive while the same request's body is still uploading.
    assert_eq!(
        client
            .post(format!("{url}/api/speech/transcribe"))
            .bearer_auth(token)
            .json(&request)
            .send()
            .await
            .unwrap()
            .status(),
        409
    );
    assert_eq!(
        client
            .post(format!("{url}/api/speech/transcribe/cancel"))
            .bearer_auth(token)
            .json(&serde_json::json!({"request_id":"../other"}))
            .send()
            .await
            .unwrap()
            .status(),
        400
    );
    assert_eq!(
        client
            .post(format!("{url}/api/speech/transcribe"))
            .bearer_auth(token)
            .header("Content-Type", "application/json")
            .body(vec![b' '; rufren_speech::RECOGNITION_MAX_REQUEST_BYTES + 1])
            .send()
            .await
            .unwrap()
            .status(),
        413
    );
    server.abort();
}

#[tokio::test]
async fn network_current_refuses_protocol2_push_and_pull_before_history() {
    let mut store = Store::open(":memory:").unwrap();
    let mut first = starter(Uuid::new_v4(), 1);
    first.version = 1;
    let mut second = starter(Uuid::new_v4(), 2);
    second.version = 2;
    store.append(&[first.clone(), second.clone()]).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}/v1/sync", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            rufren_relay::router(store, "compatibility-key".into(), vec![]),
        )
        .await
        .unwrap();
    });
    let client = reqwest::Client::new();
    let mut rejected = starter(Uuid::new_v4(), 3);
    rejected.version = 2;
    for events in [vec![], vec![rejected]] {
        let response = client
            .post(&endpoint)
            .bearer_auth("compatibility-key")
            .json(&SyncRequest {
                version: 2,
                relay: None,
                after: 0,
                events,
            })
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 409);
        let error = response.text().await.unwrap();
        assert!(!error.contains(&first.id.to_string()));
        assert!(!error.contains(&second.id.to_string()));
        assert!(!error.contains("\"events\""));
    }
    let page: SyncResponse = client
        .post(&endpoint)
        .bearer_auth("compatibility-key")
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
    assert_eq!(page.version, SYNC_VERSION);
    assert_eq!(page.events, vec![first, second]);
    assert_eq!(page.cursor, 2);
    server.abort();
}

#[tokio::test]
async fn network_dialogue_auth_precedes_decode_disabled_is_explicit_and_upload_bounded() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            rufren_relay::router(
                Store::open(":memory:").unwrap(),
                "dialogue-test-key".into(),
                vec![],
            ),
        )
        .await
        .unwrap();
    });
    let client = reqwest::Client::new();
    let url = format!("{base}/api/teacher/dialogue");
    assert_eq!(
        client
            .post(&url)
            .body("not JSON")
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    let request = serde_json::json!({"request_id":"10000000-0000-4000-8000-000000000001","thread_id":"10000000-0000-4000-8000-000000000001","parent_reply_id":null,
        "config":{"intent":"clarification","topic":"Location","teacher_role":null,"learner_role":null},"action":"ask","correction":"off","anchor":null,"ancestors":[],"omitted_parent_id":null,"omitted_count":0,"text":"Why is it в школе?"});
    assert_eq!(
        client
            .post(&url)
            .bearer_auth("dialogue-test-key")
            .json(&request)
            .send()
            .await
            .unwrap()
            .status(),
        503
    );
    let mut invalid = request.clone();
    invalid["text"] = serde_json::json!("x".repeat(2049));
    assert_eq!(
        client
            .post(&url)
            .bearer_auth("dialogue-test-key")
            .json(&invalid)
            .send()
            .await
            .unwrap()
            .status(),
        400
    );
    invalid["text"] = serde_json::json!("x".repeat(32769));
    assert_eq!(
        client
            .post(&url)
            .bearer_auth("dialogue-test-key")
            .json(&invalid)
            .send()
            .await
            .unwrap()
            .status(),
        413
    );
    let status: serde_json::Value = client
        .get(format!("{base}/api/teacher/status"))
        .bearer_auth("dialogue-test-key")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(status["enabled"], false);
    assert!(status.get("dialogue").is_none());
    server.abort();
}
