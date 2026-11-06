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
#[path = "../src/reader.rs"]
mod reader;
#[path = "../src/model.rs"]
mod model;
#[path = "../src/review_timing.rs"]
mod review_timing;
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
#[path = "../src/teacher_dialogue.rs"]
mod teacher_dialogue;
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
use std::rc::Rc;
use teacher_dialogue::{DialogueDeliveryStatus, DialogueDispatch, DialogueDraft};
use uuid::Uuid;
fn app() -> AppState {
    AppState::from_repository(Rc::new(Repository::open_at(":memory:").unwrap())).unwrap()
}
fn config() -> DialogueConfig {
    DialogueConfig {
        intent: DialogueIntent::Clarification,
        topic: "A synthetic Russian question".into(),
        teacher_role: None,
        learner_role: None,
    }
}
fn reply(request: &TeacherDialogueRequest, id: &str) -> TeacherDialogueResponse {
    TeacherDialogueResponse {
        request: request.clone(),
        content: TeacherDialogueContent {
            explanation_english: Some("Synthetic explanation for the exact saved question.".into()),
            utterance: None,
            next_question: None,
            correction: None,
            scene_state: DialogueSceneState::Open,
        },
        provenance: TutorProvenance {
            provider: "synthetic".into(),
            model: "no-model".into(),
            review_status: ReviewStatus::AiGeneratedTeaching,
            scope: "Synthetic text only; no inference.".into(),
            prompt_version: "dialogue-test-v1".into(),
            response_id: id.into(),
        },
        usage: TutorUsage {
            accounting: UsageAccounting::SubscriptionQuota,
            input_tokens: 0,
            output_tokens: 0,
            estimated_micro_usd: 0,
            reserved_micro_usd: 0,
        },
    }
}
fn prepared(state: &mut AppState, text: &str) -> (DialogueDraft, DialogueDispatch) {
    let mut draft = state
        .new_dialogue(config(), DialogueAction::Ask, None)
        .unwrap();
    state.save_dialogue_draft(&mut draft, text.into()).unwrap();
    let id = state.prepare_dialogue_turn(&mut draft).unwrap();
    assert!(
        state.library.dialogue_user(id).is_some(),
        "the immutable user turn exists before dispatch"
    );
    let dispatch = state.begin_dialogue_dispatch(id).unwrap();
    (draft, dispatch)
}
#[test]
fn dialogue_message_precedes_dispatch_and_retries_keep_exact_frozen_request() {
    let mut state = app();
    assert!(state.begin_dialogue_dispatch(Uuid::new_v4()).is_err());
    let (mut draft, dispatch) = prepared(&mut state, "Why does this ending change?");
    let initial = state.library.clone();
    let bytes = serde_json::to_vec(&dispatch.request).unwrap();
    assert_eq!(state.prepare_dialogue_turn(&mut draft).unwrap(), draft.id);
    assert_eq!(state.library, initial);
    assert!(
        state.begin_dialogue_dispatch(draft.id).is_err(),
        "no concurrent duplicate send"
    );
    state
        .fail_dialogue_dispatch(&dispatch, "Synthetic offline failure".into())
        .unwrap();
    assert_eq!(
        state.dialogue_outbox().unwrap()[0].status,
        DialogueDeliveryStatus::OutcomeUnknown
    );
    let retry = state.begin_dialogue_dispatch(draft.id).unwrap();
    assert_ne!(retry.dispatch_id, dispatch.dispatch_id);
    assert_eq!(serde_json::to_vec(&retry.request).unwrap(), bytes);
    assert_eq!(state.library, initial);
    assert!(
        state
            .save_dialogue_draft(&mut draft, "Changed sent text".into())
            .is_err()
    );
    let response = reply(&retry.request, "retry-reply");
    let id = state.keep_dialogue_reply(&retry, &response).unwrap();
    let next = state.active_dialogue_draft().unwrap().unwrap();
    assert_eq!(next.parent_reply_id, Some(id));
    assert!(next.text.is_empty());
    assert!(state.dialogue_outbox().unwrap().is_empty());
    assert_eq!(state.library.attempts().count(), 0);
    assert_eq!(state.library.presentations().count(), 0);
}
#[test]
fn dialogue_late_and_mismatched_replies_never_overwrite_another_composer() {
    let mut state = app();
    let (_, dispatch) = prepared(&mut state, "Question A");
    let mut other = state
        .new_dialogue(config(), DialogueAction::Ask, None)
        .unwrap();
    state
        .save_dialogue_draft(&mut other, "Unsent question B".into())
        .unwrap();
    let original = state.library.clone();
    let mut wrong = reply(&dispatch.request, "wrong");
    wrong.request.text = "A different question".into();
    assert!(state.keep_dialogue_reply(&dispatch, &wrong).is_err());
    assert_eq!(state.library, original);
    assert_eq!(state.active_dialogue_draft().unwrap(), Some(other.clone()));
    let response = reply(&dispatch.request, "late");
    let id = state.keep_dialogue_reply(&dispatch, &response).unwrap();
    assert_eq!(state.library.dialogue_reply(id), Some(&response));
    assert_eq!(state.active_dialogue_draft().unwrap(), Some(other.clone()));
    state.keep_dialogue_reply(&dispatch, &response).unwrap();
    assert_eq!(state.active_dialogue_draft().unwrap(), Some(other));
    assert_eq!(
        state
            .library
            .dialogue_replies(dispatch.request.request_id)
            .len(),
        1
    );
}
#[test]
fn dialogue_two_devices_keep_sibling_branches_and_exact_reply_identity_collisions() {
    let mut a = app();
    let (_, root) = prepared(&mut a, "Root question");
    let root_reply = reply(&root.request, "root-reply");
    let parent = a.keep_dialogue_reply(&root, &root_reply).unwrap();
    let mut b = app();
    b.repo
        .append(&a.library.events.values().cloned().collect::<Vec<_>>())
        .unwrap();
    b.refresh().unwrap();
    let mut da = a.active_dialogue_draft().unwrap().unwrap();
    a.save_dialogue_draft(&mut da, "Branch A".into()).unwrap();
    let aid = a.prepare_dialogue_turn(&mut da).unwrap();
    let ad = a.begin_dialogue_dispatch(aid).unwrap();
    let mut db = b.open_dialogue_reply(parent).unwrap();
    b.save_dialogue_draft(&mut db, "Branch B".into()).unwrap();
    let bid = b.prepare_dialogue_turn(&mut db).unwrap();
    let bd = b.begin_dialogue_dispatch(bid).unwrap();
    assert_ne!(aid, bid);
    assert_eq!(ad.request.parent_reply_id, Some(parent));
    assert_eq!(ad.request.ancestors, bd.request.ancestors);
    let ar = reply(&ad.request, "reply-A");
    let br = reply(&bd.request, "reply-B");
    a.keep_dialogue_reply(&ad, &ar).unwrap();
    b.keep_dialogue_reply(&bd, &br).unwrap();
    a.repo
        .append(&b.library.events.values().cloned().collect::<Vec<_>>())
        .unwrap();
    a.refresh().unwrap();
    b.repo
        .append(&a.library.events.values().cloned().collect::<Vec<_>>())
        .unwrap();
    b.refresh().unwrap();
    assert_eq!(a.library, b.library);
    assert_eq!(a.library.dialogue_children(parent).len(), 2);
    assert_eq!(a.library.dialogue_threads()[0].leaf_ids.len(), 2);
    let before = a.library.clone();
    let mut changed = ar.clone();
    changed.content.explanation_english =
        Some("Changed under the same provider response identity.".into());
    assert!(a.keep_dialogue_reply(&ad, &changed).is_err());
    assert_eq!(a.library, before);
}
#[test]
fn dialogue_history_is_exact_bounded_chosen_ancestor_suffix_with_explicit_omission() {
    let mut state = app();
    let (mut draft, mut dispatch) = prepared(&mut state, "First question");
    for index in 0..7 {
        let response = reply(&dispatch.request, &format!("chain-{index}"));
        state.keep_dialogue_reply(&dispatch, &response).unwrap();
        draft = state.active_dialogue_draft().unwrap().unwrap();
        state
            .save_dialogue_draft(&mut draft, format!("Follow-up {index}"))
            .unwrap();
        let id = state.prepare_dialogue_turn(&mut draft).unwrap();
        dispatch = state.begin_dialogue_dispatch(id).unwrap();
        assert!(dispatch.request.ancestors.len() <= 8);
        assert_eq!(dispatch.request.ancestors.len() % 2, 0);
        assert_eq!(
            dispatch.request.ancestors.last().unwrap().message_id,
            dispatch.request.parent_reply_id.unwrap()
        );
        assert!(
            serde_json::to_vec(&dispatch.request.ancestors)
                .unwrap()
                .len()
                <= DIALOGUE_MAX_HISTORY_BYTES
        );
        assert!(serde_json::to_vec(&dispatch.request).unwrap().len() <= DIALOGUE_MAX_REQUEST_BYTES);
        for ancestor in &dispatch.request.ancestors {
            let body = match ancestor.role {
                DialogueRole::User => teacher_dialogue_user_body(
                    state.library.dialogue_user(ancestor.message_id).unwrap(),
                ),
                DialogueRole::Assistant => teacher_dialogue_assistant_body(
                    &state
                        .library
                        .dialogue_reply(ancestor.message_id)
                        .unwrap()
                        .content,
                ),
            };
            assert_eq!(ancestor.text, body);
        }
        if index >= 4 {
            assert!(dispatch.request.omitted_count > 0);
            let first = state
                .library
                .dialogue_user(dispatch.request.ancestors[0].message_id)
                .unwrap();
            assert_eq!(dispatch.request.omitted_parent_id, first.parent_reply_id);
        }
    }
    assert_eq!(state.library.attempts().count(), 0);
}
#[test]
fn dialogue_pending_commit_recovers_real_sqlite_fact_failure_before_manual_send() {
    let folder = std::env::temp_dir().join(format!("rufren-dialogue-commit-{}", Uuid::new_v4()));
    std::fs::create_dir(&folder).unwrap();
    let database = folder.join("library.sqlite");
    let repo = Rc::new(Repository::open_at(&database).unwrap());
    let mut state = AppState::from_repository(repo).unwrap();
    let sql = rusqlite::Connection::open(&database).unwrap();
    sql.execute_batch("CREATE TRIGGER fail_dialogue_user BEFORE INSERT ON events WHEN json_extract(NEW.json,'$.fact.kind')='teacher_dialogue_user_recorded' BEGIN SELECT RAISE(ABORT,'synthetic interrupted user commit'); END;").unwrap();
    let mut draft = state
        .new_dialogue(config(), DialogueAction::Ask, None)
        .unwrap();
    state
        .save_dialogue_draft(&mut draft, "Durable before network".into())
        .unwrap();
    assert!(state.prepare_dialogue_turn(&mut draft).is_err());
    assert!(state.library.dialogue_user(draft.id).is_none());
    assert!(state.begin_dialogue_dispatch(draft.id).is_err());
    assert_eq!(
        state.dialogue_outbox().unwrap()[0].status,
        DialogueDeliveryStatus::PendingCommit
    );
    let saved: serde_json::Value = state.repo.get("teacher_dialogue_state").unwrap().unwrap();
    let event: Event =
        serde_json::from_value(saved["outbox"][draft.id.to_string()]["event"].clone()).unwrap();
    sql.execute_batch("DROP TRIGGER fail_dialogue_user;")
        .unwrap();
    assert_eq!(state.reconcile_dialogue().unwrap().committed, 1);
    assert_eq!(state.reconcile_dialogue().unwrap().committed, 0);
    assert_eq!(state.library.events[&event.id], event);
    assert_eq!(
        state.dialogue_outbox().unwrap()[0].status,
        DialogueDeliveryStatus::Ready
    );
    let dispatch = state.begin_dialogue_dispatch(draft.id).unwrap();
    let Fact::TeacherDialogueUserRecorded(original) = event.fact else {
        unreachable!()
    };
    assert_eq!(dispatch.request, original);
    drop(sql);
    drop(state);
    std::fs::remove_dir_all(folder).unwrap();
}
#[test]
fn dialogue_reload_interrupted_send_stays_manual_and_stale_drafts_are_preserved() {
    let mut state = app();
    let (_, dispatch) = prepared(&mut state, "Saved request");
    let mut local: serde_json::Value = state.repo.get("teacher_dialogue_state").unwrap().unwrap();
    local["outbox"][dispatch.request.request_id.to_string()]["runtime_instance"] =
        serde_json::json!(Uuid::new_v4());
    state.repo.set("teacher_dialogue_state", &local).unwrap();
    let before = state.library.clone();
    assert_eq!(state.reconcile_dialogue().unwrap().interrupted, 1);
    assert_eq!(state.library, before);
    assert_eq!(
        state.dialogue_outbox().unwrap()[0].status,
        DialogueDeliveryStatus::OutcomeUnknown
    );
    let mut draft = state
        .new_dialogue(config(), DialogueAction::Ask, None)
        .unwrap();
    let mut stale = draft.clone();
    state
        .save_dialogue_draft(&mut draft, "Newer local text".into())
        .unwrap();
    assert!(
        state
            .save_dialogue_draft(&mut stale, "Stale overwrite".into())
            .is_err()
    );
    assert_eq!(state.active_dialogue_draft().unwrap(), Some(draft.clone()));
    let revision = draft.revision;
    state.clear_dialogue_audio(&mut draft).unwrap();
    assert_eq!(draft.text, "Newer local text");
    assert_eq!(draft.revision, revision + 1);
    assert!(
        state
            .adopt_dialogue_audio(
                &mut draft,
                "Invented words".into(),
                DialogueConfirmedAudioRef {
                    recording: Uuid::new_v4(),
                    transcription: Uuid::new_v4(),
                    confirmation: Uuid::new_v4()
                }
            )
            .is_err()
    );
}
#[test]
fn dialogue_local_limits_refuse_new_work_without_erasing_unfinished_content() {
    let mut state = app();
    let mut ids = Vec::new();
    for index in 0..16 {
        let mut draft = state
            .new_dialogue(config(), DialogueAction::Ask, None)
            .unwrap();
        state
            .save_dialogue_draft(&mut draft, format!("Unfinished {index}"))
            .unwrap();
        ids.push(draft.id);
    }
    assert!(
        state
            .new_dialogue(config(), DialogueAction::Ask, None)
            .is_err()
    );
    let drafts = state.dialogue_drafts().unwrap();
    assert_eq!(drafts.len(), 16);
    assert!(
        ids.iter()
            .all(|id| drafts.iter().any(|d| d.id == *id && !d.text.is_empty()))
    );
    let audio = Event::new(
        state.device,
        repository::now_ms(),
        Fact::DialogueAudioAttached(DialogueAudioAttachment {
            thread_id: ids[0],
            owner: DialogueAudioOwner::UserInput { input_id: ids[0] },
            asset_id: "synthetic-unsent-original".into(),
            asset: AudioAsset {
                sha256: "a".repeat(64),
                media_type: "audio/wav".into(),
                sample_rate_hz: 16000,
                channels: 1,
                frames: 16000,
            },
            provenance_json: None,
        }),
    )
    .unwrap();
    state.repo.append(&[audio.clone()]).unwrap();
    let facts_before = state.repo.library().unwrap();
    state.discard_dialogue_draft(ids[0]).unwrap();
    assert_eq!(state.dialogue_drafts().unwrap().len(), 15);
    assert!(
        state
            .dialogue_drafts()
            .unwrap()
            .iter()
            .all(|draft| draft.id != ids[0])
    );
    assert_eq!(
        state.repo.library().unwrap(),
        facts_before,
        "discard never deletes original audio facts"
    );
    assert!(
        state
            .new_dialogue(config(), DialogueAction::Ask, None)
            .is_ok(),
        "explicit discard frees capacity without fake chat"
    );
    let mut other = app();
    for index in 0..8 {
        let mut draft = other
            .new_dialogue(config(), DialogueAction::Ask, None)
            .unwrap();
        other
            .save_dialogue_draft(&mut draft, format!("Pending {index}"))
            .unwrap();
        other.prepare_dialogue_turn(&mut draft).unwrap();
    }
    let mut ninth = other
        .new_dialogue(config(), DialogueAction::Ask, None)
        .unwrap();
    other
        .save_dialogue_draft(&mut ninth, "Ninth pending input".into())
        .unwrap();
    assert!(other.prepare_dialogue_turn(&mut ninth).is_err());
    assert_eq!(other.library.dialogue_threads().len(), 8);
    let saved = other.dialogue_outbox().unwrap()[0].request_id;
    assert!(
        other.discard_dialogue_draft(saved).is_err(),
        "queued/immutable messages cannot be discarded"
    );
    assert_eq!(
        other.active_dialogue_draft().unwrap().unwrap().text,
        "Ninth pending input"
    );
}

