use rufren_tutor_contract::*;
use serde_json::json;

fn request(opt_in: bool) -> TeacherRequest {
    TeacherRequest::TeachLesson(LessonTeachingRequest {
        request_id: "guided-request".into(),
        lesson_id: "hello".into(),
        title: "A first exchange".into(),
        objective: "Understand and return a greeting.".into(),
        material: "Здравствуйте! — Hello!".into(),
        learner_context: None,
        lesson_flow: opt_in.then_some(LessonFlowCapability::GuidedV1),
    })
}

fn guided() -> TeacherContent {
    serde_json::from_value(json!({
        "explanation":"Learn a polite greeting, then understand and return it.",
        "examples":[{"russian":"Здравствуйте!","english":"Hello!","note":"Polite greeting."}],
        "next_step":"Try the first step.","caveats":[],"feedback":null,
        "practice":[
            {"prompt":"What does the written greeting mean?","expected_response":"Hello!","teaching_note":"Accept an equivalent English greeting.","reference_russian":"Здравствуйте!","reference_english":"Hello!","skill":"reading"},
            {"prompt":"Listen, then give the meaning in English.","expected_response":"Hello!","teaching_note":"Recognise a polite greeting.","reference_russian":"Здравствуйте!","reference_english":"Hello!","skill":"listening"},
            {"prompt":"Record a polite hello in Russian.","expected_response":"Здравствуйте!","teaching_note":"Accept an appropriate polite greeting.","reference_russian":"Здравствуйте!","reference_english":"Hello!","skill":"speaking"}
        ],
        "lesson_flow":{"steps":[
            {"id":"read-greeting","goal":"Recognise a polite greeting.","orientation":"Use this greeting when addressing someone politely.","detail":"The same greeting also addresses a group.","examples":[{"example_index":0,"presentation":"model"}],"practice_index":0},
            {"id":"hear-greeting","goal":"Recognise the greeting by listening.","orientation":"Listen first. Open a model only if you need support.","detail":null,"examples":[{"example_index":0,"presentation":"optional_support"}],"practice_index":1},
            {"id":"return-greeting","goal":"Greet someone politely.","orientation":"Record a short reply; confirm any transcript before requesting text feedback.","detail":null,"examples":[],"practice_index":2}
        ]}
    })).unwrap()
}

#[test]
fn lesson_flow_opt_in_round_trip_preserves_original_practice_indices() {
    let guided_request = request(true);
    guided_request.validate().unwrap();
    let content = guided();
    content.validate_for(&guided_request).unwrap();
    let round_trip: TeacherContent =
        serde_json::from_slice(&serde_json::to_vec(&content).unwrap()).unwrap();
    assert_eq!(round_trip, content);
    let steps = &content.lesson_flow.as_ref().unwrap().steps;
    assert_eq!(
        steps
            .iter()
            .filter_map(|s| s.practice_index)
            .collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    assert_eq!(
        steps[1].examples[0].presentation,
        ExamplePresentation::OptionalSupport
    );
    assert!(matches!(
        content.validate_for(&request(false)),
        Err(TutorError::InvalidResponse)
    ));
}

#[test]
fn lesson_flow_requires_both_explicit_capability_and_complete_valid_flow() {
    let mut content = guided();
    content.lesson_flow = None;
    content.validate_for(&request(false)).unwrap();
    assert!(matches!(
        content.validate_for(&request(true)),
        Err(TutorError::InvalidResponse)
    ));
    let mut encoded = serde_json::to_value(request(true)).unwrap();
    encoded["input"]["lesson_flow"] = json!("guided_v99");
    assert!(serde_json::from_value::<TeacherRequest>(encoded).is_err());
}

#[test]
fn lesson_flow_rejects_dangling_duplicate_and_uncovered_references() {
    let mutations: [fn(&mut TeacherLessonFlow); 6] = [
        |f| f.steps[0].examples[0].example_index = 6,
        |f| f.steps[2].practice_index = Some(3),
        |f| f.steps[2].practice_index = Some(0),
        |f| f.steps[1].practice_index = None,
        |f| f.steps[2].id = f.steps[0].id.clone(),
        |f| {
            let duplicate = f.steps[0].examples[0].clone();
            f.steps[0].examples.push(duplicate);
        },
    ];
    for mutate in mutations {
        let mut content = guided();
        mutate(content.lesson_flow.as_mut().unwrap());
        assert!(matches!(
            content.validate_for(&request(true)),
            Err(TutorError::InvalidResponse)
        ));
    }
}

#[test]
fn lesson_flow_requires_purposeful_bounded_steps_and_no_forced_listening_model() {
    let mutations: [fn(&mut TeacherLessonFlow); 7] = [
        |f| f.steps.clear(),
        |f| f.steps = vec![f.steps[0].clone(); LESSON_FLOW_MAX_STEPS + 1],
        |f| f.steps[0].goal = " ".into(),
        |f| f.steps[0].orientation = "x".repeat(2001),
        |f| f.steps[0].detail = Some("x".repeat(6001)),
        |f| {
            f.steps[2].practice_index = None;
            f.steps[2].examples.clear();
        },
        |f| f.steps[1].examples[0].presentation = ExamplePresentation::Model,
    ];
    for mutate in mutations {
        let mut content = guided();
        mutate(content.lesson_flow.as_mut().unwrap());
        assert!(matches!(
            content.validate_for(&request(true)),
            Err(TutorError::InvalidResponse)
        ));
    }
}

#[test]
fn lesson_flow_legacy_json_bytes_remain_unchanged() {
    let old_request = r#"{"mode":"teach_lesson","input":{"request_id":"old-request","lesson_id":"hello","title":"Hello","objective":"Say hello","material":"Здравствуйте!","learner_context":null}}"#;
    let parsed: TeacherRequest = serde_json::from_str(old_request).unwrap();
    assert!(!parsed.wants_lesson_flow());
    assert_eq!(serde_json::to_string(&parsed).unwrap(), old_request);
    let old_content = r#"{"explanation":"A greeting.","examples":[{"russian":"Здравствуйте!","english":"Hello!","note":"Polite."}],"next_step":"Try it.","caveats":[],"feedback":null,"practice":[{"prompt":"Say hello.","expected_response":"Здравствуйте!","teaching_note":"Polite."}]}"#;
    let parsed: TeacherContent = serde_json::from_str(old_content).unwrap();
    parsed.validate_for(&request(false)).unwrap();
    assert_eq!(serde_json::to_string(&parsed).unwrap(), old_content);
    let old_status = r#"{"enabled":false,"provider":null,"model":null,"message":"Not configured","data_sent":"Text","max_text_bytes":8192,"max_context_bytes":2048,"remaining_reservations":0,"remaining_budget_micro_usd":0,"accounting":"api_token_estimate"}"#;
    let parsed: TutorStatus = serde_json::from_str(old_status).unwrap();
    assert!(parsed.lesson_flow.is_none());
    assert_eq!(serde_json::to_string(&parsed).unwrap(), old_status);
}
