use rufren_content::{
    AnswerSpec, LessonPracticeAction, SoundUtteranceKind, load_embedded, sound_contrast,
    sound_contrasts,
};
use std::collections::BTreeSet;

#[test]
fn sound_contrast_exact_member_and_utterance_identities_remain_separate() {
    let pack = load_embedded().unwrap();
    pack.validate_sound_contrasts().unwrap();
    assert_eq!(
        pack.revision, "2026-11-05.4",
        "Sound Focus does not migrate the phrase pack"
    );
    let contrasts = sound_contrasts();
    let mut ids = BTreeSet::new();
    for definition in &contrasts {
        assert!(ids.insert(definition.id.as_str()));
        assert_eq!(definition.version, "1");
        assert_eq!(definition.members.len(), 2);
        assert_eq!(definition.identify_task.version, "1");
        assert_eq!(definition.produce_task.version, "1");
        assert!(ids.insert(&definition.identify_task.id));
        assert!(ids.insert(&definition.produce_task.id));
        assert!(
            definition
                .produce_task
                .instruction
                .starts_with("Select the meaning you intend")
        );
        for member in &definition.members {
            assert!(ids.insert(&member.id));
            assert!(ids.insert(&member.utterance.id));
            assert_eq!(member.version, "1");
            assert_eq!(member.utterance.version, "1");
            assert_eq!(member.utterance.related_phrase_revision, "2026-11-05.3");
            assert!(
                member
                    .option_label
                    .chars()
                    .all(|c| !('А'..='я').contains(&c) && c != 'Ё' && c != 'ё')
            );
        }
        let serialized = serde_json::to_vec(definition).unwrap();
        assert_eq!(
            serde_json::from_slice::<rufren_content::SoundContrastDefinition>(&serialized).unwrap(),
            *definition
        );
    }
    let hard = &contrasts[0];
    assert_eq!(hard.id, "ru.sound.contrast.l-hard-soft");
    assert_eq!(hard.identify_task.id, "ru.task.sound.l-hard-soft.identify");
    assert_eq!(hard.produce_task.id, "ru.task.sound.l-hard-soft.produce");
    for (member, id, label, text, phrase) in [
        (
            &hard.members[0],
            "onion",
            "An onion",
            "Лук.",
            "ru.phrase.naming.07",
        ),
        (
            &hard.members[1],
            "hatch",
            "A hatch",
            "Люк.",
            "ru.phrase.naming.08",
        ),
    ] {
        assert_eq!(member.id, format!("ru.sound.member.{id}"));
        assert_eq!(
            member.utterance.id,
            format!("ru.sound.utterance.{id}.isolated")
        );
        assert_eq!(member.option_label, label);
        assert_eq!(member.utterance.text, text);
        assert_eq!(member.utterance.kind, SoundUtteranceKind::IsolatedWord);
        assert_eq!(member.utterance.related_phrase_id, phrase);
        assert_ne!(
            member.utterance.text,
            pack.phrase(phrase).unwrap().russian,
            "a lexical carrier is not the isolated audio utterance"
        );
    }
    let stress = &contrasts[1];
    assert_eq!(stress.id, "ru.sound.contrast.lexical-stress");
    assert_eq!(
        stress.identify_task.id,
        "ru.task.sound.lexical-stress.identify"
    );
    assert_eq!(
        stress.produce_task.id,
        "ru.task.sound.lexical-stress.produce"
    );
    assert_eq!(stress.members[0].utterance.text, "Это за́мок.");
    assert_eq!(stress.members[1].utterance.text, "Это замо́к.");
    assert_eq!(stress.members[0].option_label, "A castle");
    assert_eq!(stress.members[1].option_label, "A lock");
    assert_eq!(stress.members[0].lexeme, stress.members[1].lexeme);
    assert_ne!(
        stress.members[0].supported_lexeme,
        stress.members[1].supported_lexeme
    );
    assert_ne!(
        stress.members[0].utterance.id,
        stress.members[1].utterance.id
    );
    assert!(
        sound_contrast(&stress.id, "unknown").is_none(),
        "never silently substitute another version"
    );
}

