//! Bounded synthetic interaction tests. No model, microphone, credentials, or private corpus.
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
#[path = "../src/teacher_notebook.rs"]
mod teacher_notebook;
#[path = "../src/teacher_practice.rs"]
mod teacher_practice;
#[path = "../src/teacher_revisit.rs"]
mod teacher_revisit;

use base64::{Engine, engine::general_purpose::STANDARD};
use model::AppState;
use repository::Repository;
use rufren_audio::PcmAudio;
use rufren_core::*;
use rufren_speech::*;
use rufren_tutor::*;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::rc::Rc;
use uuid::Uuid;
fn app() -> AppState {
    AppState::from_repository(Rc::new(Repository::open_at(":memory:").unwrap())).unwrap()
}
fn provenance(id: &str) -> TutorProvenance {
    TutorProvenance {
        provider: "synthetic-provider".into(),
        model: "synthetic-model".into(),
        review_status: ReviewStatus::AiGeneratedTeaching,
        scope: "Language content; no acoustic judgment".into(),
        prompt_version: "teacher-test-v1".into(),
        response_id: id.into(),
    }
}
fn usage() -> TutorUsage {
    TutorUsage {
        accounting: UsageAccounting::SubscriptionQuota,
        input_tokens: 100,
        output_tokens: 30,
        estimated_micro_usd: 0,
        reserved_micro_usd: 0,
    }
}
fn lesson() -> TeacherResponse {
    let request = TeacherRequest::TeachLesson(LessonTeachingRequest {
        lesson_flow: None,
        request_id: Uuid::new_v4().to_string(),
        lesson_id: "ru.lesson.synthetic".into(),
        title: "Talking about reading".into(),
        objective: "Say that you are reading a book".into(),
        material: "Я читаю книгу. — I am reading a book.".into(),
        learner_context: None,
    });
    TeacherResponse {
        request,
        provenance: provenance("lesson-response"),
        usage: usage(),
        content: TeacherContent {
            lesson_flow: None,
            explanation: "Use читаю for what I am reading now.".into(),
            examples: vec![TutorExample {
                russian: "Я читаю книгу.".into(),
                english: "I am reading a book.".into(),
                note: "An example, not the only valid wording.".into(),
            }],
            next_step: "Try one short response.".into(),
            caveats: vec![],
            feedback: None,
            practice: vec![
                TeacherPractice {
                    prompt: "Write Russian expressing: I am reading a book.".into(),
                    expected_response: Some("Я читаю книгу.".into()),
                    teaching_note: "Accept valid alternatives preserving the message.".into(),
                    reference_russian: Some("Я читаю книгу.".into()),
                    reference_english: Some("I am reading a book.".into()),
                    skill: Some(TeacherSkill::Writing),
                },
                TeacherPractice {
                    prompt: "Explain this message in English.".into(),
                    expected_response: Some("I am reading a book.".into()),
                    teaching_note: "Meaning matters more than an exact English wording.".into(),
                    reference_russian: Some("Я читаю книгу.".into()),
                    reference_english: Some("I am reading a book.".into()),
                    skill: Some(TeacherSkill::Reading),
                },
            ],
        },
    }
}
fn feedback(request: &TeacherRequest) -> TeacherResponse {
    TeacherResponse {
        request: request.clone(),
        provenance: provenance("feedback-response"),
        usage: usage(),
        content: TeacherContent {
            lesson_flow: None,
            explanation:
                "This feedback concerns the words supplied for this exact task, not their sound."
                    .into(),
            examples: vec![],
            next_step: "Try another meaningful sentence.".into(),
            caveats: vec![],
            feedback: Some(AiAnswerFeedback {
                verdict: AiVerdict::Correct,
                suggested_response: None,
                corrections: vec![],
            }),
            practice: vec![],
        },
    }
}
fn recording(state: &mut AppState) -> (Uuid, Uuid, Vec<u8>) {
    let audio = PcmAudio::from_mono(
        16000,
        (0..16000)
            .map(|i| ((i % 64) as f32 - 32.0) / 256.0)
            .collect(),
    )
    .unwrap();
    let bytes = audio.to_wav();
    let mut revision = starter(state.device, 0);
    if let Fact::PassageRevised(r) = &mut revision.fact {
        r.passage.russian = "Я читаю книгу.".into();
        r.passage.translation = "I am reading a book.".into();
    }
    let attempt = Event::new(
        state.device,
        1,
        Fact::AttemptRecorded(Attempt {
            session: Uuid::new_v4(),
            revision: revision.id,
            skill: Skill::Speaking,
            response: "Saved recording label; not transcribed words".into(),
            cues: vec![],
            self_rating: SelfRating::NotReported,
            policy: "speaking-fixture".into(),
        }),
    )
    .unwrap();
    let attachment = Event::new(
        state.device,
        1,
        Fact::RecordingAttached(AudioAttachment {
            asset_id: format!("synthetic-{}", Uuid::new_v4()),
            asset: AudioAsset {
                sha256: format!("{:x}", Sha256::digest(&bytes)),
                media_type: "audio/wav".into(),
                sample_rate_hz: 16000,
                channels: 1,
                frames: 16000,
            },
            target: AnnotationTarget::Attempt(attempt.id),
            role: AudioRole::Attempt,
            note: "Synthetic public test waveform".into(),
        }),
    )
    .unwrap();
    let attempt_id = attempt.id;
    let recording_id = attachment.id;
    state.repo.append(&[revision, attempt, attachment]).unwrap();
    state.refresh().unwrap();
    (attempt_id, recording_id, bytes)
}
fn recognized(request: &RecognitionRequest) -> RecognitionResponse {
    let bytes = STANDARD.decode(&request.wav_base64).unwrap();
    let pcm = PcmAudio::from_wav(&bytes).unwrap();
    let metadata = pcm.metadata();
    let mut hash = Sha256::new();
    for sample in pcm.samples() {
        hash.update(sample.to_le_bytes());
    }
    let pcm_hash = format!("{:x}", hash.finalize());
    let value = json!({"schema_version":RECOGNITION_SCHEMA,"request_id":request.request_id,"attempt_id":request.attempt_id,"revision_id":request.revision_id,"asset_id":request.asset_id,"audio_sha256":request.audio_sha256,"original_audio":{"sample_rate":metadata.sample_rate,"channels":metadata.channels,"frames":metadata.frames,"encoding":"ieee_f32le","bits":32},"source_pcm_sha256":pcm_hash,"source_pcm_hash_format":"interleaved-little-endian-f32-no-header-v1","span":{"start_frame":0,"end_frame":metadata.frames},"span_coordinate_system":"original_frames_half_open","derivative":{"sample_rate":16000,"channels":1,"frames":metadata.frames,"pcm_sha256":pcm_hash,"pcm_hash_format":"little-endian-f32-no-header-v1","method":"arithmetic-channel-mean; scipy.signal.resample_poly default Kaiser5","scipy_version":"1.17.0","amplitude_normalization":false},"evidence_kind":"transcript_hypothesis","transcript":"Я читаю книгу.","language":"ru","language_selection":"explicit_not_detected","target_conditioned":false,"word_timestamps":null,"word_alignment_status":"not_run","confidence":null,"acoustic_assessment":null,"provenance":{"provider":"local_qwen3_asr","model":RECOGNITION_MODEL,"revision":RECOGNITION_REVISION,"runtime":RECOGNITION_RUNTIME,"mlx_version":"0.30.0","manifest_sha256":"d".repeat(64),"worker_sha256":"e".repeat(64),"decoding":{"temperature":0.0,"max_tokens":1024,"seed":42,"language":"Russian","hotwords":[]}},"timing":{"started_at":"2026-11-05T00:00:00Z","completed_at":"2026-11-05T00:00:01Z","elapsed_seconds":1.0,"process_peak_rss_bytes":1000000,"metal_peak_bytes":1000000},"usage":{"prompt_tokens":100,"generation_tokens":8}});
    let response: RecognitionResponse = serde_json::from_value(value).unwrap();
    response.validate_for(request).unwrap();
    response
}

