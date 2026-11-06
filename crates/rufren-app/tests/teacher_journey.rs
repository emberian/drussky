//! App-to-provider DTO-to-durable-fact journeys. Provider responses below are typed fixtures;
//! provider transport and live-model behavior are tested in the gateway lane.
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
use rufren_content::ExerciseKind;
use rufren_core::*;
use rufren_tutor::*;
use std::rc::Rc;
use uuid::Uuid;

fn app() -> AppState {
    AppState::from_repository(Rc::new(Repository::open_at(":memory:").unwrap())).unwrap()
}
fn complete(state: &mut AppState, response: &str) -> Uuid {
    let session = state.session.as_ref().unwrap().id;
    state.phase(Phase::Retrieve).unwrap();
    state.update_response(response.into()).unwrap();
    state.phase(Phase::Reflect).unwrap();
    state.record(SelfRating::Recalled).unwrap();
    state
        .library
        .attempts()
        .find(|(_, a)| a.session == session)
        .unwrap()
        .0
        .id
}
fn choice() -> (AppState, Uuid) {
    let mut state = app();
    let id = state
        .content
        .exercises
        .iter()
        .find(|e| e.kind == ExerciseKind::ReadingComprehension)
        .unwrap()
        .id
        .clone();
    state.start_exercise(&id).unwrap();
    let rufren_content::AnswerSpec::Choice { correct_option } =
        state.exercise().unwrap().answer.clone()
    else {
        panic!("choice fixture")
    };
    let attempt = complete(&mut state, &correct_option);
    (state, attempt)
}
fn writing() -> (AppState, Uuid) {
    let mut state = app();
    let mut revision = starter(state.device, 0);
    if let Fact::PassageRevised(r) = &mut revision.fact {
        r.passage.russian = "Я читаю книгу.".into();
        r.passage.translation = "I am reading a book.".into();
    }
    state.repo.append(&[revision.clone()]).unwrap();
    state.refresh().unwrap();
    state.start(revision.id, Skill::Writing).unwrap();
    let attempt = complete(&mut state, "Я читаю книга.");
    (state, attempt)
}
fn input(request: &TeacherRequest) -> &AnswerFeedbackRequest {
    let TeacherRequest::AnswerFeedback(value) = request else {
        panic!("feedback request")
    };
    value
}
fn feedback(request: &TeacherRequest) -> TeacherResponse {
    let content=TeacherContent { lesson_flow: None,explanation:"The object of читаю uses the accusative here. The correction concerns the written form, not pronunciation.".into(),examples:vec![],next_step:"Try the same meaning with another object.".into(),caveats:vec!["This is an attributed AI judgment of language content.".into()],feedback:Some(AiAnswerFeedback {verdict:AiVerdict::PartlyCorrect,suggested_response:Some("Я читаю книгу.".into()),corrections:if input(request).response.contains("книга") {vec![TextCorrection {original:"книга".into(),replacement:"книгу".into(),reason:"Accusative direct object".into()}]}else{vec![]}}),practice:vec![]};
    let response = TeacherResponse {
        request: request.clone(),
        content,
        provenance: TutorProvenance {
            provider: "anthropic".into(),
            model: "provider-model-fixture".into(),
            review_status: ReviewStatus::AiGeneratedTeaching,
            scope: "text language content".into(),
            prompt_version: "teacher-v1-fixture".into(),
            response_id: "provider-response-fixture-1".into(),
        },
        usage: TutorUsage {
            accounting: UsageAccounting::SubscriptionQuota,
            input_tokens: 120,
            output_tokens: 40,
            estimated_micro_usd: 0,
            reserved_micro_usd: 0,
        },
    };
    // Exercise the actual transport representation, including the gateway's echoed binding.
    serde_json::from_str(&serde_json::to_string(&response).unwrap()).unwrap()
}

#[test]
fn teacher_journey_synced_choice_uses_saved_question_and_literal_id_without_original_pack() {
    let (mut source, attempt) = choice();
    let original = source.answer_feedback_request(attempt).unwrap();
    let original = input(&original).clone();
    let next = source
        .content
        .exercises
        .iter()
        .find(|e| e.kind == ExerciseKind::WrittenRetrieval)
        .unwrap()
        .id
        .clone();
    source.start_exercise(&next).unwrap();
    let mut remote = app();
    let mut future = (*remote.content).clone();
    future.revision = "different-pack".into();
    future.exercises.clear();
    remote.content = Rc::new(future);
    let exported = repository::export_library(source.library.clone()).unwrap();
    let imported = repository::import_library(&exported).unwrap();
    remote
        .repo
        .append(&imported.events.into_values().collect::<Vec<_>>())
        .unwrap();
    remote.refresh().unwrap();
    assert!(remote.session.is_none() && remote.plan.is_none());
    let request = remote.answer_feedback_request(attempt).unwrap();
    let saved = input(&request);
    assert_eq!(saved.revision_id, original.revision_id);
    assert_eq!(saved.context_id, original.context_id);
    assert_eq!(saved.exercise_id, original.exercise_id);
    assert_eq!(saved.prompt, original.prompt);
    assert_eq!(saved.reference, original.reference);
    assert_eq!(saved.response, original.response);
    assert_eq!(saved.task, original.task);
    assert_eq!(
        saved.response,
        remote.library.attempt(attempt).unwrap().response
    );
    assert!(
        saved.prompt.contains(&format!("{} = ", saved.response)),
        "choice IDs are interpretable without the old pack"
    );
    let response = feedback(&request);
    remote.keep_teacher_response(&request, &response).unwrap();
    assert_eq!(
        remote
            .library
            .ai_assessments(attempt, JudgmentDimension::Comprehension)
            .len(),
        1
    );
}