#[test]
fn dialogue_source_return_recovers_exact_focus_without_origin_device_cursor() {
    for guided in [false, true] {
        let mut producer = app();
        let mut response: TeacherResponse = serde_json::from_str(include_str!(
            "../../../scripts/platform/fixtures/teacher-notebook-response.json"
        ))
        .unwrap();
        if guided {
            let TeacherRequest::TeachLesson(request) = &mut response.request else {
                unreachable!()
            };
            request.lesson_flow = Some(LessonFlowCapability::GuidedV1);
            response.content.lesson_flow = Some(serde_json::from_value(serde_json::json!({"steps":[
                {"id":"start","goal":"Begin","orientation":"First step.","examples":[{"example_index":0,"presentation":"model"}]},
                {"id":"target","goal":"Write","orientation":"The exact source step.","examples":[],"practice_index":0}
            ]})).unwrap());
        }
        producer
            .keep_teacher_response(&response.request, &response)
            .unwrap();
        let document = teacher_document_id(&response);
        producer.set_teacher_lesson_cursor(document, 1).unwrap();
        let focus = if guided {
            TeacherDialogueFocus::Step {
                step_id: "target".into(),
            }
        } else {
            TeacherDialogueFocus::Practice { index: 0 }
        };
        let source = teacher_dialogue_anchor(&producer.library, document, focus).unwrap();
        let mut consumer = app();
        consumer
            .repo
            .append(
                &producer
                    .library
                    .events
                    .values()
                    .cloned()
                    .collect::<Vec<_>>(),
            )
            .unwrap();
        consumer.refresh().unwrap();
        assert_eq!(consumer.teacher_lesson_cursor(document).unwrap(), 0);
        let mut unfinished = consumer.teacher_practice_draft(&response, 0).unwrap();
        unfinished.response = "Keep this unfinished answer".into();
        consumer.save_teacher_practice_draft(&unfinished).unwrap();
        let before = consumer.library.events.clone();
        consumer.return_to_dialogue_source(&source).unwrap();
        assert_eq!(consumer.teacher_lesson_cursor(document).unwrap(), 1);
        assert_eq!(
            consumer.selected_teacher_document().unwrap(),
            Some(response.clone())
        );
        assert_eq!(
            consumer
                .teacher_practice_draft(&response, 0)
                .unwrap()
                .response,
            unfinished.response
        );
        assert_eq!(consumer.library.events, before);
        let practice = teacher_dialogue_anchor(
            &consumer.library,
            document,
            TeacherDialogueFocus::Practice { index: 0 },
        )
        .unwrap();
        consumer.set_teacher_lesson_cursor(document, 0).unwrap();
        consumer.return_to_dialogue_source(&practice).unwrap();
        assert_eq!(consumer.teacher_lesson_cursor(document).unwrap(), 1);
        let mut changed = source;
        changed.snapshot.push_str("Changed source");
        assert!(consumer.return_to_dialogue_source(&changed).is_err());
        assert_eq!(consumer.teacher_lesson_cursor(document).unwrap(), 1);
    }
}