#[test]
fn teacher_interaction_inline_practice_records_exact_question_without_invented_self_grade() {
    let mut state = app();
    let teacher = lesson();
    teacher.content.validate_for(&teacher.request).unwrap();
    for index in [0, 1] {
        let mut draft = state.teacher_practice_draft(&teacher, index).unwrap();
        draft.response = if index == 0 {
            "Я читаю книгу."
        } else {
            "I am reading a book."
        }
        .into();
        let attempt = state.submit_teacher_practice(&teacher, &mut draft).unwrap();
        let a = state.library.attempt(attempt).unwrap();
        assert_eq!(a.self_rating, SelfRating::NotReported);
        assert_eq!(a.cues, vec![Cue::ReferenceMaterial]);
        assert_eq!(a.response, draft.response);
        let contexts = state.library.contexts(attempt);
        assert_eq!(contexts.len(), 1);
        let c = contexts[0].1;
        let exercise = &teacher.content.practice[index];
        assert!(c.prompt.as_ref().unwrap().starts_with(&exercise.prompt));
        assert!(
            c.reference_answer
                .as_ref()
                .unwrap()
                .contains(exercise.reference_russian.as_ref().unwrap())
        );
        assert!(
            c.reference_answer
                .as_ref()
                .unwrap()
                .contains(exercise.reference_english.as_ref().unwrap())
        );
        assert!(
            c.reference_answer
                .as_ref()
                .unwrap()
                .contains(&exercise.teaching_note)
        );
        assert_eq!(
            c.task,
            if index == 0 {
                TaskKind::Composition
            } else {
                TaskKind::MeaningRecall
            }
        );
        assert!(c.target_features.is_empty());
        let review = SpacedReview.project(
            &state.library,
            ReviewKey {
                revision: a.revision,
                skill: a.skill,
            },
            repository::now_ms(),
        );
        assert_eq!(review.last_evidence, Some(ReviewEvidence::NotReported));
        assert_eq!(review.interval_ms, 0);
        assert_eq!(review.successful_spaced_reviews, 0);
        assert_eq!(review.coach_confirmed_unassisted, 0);
        let request = state.answer_feedback_request(attempt).unwrap();
        let TeacherRequest::AnswerFeedback(input) = request else {
            unreachable!()
        };
        assert_eq!(Some(input.prompt), c.prompt);
        assert_eq!(input.reference, c.reference_answer);
        assert_eq!(input.response, draft.response);
    }
}

