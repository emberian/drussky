use rufren_core::*;
use std::collections::BTreeSet;
use uuid::Uuid;

fn event(time: i64, fact: Fact) -> Event {
    Event::new(Uuid::new_v4(), time, fact).unwrap()
}
fn attempt(revision: Uuid, skill: Skill, time: i64, rating: SelfRating, cues: Vec<Cue>) -> Event {
    event(
        time,
        Fact::AttemptRecorded(Attempt {
            session: Uuid::new_v4(),
            revision,
            skill,
            response: "Ответ".into(),
            cues,
            self_rating: rating,
            policy: SPACED_POLICY_ID.into(),
        }),
    )
}
fn key(revision: Uuid) -> ReviewKey {
    ReviewKey {
        revision,
        skill: Skill::Reading,
    }
}
fn provenance() -> CoachProvenance {
    CoachProvenance {
        coach: "coach-local-id".into(),
        method: "listened and compared meaning".into(),
        note: "A bounded human judgment".into(),
    }
}
fn judgment(
    attempt: Uuid,
    dimension: JudgmentDimension,
    verdict: Verdict,
    parents: &[Uuid],
) -> Event {
    event(
        0,
        Fact::CoachJudgmentRecorded(CoachJudgment {
            attempt,
            dimension,
            verdict,
            confidence_percent: 90,
            provenance: provenance(),
            supersedes: parents.iter().copied().collect(),
        }),
    )
}
fn context(attempt: Uuid, novel: bool, response: Option<GrammarResponse>) -> Event {
    event(
        0,
        Fact::AttemptContextRecorded(AttemptContext {
            attempt,
            content_pack: Some("ru.beginner".into()),
            content_revision: Some("draft1".into()),
            exercise_id: Some("roles-probe".into()),
            prompt: None,
            reference_answer: None,
            task: TaskKind::GrammarGeneralization,
            target_features: ["ru.target.roles".into()].into(),
            novel_context: novel,
            delayed_probe: true,
            practice_duration_ms: Some(12_000),
            effort: Some(3),
            grammar_response: response,
            supersedes: BTreeSet::new(),
        }),
    )
}
fn reviewed() -> Event {
    let mut e = starter(Uuid::new_v4(), 0);
    e.version = 2; // Existing authored pack imports retain their original event envelope.
    if let Fact::PassageRevised(r) = &mut e.fact {
        r.passage.coach_reviewed = true;
    }
    e
}

#[test]
fn learning_legacy_events_remain_byte_identical_and_v2_cannot_hide_in_v1() {
    let mut base = starter(Uuid::new_v4(), 0);
    base.version = 1;
    let json = serde_json::to_string(&base).unwrap();
    let mut library = Library::default();
    library.merge(&[base.clone()]).unwrap();
    assert_eq!(
        serde_json::to_string(&library.events[&base.id]).unwrap(),
        json
    );
    let a = attempt(base.id, Skill::Reading, 0, SelfRating::Recalled, vec![]);
    let mut legacy_attempt = a.clone();
    legacy_attempt.version = 1;
    library.merge(&[legacy_attempt]).unwrap();
    let mut newer = context(a.id, true, None);
    newer.version = 1;
    assert!(matches!(library.merge(&[newer]), Err(Error::Version(1))));
    assert_eq!(EVENT_VERSION, 8);
    assert_eq!(SYNC_VERSION, 8);
}

