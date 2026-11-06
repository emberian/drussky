//! Grounded context selection and bounded upload composition; no model/network calls.
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
use std::{collections::BTreeSet, rc::Rc};
use teacher_context::{MAX_TEACHER_CONTEXT_BYTES, PracticeRelevance, ResponseEvidence};
use uuid::Uuid;
fn app() -> AppState {
    let mut state =
        AppState::from_repository(Rc::new(Repository::open_at(":memory:").unwrap())).unwrap();
    let id = state.content.lessons[0].id.clone();
    state.choose_lesson(&id).unwrap();
    state.import_lesson(&id).unwrap();
    state
}
fn reference(state: &AppState) -> Uuid {
    let lesson = state.lesson().unwrap();
    state.phrase_revision(state.content.phrase(&lesson.phrase_ids[0]).unwrap())
}
fn context(state: &AppState, attempt: Uuid, prompt: &str) -> AttemptContext {
    let lesson = state.lesson().unwrap();
    AttemptContext {
        attempt,
        content_pack: Some(state.content.id.clone()),
        content_revision: Some(state.content.revision.clone()),
        exercise_id: Some(lesson.exercise_ids[0].clone()),
        prompt: Some(prompt.into()),
        reference_answer: Some("An authored example, not an exhaustive answer list".into()),
        task: TaskKind::FormRecall,
        target_features: lesson.target_ids.iter().cloned().collect(),
        novel_context: false,
        delayed_probe: false,
        practice_duration_ms: None,
        effort: None,
        grammar_response: None,
        supersedes: BTreeSet::new(),
    }
}
fn add(
    state: &mut AppState,
    revision: Uuid,
    skill: Skill,
    rating: SelfRating,
    cues: Vec<Cue>,
    time: i64,
    text: &str,
) -> (Uuid, Uuid) {
    let attempt = Event::new(
        state.device,
        time,
        Fact::AttemptRecorded(Attempt {
            session: Uuid::new_v4(),
            revision,
            skill,
            response: text.into(),
            cues,
            self_rating: rating,
            policy: "context-fixture".into(),
        }),
    )
    .unwrap();
    let c = context(
        state,
        attempt.id,
        "Translate the supplied meaning into Russian.",
    );
    let metadata = Event::new(state.device, time, Fact::AttemptContextRecorded(c)).unwrap();
    let ids = (attempt.id, metadata.id);
    state.repo.append(&[attempt, metadata]).unwrap();
    state.refresh().unwrap();
    ids
}
fn opinion(
    state: &mut AppState,
    attempt: Uuid,
    context: Uuid,
    verdict: Verdict,
    time: i64,
    note: &str,
) -> Uuid {
    let a = state.library.attempt(attempt).unwrap();
    let value = AiAssessment {
        request_id: Uuid::new_v4().to_string(),
        attempt,
        revision: a.revision,
        skill: a.skill,
        task: TaskKind::FormRecall,
        response: a.response.clone(),
        context: Some(context),
        confirmed_transcript: None,
        evidence: AiTextEvidence::WrittenText,
        dimension: JudgmentDimension::Grammar,
        verdict,
        suggested_response: None,
        corrections: vec![],
        explanation: note.into(),
        next_step: "Try a new message.".into(),
        provenance: AiProvenance {
            provider: "synthetic".into(),
            model: "model-test".into(),
            response_id: Uuid::new_v4().to_string(),
            prompt_version: "test-v1".into(),
        },
        supersedes: BTreeSet::new(),
    };
    let event = Event::new(state.device, time, Fact::AiAssessmentRecorded(value)).unwrap();
    let id = event.id;
    state.repo.append(&[event]).unwrap();
    state.refresh().unwrap();
    id
}
fn generated(state: &AppState) -> TeacherResponse {
    let mut request = state.lesson_teaching_request().unwrap();
    if let TeacherRequest::TeachLesson(request) = &mut request {
        request.lesson_flow = None;
    }
    TeacherResponse {
        request,
        provenance: TutorProvenance {
            provider: "synthetic".into(),
            model: "model-test".into(),
            review_status: ReviewStatus::AiGeneratedTeaching,
            scope: "language content".into(),
            prompt_version: "test-v1".into(),
            response_id: "generated-lesson-one".into(),
        },
        usage: TutorUsage {
            accounting: UsageAccounting::SubscriptionQuota,
            input_tokens: 0,
            output_tokens: 0,
            estimated_micro_usd: 0,
            reserved_micro_usd: 0,
        },
        content: TeacherContent {
            lesson_flow: None,
            explanation: "A short meaningful greeting.".into(),
            examples: vec![TutorExample {
                russian: "Привет.".into(),
                english: "Hi.".into(),
                note: "Informal greeting".into(),
            }],
            next_step: "Try the greeting.".into(),
            caveats: vec![],
            feedback: None,
            practice: vec![TeacherPractice {
                prompt: "Write a short informal greeting in Russian.".into(),
                expected_response: Some("Привет.".into()),
                teaching_note: "Accept appropriate informal greetings.".into(),
                reference_russian: Some("Привет.".into()),
                reference_english: Some("Hi.".into()),
                skill: Some(TeacherSkill::Writing),
            }],
        },
    }
}

