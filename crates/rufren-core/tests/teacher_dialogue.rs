use rufren_core::*;
use rufren_tutor_contract::*;
use uuid::Uuid;

fn request() -> TeacherDialogueRequest {
    let id = Uuid::new_v4();
    TeacherDialogueRequest {
        request_id: id,
        thread_id: id,
        parent_reply_id: None,
        config: DialogueConfig {
            intent: DialogueIntent::Clarification,
            topic: "A public greeting".into(),
            teacher_role: None,
            learner_role: None,
        },
        action: DialogueAction::Ask,
        correction: DialogueCorrection::OneTargeted,
        anchor: None,
        ancestors: vec![],
        omitted_parent_id: None,
        omitted_count: 0,
        text: "What is the difference between привет and здравствуйте?".into(),
        confirmed_audio: None,
    }
}
fn response(request: TeacherDialogueRequest, source: &str) -> TeacherDialogueResponse {
    TeacherDialogueResponse {
        request,
        content: TeacherDialogueContent {
            explanation_english: Some("Use the informal greeting with someone you know.".into()),
            utterance: Some(TutorExample {
                russian: "Привет!".into(),
                english: "Hi!".into(),
                note: "Informal.".into(),
            }),
            next_question: None,
            correction: None,
            scene_state: DialogueSceneState::Open,
        },
        provenance: TutorProvenance {
            provider: "public-mock".into(),
            model: "public-model".into(),
            response_id: source.into(),
            prompt_version: "dialogue-v1".into(),
            review_status: ReviewStatus::AiGeneratedTeaching,
            scope: "Language content; no acoustic grade.".into(),
        },
        usage: TutorUsage {
            accounting: UsageAccounting::SubscriptionQuota,
            input_tokens: 10,
            output_tokens: 20,
            estimated_micro_usd: 0,
            reserved_micro_usd: 0,
        },
    }
}
fn user(r: &TeacherDialogueRequest) -> Event {
    Event::new(
        Uuid::new_v4(),
        1,
        Fact::TeacherDialogueUserRecorded(r.clone()),
    )
    .unwrap()
}
fn reply(r: &TeacherDialogueResponse) -> Event {
    Event::new(
        Uuid::new_v4(),
        2,
        Fact::TeacherDialogueReplyRecorded(r.clone()),
    )
    .unwrap()
}
fn next(r: &TeacherDialogueResponse, text: &str) -> TeacherDialogueRequest {
    let mut result = r.request.clone();
    result.request_id = Uuid::new_v4();
    result.parent_reply_id = Some(teacher_dialogue_reply_id(r));
    result.action = DialogueAction::Ask;
    result.text = text.into();
    result.confirmed_audio = None;
    result.ancestors.push(DialogueAncestor {
        message_id: r.request.request_id,
        role: DialogueRole::User,
        text: teacher_dialogue_user_body(&r.request),
    });
    result.ancestors.push(DialogueAncestor {
        message_id: teacher_dialogue_reply_id(r),
        role: DialogueRole::Assistant,
        text: teacher_dialogue_assistant_body(&r.content),
    });
    result
}
fn archived() -> (Event, TeacherResponse) {
    let r:TeacherResponse=serde_json::from_value(serde_json::json!({
        "request":{"mode":"teach_lesson","input":{"request_id":"public-lesson-request","lesson_id":"ru.lesson.greetings","title":"Greetings","objective":"Return a greeting.","material":"Привет! — Hi!","learner_context":null}},
        "content":{"explanation":"An informal greeting.","examples":[{"russian":"Привет!","english":"Hi!","note":"To a friend."}],"next_step":"Try replying.","caveats":[],"feedback":null,"practice":[{"prompt":"Greet a friend.","expected_response":"Привет!","teaching_note":"An informal greeting.","reference_russian":"Привет!","reference_english":"Hi!","skill":"writing"}]},
        "provenance":{"provider":"public-mock","model":"public-model","review_status":"ai_generated_teaching","scope":"Language content.","prompt_version":"teacher-v5","response_id":"public-lesson-response"},
        "usage":{"input_tokens":10,"output_tokens":20,"estimated_micro_usd":0,"reserved_micro_usd":0}
    })).unwrap();
    let event = Event::new(
        Uuid::new_v4(),
        0,
        Fact::TeacherDocumentRecorded(TeacherDocument {
            format_version: 1,
            document_id: teacher_document_id(&r),
            response: r.clone(),
            receipt: TeacherDocumentReceipt::RecoveredLocalCache,
            context_source_fact_ids: None,
        }),
    )
    .unwrap();
    (event, r)
}

