//! Migration regression against the immutable pack shipped in the first iPad build.
//! These assertions compare actual core Events and spaced-review state, not just text.
use rufren_content::{ContentPack, Phrase};
use rufren_core::{
    Attempt, Event, Fact, Library, Passage, ReviewKey, Revision, SelfRating, Skill, SpacedReview,
};
use std::collections::BTreeSet;
use uuid::Uuid;

const NAMESPACE: Uuid = Uuid::from_u128(0x2774e059_4afc_50c2_8e35_0e5bf8b33bed);
// Deterministic authored event IDs already exist as v2; protocol upgrades must not
// change their immutable envelope while retaining the same UUID.
const AUTHORED_EVENT_VERSION: u32 = 2;
const LEGACY: &str = include_str!("fixtures/beginner-v1-2026-11-05.3.json");
const CANDIDATE: &str = include_str!("../content/staged/beginner-v1-2026-11-05.4.json");

fn passage_id(pack: &ContentPack, phrase: &Phrase) -> Uuid {
    Uuid::new_v5(&NAMESPACE, format!("{}:{}", pack.id, phrase.id).as_bytes())
}
fn authored_event(pack: &ContentPack, phrase: &Phrase, legacy: bool) -> Event {
    let stable = passage_id(pack, phrase);
    let resolved = pack.resolved_phrase_lineage(phrase);
    let (revision, previous) = if legacy {
        (pack.revision.as_str(), pack.previous_revisions.as_slice())
    } else {
        (resolved.revision, resolved.previous_revisions)
    };
    Event {
        version: AUTHORED_EVENT_VERSION,
        id: Uuid::new_v5(&stable, revision.as_bytes()),
        device: NAMESPACE,
        recorded_at_ms: 0,
        fact: Fact::PassageRevised(Revision {
            passage: Passage {
                id: stable,
                title: phrase.translation.clone(),
                russian: phrase.russian.clone(),
                translation: phrase.translation.clone(),
                source: format!(
                    "Rufren original · editorial draft · {}@{}",
                    phrase.id, revision
                ),
                coach_reviewed: false,
            },
            parents: previous
                .iter()
                .map(|r| Uuid::new_v5(&stable, r.as_bytes()))
                .collect(),
        }),
    }
}
fn packs() -> (ContentPack, ContentPack) {
    (
        ContentPack::from_json(LEGACY).unwrap(),
        ContentPack::from_json(CANDIDATE).unwrap(),
    )
}

#[test]
fn content_lineage_v4_preserves_every_legacy_event_byte_and_review_key() {
    let (old, new) = packs();
    assert_eq!(old.revision, "2026-11-05.3");
    assert_eq!(new.revision, "2026-11-05.4");
    assert_eq!(old.phrases.len(), 103);
    assert_eq!(new.phrases.len(), 169);
    let legacy_events: Vec<_> = old
        .phrases
        .iter()
        .map(|p| authored_event(&old, p, true))
        .collect();
    let replacement_events: Vec<_> = new
        .phrases
        .iter()
        .map(|p| authored_event(&new, p, false))
        .collect();
    let mut attempts = Vec::new();
    for (index, phrase) in old.phrases.iter().enumerate() {
        assert!(phrase.phrase_revision.is_none());
        assert!(phrase.previous_phrase_revisions.is_none());
        let pinned = new.phrase(&phrase.id).unwrap();
        let mut unchanged_payload = pinned.clone();
        unchanged_payload.phrase_revision = None;
        unchanged_payload.previous_phrase_revisions = None;
        assert_eq!(
            phrase, &unchanged_payload,
            "stress, targets, and all authored data must remain unchanged too"
        );
        let lineage = new.resolved_phrase_lineage(pinned);
        assert_eq!(lineage.revision, "2026-11-05.3");
        assert_eq!(
            lineage.previous_revisions,
            &["2026-11-05.1", "2026-11-05.2"]
        );
        let before = authored_event(&old, phrase, true);
        let after = authored_event(&new, pinned, false);
        assert_eq!(before.version, 2);
        assert_eq!(after.version, 2);
        assert_eq!(
            before, after,
            "same deterministic UUID must carry the exact old envelope/fact"
        );
        assert_eq!(
            serde_json::to_vec(&before).unwrap(),
            serde_json::to_vec(&after).unwrap()
        );
        for skill in Skill::ALL {
            let old_key = ReviewKey {
                revision: before.id,
                skill,
            };
            let new_key = ReviewKey {
                revision: after.id,
                skill,
            };
            assert_eq!(old_key, new_key);
            let attempt_id =
                Uuid::new_v5(&before.id, format!("legacy-attempt-{skill:?}").as_bytes());
            attempts.push(Event {
                version: 2,
                id: attempt_id,
                device: NAMESPACE,
                recorded_at_ms: 1_000 + index as i64,
                fact: Fact::AttemptRecorded(Attempt {
                    session: attempt_id,
                    revision: before.id,
                    skill,
                    response: "A preserved learner response".into(),
                    cues: vec![],
                    self_rating: SelfRating::Recalled,
                    policy: "fixture-spaced-history".into(),
                }),
            });
        }
    }
    let mut library = Library::default();
    library.merge(&legacy_events).unwrap();
    library.merge(&attempts).unwrap();
    let now = 1_000_000;
    let prior: Vec<_> = attempts
        .iter()
        .map(|e| {
            let Fact::AttemptRecorded(a) = &e.fact else {
                unreachable!()
            };
            SpacedReview.project(
                &library,
                ReviewKey {
                    revision: a.revision,
                    skill: a.skill,
                },
                now,
            )
        })
        .collect();
    library.merge(&replacement_events).unwrap();
    for (attempt, expected) in attempts.iter().zip(prior) {
        let Fact::AttemptRecorded(a) = &attempt.fact else {
            unreachable!()
        };
        let actual = SpacedReview.project(
            &library,
            ReviewKey {
                revision: a.revision,
                skill: a.skill,
            },
            now,
        );
        assert_eq!(
            actual, expected,
            "pack import must preserve due state and existing history"
        );
        assert_eq!(actual.attempts, 1);
        assert!(!actual.is_new());
    }
    let mut opposite_order = Library::default();
    opposite_order.merge(&replacement_events).unwrap();
    opposite_order.merge(&attempts).unwrap();
    opposite_order.merge(&legacy_events).unwrap();
    assert_eq!(library.events, opposite_order.events);
    assert!(library.heads().values().all(|heads| heads.len() == 1));
}