#[test]
fn learning_cues_never_become_unassisted_and_skills_revisions_are_separate() {
    let base = starter(Uuid::new_v4(), 0);
    let mut library = Library::default();
    let a = attempt(
        base.id,
        Skill::Reading,
        0,
        SelfRating::Recalled,
        vec![Cue::Translation],
    );
    library.merge(&[base.clone(), a]).unwrap();
    let state = SpacedReview.project(&library, key(base.id), 0);
    assert_eq!(state.last_evidence, Some(ReviewEvidence::Assisted));
    assert_eq!(state.due_at_ms, 30 * MINUTE_MS);
    assert_eq!(state.unassisted_self_reports, 0);
    assert_eq!(state.assisted_attempts, 1);
    let writing = SpacedReview.project(
        &library,
        ReviewKey {
            revision: base.id,
            skill: Skill::Writing,
        },
        0,
    );
    assert!(writing.is_new());
    let Fact::PassageRevised(mut r) = base.fact.clone() else {
        unreachable!()
    };
    r.parents.insert(base.id);
    r.passage.title = "Changed wording".into();
    let revised = event(1, Fact::PassageRevised(r));
    library.merge(&[revised.clone()]).unwrap();
    assert!(SpacedReview.project(&library, key(revised.id), 1).is_new());
    assert_eq!(
        SpacedReview.queue(&library, 1, &[Skill::Reading])[0]
            .state
            .key
            .revision,
        revised.id
    );
}

#[test]
fn learning_spacing_requires_delay_bounds_intervals_and_does_not_move_due_on_cramming() {
    let base = starter(Uuid::new_v4(), 0);
    let mut library = Library::default();
    library
        .merge(&[
            base.clone(),
            attempt(base.id, Skill::Reading, 0, SelfRating::Recalled, vec![]),
        ])
        .unwrap();
    let cramming: Vec<_> = (1..20)
        .map(|t| attempt(base.id, Skill::Reading, t, SelfRating::Recalled, vec![]))
        .collect();
    library.merge(&cramming).unwrap();
    let state = SpacedReview.project(&library, key(base.id), DAY_MS - 1);
    assert_eq!(state.due_at_ms, DAY_MS);
    assert_eq!(state.successful_spaced_reviews, 0);
    assert!(SpacedReview
        .select(&library, DAY_MS - 1, &[Skill::Reading])
        .is_none());
    let mut now = DAY_MS;
    for _ in 0..15 {
        library
            .merge(&[attempt(
                base.id,
                Skill::Reading,
                now,
                SelfRating::Recalled,
                vec![],
            )])
            .unwrap();
        let state = SpacedReview.project(&library, key(base.id), now);
        assert!(state.interval_ms <= 120 * DAY_MS);
        now = state.due_at_ms;
    }
    assert_eq!(
        SpacedReview
            .project(&library, key(base.id), now)
            .interval_ms,
        120 * DAY_MS
    );
    library
        .merge(&[attempt(
            base.id,
            Skill::Reading,
            now,
            SelfRating::Again,
            vec![],
        )])
        .unwrap();
    let state = SpacedReview.project(&library, key(base.id), now);
    assert_eq!(state.interval_ms, 10 * MINUTE_MS);
    assert_eq!(state.due_at_ms, now + 10 * MINUTE_MS);
}

#[test]
fn learning_future_times_are_visible_but_cannot_suppress_today_and_order_converges() {
    let base = starter(Uuid::new_v4(), 0);
    let a = attempt(
        base.id,
        Skill::Reading,
        10 * DAY_MS,
        SelfRating::Recalled,
        vec![],
    );
    let b = attempt(base.id, Skill::Reading, 0, SelfRating::Again, vec![]);
    let mut left = Library::default();
    let mut right = Library::default();
    left.merge(&[a.clone(), base.clone(), b.clone()]).unwrap();
    right.merge(&[b, a, base.clone()]).unwrap();
    let state = SpacedReview.project(&left, key(base.id), DAY_MS);
    assert_eq!(state, SpacedReview.project(&right, key(base.id), DAY_MS));
    assert_eq!(state.attempts, 1);
    assert_eq!(state.ignored_future_attempts, 1);
    assert!(state.is_due(DAY_MS));
}

