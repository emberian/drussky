use rufren_core::*;
use uuid::Uuid;

fn edit(base: &Event, title: &str) -> Event {
    let Fact::PassageRevised(mut r) = base.fact.clone() else {
        unreachable!()
    };
    r.passage.title = title.into();
    r.parents = [base.id].into();
    Event::new(Uuid::new_v4(), 0, Fact::PassageRevised(r)).unwrap()
}

#[test]
fn replication_concurrent_edits_converge_without_losing_either_revision() {
    let base = starter(Uuid::new_v4(), 0);
    let a = edit(&base, "Coach's wording");
    let b = edit(&base, "My wording");
    let mut left = Library::default();
    let mut right = Library::default();
    left.merge(&[base.clone(), a.clone(), b.clone()]).unwrap();
    right
        .merge(&[b.clone(), a.clone(), base.clone(), a.clone()])
        .unwrap();
    assert_eq!(left, right);
    assert_eq!(left.heads().values().next().unwrap().len(), 2);
    let Fact::PassageRevised(mut r) = a.fact.clone() else {
        unreachable!()
    };
    r.parents = [a.id, b.id].into();
    r.passage.title = "Merged wording".into();
    let resolved = Event::new(Uuid::new_v4(), 0, Fact::PassageRevised(r)).unwrap();
    left.merge(&[resolved.clone()]).unwrap();
    assert_eq!(left.heads().values().next().unwrap(), &vec![resolved.id]);
}

#[test]
fn replication_bad_batch_is_atomic_and_future_versions_are_preserved_by_rejection() {
    let base = starter(Uuid::new_v4(), 0);
    let mut library = Library::default();
    library.merge(&[base.clone()]).unwrap();
    let before = library.clone();
    let mut collision = base.clone();
    collision.recorded_at_ms += 1;
    assert!(library.merge(&[edit(&base, "Valid"), collision]).is_err());
    assert_eq!(library, before);
    let mut future = edit(&base, "Future");
    future.version += 1;
    assert!(matches!(library.merge(&[future]), Err(Error::Version(_))));
    assert_eq!(library, before);
}

#[test]
fn replication_cyclic_or_cross_item_history_is_rejected() {
    let a = starter(Uuid::new_v4(), 0);
    let mut b = edit(&a, "B");
    let mut cycle = a.clone();
    if let Fact::PassageRevised(r) = &mut cycle.fact {
        r.parents.insert(b.id);
    }
    assert!(Library::default().merge(&[cycle, b.clone()]).is_err());
    if let Fact::PassageRevised(r) = &mut b.fact {
        r.passage.id = Uuid::new_v4();
    }
    assert!(Library::default().merge(&[a, b]).is_err());
}

#[test]
fn replication_unknown_fields_are_not_silently_discarded() {
    let event = starter(Uuid::new_v4(), 0);
    let mut json = serde_json::to_value(event).unwrap();
    json["fact"]["data"]["passage"]["newer_annotation"] = serde_json::json!("keep me");
    assert!(serde_json::from_value::<Event>(json).is_err());
}