#[test]
fn teacher_context_no_history_retains_declared_baseline_without_diagnosing_current_ability() {
    let state = app();
    let summary = state.lesson_practice_summary().unwrap();
    assert!(summary.observations.is_empty());
    let text = summary.provider_context();
    assert!(text.contains("began Russian from zero"));
    assert!(text.contains("short replayable chunks"));
    assert!(text.contains("prior ability is unknown"));
    assert!(!text.contains("complete beginner"));
    assert!(!text.contains("working memory"));
    assert!(text.len() <= MAX_TEACHER_CONTEXT_BYTES);
    let TeacherRequest::TeachLesson(request) = state.lesson_teaching_request().unwrap() else {
        unreachable!()
    };
    assert_eq!(request.learner_context.as_deref(), Some(text));
    assert!(summary.source_fact_ids().is_empty());
}

#[test]
fn teacher_context_reports_cues_self_rating_and_attributed_feedback_without_mastery_claims() {
    let mut state = app();
    let now = repository::now_ms();
    let revision = reference(&state);
    let (a, c) = add(
        &mut state,
        revision,
        Skill::Writing,
        SelfRating::Recalled,
        vec![Cue::Replay],
        now - 1000,
        "Привет.",
    );
    let feedback = opinion(
        &mut state,
        a,
        c,
        Verdict::PartlyCorrect,
        now - 500,
        "Use the appropriate register for this person.",
    );
    let summary = state.lesson_practice_summary_at(None, now).unwrap();
    assert_eq!(summary.observations.len(), 1);
    let o = &summary.observations[0];
    assert_eq!(o.attempt_id, a);
    assert_eq!(o.context_id, Some(c));
    assert_eq!(o.cues, vec![Cue::Replay]);
    assert!(o.assisted);
    assert_eq!(o.self_rating, SelfRating::Recalled);
    assert_eq!(summary.counts.assisted_rated, 1);
    assert_eq!(summary.counts.unassisted_recalled, 0);
    assert_eq!(o.feedback[0].fact_id, feedback);
    assert!(summary.source_fact_ids().contains(&feedback));
    assert!(
        summary
            .provider_context()
            .contains("AI opinion whole-response")
    );
    assert!(summary.provider_context().contains("not feature mastery"));
}

#[test]
fn teacher_context_excludes_unrelated_private_passages_even_with_shared_targets_or_russian_text() {
    let mut state = app();
    let now = repository::now_ms();
    let russian = state
        .library
        .revision(reference(&state))
        .unwrap()
        .passage
        .russian
        .clone();
    let mut private = starter(state.device, now);
    if let Fact::PassageRevised(r) = &mut private.fact {
        r.passage.russian = russian;
        r.passage.title = "PRIVATE-TITLE-SENTINEL".into();
        r.passage.translation = "PRIVATE-PASSAGE-SENTINEL".into();
    }
    state.repo.append(&[private.clone()]).unwrap();
    state.refresh().unwrap();
    let (a, c) = add(
        &mut state,
        private.id,
        Skill::Writing,
        SelfRating::Recalled,
        vec![],
        now - 1000,
        "PRIVATE-ANSWER-SENTINEL",
    );
    let mut corrected = state.library.events[&c].clone();
    if let Fact::AttemptContextRecorded(ctx) = &mut corrected.fact {
        ctx.content_pack = None;
        ctx.exercise_id = None;
        ctx.supersedes = [c].into();
    }
    corrected.id = Uuid::new_v4();
    state.repo.append(&[corrected]).unwrap();
    state.refresh().unwrap();
    let summary = state.lesson_practice_summary_at(None, now).unwrap();
    assert!(summary.observations.iter().all(|o| o.attempt_id != a));
    assert!(!summary.provider_context().contains("PRIVATE"));
    assert!(summary.source_fact_ids().is_empty());
}