#[test]
fn learning_concurrent_coach_judgments_need_explicit_resolution_and_preserve_self_report() {
    let base = starter(Uuid::new_v4(), 0);
    let a = attempt(base.id, Skill::Reading, 0, SelfRating::Recalled, vec![]);
    let good = judgment(a.id, JudgmentDimension::Recall, Verdict::Correct, &[]);
    let bad = judgment(a.id, JudgmentDimension::Recall, Verdict::Incorrect, &[]);
    let mut library = Library::default();
    library
        .merge(&[bad.clone(), a.clone(), good.clone(), base.clone()])
        .unwrap();
    let state = SpacedReview.project(&library, key(base.id), 0);
    assert_eq!(state.last_evidence, Some(ReviewEvidence::ContestedJudgment));
    assert_eq!(state.unassisted_self_reports, 1);
    assert_eq!(state.coach_confirmed_unassisted, 0);
    assert_eq!(state.interval_ms, 10 * MINUTE_MS);
    let resolution = judgment(
        a.id,
        JudgmentDimension::Recall,
        Verdict::Correct,
        &[good.id, bad.id],
    );
    library.merge(&[resolution.clone()]).unwrap();
    assert_eq!(library.judgments(a.id, JudgmentDimension::Recall).len(), 1);
    let state = SpacedReview.project(&library, key(base.id), 0);
    assert_eq!(
        state.last_evidence,
        Some(ReviewEvidence::CoachConfirmedUnassisted)
    );
    assert_eq!(
        library.attempt(a.id).unwrap().self_rating,
        SelfRating::Recalled
    );
    assert!(library.events.contains_key(&good.id) && library.events.contains_key(&bad.id));
    let assisted = attempt(
        base.id,
        Skill::Writing,
        0,
        SelfRating::Recalled,
        vec![Cue::Coach],
    );
    library
        .merge(&[
            assisted.clone(),
            judgment(
                assisted.id,
                JudgmentDimension::Recall,
                Verdict::Correct,
                &[],
            ),
        ])
        .unwrap();
    assert_eq!(
        SpacedReview.evidence(&library, assisted.id, library.attempt(assisted.id).unwrap()),
        ReviewEvidence::Assisted
    );
}

#[test]
fn learning_dangling_assessments_validate_on_arrival_and_bad_resolution_is_atomic() {
    let base = starter(Uuid::new_v4(), 0);
    let a = attempt(base.id, Skill::Reading, 0, SelfRating::Again, vec![]);
    let j = judgment(a.id, JudgmentDimension::Recall, Verdict::Incorrect, &[]);
    let mut library = Library::default();
    library.merge(&[j.clone()]).unwrap();
    library.merge(&[a.clone(), base.clone()]).unwrap();
    let other = attempt(base.id, Skill::Writing, 0, SelfRating::Again, vec![]);
    library.merge(&[other.clone()]).unwrap();
    let before = library.clone();
    let bad = judgment(
        other.id,
        JudgmentDimension::Recall,
        Verdict::Correct,
        &[j.id],
    );
    assert!(library.merge(&[bad]).is_err());
    assert_eq!(library, before);
    let wrong_type = judgment(base.id, JudgmentDimension::Recall, Verdict::Correct, &[]);
    assert!(library.merge(&[wrong_type]).is_err());
    let x = judgment(a.id, JudgmentDimension::Recall, Verdict::Correct, &[]);
    let mut y = judgment(a.id, JudgmentDimension::Recall, Verdict::Incorrect, &[x.id]);
    let mut x_cycle = x.clone();
    if let Fact::CoachJudgmentRecorded(j) = &mut x_cycle.fact {
        j.supersedes.insert(y.id);
    }
    assert!(Library::default().merge(&[x_cycle, y.clone()]).is_err());
    if let Fact::CoachJudgmentRecorded(j) = &mut y.fact {
        j.supersedes = [base.id].into();
    }
    assert!(library.merge(&[y]).is_err());
}

