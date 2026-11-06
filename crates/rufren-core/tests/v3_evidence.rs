use rufren_core::*;
use serde_json::json;
use std::collections::BTreeSet;
use uuid::Uuid;
fn event(time: i64, fact: Fact) -> Event {
    Event::new(Uuid::new_v4(), time, fact).unwrap()
}
fn speech() -> (Library, Event, Event, SpeechTranscription) {
    let revision = starter(Uuid::new_v4(), 0);
    let hash = "a".repeat(64);
    let attempt = event(
        1,
        Fact::AttemptRecorded(Attempt {
            session: Uuid::new_v4(),
            revision: revision.id,
            skill: Skill::Speaking,
            response: "Recording captured: asset-one".into(),
            cues: vec![],
            self_rating: SelfRating::NotReported,
            policy: "spoken-response".into(),
        }),
    );
    let recording = event(
        1,
        Fact::RecordingAttached(AudioAttachment {
            asset_id: "asset-one".into(),
            asset: AudioAsset {
                sha256: hash.clone(),
                media_type: "audio/wav".into(),
                sample_rate_hz: 48000,
                channels: 1,
                frames: 96000,
            },
            target: AnnotationTarget::Attempt(attempt.id),
            role: AudioRole::Attempt,
            note: "Original WAV".into(),
        }),
    );
    let audit = json!({"schema_version":"rufren-speech-transcript-v1","request_id":"asr-request-one","attempt_id":attempt.id.to_string(),"revision_id":revision.id.to_string(),"asset_id":"asset-one","audio_sha256":hash,"original_audio":{"sample_rate":48000,"channels":1,"frames":96000,"encoding":"pcm_s16le","bits":16},"source_pcm_sha256":"b".repeat(64),"source_pcm_hash_format":"interleaved-little-endian-f32-no-header-v1","span":{"start_frame":0,"end_frame":96000},"span_coordinate_system":"original_frames_half_open","derivative":{"sample_rate":16000,"channels":1,"frames":32000,"pcm_sha256":"c".repeat(64),"pcm_hash_format":"little-endian-f32-no-header-v1","method":"arithmetic-channel-mean; scipy.signal.resample_poly default Kaiser5","scipy_version":"1.17.0","amplitude_normalization":false},"evidence_kind":"transcript_hypothesis","transcript":"Я читаю книгу.","language":"ru","language_selection":"explicit_not_detected","target_conditioned":false,"word_timestamps":null,"word_alignment_status":"not_run","confidence":null,"acoustic_assessment":null,"provenance":{"provider":"local_qwen3_asr","model":"worker-model","revision":"model-revision","runtime":"mlx-audio==0.6.4","mlx_version":"0.30.0","manifest_sha256":"d".repeat(64),"worker_sha256":"e".repeat(64),"decoding":{"temperature":0.0,"max_tokens":1024,"seed":42,"language":"Russian","hotwords":[]}},"timing":{"started_at":"2026-11-05T00:00:00Z","completed_at":"2026-11-05T00:00:01Z","elapsed_seconds":1.0,"process_peak_rss_bytes":1000000,"metal_peak_bytes":1000000},"usage":{"prompt_tokens":100,"generation_tokens":8}});
    let transcription = SpeechTranscription {
        recording: recording.id,
        attempt: attempt.id,
        revision: revision.id,
        asset_id: "asset-one".into(),
        audio_sha256: "a".repeat(64),
        span: OriginalFrameSpan {
            start_frame: 0,
            end_frame: 96000,
        },
        original_audio: SpeechOriginalAudio {
            sample_rate_hz: 48000,
            channels: 1,
            frames: 96000,
            encoding: "pcm_s16le".into(),
            bits: 16,
        },
        transcript: "Я читаю книгу.".into(),
        language: "ru".into(),
        provenance: SpeechModelProvenance {
            schema_version: "rufren-speech-transcript-v1".into(),
            request_id: "asr-request-one".into(),
            provider: "local_qwen3_asr".into(),
            model: "worker-model".into(),
            model_revision: "model-revision".into(),
            runtime: "mlx-audio==0.6.4".into(),
            manifest_sha256: "d".repeat(64),
            worker_sha256: "e".repeat(64),
        },
        audit_json: audit.to_string(),
    };
    let mut library = Library::default();
    library
        .merge(&[revision, attempt.clone(), recording.clone()])
        .unwrap();
    (library, attempt, recording, transcription)
}
fn confirmation(transcription: Uuid, text: &str, operation: TranscriptOperation) -> Event {
    event(
        3,
        Fact::TranscriptConfirmed(TranscriptConfirmation {
            transcription,
            text: text.into(),
            operation,
            supersedes: BTreeSet::new(),
        }),
    )
}
fn assessment(t: &SpeechTranscription, confirmation: Uuid, text: &str) -> AiAssessment {
    AiAssessment {
        request_id: "teacher-one".into(),
        attempt: t.attempt,
        revision: t.revision,
        skill: Skill::Speaking,
        task: TaskKind::FormRecall,
        response: text.into(),
        context: None,
        confirmed_transcript: Some(confirmation),
        evidence: AiTextEvidence::Transcript,
        dimension: JudgmentDimension::Grammar,
        verdict: Verdict::Correct,
        suggested_response: None,
        corrections: vec![],
        explanation: "This feedback concerns the learner-confirmed language content only.".into(),
        next_step: "Use these words in a different message.".into(),
        provenance: AiProvenance {
            provider: "teacher".into(),
            model: "model".into(),
            response_id: "teacher-response".into(),
            prompt_version: "teacher-method-v1".into(),
        },
        supersedes: BTreeSet::new(),
    }
}
fn change_audit(t: &mut SpeechTranscription, key: &str, value: serde_json::Value) {
    let mut audit: serde_json::Value = serde_json::from_str(&t.audit_json).unwrap();
    audit[key] = value;
    t.audit_json = audit.to_string();
}

