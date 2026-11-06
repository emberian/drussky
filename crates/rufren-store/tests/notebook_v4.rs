use rufren_core::*;
use rufren_store::{DB_SCHEMA_VERSION, Store};
use uuid::Uuid;

#[test]
fn notebook_v4_reopen_preserves_legacy_json_and_refuses_v3_before_writing() {
    assert_eq!(EVENT_VERSION, 4);
    assert_eq!(SYNC_VERSION, 4);
    assert_eq!(DB_SCHEMA_VERSION, 1);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("notebook.sqlite");
    drop(Store::open(&path).unwrap());
    let raw = rusqlite::Connection::open(&path).unwrap();
    let originals: Vec<_> = (1..=3)
        .map(|version| {
            let mut event = starter(Uuid::new_v4(), version as i64);
            event.version = version;
            let json = serde_json::to_string_pretty(&event).unwrap();
            raw.execute(
                "INSERT INTO events(id,json) VALUES (?1,?2)",
                rusqlite::params![event.id.to_string(), &json],
            )
            .unwrap();
            (event, json)
        })
        .collect();
    drop(raw);
    let mut store = Store::open(&path).unwrap();
    assert_eq!(store.library().unwrap().events.len(), 3);
    let response = serde_json::from_str(include_str!(
        "../../../scripts/platform/fixtures/teacher-notebook-response.json"
    ))
    .unwrap();
    let document_id = teacher_document_id(&response);
    let new = Event::new(
        Uuid::new_v4(),
        4,
        Fact::TeacherDocumentRecorded(TeacherDocument {
            format_version: TEACHER_DOCUMENT_FORMAT_VERSION,
            document_id,
            response,
            receipt: TeacherDocumentReceipt::LiveReceived { received_at_ms: 4 },
            context_source_fact_ids: None,
        }),
    )
    .unwrap();
    assert!(
        store
            .exchange(&SyncRequest {
                version: 3,
                relay: None,
                after: 0,
                events: vec![new.clone()],
            })
            .is_err()
    );
    assert_eq!(store.library().unwrap().events.len(), 3);
    assert!(
        store.get::<Uuid>("relay").unwrap().is_none(),
        "protocol refusal precedes relay identity/cursor/write processing"
    );
    let accepted = store
        .exchange(&SyncRequest {
            version: 4,
            relay: None,
            after: 0,
            events: vec![new.clone()],
        })
        .unwrap();
    assert_eq!(accepted.version, 4);
    assert_eq!(accepted.accepted, vec![new.id]);
    assert_eq!(accepted.events.len(), 4);
    drop(store);
    let reopened = Store::open(&path).unwrap();
    assert_eq!(reopened.library().unwrap().teacher_documents().len(), 1);
    assert_eq!(reopened.library().unwrap().events.get(&new.id), Some(&new));
    drop(reopened);
    let raw = rusqlite::Connection::open(&path).unwrap();
    let db_version: u32 = raw
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap();
    assert_eq!(db_version, 1);
    for (event, expected) in originals {
        let actual: String = raw
            .query_row(
                "SELECT json FROM events WHERE id=?1",
                [event.id.to_string()],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(actual, expected, "legacy event JSON is retained verbatim");
    }
}