#[test]
fn teacher_context_preserves_old_revision_evidence_across_lesson_phrase_edits_and_scope_changes() {
    let mut state = app();
    let lesson = state.lesson().unwrap().id.clone();
    let now = repository::now_ms();
    let old = reference(&state);
    let (a, _) = add(
        &mut state,
        old,
        Skill::Reading,
        SelfRating::Again,
        vec![],
        now - 1000,
        "An earlier meaning response",
    );
    let mut r = state.library.revision(old).unwrap().clone();
    r.parents = [old].into();
    r.passage.russian.push_str(" Ещё пример.");
    let newer = Event::new(state.device, now, Fact::PassageRevised(r)).unwrap();
    state.repo.append(&[newer.clone()]).unwrap();
    state.refresh().unwrap();
    let summary = state.lesson_practice_summary_at(None, now).unwrap();
    assert_eq!(summary.observations[0].attempt_id, a);
    assert_eq!(summary.observations[0].revision_id, old);
    assert_ne!(summary.observations[0].revision_id, newer.id);
    assert_eq!(
        summary.observations[0].relevance,
        PracticeRelevance::LessonPassageFamily
    );
    let other = state
        .content
        .lessons
        .iter()
        .find(|l| l.id != lesson)
        .unwrap()
        .id
        .clone();
    state.choose_lesson(&other).unwrap();
    assert!(
        state
            .lesson_practice_summary_at(None, now)
            .unwrap()
            .observations
            .is_empty()
    );
    state.choose_lesson(&lesson).unwrap();
    assert_eq!(
        state
            .lesson_practice_summary_at(None, now)
            .unwrap()
            .observations[0]
            .attempt_id,
        a
    );
}

#[test]
fn teacher_context_budget_caps_recent_examples_and_keeps_unicode_excerpts_explicit() {
    let mut state = app();
    let now = repository::now_ms();
    let revision = reference(&state);
    for i in 0..8 {
        let (a, c) = add(
            &mut state,
            revision,
            Skill::Writing,
            SelfRating::NotReported,
            vec![Cue::ReferenceMaterial],
            now - 1000 + i,
            &"Ответ я читаю книгу. ".repeat(30),
        );
        opinion(
            &mut state,
            a,
            c,
            Verdict::Correct,
            now - 500 + i,
            &"Наблюдение об этом ответе. ".repeat(20),
        );
        opinion(
            &mut state,
            a,
            c,
            Verdict::Incorrect,
            now - 400 + i,
            &"Другое мнение об этом ответе. ".repeat(20),
        );
    }
    let summary = state.lesson_practice_summary_at(None, now).unwrap();
    assert!(!summary.observations.is_empty());
    assert!(summary.observations.len() <= 4);
    assert!(summary.provider_context().len() <= 2048);
    assert_eq!(summary.counts.not_reported, summary.observations.len());
    assert_eq!(summary.counts.unassisted_recalled, 0);
    assert!(summary.omissions.recent_limit + summary.omissions.text_budget > 0);
    for o in &summary.observations {
        assert!(o.response.as_ref().unwrap().truncated);
        assert!(o.response.as_ref().unwrap().text.len() <= 160);
        assert!(o.feedback.len() <= 2);
        assert!(o.feedback_conflicting);
        assert!(
            o.feedback
                .iter()
                .all(|f| f.explanation.as_ref().unwrap().text.len() <= 100)
        );
    }
}