#[test]
fn v3_transcript_confirmation_and_teacher_feedback_preserve_the_original_recording_attempt() {
    let (mut library, attempt, _, t) = speech();
    let original = library.attempt(attempt.id).unwrap().clone();
    let observed = event(2, Fact::SpeechTranscribed(t.clone()));
    let confirmed = confirmation(observed.id, &t.transcript, TranscriptOperation::Confirm);
    let feedback = event(
        4,
        Fact::AiAssessmentRecorded(assessment(&t, confirmed.id, &t.transcript)),
    );
    library
        .merge(&[feedback.clone(), confirmed.clone(), observed.clone()])
        .unwrap();
    assert_eq!(library.attempt(attempt.id).unwrap(), &original);
    assert_eq!(library.speech_transcriptions(attempt.id).len(), 1);
    assert_eq!(library.transcript_confirmations(observed.id).len(), 1);
    assert_eq!(
        library.ai_assessments(attempt.id, JudgmentDimension::Grammar)[0]
            .0
            .id,
        feedback.id
    );
    assert!(
        library
            .judgments(attempt.id, JudgmentDimension::Grammar)
            .is_empty()
    );
    assert_eq!(
        SpacedReview
            .project(
                &library,
                ReviewKey {
                    revision: t.revision,
                    skill: Skill::Speaking
                },
                5
            )
            .coach_confirmed_unassisted,
        0
    );
    let restored: Library =
        serde_json::from_str(&serde_json::to_string(&library).unwrap()).unwrap();
    assert_eq!(library, restored);
}

