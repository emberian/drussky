use rufren_core::*;
use rufren_store::Store;
use uuid::Uuid;

async fn exchange(client: &reqwest::Client, url: &str, store: &mut Store, events: Vec<Event>) {
    let binding = store.get::<SyncBinding>("sync").unwrap();
    let request = SyncRequest {
        version: SCHEMA_VERSION,
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
        version: 1,
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
