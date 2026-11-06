use rufren_core::*;
use rufren_store::Store;
use uuid::Uuid;

#[test]
fn durability_reopen_and_retry_preserve_events_and_device() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("library.sqlite");
    let mut db = Store::open(&path).unwrap();
    let device = db.identity("device").unwrap();
    let event = starter(device, 0);
    db.append(&[event.clone(), event.clone()]).unwrap();
    drop(db);
    let db = Store::open(&path).unwrap();
    assert_eq!(db.identity("device").unwrap(), device);
    assert_eq!(db.library().unwrap().events.len(), 1);
}

#[test]
fn durability_cursor_pages_and_collision_roll_back() {
    let mut db = Store::open(":memory:").unwrap();
    let mut cursor = 0;
    let mut relay = None;
    let mut count = 0;
    let events: Vec<_> = (0..SYNC_BATCH + 3)
        .map(|_| starter(Uuid::new_v4(), 0))
        .collect();
    db.append(&events).unwrap();
    let request = SyncRequest {
        version: 1,
        relay,
        after: cursor,
        events: vec![],
    };
    let response = db.exchange(&request).unwrap();
    cursor = response.cursor;
    relay = Some(response.relay);
    count += response.events.len();
    assert_eq!(count, SYNC_BATCH);
    assert!(response.has_more);
    let page2 = db
        .exchange(&SyncRequest {
            version: 1,
            relay,
            after: cursor,
            events: vec![],
        })
        .unwrap();
    count += page2.events.len();
    cursor = page2.cursor;
    assert_eq!(count, SYNC_BATCH + 3);
    assert!(!page2.has_more);
    let retry = db
        .exchange(&SyncRequest {
            version: 1,
            relay,
            after: cursor,
            events: vec![],
        })
        .unwrap();
    assert!(retry.events.is_empty());
    let base = response.events[0].clone();
    let mut bad = base.clone();
    bad.recorded_at_ms += 1;
    let new = starter(Uuid::new_v4(), 0);
    assert!(
        db.exchange(&SyncRequest {
            version: 1,
            relay,
            after: cursor,
            events: vec![new, bad]
        })
        .is_err()
    );
    assert_eq!(db.library().unwrap().events.len(), SYNC_BATCH + 3);
    assert!(
        db.exchange(&SyncRequest {
            version: 1,
            relay: Some(Uuid::new_v4()),
            after: 0,
            events: vec![]
        })
        .is_err()
    );
}

#[test]
fn durability_disconnect_and_rejected_download_preserve_local_state() {
    let mut db = Store::open(":memory:").unwrap();
    let relay = Uuid::new_v4();
    let binding = SyncBinding {
        endpoint: "http://localhost".into(),
        relay,
        cursor: 4,
    };
    db.set("sync", &binding).unwrap();
    let mut bad = starter(Uuid::new_v4(), 0);
    bad.version = 999;
    let response = SyncResponse {
        version: 1,
        relay,
        accepted: vec![Uuid::new_v4()],
        events: vec![bad],
        cursor: 5,
        has_more: false,
    };
    assert!(
        db.accept(
            &SyncBinding {
                cursor: 5,
                ..binding.clone()
            },
            &response
        )
        .is_err()
    );
    assert_eq!(db.get::<SyncBinding>("sync").unwrap().unwrap(), binding);
    assert!(db.acknowledged(relay).unwrap().is_empty());
    db.set("sync", &Option::<SyncBinding>::None).unwrap();
    assert!(db.get::<SyncBinding>("sync").unwrap().is_none());
    let event = starter(Uuid::new_v4(), 0);
    db.accept(
        &binding,
        &SyncResponse {
            version: 1,
            relay,
            accepted: vec![event.id],
            events: vec![event],
            cursor: 4,
            has_more: false,
        },
    )
    .unwrap();
    db.disconnect().unwrap();
    assert!(db.get::<SyncBinding>("sync").unwrap().is_none());
    assert!(db.acknowledged(relay).unwrap().is_empty());
    assert_eq!(db.library().unwrap().events.len(), 1);
}