#[test]
fn learning_annotation_spans_use_exact_utf8_and_concurrent_notes_are_preserved() {
    let base = starter(Uuid::new_v4(), 0);
    let mut library = Library::default();
    library.merge(&[base.clone()]).unwrap();
    let annotation = |start, end| {
        event(
            0,
            Fact::CoachAnnotationRecorded(CoachAnnotation {
                target: AnnotationTarget::Revision(base.id),
                span: Some(TextSpan { start, end }),
                note: "Read this word with the coach".into(),
                suggested_correction: None,
                provenance: provenance(),
                supersedes: BTreeSet::new(),
            }),
        )
    };
    assert!(library.merge(&[annotation(1, 2)]).is_err());
    let a = annotation(0, 6);
    let b = annotation(0, 6);
    library.merge(&[a.clone(), b.clone()]).unwrap();
    assert_eq!(
        library
            .annotations(AnnotationTarget::Revision(base.id))
            .len(),
        2
    );
    let mut replacement = annotation(0, 6);
    if let Fact::CoachAnnotationRecorded(c) = &mut replacement.fact {
        c.supersedes = [a.id, b.id].into();
    }
    library.merge(&[replacement]).unwrap();
    assert_eq!(
        library
            .annotations(AnnotationTarget::Revision(base.id))
            .len(),
        1
    );
}

#[test]
fn learning_both_policies_are_auditable_without_mandatory_human_review() {
    let a = reviewed();
    let b = reviewed();
    let c = reviewed();
    let domain = ContrastDomain {
        feature: "ru.target.roles".into(),
        skill: Skill::Reading,
        anchor_revision: a.id,
        contrast_revision: b.id,
        novel_probe_revision: c.id,
    };
    let mut library = Library::default();
    library.merge(&[a.clone(), b.clone(), c.clone()]).unwrap();
    let context = PolicyContext {
        session: Uuid::new_v4(),
        now_ms: 0,
        skills: vec![Skill::Reading],
        contrast_domain: Some(domain.clone()),
    };
    for planner in [
        &SpacedReview as &dyn TeachingPlanner,
        &ContrastPolicy as &dyn TeachingPlanner,
    ] {
        let decision = planner.decide(&library, &context).unwrap().unwrap();
        assert_eq!(decision.policy, planner.policy_id());
        assert!(!decision.candidates.is_empty());
        decision.validate().unwrap();
        let saved = event(0, Fact::TeachingDecisionRecorded(decision));
        library.merge(&[saved]).unwrap();
    }
    let next = ContrastPolicy
        .plan(&library, &domain, DAY_MS, Uuid::new_v4())
        .unwrap();
    assert_eq!(
        next.action,
        TeachingAction::ExplainAnchor,
        "recommendations are not completed instruction"
    );
    let mut bad_domain = domain.clone();
    let unreviewed = starter(Uuid::new_v4(), 0);
    bad_domain.anchor_revision = unreviewed.id;
    library.merge(&[unreviewed]).unwrap();
    assert!(
        ContrastPolicy
            .plan(&library, &bad_domain, 0, Uuid::new_v4())
            .is_ok(),
        "AI teaching does not require human signoff"
    );
}

#[test]
fn learning_grammar_beliefs_keep_unknown_explanation_and_recover_from_contradiction() {
    let mut beliefs = GrammarBeliefs::default();
    for _ in 0..10 {
        beliefs.observe(GrammarResponse::FirstNounActor);
    }
    assert!(beliefs.first_noun_actor_bps > 9000);
    assert!(beliefs.case_sensitive_bps > 0 && beliefs.unmodeled_bps > 0);
    for _ in 0..10 {
        beliefs.observe(GrammarResponse::CaseSensitive);
    }
    assert!(beliefs.case_sensitive_bps > 9000);
    for _ in 0..10 {
        beliefs.observe(GrammarResponse::Other);
    }
    assert!(beliefs.unmodeled_bps > 8000);
    assert_eq!(
        u32::from(beliefs.case_sensitive_bps)
            + u32::from(beliefs.first_noun_actor_bps)
            + u32::from(beliefs.unmodeled_bps),
        10_000
    );
}

