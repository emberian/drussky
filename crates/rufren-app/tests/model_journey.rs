//! Native journeys through the real app model, learning policy, and SQLite repository.
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
use repository::Repository;
use rufren_content::ExerciseKind;
use rufren_core::*;
use std::{collections::BTreeSet, path::PathBuf, rc::Rc};
use uuid::Uuid;
fn memory_app() -> AppState {
    AppState::from_repository(Rc::new(Repository::open_at(":memory:").unwrap())).unwrap()
}
fn exercise_id(state: &AppState, kind: ExerciseKind) -> String {
    state
        .content
        .exercises
        .iter()
        .find(|ex| ex.kind == kind)
        .unwrap()
        .id
        .clone()
}
fn recorded(state: &AppState, session: Uuid) -> (&Event, &Attempt) {
    let mut matches = state
        .library
        .attempts()
        .filter(|(_, a)| a.session == session);
    let result = matches.next().expect("completed session has an attempt");
    assert!(
        matches.next().is_none(),
        "one completion cannot count twice"
    );
    result
}
fn answer(state: &mut AppState, response: &str) {
    state.phase(Phase::Retrieve).unwrap();
    state.update_response(response.into()).unwrap();
    state.phase(Phase::Reflect).unwrap();
}
struct Notebook(PathBuf);
impl Notebook {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("rufren-model-journey-{}", Uuid::new_v4()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn open(&self) -> AppState {
        AppState::from_repository(Rc::new(
            Repository::open_at(self.0.join("library.sqlite")).unwrap(),
        ))
        .unwrap()
    }
}
impl Drop for Notebook {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
} // Only this test's unique database directory.

#[test]
fn model_journey_imports_are_replica_deterministic_order_independent_and_idempotent() {
    let mut left = memory_app();
    let mut right = memory_app();
    assert_ne!(left.device, right.device);
    let lessons: Vec<_> = left.content.lessons.iter().map(|l| l.id.clone()).collect();
    for id in &lessons {
        left.import_lesson(id).unwrap();
    }
    for id in lessons.iter().rev() {
        right.import_lesson(id).unwrap();
    }
    assert!(!left.library.events.is_empty());
    assert_eq!(left.library, right.library);
    let imported = left.library.clone();
    for id in lessons.iter().rev() {
        left.import_lesson(id).unwrap();
    }
    left.repo
        .append(&right.library.events.values().cloned().collect::<Vec<_>>())
        .unwrap();
    left.refresh().unwrap();
    assert_eq!(
        left.library, imported,
        "reimport and sync must not duplicate authored phrases"
    );
    assert!(left.library.heads().values().all(|h| h.len() == 1));
    assert!(left
        .library
        .events
        .values()
        .all(|e| matches!(&e.fact,Fact::PassageRevised(r) if !r.passage.coach_reviewed)));
}

#[test]
fn model_journey_reading_writing_hints_survive_database_reopen_and_remain_assisted() {
    let notebook = Notebook::new();
    let mut state = notebook.open();
    let id = exercise_id(&state, ExerciseKind::ReadingComprehension);
    state.start_exercise(&id).unwrap();
    let reading_session = state.session.as_ref().unwrap().id;
    let device = state.device;
    assert!(state.record(SelfRating::Recalled).is_err());
    assert!(state.hint().is_err());
    assert!(state.update_response("too early".into()).is_err());
    assert!(state.phase(Phase::Reflect).is_err());
    assert!(state.phase(Phase::Complete).is_err());
    assert!(state.library.attempts().next().is_none());
    state.phase(Phase::Retrieve).unwrap();
    assert!(
        state.phase(Phase::Reflect).is_err(),
        "empty retrieval cannot finish"
    );
    state
        .update_response("A spaceship is going toward a star.".into())
        .unwrap();
    state.hint().unwrap();
    state.hint().unwrap();
    drop(state);
    let mut state = notebook.open();
    let resumed = state.session.as_ref().unwrap();
    assert_eq!(state.device, device);
    assert_eq!(resumed.id, reading_session);
    assert_eq!(resumed.phase, Phase::Retrieve);
    assert_eq!(resumed.draft, "A spaceship is going toward a star.");
    assert_eq!(resumed.cues, vec![Cue::Translation]);
    assert_eq!(state.exercise().unwrap().id, id);
    state.phase(Phase::Reflect).unwrap();
    assert!(state.update_response("after comparison".into()).is_err());
    state.record(SelfRating::Recalled).unwrap();
    let (e, a) = recorded(&state, reading_session);
    assert_eq!(a.self_rating, SelfRating::Recalled);
    assert_eq!(
        SpacedReview.evidence(&state.library, e.id, a),
        ReviewEvidence::Assisted
    );
    assert!(state.record(SelfRating::Recalled).is_err());
    let writing = exercise_id(&state, ExerciseKind::WrittenRetrieval);
    state.start_exercise(&writing).unwrap();
    let writing_session = state.session.as_ref().unwrap().id;
    state.phase(Phase::Retrieve).unwrap();
    state.update_response("Это корабль.".into()).unwrap();
    state.hint().unwrap();
    drop(state);
    let mut state = notebook.open();
    assert_eq!(state.session.as_ref().unwrap().draft, "Это корабль.");
    assert_eq!(
        state.session.as_ref().unwrap().cues,
        vec![Cue::OriginalText]
    );
    state.phase(Phase::Reflect).unwrap();
    state.record(SelfRating::Recalled).unwrap();
    let (e, a) = recorded(&state, writing_session);
    assert_eq!(a.skill, Skill::Writing);
    assert_eq!(
        SpacedReview.evidence(&state.library, e.id, a),
        ReviewEvidence::Assisted
    );
    assert_eq!(state.library.attempts().count(), 2);
    drop(state);
    let state = notebook.open();
    assert_eq!(state.session.as_ref().unwrap().phase, Phase::Complete);
    assert_eq!(state.library.attempts().count(), 2);
}

#[test]
fn model_journey_answers_keep_exact_pack_exercise_targets_and_task_without_fabricated_judgments() {
    let mut state = memory_app();
    for (kind, task) in [
        (ExerciseKind::ReadingComprehension, TaskKind::MeaningRecall),
        (ExerciseKind::Contrast, TaskKind::GrammarGeneralization),
        (ExerciseKind::WrittenRetrieval, TaskKind::FormRecall),
    ] {
        let id = exercise_id(&state, kind);
        let ex = state.content.exercise(&id).unwrap().clone();
        state.start_exercise(&id).unwrap();
        let session = state.session.as_ref().unwrap().id;
        let response = ex
            .options
            .first()
            .map_or("Мой ответ", |o| o.id.as_str())
            .to_owned();
        answer(&mut state, &response);
        state.record(SelfRating::Recalled).unwrap();
        let (e, a) = recorded(&state, session);
        assert_eq!(a.response, response);
        assert_eq!(a.policy, "authored-curriculum-v1");
        let contexts = state.library.contexts(e.id);
        assert_eq!(contexts.len(), 1);
        let c = contexts[0].1;
        assert_eq!(c.content_pack.as_deref(), Some(state.content.id.as_str()));
        assert_eq!(
            c.content_revision.as_deref(),
            Some(state.content.revision.as_str())
        );
        assert_eq!(c.exercise_id.as_deref(), Some(id.as_str()));
        assert_eq!(
            c.target_features,
            ex.target_ids.into_iter().collect::<BTreeSet<_>>()
        );
        assert_eq!(c.task, task);
        assert!(!c.delayed_probe && !c.novel_context);
        assert!(
            c.practice_duration_ms.is_none(),
            "elapsed time includes pauses and cannot establish practice effort"
        );
        for dimension in [
            JudgmentDimension::Recall,
            JudgmentDimension::Grammar,
            JudgmentDimension::Comprehension,
        ] {
            assert!(
                state.library.judgments(e.id, dimension).is_empty(),
                "authored answer matching cannot invent coach evidence"
            );
        }
    }
}

#[test]
fn model_journey_free_practice_replaces_authored_plan_and_stale_saved_plan_is_ignored() {
    let mut state = memory_app();
    let id = exercise_id(&state, ExerciseKind::ReadingComprehension);
    state.start_exercise(&id).unwrap();
    let stale = state.plan.clone().unwrap();
    let revision = state.session.as_ref().unwrap().revision;
    state.start(revision, Skill::Writing).unwrap();
    let free = state.session.as_ref().unwrap().id;
    assert_ne!(stale.session, free);
    assert_eq!(state.plan.as_ref().unwrap().session, free);
    assert_eq!(state.plan.as_ref().unwrap().policy, "free-practice-v1");
    assert!(state.exercise().is_none());
    let repo = state.repo.clone();
    drop(state);
    let mut state = AppState::from_repository(repo.clone()).unwrap();
    assert!(state.exercise().is_none());
    answer(&mut state, "Свободный ответ");
    state.record(SelfRating::Recalled).unwrap();
    let (e, a) = recorded(&state, free);
    assert_eq!(a.policy, "free-practice-v1");
    let c = state.library.contexts(e.id)[0].1;
    assert!(c.exercise_id.is_none() && c.content_pack.is_none() && c.content_revision.is_none());
    assert!(c.target_features.is_empty());
    state.start(revision, Skill::Reading).unwrap();
    let next = state.session.as_ref().unwrap().id;
    repo.set("session_plan", &stale).unwrap();
    drop(state);
    let mut state = AppState::from_repository(repo).unwrap();
    assert_eq!(state.session.as_ref().unwrap().id, next);
    assert!(state.plan.is_none() && state.exercise().is_none());
    answer(&mut state, "Independent meaning");
    state.record(SelfRating::Recalled).unwrap();
    assert_eq!(recorded(&state, next).1.policy, "free-practice-v1");
}

#[test]
fn model_journey_coach_disagreement_survives_until_explicit_resolution_with_history_intact() {
    let mut state = memory_app();
    let id = exercise_id(&state, ExerciseKind::ReadingComprehension);
    state.start_exercise(&id).unwrap();
    let session = state.session.as_ref().unwrap().id;
    answer(&mut state, "My interpretation");
    state.record(SelfRating::Recalled).unwrap();
    let attempt = recorded(&state, session).0.id;
    let before = state.library.clone();
    assert!(state
        .coach_judgment(
            attempt,
            JudgmentDimension::Recall,
            Verdict::Correct,
            "unattributed".into(),
            90,
            false
        )
        .is_err());
    assert_eq!(state.library, before);
    state.preferences.coach_name = "Coach A".into();
    state
        .coach_judgment(
            attempt,
            JudgmentDimension::Recall,
            Verdict::Correct,
            "Meaning retained".into(),
            90,
            false,
        )
        .unwrap();
    state.preferences.coach_name = "Coach B".into();
    state
        .coach_judgment(
            attempt,
            JudgmentDimension::Recall,
            Verdict::Incorrect,
            "One role reversed".into(),
            85,
            false,
        )
        .unwrap();
    let heads: BTreeSet<_> = state
        .library
        .judgments(attempt, JudgmentDimension::Recall)
        .iter()
        .map(|(e, _)| e.id)
        .collect();
    assert_eq!(heads.len(), 2);
    assert_eq!(
        SpacedReview.evidence(
            &state.library,
            attempt,
            state.library.attempt(attempt).unwrap()
        ),
        ReviewEvidence::ContestedJudgment
    );
    state
        .coach_judgment(
            attempt,
            JudgmentDimension::Recall,
            Verdict::Correct,
            "Listened again together".into(),
            95,
            true,
        )
        .unwrap();
    let active = state.library.judgments(attempt, JudgmentDimension::Recall);
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].1.supersedes, heads);
    assert_eq!(
        state
            .library
            .events
            .values()
            .filter(|e| matches!(e.fact, Fact::CoachJudgmentRecorded(_)))
            .count(),
        3
    );
    assert_eq!(
        state.library.attempt(attempt).unwrap().self_rating,
        SelfRating::Recalled
    );
    assert_eq!(
        SpacedReview.evidence(
            &state.library,
            attempt,
            state.library.attempt(attempt).unwrap()
        ),
        ReviewEvidence::CoachConfirmedUnassisted
    );
}

