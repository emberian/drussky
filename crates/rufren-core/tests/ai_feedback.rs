use rufren_core::*;
use std::collections::BTreeSet;
use uuid::Uuid;
fn event(fact: Fact) -> Event {
    Event::new(Uuid::new_v4(), 0, fact).unwrap()
}
fn fixture() -> (Library, Event, Event, Event, AiAssessment) {
    let revision = starter(Uuid::new_v4(), 0);
    let attempt = event(Fact::AttemptRecorded(Attempt {
        session: Uuid::new_v4(),
        revision: revision.id,
        skill: Skill::Writing,
        response: "Я читаю книгу.".into(),
        cues: vec![],
        self_rating: SelfRating::Recalled,
        policy: SPACED_POLICY_ID.into(),
    }));
    let context = event(Fact::AttemptContextRecorded(AttemptContext {
        attempt: attempt.id,
        content_pack: Some("ru.beginner".into()),
        content_revision: Some("v3".into()),
        exercise_id: Some("ru.exercise.write".into()),
        prompt: None,
        reference_answer: None,
        task: TaskKind::FormRecall,
        target_features: ["ru.target.read".into()].into(),
        novel_context: false,
        delayed_probe: false,
        practice_duration_ms: None,
        effort: None,
        grammar_response: None,
        supersedes: BTreeSet::new(),
    }));
    let assessment = AiAssessment {
        request_id: Uuid::new_v4().to_string(),
        attempt: attempt.id,
        revision: revision.id,
        skill: Skill::Writing,
        task: TaskKind::FormRecall,
        response: "Я читаю книгу.".into(),
        context: Some(context.id),
        confirmed_transcript: None,
        evidence: AiTextEvidence::WrittenText,
        dimension: JudgmentDimension::Grammar,
        verdict: Verdict::Correct,
        suggested_response: None,
        corrections: vec![],
        explanation: "The verb agrees with the speaker, and книгу marks the object.".into(),
        next_step: "Try changing the object while keeping the meaning clear.".into(),
        provenance: AiProvenance {
            provider: "anthropic".into(),
            model: "model-under-test".into(),
            response_id: "provider-response-1".into(),
            prompt_version: "russian-teacher-v1".into(),
        },
        supersedes: BTreeSet::new(),
    };
    let mut library = Library::default();
    library
        .merge(&[revision.clone(), attempt.clone(), context.clone()])
        .unwrap();
    (library, revision, attempt, context, assessment)
}

#[test]
fn ai_feedback_roundtrip_is_durable_and_separate_from_human_confirmed_evidence() {
    let (mut library, revision, attempt, _, mut assessment) = fixture();
    assessment.dimension = JudgmentDimension::Recall;
    let before = SpacedReview.project(
        &library,
        ReviewKey {
            revision: revision.id,
            skill: Skill::Writing,
        },
        0,
    );
    let saved = event(Fact::AiAssessmentRecorded(assessment));
    let encoded = serde_json::to_string(&saved).unwrap();
    let restored: Event = serde_json::from_str(&encoded).unwrap();
    assert_eq!(saved, restored);
    library.merge(&[restored.clone(), restored]).unwrap();
    assert_eq!(
        library
            .ai_assessments(attempt.id, JudgmentDimension::Recall)
            .len(),
        1
    );
    assert!(library
        .judgments(attempt.id, JudgmentDimension::Recall)
        .is_empty());
    assert_eq!(
        SpacedReview.project(
            &library,
            ReviewKey {
                revision: revision.id,
                skill: Skill::Writing
            },
            0
        ),
        before
    );
    assert_eq!(before.coach_confirmed_unassisted, 0);
    assert_eq!(
        library.attempt(attempt.id).unwrap().self_rating,
        SelfRating::Recalled
    );
    assert!(library.learner_snapshot(0).features[0].judgments.is_empty());
}