#[test]
fn teacher_context_future_ambiguous_and_duplicate_session_records_are_not_silent_evidence_winners()
{
    let mut state = app();
    let now = repository::now_ms();
    let revision = reference(&state);
    let (_, c) = add(
        &mut state,
        revision,
        Skill::Writing,
        SelfRating::Again,
        vec![],
        now - 1000,
        "AMBIGUOUS-SENTINEL",
    );
    let mut competing = state.library.events[&c].clone();
    competing.id = Uuid::new_v4();
    if let Fact::AttemptContextRecorded(ctx) = &mut competing.fact {
        ctx.prompt = Some("Different saved task".into());
    }
    state.repo.append(&[competing]).unwrap();
    let (a, _) = add(
        &mut state,
        revision,
        Skill::Writing,
        SelfRating::Recalled,
        vec![],
        now - 900,
        "DUPLICATE-SENTINEL",
    );
    let mut duplicate = state.library.events[&a].clone();
    duplicate.id = Uuid::new_v4();
    if let Fact::AttemptRecorded(attempt) = &mut duplicate.fact {
        attempt.response = "Different completed response under same session".into();
    }
    state.repo.append(&[duplicate]).unwrap();
    add(
        &mut state,
        revision,
        Skill::Writing,
        SelfRating::Recalled,
        vec![],
        now + 100000,
        "FUTURE-SENTINEL",
    );
    let summary = state.lesson_practice_summary_at(None, now).unwrap();
    assert!(summary.observations.is_empty());
    assert_eq!(summary.omissions.ambiguous_context, 1);
    assert_eq!(summary.omissions.duplicate_session, 2);
    assert_eq!(summary.omissions.future, 1);
    assert!(!summary.provider_context().contains("SENTINEL"));
}

#[test]
fn teacher_context_qualified_ai_practice_follows_actual_origin_lesson_and_exposes_source_facts() {
    let mut state = app();
    let lesson = state.lesson().unwrap().id.clone();
    let answer = generated(&state);
    let mut draft = state.teacher_practice_draft(&answer, 0).unwrap();
    draft.response = "Привет.".into();
    let a = state.submit_teacher_practice(&answer, &mut draft).unwrap();
    let ctx = state.library.contexts(a)[0].0.id;
    let origin = teacher_practice::saved_teacher_practice_origin(&state.library, a, ctx).unwrap();
    assert_eq!(origin.lesson_id, lesson);
    let summary = state.lesson_practice_summary().unwrap();
    assert_eq!(summary.observations.len(), 1);
    assert_eq!(
        summary.observations[0].relevance,
        PracticeRelevance::QualifiedAiLessonExercise
    );
    assert_eq!(summary.counts.not_reported, 1);
    assert!(summary.source_fact_ids().contains(&ctx));
    let target = state.lesson().unwrap().target_ids[0].clone();
    assert!(
        state
            .lesson_practice_summary_for_target(&target)
            .unwrap()
            .observations
            .is_empty(),
        "generated lesson association is not an invented assessed-target link"
    );
    let other = state
        .content
        .lessons
        .iter()
        .find(|l| l.id != lesson)
        .unwrap()
        .id
        .clone();
    state.choose_lesson(&other).unwrap();
    assert!(
        state
            .lesson_practice_summary()
            .unwrap()
            .observations
            .is_empty()
    );
    state.choose_lesson(&lesson).unwrap();
    assert_eq!(
        state.lesson_practice_summary().unwrap().observations[0].attempt_id,
        a
    );
}

#[test]
fn teacher_context_legacy_generated_retries_preserve_bytes_and_foreign_origin_ids_do_not_qualify() {
    let mut source = app();
    let answer = generated(&source);
    let mut draft = source.teacher_practice_draft(&answer, 0).unwrap();
    draft.response = "LEGACY-ANSWER-SENTINEL".into();
    let a = source.submit_teacher_practice(&answer, &mut draft).unwrap();
    let context_id = source.library.contexts(a)[0].0.id;
    let passage = source
        .library
        .revision(source.library.attempt(a).unwrap().revision)
        .unwrap()
        .passage
        .id;
    let mut old_events: Vec<_> = source.library.events.values().cloned().collect();
    for event in &mut old_events {
        if event.id == context_id {
            let Fact::AttemptContextRecorded(c) = &mut event.fact else {
                unreachable!()
            };
            c.exercise_id = Some(format!("teacher:{passage}"));
        }
    }
    let mut legacy = app();
    legacy.repo.append(&old_events).unwrap();
    legacy.refresh().unwrap();
    let before = legacy.library.clone();
    let old_json = serde_json::to_string(&legacy.library.events[&context_id]).unwrap();
    assert_eq!(
        legacy.submit_teacher_practice(&answer, &mut draft).unwrap(),
        a
    );
    assert_eq!(legacy.library, before);
    assert_eq!(
        serde_json::to_string(&legacy.library.events[&context_id]).unwrap(),
        old_json
    );
    assert!(
        legacy
            .lesson_practice_summary()
            .unwrap()
            .observations
            .is_empty()
    );
    let origin =
        teacher_practice::saved_teacher_practice_origin(&source.library, a, context_id).unwrap();
    for id in [
        format!("teacher:v2:{}:{}:0", origin.lesson_id, Uuid::new_v4()),
        format!("teacher:v2:{}:{}:00", origin.lesson_id, origin.source),
    ] {
        let mut remote = app();
        let mut events: Vec<_> = source.library.events.values().cloned().collect();
        for event in &mut events {
            if event.id == context_id {
                let Fact::AttemptContextRecorded(c) = &mut event.fact else {
                    unreachable!()
                };
                c.exercise_id = Some(id.clone());
            }
        }
        remote.repo.append(&events).unwrap();
        remote.refresh().unwrap();
        assert!(
            teacher_practice::saved_teacher_practice_origin(&remote.library, a, context_id)
                .is_none()
        );
        assert!(
            remote
                .lesson_practice_summary()
                .unwrap()
                .observations
                .is_empty()
        );
    }
}