#[test]
fn model_journey_audio_input_differs_from_replay_and_first_model_play_assists_production() {
    for kind in [
        ExerciseKind::ListeningComprehension,
        ExerciseKind::Dictation,
    ] {
        let mut state = memory_app();
        let id = exercise_id(&state, kind);
        state.start_exercise(&id).unwrap();
        state.phase(Phase::Retrieve).unwrap();
        state.heard_reference().unwrap();
        assert!(
            state.session.as_ref().unwrap().cues.is_empty(),
            "first audio is the task input"
        );
        let repo = state.repo.clone();
        drop(state);
        let mut state = AppState::from_repository(repo).unwrap();
        state.heard_reference().unwrap();
        state.heard_reference().unwrap();
        assert_eq!(state.session.as_ref().unwrap().cues, vec![Cue::Replay]);
        assert_eq!(state.plan.as_ref().unwrap().reference_plays, 3);
    }
    for kind in [
        ExerciseKind::ReadingComprehension,
        ExerciseKind::WrittenRetrieval,
        ExerciseKind::SpokenRetrieval,
    ] {
        let mut state = memory_app();
        let id = exercise_id(&state, kind);
        state.start_exercise(&id).unwrap();
        let session = state.session.as_ref().unwrap().id;
        state.phase(Phase::Retrieve).unwrap();
        state.heard_reference().unwrap();
        assert_eq!(
            state.session.as_ref().unwrap().cues,
            vec![Cue::Replay],
            "optional model audio assists even on its first play"
        );
        if kind == ExerciseKind::SpokenRetrieval {
            // Fixture for the studio's saved-recording callback. Bytes live in the asset store;
            // completion must link this recording to the exact immutable attempt.
            state.plan.as_mut().unwrap().response_recording = Some(AudioAttachment {
                asset_id: "journey-spoken-original".into(),
                asset: AudioAsset {
                    sha256: "c".repeat(64),
                    media_type: "audio/webm".into(),
                    sample_rate_hz: 48_000,
                    channels: 1,
                    frames: 96_000,
                },
                target: AnnotationTarget::Revision(state.session.as_ref().unwrap().revision),
                role: AudioRole::Attempt,
                note: "Saved learner response".into(),
            });
        }
        state.update_response("Я отвечаю".into()).unwrap();
        state.phase(Phase::Reflect).unwrap();
        state.record(SelfRating::Recalled).unwrap();
        let (e, a) = recorded(&state, session);
        assert_eq!(
            SpacedReview.evidence(&state.library, e.id, a),
            ReviewEvidence::Assisted
        );
        if kind == ExerciseKind::SpokenRetrieval {
            assert_eq!(state.library.contexts(e.id)[0].1.task, TaskKind::FormRecall);
            let recordings = state.library.recordings(AnnotationTarget::Attempt(e.id));
            assert_eq!(recordings.len(), 1);
            assert_eq!(recordings[0].1.asset_id, "journey-spoken-original");
        }
    }
}