#[test]
fn v3_wrong_recording_hash_attempt_revision_and_worker_audit_are_rejected_atomically() {
    let (mut library, _, recording, t) = speech();
    let before = library.clone();
    let mut wrong = t.clone();
    wrong.recording = t.attempt;
    assert!(
        library
            .merge(&[event(2, Fact::SpeechTranscribed(wrong))])
            .is_err()
    );
    let mut wrong = t.clone();
    wrong.audio_sha256 = "f".repeat(64);
    let value = wrong.audio_sha256.clone();
    change_audit(&mut wrong, "audio_sha256", json!(value));
    assert!(
        library
            .merge(&[event(2, Fact::SpeechTranscribed(wrong))])
            .is_err()
    );
    let mut wrong = t.clone();
    wrong.attempt = Uuid::new_v4();
    let value = wrong.attempt.to_string();
    change_audit(&mut wrong, "attempt_id", json!(value));
    assert!(
        library
            .merge(&[event(2, Fact::SpeechTranscribed(wrong))])
            .is_err()
    );
    let mut wrong = t.clone();
    wrong.revision = Uuid::new_v4();
    let value = wrong.revision.to_string();
    change_audit(&mut wrong, "revision_id", json!(value));
    assert!(
        library
            .merge(&[event(2, Fact::SpeechTranscribed(wrong))])
            .is_err()
    );
    let mut wrong = t.clone();
    wrong.transcript = "Changed text outside the immutable worker response".into();
    assert!(wrong.validate().is_err());
    let mut wrong = t;
    wrong.span.end_frame += 1;
    assert!(wrong.validate().is_err());
    assert_eq!(library, before);
    assert!(library.events.contains_key(&recording.id));
}

#[test]
fn v3_empty_hypotheses_are_valid_but_confirmation_requires_explicit_nonempty_words() {
    let (mut library, _, _, mut t) = speech();
    t.transcript.clear();
    change_audit(&mut t, "transcript", json!(""));
    t.validate().unwrap();
    let observed = event(2, Fact::SpeechTranscribed(t));
    library.merge(&[observed.clone()]).unwrap();
    assert!(
        Event::new(
            Uuid::new_v4(),
            3,
            Fact::TranscriptConfirmed(TranscriptConfirmation {
                transcription: observed.id,
                text: String::new(),
                operation: TranscriptOperation::Confirm,
                supersedes: BTreeSet::new()
            })
        )
        .is_err()
    );
    let edit = confirmation(observed.id, "Привет.", TranscriptOperation::Edit);
    library.merge(&[edit]).unwrap();
    assert_eq!(
        library
            .speech_transcription(observed.id)
            .unwrap()
            .transcript,
        ""
    );
}

#[test]
fn v3_confirm_edits_and_conflicts_remain_historical_and_cannot_cross_transcriptions() {
    let (mut library, _, _, t) = speech();
    let observed = event(2, Fact::SpeechTranscribed(t.clone()));
    library.merge(&[observed.clone()]).unwrap();
    assert!(
        library
            .merge(&[confirmation(
                observed.id,
                "Different words",
                TranscriptOperation::Confirm
            )])
            .is_err()
    );
    let a = confirmation(observed.id, &t.transcript, TranscriptOperation::Confirm);
    let b = confirmation(observed.id, "Я читаю газету.", TranscriptOperation::Edit);
    library.merge(&[a.clone(), b.clone()]).unwrap();
    assert_eq!(library.transcript_confirmations(observed.id).len(), 2);
    let mut resolved = confirmation(observed.id, "Я читаю газету.", TranscriptOperation::Edit);
    if let Fact::TranscriptConfirmed(c) = &mut resolved.fact {
        c.supersedes = [a.id, b.id].into();
    }
    library.merge(&[resolved.clone()]).unwrap();
    assert_eq!(
        library.transcript_confirmations(observed.id)[0].0.id,
        resolved.id
    );
    let mut cross = confirmation(Uuid::new_v4(), "Some words", TranscriptOperation::Edit);
    if let Fact::TranscriptConfirmed(c) = &mut cross.fact {
        c.supersedes = [a.id].into();
    }
    assert!(library.merge(&[cross]).is_err());
    assert!(library.events.contains_key(&a.id) && library.events.contains_key(&b.id));
}

