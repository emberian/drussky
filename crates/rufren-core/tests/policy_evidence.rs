use rufren_core::*;
use std::collections::BTreeSet;
use uuid::Uuid;
fn event(time: i64, fact: Fact) -> Event {
    Event::new(Uuid::new_v4(), time, fact).unwrap()
}
fn passage(russian: &str) -> Event {
    let mut e = starter(Uuid::new_v4(), 0);
    e.version = 2; // Legacy authored material has incomplete presentation history.
    if let Fact::PassageRevised(r) = &mut e.fact {
        r.passage.russian = russian.into();
    }
    e
}
fn domain() -> (Library, ContrastDomain) {
    let anchor = passage("Аня видит Веру.");
    let contrast = passage("Веру видит Аня.");
    let probe = passage("Машу видит Саша.");
    let domain = ContrastDomain {
        feature: "ru.target.roles".into(),
        skill: Skill::Reading,
        anchor_revision: anchor.id,
        contrast_revision: contrast.id,
        novel_probe_revision: probe.id,
    };
    let mut library = Library::default();
    library.merge(&[anchor, contrast, probe]).unwrap();
    (library, domain)
}
fn completed(decision: &TeachingDecision, time: i64) -> Event {
    event(
        time,
        Fact::AttemptRecorded(Attempt {
            session: decision.session,
            revision: decision.selected.revision,
            skill: decision.selected.skill,
            response: "My response".into(),
            cues: vec![],
            self_rating: SelfRating::Recalled,
            policy: decision.policy.clone(),
        }),
    )
}
fn diagnostic_context(attempt: Uuid, prompt: &str) -> Event {
    event(
        0,
        Fact::AttemptContextRecorded(AttemptContext {
            attempt,
            content_pack: None,
            content_revision: None,
            exercise_id: None,
            prompt: Some(prompt.into()),
            reference_answer: None,
            task: TaskKind::GrammarGeneralization,
            target_features: ["ru.target.roles".into()].into(),
            novel_context: true,
            delayed_probe: true,
            practice_duration_ms: None,
            effort: None,
            grammar_response: Some(GrammarResponse::CaseSensitive),
            supersedes: BTreeSet::new(),
        }),
    )
}
fn diagnostic(revision: Uuid) -> GrammarDiagnostic {
    GrammarDiagnostic {
        revision,
        skill: Skill::Reading,
        feature: "ru.target.roles".into(),
        prompt: "Who sees whom? a=Anya sees Vera; b=Vera sees Anya".into(),
        case_sensitive_response: "a".into(),
        first_noun_actor_response: "b".into(),
        source: "explicit item hypothesis, not a human gate".into(),
    }
}
fn response(revision: Uuid, time: i64, text: &str) -> Event {
    event(
        time,
        Fact::AttemptRecorded(Attempt {
            session: Uuid::new_v4(),
            revision,
            skill: Skill::Reading,
            response: text.into(),
            cues: vec![],
            self_rating: SelfRating::Recalled,
            policy: "diagnostic-practice".into(),
        }),
    )
}

#[test]
fn policy_evidence_recommendations_never_count_as_learning_or_postpone_delayed_practice() {
    let (mut library, domain) = domain();
    let plan = ContrastPolicy
        .plan(&library, &domain, 0, Uuid::new_v4())
        .unwrap();
    assert_eq!(plan.action, TeachingAction::ExplainAnchor);
    let saved = event(0, Fact::TeachingDecisionRecorded(plan.clone()));
    library.merge(&[saved.clone()]).unwrap();
    assert!(library
        .completed_practice_for_decision(saved.id, 2 * DAY_MS)
        .is_none());
    assert_eq!(
        ContrastPolicy
            .plan(&library, &domain, 2 * DAY_MS, Uuid::new_v4())
            .unwrap()
            .selected
            .revision,
        domain.anchor_revision
    );
    library.merge(&[completed(&plan, MINUTE_MS)]).unwrap();
    let due = DAY_MS + MINUTE_MS;
    for time in [MINUTE_MS + 1, DAY_MS / 2, due - 1] {
        let recommendation = ContrastPolicy
            .plan(&library, &domain, time, Uuid::new_v4())
            .unwrap();
        library
            .merge(&[event(time, Fact::TeachingDecisionRecorded(recommendation))])
            .unwrap();
    }
    let next = ContrastPolicy
        .plan(&library, &domain, due, Uuid::new_v4())
        .unwrap();
    assert_eq!(next.selected.revision, domain.novel_probe_revision);
    assert_eq!(
        next.action,
        TeachingAction::DelayedNovelProbe,
        "content definitions alone are not prior exposure"
    );
    assert!(next.predictions[0].requires_novel_context);
}

#[test]
fn policy_evidence_ai_can_teach_drafts_and_old_human_action_still_decodes_without_being_emitted() {
    let (library, domain) = domain();
    assert!(
        !library
            .revision(domain.anchor_revision)
            .unwrap()
            .passage
            .coach_reviewed
    );
    let plan = ContrastPolicy
        .plan(&library, &domain, 0, Uuid::new_v4())
        .unwrap();
    assert!(plan
        .candidates
        .iter()
        .all(|c| c.action != TeachingAction::AskCoach));
    assert!(plan.assumptions.iter().any(|s| s.contains("AI teaching")));
    let legacy: TeachingAction = serde_json::from_str("\"ask_coach\"").unwrap();
    assert_eq!(legacy, TeachingAction::AskCoach);
    assert_eq!(
        plan.predictions[0].predicted_success_bps, plan.predictions[0].comparator_success_bps,
        "do not invent an estimated advantage over the baseline"
    );
}

