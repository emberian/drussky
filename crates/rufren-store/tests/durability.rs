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
        version: SYNC_VERSION,
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
            version: SYNC_VERSION,
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
            version: SYNC_VERSION,
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
            version: SYNC_VERSION,
            relay,
            after: cursor,
            events: vec![new, bad]
        })
        .is_err()
    );
    assert_eq!(db.library().unwrap().events.len(), SYNC_BATCH + 3);
    assert!(
        db.exchange(&SyncRequest {
            version: SYNC_VERSION,
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
        version: SYNC_VERSION,
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
            version: SYNC_VERSION,
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

#[test]
fn durability_lost_response_retry_keeps_cursor_and_accepts_once() {
    let mut relay = Store::open(":memory:").unwrap();
    let event = starter(Uuid::new_v4(), 0);
    let request = SyncRequest {
        version: SYNC_VERSION,
        relay: None,
        after: 0,
        events: vec![event.clone()],
    };
    let lost = relay.exchange(&request).unwrap();
    let retried = relay.exchange(&request).unwrap();
    assert_eq!(retried.cursor, lost.cursor);
    assert_eq!(retried.events, lost.events);
    assert_eq!(retried.accepted, vec![event.id]);
    let next = relay
        .exchange(&SyncRequest {
            version: SYNC_VERSION,
            relay: Some(lost.relay),
            after: lost.cursor,
            events: vec![event],
        })
        .unwrap();
    assert!(next.events.is_empty());
    assert_eq!(next.cursor, lost.cursor);
}

#[test]
fn durability_invalid_ack_and_binding_cannot_advance_or_insert() {
    let mut db = Store::open(":memory:").unwrap();
    let relay = Uuid::new_v4();
    let event = starter(Uuid::new_v4(), 0);
    let binding = SyncBinding {
        endpoint: "https://private.example".into(),
        relay,
        cursor: 1,
    };
    let mut response = SyncResponse {
        version: SYNC_VERSION,
        relay,
        accepted: vec![Uuid::new_v4()],
        events: vec![event.clone()],
        cursor: 1,
        has_more: false,
    };
    assert!(db.accept(&binding, &response).is_err());
    assert!(db.library().unwrap().events.is_empty());
    assert!(db.get::<SyncBinding>("sync").unwrap().is_none());
    response.accepted = vec![event.id];
    db.accept(&binding, &response).unwrap();
    db.accept(&binding, &response).unwrap(); // replaying an already committed page is harmless
    let unchanged = db.library().unwrap();
    response.cursor = 2;
    assert!(db.accept(&binding, &response).is_err()); // binding/response disagree
    response.events.clear();
    assert!(
        db.accept(
            &SyncBinding {
                cursor: 2,
                ..binding.clone()
            },
            &response
        )
        .is_err()
    ); // empty advancing page
    assert_eq!(db.library().unwrap(), unchanged);
    assert_eq!(db.get::<SyncBinding>("sync").unwrap().unwrap(), binding);
}

#[test]
fn durability_projection_observes_other_connection_and_schema_preserves_v1() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("library.sqlite");
    let mut a = Store::open(&path).unwrap();
    let mut b = Store::open(&path).unwrap();
    let mut legacy = starter(Uuid::new_v4(), 0);
    legacy.version = 1;
    a.append(&[legacy.clone()]).unwrap();
    assert_eq!(b.library().unwrap().events[&legacy.id].version, 1);
    let current = starter(Uuid::new_v4(), 1);
    b.append(&[current]).unwrap();
    assert_eq!(a.library().unwrap().events.len(), 2);
    drop((a, b));
    let connection = rusqlite::Connection::open(&path).unwrap();
    let physical: u32 = connection
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .unwrap();
    assert_eq!(physical, rufren_store::DB_SCHEMA_VERSION);
    connection.pragma_update(None, "user_version", 999).unwrap();
    assert!(Store::open(&path).is_err());
    assert_eq!(
        connection
            .query_row("SELECT COUNT(*) FROM events", [], |r| r.get::<_, u64>(0))
            .unwrap(),
        2
    );
}

#[test]
fn durability_download_pages_are_byte_bounded_and_resume_without_loss() {
    let mut relay = Store::open(":memory:").unwrap();
    let events: Vec<_> = (0..20)
        .map(|_| {
            let mut event = starter(Uuid::new_v4(), 0);
            if let Fact::PassageRevised(ref mut r) = event.fact {
                r.passage.russian = "я".repeat(16_000);
                r.passage.translation = "\\".repeat(32_000);
                r.passage.source = "\n".repeat(32_000);
            }
            event
        })
        .collect();
    relay.append(&events).unwrap();
    let mut after = 0;
    let mut id = None;
    let mut seen = std::collections::BTreeSet::new();
    loop {
        let page = relay
            .exchange(&SyncRequest {
                version: SYNC_VERSION,
                relay: id,
                after,
                events: vec![],
            })
            .unwrap();
        assert!(serde_json::to_vec(&page).unwrap().len() < 1_100_000);
        assert!(page.cursor > after);
        for event in &page.events {
            assert!(seen.insert(event.id));
        }
        after = page.cursor;
        id = Some(page.relay);
        if !page.has_more {
            break;
        }
    }
    assert_eq!(seen.len(), events.len());
}

#[test]
fn durability_current_reopens_legacy_v1_v2_without_rewriting_and_rejects_old_protocol() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("legacy.sqlite");
    drop(Store::open(&path).unwrap());
    let device = Uuid::new_v4();
    let mut passage = starter(device, 1);
    passage.version = 1;
    let mut attempt = Event::new(
        device,
        2,
        Fact::AttemptRecorded(Attempt {
            session: Uuid::new_v4(),
            revision: passage.id,
            skill: Skill::Reading,
            response: "Legacy meaning response".into(),
            cues: vec![Cue::ReferenceMaterial],
            self_rating: SelfRating::Recalled,
            policy: "legacy-policy-v2".into(),
        }),
    )
    .unwrap();
    attempt.version = 2;
    let originals = vec![passage.clone(), attempt.clone()];
    let original_json: Vec<_> = originals
        .iter()
        .map(|event| serde_json::to_string_pretty(event).unwrap())
        .collect();
    {
        let connection = rusqlite::Connection::open(&path).unwrap();
        for (event, json) in originals.iter().zip(&original_json) {
            connection
                .execute(
                    "INSERT INTO events(id,json) VALUES (?1,?2)",
                    rusqlite::params![event.id.to_string(), json],
                )
                .unwrap();
        }
    }
    let mut store = Store::open(&path).unwrap();
    for event in &originals {
        assert_eq!(&store.library().unwrap().events[&event.id], event);
    }
    assert!(store.get::<Uuid>("relay").unwrap().is_none());
    let mut unsaved = starter(device, 3);
    unsaved.version = 2;
    assert!(
        store
            .exchange(&SyncRequest {
                version: 2,
                relay: None,
                after: 0,
                events: vec![unsaved]
            })
            .is_err()
    );
    assert!(store.get::<Uuid>("relay").unwrap().is_none()); // rejected before relay identity or data changes
    assert_eq!(store.library().unwrap().events.len(), 2);
    let current = starter(device, 4);
    assert_eq!(current.version, EVENT_VERSION);
    store.append(&[current.clone()]).unwrap();
    let page = store
        .exchange(&SyncRequest {
            version: SYNC_VERSION,
            relay: None,
            after: 0,
            events: vec![],
        })
        .unwrap();
    assert_eq!(page.version, SYNC_VERSION);
    assert_eq!(page.events, vec![passage, attempt, current]);
    drop(store);
    let connection = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        connection
            .pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))
            .unwrap(),
        1
    );
    for (event, json) in originals.iter().zip(&original_json) {
        let saved: String = connection
            .query_row(
                "SELECT json FROM events WHERE id=?1",
                [event.id.to_string()],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(&saved, json); // not just equivalent parsed data: the historical encoding is retained verbatim
    }
}
