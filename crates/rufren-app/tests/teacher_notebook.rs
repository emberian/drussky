//! Durable notebook migration, sync/export, and exact lesson recovery. No provider calls.
#![cfg(not(target_arch = "wasm32"))]
#![allow(dead_code)]
#[path = "../src/drafts.rs"]
mod drafts;
#[path = "../src/editor_drafts.rs"]
mod editor_drafts;
#[path = "../src/exposure.rs"]
mod exposure;
#[path = "../src/learning.rs"]
mod learning;
#[path = "../src/daily_plan.rs"]
mod daily_plan;
#[path = "../src/sound_focus.rs"]
mod sound_focus;
#[path = "../src/model.rs"]
mod model;
#[path = "../src/native_connection.rs"]
mod native_connection;
#[path = "../src/native_http.rs"]
mod native_http;
#[path = "../src/recognition.rs"]
mod recognition;
#[path = "../src/repository.rs"]
mod repository;
#[path = "../src/teacher.rs"]
mod teacher;
#[path = "../src/teacher_audio_practice.rs"]
mod teacher_audio_practice;
#[path = "../src/teacher_context.rs"]
mod teacher_context;
#[path = "../src/teacher_notebook.rs"]
mod teacher_notebook;
#[path = "../src/teacher_practice.rs"]
mod teacher_practice;
#[path = "../src/teacher_revisit.rs"]
mod teacher_revisit;

use model::AppState;
use repository::Repository;
use rufren_core::*;
use rufren_tutor::*;
use serde_json::json;
use std::rc::Rc;
use uuid::Uuid;

fn app() -> AppState {
    AppState::from_repository(Rc::new(Repository::open_at(":memory:").unwrap())).unwrap()
}
fn lesson(state: &AppState, source: &str) -> TeacherResponse {
    serde_json::from_value(json!({
        "request":{"mode":"teach_lesson","input":{
            "request_id":format!("request-{source}"),"lesson_id":state.content.lessons[0].id,
            "title":format!("Synthetic lesson {source}"),"objective":"Express a short message.",
            "material":"Я читаю книгу. — I am reading a book.","learner_context":null}},
        "content":{"explanation":"Use читаю to say what I am reading.",
            "examples":[{"russian":"Я читаю книгу.","english":"I am reading a book.","note":"Synthetic example."}],
            "next_step":"Try one exercise.","caveats":[],"feedback":null,
            "practice":[
                {"prompt":"Write Russian for: I am reading a book.","expected_response":"Я читаю книгу.","teaching_note":"Accept valid alternatives.","reference_russian":"Я читаю книгу.","reference_english":"I am reading a book.","skill":"writing"},
                {"prompt":"Explain this message in English.","expected_response":"I am reading a book.","teaching_note":"Preserve the meaning.","reference_russian":"Я читаю книгу.","reference_english":"I am reading a book.","skill":"reading"}]},
        "provenance":{"provider":"synthetic","model":"no-model","response_id":source,
            "prompt_version":"notebook-fixture-v1","scope":"Synthetic persistence test.","review_status":"ai_generated_teaching"},
        "usage":{"accounting":"subscription_quota","input_tokens":0,"output_tokens":0,"estimated_micro_usd":0,"reserved_micro_usd":0}
    })).unwrap()
}
fn feedback(request: &TeacherRequest) -> TeacherResponse {
    serde_json::from_value(json!({"request":request,
        "content":{"explanation":"This preserves the intended message.","examples":[],"next_step":"Try another message.","caveats":[],"practice":[],"feedback":{"verdict":"correct","suggested_response":null,"corrections":[]}},
        "provenance":{"provider":"synthetic","model":"no-model","response_id":"feedback-one","prompt_version":"notebook-fixture-v1","scope":"Text only.","review_status":"ai_generated_teaching"},
        "usage":{"input_tokens":0,"output_tokens":0,"estimated_micro_usd":0,"reserved_micro_usd":0}
    })).unwrap()
}