#[test]
fn v3_ai_cannot_skip_confirmation_change_confirmed_words_or_relabel_written_evidence() {
    let (mut library, _, _, t) = speech();
    let observed = event(2, Fact::SpeechTranscribed(t.clone()));
    let confirmed = confirmation(observed.id, &t.transcript, TranscriptOperation::Confirm);
    library.merge(&[observed, confirmed.clone()]).unwrap();
    let before = library.clone();
    let mut a = assessment(&t, confirmed.id, &t.transcript);
    a.confirmed_transcript = None;
    assert!(
        library
            .merge(&[event(4, Fact::AiAssessmentRecorded(a))])
            .is_err(),
        "literal attempt label remains immutable when no explicit transcript proof is supplied"
    );
    let mut a = assessment(&t, confirmed.id, &t.transcript);
    a.response = "Words never confirmed".into();
    assert!(
        library
            .merge(&[event(4, Fact::AiAssessmentRecorded(a))])
            .is_err()
    );
    let mut a = assessment(&t, confirmed.id, &t.transcript);
    a.evidence = AiTextEvidence::WrittenText;
    assert!(a.validate().is_err());
    let (_, _, _, other) = speech();
    let mut a = assessment(&t, confirmed.id, &t.transcript);
    a.attempt = other.attempt;
    a.revision = other.revision;
    assert!(
        library
            .merge(&[event(4, Fact::AiAssessmentRecorded(a))])
            .is_err()
    );
    assert_eq!(library, before);
}

#[test]
fn v3_protocol_and_new_fact_gates_preserve_old_v1_v2_facts_without_rewriting_them() {
    assert!(EVENT_VERSION >= 3);
    assert!(SYNC_VERSION >= 3);
    for version in [1, 2] {
        let mut old = starter(Uuid::new_v4(), 0);
        old.version = version;
        let text = serde_json::to_string(&old).unwrap();
        let restored: Event = serde_json::from_str(&text).unwrap();
        restored.validate().unwrap();
        assert_eq!(serde_json::to_string(&restored).unwrap(), text);
    }
    let (_, attempt, _, t) = speech();
    let presented = event(
        1,
        Fact::StimulusPresented(StimulusPresentation {
            session: None,
            revision: None,
            russian: "Привет".into(),
            purpose: PresentationPurpose::Reference,
            decision: None,
            utterance_family: None,
        }),
    );
    for mut new in [attempt, presented, event(2, Fact::SpeechTranscribed(t))] {
        new.version = 3;
        new.validate().unwrap();
        new.version = 2;
        assert!(matches!(new.validate(), Err(Error::Version(2))));
    }
}

#[test]
fn v3_unrated_responses_do_not_fabricate_recall_failure_or_move_existing_spacing() {
    let revision = starter(Uuid::new_v4(), 0);
    let key = ReviewKey {
        revision: revision.id,
        skill: Skill::Writing,
    };
    let mut library = Library::default();
    library.merge(&[revision.clone()]).unwrap();
    let attempt = |rating, time| {
        event(
            time,
            Fact::AttemptRecorded(Attempt {
                session: Uuid::new_v4(),
                revision: revision.id,
                skill: Skill::Writing,
                response: "Typed for teacher feedback".into(),
                cues: vec![Cue::ReferenceMaterial],
                self_rating: rating,
                policy: "interactive-teacher".into(),
            }),
        )
    };
    library
        .merge(&[attempt(SelfRating::NotReported, 1)])
        .unwrap();
    let unrated = SpacedReview.project(&library, key, 2);
    assert_eq!(unrated.interval_ms, 0);
    assert_eq!(unrated.last_evidence, Some(ReviewEvidence::NotReported));
    assert_eq!(unrated.successful_spaced_reviews, 0);
    assert_eq!(unrated.unassisted_self_reports, 0);
    let mut rated = attempt(SelfRating::Recalled, 3);
    if let Fact::AttemptRecorded(a) = &mut rated.fact {
        a.cues.clear();
    }
    library.merge(&[rated]).unwrap();
    let before = SpacedReview.project(&library, key, 4);
    assert_eq!(before.interval_ms, DAY_MS);
    library
        .merge(&[attempt(SelfRating::NotReported, 2 * DAY_MS)])
        .unwrap();
    let after = SpacedReview.project(&library, key, 2 * DAY_MS);
    assert_eq!(after.due_at_ms, before.due_at_ms);
    assert_eq!(after.interval_ms, before.interval_ms);
    assert_eq!(after.successful_spaced_reviews, 0);
}