#[test]
fn model_journey_retry_after_durable_attempt_does_not_duplicate_attempt_or_context() {
    let mut state = memory_app();
    let id = exercise_id(&state, ExerciseKind::WrittenRetrieval);
    state.start_exercise(&id).unwrap();
    let session = state.session.as_ref().unwrap().id;
    answer(&mut state, "Это мой ответ.");
    let reflection = state.session.clone();
    state.record(SelfRating::Recalled).unwrap();
    let before = state.library.clone();
    let repo = state.repo.clone();
    // Simulate interruption between fact commit and updating the local session phase.
    repo.set("session", &reflection).unwrap();
    drop(state);
    let mut resumed = AppState::from_repository(repo).unwrap();
    assert_eq!(resumed.session.as_ref().unwrap().phase, Phase::Reflect);
    resumed.record(SelfRating::Again).unwrap();
    assert_eq!(resumed.library, before);
    assert_eq!(resumed.session.as_ref().unwrap().phase, Phase::Complete);
    let (e, a) = recorded(&resumed, session);
    assert_eq!(a.self_rating, SelfRating::Recalled);
    assert_eq!(resumed.library.contexts(e.id).len(), 1);
}

#[test]
fn model_journey_resumed_question_stimuli_and_provenance_stay_pinned_across_content_upgrade() {
    let notebook = Notebook::new();
    let mut state = notebook.open();
    let id = state
        .content
        .exercises
        .iter()
        .find(|ex| ex.kind == ExerciseKind::Contrast && !ex.stimulus_phrase_ids.is_empty())
        .expect("a contrast with visible material for snapshot testing")
        .id
        .clone();
    state.start_exercise(&id).unwrap();
    let original_exercise = state.exercise().unwrap().clone();
    let original_reference = state
        .stimulus(&original_exercise.reference_phrase_id)
        .unwrap()
        .clone();
    let original_phrases: Vec<_> = original_exercise
        .stimulus_phrase_ids
        .iter()
        .map(|id| state.stimulus(id).unwrap().clone())
        .collect();
    let original_pack = state.content.id.clone();
    let original_pack_revision = state.content.revision.clone();
    let session = state.session.as_ref().unwrap().id;
    let revision = state.session.as_ref().unwrap().revision;
    state.phase(Phase::Retrieve).unwrap();
    state
        .update_response("Response begun under the original question".into())
        .unwrap();
    drop(state);
    let mut state = notebook.open();
    // Model the bundled pack changing between app launches while an old session is saved.
    let mut upgraded = (*state.content).clone();
    upgraded.revision = "future-editorial-revision".into();
    let changed_exercise = upgraded
        .exercises
        .iter_mut()
        .find(|ex| ex.id == id)
        .unwrap();
    changed_exercise.instruction =
        "A different question must not replace the saved question".into();
    changed_exercise.target_ids = vec!["ru.target.destination".into()];
    for phrase in &mut upgraded.phrases {
        if original_exercise.stimulus_phrase_ids.contains(&phrase.id)
            || original_exercise.reference_phrase_id == phrase.id
        {
            phrase.russian = "Совсем другой текст.".into();
            phrase.translation = "Entirely different text.".into();
            phrase.chunks = vec![phrase.russian.clone()];
        }
    }
    state.content = Rc::new(upgraded);
    assert_eq!(state.session.as_ref().unwrap().id, session);
    assert_eq!(state.exercise().unwrap(), &original_exercise);
    assert_eq!(
        state
            .stimulus(&original_exercise.reference_phrase_id)
            .unwrap(),
        &original_reference
    );
    for phrase in &original_phrases {
        assert_eq!(state.stimulus(&phrase.id).unwrap(), phrase);
    }
    assert_eq!(
        state.session.as_ref().unwrap().draft,
        "Response begun under the original question"
    );
    state.phase(Phase::Reflect).unwrap();
    state.record(SelfRating::Recalled).unwrap();
    let (event, attempt) = recorded(&state, session);
    assert_eq!(attempt.revision, revision);
    let context = state.library.contexts(event.id)[0].1;
    assert_eq!(
        context.content_pack.as_deref(),
        Some(original_pack.as_str())
    );
    assert_eq!(
        context.content_revision.as_deref(),
        Some(original_pack_revision.as_str())
    );
    assert_eq!(context.exercise_id.as_deref(), Some(id.as_str()));
    assert_eq!(
        context.target_features,
        original_exercise.target_ids.into_iter().collect()
    );
    assert_eq!(context.task, TaskKind::GrammarGeneralization);
    assert_eq!(
        state.library.revision(revision).unwrap().passage.russian,
        original_reference.russian
    );
}

