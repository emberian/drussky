use rufren_core::*;
use rufren_store::{DB_SCHEMA_VERSION, Store};
use uuid::Uuid;

#[test]
fn notebook_v4_reopens_verbatim_under_current_and_refuses_legacy_protocols() {
    assert_eq!(DB_SCHEMA_VERSION, 1);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("notebook.sqlite");
    drop(Store::open(&path).unwrap());
    let raw = rusqlite::Connection::open(&path).unwrap();
    let mut originals: Vec<_> = (1..=3)
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
    let response = serde_json::from_str(include_str!(
        "../../../scripts/platform/fixtures/teacher-notebook-response.json"
    ))
    .unwrap();
    let document_id = teacher_document_id(&response);
    let mut archive = Event::new(
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
    archive.version = 4; // Historical archive encoding must not be upgraded on read.
    let archive_json = serde_json::to_string_pretty(&archive).unwrap();
    raw.execute(
        "INSERT INTO events(id,json) VALUES (?1,?2)",
        rusqlite::params![archive.id.to_string(), &archive_json],
    )
    .unwrap();
    originals.push((archive, archive_json));
    let request_id = Uuid::new_v4();
    let request = serde_json::from_value(serde_json::json!({
        "request_id":request_id,"thread_id":request_id,"parent_reply_id":null,
        "config":{"intent":"clarification","topic":"Synthetic current dialogue","teacher_role":null,"learner_role":null},
        "action":"ask","correction":"off","anchor":null,"ancestors":[],"omitted_parent_id":null,"omitted_count":0,"text":"Why does this ending change?"
    })).unwrap();
    let mut dialogue = Event::new(
        Uuid::new_v4(),
        5,
        Fact::TeacherDialogueUserRecorded(request),
    )
    .unwrap();
    dialogue.version = 5; // Historical dialogue bytes remain5 under the current reader.
    let dialogue_json = serde_json::to_string_pretty(&dialogue).unwrap();
    raw.execute(
        "INSERT INTO events(id,json) VALUES (?1,?2)",
        rusqlite::params![dialogue.id.to_string(), &dialogue_json],
    )
    .unwrap();
    originals.push((dialogue, dialogue_json));
    let prepared: SoundTrialPrepared = serde_json::from_str(include_str!(
        "../../../scripts/platform/fixtures/sound-v6-prepared.json"
    ))
    .unwrap();
    let trial_id = prepared.trial_id;
    let mut sound = Event::new(Uuid::new_v4(), 6, Fact::SoundTrialPrepared(prepared)).unwrap();
    sound.version = 6; // Preserve the actual historical Sound envelope unchanged.
    let sound_json = serde_json::to_string_pretty(&sound).unwrap();
    raw.execute(
        "INSERT INTO events(id,json) VALUES (?1,?2)",
        rusqlite::params![sound.id.to_string(), &sound_json],
    )
    .unwrap();
    originals.push((sound, sound_json));
    drop(raw);
    let mut store = Store::open(&path).unwrap();
    assert_eq!(store.library().unwrap().events.len(), originals.len());
    let definition: ReaderDefinition = serde_json::from_str(include_str!(
        "../../../scripts/platform/fixtures/reader-v7-definition.json"
    ))
    .unwrap();
    let new = reader_document_event(&definition).unwrap();
    assert_eq!(new.version, EVENT_VERSION);
    assert!(new.version >= 7);
    for version in 3..SYNC_VERSION {
        assert!(
            store
                .exchange(&SyncRequest {
                    version,
                    relay: None,
                    after: 0,
                    events: vec![new.clone()]
                })
                .is_err()
        );
        assert_eq!(store.library().unwrap().events.len(), originals.len());
        assert!(
            store.get::<Uuid>("relay").unwrap().is_none(),
            "old protocol refusal precedes identity/cursor/write processing"
        );
    }
    let accepted = store
        .exchange(&SyncRequest {
            version: SYNC_VERSION,
            relay: None,
            after: 0,
            events: vec![new.clone()],
        })
        .unwrap();
    assert_eq!(accepted.version, SYNC_VERSION);
    assert_eq!(accepted.accepted, vec![new.id]);
    assert_eq!(accepted.events.len(), originals.len() + 1);
    drop(store);
    let reopened = Store::open(&path).unwrap();
    assert_eq!(reopened.library().unwrap().teacher_documents().len(), 1);
    assert_eq!(reopened.library().unwrap().events.get(&new.id), Some(&new));
    assert!(
        reopened
            .library()
            .unwrap()
            .dialogue_user(request_id)
            .is_some()
    );
    assert_eq!(
        reopened
            .library()
            .unwrap()
            .reader_document(new.id)
            .unwrap()
            .definition,
        definition
    );
    assert!(
        reopened
            .library()
            .unwrap()
            .sound_prepared(trial_id)
            .is_some()
    );
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