#[test]
fn sound_contrast_active_projection_replaces_new_actions_without_rewriting_history() {
    let pack = load_embedded().unwrap();
    let original = serde_json::to_vec(&pack).unwrap();
    let naming = pack.lesson("ru.lesson.naming").unwrap();
    let actions = pack.lesson_practice_actions(naming).unwrap();
    assert_eq!(actions.len(), 10);
    assert!(
        matches!(&actions[0], LessonPracticeAction::SoundFocus(definition) if definition.id == "ru.sound.contrast.l-hard-soft")
    );
    assert!(
        matches!(&actions[1], LessonPracticeAction::SoundFocus(definition) if definition.id == "ru.sound.contrast.lexical-stress")
    );
    let active_ids: BTreeSet<_> = actions
        .iter()
        .filter_map(|action| match action {
            LessonPracticeAction::Exercise(exercise) => Some(exercise.id.as_str()),
            _ => None,
        })
        .collect();
    for id in [
        "ru.exercise.naming.hear-soft",
        "ru.exercise.naming.hear-stress",
        "ru.exercise.naming.say-contrast",
    ] {
        assert!(!active_ids.contains(id));
        assert!(naming.exercise_ids.iter().any(|old| old == id));
        assert!(pack.exercise(id).is_some());
        assert!(
            pack.lesson_exercises(naming)
                .any(|exercise| exercise.id == id)
        );
    }
    assert!(active_ids.contains("ru.exercise.naming.mark-stress"));
    for (id, expected) in [
        ("ru.exercise.naming.hear-soft", "hatch"),
        ("ru.exercise.naming.hear-stress", "lock"),
    ] {
        assert!(
            matches!(&pack.exercise(id).unwrap().answer, AnswerSpec::Choice { correct_option } if correct_option == expected)
        );
    }
    for lesson in pack.lessons.iter().filter(|lesson| lesson.id != naming.id) {
        let actions = pack.lesson_practice_actions(lesson).unwrap();
        assert_eq!(actions.len(), lesson.exercise_ids.len());
        assert!(
            actions
                .iter()
                .all(|action| matches!(action, LessonPracticeAction::Exercise(_)))
        );
    }
    assert_eq!(serde_json::to_vec(&pack).unwrap(), original);
}

#[test]
fn sound_contrast_unavailable_or_changed_lexical_bindings_fail_closed() {
    let original = load_embedded().unwrap();
    for mode in 0..4 {
        let mut bad = original.clone();
        match mode {
            0 => {
                bad.phrases
                    .iter_mut()
                    .find(|phrase| phrase.id == "ru.phrase.naming.09")
                    .unwrap()
                    .phrase_revision = Some("2026-11-05.4".into())
            }
            1 => {
                bad.phrases
                    .iter_mut()
                    .find(|phrase| phrase.id == "ru.phrase.naming.09")
                    .unwrap()
                    .stressed = "Э́то замо́к.".into()
            }
            2 => bad
                .sources
                .retain(|source| source.id != "ru.source.softness"),
            _ => bad
                .lessons
                .iter_mut()
                .find(|lesson| lesson.id == "ru.lesson.naming")
                .unwrap()
                .phrase_ids
                .retain(|id| id != "ru.phrase.naming.07"),
        }
        assert!(bad.validate_sound_contrasts().is_err());
        assert!(
            bad.lesson_practice_actions(bad.lesson("ru.lesson.naming").unwrap())
                .is_err()
        );
    }
    let mut stale = original.lesson("ru.lesson.naming").unwrap().clone();
    stale.goal = "A stale or foreign definition".into();
    assert!(original.lesson_practice_actions(&stale).is_err());
}