#[test]
fn notebook_archive_keeps_older_exercises_drafts_exact_feedback_origin_and_local_cursor() {
    let mut state = app();
    let first = lesson(&state, "first");
    state.keep_teacher_response(&first.request, &first).unwrap();
    let first_id = teacher_document_id(&first);
    let mut pending = state.teacher_practice_draft(&first, 1).unwrap();
    pending.response = "Unfinished English answer".into();
    state.save_teacher_practice_draft(&pending).unwrap();
    let mut submitted = state.teacher_practice_draft(&first, 0).unwrap();
    submitted.response = "Я читаю книгу.".into();
    let attempt = state
        .submit_teacher_practice(&first, &mut submitted)
        .unwrap();
    let request = state.answer_feedback_request(attempt).unwrap();
    let answer = feedback(&request);
    state.keep_teacher_response(&request, &answer).unwrap();
    assert_eq!(
        state.teacher_lesson_for_feedback(&answer),
        Some((first_id, 0))
    );
    let next = lesson(&state, "second");
    state.keep_teacher_response(&next.request, &next).unwrap();
    assert_eq!(
        state.selected_teacher_document().unwrap(),
        Some(next.clone())
    );
    assert_eq!(state.teacher_notebook().len(), 3);
    state.select_teacher_document(first_id).unwrap();
    // Legacy lesson: orientation + two practices + an explicit finished-for-now cursor.
    state.set_teacher_lesson_cursor(first_id, 3).unwrap();
    assert!(state.set_teacher_lesson_cursor(first_id, 4).is_err());
    assert_eq!(state.teacher_lesson_cursor(first_id).unwrap(), 3);
    let before = state.library.clone();
    let repo = state.repo.clone();
    drop(state);
    let mut reopened = AppState::from_repository(repo).unwrap();
    assert_eq!(
        reopened.selected_teacher_document().unwrap(),
        Some(first.clone())
    );
    assert_eq!(reopened.teacher_lesson_cursor(first_id).unwrap(), 3);
    let recovered = reopened.teacher_practice_draft(&first, 1).unwrap();
    assert_eq!(recovered.id, pending.id);
    assert_eq!(recovered.response, pending.response);
    assert_eq!(
        reopened.library, before,
        "navigation/reload fabricates no exposure or completion"
    );
    assert_eq!(
        reopened.teacher_lesson_for_feedback(&answer),
        Some((first_id, 0)),
        "later lessons never replace feedback's exact origin"
    );
    let mut wrong = answer.clone();
    if let TeacherRequest::AnswerFeedback(r) = &mut wrong.request {
        r.prompt = "Another task".into();
    }
    assert_eq!(reopened.teacher_lesson_for_feedback(&wrong), None);
    let mut rejected = next.clone();
    rejected.provenance.response_id.clear();
    assert!(
        reopened
            .keep_teacher_response(&next.request, &rejected)
            .is_err()
    );
    assert_eq!(reopened.library, before);
    let exported = repository::export_library(reopened.library.clone()).unwrap();
    let imported = repository::import_library(&exported).unwrap();
    let mut remote = app();
    remote
        .repo
        .append(&imported.events.into_values().collect::<Vec<_>>())
        .unwrap();
    remote.refresh().unwrap();
    assert_eq!(remote.teacher_notebook().len(), 3);
    assert_eq!(
        remote.library.teacher_document(first_id).unwrap().response,
        first
    );
    assert_eq!(remote.selected_teacher_document().unwrap(), None);
    assert_eq!(
        remote.teacher_lesson_cursor(first_id).unwrap(),
        0,
        "cursor and mutable drafts are local"
    );
    assert_eq!(
        remote.teacher_lesson_for_feedback(&answer),
        Some((first_id, 0))
    );
}

