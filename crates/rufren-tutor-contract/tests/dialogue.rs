use rufren_tutor_contract::*;
use uuid::Uuid;
fn id(n: u128) -> Uuid {
    Uuid::from_u128(n)
}
fn request() -> TeacherDialogueRequest {
    TeacherDialogueRequest {
        request_id: id(1),
        thread_id: id(1),
        parent_reply_id: None,
        config: DialogueConfig {
            intent: DialogueIntent::Clarification,
            topic: "Location and direction".into(),
            teacher_role: None,
            learner_role: None,
        },
        action: DialogueAction::Ask,
        correction: DialogueCorrection::Off,
        anchor: None,
        ancestors: vec![],
        omitted_parent_id: None,
        omitted_count: 0,
        text: "Why is it в школе?".into(),
        confirmed_audio: None,
    }
}
fn reply() -> TeacherDialogueContent {
    TeacherDialogueContent {
        explanation_english: Some(
            "This expresses a location, so use the prepositional case.".into(),
        ),
        utterance: Some(TutorExample {
            russian: "Я в шко́ле.".into(),
            english: "I am at school.".into(),
            note: "Location.".into(),
        }),
        next_question: None,
        correction: None,
        scene_state: DialogueSceneState::Open,
    }
}
fn continuation() -> TeacherDialogueRequest {
    let mut r = request();
    r.request_id = id(3);
    r.parent_reply_id = Some(id(2));
    r.ancestors = vec![
        DialogueAncestor {
            message_id: id(1),
            role: DialogueRole::User,
            text: "Why is it в школе?".into(),
        },
        DialogueAncestor {
            message_id: id(2),
            role: DialogueRole::Assistant,
            text: "It expresses a location.".into(),
        },
    ];
    r
}
#[test]
fn dialogue_initial_and_contiguous_suffix_round_trip() {
    for r in [request(), continuation()] {
        r.validate().unwrap();
        let bytes = serde_json::to_vec(&r).unwrap();
        assert_eq!(
            serde_json::from_slice::<TeacherDialogueRequest>(&bytes).unwrap(),
            r
        );
        reply().validate_for(&r).unwrap();
    }
    let mut r = continuation();
    r.thread_id = id(9);
    r.omitted_count = 2;
    r.omitted_parent_id = Some(id(8));
    r.validate().unwrap();
}
#[test]
fn dialogue_rejects_ambiguous_parent_role_omission_and_duplicate_identity() {
    let edits: [fn(&mut TeacherDialogueRequest); 8] = [
        |r| r.parent_reply_id = Some(id(8)),
        |r| r.ancestors[0].role = DialogueRole::Assistant,
        |r| r.ancestors[0].message_id = r.request_id,
        |r| {
            r.ancestors.pop();
        },
        |r| r.omitted_count = 2,
        |r| r.omitted_parent_id = Some(id(8)),
        |r| r.thread_id = id(8),
        |r| {
            r.omitted_count = 2;
            r.omitted_parent_id = Some(id(8));
        },
    ];
    for edit in edits {
        let mut r = continuation();
        edit(&mut r);
        assert!(r.validate().is_err());
    }
}
#[test]
fn dialogue_empty_start_is_action_not_fictional_learner_text() {
    let mut r = request();
    r.text.clear();
    assert!(r.validate().is_err());
    r.action = DialogueAction::StartScene;
    assert!(r.validate().is_err());
    r.config.intent = DialogueIntent::Roleplay;
    r.validate().unwrap();
    assert!(reply().validate_for(&r).is_ok());
    r.correction = DialogueCorrection::OneTargeted;
    let mut invented = reply();
    invented.correction = Some(TextCorrection {
        original: String::new(),
        replacement: "Здравствуйте!".into(),
        reason: "An invented learner production.".into(),
    });
    assert!(invented.validate_for(&r).is_err());
    r.text = "Begin a polite first meeting.".into();
    r.validate().unwrap();
    assert!(invented.validate_for(&r).is_err());
    r.text.clear();
    let mut no_russian = reply();
    no_russian.utterance = None;
    assert!(no_russian.validate_for(&r).is_err());
    r.confirmed_audio = Some(DialogueConfirmedAudioRef {
        recording: id(4),
        transcription: id(5),
        confirmation: id(6),
    });
    assert!(r.validate().is_err());
}
#[test]
fn dialogue_bounds_keep_whole_messages_and_exact_anchor() {
    let mut r = request();
    r.text = "а".repeat(1024);
    r.validate().unwrap();
    r.text.push('а');
    assert!(r.validate().is_err());
    let mut r = request();
    r.anchor = Some(TeacherDialogueAnchor {
        document_id: id(9),
        focus: TeacherDialogueFocus::Step {
            step_id: "location".into(),
        },
        snapshot_version: 1,
        snapshot: "x".repeat(DIALOGUE_MAX_ANCHOR_BYTES),
    });
    r.validate().unwrap();
    r.anchor.as_mut().unwrap().snapshot.push('x');
    assert!(r.validate().is_err());
    r.anchor.as_mut().unwrap().snapshot = "A quoted source.".into();
    r.anchor.as_mut().unwrap().snapshot_version = 2;
    assert!(r.validate().is_err());
    let mut r = continuation();
    r.ancestors[1].text = "x".repeat(7169);
    assert!(r.validate().is_err());
    let mut r = continuation();
    r.ancestors = vec![r.ancestors[0].clone(); 10];
    assert!(r.validate().is_err());
}
#[test]
fn dialogue_current_literal_correction_and_structural_question_slot() {
    let mut r = request();
    r.text = "Я в школу.".into();
    let mut c = reply();
    c.correction = Some(TextCorrection {
        original: "школу".into(),
        replacement: "школе".into(),
        reason: "Location.".into(),
    });
    assert!(c.validate_for(&r).is_err());
    r.correction = DialogueCorrection::OneTargeted;
    c.validate_for(&r).unwrap();
    c.correction.as_mut().unwrap().original = "ancestor-only".into();
    assert!(c.validate_for(&r).is_err());
    c.correction = None;
    c.next_question = Some(TutorExample {
        russian: "Как дела́?".into(),
        english: "How are things?".into(),
        note: "One question.".into(),
    });
    c.validate_for(&r).unwrap();
    c.utterance.as_mut().unwrap().russian = "«Где?» и «Когда?» — разные вопро́сы.".into();
    c.validate_for(&r).unwrap();
    let mut encoded = serde_json::to_value(&c).unwrap();
    encoded["next_question"] = serde_json::json!([
        encoded["next_question"].clone(),
        encoded["next_question"].clone()
    ]);
    assert!(serde_json::from_value::<TeacherDialogueContent>(encoded).is_err());
}
#[test]
fn dialogue_response_size_and_provenance_are_bounded() {
    let r = request();
    let mut c = reply();
    c.explanation_english = Some("x".repeat(4097));
    assert!(c.validate_for(&r).is_err());
    let mut c = reply();
    c.explanation_english = Some("x".repeat(4096));
    c.utterance.as_mut().unwrap().english = "x".repeat(1024);
    c.utterance.as_mut().unwrap().note = "x".repeat(512);
    c.correction = Some(TextCorrection {
        original: String::new(),
        replacement: "x".repeat(512),
        reason: "x".repeat(1024),
    });
    let mut r = request();
    r.correction = DialogueCorrection::OneTargeted;
    assert!(c.validate_for(&r).is_err());
    let response = TeacherDialogueResponse {
        request: request(),
        content: reply(),
        provenance: TutorProvenance {
            provider: "mock".into(),
            model: "mock".into(),
            response_id: "mock".into(),
            prompt_version: "rufren-ai-dialogue-v1".into(),
            review_status: ReviewStatus::AiGeneratedTeaching,
            scope: "Words only.".into(),
        },
        usage: TutorUsage {
            accounting: UsageAccounting::SubscriptionQuota,
            input_tokens: 0,
            output_tokens: 0,
            estimated_micro_usd: 0,
            reserved_micro_usd: 0,
        },
    };
    response.validate().unwrap();
}