#[test]
fn teacher_interaction_draft_reload_submit_retry_and_new_answer_preserve_both_attempts() {
    let mut state = app();
    let teacher = lesson();
    let mut draft = state.teacher_practice_draft(&teacher, 0).unwrap();
    draft.response = "Я читаю книгу.".into();
    state.save_teacher_practice_draft(&draft).unwrap();
    let id = draft.id;
    let repo = state.repo.clone();
    drop(state);
    let mut state = AppState::from_repository(repo).unwrap();
    let mut resumed = state.teacher_practice_draft(&teacher, 0).unwrap();
    assert_eq!(resumed.id, id);
    assert_eq!(resumed.response, draft.response);
    let first = state
        .submit_teacher_practice(&teacher, &mut resumed)
        .unwrap();
    let before = state.library.clone();
    assert_eq!(
        state
            .submit_teacher_practice(&teacher, &mut resumed)
            .unwrap(),
        first
    );
    assert_eq!(state.library, before);
    resumed.response = "Сейчас я читаю книгу.".into();
    assert!(
        state
            .submit_teacher_practice(&teacher, &mut resumed)
            .is_err()
    );
    assert_eq!(
        state.library, before,
        "a changed answer cannot overwrite the original attempt ID"
    );
    resumed.id = Uuid::new_v4();
    resumed.saved_attempt = None;
    state.save_teacher_practice_draft(&resumed).unwrap();
    let second = state
        .submit_teacher_practice(&teacher, &mut resumed)
        .unwrap();
    assert_ne!(first, second);
    assert_eq!(
        state.library.attempt(first).unwrap().response,
        "Я читаю книгу."
    );
    assert_eq!(
        state.library.attempt(second).unwrap().response,
        "Сейчас я читаю книгу."
    );
    assert_eq!(state.library.attempts().count(), 2);
}

#[test]
fn teacher_interaction_draft_cannot_submit_against_another_teacher_source() {
    let mut state = app();
    let teacher = lesson();
    let mut draft = state.teacher_practice_draft(&teacher, 0).unwrap();
    draft.response = "Я читаю книгу.".into();
    let before = state.library.clone();
    let mut changed = teacher.clone();
    changed.provenance.response_id = "another-provider-response".into();
    assert!(state.submit_teacher_practice(&changed, &mut draft).is_err());
    assert_eq!(state.library, before);
    let mut changed = teacher;
    changed.provenance.review_status = ReviewStatus::UnreviewedAiSuggestion;
    assert!(state.submit_teacher_practice(&changed, &mut draft).is_err());
    assert_eq!(state.library, before);
}

