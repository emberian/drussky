use rufren_core::*;
use rufren_tutor_contract::*;
use serde_json::json;
use uuid::Uuid;
fn request(id: Uuid) -> TeacherDialogueRequest {
    serde_json::from_value(json!({"request_id":id,"thread_id":id,"parent_reply_id":null,"config":{"intent":"clarification","topic":"Public greeting","teacher_role":null,"learner_role":null},"action":"ask","correction":"off","anchor":null,"ancestors":[],"omitted_parent_id":null,"omitted_count":0,"text":"Привет!"})).unwrap()
}
fn event(fact: Fact) -> Event {
    Event::new(Uuid::new_v4(), 100, fact).unwrap()
}
fn attachment(input: Uuid) -> Event {
    event(Fact::DialogueAudioAttached(DialogueAudioAttachment {
        thread_id: input,
        owner: DialogueAudioOwner::UserInput { input_id: input },
        asset_id: "dialogue-original-one".into(),
        asset: AudioAsset {
            sha256: "a".repeat(64),
            media_type: "audio/wav".into(),
            sample_rate_hz: 48000,
            channels: 1,
            frames: 96000,
        },
        provenance_json: None,
    }))
}
fn transcript(record: &Event, input: Uuid) -> Event {
    let audit = json!({"schema_version":"rufren-speech-transcript-v1","request_id":"asr-dialogue-one","attempt_id":null,"revision_id":null,"asset_id":"dialogue-original-one","audio_sha256":"a".repeat(64),"original_audio":{"sample_rate":48000,"channels":1,"frames":96000,"encoding":"pcm_s16le","bits":16},"source_pcm_sha256":"b".repeat(64),"source_pcm_hash_format":"interleaved-little-endian-f32-no-header-v1","span":{"start_frame":0,"end_frame":96000},"span_coordinate_system":"original_frames_half_open","derivative":{"sample_rate":16000,"channels":1,"frames":32000,"pcm_sha256":"c".repeat(64),"pcm_hash_format":"little-endian-f32-no-header-v1","method":"arithmetic-channel-mean; scipy.signal.resample_poly default Kaiser5","scipy_version":"1.17.0","amplitude_normalization":false},"evidence_kind":"transcript_hypothesis","transcript":"Привет!","language":"ru","language_selection":"explicit_not_detected","target_conditioned":false,"word_timestamps":null,"word_alignment_status":"not_run","confidence":null,"acoustic_assessment":null,"provenance":{"provider":"local_qwen3_asr","model":"worker-model","revision":"model-revision","runtime":"mlx-audio==0.6.4","mlx_version":"0.30.0","manifest_sha256":"d".repeat(64),"worker_sha256":"e".repeat(64),"decoding":{"temperature":0.0,"max_tokens":1024,"seed":42,"language":"Russian","hotwords":[]}},"timing":{"started_at":"2026-11-05T00:00:00Z","completed_at":"2026-11-05T00:00:01Z","elapsed_seconds":1.0,"process_peak_rss_bytes":1000000,"metal_peak_bytes":1000000},"usage":{"prompt_tokens":100,"generation_tokens":8}});
    Event {
        version: 5,
        id: Uuid::new_v5(&record.id, b"asr-dialogue-one"),
        device: Uuid::new_v4(),
        recorded_at_ms: 101,
        fact: Fact::DialogueSpeechTranscribed(DialogueSpeechTranscription {
            thread_id: input,
            input_id: input,
            recording: record.id,
            asset_id: "dialogue-original-one".into(),
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
            transcript: "Привет!".into(),
            language: "ru".into(),
            provenance: SpeechModelProvenance {
                schema_version: "rufren-speech-transcript-v1".into(),
                request_id: "asr-dialogue-one".into(),
                provider: "local_qwen3_asr".into(),
                model: "worker-model".into(),
                model_revision: "model-revision".into(),
                runtime: "mlx-audio==0.6.4".into(),
                manifest_sha256: "d".repeat(64),
                worker_sha256: "e".repeat(64),
            },
            audit_json: audit.to_string(),
        }),
    }
}
fn confirmation(input: Uuid, transcription: Uuid) -> Event {
    event(Fact::DialogueTranscriptConfirmed(
        DialogueTranscriptConfirmation {
            thread_id: input,
            input_id: input,
            transcription,
            text: "Привет!".into(),
            operation: TranscriptOperation::Confirm,
            supersedes: Default::default(),
        },
    ))
}
#[test]
fn dialogue_audio_complete_chain_is_exact_and_never_an_attempt_or_passage() {
    let input = Uuid::new_v4();
    let recording = attachment(input);
    let hypothesis = transcript(&recording, input);
    let adopted = confirmation(input, hypothesis.id);
    let source = DialogueConfirmedAudioRef {
        recording: recording.id,
        transcription: hypothesis.id,
        confirmation: adopted.id,
    };
    let mut r = request(input);
    r.confirmed_audio = Some(source.clone());
    let user = event(Fact::TeacherDialogueUserRecorded(r.clone()));
    let mut library = Library::default();
    library
        .merge(&[user, adopted, hypothesis, recording])
        .unwrap();
    assert_eq!(
        library
            .dialogue_confirmed_input(input, input, &source)
            .unwrap(),
        r.text
    );
    assert_eq!(library.attempts().count(), 0);
    assert!(library.heads().is_empty());
    assert!(SpacedReview.queue(&library, 1000, &Skill::ALL).is_empty());
    assert!(library.learner_snapshot(1000).features.is_empty());
    let mut wrong = r.clone();
    wrong.text = "Different words".into();
    assert!(
        library
            .merge(&[event(Fact::TeacherDialogueUserRecorded(wrong))])
            .is_err()
    );
    assert!(
        library
            .dialogue_confirmed_input(Uuid::new_v4(), input, &source)
            .is_err()
    );
}
#[test]
fn dialogue_audio_missing_parents_merge_but_known_wrong_input_is_rejected_atomically() {
    let input = Uuid::new_v4();
    let recording = attachment(input);
    let hypothesis = transcript(&recording, input);
    let mut library = Library::default();
    library.merge(&[hypothesis]).unwrap();
    let before = library.clone();
    let mut wrong = recording.clone();
    if let Fact::DialogueAudioAttached(a) = &mut wrong.fact {
        a.owner = DialogueAudioOwner::UserInput {
            input_id: Uuid::new_v4(),
        };
    }
    assert!(library.merge(&[wrong]).is_err());
    assert_eq!(library, before);
    library.merge(&[recording]).unwrap();
}
#[test]
fn dialogue_audio_confirmation_conflicts_need_explicit_adoption_without_rewriting_sent_words() {
    let input = Uuid::new_v4();
    let recording = attachment(input);
    let hypothesis = transcript(&recording, input);
    let adopted = confirmation(input, hypothesis.id);
    let mut second = adopted.clone();
    second.id = Uuid::new_v4();
    if let Fact::DialogueTranscriptConfirmed(c) = &mut second.fact {
        c.text = "Здравствуйте!".into();
        c.operation = TranscriptOperation::Edit;
    }
    let source = DialogueConfirmedAudioRef {
        recording: recording.id,
        transcription: hypothesis.id,
        confirmation: adopted.id,
    };
    let mut r = request(input);
    r.confirmed_audio = Some(source.clone());
    let sent = event(Fact::TeacherDialogueUserRecorded(r.clone()));
    let mut library = Library::default();
    library
        .merge(&[recording, hypothesis, adopted.clone(), sent, second.clone()])
        .unwrap();
    assert!(
        library
            .dialogue_confirmed_input(input, input, &source)
            .is_err()
    );
    assert_eq!(library.dialogue_user(input).unwrap().text, "Привет!");
    let mut resolution = adopted.clone();
    resolution.id = Uuid::new_v4();
    if let Fact::DialogueTranscriptConfirmed(c) = &mut resolution.fact {
        c.supersedes = [adopted.id, second.id].into_iter().collect();
    }
    library.merge(&[resolution.clone()]).unwrap();
    let selected = DialogueConfirmedAudioRef {
        confirmation: resolution.id,
        ..source
    };
    assert_eq!(
        library
            .dialogue_confirmed_input(input, input, &selected)
            .unwrap(),
        "Привет!"
    );
}
#[test]
fn dialogue_audio_asr_audit_cannot_carry_attempt_bindings_or_invent_acoustic_evidence() {
    let input = Uuid::new_v4();
    let recording = attachment(input);
    let mut hypothesis = transcript(&recording, input);
    if let Fact::DialogueSpeechTranscribed(t) = &mut hypothesis.fact {
        let mut audit: serde_json::Value = serde_json::from_str(&t.audit_json).unwrap();
        audit["attempt_id"] = json!(Uuid::new_v4());
        t.audit_json = audit.to_string();
    }
    assert!(hypothesis.validate().is_err());
    let mut hypothesis = transcript(&recording, input);
    if let Fact::DialogueSpeechTranscribed(t) = &mut hypothesis.fact {
        let mut audit: serde_json::Value = serde_json::from_str(&t.audit_json).unwrap();
        audit["acoustic_assessment"] = json!({"score":99});
        t.audit_json = audit.to_string();
    }
    assert!(hypothesis.validate().is_err());
}
#[test]
fn dialogue_audio_ai_voice_matches_exact_reply_chunk_and_original_asset_identity() {
    let input = Uuid::new_v4();
    let r = request(input);
    let response:TeacherDialogueResponse=serde_json::from_value(json!({"request":r,"content":{"explanation_english":"An informal greeting.","utterance":{"russian":"Привет!","english":"Hi!","note":"To a friend."},"next_question":null,"correction":null,"scene_state":"open"},"provenance":{"provider":"fixture","model":"fixture","response_id":"spoken-reply","review_status":"ai_generated_teaching","prompt_version":"v1","scope":"Synthetic text"},"usage":{"input_tokens":0,"output_tokens":0,"estimated_micro_usd":0,"reserved_micro_usd":0}})).unwrap();
    let reply = event(Fact::TeacherDialogueReplyRecorded(response.clone()));
    let hash = dialogue_text_sha256("Привет!");
    let make = |source: &str| {
        event(Fact::DialogueAudioAttached(DialogueAudioAttachment{thread_id:input,owner:DialogueAudioOwner::AssistantChunk{reply_id:teacher_dialogue_reply_id(&response),chunk_index:0,source_text_sha256:dialogue_text_sha256(source)},asset_id:"reply-original".into(),asset:AudioAsset{sha256:"f".repeat(64),media_type:"audio/wav".into(),sample_rate_hz:24000,channels:1,frames:2400},provenance_json:Some(json!({"kind":"system_generated_speech","provider":"fixture","model":"fixture","voice":"fixture","request_id":"voice-request","source_text":source,"source_text_sha256":dialogue_text_sha256(source)}).to_string())}))
    };
    let mut library = Library::default();
    library.merge(&[reply]).unwrap();
    assert!(library.merge(&[make("Пока!")]).is_err());
    library.merge(&[make("Привет!")]).unwrap();
    assert_eq!(
        library
            .dialogue_audio_attachments(&DialogueAudioOwner::AssistantChunk {
                reply_id: teacher_dialogue_reply_id(&response),
                chunk_index: 0,
                source_text_sha256: hash
            })
            .len(),
        1
    );
    let mut collision = make("Привет!");
    if let Fact::DialogueAudioAttached(a) = &mut collision.fact {
        a.asset.sha256 = "e".repeat(64);
    }
    assert!(library.merge(&[collision]).is_err());
}
