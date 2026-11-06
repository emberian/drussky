use super::*;
use axum::{
    Router,
    body::Body,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::Response,
    routing::post,
};
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    sync::atomic::{AtomicUsize, Ordering},
};

#[derive(Clone)]
struct Mock {
    replies: Arc<Mutex<VecDeque<(u16, Value)>>>,
    seen: Arc<Mutex<Vec<(String, Value)>>>,
    calls: Arc<AtomicUsize>,
    delay: Duration,
}

async fn mock(
    State(state): State<Mock>,
    headers: HeaderMap,
    request: axum::extract::Request,
) -> Response {
    state.calls.fetch_add(1, Ordering::SeqCst);
    assert_eq!(headers["x-api-key"], "test-key");
    assert_eq!(headers["anthropic-version"], "2023-06-01");
    let path = request.uri().path().to_owned();
    let body = axum::body::to_bytes(request.into_body(), 32768)
        .await
        .unwrap();
    state
        .seen
        .lock()
        .unwrap()
        .push((path, serde_json::from_slice(&body).unwrap()));
    if !state.delay.is_zero() {
        tokio::time::sleep(state.delay).await;
    }
    let (status, value) = state
        .replies
        .lock()
        .unwrap()
        .pop_front()
        .unwrap_or((500, json!({})));
    Response::builder()
        .status(StatusCode::from_u16(status).unwrap())
        .header("content-type", "application/json")
        .body(Body::from_stream(futures_util::stream::iter([Ok::<
            _,
            std::io::Error,
        >(
            value.to_string(),
        )])))
        .unwrap()
}

