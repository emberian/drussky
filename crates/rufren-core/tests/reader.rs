use rufren_core::*;
use rufren_tutor_contract::*;
use std::collections::BTreeSet;
use uuid::Uuid;

fn definition(parts: &[&str]) -> ReaderDefinition {
    let original = parts.concat();
    let mut start = 0;
    let chunks = parts
        .iter()
        .enumerate()
        .map(|(i, text)| {
            let end = start + text.len();
            let chunk = ReaderChunk {
                id: format!("chunk-{}", i + 1),
                span: TextSpan { start, end },
                text: (*text).into(),
                stress: None,
                translation: Some(format!("AI translation {}", i + 1)),
                glosses: vec![],
                grammar_notes: vec![],
                prompts: vec![],
            };
            start = end;
            chunk
        })
        .collect();
    ReaderDefinition {
        id: "ru.reader.public-test".into(),
        version: "1".into(),
        title: "Public test".into(),
        original_sha256: reader_text_sha256(&original),
        original,
        source: ReaderSource {
            kind: ReaderSourceKind::AiAuthored,
            title: "Original public test".into(),
            author: "AI".into(),
            edition: "Test v1".into(),
            url: None,
            rights: "Original AI-authored test material".into(),
            provenance: "Synthetic public test".into(),
        },
        help_provenance: "AI-authored test support; not original authorship".into(),
        chunks,
    }
}
fn sample() -> ReaderDefinition {
    let mut d = definition(&["Тень.\n\n", "Ещё тень."]);
    d.chunks[0].stress = Some("Те\u{301}нь.\n\n".into());
    d.chunks[1].stress = Some("Ещё те\u{301}нь.".into());
    let second_start = d.chunks[1].span.start;
    d.chunks[1].glosses.push(ReaderNote {
        span: TextSpan {
            start: second_start,
            end: second_start + "Ещё".len(),
        },
        text: "Ещё".into(),
        explanation: "AI contextual gloss: another".into(),
    });
    d
}
fn request(anchor: TeacherDialogueAnchor) -> TeacherDialogueRequest {
    let id = Uuid::new_v4();
    TeacherDialogueRequest {
        request_id: id,
        thread_id: id,
        parent_reply_id: None,
        config: DialogueConfig {
            intent: DialogueIntent::Clarification,
            topic: "Meaning in this source".into(),
            teacher_role: None,
            learner_role: None,
        },
        action: DialogueAction::Ask,
        correction: DialogueCorrection::OneTargeted,
        anchor: Some(anchor),
        ancestors: vec![],
        omitted_parent_id: None,
        omitted_count: 0,
        text: "Does this mean another shadow?".into(),
        confirmed_audio: None,
    }
}
#[test]
fn reader_canonical_source_is_idempotent_and_creates_no_learning() {
    let d = sample();
    let e = reader_document_event(&d).unwrap();
    assert_eq!(e, reader_document_event(&d).unwrap());
    assert_eq!((e.version, e.recorded_at_ms), (7, 0));
    let mut l = Library::default();
    l.merge(&[e.clone(), e.clone()]).unwrap();
    assert_eq!(l.reader_documents().len(), 1);
    assert_eq!(l.attempts().count(), 0);
    assert!(l.heads().is_empty());
    assert!(l.learner_snapshot(100).features.is_empty());
    let mut changed = d.clone();
    changed.source.rights.push_str(" changed");
    assert!(l
        .merge(&[reader_document_event(&changed).unwrap()])
        .is_err());
    let mut old = e;
    old.version = 6;
    assert!(matches!(old.validate(), Err(Error::Version(6))));
}
#[test]
fn reader_utf8_partition_notes_and_exact_hash_are_validated() {
    let d = sample();
    d.validate().unwrap();
    assert_eq!(
        d.chunks.iter().map(|c| c.text.as_str()).collect::<String>(),
        d.original
    );
    let mut bad = d.clone();
    bad.chunks[1].span.start += 1;
    assert!(bad.validate().is_err());
    let mut bad = d.clone();
    bad.chunks[1].glosses[0].span.end -= 1;
    assert!(bad.validate().is_err());
    let mut bad = d.clone();
    bad.chunks[1].glosses[0].text = "еще".into();
    assert!(bad.validate().is_err());
    let mut bad = d.clone();
    bad.original.push(' ');
    assert!(bad.validate().is_err());
    let mut bad = d;
    bad.chunks[0].text = "Тень. ".into();
    assert!(bad.validate().is_err());
}
#[test]
fn reader_stress_only_adds_acute_and_source_rights_remain_distinct() {
    let d = sample();
    for altered in ["Еще те\u{301}нь.", "Ещё те\u{301}нь!", "Ещё  те\u{301}нь."]
    {
        let mut bad = d.clone();
        bad.chunks[1].stress = Some(altered.into());
        assert!(bad.validate().is_err());
    }
    let mut bad = d.clone();
    bad.source.rights.clear();
    assert!(bad.validate().is_err());
    let mut bad = d.clone();
    bad.help_provenance.clear();
    assert!(bad.validate().is_err());
    let mut authentic = d;
    authentic.source.kind = ReaderSourceKind::AuthenticExcerpt;
    assert!(authentic.validate().is_err());
    authentic.source.url = Some("https://example.org/public-edition".into());
    authentic.validate().unwrap();
    authentic.source.url = Some("javascript:alert(1)".into());
    assert!(authentic.validate().is_err());
}
#[test]
fn reader_audio_layers_are_distinct_and_keep_original_is_practiceable() {
    let d = sample();
    let doc = reader_document(&d).unwrap();
    let a = reader_reference_events(&doc, "chunk-1", ReaderTextLayer::Original).unwrap();
    let b = reader_reference_events(&doc, "chunk-1", ReaderTextLayer::StressScaffold).unwrap();
    let kept = reader_excerpt_events(&doc, d.chunks[0].span).unwrap();
    assert_ne!(a.0.id, b.0.id);
    assert_ne!(a.0.id, kept.0.id);
    assert!(is_reserved_practice_reference(&a.0));
    assert!(is_reserved_practice_reference(&b.0));
    assert!(!is_reserved_practice_reference(&kept.0));
    let mut l = Library::default();
    l.merge(&[
        reader_document_event(&d).unwrap(),
        a.0.clone(),
        a.1.clone(),
        b.0.clone(),
        b.1.clone(),
        kept.0.clone(),
        kept.1.clone(),
    ])
    .unwrap();
    assert_eq!(
        l.revision(a.0.id).unwrap().passage.russian,
        d.chunks[0].text
    );
    assert_eq!(
        l.revision(b.0.id).unwrap().passage.russian,
        d.chunks[0].stress.as_ref().unwrap().as_str()
    );
    assert_eq!(
        l.reader_excerpt_link(b.0.id).unwrap().span,
        d.chunks[0].span
    );
    assert_eq!(l.attempts().count(), 0);
    let queue = SpacedReview.queue(&l, 100, &Skill::ALL);
    assert!(!queue.is_empty());
    assert_eq!(
        queue
            .iter()
            .map(|c| c.state.key.revision)
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([kept.0.id])
    );
}
#[test]
fn reader_kept_repeated_phrase_uses_exact_offsets_and_only_whole_chunk_translation() {
    let d = definition(&["Да, да."]);
    let doc = reader_document(&d).unwrap();
    let first = reader_excerpt_events(&doc, TextSpan { start: 0, end: 4 }).unwrap();
    let second = reader_excerpt_events(&doc, TextSpan { start: 6, end: 10 }).unwrap();
    assert_ne!(first.0.id, second.0.id);
    let Fact::PassageRevised(p) = &first.0.fact else {
        panic!()
    };
    assert!(p.passage.translation.is_empty());
    let Fact::ReaderExcerptLinked(link) = &second.1.fact else {
        panic!()
    };
    assert_eq!(link.chunk_id.as_deref(), Some("chunk-1"));
    let whole = reader_excerpt_events(&doc, d.chunks[0].span).unwrap();
    let Fact::PassageRevised(p) = &whole.0.fact else {
        panic!()
    };
    assert_eq!(p.passage.translation, "AI translation 1");
    assert!(reader_excerpt_events(&doc, TextSpan { start: 1, end: 4 }).is_err());
}
#[test]
fn reader_kept_phrase_can_gain_meaning_in_an_ordinary_child_revision() {
    let d = sample();
    let doc = reader_document(&d).unwrap();
    let kept = reader_excerpt_events(&doc, TextSpan { start: 0, end: 8 }).unwrap();
    let Fact::PassageRevised(mut r) = kept.0.fact.clone() else {
        panic!()
    };
    r.parents.insert(kept.0.id);
    r.passage.translation = "A shadow.".into();
    let child = Event::new(Uuid::new_v4(), 42, Fact::PassageRevised(r)).unwrap();
    let mut l = Library::default();
    l.merge(&[
        reader_document_event(&d).unwrap(),
        kept.0.clone(),
        kept.1,
        child.clone(),
    ])
    .unwrap();
    assert_eq!(
        l.revision(child.id).unwrap().passage.translation,
        "A shadow."
    );
    assert!(l.reader_excerpt_link(kept.0.id).is_some());
}
#[test]
fn reader_partial_graph_accepts_late_dependencies_and_rejects_contradictions_atomically() {
    let d = sample();
    let doc = reader_document(&d).unwrap();
    let pair = reader_excerpt_events(&doc, d.chunks[1].span).unwrap();
    let mut l = Library::default();
    l.merge(&[pair.1.clone()]).unwrap();
    assert!(l.reader_document(doc.document_id).is_none());
    l.merge(&[pair.0.clone()]).unwrap();
    l.merge(&[reader_document_event(&d).unwrap()]).unwrap();
    let mut forged = pair.0.clone();
    let Fact::PassageRevised(r) = &mut forged.fact else {
        panic!()
    };
    r.passage.russian = "Неверный текст".into();
    let mut partial = Library::default();
    partial.merge(&[pair.1.clone()]).unwrap();
    assert!(partial.merge(&[forged]).is_err());
    assert!(partial.revision(pair.0.id).is_none());
    let mut wrong_doc = reader_document_event(&d).unwrap();
    wrong_doc.fact = pair.0.fact;
    assert!(partial.merge(&[wrong_doc]).is_err());
    let mut bad_source = pair.1;
    let Fact::ReaderExcerptLinked(link) = &mut bad_source.fact else {
        panic!()
    };
    link.text = "X".repeat(link.text.len());
    let mut delayed = Library::default();
    delayed.merge(&[bad_source]).unwrap();
    assert!(delayed
        .merge(&[reader_document_event(&d).unwrap()])
        .is_err());
    assert!(delayed.reader_document(doc.document_id).is_none());
}
#[test]
fn reader_near_64k_unicode_and_json_escape_heavy_sources_fit_declared_bounds() {
    for unit in ["Жё", "\"\\\n\t"] {
        let mut d = definition(&["stub"]);
        let text = unit.repeat((READER_MAX_ORIGINAL_BYTES - 16) / unit.len());
        let mut parts = vec![];
        let mut start = 0;
        while start < text.len() {
            let mut end = (start + 4096).min(text.len());
            while !text.is_char_boundary(end) {
                end -= 1;
            }
            parts.push(&text[start..end]);
            start = end;
        }
        let large = definition(&parts);
        d.original = large.original;
        d.original_sha256 = large.original_sha256;
        d.chunks = large.chunks;
        assert!(d.original.len() > 65_000);
        d.validate().unwrap();
        let e = reader_document_event(&d).unwrap();
        e.validate().unwrap();
        assert!(serde_json::to_vec(&e).unwrap().len() < READER_MAX_DOCUMENT_BYTES + 1024);
        let mut l = Library::default();
        l.merge(&[e]).unwrap();
        assert_eq!(l.reader_documents()[0].1.definition.original, d.original);
    }
}
#[test]
fn reader_chunk_anchor_keeps_selected_source_before_context_and_optional_ai_help() {
    let text = "Начало. ".repeat(280);
    let mut d = definition(&[&text, "Она остановила запись.", " Потом стало тихо."]);
    // Deliberately large optional help cannot displace adjacent literal context.
    d.help_provenance = "AI help ".repeat(240);
    d.chunks[1].translation = Some("T".repeat(4096));
    let mut l = Library::default();
    l.merge(&[reader_document_event(&d).unwrap()]).unwrap();
    let a = teacher_dialogue_anchor(
        &l,
        reader_document(&d).unwrap().document_id,
        TeacherDialogueFocus::ReaderChunk {
            chunk_id: "chunk-2".into(),
        },
    )
    .unwrap();
    assert!(a.snapshot.len() <= TEACHER_DIALOGUE_ANCHOR_BYTES);
    assert!(a.snapshot.contains("Она остановила запись."));
    let selected = a.snapshot.find("Selected ORIGINAL chunk").unwrap();
    let context = a.snapshot.find("Surrounding ORIGINAL context").unwrap();
    let help = a
        .snapshot
        .find("AI-authored assistance provenance")
        .unwrap();
    assert!(selected < context && context < help);
    assert!(a.snapshot.contains(" Потом стало тихо."));
}
#[test]
fn reader_whole_long_anchor_is_explicitly_partial_and_new_dialogue_requires_seven() {
    let d = definition(&[&"Ж".repeat(2000), &"Следующий абзац. ".repeat(100)]);
    let e = reader_document_event(&d).unwrap();
    let mut l = Library::default();
    l.merge(&[e.clone()]).unwrap();
    let a = teacher_dialogue_anchor(&l, e.id, TeacherDialogueFocus::ReaderWhole).unwrap();
    assert!(a.snapshot.contains("Omitted source was NOT supplied"));
    assert!(a.snapshot.contains(&d.original_sha256));
    assert!(a.snapshot.len() <= 8192);
    let r = request(a);
    r.validate().unwrap();
    let user = Event::new(
        Uuid::new_v4(),
        10,
        Fact::TeacherDialogueUserRecorded(r.clone()),
    )
    .unwrap();
    let mut old = user.clone();
    old.version = 6;
    assert!(matches!(old.validate(), Err(Error::Version(6))));
    let mut incomplete = Library::default();
    incomplete.merge(&[user]).unwrap();
    incomplete.merge(&[e.clone()]).unwrap();
    let mut forged = r;
    forged
        .anchor
        .as_mut()
        .unwrap()
        .snapshot
        .push_str(" invented");
    let fake = Event::new(
        Uuid::new_v4(),
        11,
        Fact::TeacherDialogueUserRecorded(forged),
    )
    .unwrap();
    let mut bad = Library::default();
    bad.merge(&[fake]).unwrap();
    assert!(bad.merge(&[e]).is_err());
}
#[test]
fn reader_focus_wire_addition_preserves_existing_focus_and_validates_ids() {
    assert_eq!(
        serde_json::to_string(&TeacherDialogueFocus::Explanation).unwrap(),
        "{\"kind\":\"explanation\"}"
    );
    assert_eq!(
        serde_json::to_value(TeacherDialogueFocus::ReaderChunk {
            chunk_id: "chunk-2".into()
        })
        .unwrap(),
        serde_json::json!({"kind":"reader_chunk","chunk_id":"chunk-2"})
    );
    let d = sample();
    let e = reader_document_event(&d).unwrap();
    let mut l = Library::default();
    l.merge(&[e.clone()]).unwrap();
    let a = teacher_dialogue_anchor(
        &l,
        e.id,
        TeacherDialogueFocus::ReaderChunk {
            chunk_id: "chunk-2".into(),
        },
    )
    .unwrap();
    let mut r = request(a);
    r.anchor.as_mut().unwrap().focus = TeacherDialogueFocus::ReaderChunk {
        chunk_id: "bad\nidentity".into(),
    };
    assert!(r.validate().is_err());
    let mut old = r;
    old.anchor.as_mut().unwrap().focus = TeacherDialogueFocus::Explanation;
    let mut event = Event::new(Uuid::new_v4(), 12, Fact::TeacherDialogueUserRecorded(old)).unwrap();
    event.version = 5;
    event.validate().unwrap();
}

