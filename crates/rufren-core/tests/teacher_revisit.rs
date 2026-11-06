use rufren_core::*;
use rufren_tutor_contract::TeacherResponse;
use serde_json::json;
use uuid::Uuid;

fn key() -> TeacherRevisitKey {
    TeacherRevisitKey {
        document_id: Uuid::new_v4(),
        practice_index: 0,
    }
}
fn seed() -> TeacherRevisitSeed {
    TeacherRevisitSeed {
        attempt_id: Uuid::new_v4(),
        context_id: Uuid::new_v4(),
        occurred_at_ms: 0,
    }
}
fn observation(
    time: i64,
    rating: SelfRating,
    cues: Vec<Cue>,
    verdict: Option<Verdict>,
) -> TeacherRevisitObservation {
    TeacherRevisitObservation {
        attempt_id: Uuid::new_v4(),
        context_id: Uuid::new_v4(),
        started_at_ms: time,
        occurred_at_ms: time,
        self_rating: rating,
        cues,
        ai_judgments: verdict
            .map(|verdict| TeacherRevisitAiJudgment {
                fact_id: Uuid::new_v4(),
                recorded_at_ms: time,
                verdict,
            })
            .into_iter()
            .collect(),
        prior_target_exposure_at_ms: None,
    }
}
fn response() -> TeacherResponse {
    serde_json::from_value(json!({
        "request":{"mode":"teach_lesson","input":{"request_id":"public-return-request","lesson_id":"ru.lesson.greetings","title":"Greetings","objective":"Return a greeting.","material":"Привет! — Hello!","learner_context":null}},
        "content":{"explanation":"An informal greeting.","examples":[{"russian":"Привет!","english":"Hi!","note":"To a friend."}],"next_step":"Try it.","caveats":[],"feedback":null,"practice":[{"prompt":"What does this greeting mean?","expected_response":"Hi!","teaching_note":"An informal greeting.","reference_russian":"Привет!","reference_english":"Hi!","skill":"reading"}]},
        "provenance":{"provider":"public-mock","model":"public-model","review_status":"ai_generated_teaching","scope":"Language content.","prompt_version":"teacher-v5","response_id":"public-return-response"},
        "usage":{"input_tokens":10,"output_tokens":20,"estimated_micro_usd":0,"reserved_micro_usd":0}
    })).unwrap()
}

#[test]
fn teacher_revisit_guided_seed_is_a_neutral_next_day_task_anchor() {
    let key = key();
    let seed = seed();
    let s = TeacherRevisitPolicy
        .project(key, seed, &[], DAY_MS / 2)
        .unwrap();
    assert_eq!(s.due_at_ms, DAY_MS);
    assert_eq!(s.rung, 0);
    assert_eq!(s.completed_returns, 0);
    assert_eq!(s.ai_correct_advances, 0);
    assert_eq!(s.self_report_advances, 0);
    assert_eq!(s.last_evidence, TeacherRevisitEvidence::GuidedSeed);
    assert!(!s.is_due(DAY_MS - 1));
    assert!(s.is_due(DAY_MS));
    assert!(TeacherRevisitPolicy.project(key, seed, &[], -1).is_err());
}

#[test]
fn teacher_revisit_ai_correct_and_self_recall_are_distinct_without_invented_ratings() {
    let seed = seed();
    let key = key();
    let ai = observation(
        DAY_MS,
        SelfRating::NotReported,
        vec![],
        Some(Verdict::Correct),
    );
    let a = TeacherRevisitPolicy
        .project(key, seed, &[ai.clone()], DAY_MS)
        .unwrap();
    assert_eq!(a.due_at_ms, 4 * DAY_MS);
    assert_eq!(a.ai_correct_advances, 1);
    assert_eq!(a.self_report_advances, 0);
    assert_eq!(ai.self_rating, SelfRating::NotReported);
    let recalled = observation(DAY_MS, SelfRating::Recalled, vec![], None);
    let b = TeacherRevisitPolicy
        .project(key, seed, &[recalled], DAY_MS)
        .unwrap();
    assert_eq!(b.interval_ms, a.interval_ms);
    assert_eq!(b.self_report_advances, 1);
    assert_eq!(b.ai_correct_advances, 0);
}