async fn service(
    replies: Vec<(u16, Value)>,
    delay: Duration,
    timeout: Duration,
    budget: u64,
) -> (TutorService, Mock, tokio::task::JoinHandle<()>) {
    let state = Mock {
        replies: Arc::new(Mutex::new(replies.into())),
        seen: Arc::default(),
        calls: Arc::default(),
        delay,
    };
    let app = Router::new()
        .route("/v1/messages", post(mock))
        .route("/v1/messages/count_tokens", post(mock))
        .with_state(state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (
        TutorService::configured("test-key".into(), budget, url, timeout).unwrap(),
        state,
        task,
    )
}

fn request() -> TutorRequest {
    TutorRequest {
        task: TutorTask::Explain,
        text: "Почему «в Москве», но «в Москву»?".into(),
        learner_context: Some("Beginning Russian".into()),
    }
}

fn content() -> Value {
    json!({"explanation":"Location and direction use different cases.","examples":[{"russian":"Я в Москве.","english":"I am in Moscow.","note":"Location."}],"next_step":"Contrast one location and one destination.","caveats":["Generated practice draft; verify examples with a coach."]})
}

fn response(content: Value) -> Value {
    json!({"id":"msg_mock_001","model":MODEL,"stop_reason":"end_turn","content":[{"type":"text","text":content.to_string()}],"usage":{"input_tokens":1000,"output_tokens":200}})
}

#[tokio::test]
async fn provider_success_preserves_provenance_and_bounded_request() {
    let (service, mock, task) = service(
        vec![
            (200, json!({"input_tokens":1000})),
            (200, response(content())),
        ],
        Duration::ZERO,
        Duration::from_secs(2),
        DEFAULT_SESSION_BUDGET_MICRO_USD,
    )
    .await;
    let result = service.suggest(&request()).await.unwrap();
    assert_eq!(
        result.provenance.review_status,
        ReviewStatus::UnreviewedAiSuggestion
    );
    assert_eq!(result.task, TutorTask::Explain);
    assert_eq!(result.usage.estimated_micro_usd, 8000);
    assert_eq!(result.usage.reserved_micro_usd, RESERVATION_MICRO_USD);
    let seen = mock.seen.lock().unwrap();
    assert_eq!(seen.len(), 2);
    assert_eq!(seen[0].0, "/v1/messages/count_tokens");
    assert_eq!(seen[1].0, "/v1/messages");
    assert_eq!(seen[0].1["messages"], seen[1].1["messages"]);
    assert_eq!(seen[0].1["output_config"], seen[1].1["output_config"]);
    assert_eq!(seen[1].1["max_tokens"], MAX_OUTPUT_TOKENS);
    assert_eq!(seen[1].1["thinking"]["type"], "adaptive");
    assert_eq!(seen[1].1["output_config"]["effort"], "max");
    assert!(seen[1].1.get("tools").is_none());
    task.abort();
}

#[tokio::test]
async fn provider_invalid_structured_content_is_rejected() {
    for invalid in [
        json!({"explanation":"Claim","correct":true}),
        json!({"explanation":"", "examples":[],"next_step":"Try","caveats":[]}),
        json!({"explanation":"Claim","examples":[],"next_step":"Try","caveats":[],"pronunciation_score":100}),
    ] {
        let (service, _, task) = service(
            vec![
                (200, json!({"input_tokens":1000})),
                (200, response(invalid)),
            ],
            Duration::ZERO,
            Duration::from_secs(2),
            DEFAULT_SESSION_BUDGET_MICRO_USD,
        )
        .await;
        assert!(matches!(
            service.suggest(&request()).await,
            Err(TutorError::InvalidResponse)
        ));
        task.abort();
    }
}

#[tokio::test]
async fn provider_auth_and_rate_errors_are_redacted_without_retries() {
    for (status, expected) in [
        (401, "authentication"),
        (403, "authentication"),
        (429, "rate"),
    ] {
        let (service, mock, task) = service(
            vec![(status, json!({"error":"private upstream detail test-key"}))],
            Duration::ZERO,
            Duration::from_secs(2),
            DEFAULT_SESSION_BUDGET_MICRO_USD,
        )
        .await;
        let error = service.suggest(&request()).await.unwrap_err();
        assert!(match expected {
            "rate" => matches!(error, TutorError::RateLimited),
            _ => matches!(error, TutorError::Authentication),
        });
        assert!(!error.to_string().contains("test-key"));
        assert_eq!(mock.calls.load(Ordering::SeqCst), 1);
        task.abort();
    }
}

#[tokio::test]
async fn provider_generation_rate_limit_consumes_reservation() {
    let (service, mock, task) = service(
        vec![(200, json!({"input_tokens":1000})), (429, json!({}))],
        Duration::ZERO,
        Duration::from_secs(2),
        RESERVATION_MICRO_USD,
    )
    .await;
    assert!(matches!(
        service.suggest(&request()).await,
        Err(TutorError::RateLimited)
    ));
    assert!(matches!(
        service.suggest(&request()).await,
        Err(TutorError::BudgetExceeded)
    ));
    assert_eq!(mock.calls.load(Ordering::SeqCst), 2);
    task.abort();
}

#[tokio::test]
async fn provider_timeout_does_not_retry_or_refund() {
    let (service, mock, task) = service(
        vec![(200, json!({"input_tokens":1000}))],
        Duration::from_millis(100),
        Duration::from_millis(25),
        RESERVATION_MICRO_USD,
    )
    .await;
    assert!(matches!(
        service.suggest(&request()).await,
        Err(TutorError::Timeout)
    ));
    assert!(matches!(
        service.suggest(&request()).await,
        Err(TutorError::BudgetExceeded)
    ));
    assert_eq!(mock.calls.load(Ordering::SeqCst), 1);
    task.abort();
}

#[tokio::test]
async fn provider_bounded_upload_rejects_before_network() {
    let (service, mock, task) = service(
        vec![],
        Duration::ZERO,
        Duration::from_secs(2),
        DEFAULT_SESSION_BUDGET_MICRO_USD,
    )
    .await;
    let mut input = request();
    input.text = "я".repeat(MAX_TEXT_BYTES / 2 + 1);
    assert!(matches!(
        service.suggest(&input).await,
        Err(TutorError::InvalidRequest)
    ));
    input.text = "Valid".into();
    input.learner_context = Some("x".repeat(MAX_CONTEXT_BYTES + 1));
    assert!(matches!(
        service.suggest(&input).await,
        Err(TutorError::InvalidRequest)
    ));
    assert_eq!(mock.calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        service.status().remaining_budget_micro_usd,
        DEFAULT_SESSION_BUDGET_MICRO_USD
    );
    task.abort();
}

#[tokio::test]
async fn provider_input_token_limit_stops_generation() {
    let (service, mock, task) = service(
        vec![(200, json!({"input_tokens":MAX_INPUT_TOKENS}))],
        Duration::ZERO,
        Duration::from_secs(2),
        DEFAULT_SESSION_BUDGET_MICRO_USD,
    )
    .await;
    assert!(matches!(
        service.suggest(&request()).await,
        Err(TutorError::BudgetExceeded)
    ));
    assert_eq!(mock.calls.load(Ordering::SeqCst), 1);
    task.abort();
}

#[tokio::test]
async fn provider_response_size_and_truncation_are_rejected() {
    let mut truncated = response(content());
    truncated["stop_reason"] = json!("max_tokens");
    for value in [
        json!({"padding":"x".repeat(MAX_RESPONSE_BYTES+1)}),
        truncated,
    ] {
        let (service, _, task) = service(
            vec![(200, json!({"input_tokens":1000})), (200, value.clone())],
            Duration::ZERO,
            Duration::from_secs(2),
            DEFAULT_SESSION_BUDGET_MICRO_USD,
        )
        .await;
        let error = service.suggest(&request()).await.unwrap_err();
        if value.get("padding").is_some() {
            assert!(matches!(error, TutorError::ResponseTooLarge));
        } else {
            assert!(matches!(error, TutorError::InvalidResponse));
        }
        task.abort();
    }
}

#[tokio::test]
async fn provider_disabled_is_explicit() {
    let service = TutorService::disabled();
    assert!(!service.status().enabled);
    assert!(matches!(
        service.suggest(&request()).await,
        Err(TutorError::Disabled)
    ));
}

fn teacher_request() -> TeacherRequest {
    TeacherRequest::AnswerFeedback(AnswerFeedbackRequest {
        request_id: "request-1".into(),
        revision_id: "revision-1".into(),
        attempt_id: Some("attempt-1".into()),
        context_id: Some("context-1".into()),
        exercise_id: Some("exercise-1".into()),
        confirmed_transcript_id: None,
        skill: TeacherSkill::Writing,
        task: "written_retrieval".into(),
        prompt: "Say that you are at school.".into(),
        reference: Some("Я в школе.".into()),
        response: "Я в школу.".into(),
        cues: vec!["hint".into()],
        evidence: TextEvidence::WrittenText,
    })
}
fn teacher_content() -> Value {
    let mut result = content();
    result["feedback"] = json!({"verdict":"partly_correct","suggested_response":"Я в школе.","corrections":[{"original":"школу","replacement":"школе","reason":"Use the prepositional case for location."}]});
    result["practice"] = json!([{"prompt":"Say “I am in Moscow.”","expected_response":"Я в Москве.","teaching_note":"Transfer the location pattern."}]);
    result
}

#[test]
fn provider_teacher_schema_advertises_all_four_executable_skills() {
    let schema = anthropic::teacher_schema();
    let practice = &schema["properties"]["practice"]["items"];
    assert_eq!(
        practice["properties"]["skill"]["enum"],
        json!(["reading", "writing", "listening", "speaking"])
    );
    let required = practice["required"].as_array().unwrap();
    for field in [
        "prompt",
        "expected_response",
        "teaching_note",
        "reference_russian",
        "reference_english",
        "skill",
    ] {
        assert!(required.contains(&json!(field)));
    }
}

#[tokio::test]
async fn provider_teacher_binds_exact_answer_and_ai_provenance() {
    let (service, mock, task) = service(
        vec![
            (200, json!({"input_tokens":1000})),
            (200, response(teacher_content())),
        ],
        Duration::ZERO,
        Duration::from_secs(2),
        DEFAULT_SESSION_BUDGET_MICRO_USD,
    )
    .await;
    let request = teacher_request();
    let result = service.teach(&request).await.unwrap();
    assert_eq!(result.request, request);
    assert_eq!(
        result.provenance.review_status,
        ReviewStatus::AiGeneratedTeaching
    );
    assert_eq!(result.provenance.response_id, "msg_mock_001");
    assert_eq!(result.provenance.prompt_version, "rufren-ai-teacher-v6");
    assert_eq!(
        result.content.feedback.unwrap().verdict,
        AiVerdict::PartlyCorrect
    );
    let seen = mock.seen.lock().unwrap();
    let submitted: TeacherRequest =
        serde_json::from_str(seen[1].1["messages"][0]["content"].as_str().unwrap()).unwrap();
    assert_eq!(submitted, request);
    task.abort();
}

#[tokio::test]
async fn provider_teacher_rejects_corrections_to_unsubmitted_words_and_missing_feedback() {
    let mut wrong = teacher_content();
    wrong["feedback"]["corrections"][0]["original"] = json!("unsubmitted word");
    let mut missing = teacher_content();
    missing["feedback"] = Value::Null;
    for invalid in [wrong, missing] {
        let (service, _, task) = service(
            vec![
                (200, json!({"input_tokens":1000})),
                (200, response(invalid)),
            ],
            Duration::ZERO,
            Duration::from_secs(2),
            DEFAULT_SESSION_BUDGET_MICRO_USD,
        )
        .await;
        assert!(matches!(
            service.teach(&teacher_request()).await,
            Err(TutorError::InvalidResponse)
        ));
        task.abort();
    }
}

#[tokio::test]
async fn provider_teacher_lesson_has_practice_without_an_answer_grade() {
    let request = TeacherRequest::TeachLesson(LessonTeachingRequest {
        request_id: "request-lesson".into(),
        lesson_id: "ru.lesson.location".into(),
        title: "Location".into(),
        objective: "Distinguish location and destination.".into(),
        material: "Я в школе. Я иду в школу.".into(),
        learner_context: None,
        lesson_flow: None,
    });
    let mut lesson = teacher_content();
    lesson["feedback"] = Value::Null;
    let (service, _, task) = service(
        vec![(200, json!({"input_tokens":1000})), (200, response(lesson))],
        Duration::ZERO,
        Duration::from_secs(2),
        DEFAULT_SESSION_BUDGET_MICRO_USD,
    )
    .await;
    let result = service.teach(&request).await.unwrap();
    assert_eq!(result.request, request);
    assert!(!result.content.practice.is_empty());
    assert!(result.content.feedback.is_none());
    task.abort();
}

#[tokio::test]
async fn provider_teacher_guided_schema_is_opt_in_and_binds_original_arrays() {
    let mut request = TeacherRequest::TeachLesson(LessonTeachingRequest {
        request_id: "guided-request".into(),
        lesson_id: "hello".into(),
        title: "A first exchange".into(),
        objective: "Greet politely.".into(),
        material: "Здравствуйте! — Hello!".into(),
        learner_context: None,
        lesson_flow: None,
    });
    assert!(
        anthropic::teacher_schema_for(&request)["properties"]
            .get("lesson_flow")
            .is_none()
    );
    if let TeacherRequest::TeachLesson(input) = &mut request {
        input.lesson_flow = Some(LessonFlowCapability::GuidedV1);
    }
    let mut output = teacher_content();
    output["feedback"] = Value::Null;
    output["lesson_flow"] = json!({"steps":[{"id":"location","goal":"Express a location.","orientation":"Try the location pattern.","detail":null,"examples":[],"practice_index":0}]});
    let (service, mock, task) = service(
        vec![(200, json!({"input_tokens":1000})), (200, response(output))],
        Duration::ZERO,
        Duration::from_secs(2),
        DEFAULT_SESSION_BUDGET_MICRO_USD,
    )
    .await;
    let result = service.teach(&request).await.unwrap();
    assert_eq!(result.request, request);
    assert_eq!(
        result.content.lesson_flow.unwrap().steps[0].practice_index,
        Some(0)
    );
    let seen = mock.seen.lock().unwrap();
    let schema = &seen[1].1["output_config"]["format"]["schema"];
    assert!(schema["properties"].get("lesson_flow").is_some());
    assert!(
        schema["required"]
            .as_array()
            .unwrap()
            .contains(&json!("lesson_flow"))
    );
    let submitted: TeacherRequest =
        serde_json::from_str(seen[1].1["messages"][0]["content"].as_str().unwrap()).unwrap();
    assert_eq!(submitted, request);
    task.abort();
}

#[tokio::test]
async fn provider_teacher_revisit_keeps_historical_task_bytes_and_current_attempt_policy() {
    let mut request = teacher_request();
    if let TeacherRequest::AnswerFeedback(input) = &mut request {
        input.exercise_id =
            Some("teacher:revisit:v1:11111111-1111-4111-8111-111111111111:0".into());
        input.reference = Some("Example answer: Я в школе.\nTeaching note: This is a first encounter; using the translation is expected.\nRufren teacher revisit v1: {\"format_version\":1,\"document_id\":\"11111111-1111-4111-8111-111111111111\",\"practice_index\":0,\"source_attempt_id\":\"22222222-2222-4222-8222-222222222222\",\"source_context_id\":\"33333333-3333-4333-8333-333333333333\",\"started_at_ms\":1234}".into());
        input.cues.clear();
    }
    let exact_input = serde_json::to_string(&request).unwrap();
    let (service, mock, task) = service(
        vec![
            (200, json!({"input_tokens":1000})),
            (200, response(teacher_content())),
        ],
        Duration::ZERO,
        Duration::from_secs(2),
        DEFAULT_SESSION_BUDGET_MICRO_USD,
    )
    .await;
    let result = service.teach(&request).await.unwrap();
    assert_eq!(result.request, request);
    assert_eq!(result.provenance.prompt_version, "rufren-ai-teacher-v6");
    let seen = mock.seen.lock().unwrap();
    // The provider receives the exact historical data, with the same server policy
    // used by CLI teaching. This checks transport/context precedence, not model truth.
    assert_eq!(seen[1].1["messages"][0]["content"], exact_input);
    assert_eq!(seen[1].1["system"], anthropic::TEACHER_SYSTEM);
    let system = seen[1].1["system"].as_str().unwrap();
    assert!(system.contains("teacher:revisit:v1:"));
    assert!(system.contains("Current-attempt evidence takes precedence"));
    assert!(system.contains("does not prove independent recall"));
    task.abort();
}

#[tokio::test]
async fn provider_teacher_upload_limits_precede_network_and_budget() {
    let (service, mock, task) = service(
        vec![],
        Duration::ZERO,
        Duration::from_secs(2),
        DEFAULT_SESSION_BUDGET_MICRO_USD,
    )
    .await;
    let TeacherRequest::AnswerFeedback(mut request) = teacher_request() else {
        unreachable!()
    };
    request.response = "x".repeat(8193);
    assert!(matches!(
        service
            .teach(&TeacherRequest::AnswerFeedback(request))
            .await,
        Err(TutorError::InvalidRequest)
    ));
    assert_eq!(mock.calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        service.status().remaining_budget_micro_usd,
        DEFAULT_SESSION_BUDGET_MICRO_USD
    );
    task.abort();
}

#[tokio::test]
async fn provider_api_reasoning_blocks_and_optional_model_prices_are_explicit() {
    let mut output = response(teacher_content());
    output["model"] = json!("claude-sonnet-5-5");
    let text = output["content"][0].clone();
    output["content"] = json!([{"type":"thinking","thinking":"Private reasoning is not forwarded as teaching content.","signature":"fixture"},text]);
    let (mut service, _, task) = service(
        vec![(200, json!({"input_tokens":1000})), (200, output)],
        Duration::ZERO,
        Duration::from_secs(2),
        DEFAULT_SESSION_BUDGET_MICRO_USD,
    )
    .await;
    let Some(Provider::Anthropic(api)) = service.provider.as_mut() else {
        unreachable!()
    };
    api.select_model("claude-sonnet-5-5").unwrap();
    assert_eq!(
        api.reservation(),
        MAX_INPUT_TOKENS * 2 + MAX_OUTPUT_TOKENS * 10
    );
    assert!(matches!(
        api.select_model("unpriced-model"),
        Err(TutorError::Configuration)
    ));
    let result = service.teach(&teacher_request()).await.unwrap();
    assert_eq!(result.provenance.model, "claude-sonnet-5-5");
    assert_eq!(result.usage.estimated_micro_usd, 4000);
    assert_eq!(
        result.usage.reserved_micro_usd,
        MAX_INPUT_TOKENS * 2 + MAX_OUTPUT_TOKENS * 10
    );
    assert_eq!(result.usage.accounting, UsageAccounting::ApiTokenEstimate);
    task.abort();
}

fn dialogue_request() -> TeacherDialogueRequest {
    serde_json::from_value(json!({
        "request_id":"10000000-0000-4000-8000-000000000001","thread_id":"10000000-0000-4000-8000-000000000001","parent_reply_id":null,
        "config":{"intent":"clarification","topic":"Location and direction","teacher_role":null,"learner_role":null},
        "action":"ask","correction":"off","anchor":null,"ancestors":[],"omitted_parent_id":null,"omitted_count":0,
        "text":"Why is it в школе?"
    })).unwrap()
}
fn dialogue_content() -> Value {
    json!({"explanation_english":"This means location, so use the prepositional case.",
        "utterance":{"russian":"Я в шко́ле.","english":"I am at school.","note":"Location."},
        "next_question":null,"correction":null,"scene_state":"open"})
}
#[tokio::test]
async fn provider_dialogue_targeted_help_preserves_prior_line_and_scoped_instruction() {
    let mut request = dialogue_request();
    let first = request.request_id;
    let parent = "10000000-0000-4000-8000-000000000002".parse().unwrap();
    request.request_id = "10000000-0000-4000-8000-000000000003".parse().unwrap();
    request.parent_reply_id = Some(parent);
    request.config.intent = DialogueIntent::Roleplay;
    request.config.topic = "A ship's AI offers tea.".into();
    request.text="Could you explain your last question in English and give me one short Russian reply I can use?".into();
    request.ancestors = vec![
        DialogueAncestor {
            message_id: first,
            role: DialogueRole::User,
            text: "[Learner action: start the scene]".into(),
        },
        DialogueAncestor {
            message_id: parent,
            role: DialogueRole::Assistant,
            text: "Хоти́те ча́ю?\nWould you like some tea?".into(),
        },
    ];
    let exact_input = serde_json::to_string(&request).unwrap();
    let (service, mock, task) = service(
        vec![
            (200, json!({"input_tokens":1000})),
            (200, response(dialogue_content())),
        ],
        Duration::ZERO,
        Duration::from_secs(2),
        DEFAULT_SESSION_BUDGET_MICRO_USD,
    )
    .await;
    let response = service.dialogue(&request).await.unwrap();
    assert_eq!(response.request, request);
    assert_eq!(response.provenance.prompt_version, "rufren-ai-dialogue-v2");
    let seen = mock.seen.lock().unwrap();
    assert_eq!(seen[1].1["messages"][0]["content"], exact_input);
    let system = seen[1].1["system"].as_str().unwrap();
    assert_eq!(system, dialogue::SYSTEM);
    assert!(system.contains("faithful brief meaning and one usable short model reply"));
    assert!(system.contains("written word-stress marks do not fix sentence pitch"));
    task.abort();
}
#[tokio::test]
async fn provider_dialogue_exact_binding_and_budget_shared_with_teaching() {
    let (service, mock, task) = service(
        vec![
            (200, json!({"input_tokens":1000})),
            (200, response(dialogue_content())),
        ],
        Duration::ZERO,
        Duration::from_secs(2),
        RESERVATION_MICRO_USD,
    )
    .await;
    let request = dialogue_request();
    let result = service.dialogue(&request).await.unwrap();
    assert_eq!(result.request, request);
    assert_eq!(result.provenance.prompt_version, "rufren-ai-dialogue-v2");
    assert_eq!(
        service.status().dialogue,
        Some(DialogueCapability::ThreadedV1)
    );
    assert!(matches!(
        service.teach(&teacher_request()).await,
        Err(TutorError::BudgetExceeded)
    ));
    let seen = mock.seen.lock().unwrap();
    assert_eq!(seen.len(), 2);
    assert_eq!(seen[1].1["system"], dialogue::SYSTEM);
    assert_eq!(
        seen[1].1["output_config"]["format"]["schema"],
        dialogue::schema()
    );
    let input: TeacherDialogueRequest =
        serde_json::from_str(seen[1].1["messages"][0]["content"].as_str().unwrap()).unwrap();
    assert_eq!(input, request);
    task.abort();
}
#[tokio::test]
async fn provider_dialogue_invalid_current_correction_rejected_without_retry() {
    let mut invalid = dialogue_content();
    invalid["correction"] =
        json!({"original":"школе","replacement":"школу","reason":"An unwanted correction."});
    let (service, mock, task) = service(
        vec![
            (200, json!({"input_tokens":1000})),
            (200, response(invalid)),
        ],
        Duration::ZERO,
        Duration::from_secs(2),
        DEFAULT_SESSION_BUDGET_MICRO_USD,
    )
    .await;
    assert!(matches!(
        service.dialogue(&dialogue_request()).await,
        Err(TutorError::InvalidResponse)
    ));
    assert_eq!(mock.calls.load(Ordering::SeqCst), 2);
    task.abort();
}
#[tokio::test]
async fn provider_dialogue_bounds_precede_network_and_quota_and_auth_redacts() {
    let (service, mock, task) = self::service(
        vec![],
        Duration::ZERO,
        Duration::from_secs(2),
        DEFAULT_SESSION_BUDGET_MICRO_USD,
    )
    .await;
    let mut invalid = dialogue_request();
    invalid.text = "x".repeat(2049);
    assert!(matches!(
        service.dialogue(&invalid).await,
        Err(TutorError::InvalidRequest)
    ));
    assert_eq!(mock.calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        service.status().remaining_budget_micro_usd,
        DEFAULT_SESSION_BUDGET_MICRO_USD
    );
    task.abort();
    let (service, mock, task) = self::service(
        vec![(401, json!({"error":"PRIVATE-DIALOGUE-TEXT"}))],
        Duration::ZERO,
        Duration::from_secs(2),
        DEFAULT_SESSION_BUDGET_MICRO_USD,
    )
    .await;
    let error = service.dialogue(&dialogue_request()).await.unwrap_err();
    assert!(matches!(error, TutorError::Authentication));
    assert!(!error.to_string().contains("PRIVATE"));
    assert_eq!(mock.calls.load(Ordering::SeqCst), 1);
    task.abort();
}