#[test]
fn policy_evidence_diagnosis_requires_a_separating_item_and_exact_question_not_legacy_labels() {
    let (mut library, domain) = domain();
    let item = diagnostic(domain.contrast_revision);
    let a = response(item.revision, 0, "a");
    let context = diagnostic_context(a.id, &item.prompt);
    library.merge(&[a, context]).unwrap();
    assert_eq!(
        ContrastPolicy.beliefs(&library, &domain.feature, domain.skill, 0),
        GrammarBeliefs::default()
    );
    let weights = ContrastPolicy
        .beliefs_with_diagnostics(&library, &domain.feature, domain.skill, 0, &[item.clone()])
        .unwrap();
    assert_eq!(weights.observations, 1);
    assert!(weights.case_sensitive_bps > weights.first_noun_actor_bps);
    let mut nondiagnostic = item.clone();
    nondiagnostic.first_noun_actor_response = nondiagnostic.case_sensitive_response.clone();
    assert!(ContrastPolicy
        .beliefs_with_diagnostics(&library, &domain.feature, domain.skill, 0, &[nondiagnostic])
        .is_err());
    let mut other_question = item;
    other_question.prompt = "A different question".into();
    assert_eq!(
        ContrastPolicy
            .beliefs_with_diagnostics(
                &library,
                &domain.feature,
                domain.skill,
                0,
                &[other_question]
            )
            .unwrap()
            .observations,
        0
    );
}

#[test]
fn policy_evidence_repeated_utterances_do_not_multiply_diagnostic_evidence_and_instruction_resets_epoch(
) {
    let (mut library, domain) = domain();
    let item = diagnostic(domain.contrast_revision);
    for time in 1..8 {
        let a = response(item.revision, time, "a");
        library
            .merge(&[diagnostic_context(a.id, &item.prompt), a])
            .unwrap();
    }
    assert_eq!(
        ContrastPolicy
            .beliefs_with_diagnostics(&library, &domain.feature, domain.skill, 8, &[item.clone()])
            .unwrap()
            .observations,
        1
    );
    let plan = ContrastPolicy
        .plan(&library, &domain, 10, Uuid::new_v4())
        .unwrap();
    library
        .merge(&[event(10, Fact::TeachingDecisionRecorded(plan.clone()))])
        .unwrap();
    assert_eq!(
        ContrastPolicy
            .beliefs_with_diagnostics(&library, &domain.feature, domain.skill, 11, &[item.clone()])
            .unwrap()
            .observations,
        1,
        "an uncompleted instruction recommendation cannot reset evidence"
    );
    library.merge(&[completed(&plan, 12)]).unwrap();
    assert_eq!(
        ContrastPolicy
            .beliefs_with_diagnostics(&library, &domain.feature, domain.skill, 12, &[item.clone()])
            .unwrap()
            .observations,
        0
    );
    let a = response(item.revision, 13, "b");
    library
        .merge(&[diagnostic_context(a.id, &item.prompt), a])
        .unwrap();
    let after = ContrastPolicy
        .beliefs_with_diagnostics(&library, &domain.feature, domain.skill, 13, &[item])
        .unwrap();
    assert_eq!(after.observations, 1);
    assert!(after.first_noun_actor_bps > after.case_sensitive_bps);
}

#[test]
fn policy_evidence_novelty_uses_completed_utterance_exposure_across_revisions_skills_and_larger_texts(
) {
    let (mut library, domain) = domain();
    assert_eq!(
        library.stimulus_novelty(domain.novel_probe_revision, DAY_MS, None),
        StimulusNovelty::NoPriorRecordedExposure
    );
    let dialogue = passage("Привет! Ма́шу видит САША. Аня видит Веру.");
    let a = event(
        0,
        Fact::AttemptRecorded(Attempt {
            session: Uuid::new_v4(),
            revision: dialogue.id,
            skill: Skill::Writing,
            response: "Copied earlier".into(),
            cues: vec![Cue::OriginalText],
            self_rating: SelfRating::WithHelp,
            policy: "free".into(),
        }),
    );
    library.merge(&[dialogue, a.clone()]).unwrap();
    assert_eq!(
        library.stimulus_novelty(domain.novel_probe_revision, DAY_MS, None),
        StimulusNovelty::PreviouslyExposed { event: a.id }
    );
    assert!(stimulus_contains("Он летит к звезде.", "он летит"));
    assert!(!stimulus_contains("онлайн", "он"));
    assert!(!stimulus_contains("всё", "все"));
}

#[test]
fn policy_evidence_whole_answer_judgments_are_not_copied_into_direct_feature_mastery() {
    let (mut library, domain) = domain();
    let a = response(domain.anchor_revision, 0, "a");
    let mut context = diagnostic_context(a.id, "Who sees whom?");
    if let Fact::AttemptContextRecorded(c) = &mut context.fact {
        c.target_features.insert("ru.target.lexical".into());
    }
    let judgment = event(
        0,
        Fact::CoachJudgmentRecorded(CoachJudgment {
            attempt: a.id,
            dimension: JudgmentDimension::Grammar,
            verdict: Verdict::Correct,
            confidence_percent: 90,
            provenance: CoachProvenance {
                coach: "optional human".into(),
                method: "whole answer check".into(),
                note: String::new(),
            },
            supersedes: BTreeSet::new(),
        }),
    );
    library.merge(&[a, context, judgment]).unwrap();
    let snapshot = library.learner_snapshot(0);
    assert_eq!(snapshot.features.len(), 2);
    for feature in snapshot.features {
        assert!(feature.judgments.is_empty());
        assert_eq!(
            feature.attempt_judgments[&JudgmentDimension::Grammar].correct,
            1
        );
        assert_eq!(
            feature.attempt_judgments[&JudgmentDimension::Grammar].delayed_novel_correct,
            0
        );
    }
}