#[test]
fn model_journey_only_explicit_auditory_contrasts_record_sound_discrimination_evidence() {
    let mut state = memory_app();
    for (exercise, expected) in [
        (
            "ru.exercise.naming.hear-soft",
            TaskKind::SoundDiscrimination,
        ),
        (
            "ru.exercise.naming.hear-stress",
            TaskKind::SoundDiscrimination,
        ),
        ("ru.exercise.hello.listen", TaskKind::MeaningRecall),
        ("ru.exercise.naming.mark-stress", TaskKind::FormRecall),
        ("ru.exercise.naming.say-contrast", TaskKind::FormRecall),
    ] {
        state.start_exercise(exercise).unwrap();
        let context = state.attempt_context(Uuid::new_v4()).unwrap().unwrap();
        assert_eq!(
            context.task, expected,
            "task metadata must describe the elicited response in {exercise}"
        );
        assert_eq!(context.exercise_id.as_deref(), Some(exercise));
    }
    // Tagging a semantic listening exercise with an additional sound feature does not turn its
    // meaning answer into evidence that this sound contrast was perceptually discriminated.
    state.start_exercise("ru.exercise.hello.listen").unwrap();
    state
        .plan
        .as_mut()
        .unwrap()
        .snapshot
        .as_mut()
        .unwrap()
        .exercise
        .target_ids
        .push("ru.target.hard_soft".into());
    assert_eq!(
        state.attempt_context(Uuid::new_v4()).unwrap().unwrap().task,
        TaskKind::MeaningRecall
    );
}