#[test]
fn teacher_interaction_asr_confirmation_edit_and_teacher_feedback_keep_exact_source_chain() {
    let mut state = app();
    let (attempt, recording, bytes) = recording(&mut state);
    let original = state.library.attempt(attempt).unwrap().clone();
    assert!(state.answer_feedback_request(attempt).is_err());
    let request = state
        .recognition_request(recording, &bytes, "recognition-one".into())
        .unwrap();
    let response = recognized(&request);
    let observed = state
        .keep_transcription(recording, &request, &response)
        .unwrap();
    assert_eq!(
        state
            .keep_transcription(recording, &request, &response)
            .unwrap(),
        observed
    );
    assert!(state.library.transcript_confirmations(observed).is_empty());
    let confirmed = state
        .confirm_transcription(observed, response.transcript.clone())
        .unwrap();
    assert_eq!(
        state
            .confirm_transcription(observed, response.transcript.clone())
            .unwrap(),
        confirmed
    );
    let edited = state
        .confirm_transcription(observed, "Сейчас я читаю книгу.".into())
        .unwrap();
    assert_ne!(confirmed, edited);
    assert_eq!(state.library.transcript_confirmations(observed).len(), 1);
    assert!(state.library.events.contains_key(&confirmed));
    assert_eq!(
        state
            .library
            .speech_transcription(observed)
            .unwrap()
            .transcript,
        response.transcript
    );
    let request = state.confirmed_transcript_feedback_request(edited).unwrap();
    let TeacherRequest::AnswerFeedback(input) = &request else {
        unreachable!()
    };
    assert_eq!(input.evidence, TextEvidence::Transcript);
    assert_eq!(input.confirmed_transcript_id, Some(edited.to_string()));
    assert_eq!(input.response, "Сейчас я читаю книгу.");
    assert_eq!(input.attempt_id, Some(attempt.to_string()));
    let answer = feedback(&request);
    state.keep_teacher_response(&request, &answer).unwrap();
    let opinions = state
        .library
        .ai_assessments(attempt, JudgmentDimension::Grammar);
    assert_eq!(opinions.len(), 1);
    let a = opinions[0].1;
    assert_eq!(a.confirmed_transcript, Some(edited));
    assert_eq!(a.evidence, AiTextEvidence::Transcript);
    assert_eq!(a.response, input.response);
    assert_eq!(state.library.attempt(attempt).unwrap(), &original);
    for dimension in [
        JudgmentDimension::Pronunciation,
        JudgmentDimension::Intelligibility,
        JudgmentDimension::Fluency,
    ] {
        assert!(state.library.ai_assessments(attempt, dimension).is_empty());
        assert!(state.library.judgments(attempt, dimension).is_empty());
    }
    let key = ReviewKey {
        revision: original.revision,
        skill: Skill::Speaking,
    };
    let review = SpacedReview.project(&state.library, key, repository::now_ms());
    assert_eq!(review.interval_ms, 0);
    assert_eq!(review.coach_confirmed_unassisted, 0);
}

#[test]
fn teacher_interaction_asr_rejects_wrong_bytes_hash_shape_encoding_and_other_take() {
    let mut state = app();
    let (_, take, bytes) = recording(&mut state);
    let mut altered = bytes.clone();
    let last = altered.len() - 1;
    altered[last] ^= 1;
    assert!(
        state
            .recognition_request(take, &altered, "recognition-wrong".into())
            .is_err()
    );
    let request = state
        .recognition_request(take, &bytes, "recognition-valid".into())
        .unwrap();
    let response = recognized(&request);
    let before = state.library.clone();
    let mut wrong = response.clone();
    wrong.audio_sha256 = "f".repeat(64);
    assert!(state.keep_transcription(take, &request, &wrong).is_err());
    assert_eq!(state.library, before);
    let mut wrong = response.clone();
    wrong.source_pcm_sha256 = "f".repeat(64);
    wrong.validate_for(&request).unwrap();
    assert!(
        state.keep_transcription(take, &request, &wrong).is_err(),
        "decoded source PCM identity must match the exact original WAV, independently of a provider claim"
    );
    assert_eq!(state.library, before);
    let mut wrong = response.clone();
    wrong.original_audio.frames += 1;
    wrong.span.end_frame += 1;
    wrong.derivative.frames += 1;
    wrong.validate_for(&request).unwrap();
    assert!(
        state.keep_transcription(take, &request, &wrong).is_err(),
        "the actual decoded source shape is independently checked"
    );
    assert_eq!(state.library, before);
    let mut wrong = response.clone();
    wrong.original_audio.encoding = "pcm_s16le".into();
    wrong.original_audio.bits = 16;
    wrong.validate_for(&request).unwrap();
    assert!(
        state.keep_transcription(take, &request, &wrong).is_err(),
        "the actual original float32 WAV header cannot become PCM16 by provider assertion"
    );
    assert_eq!(state.library, before);
    let (_, other, _) = recording(&mut state);
    let before = state.library.clone();
    assert!(
        state
            .keep_transcription(other, &request, &response)
            .is_err()
    );
    assert_eq!(state.library, before);
}

