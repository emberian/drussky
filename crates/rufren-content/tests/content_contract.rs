use rufren_content::{
    AnswerMatch, AnswerPolicy, AnswerSpec, AudioStatus, ContentError, ContentPack, EditorialStatus,
    ExerciseKind, ResponseMode, load_embedded, normalize_answer,
};
use std::collections::BTreeSet;

#[test]
fn embedded_pack_has_connected_original_curriculum() {
    let pack = load_embedded().expect("embedded curriculum must validate");
    assert!(pack.lessons.len() >= 24);
    assert!(pack.phrases.len() >= 169);
    assert!(pack.exercises.len() >= 172);
    assert_eq!(pack.script.len(), 33);
    assert_eq!(pack.editorial_status, EditorialStatus::GeneratedDraft);
    assert!(
        pack.phrases
            .iter()
            .all(|p| p.editorial_status == EditorialStatus::GeneratedDraft)
    );
    assert!(
        pack.phrases
            .iter()
            .all(|p| p.audio == AudioStatus::PendingGeneratedSpeech)
    );
    assert!(pack.phrases.iter().any(|p| p.russian.contains('ё')));
    assert!(pack.lessons.iter().any(|l| l.id == "ru.lesson.logic"));
    assert!(pack.lessons.iter().any(|l| l.id == "ru.lesson.reasons"));
    assert!(pack.lessons.iter().any(|l| l.id == "ru.lesson.reader"));
    for lesson in &pack.lessons {
        assert_eq!(pack.lesson_phrases(lesson).count(), lesson.phrase_ids.len());
        assert_eq!(
            pack.lesson_exercises(lesson).count(),
            lesson.exercise_ids.len()
        );
    }
    let mut encountered = BTreeSet::new();
    let mut had_branch = false;
    while encountered.len() < pack.lessons.len() {
        let available: Vec<_> = pack.available_lessons(&encountered).collect();
        assert!(!available.is_empty(), "all lessons must be reachable");
        had_branch |= available.len() > 1;
        for lesson in &available {
            assert!(
                lesson
                    .prerequisites
                    .iter()
                    .all(|id| encountered.contains(id))
            );
        }
        let next: Vec<_> = available.iter().map(|lesson| lesson.id.clone()).collect();
        encountered.extend(next);
    }
    assert!(
        had_branch,
        "everyday encounters should branch before advanced previews"
    );
    assert_eq!(pack.available_lessons(&encountered).count(), 0);
}

#[test]
fn answer_matching_preserves_russian_distinctions() {
    let ordinary = AnswerPolicy::default();
    assert_eq!(
        normalize_answer("  Во́ду,\n пожалуйста!  ", ordinary),
        "воду, пожалуйста"
    );
    assert_eq!(
        normalize_answer("идёт", ordinary),
        normalize_answer("иде\u{308}т", ordinary)
    );
    assert_ne!(
        normalize_answer("идёт", ordinary),
        normalize_answer("идет", ordinary)
    );
    assert_ne!(
        normalize_answer("все", ordinary),
        normalize_answer("всё", ordinary)
    );
    assert_ne!(
        normalize_answer("и", ordinary),
        normalize_answer("й", ordinary)
    );
    assert_ne!(
        normalize_answer("воду", ordinary),
        normalize_answer("воды", ordinary)
    );
    assert_ne!(
        normalize_answer("Она здесь", ordinary),
        normalize_answer("Она не здесь", ordinary)
    );
    // Cyrillic and Latin homoglyphs do not become interchangeable.
    assert_ne!(
        normalize_answer("карта", ordinary),
        normalize_answer("кapтa", ordinary)
    );
    let stress = AnswerPolicy {
        require_stress: true,
        ..ordinary
    };
    assert_ne!(
        normalize_answer("за́мок", stress),
        normalize_answer("замо́к", stress)
    );
    assert_ne!(
        normalize_answer("за́мок", stress),
        normalize_answer("замок", stress)
    );
    let strict = AnswerPolicy {
        ignore_terminal_punctuation: false,
        ..ordinary
    };
    assert_ne!(
        normalize_answer("Это робот?", strict),
        normalize_answer("Это робот.", strict)
    );
}

