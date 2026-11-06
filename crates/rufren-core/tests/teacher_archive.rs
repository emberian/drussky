use rufren_core::*;
use rufren_tutor_contract::*;
use serde_json::json;
use uuid::Uuid;

fn response() -> TeacherResponse {
    serde_json::from_value(json!({
        "request":{"mode":"teach_lesson","input":{"request_id":"public-request-one","lesson_id":"ru.lesson.greetings","title":"Greetings","objective":"Return a greeting.","material":"Привет! — Hello!","learner_context":null}},
        "content":{"explanation":"Use this greeting with someone you know.","examples":[{"russian":"Привет!","english":"Hello!","note":"Informal."}],"next_step":"Try replying.","caveats":[],"feedback":null,"practice":[{"prompt":"Greet a friend.","expected_response":"Привет!","teaching_note":"An informal greeting.","reference_russian":"Привет!","reference_english":"Hi!","skill":"writing"}]},
        "provenance":{"provider":"public-mock","model":"public-model","review_status":"ai_generated_teaching","scope":"Language content; no acoustic assessment.","prompt_version":"teacher-v4","response_id":"public-response-one"},
        "usage":{"accounting":"subscription_quota","input_tokens":100,"output_tokens":50,"estimated_micro_usd":0,"reserved_micro_usd":0}
    })).unwrap()
}
fn document(response: TeacherResponse, receipt: TeacherDocumentReceipt) -> TeacherDocument {
    TeacherDocument {
        format_version: TEACHER_DOCUMENT_FORMAT_VERSION,
        document_id: teacher_document_id(&response),
        response,
        receipt,
        context_source_fact_ids: None,
    }
}
fn archived(document: TeacherDocument, time: i64) -> Event {
    Event::new(
        Uuid::new_v4(),
        time,
        Fact::TeacherDocumentRecorded(document),
    )
    .unwrap()
}

#[test]
fn teacher_archive_independent_receipts_converge_without_replacing_original() {
    let answer = response();
    let expected_id = Uuid::new_v5(
        &Uuid::NAMESPACE_URL,
        b"rufren:teacher-practice:public-mock:public-model:public-response-one",
    );
    assert_eq!(teacher_document_id(&answer), expected_id);
    let live = archived(
        document(
            answer.clone(),
            TeacherDocumentReceipt::LiveReceived {
                received_at_ms: 100,
            },
        ),
        110,
    );
    let mut recovered_doc = document(answer.clone(), TeacherDocumentReceipt::RecoveredLocalCache);
    recovered_doc.context_source_fact_ids = Some(vec![Uuid::new_v4()]);
    let recovered = archived(recovered_doc, 9000);
    let mut a = Library::default();
    a.merge(&[live.clone(), recovered.clone(), live.clone()])
        .unwrap();
    let mut b = Library::default();
    b.merge(&[recovered, live]).unwrap();
    assert_eq!(a, b);
    assert_eq!(a.teacher_document(expected_id).unwrap().response, answer);
    assert_eq!(a.teacher_document_receipts(expected_id).len(), 2);
    let summaries = a.teacher_documents();
    assert_eq!(summaries.len(), 1);
    let s = &summaries[0];
    assert_eq!(s.kind, TeacherDocumentKind::Lesson);
    assert_eq!(s.lesson_id.as_deref(), Some("ru.lesson.greetings"));
    assert_eq!(s.latest_received_at_ms, Some(100));
    assert_eq!((s.first_archived_at_ms, s.last_archived_at_ms), (110, 9000));
    assert_eq!(s.practice_count, 1);
    assert_eq!(s.receipt_count, 2);
}

#[test]
fn teacher_archive_source_reuse_different_request_or_content_fails_atomically() {
    let original = archived(
        document(response(), TeacherDocumentReceipt::RecoveredLocalCache),
        20,
    );
    let mut library = Library::default();
    library.merge(&[original]).unwrap();
    let before = library.clone();
    for change_request in [false, true] {
        let mut changed = response();
        if change_request {
            let TeacherRequest::TeachLesson(r) = &mut changed.request else {
                unreachable!()
            };
            r.request_id = "other-request".into();
        } else {
            changed.content.explanation = "A different explanation.".into();
        }
        let incoming = archived(
            document(
                changed,
                TeacherDocumentReceipt::LiveReceived { received_at_ms: 50 },
            ),
            50,
        );
        let innocent = starter(Uuid::new_v4(), 50);
        assert!(library.merge(&[innocent, incoming]).is_err());
        assert_eq!(library, before);
    }
}

