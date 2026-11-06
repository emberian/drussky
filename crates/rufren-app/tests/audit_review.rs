//! Delayed review must attempt retrieval before the app reveals the answer again.
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
#[path = "../src/model.rs"]
mod model;
#[path = "../src/native_connection.rs"]
mod native_connection;
#[path = "../src/native_http.rs"]
mod native_http;
#[path = "../src/repository.rs"]
mod repository;
use model::AppState;
use repository::{Repository, now_ms};
use rufren_core::*;
use std::rc::Rc;
use uuid::Uuid;

#[test]
fn audit_review_due_retrieval_precedes_answer_reexposure_and_hint_remains_assisted() {
    let mut state =
        AppState::from_repository(Rc::new(Repository::open_at(":memory:").unwrap())).unwrap();
    state.editor.title = "A delayed question".into();
    state.editor.russian = "Это книга.".into();
    state.editor.translation = "This is a book.".into();
    state.save_passage(false).unwrap();
    let revision = state.editor.parent.unwrap();
    state
        .repo
        .append(&[Event::new(
            state.device,
            now_ms() - 2 * DAY_MS,
            Fact::AttemptRecorded(Attempt {
                session: Uuid::new_v4(),
                revision,
                skill: Skill::Reading,
                response: "This is a book.".into(),
                cues: vec![],
                self_rating: SelfRating::Recalled,
                policy: SPACED_POLICY_ID.into(),
            }),
        )
        .unwrap()])
        .unwrap();
    state.preferences.review_skill = Skill::Reading;
    state.begin().unwrap();
    assert_eq!(state.session.as_ref().unwrap().revision, revision);
    assert_eq!(
        state.session.as_ref().unwrap().phase,
        Phase::Retrieve,
        "a due review cannot show both the Russian and its translation at Encounter before measuring retrieval"
    );
    assert_eq!(state.plan.as_ref().unwrap().policy, SPACED_POLICY_ID);
    state.hint().unwrap();
    state.update_response("This is a book.".into()).unwrap();
    let session = state.session.as_ref().unwrap().id;
    state.phase(Phase::Reflect).unwrap();
    state.record(SelfRating::Recalled).unwrap();
    let (event, attempt) = state
        .library
        .attempts()
        .find(|(_, a)| a.session == session)
        .unwrap();
    assert_eq!(
        SpacedReview.evidence(&state.library, event.id, attempt),
        ReviewEvidence::Assisted
    );
    assert_eq!(
        SpacedReview
            .project(
                &state.library,
                ReviewKey {
                    revision,
                    skill: Skill::Reading
                },
                now_ms()
            )
            .successful_spaced_reviews,
        0
    );
}