#[test]
fn exercise_judgments_have_an_explicit_scope() {
    let pack = load_embedded().unwrap();
    let writing = pack.exercise("ru.exercise.repair.write").unwrap();
    assert_eq!(
        writing.check_answer("Пожалуйста, повторите!"),
        AnswerMatch::Accepted
    );
    assert_eq!(
        writing.check_answer("Можно ещё раз?"),
        AnswerMatch::NotMatched
    );
    let stress = pack.exercise("ru.exercise.naming.mark-stress").unwrap();
    assert_eq!(stress.check_answer("за́мок"), AnswerMatch::Accepted);
    assert_eq!(stress.check_answer("замо́к"), AnswerMatch::NotMatched);
    assert_eq!(stress.check_answer("замок"), AnswerMatch::NotMatched);
    let speech = pack.exercise("ru.exercise.repair.say").unwrap();
    assert_eq!(
        speech.check_answer("Повторите, пожалуйста."),
        AnswerMatch::NeedsReview
    );
    let composition = pack.exercise("ru.exercise.repair.compose").unwrap();
    assert_eq!(
        composition.check_answer("Я не понимаю."),
        AnswerMatch::NeedsReview
    );
    let contrast = pack.exercise("ru.exercise.roles.contrast").unwrap();
    assert_eq!(contrast.check_answer("supported"), AnswerMatch::Accepted);
    assert_eq!(
        contrast.check_answer("Вера видит Аню."),
        AnswerMatch::NotMatched
    );
    assert_eq!(contrast.response_mode, ResponseMode::Choice);
}

#[test]
fn listening_cannot_silently_use_written_stimuli() {
    let pack = load_embedded().unwrap();
    for ex in &pack.exercises {
        if matches!(
            ex.kind,
            ExerciseKind::ListeningComprehension | ExerciseKind::Dictation
        ) {
            assert!(ex.requires_audio);
            assert!(ex.needs_audio(&pack));
            assert!(!ex.stimulus_phrase_ids.is_empty());
        } else {
            assert!(!ex.needs_audio(&pack));
        }
    }
}

#[test]
fn validator_rejects_dangling_links_and_cycles() {
    let original = load_embedded().unwrap();
    let mut bad = original.clone();
    bad.lessons[0].phrase_ids.push("ru.phrase.missing".into());
    assert!(matches!(bad.validate(), Err(ContentError::Invalid(_))));
    let mut bad = original.clone();
    bad.targets[0].prerequisites = vec![bad.targets[1].id.clone()];
    bad.targets[1].prerequisites = vec![bad.targets[0].id.clone()];
    assert!(matches!(bad.validate(), Err(ContentError::Invalid(_))));
    let mut bad = original.clone();
    bad.lessons[0].prerequisites = vec![bad.lessons[1].id.clone()];
    assert!(bad.validate().is_err());
    let mut bad = original.clone();
    let first = bad.phrases[0].clone();
    bad.phrases.push(first);
    assert!(bad.validate().is_err());
    let mut bad = original;
    bad.exercises[0].answer = AnswerSpec::Choice {
        correct_option: "nonexistent".into(),
    };
    assert!(bad.validate().is_err());
}

#[test]
fn validator_rejects_false_stress_spelling_and_versions() {
    let original = load_embedded().unwrap();
    let mut bad = original.clone();
    let p = bad
        .phrases
        .iter_mut()
        .find(|p| p.russian.contains('ё'))
        .unwrap();
    p.stressed = p.stressed.replace('ё', "е");
    assert!(
        bad.validate().is_err(),
        "stress support cannot change ё to е"
    );
    let mut bad = original.clone();
    bad.schema_version = 999;
    assert!(matches!(
        bad.validate(),
        Err(ContentError::UnsupportedSchema(999))
    ));
    let mut value = serde_json::to_value(original).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .insert("invented_field".into(), true.into());
    assert!(matches!(
        ContentPack::from_json(&value.to_string()),
        Err(ContentError::Json(_))
    ));
}

