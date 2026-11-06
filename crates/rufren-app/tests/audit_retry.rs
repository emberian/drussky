//! Browser quota can stop a multi-event append between its immutable facts.
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
#[path = "../src/teacher_dialogue.rs"]
mod teacher_dialogue;
#[path = "../src/model.rs"]
mod model;
#[path = "../src/native_connection.rs"]
mod native_connection;
#[path = "../src/native_http.rs"]
mod native_http;
#[path = "../src/repository.rs"]
mod repository;
#[path = "../src/recognition.rs"]
mod recognition;
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
use repository::{Repository, now_ms};
use rufren_content::ExerciseKind;
use rufren_core::*;
use std::rc::Rc;

#[test]
fn audit_retry_partial_attempt_repairs_provenance_and_spoken_recording_idempotently() {
    for kind in [
        ExerciseKind::WrittenRetrieval,
        ExerciseKind::SpokenRetrieval,
    ] {
        let repo = Rc::new(Repository::open_at(":memory:").unwrap());
        let mut state = AppState::from_repository(repo.clone()).unwrap();
        let exercise = state
            .content
            .exercises
            .iter()
            .find(|e| e.kind == kind)
            .unwrap()
            .id
            .clone();
        state.start_exercise(&exercise).unwrap();
        state.phase(Phase::Retrieve).unwrap();
        if kind == ExerciseKind::SpokenRetrieval {
            state.plan.as_mut().unwrap().response_recording = Some(AudioAttachment {
                asset_id: "audit-partially-committed-original".into(),
                asset: AudioAsset {
                    sha256: "e".repeat(64),
                    media_type: "audio/wav".into(),
                    sample_rate_hz: 48_000,
                    channels: 1,
                    frames: 48_000,
                },
                target: AnnotationTarget::Revision(state.session.as_ref().unwrap().revision),
                role: AudioRole::Attempt,
                note: String::new(),
            });
            repo.set("session_plan", &state.plan).unwrap();
        }
        state.update_response("Мой ответ".into()).unwrap();
        state.phase(Phase::Reflect).unwrap();
        let reflection = state.session.clone().unwrap();
        // Simulate a browser quota failure after the first journal write of
        // record()'s [attempt, context, audio attachment] batch.
        let partial = Event::new(
            state.device,
            now_ms(),
            Fact::AttemptRecorded(Attempt {
                session: reflection.id,
                revision: reflection.revision,
                skill: reflection.skill,
                response: reflection.draft.clone(),
                cues: reflection.cues.clone(),
                self_rating: SelfRating::Recalled,
                policy: state.plan.as_ref().unwrap().policy.clone(),
            }),
        )
        .unwrap();
        repo.append(&[partial.clone()]).unwrap();
        drop(state);
        let mut state = AppState::from_repository(repo.clone()).unwrap();
        state.record(SelfRating::Again).unwrap();
        assert_eq!(state.library.attempts().count(), 1);
        assert_eq!(
            state.library.attempt(partial.id).unwrap().self_rating,
            SelfRating::Recalled
        );
        let contexts = state.library.contexts(partial.id);
        assert_eq!(
            contexts.len(),
            1,
            "retry must repair the missing authored exercise provenance"
        );
        assert_eq!(
            contexts[0].1.exercise_id.as_deref(),
            Some(exercise.as_str())
        );
        let recordings = state
            .library
            .recordings(AnnotationTarget::Attempt(partial.id));
        assert_eq!(
            recordings.len(),
            usize::from(kind == ExerciseKind::SpokenRetrieval),
            "a partial attempt must not orphan its exact original audio"
        );
        let completed = state.library.clone();
        repo.set("session", &Some(reflection)).unwrap();
        drop(state);
        let mut state = AppState::from_repository(repo).unwrap();
        state.record(SelfRating::Again).unwrap();
        assert_eq!(
            state.library, completed,
            "recovery facts must also be idempotent"
        );
    }
}