#[test]
fn teacher_journey_provider_feedback_becomes_exact_sourced_ai_fact_with_retry_safety() {
    let (mut state, attempt) = writing();
    let a = state.library.attempt(attempt).unwrap();
    let key = ReviewKey {
        revision: a.revision,
        skill: a.skill,
    };
    let before = SpacedReview.project(&state.library, key, repository::now_ms());
    let request = state.answer_feedback_request(attempt).unwrap();
    let response = feedback(&request);
    response.content.validate_for(&request).unwrap();
    state.keep_teacher_response(&request, &response).unwrap();
    let active = state
        .library
        .ai_assessments(attempt, JudgmentDimension::Grammar);
    assert_eq!(active.len(), 1);
    let saved_id = active[0].0.id;
    let assessment = active[0].1;
    assert_eq!(assessment.attempt, attempt);
    assert_eq!(assessment.revision.to_string(), input(&request).revision_id);
    assert_eq!(assessment.response, input(&request).response);
    assert_eq!(assessment.request_id, input(&request).request_id);
    assert_eq!(
        assessment.context.map(|id| id.to_string()),
        input(&request).context_id
    );
    assert_eq!(assessment.task, TaskKind::FormRecall);
    assert_eq!(assessment.evidence, AiTextEvidence::WrittenText);
    assert_eq!(assessment.verdict, Verdict::PartlyCorrect);
    assert_eq!(assessment.corrections[0].original, "книга");
    assert_eq!(assessment.corrections[0].replacement, "книгу");
    assert_eq!(
        assessment.suggested_response.as_deref(),
        Some("Я читаю книгу.")
    );
    assert_eq!(assessment.provenance.provider, response.provenance.provider);
    assert_eq!(assessment.provenance.model, response.provenance.model);
    assert_eq!(
        assessment.provenance.response_id,
        response.provenance.response_id
    );
    assert_eq!(
        assessment.provenance.prompt_version,
        response.provenance.prompt_version
    );
    assert!(
        state
            .library
            .judgments(attempt, JudgmentDimension::Grammar)
            .is_empty()
    );
    assert!(
        state
            .library
            .judgments(attempt, JudgmentDimension::Recall)
            .is_empty()
    );
    assert_eq!(
        SpacedReview.project(&state.library, key, repository::now_ms()),
        before
    );
    assert_eq!(before.coach_confirmed_unassisted, 0);
    let facts = state.library.clone();
    let repo = state.repo.clone();
    drop(state);
    let mut reopened = AppState::from_repository(repo).unwrap();
    reopened.keep_teacher_response(&request, &response).unwrap();
    assert_eq!(reopened.library, facts);
    assert_eq!(
        reopened
            .library
            .ai_assessments(attempt, JudgmentDimension::Grammar)[0]
            .0
            .id,
        saved_id
    );
    assert!(
        reopened
            .repo
            .get::<TeacherResponse>("teacher_response")
            .unwrap()
            .is_some()
    );
}

#[test]
fn teacher_journey_mismatched_echo_and_stale_attempt_binding_leave_history_unchanged() {
    let (mut state, attempt) = writing();
    let request = state.answer_feedback_request(attempt).unwrap();
    let mut response = feedback(&request);
    let before = state.library.clone();
    let TeacherRequest::AnswerFeedback(echo) = &mut response.request else {
        unreachable!()
    };
    echo.request_id = Uuid::new_v4().to_string();
    assert!(state.keep_teacher_response(&request, &response).is_err());
    assert_eq!(state.library, before);
    for change in ["response", "revision", "context", "task"] {
        let mut stale = request.clone();
        let TeacherRequest::AnswerFeedback(r) = &mut stale else {
            unreachable!()
        };
        match change {
            "response" => r.response = "A stale, different answer".into(),
            "revision" => r.revision_id = Uuid::new_v4().to_string(),
            "context" => r.context_id = Some(Uuid::new_v4().to_string()),
            "task" => r.task = "composition".into(),
            _ => unreachable!(),
        }
        let response = feedback(&stale);
        assert!(
            state.keep_teacher_response(&stale, &response).is_err(),
            "stale {change} must not be rebound to the saved answer"
        );
        assert_eq!(state.library, before);
    }
    assert!(
        state
            .repo
            .get::<TeacherResponse>("teacher_response")
            .unwrap()
            .is_none()
    );
}