#[test]
fn teacher_archive_recovery_is_not_exposure_attempt_or_known_receive_time() {
    let mut library = Library::default();
    let id = teacher_document_id(&response());
    library
        .merge(&[archived(
            document(response(), TeacherDocumentReceipt::RecoveredLocalCache),
            12345,
        )])
        .unwrap();
    let summary = &library.teacher_documents()[0];
    assert_eq!(summary.latest_received_at_ms, None);
    assert_eq!(summary.first_archived_at_ms, 12345);
    assert_eq!(library.attempts().count(), 0);
    assert!(library.heads().is_empty());
    assert_eq!(
        library
            .events
            .values()
            .filter(|e| matches!(e.fact, Fact::StimulusPresented(_)))
            .count(),
        0
    );
    assert_eq!(library.teacher_document(id).unwrap().response, response());
}

#[test]
fn teacher_archive_preserves_feedback_request_and_does_not_promote_ai_to_human_evidence() {
    let attempt_id = Uuid::new_v4();
    let mut answer = response();
    answer.provenance.response_id = "public-feedback-one".into();
    answer.request = TeacherRequest::AnswerFeedback(AnswerFeedbackRequest {
        request_id: "public-feedback-request".into(),
        revision_id: Uuid::new_v4().to_string(),
        attempt_id: Some(attempt_id.to_string()),
        context_id: None,
        exercise_id: Some("teacher:v2:ru.lesson.greetings:source:0".into()),
        confirmed_transcript_id: None,
        skill: TeacherSkill::Writing,
        task: "Composition".into(),
        prompt: "Greet a friend.".into(),
        reference: Some("Привет!".into()),
        response: "Привет!".into(),
        cues: vec!["reference_material".into()],
        evidence: TextEvidence::WrittenText,
    });
    answer.content.feedback = Some(AiAnswerFeedback {
        verdict: AiVerdict::Correct,
        suggested_response: None,
        corrections: vec![],
    });
    let mut library = Library::default();
    library
        .merge(&[archived(
            document(answer.clone(), TeacherDocumentReceipt::RecoveredLocalCache),
            500,
        )])
        .unwrap();
    let s = &library.teacher_documents()[0];
    assert_eq!(s.kind, TeacherDocumentKind::Feedback);
    assert_eq!(
        s.attempt_id.as_deref(),
        Some(attempt_id.to_string().as_str())
    );
    assert_eq!(s.lesson_id, None); // A task/title cannot manufacture an origin lesson.
    assert_eq!(
        library.teacher_document(s.document_id).unwrap().response,
        answer
    );
    assert!(
        library
            .judgments(attempt_id, JudgmentDimension::Grammar)
            .is_empty()
    );
    assert!(
        library
            .ai_assessments(attempt_id, JudgmentDimension::Grammar)
            .is_empty()
    ); // Archive is not a second assessment.
}

#[test]
fn teacher_archive_validates_format_source_provenance_context_and_exact_contract() {
    let base = document(response(), TeacherDocumentReceipt::RecoveredLocalCache);
    let mut bad = Vec::new();
    let mut d = base.clone();
    d.format_version = 2;
    bad.push(d);
    let mut d = base.clone();
    d.document_id = Uuid::new_v4();
    bad.push(d);
    let mut d = base.clone();
    d.response.provenance.response_id.clear();
    bad.push(d);
    let mut d = base.clone();
    d.response.provenance.review_status = ReviewStatus::UnreviewedAiSuggestion;
    bad.push(d);
    let mut d = base.clone();
    d.response.content.feedback = Some(AiAnswerFeedback {
        verdict: AiVerdict::Correct,
        suggested_response: None,
        corrections: vec![],
    });
    bad.push(d);
    let mut d = base.clone();
    let id = Uuid::new_v4();
    d.context_source_fact_ids = Some(vec![id, id]);
    bad.push(d);
    let mut d = base.clone();
    d.context_source_fact_ids = Some(vec![Uuid::nil()]);
    bad.push(d);
    let mut d = base.clone();
    d.context_source_fact_ids = Some((0..65).map(|_| Uuid::new_v4()).collect());
    bad.push(d);
    let mut d = base.clone();
    d.receipt = TeacherDocumentReceipt::LiveReceived { received_at_ms: -1 };
    bad.push(d);
    let mut d = base.clone();
    d.response.content.explanation = "x".repeat(12001);
    bad.push(d);
    for d in bad {
        assert!(d.validate().is_err());
    }
    let mut event = archived(base, 5);
    let Fact::TeacherDocumentRecorded(d) = &mut event.fact else {
        unreachable!()
    };
    d.context_source_fact_ids = Some(vec![event.id]);
    assert!(event.validate().is_err());
}