#[test]
fn notebook_recovers_surviving_caches_once_without_guessing_receipt_or_exposure() {
    let mut state = app();
    let first = lesson(&state, "cached-lesson");
    let latest = lesson(&state, "cached-current");
    state.repo.set("teacher_lesson_response", &first).unwrap();
    state.repo.set("teacher_response", &latest).unwrap();
    state
        .repo
        .set(
            "teacher_context_prepared",
            &json!({"request_id":"unrelated","source_fact_ids":[Uuid::new_v4()]}),
        )
        .unwrap();
    let original_cache = state
        .repo
        .get::<TeacherResponse>("teacher_lesson_response")
        .unwrap();
    assert_eq!(state.recover_cached_teacher_responses().unwrap(), 2);
    assert_eq!(state.recover_cached_teacher_responses().unwrap(), 0);
    assert_eq!(state.teacher_notebook().len(), 2);
    for entry in state.teacher_notebook() {
        assert!(entry.received_at_ms.is_none() && entry.recovered);
        assert!(entry.archived_at_ms > 0);
        assert!(
            entry.context_source_fact_ids.is_none(),
            "unmatched context is not reconstructed"
        );
        let document = state.library.teacher_document(entry.document_id).unwrap();
        assert_eq!(
            document.receipt,
            TeacherDocumentReceipt::RecoveredLocalCache
        );
    }
    assert_eq!(state.library.presentations().count(), 0);
    assert_eq!(state.library.attempts().count(), 0);
    assert_eq!(
        state
            .repo
            .get::<TeacherResponse>("teacher_lesson_response")
            .unwrap(),
        original_cache
    );
    assert_eq!(
        state.selected_teacher_document().unwrap(),
        None,
        "recovery does not change the local view"
    );
    let copied = state.library.clone();
    let mut remote = app();
    remote
        .keep_teacher_response(&first.request, &first)
        .unwrap();
    remote
        .repo
        .append(&copied.events.into_values().collect::<Vec<_>>())
        .unwrap();
    remote.refresh().unwrap();
    let id = teacher_document_id(&first);
    assert_eq!(
        remote.library.teacher_document_receipts(id).len(),
        2,
        "independent valid receipts coexist"
    );
    assert_eq!(
        remote.teacher_notebook().len(),
        2,
        "one logical document per exact response"
    );
}

#[test]
fn notebook_bad_legacy_cache_preserves_raw_but_does_not_block_valid_current_work() {
    let mut state = app();
    let valid = lesson(&state, "surviving");
    state.repo.set("teacher_lesson_response", &valid).unwrap();
    let mut invalid = valid.clone();
    invalid.provenance.response_id.clear();
    state.repo.set("teacher_response", &invalid).unwrap();
    assert!(state.recover_cached_teacher_responses().is_err());
    assert_eq!(
        state.teacher_notebook().len(),
        1,
        "valid legacy responses recover independently"
    );
    let current = lesson(&state, "new-after-corrupt-cache");
    state
        .keep_teacher_response(&current.request, &current)
        .unwrap();
    assert_eq!(state.selected_teacher_document().unwrap(), Some(current));
    assert_eq!(state.teacher_notebook().len(), 2);
    assert!(state.notice.contains("original caches were kept"));
    assert_eq!(
        state
            .repo
            .get::<TeacherResponse>("teacher_response")
            .unwrap(),
        Some(invalid)
    );
    assert_eq!(
        state
            .repo
            .get::<TeacherResponse>("teacher_lesson_response")
            .unwrap(),
        Some(valid)
    );
}

#[test]
fn notebook_guided_flow_keeps_exact_payload_and_bounded_local_resume_without_exposure() {
    let mut state = app();
    let mut response = lesson(&state, "guided");
    if let TeacherRequest::TeachLesson(request) = &mut response.request {
        request.lesson_flow = Some(LessonFlowCapability::GuidedV1);
    }
    response.content.lesson_flow = Some(TeacherLessonFlow {
        steps: vec![
            TeacherLessonStep {
                id: "message".into(),
                goal: "Understand a short message.".into(),
                orientation: "Notice what the speaker is reading.".into(),
                detail: None,
                examples: vec![LessonExampleRef {
                    example_index: 0,
                    presentation: ExamplePresentation::Model,
                }],
                practice_index: Some(0),
            },
            TeacherLessonStep {
                id: "meaning".into(),
                goal: "Express the meaning.".into(),
                orientation: "Try the English message.".into(),
                detail: Some("A paraphrase is welcome.".into()),
                examples: vec![],
                practice_index: Some(1),
            },
        ],
    });
    state
        .keep_teacher_response(&response.request, &response)
        .unwrap();
    let id = teacher_document_id(&response);
    state.set_teacher_lesson_cursor(id, 1).unwrap();
    let before = state.library.clone();
    assert_eq!(state.teacher_lesson_cursor(id).unwrap(), 1);
    state.set_teacher_lesson_cursor(id, 2).unwrap();
    assert!(state.set_teacher_lesson_cursor(id, 3).is_err());
    assert_eq!(
        state.library, before,
        "finished-for-now navigation is not a learning fact"
    );
    let text = repository::export_library(before).unwrap();
    let restored = repository::import_library(&text).unwrap();
    assert_eq!(restored.teacher_document(id).unwrap().response, response);
}