#[test]
fn dialogue_saved_question_and_reply_are_not_practice_or_memory_evidence() {
    let question = request();
    let answer = response(question.clone(), "public-reply-one");
    let mut l = Library::default();
    l.merge(&[user(&question)]).unwrap();
    assert_eq!(l.dialogue_threads()[0].leaf_ids, vec![question.request_id]);
    l.merge(&[reply(&answer)]).unwrap();
    let id = teacher_dialogue_reply_id(&answer);
    assert_eq!(l.dialogue_user(question.request_id), Some(&question));
    assert_eq!(l.dialogue_reply(id), Some(&answer));
    let path = l.dialogue_path(id).unwrap();
    assert!(path.is_complete());
    assert_eq!(path.messages.len(), 2);
    assert_eq!(path.messages[0].body(), question.text);
    assert_eq!(
        path.messages[1].body(),
        teacher_dialogue_assistant_body(&answer.content)
    );
    assert_eq!(l.attempts().count(), 0);
    assert!(l.heads().is_empty());
    assert!(l.learner_snapshot(100).features.is_empty());
    assert!(SpacedReview.queue(&l, 100, &Skill::ALL).is_empty());
}

#[test]
fn dialogue_concurrent_branches_and_equal_logical_receipts_converge_without_clocks() {
    let root = request();
    let a = response(root.clone(), "public-reply-A");
    let b = response(root.clone(), "public-reply-B");
    let ca = next(&a, "Why is that informal?");
    let cb = next(&b, "Can we try a greeting?");
    let mut events = vec![user(&root), reply(&a), reply(&b), user(&ca), user(&cb)];
    let mut duplicate = user(&root);
    duplicate.recorded_at_ms = 999999;
    events.push(duplicate);
    let mut x = Library::default();
    x.merge(&events).unwrap();
    events.reverse();
    let mut y = Library::default();
    y.merge(&events).unwrap();
    assert_eq!(x, y);
    assert_eq!(x.dialogue_replies(root.request_id).len(), 2);
    assert_eq!(x.dialogue_threads()[0].user_messages, 3);
    assert_eq!(x.dialogue_threads()[0].leaf_ids.len(), 2);
    assert_eq!(
        x.dialogue_children(teacher_dialogue_reply_id(&a)),
        vec![&ca]
    );
    assert_eq!(x.dialogue_path(cb.request_id).unwrap().messages.len(), 3);
}

#[test]
fn dialogue_changed_logical_request_or_provider_response_rejects_whole_batch() {
    let root = request();
    let answer = response(root.clone(), "public-reply-reused");
    let mut l = Library::default();
    l.merge(&[user(&root), reply(&answer)]).unwrap();
    let before = l.clone();
    let mut changed = root.clone();
    changed.text = "A different question.".into();
    assert!(
        l.merge(&[starter(Uuid::new_v4(), 1), user(&changed)])
            .is_err()
    );
    assert_eq!(l, before);
    let mut changed = answer.clone();
    changed.content.explanation_english = Some("A different answer.".into());
    assert!(l.merge(&[reply(&changed)]).is_err());
    assert_eq!(l, before);
}

#[test]
fn dialogue_missing_parent_and_source_are_explicit_then_validate_on_arrival() {
    let root = request();
    let answer = response(root.clone(), "public-dangling-reply");
    let child = next(&answer, "Could you explain more?");
    let mut l = Library::default();
    l.merge(&[user(&child)]).unwrap();
    let path = l.dialogue_path(child.request_id).unwrap();
    assert!(!path.is_complete());
    assert_eq!(
        path.missing_parent,
        Some(teacher_dialogue_reply_id(&answer))
    );
    l.merge(&[reply(&answer)]).unwrap();
    assert_eq!(
        l.dialogue_path(child.request_id).unwrap().missing_parent,
        Some(root.request_id)
    );
    l.merge(&[user(&root)]).unwrap();
    assert!(l.dialogue_path(child.request_id).unwrap().is_complete());
    let (archive, r) = archived();
    let mut source = Library::default();
    source.merge(&[archive.clone()]).unwrap();
    let anchor = teacher_dialogue_anchor(
        &source,
        teacher_document_id(&r),
        TeacherDialogueFocus::Practice { index: 0 },
    )
    .unwrap();
    let mut anchored = request();
    anchored.anchor = Some(anchor);
    let mut partial = Library::default();
    partial.merge(&[user(&anchored)]).unwrap();
    assert_eq!(
        partial
            .dialogue_path(anchored.request_id)
            .unwrap()
            .missing_documents,
        vec![teacher_document_id(&r)]
    );
    partial.merge(&[archive]).unwrap();
    assert!(
        partial
            .dialogue_path(anchored.request_id)
            .unwrap()
            .is_complete()
    );
}