#[test]
fn teacher_context_speaking_without_confirmed_words_does_not_send_labels_or_failure_as_language_evidence()
 {
    let mut state = app();
    let now = repository::now_ms();
    let revision = reference(&state);
    let (a, c) = add(
        &mut state,
        revision,
        Skill::Speaking,
        SelfRating::NotReported,
        vec![],
        now - 1000,
        "AUDIO-FILE-LABEL-SENTINEL",
    );
    opinion(
        &mut state,
        a,
        c,
        Verdict::Incorrect,
        now - 500,
        "RECOGNIZER-FAILURE-SENTINEL is not a language judgment.",
    );
    let summary = state.lesson_practice_summary_at(None, now).unwrap();
    assert_eq!(summary.observations.len(), 1);
    let o = &summary.observations[0];
    assert!(o.response.is_none());
    assert!(matches!(
        o.response_evidence,
        ResponseEvidence::Unavailable { .. }
    ));
    assert!(o.feedback.is_empty());
    assert_eq!(summary.counts.not_reported, 1);
    assert_eq!(summary.counts.unassisted_again, 0);
    assert_eq!(summary.counts.assisted_rated, 0);
    assert!(!summary.provider_context().contains("SENTINEL"));
    assert!(
        summary
            .provider_context()
            .contains("No uniquely confirmed spoken words")
    );
}

#[test]
fn teacher_context_prepared_snapshot_matches_request_without_recomputing_later_history() {
    let mut state = app();
    let now = repository::now_ms();
    let revision = reference(&state);
    let (a, _) = add(
        &mut state,
        revision,
        Skill::Writing,
        SelfRating::Again,
        vec![],
        now - 1000,
        "Earlier response",
    );
    let TeacherRequest::TeachLesson(first) = state.lesson_teaching_request().unwrap() else {
        unreachable!()
    };
    let saved = state
        .prepared_lesson_context(&first.request_id)
        .unwrap()
        .unwrap();
    assert_eq!(
        Some(saved.summary.provider_context()),
        first.learner_context.as_deref()
    );
    assert!(saved.source_fact_ids.contains(&a));
    let (b, _) = add(
        &mut state,
        revision,
        Skill::Writing,
        SelfRating::NotReported,
        vec![Cue::ReferenceMaterial],
        now - 100,
        "New response after preparation",
    );
    let later = state.lesson_practice_summary_at(None, now).unwrap();
    assert!(later.source_fact_ids().contains(&b));
    let retained = state
        .prepared_lesson_context(&first.request_id)
        .unwrap()
        .unwrap();
    assert_eq!(retained, saved);
    assert!(!retained.source_fact_ids.contains(&b));
    assert!(
        state
            .prepared_lesson_context("unrelated-request")
            .unwrap()
            .is_none()
    );
    let TeacherRequest::TeachLesson(second) = state.lesson_teaching_request().unwrap() else {
        unreachable!()
    };
    assert_ne!(first.request_id, second.request_id);
    assert!(
        state
            .prepared_lesson_context(&first.request_id)
            .unwrap()
            .is_none(),
        "only the latest bounded snapshot is retained"
    );
    let actual: teacher_context::PreparedTeacherContext =
        state.repo.get("teacher_context_prepared").unwrap().unwrap();
    assert_eq!(actual.request_id, second.request_id);
    assert_eq!(
        Some(actual.summary.provider_context()),
        second.learner_context.as_deref()
    );
}