#[test]
fn teacher_revisit_late_ai_is_cutoff_bound_once_per_occurrence_and_not_arrival_anchored() {
    let key = key();
    let seed = seed();
    let mut o = observation(
        DAY_MS,
        SelfRating::NotReported,
        vec![],
        Some(Verdict::Correct),
    );
    o.ai_judgments[0].recorded_at_ms = 2 * DAY_MS;
    let before = TeacherRevisitPolicy
        .project(key, seed, &[o.clone()], DAY_MS)
        .unwrap();
    assert_eq!(before.due_at_ms, 2 * DAY_MS);
    assert_eq!(before.ignored_future_judgments, 1);
    assert_eq!(before.ai_correct_advances, 0);
    let after = TeacherRevisitPolicy
        .project(key, seed, &[o.clone(), o.clone()], 2 * DAY_MS)
        .unwrap();
    assert_eq!(after.due_at_ms, 4 * DAY_MS);
    assert_eq!(after.completed_returns, 1);
    assert_eq!(after.ai_correct_advances, 1);
    assert_eq!(
        TeacherRevisitPolicy
            .project(key, seed, &[o], 3 * DAY_MS)
            .unwrap(),
        after
    );
}

#[test]
fn teacher_revisit_replay_holds_answer_help_and_reported_retry_reset_without_early_postponement() {
    let key = key();
    let seed = seed();
    let success = observation(DAY_MS, SelfRating::Recalled, vec![], None);
    let replay = observation(
        4 * DAY_MS,
        SelfRating::NotReported,
        vec![Cue::Replay],
        Some(Verdict::Correct),
    );
    let helped = observation(
        7 * DAY_MS,
        SelfRating::Recalled,
        vec![Cue::Translation],
        Some(Verdict::Correct),
    );
    let a = TeacherRevisitPolicy
        .project(key, seed, &[success.clone(), replay.clone()], 4 * DAY_MS)
        .unwrap();
    assert_eq!(a.interval_ms, 3 * DAY_MS);
    assert_eq!(a.due_at_ms, 7 * DAY_MS);
    assert_eq!(a.replay_only_returns, 1);
    assert_eq!(a.ai_correct_advances, 0);
    let b = TeacherRevisitPolicy
        .project(key, seed, &[success.clone(), replay, helped], 7 * DAY_MS)
        .unwrap();
    assert_eq!(b.rung, 0);
    assert_eq!(b.due_at_ms, 8 * DAY_MS);
    assert_eq!(b.answer_help_returns, 1);
    let error = observation(2 * DAY_MS, SelfRating::Again, vec![], None);
    let repeat = observation(2 * DAY_MS + MINUTE_MS, SelfRating::Again, vec![], None);
    let c = TeacherRevisitPolicy
        .project(key, seed, &[success, error, repeat], 3 * DAY_MS)
        .unwrap();
    assert_eq!(c.due_at_ms, 3 * DAY_MS);
    assert_eq!(c.rung, 0);
    assert_eq!(c.early_error_rechecks, 2);
}

#[test]
fn teacher_revisit_unrated_completed_returns_hold_interval_without_becoming_forever_due() {
    let key = key();
    let seed = seed();
    let first = observation(DAY_MS, SelfRating::NotReported, vec![], None);
    let early = observation(DAY_MS + MINUTE_MS, SelfRating::NotReported, vec![], None);
    let second = observation(2 * DAY_MS, SelfRating::NotReported, vec![], None);
    let s = TeacherRevisitPolicy
        .project(key, seed, &[first, early, second], 2 * DAY_MS)
        .unwrap();
    assert_eq!(s.due_at_ms, 3 * DAY_MS);
    assert_eq!(s.rung, 0);
    assert_eq!(s.unrated_returns, 2);
    assert_eq!(s.ignored_early_returns, 1);
    assert_eq!(s.self_report_advances + s.ai_correct_advances, 0);
}

#[test]
fn teacher_revisit_recent_reference_exposure_is_a_delay_qualifier_without_fake_cues() {
    let key = key();
    let seed = seed();
    let mut o = observation(
        DAY_MS,
        SelfRating::NotReported,
        vec![],
        Some(Verdict::Correct),
    );
    o.prior_target_exposure_at_ms = Some(DAY_MS - MINUTE_MS);
    let s = TeacherRevisitPolicy
        .project(key, seed, &[o.clone()], DAY_MS)
        .unwrap();
    assert_eq!(s.last_evidence, TeacherRevisitEvidence::RecentRestudy);
    assert_eq!(s.recent_restudy_returns, 1);
    assert_eq!(s.due_at_ms, 2 * DAY_MS);
    assert_eq!(s.ai_correct_advances, 0);
    assert!(o.cues.is_empty());
    o.prior_target_exposure_at_ms = Some(DAY_MS + 1);
    assert!(TeacherRevisitPolicy
        .project(key, seed, &[o], 2 * DAY_MS)
        .is_err());
}