#[test]
fn learning_predictions_require_completed_practice_delay_and_independent_judgment() {
    let a = reviewed();
    let b = reviewed();
    let c = reviewed();
    let domain = ContrastDomain {
        feature: "ru.target.roles".into(),
        skill: Skill::Reading,
        anchor_revision: a.id,
        contrast_revision: b.id,
        novel_probe_revision: c.id,
    };
    let mut library = Library::default();
    library.merge(&[a, b, c.clone()]).unwrap();
    let mut decision = ContrastPolicy
        .plan(&library, &domain, 0, Uuid::new_v4())
        .unwrap();
    decision.predictions[0].requires_novel_context = false; // This test measures delayed retention, not novel exposure.
    let mut training = attempt(
        domain.anchor_revision,
        Skill::Reading,
        2 * MINUTE_MS,
        SelfRating::Recalled,
        vec![],
    );
    if let Fact::AttemptRecorded(a) = &mut training.fact {
        a.session = decision.session;
        a.policy = decision.policy.clone();
    }
    let saved = event(0, Fact::TeachingDecisionRecorded(decision));
    library.merge(&[saved.clone()]).unwrap();
    assert!(
        library
            .prediction_observations(saved.id, 0, 2 * DAY_MS)
            .is_empty(),
        "a recommendation alone supplies no completion anchor"
    );
    library.merge(&[training]).unwrap();
    let probe_time = DAY_MS + 2 * MINUTE_MS;
    let early = attempt(
        c.id,
        Skill::Reading,
        probe_time - 1,
        SelfRating::Recalled,
        vec![],
    );
    let helped = attempt(
        c.id,
        Skill::Reading,
        probe_time,
        SelfRating::Recalled,
        vec![Cue::OriginalText],
    );
    let valid = attempt(
        c.id,
        Skill::Reading,
        probe_time,
        SelfRating::Recalled,
        vec![],
    );
    for a in [early, helped, valid.clone()] {
        library
            .merge(&[
                a.clone(),
                context(a.id, true, Some(GrammarResponse::CaseSensitive)),
            ])
            .unwrap();
    }
    let observations = library.prediction_observations(saved.id, 0, probe_time);
    assert_eq!(observations.len(), 1);
    assert_eq!(observations[0].outcome, ProbeOutcome::SelfReportOnly);
    library
        .merge(&[judgment(
            valid.id,
            JudgmentDimension::Grammar,
            Verdict::Correct,
            &[],
        )])
        .unwrap();
    let observations = library.prediction_observations(saved.id, 0, probe_time);
    assert_eq!(observations[0].outcome, ProbeOutcome::Correct);
    assert_eq!(observations[0].practice_duration_ms, Some(12_000));
    assert!(library
        .prediction_observations(saved.id, 0, probe_time - 1)
        .is_empty());
    assert_eq!(
        ContrastPolicy
            .plan(&library, &domain, probe_time, Uuid::new_v4())
            .unwrap()
            .action,
        TeachingAction::ContrastExamples
    );
}

#[test]
fn learning_audio_links_replicate_metadata_and_never_create_attempts() {
    let base = starter(Uuid::new_v4(), 0);
    let asset = AudioAsset {
        sha256: "a".repeat(64),
        media_type: "audio/webm;codecs=opus".into(),
        sample_rate_hz: 48_000,
        channels: 1,
        frames: 96_000,
    };
    let attachment = AudioAttachment {
        asset_id: "private-asset-id".into(),
        asset,
        target: AnnotationTarget::Revision(base.id),
        role: AudioRole::Reference,
        note: "Natural rendition".into(),
    };
    let link = event(0, Fact::RecordingAttached(attachment.clone()));
    let mut library = Library::default();
    library.merge(&[link.clone(), base.clone()]).unwrap();
    assert_eq!(
        library
            .recordings(AnnotationTarget::Revision(base.id))
            .len(),
        1
    );
    assert_eq!(library.attempts().count(), 0);
    assert!(SpacedReview.project(&library, key(base.id), 0).is_new());
    let mut conflicting = attachment.clone();
    conflicting.asset.frames += 1;
    let before = library.clone();
    assert!(library
        .merge(&[event(0, Fact::RecordingAttached(conflicting))])
        .is_err());
    assert_eq!(library, before);
    let mut malformed = attachment.clone();
    malformed.asset.sha256 = "not-a-hash".into();
    assert!(Event::new(Uuid::new_v4(), 0, Fact::RecordingAttached(malformed)).is_err());
    let a = attempt(base.id, Skill::Speaking, 0, SelfRating::Again, vec![]);
    let mut learner = attachment;
    learner.target = AnnotationTarget::Attempt(a.id);
    learner.role = AudioRole::Attempt;
    library
        .merge(&[event(0, Fact::RecordingAttached(learner)), a.clone()])
        .unwrap();
    assert_eq!(library.recordings(AnnotationTarget::Attempt(a.id)).len(), 1);
}