#[test]
fn v3_presentation_history_recognizes_embedded_phrases_and_pre_log_uncertainty() {
    let revision = starter(Uuid::new_v4(), 0);
    let mut library = Library::default();
    library.merge(&[revision.clone()]).unwrap();
    assert_eq!(
        library.stimulus_novelty(revision.id, 10, None),
        StimulusNovelty::NoPriorRecordedExposure
    );
    let display = event(
        1,
        Fact::StimulusPresented(StimulusPresentation {
            session: None,
            revision: None,
            russian: format!(
                "Привет! {} До свидания.",
                if let Fact::PassageRevised(r) = &revision.fact {
                    r.passage.russian.to_uppercase()
                } else {
                    unreachable!()
                }
            ),
            purpose: PresentationPurpose::Instruction,
            decision: None,
            utterance_family: None,
        }),
    );
    library.merge(&[display.clone()]).unwrap();
    assert_eq!(
        library.stimulus_novelty(revision.id, 10, None),
        StimulusNovelty::PreviouslyExposed { event: display.id }
    );
    let mut old = starter(Uuid::new_v4(), 0);
    old.version = 2;
    let mut legacy = Library::default();
    legacy.merge(&[old.clone()]).unwrap();
    assert_eq!(
        legacy.stimulus_novelty(old.id, 10, None),
        StimulusNovelty::NoPriorRecordedExposure
    );
    let mut unlogged_probe = event(
        2,
        Fact::AttemptRecorded(Attempt {
            session: Uuid::new_v4(),
            revision: old.id,
            skill: Skill::Reading,
            response: "Legacy response".into(),
            cues: vec![],
            self_rating: SelfRating::Recalled,
            policy: "legacy-probe".into(),
        }),
    );
    unlogged_probe.version = 2;
    legacy.merge(&[unlogged_probe.clone()]).unwrap();
    assert_eq!(
        legacy.stimulus_novelty(old.id, 10, Some(unlogged_probe.id)),
        StimulusNovelty::UnknownHistory,
        "a legacy completed probe without a first-input presentation cannot prove pre-response novelty"
    );
}

#[test]
fn v3_probe_own_first_input_is_not_prior_restudy_but_answer_exposure_is() {
    let mut revision = starter(Uuid::new_v4(), 0);
    revision.version = 2; // New authored definitions remain pinned v2; definitions are not exposure.
    let russian = if let Fact::PassageRevised(r) = &revision.fact {
        r.passage.russian.clone()
    } else {
        unreachable!()
    };
    let session = Uuid::new_v4();
    let a = event(
        10,
        Fact::AttemptRecorded(Attempt {
            session,
            revision: revision.id,
            skill: Skill::Reading,
            response: "Meaning".into(),
            cues: vec![],
            self_rating: SelfRating::Recalled,
            policy: "probe".into(),
        }),
    );
    let prompt = event(
        1,
        Fact::StimulusPresented(StimulusPresentation {
            session: Some(session),
            revision: Some(revision.id),
            russian: russian.clone(),
            purpose: PresentationPurpose::TaskInput,
            decision: None,
            utterance_family: None,
        }),
    );
    let mut library = Library::default();
    library
        .merge(&[revision.clone(), prompt.clone(), a.clone()])
        .unwrap();
    assert_eq!(
        library.stimulus_novelty(revision.id, 10, Some(a.id)),
        StimulusNovelty::NoPriorRecordedExposure
    );
    let answer = event(
        0,
        Fact::StimulusPresented(StimulusPresentation {
            session: Some(session),
            revision: Some(revision.id),
            russian,
            purpose: PresentationPurpose::Answer,
            decision: None,
            utterance_family: None,
        }),
    );
    library.merge(&[answer.clone()]).unwrap();
    assert_eq!(
        library.stimulus_novelty(revision.id, 10, Some(a.id)),
        StimulusNovelty::PreviouslyExposed { event: answer.id }
    );
}