#[test]
fn teacher_revisit_cotimed_mixed_responses_do_not_choose_a_uuid_ordered_positive_winner() {
    let key = key();
    let seed = seed();
    let mut correct = observation(
        DAY_MS,
        SelfRating::NotReported,
        vec![],
        Some(Verdict::Correct),
    );
    let mut replay = observation(DAY_MS, SelfRating::Recalled, vec![Cue::Replay], None);
    correct.attempt_id = Uuid::from_u128(1);
    replay.attempt_id = Uuid::from_u128(2);
    let a = TeacherRevisitPolicy
        .project(key, seed, &[correct.clone(), replay.clone()], DAY_MS)
        .unwrap();
    correct.attempt_id = Uuid::from_u128(2);
    replay.attempt_id = Uuid::from_u128(1);
    let b = TeacherRevisitPolicy
        .project(key, seed, &[replay, correct], DAY_MS)
        .unwrap();
    assert_eq!(a.due_at_ms, b.due_at_ms);
    assert_eq!(a.rung, b.rung);
    assert_eq!(a.ai_correct_advances, 0);
    assert_eq!(a.completed_returns, 2);
    assert_eq!(a.last_evidence, TeacherRevisitEvidence::ReplayOnly);
    let mut mixed = observation(
        DAY_MS,
        SelfRating::NotReported,
        vec![],
        Some(Verdict::Correct),
    );
    mixed.ai_judgments.push(TeacherRevisitAiJudgment {
        fact_id: Uuid::new_v4(),
        recorded_at_ms: DAY_MS,
        verdict: Verdict::Incorrect,
    });
    let c = TeacherRevisitPolicy
        .project(key, seed, &[mixed], DAY_MS)
        .unwrap();
    assert_eq!(c.last_evidence, TeacherRevisitEvidence::ContestedAi);
    assert_eq!(c.rung, 0);
    assert_eq!(c.due_at_ms, 2 * DAY_MS);
}

#[test]
fn teacher_revisit_future_and_duplicate_occurrences_do_not_add_spacing_successes() {
    let key = key();
    let seed = seed();
    let first = observation(DAY_MS, SelfRating::Recalled, vec![], None);
    let future = observation(
        99 * DAY_MS,
        SelfRating::Recalled,
        vec![],
        Some(Verdict::Correct),
    );
    let s = TeacherRevisitPolicy
        .project(key, seed, &[first.clone(), future, first.clone()], DAY_MS)
        .unwrap();
    assert_eq!(s.completed_returns, 1);
    assert_eq!(s.ignored_future_attempts, 1);
    assert_eq!(s.self_report_advances, 1);
    let mut collision = first.clone();
    collision.self_rating = SelfRating::Again;
    assert!(TeacherRevisitPolicy
        .project(key, seed, &[first, collision], DAY_MS)
        .is_err());
}

#[test]
fn teacher_revisit_ladder_stops_at120_days_and_equal_delayed_inputs_converge() {
    let key = key();
    let seed = seed();
    let mut observations = Vec::new();
    let mut time = DAY_MS;
    for _ in 0..12 {
        observations.push(observation(
            time,
            SelfRating::NotReported,
            vec![],
            Some(Verdict::Correct),
        ));
        let s = TeacherRevisitPolicy
            .project(key, seed, &observations, time)
            .unwrap();
        assert!(s.interval_ms <= 120 * DAY_MS);
        time = s.due_at_ms;
    }
    let a = TeacherRevisitPolicy
        .project(key, seed, &observations, time)
        .unwrap();
    observations.reverse();
    let b = TeacherRevisitPolicy
        .project(key, seed, &observations, time)
        .unwrap();
    assert_eq!(a, b);
    assert_eq!(a.interval_ms, 120 * DAY_MS);
    assert_eq!(a.rung, 6);
}