#[test]
fn teacher_journey_expected_question_skill_choices_and_support_must_match_immutable_source() {
    let (mut state, attempt) = choice();
    let request = state.answer_feedback_request(attempt).unwrap();
    let before = state.library.clone();
    for change in ["skill", "prompt", "reference", "exercise", "cues"] {
        let mut wrong = request.clone();
        let TeacherRequest::AnswerFeedback(r) = &mut wrong else {
            unreachable!()
        };
        match change {
            "skill" => r.skill = TeacherSkill::Writing,
            "prompt" => r.prompt = "A different question with the same answer label".into(),
            "reference" => r.reference = Some("A different answer key".into()),
            "exercise" => r.exercise_id = Some("ru.exercise.unrelated".into()),
            "cues" => r.cues = vec!["translation".into()],
            _ => unreachable!(),
        }
        let response = feedback(&wrong);
        assert!(
            state.keep_teacher_response(&wrong, &response).is_err(),
            "the exact {change} evaluated by the model must match its persisted source"
        );
        assert_eq!(state.library, before);
    }
}

#[test]
fn teacher_journey_changed_retry_or_missing_provider_identity_cannot_replace_saved_feedback() {
    let (mut state, attempt) = writing();
    let request = state.answer_feedback_request(attempt).unwrap();
    let response = feedback(&request);
    state.keep_teacher_response(&request, &response).unwrap();
    let before = state.library.clone();
    let mut changed = response.clone();
    changed.content.explanation =
        "A different explanation under the same immutable provider response ID".into();
    assert!(state.keep_teacher_response(&request, &changed).is_err());
    assert_eq!(state.library, before);
    let mut missing = response.clone();
    missing.provenance.response_id.clear();
    assert!(state.keep_teacher_response(&request, &missing).is_err());
    assert_eq!(state.library, before);
    let mut wrong_source = response;
    wrong_source.provenance.review_status = ReviewStatus::UnreviewedAiSuggestion;
    assert!(
        state
            .keep_teacher_response(&request, &wrong_source)
            .is_err()
    );
    assert_eq!(state.library, before);
}

#[test]
fn teacher_journey_context_ambiguity_blocks_guessing_but_pinned_inflight_feedback_remains_valid() {
    let (mut state, attempt) = writing();
    let request = state.answer_feedback_request(attempt).unwrap();
    let response = feedback(&request);
    let (context_event, context) = state.library.contexts(attempt)[0];
    let original_id = context_event.id;
    let mut competing = context.clone();
    competing.prompt = Some("A different saved question".into());
    let extra = Event::new(
        state.device,
        repository::now_ms(),
        Fact::AttemptContextRecorded(competing),
    )
    .unwrap();
    state.repo.append(&[extra]).unwrap();
    state.refresh().unwrap();
    assert_eq!(state.library.contexts(attempt).len(), 2);
    assert!(
        state.answer_feedback_request(attempt).is_err(),
        "never pick a task description by timestamp"
    );
    state.keep_teacher_response(&request, &response).unwrap();
    assert_eq!(
        state
            .library
            .ai_assessments(attempt, JudgmentDimension::Grammar)[0]
            .1
            .context,
        Some(original_id),
        "the returned opinion still refers to its exact historical question"
    );
}

#[test]
fn teacher_journey_legacy_authored_response_is_not_reinterpreted_using_current_pack() {
    let (mut state, attempt) = choice();
    let (old, c) = state.library.contexts(attempt)[0];
    let old_id = old.id;
    let mut legacy = c.clone();
    legacy.prompt = None;
    legacy.reference_answer = None;
    legacy.supersedes = [old_id].into();
    let revised = Event::new(
        state.device,
        repository::now_ms(),
        Fact::AttemptContextRecorded(legacy),
    )
    .unwrap();
    state.repo.append(&[revised]).unwrap();
    state.refresh().unwrap();
    assert!(
        state.answer_feedback_request(attempt).is_err(),
        "a current exercise with the same ID cannot stand in for a missing historical question"
    );
    assert!(
        state
            .library
            .ai_assessments(attempt, JudgmentDimension::Comprehension)
            .is_empty()
    );
}

#[test]
fn teacher_journey_lesson_teacher_request_assigns_teaching_to_ai_without_friend_duties() {
    let state = app();
    let request = state.lesson_teaching_request().unwrap();
    request.validate().unwrap();
    let TeacherRequest::TeachLesson(lesson) = &request else {
        unreachable!()
    };
    let context = lesson.learner_context.as_ref().unwrap();
    assert!(context.contains("primary teacher"));
    assert!(context.contains("friend has no preparation, recording, grading or teaching duties"));
    assert!(!lesson.material.is_empty());
    assert!(!lesson.objective.is_empty());
    assert_eq!(
        state
            .library
            .ai_assessments(Uuid::new_v4(), JudgmentDimension::Grammar)
            .len(),
        0
    );
}
