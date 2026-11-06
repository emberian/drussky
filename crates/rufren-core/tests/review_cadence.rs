use rufren_core::*;
use std::collections::BTreeSet;
use uuid::Uuid;

fn pinned(
    library: &mut Library,
    revision: Uuid,
    started: i64,
    completed: i64,
    cadence: ReviewCadence,
    rating: SelfRating,
) -> Uuid {
    let session = Uuid::new_v4();
    let preference = Event::new(
        Uuid::new_v4(),
        started,
        Fact::ReviewCadenceChanged(ReviewCadenceChange {
            parents: BTreeSet::new(),
            cadence,
        }),
    )
    .unwrap();
    let start = ReviewStartDecision {
        target: ReviewTarget::Passage {
            revision,
            skill: Skill::Reading,
        },
        session,
        policy: REVIEW_TIMING_POLICY_ID.into(),
        as_of_ms: started,
        started_at_ms: started,
        cadence,
        cadence_heads: BTreeSet::from([preference.id]),
        cadence_fallback_reason: None,
        baseline_due_at_ms: started,
        effective_due_at_ms: Some(started),
        timing_paused: false,
        timing_pending: false,
        timing_heads: BTreeSet::new(),
        reason: "Actual retrieval start".into(),
        evidence_ids: BTreeSet::new(),
        evidence_total_count: 0,
        evidence_ids_sha256: review_evidence_ids_sha256(&BTreeSet::new()),
    };
    let start = Event::new(Uuid::new_v4(), started, Fact::ReviewStarted(start)).unwrap();
    let attempt = Event::new(
        Uuid::new_v4(),
        completed,
        Fact::AttemptRecorded(Attempt {
            session,
            revision,
            skill: Skill::Reading,
            response: "Ответ".into(),
            cues: vec![],
            self_rating: rating,
            policy: SPACED_POLICY_ID.into(),
        }),
    )
    .unwrap();
    let id = attempt.id;
    library.merge(&[preference, start, attempt]).unwrap();
    id
}
fn fixture() -> (Library, ReviewKey) {
    let base = starter(Uuid::new_v4(), 0);
    let key = ReviewKey {
        revision: base.id,
        skill: Skill::Reading,
    };
    let mut library = Library::default();
    library.merge(&[base]).unwrap();
    (library, key)
}

#[test]
fn pinned_generic_start_cannot_age_into_delayed_success_or_use_a_new_shorter_gap() {
    let (mut library, key) = fixture();
    pinned(
        &mut library,
        key.revision,
        0,
        0,
        ReviewCadence::MoreSpace,
        SelfRating::Recalled,
    );
    assert_eq!(SpacedReview.project(&library, key, 0).due_at_ms, 2 * DAY_MS);
    pinned(
        &mut library,
        key.revision,
        DAY_MS,
        2 * DAY_MS,
        ReviewCadence::MoreOften,
        SelfRating::Recalled,
    );
    let pending = SpacedReview.project(&library, key, 2 * DAY_MS);
    assert_eq!(pending.successful_spaced_reviews, 0);
    assert_eq!(pending.due_at_ms, 2 * DAY_MS);
    // The response above was practice at day two; day three is not a full two-day gap.
    pinned(
        &mut library,
        key.revision,
        3 * DAY_MS,
        3 * DAY_MS,
        ReviewCadence::MoreOften,
        SelfRating::Recalled,
    );
    assert_eq!(
        SpacedReview
            .project(&library, key, 3 * DAY_MS)
            .successful_spaced_reviews,
        0
    );
    pinned(
        &mut library,
        key.revision,
        5 * DAY_MS,
        5 * DAY_MS,
        ReviewCadence::MoreOften,
        SelfRating::Recalled,
    );
    let next = SpacedReview.project(&library, key, 5 * DAY_MS);
    assert_eq!(next.successful_spaced_reviews, 1);
    assert_eq!(next.interval_ms, 3 * DAY_MS / 2);
}

#[test]
fn pinned_generic_cotimed_conflicts_choose_conservative_evidence_and_interval() {
    let (mut library, key) = fixture();
    pinned(
        &mut library,
        key.revision,
        0,
        0,
        ReviewCadence::Standard,
        SelfRating::Recalled,
    );
    pinned(
        &mut library,
        key.revision,
        DAY_MS,
        DAY_MS,
        ReviewCadence::MoreSpace,
        SelfRating::Recalled,
    );
    pinned(
        &mut library,
        key.revision,
        DAY_MS,
        DAY_MS,
        ReviewCadence::MoreOften,
        SelfRating::Again,
    );
    let state = SpacedReview.project(&library, key, DAY_MS);
    assert_eq!(state.successful_spaced_reviews, 0);
    assert_eq!(state.interval_ms, 10 * MINUTE_MS);
    assert_eq!(state.due_at_ms, DAY_MS + 10 * MINUTE_MS);
    let bytes = serde_json::to_vec(&state).unwrap();
    library
        .merge(&[Event::new(
            Uuid::new_v4(),
            DAY_MS,
            Fact::ReviewCadenceChanged(ReviewCadenceChange {
                parents: BTreeSet::new(),
                cadence: ReviewCadence::MoreSpace,
            }),
        )
        .unwrap()])
        .unwrap();
    assert_eq!(
        serde_json::to_vec(&SpacedReview.project(&library, key, DAY_MS)).unwrap(),
        bytes
    );
}