#[test]
fn learning_feature_evidence_separates_tasks_coach_judgments_assistance_and_context_conflicts() {
    let base = reviewed();
    let independent = attempt(
        base.id,
        Skill::Reading,
        DAY_MS,
        SelfRating::Recalled,
        vec![],
    );
    let helped = attempt(
        base.id,
        Skill::Reading,
        DAY_MS,
        SelfRating::Recalled,
        vec![Cue::Translation],
    );
    let mut library = Library::default();
    let independent_context = context(independent.id, true, None);
    let helped_context = context(helped.id, true, None);
    library
        .merge(&[
            base.clone(),
            independent.clone(),
            helped.clone(),
            independent_context.clone(),
            helped_context.clone(),
            judgment(
                independent.id,
                JudgmentDimension::Grammar,
                Verdict::Correct,
                &[],
            ),
            judgment(helped.id, JudgmentDimension::Grammar, Verdict::Correct, &[]),
        ])
        .unwrap();
    let snapshot = library.learner_snapshot(DAY_MS);
    assert_eq!(snapshot.features.len(), 1);
    let evidence = &snapshot.features[0];
    assert_eq!(evidence.observations, 2);
    assert_eq!(evidence.independent_self_reports, 1);
    assert_eq!(evidence.assisted_attempts, 1);
    assert_eq!(
        evidence.attempt_judgments[&JudgmentDimension::Grammar].correct,
        2
    );
    assert_eq!(
        evidence.attempt_judgments[&JudgmentDimension::Grammar].delayed_novel_correct,
        0
    );
    assert!(
        evidence.judgments.is_empty(),
        "whole-response judgments are not directly feature-linked evidence"
    );
    assert!(!evidence
        .attempt_judgments
        .contains_key(&JudgmentDimension::Pronunciation));
    let mut conflicting_context = context(independent.id, false, None);
    if let Fact::AttemptContextRecorded(c) = &mut conflicting_context.fact {
        c.task = TaskKind::Pronunciation;
    }
    library.merge(&[conflicting_context.clone()]).unwrap();
    let snapshot = library.learner_snapshot(DAY_MS);
    assert_eq!(snapshot.unresolved_context_attempts, 1);
    assert_eq!(snapshot.features[0].observations, 1);
    let mut resolved = context(independent.id, true, None);
    if let Fact::AttemptContextRecorded(c) = &mut resolved.fact {
        c.supersedes = [independent_context.id, conflicting_context.id].into();
    }
    library.merge(&[resolved]).unwrap();
    assert_eq!(
        library.learner_snapshot(DAY_MS).unresolved_context_attempts,
        0
    );
}

#[test]
fn learning_optional_exercise_provenance_decodes_earlier_context_and_bounds_values() {
    let a = Uuid::new_v4();
    let saved = context(a, false, None);
    let mut json = serde_json::to_value(saved).unwrap();
    for field in [
        "content_pack",
        "content_revision",
        "exercise_id",
        "prompt",
        "reference_answer",
    ] {
        json["fact"]["data"].as_object_mut().unwrap().remove(field);
    }
    let restored: Event = serde_json::from_value(json).unwrap();
    restored.validate().unwrap();
    let Fact::AttemptContextRecorded(mut c) = restored.fact else {
        unreachable!()
    };
    assert!(c.content_pack.is_none() && c.content_revision.is_none() && c.exercise_id.is_none());
    assert!(c.prompt.is_none() && c.reference_answer.is_none());
    c.exercise_id = Some("x".repeat(129));
    assert!(Event::new(Uuid::new_v4(), 0, Fact::AttemptContextRecorded(c)).is_err());
}