#[test]
fn ai_feedback_stale_response_revision_skill_and_context_bindings_are_rejected_atomically() {
    let (mut library, _, _, _, assessment) = fixture();
    let before = library.clone();
    let mut mismatches = vec![];
    let mut changed = assessment.clone();
    changed.response = "Я читаю книги.".into();
    mismatches.push(changed);
    let mut changed = assessment.clone();
    changed.revision = Uuid::new_v4();
    mismatches.push(changed);
    let mut changed = assessment.clone();
    changed.skill = Skill::Reading;
    mismatches.push(changed);
    let mut changed = assessment.clone();
    changed.task = TaskKind::Composition;
    mismatches.push(changed);
    let mut changed = assessment.clone();
    changed.context = Some(assessment.attempt);
    mismatches.push(changed);
    let mut changed = assessment;
    changed.attempt = changed.revision;
    mismatches.push(changed);
    for changed in mismatches {
        assert!(library
            .merge(&[event(Fact::AiAssessmentRecorded(changed))])
            .is_err());
        assert_eq!(library, before);
    }
}

#[test]
fn ai_feedback_pagination_defers_checks_until_targets_arrive_without_rewriting_bindings() {
    let (_, revision, attempt, context, assessment) = fixture();
    let feedback = event(Fact::AiAssessmentRecorded(assessment.clone()));
    let mut paged = Library::default();
    paged.merge(&[feedback]).unwrap();
    paged
        .merge(&[context.clone(), attempt.clone(), revision.clone()])
        .unwrap();
    assert_eq!(
        paged.ai_assessments(attempt.id, JudgmentDimension::Grammar)[0].1,
        &assessment
    );
    let mut orphan = assessment;
    orphan.response = "A different response".into();
    let mut invalid = Library::default();
    invalid
        .merge(&[event(Fact::AiAssessmentRecorded(orphan))])
        .unwrap();
    let before = invalid.clone();
    assert!(invalid.merge(&[revision, attempt, context]).is_err());
    assert_eq!(invalid, before);
}

#[test]
fn ai_feedback_concurrent_opinions_require_explicit_resolution_and_keep_all_sources() {
    let (mut library, _, attempt, _, assessment) = fixture();
    let a = event(Fact::AiAssessmentRecorded(assessment.clone()));
    let mut other = assessment.clone();
    other.verdict = Verdict::PartlyCorrect;
    other.provenance.model = "another-model".into();
    other.provenance.response_id = "response-2".into();
    let b = event(Fact::AiAssessmentRecorded(other));
    library.merge(&[b.clone(), a.clone()]).unwrap();
    assert_eq!(
        library
            .ai_assessments(attempt.id, JudgmentDimension::Grammar)
            .len(),
        2
    );
    let mut resolved = assessment;
    resolved.supersedes = [a.id, b.id].into();
    resolved.provenance.response_id = "response-3".into();
    let saved = event(Fact::AiAssessmentRecorded(resolved));
    library.merge(&[saved.clone()]).unwrap();
    assert_eq!(
        library.ai_assessments(attempt.id, JudgmentDimension::Grammar)[0]
            .0
            .id,
        saved.id
    );
    assert!(library.events.contains_key(&a.id) && library.events.contains_key(&b.id));
    assert!(library
        .judgments(attempt.id, JudgmentDimension::Grammar)
        .is_empty());
    let mut cycle_a = a.clone();
    let mut cycle_b = b.clone();
    if let Fact::AiAssessmentRecorded(a) = &mut cycle_a.fact {
        a.supersedes = [b.id].into();
    }
    if let Fact::AiAssessmentRecorded(b) = &mut cycle_b.fact {
        b.supersedes = [a.id].into();
    }
    assert!(Library::default().merge(&[cycle_a, cycle_b]).is_err());
}