#[test]
fn teacher_archive_event4_does_not_rewrite_old123_json_or_optional_teacher_fields() {
    let response_json = serde_json::to_string(&response()).unwrap();
    assert!(!response_json.contains("lesson_flow"));
    let answer: TeacherResponse = serde_json::from_str(&response_json).unwrap();
    assert_eq!(serde_json::to_string(&answer).unwrap(), response_json);
    let id = "00000000-0000-4000-8000-000000000001";
    let device = "00000000-0000-4000-8000-000000000002";
    let passage = "00000000-0000-4000-8000-000000000003";
    let mut old = Vec::new();
    let v1 = format!(
        r#"{{"version":1,"id":"{id}","device":"{device}","recorded_at_ms":10,"fact":{{"kind":"passage_revised","data":{{"passage":{{"id":"{passage}","title":"Hi","russian":"Привет!","translation":"Hi!","source":"Public synthetic fixture","coach_reviewed":false}},"parents":[]}}}}}}"#
    );
    let v2 = format!(
        r#"{{"version":2,"id":"00000000-0000-4000-8000-000000000004","device":"{device}","recorded_at_ms":11,"fact":{{"kind":"attempt_recorded","data":{{"session":"{passage}","revision":"{id}","skill":"writing","response":"Привет!","cues":["reference_material"],"self_rating":"with_help","policy":"teacher-guided-text-v1"}}}}}}"#
    );
    let v3 = format!(
        r#"{{"version":3,"id":"00000000-0000-4000-8000-000000000005","device":"{device}","recorded_at_ms":12,"fact":{{"kind":"attempt_recorded","data":{{"session":"{passage}","revision":"{id}","skill":"writing","response":"Привет!","cues":["reference_material"],"self_rating":"not_reported","policy":"teacher-guided-text-v1"}}}}}}"#
    );
    for raw in [v1, v2, v3] {
        let event: Event = serde_json::from_str(&raw).unwrap();
        event.validate().unwrap();
        assert_eq!(serde_json::to_string(&event).unwrap(), raw);
        old.push(event);
    }
    let mut library = Library::default();
    library.merge(&old).unwrap();
    let mut new = archived(
        document(answer, TeacherDocumentReceipt::RecoveredLocalCache),
        20,
    );
    for version in 1..=3 {
        new.version = version;
        assert!(matches!(new.validate(),Err(Error::Version(v)) if v == version));
    }
    new.version = 4;
    library.merge(&[new]).unwrap();
    for event in old {
        assert_eq!(library.events.get(&event.id).unwrap(), &event);
    }
    assert_eq!((EVENT_VERSION, SYNC_VERSION, SCHEMA_VERSION), (6, 6, 6));
}

#[test]
fn teacher_archive_guided_flow_survives_exact_roundtrip_and_validates_references() {
    let mut answer = response();
    let TeacherRequest::TeachLesson(request) = &mut answer.request else {
        unreachable!()
    };
    request.lesson_flow = Some(LessonFlowCapability::GuidedV1);
    answer.content.lesson_flow = Some(TeacherLessonFlow {
        steps: vec![TeacherLessonStep {
            id: "greet".into(),
            goal: "Return a greeting.".into(),
            orientation: "Read the example, then reply.".into(),
            detail: None,
            examples: vec![LessonExampleRef {
                example_index: 0,
                presentation: ExamplePresentation::Model,
            }],
            practice_index: Some(0),
        }],
    });
    let d = document(
        answer,
        TeacherDocumentReceipt::LiveReceived { received_at_ms: 10 },
    );
    d.validate().unwrap();
    let encoded = serde_json::to_vec(&d).unwrap();
    assert_eq!(
        serde_json::from_slice::<TeacherDocument>(&encoded).unwrap(),
        d
    );
    let mut bad = d;
    bad.response.content.lesson_flow.as_mut().unwrap().steps[0].practice_index = Some(4);
    assert!(bad.validate().is_err());
}
