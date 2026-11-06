use rufren_core::*;
use std::collections::BTreeSet;
use uuid::Uuid;
fn id(n: u128) -> Uuid {
    Uuid::from_u128(n)
}
fn event(n: u128, fact: Fact) -> Event {
    Event {
        version: 8,
        id: id(n),
        device: id(999),
        recorded_at_ms: 100,
        fact,
    }
}
fn target() -> ReviewTarget {
    ReviewTarget::Passage {
        revision: id(1),
        skill: Skill::Reading,
    }
}
fn timing(n: u128, parents: &[u128], action: ReviewTimingAction) -> Event {
    event(
        n,
        Fact::ReviewTimingChanged(ReviewTimingChange {
            target: target(),
            parents: parents.iter().map(|n| id(*n)).collect(),
            action,
        }),
    )
}
fn cadence(n: u128, parents: &[u128], value: ReviewCadence) -> Event {
    event(
        n,
        Fact::ReviewCadenceChanged(ReviewCadenceChange {
            parents: parents.iter().map(|n| id(*n)).collect(),
            cadence: value,
        }),
    )
}
fn start(session: u128, heads: &[u128]) -> ReviewStartDecision {
    ReviewStartDecision {
        target: target(),
        session: id(session),
        policy: REVIEW_TIMING_POLICY_ID.into(),
        as_of_ms: 100,
        started_at_ms: 100,
        cadence: ReviewCadence::Standard,
        cadence_heads: BTreeSet::new(),
        cadence_fallback_reason: None,
        baseline_due_at_ms: 500,
        effective_due_at_ms: Some(200),
        timing_paused: false,
        timing_pending: false,
        timing_heads: heads.iter().map(|n| id(*n)).collect(),
        reason: "Learner chose a useful return".into(),
        evidence_ids: BTreeSet::new(),
        evidence_total_count: 0,
        evidence_ids_sha256: review_evidence_ids_sha256(&BTreeSet::new()),
    }
}
fn project(l: &Library, completed: &[u128]) -> ReviewTimingState {
    ReviewTimingPolicy.project(
        l,
        target(),
        500,
        100,
        &completed.iter().map(|n| id(*n)).collect(),
    )
}
#[test]
fn causal_concurrency_preserves_pause_and_explicit_resolution() {
    let mut l = Library::default();
    l.merge(&[
        timing(10, &[], ReviewTimingAction::Pause),
        timing(11, &[], ReviewTimingAction::AvailableAt { at_ms: 200 }),
    ])
    .unwrap();
    let s = project(&l, &[]);
    assert!(s.paused && s.conflicted);
    assert_eq!(s.effective_due_at_ms, None);
    l.merge(&[timing(12, &[10], ReviewTimingAction::Resume)])
        .unwrap();
    let s = project(&l, &[]);
    assert!(!s.paused);
    assert!(s.conflicted);
    assert_eq!(s.effective_due_at_ms, Some(200));
    l.merge(&[timing(13, &[11, 12], ReviewTimingAction::Resume)])
        .unwrap();
    assert_eq!(project(&l, &[]).effective_due_at_ms, Some(500));
    assert!(!project(&l, &[]).conflicted);
}
#[test]
fn sooner_replaces_baseline_future_clock_is_not_winner_and_missing_parents_block() {
    let mut l = Library::default();
    let mut a = timing(10, &[], ReviewTimingAction::AvailableAt { at_ms: 50 });
    a.recorded_at_ms = i64::MAX;
    l.merge(&[a]).unwrap();
    assert_eq!(project(&l, &[]).effective_due_at_ms, Some(50));
    l.merge(&[timing(11, &[12], ReviewTimingAction::Resume)])
        .unwrap();
    assert!(project(&l, &[]).pending);
    assert_eq!(project(&l, &[]).effective_due_at_ms, None);
    l.merge(&[timing(12, &[], ReviewTimingAction::Pause)])
        .unwrap();
    assert!(!project(&l, &[]).pending);
    assert_eq!(project(&l, &[]).effective_due_at_ms, Some(50));
}
#[test]
fn preferences_are_causal_conflicts_with_explicit_standard_fallback() {
    let mut l = Library::default();
    l.merge(&[
        cadence(10, &[], ReviewCadence::MoreSpace),
        cadence(11, &[], ReviewCadence::MoreOften),
    ])
    .unwrap();
    let s = ReviewCadencePolicy.project(&l);
    assert!(s.conflicted);
    assert_eq!(s.cadence, ReviewCadence::Standard);
    assert!(s.fallback_reason.unwrap().contains("conflicting"));
    l.merge(&[cadence(12, &[10, 11], ReviewCadence::MoreSpace)])
        .unwrap();
    let s = ReviewCadencePolicy.project(&l);
    assert!(!s.conflicted);
    assert_eq!(s.cadence, ReviewCadence::MoreSpace);
    l.merge(&[cadence(13, &[10, 11], ReviewCadence::MoreSpace)])
        .unwrap();
    let s = ReviewCadencePolicy.project(&l);
    assert!(s.conflicted);
    assert_eq!(s.cadence, ReviewCadence::MoreSpace);
    assert!(s.fallback_reason.is_none());
}
#[test]
fn only_completed_observed_one_shot_heads_are_consumed_pause_persists() {
    let mut l = Library::default();
    l.merge(&[
        timing(10, &[], ReviewTimingAction::AvailableAt { at_ms: 200 }),
        event(20, Fact::ReviewStarted(start(30, &[10]))),
    ])
    .unwrap();
    assert_eq!(project(&l, &[30]).effective_due_at_ms, Some(200)); // opening is not completion
    let attempt = event(
        21,
        Fact::AttemptRecorded(Attempt {
            session: id(30),
            revision: id(1),
            skill: Skill::Reading,
            response: "Ответ".into(),
            cues: vec![],
            self_rating: SelfRating::NotReported,
            policy: "test".into(),
        }),
    );
    l.merge(&[attempt]).unwrap();
    assert_eq!(project(&l, &[]).effective_due_at_ms, Some(200)); // caller still must establish exact completion
    assert_eq!(project(&l, &[30]).effective_due_at_ms, Some(500));
    l.merge(&[timing(
        11,
        &[],
        ReviewTimingAction::AvailableAt { at_ms: 300 },
    )])
    .unwrap();
    assert_eq!(project(&l, &[30]).effective_due_at_ms, Some(300)); // unobserved concurrent choice survives
    l.merge(&[
        timing(12, &[11], ReviewTimingAction::Pause),
        event(22, Fact::ReviewStarted(start(31, &[12]))),
        event(
            23,
            Fact::AttemptRecorded(Attempt {
                session: id(31),
                revision: id(1),
                skill: Skill::Reading,
                response: "Ответ".into(),
                cues: vec![],
                self_rating: SelfRating::NotReported,
                policy: "test".into(),
            }),
        ),
    ])
    .unwrap();
    assert!(project(&l, &[30, 31]).paused);
}
#[test]
fn invalid_cross_targets_cycles_and_session_rewrites_reject_atomically() {
    let mut l = Library::default();
    l.merge(&[timing(10, &[], ReviewTimingAction::Pause)])
        .unwrap();
    let mut other = timing(11, &[10], ReviewTimingAction::Resume);
    if let Fact::ReviewTimingChanged(c) = &mut other.fact {
        c.target = ReviewTarget::Passage {
            revision: id(2),
            skill: Skill::Reading,
        };
    }
    assert!(l.merge(&[other]).is_err());
    assert!(l
        .merge(&[
            timing(11, &[12], ReviewTimingAction::Resume),
            timing(12, &[11], ReviewTimingAction::Pause)
        ])
        .is_err());
    let s = start(30, &[10]);
    l.merge(&[event(20, Fact::ReviewStarted(s.clone()))])
        .unwrap();
    let mut changed = s.clone();
    changed.cadence = ReviewCadence::MoreSpace;
    assert!(l.merge(&[event(21, Fact::ReviewStarted(changed))]).is_err());
    assert_eq!(l.review_start_for_session(id(30), target()), Some(&s));
    let mut old = event(22, Fact::ReviewStarted(s));
    old.version = 7;
    assert!(old.validate().is_err());
}
#[test]
fn late_facts_never_rewrite_saved_start_or_old_fact_bytes() {
    let mut l = Library::default();
    let s = event(20, Fact::ReviewStarted(start(30, &[])));
    let bytes = serde_json::to_vec(&s).unwrap();
    l.merge(&[s.clone()]).unwrap();
    let mut late = cadence(10, &[], ReviewCadence::MoreSpace);
    late.recorded_at_ms = 1;
    l.merge(&[late, timing(11, &[], ReviewTimingAction::Pause)])
        .unwrap();
    assert_eq!(
        serde_json::to_vec(l.events.get(&s.id).unwrap()).unwrap(),
        bytes
    );
    let old = Event {
        version: 1,
        id: id(40),
        device: id(999),
        recorded_at_ms: 0,
        fact: Fact::AttemptRecorded(Attempt {
            session: id(41),
            revision: id(1),
            skill: Skill::Reading,
            response: "Да".into(),
            cues: vec![],
            self_rating: SelfRating::Recalled,
            policy: "old".into(),
        }),
    };
    let bytes = serde_json::to_vec(&old).unwrap();
    l.merge(&[old.clone()]).unwrap();
    assert_eq!(
        serde_json::to_vec(l.events.get(&old.id).unwrap()).unwrap(),
        bytes
    );
}
#[test]
fn paged_preference_history_has_explicit_fallback_and_wrong_types_reject() {
    let mut l = Library::default();
    l.merge(&[cadence(11, &[10], ReviewCadence::MoreSpace)])
        .unwrap();
    let s = ReviewCadencePolicy.project(&l);
    assert!(s.pending);
    assert_eq!(s.cadence, ReviewCadence::Standard);
    assert!(s.fallback_reason.unwrap().contains("incomplete"));
    assert!(l
        .merge(&[timing(10, &[], ReviewTimingAction::Pause)])
        .is_err());
    assert!(ReviewCadencePolicy.project(&l).pending);
    l.merge(&[cadence(10, &[], ReviewCadence::MoreOften)])
        .unwrap();
    let s = ReviewCadencePolicy.project(&l);
    assert!(!s.pending);
    assert_eq!(s.cadence, ReviewCadence::MoreSpace);
}
#[test]
fn historical_cadence_and_missing_input_fallback_survive_late_preferences() {
    let mut l = Library::default();
    l.merge(&[cadence(10, &[], ReviewCadence::MoreSpace)])
        .unwrap();
    let mut s = start(30, &[]);
    s.cadence = ReviewCadence::MoreSpace;
    s.cadence_heads.insert(id(10));
    l.merge(&[
        event(20, Fact::ReviewStarted(s.clone())),
        cadence(11, &[10], ReviewCadence::MoreOften),
    ])
    .unwrap();
    assert_eq!(l.review_start_for_session(id(30), target()), Some(&s));
    let mut wrong = start(31, &[]);
    wrong.cadence_heads.insert(id(10));
    assert!(l.merge(&[event(21, Fact::ReviewStarted(wrong))]).is_err());
    l.merge(&[cadence(12, &[13], ReviewCadence::MoreSpace)])
        .unwrap();
    let mut fallback = start(32, &[]);
    fallback.cadence_heads.insert(id(12));
    fallback.cadence_fallback_reason = Some("incomplete preferences; standard fallback".into());
    l.merge(&[event(22, Fact::ReviewStarted(fallback.clone()))])
        .unwrap();
    l.merge(&[cadence(13, &[], ReviewCadence::MoreSpace)])
        .unwrap();
    assert_eq!(
        l.review_start_for_session(id(32), target()),
        Some(&fallback)
    );
}
#[test]
fn bounded_evidence_and_unavailable_start_preserve_the_issued_scope() {
    let ids: BTreeSet<_> = (1000..1600).map(id).collect();
    let mut s = start(30, &[]);
    s.evidence_total_count = ids.len();
    s.evidence_ids_sha256 = review_evidence_ids_sha256(&ids);
    s.evidence_ids = ids.into_iter().take(MAX_REVIEW_EVIDENCE_IDS).collect();
    s.effective_due_at_ms = None;
    s.timing_paused = true;
    s.reason = "Explicit free practice while paused".into();
    event(20, Fact::ReviewStarted(s.clone()))
        .validate()
        .unwrap();
    s.evidence_total_count = 511;
    assert!(event(21, Fact::ReviewStarted(s)).validate().is_err());
}