#[test]
fn ai_feedback_text_and_transcripts_cannot_claim_unheard_acoustic_dimensions() {
    let (_, _, _, _, assessment) = fixture();
    for evidence in [AiTextEvidence::WrittenText, AiTextEvidence::Transcript] {
        for dimension in [
            JudgmentDimension::Pronunciation,
            JudgmentDimension::Fluency,
            JudgmentDimension::Intelligibility,
        ] {
            let mut changed = assessment.clone();
            changed.evidence = evidence;
            changed.dimension = dimension;
            assert!(Event::new(Uuid::new_v4(), 0, Fact::AiAssessmentRecorded(changed)).is_err());
        }
        let mut language = assessment.clone();
        language.evidence = evidence;
        language.context = None;
        assert!(Event::new(Uuid::new_v4(), 0, Fact::AiAssessmentRecorded(language)).is_ok());
    }
}

#[test]
fn ai_feedback_legacy_events_remain_decodable_and_new_facts_cannot_hide_in_v1() {
    let (_, mut revision, _, _, assessment) = fixture();
    revision.version = 1;
    let bytes = serde_json::to_string(&revision).unwrap();
    let legacy: Event = serde_json::from_str(&bytes).unwrap();
    legacy.validate().unwrap();
    assert_eq!(revision, legacy);
    let mut feedback = event(Fact::AiAssessmentRecorded(assessment));
    feedback.version = 1;
    assert!(matches!(feedback.validate(), Err(Error::Version(1))));
    let mut json = serde_json::to_value(feedback).unwrap();
    json["fact"]["data"]["trusted_human"] = serde_json::json!(true);
    assert!(serde_json::from_value::<Event>(json).is_err());
}

#[test]
fn ai_feedback_missing_provenance_oversized_corrections_and_cross_source_resolution_fail() {
    let (mut library, _, attempt, _, assessment) = fixture();
    let mut malformed = assessment.clone();
    malformed.provenance.response_id.clear();
    assert!(malformed.validate().is_err());
    let mut malformed = assessment.clone();
    malformed.explanation = "x".repeat(MAX_TEXT + 1);
    assert!(malformed.validate().is_err());
    let mut malformed = assessment.clone();
    malformed.corrections = vec![
        AiCorrection {
            original: "книга".into(),
            replacement: "книгу".into(),
            reason: "Accusative object".into()
        };
        17
    ];
    assert!(malformed.validate().is_err());
    let human = event(Fact::CoachJudgmentRecorded(CoachJudgment {
        attempt: attempt.id,
        dimension: JudgmentDimension::Grammar,
        verdict: Verdict::Correct,
        confidence_percent: 80,
        provenance: CoachProvenance {
            coach: "conversation partner".into(),
            method: "optional human feedback".into(),
            note: String::new(),
        },
        supersedes: BTreeSet::new(),
    }));
    library.merge(&[human.clone()]).unwrap();
    let before = library.clone();
    let mut cannot_replace_human = assessment;
    cannot_replace_human.supersedes = [human.id].into();
    assert!(library
        .merge(&[event(Fact::AiAssessmentRecorded(cannot_replace_human))])
        .is_err());
    assert_eq!(library, before);
}

#[test]
fn ai_feedback_context_correction_does_not_mutate_what_the_teacher_actually_evaluated() {
    let (mut library, _, attempt, context, assessment) = fixture();
    library
        .merge(&[event(Fact::AiAssessmentRecorded(assessment.clone()))])
        .unwrap();
    let Fact::AttemptContextRecorded(mut changed) = context.fact else {
        unreachable!()
    };
    changed.task = TaskKind::Composition;
    changed.supersedes = [context.id].into();
    library
        .merge(&[event(Fact::AttemptContextRecorded(changed))])
        .unwrap();
    assert_eq!(
        library.contexts(attempt.id)[0].1.task,
        TaskKind::Composition
    );
    assert_eq!(
        library.ai_assessments(attempt.id, JudgmentDimension::Grammar)[0].1,
        &assessment
    );
}