fn due_review_app() -> (AppState, Uuid) {
    let mut state = memory_app();
    let base = starter(state.device, 0);
    let old = Event::new(
        state.device,
        0,
        Fact::AttemptRecorded(Attempt {
            session: Uuid::new_v4(),
            revision: base.id,
            skill: Skill::Reading,
            response: "Previously recalled meaning".into(),
            cues: vec![],
            self_rating: SelfRating::Recalled,
            policy: SPACED_POLICY_ID.into(),
        }),
    )
    .unwrap();
    state.repo.append(&[base.clone(), old]).unwrap();
    state.refresh().unwrap();
    state.begin().unwrap();
    (state, base.id)
}

#[test]
fn model_journey_due_review_starts_at_retrieval_without_answer_reencounter() {
    let (mut state, revision) = due_review_app();
    let session = state.session.as_ref().unwrap();
    assert_eq!(
        session.phase,
        Phase::Retrieve,
        "a delayed retrieval cannot first show its answer in Encounter"
    );
    assert_eq!(session.revision, revision);
    assert!(session.cues.is_empty());
    let id = session.id;
    assert_eq!(
        state.library.decisions(id).len(),
        1,
        "the selected policy and alternatives are preserved before the response"
    );
    state
        .update_response("A spaceship is going to a star".into())
        .unwrap();
    state.phase(Phase::Reflect).unwrap();
    state.record(SelfRating::Recalled).unwrap();
    let (event, attempt) = recorded(&state, id);
    assert!(attempt.cues.is_empty());
    assert_eq!(attempt.policy, SPACED_POLICY_ID);
    assert!(state.library.contexts(event.id)[0].1.delayed_probe);
    let review = SpacedReview.project(
        &state.library,
        ReviewKey {
            revision,
            skill: Skill::Reading,
        },
        repository::now_ms(),
    );
    assert_eq!(review.successful_spaced_reviews, 1);
}

