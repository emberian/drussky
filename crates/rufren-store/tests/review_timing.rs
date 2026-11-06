//! Timing intent remains causal data across SQLite reopen; clocks select no winner.
use rufren_core::*;
use rufren_store::Store;
use std::collections::BTreeSet;
use uuid::Uuid;

#[test]
fn divergent_timing_and_saved_start_survive_reverse_arrival_and_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("timing.sqlite");
    let source = starter(Uuid::new_v4(), 1);
    let target = ReviewTarget::Passage { revision: source.id, skill: Skill::Reading };
    let left = Uuid::new_v4();
    let right = Uuid::new_v4();
    let cadence_a = Event::new(left, 900, Fact::ReviewCadenceChanged(ReviewCadenceChange {
        parents: BTreeSet::new(), cadence: ReviewCadence::MoreOften,
    })).unwrap();
    let cadence_b = Event::new(right, 2, Fact::ReviewCadenceChanged(ReviewCadenceChange {
        parents: BTreeSet::new(), cadence: ReviewCadence::MoreSpace,
    })).unwrap();
    let pause = Event::new(left, 800, Fact::ReviewTimingChanged(ReviewTimingChange {
        target, parents: BTreeSet::new(), action: ReviewTimingAction::Pause,
    })).unwrap();
    let defer = Event::new(right, 3, Fact::ReviewTimingChanged(ReviewTimingChange {
        target, parents: BTreeSet::new(), action: ReviewTimingAction::AvailableAt { at_ms: 5000 },
    })).unwrap();
    let start = Event::new(left, 1000, Fact::ReviewStarted(ReviewStartDecision {
        target, session: Uuid::new_v4(), policy: REVIEW_TIMING_POLICY_ID.into(),
        as_of_ms: 999, started_at_ms: 1000, cadence: ReviewCadence::MoreOften,
        cadence_heads: BTreeSet::from([cadence_a.id]), cadence_fallback_reason: None,
        baseline_due_at_ms: 500, effective_due_at_ms: Some(500), timing_paused: false, timing_pending: false, timing_heads: BTreeSet::new(),
        reason: "Synthetic storage snapshot before the other device arrived".into(),
        evidence_ids: BTreeSet::from([source.id]),
        evidence_total_count: 1,
        evidence_ids_sha256: review_evidence_ids_sha256(&BTreeSet::from([source.id])),
    })).unwrap();
    let saved = serde_json::to_string(&start).unwrap();
    let mut store = Store::open(&path).unwrap();
    // Intent and start may arrive before both their source and known cadence fact.
    let events = vec![start.clone(), defer.clone(), cadence_b.clone(), pause.clone(), cadence_a.clone(), source];
    let early = store.exchange(&SyncRequest { version: SYNC_VERSION, relay: None, after: 0, events: events[..4].to_vec() }).unwrap();
    assert_eq!(serde_json::to_string(&store.library().unwrap().events[&start.id]).unwrap(), saved);
    let result = store.exchange(&SyncRequest { version: SYNC_VERSION, relay: Some(early.relay), after: early.cursor, events: events[4..].to_vec() }).unwrap();
    assert_eq!(result.cursor, events.len() as u64);
    let library = store.library().unwrap();
    let cadence = ReviewCadencePolicy.project(&library);
    assert!(cadence.conflicted);
    assert_eq!(cadence.cadence, ReviewCadence::Standard);
    assert_eq!(cadence.heads, BTreeSet::from([cadence_a.id, cadence_b.id]));
    let timing = ReviewTimingPolicy.project(&library, target, 500, 1000, &BTreeSet::new());
    assert!(timing.conflicted && timing.paused);
    assert_eq!(timing.effective_due_at_ms, None);
    assert_eq!(timing.heads, BTreeSet::from([pause.id, defer.id]));
    let resume = Event::new(left, 4, Fact::ReviewTimingChanged(ReviewTimingChange {
        target, parents: timing.heads, action: ReviewTimingAction::Resume,
    })).unwrap();
    let retry = store.exchange(&SyncRequest { version: SYNC_VERSION, relay: Some(result.relay), after: result.cursor, events: events.clone() }).unwrap();
    assert_eq!(retry.cursor, result.cursor);
    assert!(retry.events.is_empty());
    store.exchange(&SyncRequest { version: SYNC_VERSION, relay: Some(result.relay), after: result.cursor, events: vec![resume.clone()] }).unwrap();
    drop(store);
    let reopened = Store::open(&path).unwrap();
    let library = reopened.library().unwrap();
    let timing = ReviewTimingPolicy.project(&library, target, 500, 1000, &BTreeSet::new());
    assert!(!timing.conflicted && !timing.paused);
    assert_eq!(timing.heads, BTreeSet::from([resume.id]));
    assert_eq!(timing.effective_due_at_ms, Some(500));
    assert_eq!(serde_json::to_string(&library.events[&start.id]).unwrap(), saved);
    for event in events { assert_eq!(library.events[&event.id], event); }
}