#[test]
fn dialogue_wrong_frozen_ancestors_config_echo_and_source_cannot_attach_to_known_history() {
    let root = request();
    let answer = response(root.clone(), "public-context-reply");
    let mut l = Library::default();
    l.merge(&[user(&root), reply(&answer)]).unwrap();
    let before = l.clone();
    let child = next(&answer, "Why?");
    let mut bad = child.clone();
    bad.ancestors[1].text = "An invented teacher answer.".into();
    assert!(l.merge(&[user(&bad)]).is_err());
    let mut bad = child.clone();
    bad.config.topic = "An unrelated topic".into();
    assert!(l.merge(&[user(&bad)]).is_err());
    let mut bad = response(root.clone(), "public-wrong-echo");
    bad.request.text = "Different saved input".into();
    assert!(l.merge(&[reply(&bad)]).is_err());
    assert_eq!(l, before);
    let (archive, r) = archived();
    l.merge(&[archive]).unwrap();
    let mut anchored = request();
    let mut anchor = teacher_dialogue_anchor(
        &l,
        teacher_document_id(&r),
        TeacherDialogueFocus::Explanation,
    )
    .unwrap();
    anchor.snapshot.push_str(" invented");
    anchored.anchor = Some(anchor);
    assert!(l.merge(&[user(&anchored)]).is_err());
}

#[test]
fn dialogue_unknown_bad_snapshot_becomes_rejected_when_source_arrives() {
    let (archive, r) = archived();
    let mut sources = Library::default();
    sources.merge(&[archive.clone()]).unwrap();
    let mut root = request();
    let mut a = teacher_dialogue_anchor(
        &sources,
        teacher_document_id(&r),
        TeacherDialogueFocus::Explanation,
    )
    .unwrap();
    a.snapshot = "Different private source".into();
    root.anchor = Some(a);
    let mut l = Library::default();
    l.merge(&[user(&root)]).unwrap();
    let before = l.clone();
    assert!(l.merge(&[archive]).is_err());
    assert_eq!(l, before);
}

#[test]
fn dialogue_whole_pair_suffix_omission_checks_exact_boundary_count_and_body() {
    let root = request();
    let a = response(root.clone(), "public-root-response");
    let second = next(&a, "Why?");
    let b = response(second.clone(), "public-second-response");
    let mut third = next(&b, "Another explanation, please.");
    third.ancestors.drain(..2);
    third.omitted_count = 2;
    third.omitted_parent_id = Some(teacher_dialogue_reply_id(&a));
    let mut l = Library::default();
    l.merge(&[
        user(&root),
        reply(&a),
        user(&second),
        reply(&b),
        user(&third),
    ])
    .unwrap();
    let mut bad = third.clone();
    bad.request_id = Uuid::new_v4();
    bad.omitted_count = 4;
    assert!(l.merge(&[user(&bad)]).is_err());
    let mut bad = third;
    bad.request_id = Uuid::new_v4();
    bad.omitted_parent_id = Some(Uuid::new_v4());
    assert!(l.merge(&[user(&bad)]).is_err());
}

#[test]
fn dialogue_start_scene_has_an_action_marker_and_never_a_fabricated_learner_utterance() {
    let mut root = request();
    root.config.intent = DialogueIntent::Roleplay;
    root.config.topic = "Buying tea in a cafe".into();
    root.action = DialogueAction::StartScene;
    root.text.clear();
    root.validate().unwrap();
    assert_eq!(
        teacher_dialogue_user_body(&root),
        "[Learner action: start the scene]"
    );
    let answer = response(root.clone(), "public-roleplay-response");
    let next = next(&answer, "Чай, пожалуйста.");
    assert_eq!(next.ancestors[0].text, "[Learner action: start the scene]");
    let mut l = Library::default();
    l.merge(&[user(&root), reply(&answer), user(&next)])
        .unwrap();
    assert!(l.dialogue_path(next.request_id).unwrap().is_complete());
}