#[test]
fn content_lineage_new_items_have_no_fake_parents_and_do_not_supersede_user_edits() {
    let (old, new) = packs();
    for phrase in new.phrases.iter().filter(|p| old.phrase(&p.id).is_none()) {
        let lineage = new.resolved_phrase_lineage(phrase);
        assert_eq!(lineage.revision, "2026-11-05.4");
        assert!(lineage.previous_revisions.is_empty());
        let event = authored_event(&new, phrase, false);
        let Fact::PassageRevised(revision) = event.fact else {
            unreachable!()
        };
        assert!(revision.parents.is_empty());
    }
    let original = authored_event(&old, &old.phrases[0], true);
    let pinned = authored_event(&new, new.phrase(&old.phrases[0].id).unwrap(), false);
    let Fact::PassageRevised(mut edited) = original.fact.clone() else {
        unreachable!()
    };
    edited.passage.source = "My own note and chosen wording".into();
    edited.parents = [original.id].into();
    let user_edit = Event {
        version: 2,
        id: Uuid::new_v5(&original.id, b"concurrent-user-edit"),
        device: Uuid::new_v5(&NAMESPACE, b"user-device"),
        recorded_at_ms: 500,
        fact: Fact::PassageRevised(edited),
    };
    let mut library = Library::default();
    library
        .merge(&[original, user_edit.clone(), pinned])
        .unwrap();
    assert_eq!(
        library.heads()[&passage_id(&old, &old.phrases[0])],
        vec![user_edit.id]
    );
}

#[test]
fn content_lineage_rejects_partial_self_parent_unbounded_or_unknown_declarations() {
    let (_, original) = packs();
    for mutate in 0..6 {
        let mut bad = original.clone();
        match mutate {
            0 => bad.phrases[0].previous_phrase_revisions = None,
            1 => bad.phrases[0].phrase_revision = None,
            2 => bad.phrases[0].previous_phrase_revisions = Some(vec!["2026-11-05.3".into()]),
            3 => bad.phrases[0].previous_phrase_revisions = Some(vec!["2026-11-05.1".into(); 2]),
            4 => bad.phrases[0].phrase_revision = Some("x".repeat(129)),
            _ => bad.phrases[0].phrase_revision = Some("unknown-revision".into()),
        }
        assert!(bad.validate().is_err());
    }
    // The candidate's encounter graph genuinely branches and remains reachable.
    let mut encountered = BTreeSet::new();
    let mut had_branch = false;
    while encountered.len() < original.lessons.len() {
        let frontier: Vec<_> = original
            .available_lessons(&encountered)
            .map(|l| l.id.clone())
            .collect();
        assert!(!frontier.is_empty());
        had_branch |= frontier.len() > 1;
        encountered.extend(frontier);
    }
    assert!(had_branch);
}