#[test]
fn teacher_revisit_canonical_references_keep_old_v2_identity_question_and_bytes() {
    let answer = response();
    let (source, revision, fact) = teacher_practice_reference_fact(&answer, 0).unwrap();
    let passage = Uuid::new_v5(&source, b"0");
    assert_eq!(source, teacher_document_id(&answer));
    assert_eq!(revision, Uuid::new_v5(&passage, b"reference-v1"));
    let expected = Fact::PassageRevised(Revision {
        passage: Passage {
            id: passage,
            title: "Greetings · practice 1".into(),
            russian: "Привет!".into(),
            translation: "Hi!".into(),
            source: "AI teacher exercise · public-mock · public-model · public-return-response · 0"
                .into(),
            coach_reviewed: false,
        },
        parents: Default::default(),
    });
    assert_eq!(
        serde_json::to_vec(&fact).unwrap(),
        serde_json::to_vec(&expected).unwrap()
    );
    let (prompt, reference) = teacher_practice_prompt_reference(&answer, 0).unwrap();
    assert_eq!(
        prompt,
        "What does this greeting mean?\n\nRussian text:\nПривет!"
    );
    assert_eq!(
        reference,
        "Example answer: Hi!\nReference Russian: Привет!\nReference meaning: Hi!\nTeaching note: An informal greeting."
    );
    let event = Event {
        version: 2,
        id: revision,
        device: source,
        recorded_at_ms: 0,
        fact,
    };
    event.validate().unwrap();
    let mut library = Library::default();
    library.merge(&[event]).unwrap();
    // No archive, no attempt: it still must not become an arbitrary all-four-skills passage task.
    assert!(SpacedReview.queue(&library, 0, &Skill::ALL).is_empty());
    let mut ordinary = starter(Uuid::new_v4(), 0);
    library.merge(&[ordinary.clone()]).unwrap();
    assert_eq!(SpacedReview.queue(&library, 0, &Skill::ALL).len(), 4);
    if let Fact::PassageRevised(r) = &mut ordinary.fact {
        r.passage.source = "AI teacher exercise imported legacy reference".into();
    }
    let mut legacy = Library::default();
    legacy.merge(&[ordinary]).unwrap();
    assert!(SpacedReview.queue(&legacy, 0, &Skill::ALL).is_empty());
}

#[test]
fn teacher_revisit_early_retrieval_cannot_become_delayed_by_finishing_after_due() {
    let key = key();
    let seed = seed();
    let finished = DAY_MS + 10 * MINUTE_MS;
    let mut early = observation(
        finished,
        SelfRating::NotReported,
        vec![],
        Some(Verdict::Correct),
    );
    early.started_at_ms = DAY_MS - 10 * MINUTE_MS;
    let held = TeacherRevisitPolicy
        .project(key, seed, &[early.clone()], finished)
        .unwrap();
    assert_eq!(held.ai_correct_advances, 0);
    assert_eq!(held.completed_returns, 0);
    assert_eq!(held.ignored_early_returns, 1);
    assert_eq!(held.due_at_ms, DAY_MS);

    let mut delayed = early.clone();
    delayed.attempt_id = Uuid::new_v4();
    delayed.context_id = Uuid::new_v4();
    delayed.ai_judgments[0].fact_id = Uuid::new_v4();
    delayed.started_at_ms = DAY_MS;
    let accepted = TeacherRevisitPolicy
        .project(key, seed, &[delayed.clone()], finished)
        .unwrap();
    assert_eq!(accepted.ai_correct_advances, 1);
    assert_eq!(accepted.due_at_ms, finished + 3 * DAY_MS); // Cadence anchors to completion.

    let mixed = TeacherRevisitPolicy
        .project(key, seed, &[delayed, early.clone()], finished)
        .unwrap();
    assert_eq!(mixed.ai_correct_advances, 0);
    assert_eq!(mixed.ignored_early_returns, 2);
    assert_eq!(mixed.due_at_ms, DAY_MS);

    early.started_at_ms = finished + 1;
    assert!(TeacherRevisitPolicy
        .project(key, seed, &[early], finished)
        .is_err());
}

#[test]
fn pinned_cadence_changes_future_assignments_without_reinterpreting_previous_delay() {
    use std::collections::BTreeMap;
    let key = key();
    let seed = seed();
    let first = observation(DAY_MS, SelfRating::Recalled, vec![], None);
    let early = observation(3 * DAY_MS, SelfRating::Recalled, vec![], None);
    let later = observation(7 * DAY_MS, SelfRating::Recalled, vec![], None);
    let profiles = BTreeMap::from([
        (first.attempt_id, ReviewCadence::MoreSpace),
        (early.attempt_id, ReviewCadence::MoreOften),
        (later.attempt_id, ReviewCadence::MoreOften),
    ]);
    let assigned = TeacherRevisitPolicy
        .project_with_cadence(key, seed, &[first.clone()], DAY_MS, &profiles)
        .unwrap();
    assert_eq!(assigned.interval_ms, 6 * DAY_MS);
    assert_eq!(assigned.due_at_ms, 7 * DAY_MS);
    // A newly chosen short profile cannot turn two days into the six days actually assigned.
    let pending = TeacherRevisitPolicy
        .project_with_cadence(key, seed, &[first.clone(), early], 3 * DAY_MS, &profiles)
        .unwrap();
    assert_eq!(pending.due_at_ms, assigned.due_at_ms);
    assert_eq!(pending.self_report_advances, 1);
    let next = TeacherRevisitPolicy
        .project_with_cadence(key, seed, &[first, later], 7 * DAY_MS, &profiles)
        .unwrap();
    assert_eq!(next.self_report_advances, 2);
    assert_eq!(next.interval_ms, 7 * DAY_MS / 2);
    assert_eq!(next.due_at_ms, 7 * DAY_MS + 7 * DAY_MS / 2);
}