#[test]
fn model_journey_reference_navigation_preserves_support_and_cannot_advance_independent_spacing() {
    let (mut state, revision) = due_review_app();
    let session = state.session.as_ref().unwrap().id;
    state
        .update_response("Partly remembered draft".into())
        .unwrap();
    state.navigate(model::Page::Library).unwrap();
    state.navigate(model::Page::Lessons).unwrap();
    assert_eq!(
        state.session.as_ref().unwrap().cues,
        vec![Cue::ReferenceMaterial]
    );
    let repo = state.repo.clone();
    drop(state);
    let mut state = AppState::from_repository(repo).unwrap();
    state.navigate(model::Page::Practice).unwrap();
    assert_eq!(
        state.session.as_ref().unwrap().draft,
        "Partly remembered draft"
    );
    assert_eq!(
        state.session.as_ref().unwrap().cues,
        vec![Cue::ReferenceMaterial]
    );
    state.phase(Phase::Reflect).unwrap();
    state.record(SelfRating::Recalled).unwrap();
    let (event, attempt) = recorded(&state, session);
    assert_eq!(
        SpacedReview.evidence(&state.library, event.id, attempt),
        ReviewEvidence::Assisted
    );
    let review = SpacedReview.project(
        &state.library,
        ReviewKey {
            revision,
            skill: Skill::Reading,
        },
        repository::now_ms(),
    );
    assert_eq!(review.successful_spaced_reviews, 0);
    assert_eq!(review.assisted_attempts, 1);
}