#[test]
fn dialogue_v5_gates_new_fact_but_preserves_actual_legacy_v4_archive_bytes() {
    let (mut archive, _) = archived();
    archive.version = 4;
    let raw = serde_json::to_string(&archive).unwrap();
    let restored: Event = serde_json::from_str(&raw).unwrap();
    restored.validate().unwrap();
    assert_eq!(serde_json::to_string(&restored).unwrap(), raw);
    let root = request();
    let mut event = user(&root);
    assert_eq!(event.version, 6);
    for version in 1..=4 {
        event.version = version;
        assert!(matches!(event.validate(),Err(Error::Version(v)) if v==version));
    }
    event.version = 5;
    let mut l = Library::default();
    l.merge(&[restored.clone(), event]).unwrap();
    assert_eq!(l.events[&restored.id], restored);
    assert_eq!((EVENT_VERSION, SYNC_VERSION), (6, 6));
}

#[test]
fn dialogue_partial_cycle_cannot_be_completed_by_a_later_sync_page() {
    let thread = Uuid::new_v4();
    let mut a = request();
    let mut b = request();
    a.thread_id = thread;
    b.thread_id = thread;
    let aid = teacher_dialogue_reply_id(&response(a.clone(), "public-cycle-A"));
    let bid = teacher_dialogue_reply_id(&response(b.clone(), "public-cycle-B"));
    a.parent_reply_id = Some(bid);
    b.parent_reply_id = Some(aid);
    a.omitted_parent_id = Some(aid);
    b.omitted_parent_id = Some(bid);
    a.omitted_count = 2;
    b.omitted_count = 2;
    let ar = response(a.clone(), "public-cycle-A");
    let br = response(b.clone(), "public-cycle-B");
    a.ancestors = vec![
        DialogueAncestor {
            message_id: b.request_id,
            role: DialogueRole::User,
            text: teacher_dialogue_user_body(&b),
        },
        DialogueAncestor {
            message_id: bid,
            role: DialogueRole::Assistant,
            text: teacher_dialogue_assistant_body(&br.content),
        },
    ];
    b.ancestors = vec![
        DialogueAncestor {
            message_id: a.request_id,
            role: DialogueRole::User,
            text: teacher_dialogue_user_body(&a),
        },
        DialogueAncestor {
            message_id: aid,
            role: DialogueRole::Assistant,
            text: teacher_dialogue_assistant_body(&ar.content),
        },
    ];
    let ar = response(a.clone(), "public-cycle-A");
    let br = response(b.clone(), "public-cycle-B");
    let mut l = Library::default();
    l.merge(&[user(&a)]).unwrap();
    let before = l.clone();
    assert!(l.merge(&[reply(&ar), user(&b), reply(&br)]).is_err());
    assert_eq!(l, before);
}

#[test]
fn dialogue_anchor_is_utf8_bounded_and_cannot_switch_inside_a_thread() {
    let (mut event, r) = archived();
    let Fact::TeacherDocumentRecorded(d) = &mut event.fact else {
        unreachable!()
    };
    d.response.content.explanation = "Я".repeat(5000);
    event.validate().unwrap();
    let mut l = Library::default();
    l.merge(&[event]).unwrap();
    let id = teacher_document_id(&r);
    let anchor = teacher_dialogue_anchor(&l, id, TeacherDialogueFocus::Explanation).unwrap();
    assert!(anchor.snapshot.len() <= 8192);
    assert!(
        anchor
            .snapshot
            .ends_with("[Source truncated at this UTF-8 boundary]")
    );
    let mut root = request();
    root.anchor = Some(anchor);
    let answer = response(root.clone(), "public-anchored-reply");
    l.merge(&[user(&root), reply(&answer)]).unwrap();
    let mut child = next(&answer, "Why?");
    child.anchor =
        Some(teacher_dialogue_anchor(&l, id, TeacherDialogueFocus::Practice { index: 0 }).unwrap());
    assert!(l.merge(&[user(&child)]).is_err());
}