#[test]
fn references_are_hidden_anchors_and_listening_has_no_script_or_position_recipe() {
    let pack = load_embedded().unwrap();
    let mut first = 0;
    let mut later = 0;
    for ex in &pack.exercises {
        let lesson = pack.lesson(&ex.lesson_id).unwrap();
        assert!(lesson.phrase_ids.contains(&ex.reference_phrase_id));
        assert!(pack.phrase(&ex.reference_phrase_id).is_some());
        if matches!(
            ex.kind,
            ExerciseKind::WrittenRetrieval
                | ExerciseKind::SpokenRetrieval
                | ExerciseKind::Composition
        ) {
            assert!(
                ex.stimulus_phrase_ids.is_empty(),
                "production must not expose model: {}",
                ex.id
            );
        }
        if ex.kind == ExerciseKind::ListeningComprehension {
            assert!(
                ex.options.iter().all(|o| !o
                    .text
                    .chars()
                    .any(|c| ('А'..='я').contains(&c) || c == 'ё' || c == 'Ё')),
                "English meaning choices must not leak via script: {}",
                ex.id
            );
            let AnswerSpec::Choice { correct_option } = &ex.answer else {
                panic!("listening choice expected")
            };
            let index = ex
                .options
                .iter()
                .position(|o| &o.id == correct_option)
                .unwrap();
            if index == 0 {
                first += 1;
            } else {
                later += 1;
            }
            assert_eq!(ex.stimulus_phrase_ids, vec![ex.reference_phrase_id.clone()]);
        }
    }
    assert_eq!(first, later, "do not teach a fixed answer position");
    let mut bad = pack;
    bad.exercises[0].reference_phrase_id = "ru.phrase.naming.01".into();
    assert!(
        bad.validate().is_err(),
        "an anchor must belong to the exercise's lesson"
    );
}

#[test]
fn revision_lineage_is_explicit_unique_and_bounded() {
    let original = load_embedded().unwrap();
    assert_eq!(original.revision, "2026-11-05.4");
    assert_eq!(
        original.previous_revisions,
        vec!["2026-11-05.1", "2026-11-05.2", "2026-11-05.3"]
    );
    for lineage in [
        vec![original.revision.clone()],
        vec![" ".into()],
        vec!["2026-11-05.1".into(), "2026-11-05.1".into()],
        vec!["x".repeat(129)],
        (0..65).map(|i| format!("previous-{i}")).collect(),
    ] {
        let mut bad = original.clone();
        bad.previous_revisions = lineage;
        assert!(bad.validate().is_err());
    }
}

#[test]
fn ai_teacher_is_default_and_legacy_conversation_key_remains_readable() {
    let original = load_embedded().unwrap();
    assert!(
        original
            .phrases
            .iter()
            .all(|p| p.audio == AudioStatus::PendingGeneratedSpeech)
    );
    let mut value = serde_json::to_value(&original).unwrap();
    for lesson in value["lessons"].as_array_mut().unwrap() {
        let fields = lesson.as_object_mut().unwrap();
        let invitation = fields
            .remove("conversation_invitation")
            .expect("canonical invitation key");
        assert!(!fields.contains_key("coach_activity"));
        fields.insert("coach_activity".into(), invitation);
    }
    let restored = ContentPack::from_json(&value.to_string()).unwrap();
    assert_eq!(
        restored.lessons[0].conversation_invitation,
        original.lessons[0].conversation_invitation
    );
    for exercise in &original.exercises {
        assert!(!exercise.instruction.to_lowercase().contains("coach"));
        assert!(!exercise.feedback.to_lowercase().contains("human judgment"));
        assert_eq!(
            exercise.needs_recording(&original),
            exercise.needs_audio(&original)
        );
    }
    for lesson in &original.lessons {
        assert!(
            !lesson
                .conversation_invitation
                .to_lowercase()
                .contains("record two")
        );
        assert!(
            !lesson
                .conversation_invitation
                .to_lowercase()
                .contains("review rubric")
        );
    }
}