#[test]
fn model_journey_saved_choice_question_survives_next_task_and_sync_without_local_content() {
    let mut state = memory_app();
    let id = exercise_id(&state, ExerciseKind::ReadingComprehension);
    state.start_exercise(&id).unwrap();
    let original = state.exercise().unwrap().clone();
    let original_stimuli: Vec<_> = original
        .stimulus_phrase_ids
        .iter()
        .map(|id| state.stimulus(id).unwrap().russian.clone())
        .collect();
    let chosen_id = original
        .options
        .last()
        .expect("authored choice question")
        .id
        .clone();
    let session = state.session.as_ref().unwrap().id;
    answer(&mut state, &chosen_id);
    state.record(SelfRating::Recalled).unwrap();
    let (event, attempt) = recorded(&state, session);
    let attempt_id = event.id;
    assert_eq!(
        attempt.response, chosen_id,
        "persist literal choice ID rather than rewriting learner evidence"
    );
    let captured = state.library.contexts(attempt_id)[0].1.clone();
    let prompt = captured.prompt.as_ref().unwrap();
    assert!(prompt.contains(&original.instruction));
    for option in &original.options {
        assert!(prompt.contains(&format!("{} = {}", option.id, option.text)));
    }
    for stimulus in original_stimuli {
        assert!(prompt.contains(&stimulus));
    }
    let rufren_content::AnswerSpec::Choice { correct_option } = &original.answer else {
        panic!("reading choice fixture")
    };
    let correct = original
        .options
        .iter()
        .find(|option| &option.id == correct_option)
        .unwrap();
    assert!(captured
        .reference_answer
        .as_ref()
        .unwrap()
        .contains(&format!("{} = {}", correct.id, correct.text)));
    let next = exercise_id(&state, ExerciseKind::WrittenRetrieval);
    state.start_exercise(&next).unwrap();
    assert_ne!(
        state.plan.as_ref().unwrap().exercise_id.as_deref(),
        Some(id.as_str())
    );
    let mut remote = memory_app();
    let mut future = (*remote.content).clone();
    future.revision = "replacement-pack-without-old-question".into();
    future.exercises.clear();
    remote.content = Rc::new(future);
    remote
        .repo
        .append(&state.library.events.values().cloned().collect::<Vec<_>>())
        .unwrap();
    remote.refresh().unwrap();
    assert!(remote.session.is_none() && remote.plan.is_none());
    assert!(remote.content.exercise(&id).is_none());
    assert_eq!(remote.library.contexts(attempt_id)[0].1, &captured);
    assert_eq!(
        remote.library.attempt(attempt_id).unwrap().response,
        chosen_id
    );
}