#[test]
fn learning_assisted_diagnostic_responses_do_not_shift_experimental_beliefs() {
    let base = reviewed();
    let a = attempt(base.id, Skill::Reading, 0, SelfRating::WithHelp, vec![]);
    let b = attempt(
        base.id,
        Skill::Reading,
        0,
        SelfRating::Recalled,
        vec![Cue::Coach],
    );
    let mut library = Library::default();
    library
        .merge(&[
            base,
            a.clone(),
            b.clone(),
            context(a.id, true, Some(GrammarResponse::CaseSensitive)),
            context(b.id, true, Some(GrammarResponse::CaseSensitive)),
        ])
        .unwrap();
    assert_eq!(
        ContrastPolicy.beliefs(&library, "ru.target.roles", Skill::Reading, 0),
        GrammarBeliefs::default()
    );
}

#[test]
fn learning_same_encoded_audio_hash_allows_independent_device_decoding_observations() {
    let base = starter(Uuid::new_v4(), 0);
    let make = |id: &str, rate, frames| AudioAttachment {
        asset_id: id.into(),
        asset: AudioAsset {
            sha256: "b".repeat(64),
            media_type: "audio/webm;codecs=opus".into(),
            sample_rate_hz: rate,
            channels: 1,
            frames,
        },
        target: AnnotationTarget::Revision(base.id),
        role: AudioRole::Reference,
        note: "Two-second original, decoded by this device".into(),
    };
    let at_44100 = make("device-a-original", 44_100, 88_200);
    let at_48000 = make("device-b-original", 48_000, 96_000);
    let a = event(0, Fact::RecordingAttached(at_44100));
    let b = event(0, Fact::RecordingAttached(at_48000));
    let mut left = Library::default();
    let mut right = Library::default();
    left.merge(&[base.clone(), a.clone(), b.clone()]).unwrap();
    right.merge(&[b, a, base.clone()]).unwrap();
    assert_eq!(left, right);
    assert_eq!(
        left.recordings(AnnotationTarget::Revision(base.id)).len(),
        2
    );
    assert_eq!(left.attempts().count(), 0);
    let before = left.clone();
    let id_reused = event(
        0,
        Fact::RecordingAttached(make("device-a-original", 48_000, 96_000)),
    );
    assert!(
        left.merge(&[id_reused]).is_err(),
        "an existing asset ID still denotes its immutable original metadata"
    );
    assert_eq!(left, before);
}

#[test]
fn learning_saved_question_and_reference_roundtrip_with_byte_bounds() {
    let saved = context(Uuid::new_v4(), false, None);
    let Fact::AttemptContextRecorded(mut value) = saved.fact else {
        unreachable!()
    };
    value.prompt = Some("Choose who sees whom.\na = Аня видит Веру\nb = Вера видит Аню".into());
    value.reference_answer = Some("Authored correct option: a = Аня видит Веру".into());
    let encoded = event(0, Fact::AttemptContextRecorded(value.clone()));
    let restored: Event = serde_json::from_str(&serde_json::to_string(&encoded).unwrap()).unwrap();
    assert_eq!(encoded, restored);
    restored.validate().unwrap();
    value.prompt = Some("x".repeat(8192));
    assert!(Event::new(
        Uuid::new_v4(),
        0,
        Fact::AttemptContextRecorded(value.clone())
    )
    .is_ok());
    value.prompt = Some("x".repeat(8193));
    assert!(Event::new(
        Uuid::new_v4(),
        0,
        Fact::AttemptContextRecorded(value.clone())
    )
    .is_err());
    value.prompt = Some("valid question".into());
    value.reference_answer = Some("я".repeat(4097));
    assert!(
        Event::new(
            Uuid::new_v4(),
            0,
            Fact::AttemptContextRecorded(value.clone())
        )
        .is_err(),
        "the limit counts UTF-8 bytes, not characters"
    );
    value.reference_answer = Some(" ".into());
    assert!(Event::new(Uuid::new_v4(), 0, Fact::AttemptContextRecorded(value)).is_err());
}