#[test]
fn cadence_uses_actual_previous_assignment_for_restudied_delay() {
    use std::collections::BTreeMap;
    let seed = seed();
    let mut response = observation(2 * DAY_MS, SelfRating::Recalled, vec![], None);
    response.prior_target_exposure_at_ms = Some(DAY_MS);
    let profiles = BTreeMap::from([
        (seed.attempt_id, ReviewCadence::MoreSpace),
        (response.attempt_id, ReviewCadence::MoreOften),
    ]);
    let state = TeacherRevisitPolicy
        .project_with_cadence(key(), seed, &[response], 2 * DAY_MS, &profiles)
        .unwrap();
    assert_eq!(state.last_evidence, TeacherRevisitEvidence::RecentRestudy);
    assert_eq!(state.self_report_advances, 0);
    assert_eq!(state.interval_ms, 2 * DAY_MS);
}

#[test]
fn standard_cadence_preserves_serialized_projection_and_unrelated_choices_do_nothing() {
    use std::collections::BTreeMap;
    let key = key();
    let seed = seed();
    let observations = vec![
        observation(DAY_MS, SelfRating::Recalled, vec![], None),
        observation(4 * DAY_MS, SelfRating::NotReported, vec![], None),
    ];
    let expected = TeacherRevisitPolicy
        .project(key, seed, &observations, 4 * DAY_MS)
        .unwrap();
    let profiles = BTreeMap::from([
        (seed.attempt_id, ReviewCadence::Standard),
        (observations[0].attempt_id, ReviewCadence::Standard),
        (Uuid::new_v4(), ReviewCadence::MoreSpace),
    ]);
    let actual = TeacherRevisitPolicy
        .project_with_cadence(key, seed, &observations, 4 * DAY_MS, &profiles)
        .unwrap();
    assert_eq!(
        serde_json::to_vec(&expected).unwrap(),
        serde_json::to_vec(&actual).unwrap()
    );
}

#[test]
fn cadence_interval_bounds_cover_negative_and_saturating_input() {
    for cadence in [
        ReviewCadence::MoreOften,
        ReviewCadence::Standard,
        ReviewCadence::MoreSpace,
    ] {
        assert_eq!(cadence.scale_interval_ms(i64::MIN), 10 * MINUTE_MS);
        assert_eq!(cadence.scale_interval_ms(i64::MAX), 120 * DAY_MS);
    }
    let seed = TeacherRevisitSeed {
        occurred_at_ms: i64::MAX - DAY_MS,
        ..seed()
    };
    let profiles = std::collections::BTreeMap::from([(seed.attempt_id, ReviewCadence::MoreSpace)]);
    let state = TeacherRevisitPolicy
        .project_with_cadence(key(), seed, &[], i64::MAX, &profiles)
        .unwrap();
    assert_eq!(state.due_at_ms, i64::MAX);
}

#[test]
fn open_response_keeps_start_qualification_and_cotimed_profiles_are_conservative() {
    use std::collections::BTreeMap;
    let seed = seed();
    let mut opened = observation(3 * DAY_MS, SelfRating::Recalled, vec![], None);
    opened.started_at_ms = DAY_MS;
    let profiles = BTreeMap::from([
        (seed.attempt_id, ReviewCadence::MoreSpace),
        (opened.attempt_id, ReviewCadence::MoreOften),
    ]);
    let state = TeacherRevisitPolicy
        .project_with_cadence(key(), seed, &[opened], 3 * DAY_MS, &profiles)
        .unwrap();
    assert_eq!(state.self_report_advances, 0);
    assert_eq!(state.due_at_ms, 2 * DAY_MS);
    let a = observation(2 * DAY_MS, SelfRating::Recalled, vec![], None);
    let b = observation(2 * DAY_MS, SelfRating::Again, vec![], None);
    let profiles = BTreeMap::from([
        (a.attempt_id, ReviewCadence::MoreSpace),
        (b.attempt_id, ReviewCadence::MoreOften),
    ]);
    let state = TeacherRevisitPolicy
        .project_with_cadence(key(), seed, &[a, b], 2 * DAY_MS, &profiles)
        .unwrap();
    assert_eq!(state.self_report_advances, 0);
    assert_eq!(state.last_evidence, TeacherRevisitEvidence::ClearError);
    assert_eq!(state.interval_ms, DAY_MS / 2);
}