#[test]
fn teacher_interaction_confirmed_words_cannot_cross_attempts_and_empty_asr_is_not_auto_confirmed() {
    let mut state = app();
    let (attempt, take, bytes) = recording(&mut state);
    let request = state
        .recognition_request(take, &bytes, "recognition-empty".into())
        .unwrap();
    let mut response = recognized(&request);
    response.transcript.clear();
    response.validate_for(&request).unwrap();
    let observed = state.keep_transcription(take, &request, &response).unwrap();
    assert!(state.confirm_transcription(observed, " ".into()).is_err());
    assert!(state.library.transcript_confirmations(observed).is_empty());
    let confirmed = state
        .confirm_transcription(observed, "Привет.".into())
        .unwrap();
    assert_eq!(
        state
            .library
            .transcript_confirmation(confirmed)
            .unwrap()
            .operation,
        TranscriptOperation::Edit
    );
    assert_eq!(
        state.confirmed_words(confirmed, attempt).unwrap(),
        "Привет."
    );
    let (other, _, _) = recording(&mut state);
    assert!(state.confirmed_words(confirmed, other).is_err());
    let mut request = state
        .confirmed_transcript_feedback_request(confirmed)
        .unwrap();
    if let TeacherRequest::AnswerFeedback(input) = &mut request {
        input.attempt_id = Some(other.to_string());
    }
    let answer = feedback(&request);
    let before = state.library.clone();
    assert!(state.keep_teacher_response(&request, &answer).is_err());
    assert_eq!(state.library, before);
}

#[test]
fn teacher_interaction_latest_lesson_survives_feedback_reload_and_rejected_replacement() {
    let mut state = app();
    let first = lesson();
    state.keep_teacher_response(&first.request, &first).unwrap();
    assert_eq!(
        state
            .repo
            .get::<TeacherResponse>("teacher_lesson_response")
            .unwrap(),
        Some(first.clone())
    );
    let mut draft = state.teacher_practice_draft(&first, 0).unwrap();
    draft.response = "Я читаю книгу.".into();
    let attempt = state.submit_teacher_practice(&first, &mut draft).unwrap();
    let request = state.answer_feedback_request(attempt).unwrap();
    let answer = feedback(&request);
    state.keep_teacher_response(&request, &answer).unwrap();
    let repo = state.repo.clone();
    drop(state);
    let mut state = AppState::from_repository(repo).unwrap();
    assert_eq!(
        state
            .repo
            .get::<TeacherResponse>("teacher_response")
            .unwrap(),
        Some(answer.clone()),
        "feedback remains the current response"
    );
    assert_eq!(
        state
            .repo
            .get::<TeacherResponse>("teacher_lesson_response")
            .unwrap(),
        Some(first.clone()),
        "the complete exact lesson, including remaining exercises, survives reload"
    );
    let mut next = lesson();
    next.provenance.response_id = "second-lesson-response".into();
    next.content.practice[1].prompt = "Explain the new lesson's message.".into();
    for rejection in ["echo", "content", "provenance"] {
        let mut rejected = next.clone();
        match rejection {
            "echo" => rejected.request = first.request.clone(),
            "content" => rejected.content.practice.clear(),
            "provenance" => rejected.provenance.response_id.clear(),
            _ => unreachable!(),
        }
        assert!(
            state
                .keep_teacher_response(&next.request, &rejected)
                .is_err()
        );
        assert_eq!(
            state
                .repo
                .get::<TeacherResponse>("teacher_lesson_response")
                .unwrap(),
            Some(first.clone()),
            "rejected {rejection} cannot replace the accepted lesson"
        );
        assert_eq!(
            state
                .repo
                .get::<TeacherResponse>("teacher_response")
                .unwrap(),
            Some(answer.clone()),
            "rejected {rejection} preserves the current response contract"
        );
    }
    state.keep_teacher_response(&next.request, &next).unwrap();
    assert_eq!(
        state
            .repo
            .get::<TeacherResponse>("teacher_lesson_response")
            .unwrap(),
        Some(next.clone()),
        "a newly accepted lesson replaces only the latest-lesson slot"
    );
    assert_eq!(
        state
            .repo
            .get::<TeacherResponse>("teacher_response")
            .unwrap(),
        Some(next)
    );
}