#[test]
fn reader_dialogue_actual_request_reply_path_resolves_when_exact_reader_arrives() {
    let d = sample();
    let e = reader_document_event(&d).unwrap();
    let mut source = Library::default();
    source.merge(&[e.clone()]).unwrap();
    let anchor = teacher_dialogue_anchor(
        &source,
        e.id,
        TeacherDialogueFocus::ReaderChunk {
            chunk_id: "chunk-2".into(),
        },
    )
    .unwrap();
    let request = request(anchor);
    let answer = TeacherDialogueResponse {
        request: request.clone(),
        content: TeacherDialogueContent {
            explanation_english: Some("Yes, ещё means another here.".into()),
            utterance: None,
            next_question: None,
            correction: None,
            scene_state: DialogueSceneState::Open,
        },
        provenance: TutorProvenance {
            provider: "public-mock".into(),
            model: "public-model".into(),
            response_id: "public-reader-dialogue".into(),
            prompt_version: "dialogue-v3".into(),
            review_status: ReviewStatus::AiGeneratedTeaching,
            scope: "Language meaning only".into(),
        },
        usage: TutorUsage {
            accounting: UsageAccounting::SubscriptionQuota,
            input_tokens: 10,
            output_tokens: 20,
            estimated_micro_usd: 0,
            reserved_micro_usd: 0,
        },
    };
    let user = Event::new(
        Uuid::new_v4(),
        10,
        Fact::TeacherDialogueUserRecorded(request),
    )
    .unwrap();
    let reply = Event::new(
        Uuid::new_v4(),
        11,
        Fact::TeacherDialogueReplyRecorded(answer.clone()),
    )
    .unwrap();
    let leaf = teacher_dialogue_reply_id(&answer);
    let mut partial = Library::default();
    partial.merge(&[reply, user]).unwrap();
    let path = partial.dialogue_path(leaf).unwrap();
    assert!(!path.is_complete());
    assert_eq!(path.missing_documents, vec![e.id]);
    assert!(partial.dialogue_threads()[0].incomplete);
    partial.merge(&[e]).unwrap();
    let path = partial.dialogue_path(leaf).unwrap();
    assert!(path.is_complete());
    assert_eq!(path.messages.len(), 2);
    assert!(!partial.dialogue_threads()[0].incomplete);
    assert_eq!(partial.attempts().count(), 0);
}
