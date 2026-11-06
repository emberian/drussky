use rufren_core::*;
use uuid::Uuid;
fn candidate(source: PracticeBatchSource, skill: Skill, age: i64) -> PracticeBatchCandidate {
    PracticeBatchCandidate {
        key: match source {
            PracticeBatchSource::Teacher => PracticeBatchKey::Teacher {
                document_id: Uuid::new_v4(),
                practice_index: 0,
            },
            PracticeBatchSource::Passage => PracticeBatchKey::Passage {
                revision: Uuid::new_v4(),
                skill,
            },
        },
        skill,
        due_at_ms: 0,
        last_skill_practice_ms: Some(age),
        last_source_practice_ms: Some(age),
    }
}
#[test]
fn batch_large_teacher_backlog_does_not_starve_practiced_passages() {
    let mut candidates: Vec<_> = (0..120)
        .map(|_| candidate(PracticeBatchSource::Teacher, Skill::Reading, 0))
        .collect();
    candidates.push(candidate(
        PracticeBatchSource::Passage,
        Skill::Listening,
        20,
    ));
    candidates.push(candidate(PracticeBatchSource::Teacher, Skill::Writing, -1));
    let plan = BoundedPracticeBatch
        .select(&candidates, 3, false, 100)
        .unwrap();
    assert_eq!(plan.len(), 3);
    assert!(
        plan.iter()
            .any(|c| c.key.source() == PracticeBatchSource::Passage)
    );
    assert!(plan.iter().any(|c| c.skill == Skill::Writing));
    assert_eq!(
        plan.iter()
            .filter(|c| c.key.source() == PracticeBatchSource::Teacher)
            .count(),
        2
    );
}
#[test]
fn batch_quiet_is_bounded_unique_and_does_not_replace_filtered_skills_with_new_obligations() {
    let mut candidates: Vec<_> = Skill::ALL
        .into_iter()
        .map(|s| candidate(PracticeBatchSource::Teacher, s, 0))
        .collect();
    candidates.push(candidates[2].clone());
    let plan = BoundedPracticeBatch
        .select(&candidates, 5, true, 100)
        .unwrap();
    assert_eq!(plan.len(), 2);
    assert!(
        plan.iter()
            .all(|c| matches!(c.skill, Skill::Reading | Skill::Writing))
    );
    for budget in [1, 3, 5] {
        assert!(
            BoundedPracticeBatch
                .select(&candidates, budget, false, 100)
                .unwrap()
                .len()
                <= budget
        );
    }
    assert!(
        BoundedPracticeBatch
            .select(&candidates, 2, false, 100)
            .is_err()
    );
}
#[test]
fn batch_fairness_uses_actual_recency_then_due_age_and_is_input_order_independent() {
    let mut a = candidate(PracticeBatchSource::Passage, Skill::Reading, 0);
    let mut b = candidate(PracticeBatchSource::Passage, Skill::Writing, 0);
    a.last_skill_practice_ms = Some(90);
    a.due_at_ms = -1000;
    b.last_skill_practice_ms = Some(10);
    b.due_at_ms = -10;
    let c = candidate(PracticeBatchSource::Teacher, Skill::Speaking, 50);
    let source = vec![a, b.clone(), c];
    let mut reversed = source.clone();
    reversed.reverse();
    let x = BoundedPracticeBatch.select(&source, 3, false, 100).unwrap();
    let y = BoundedPracticeBatch
        .select(&reversed, 3, false, 100)
        .unwrap();
    assert_eq!(x, y);
    assert_eq!(x[0].key, b.key);
    assert_eq!(x[1].key.source(), PracticeBatchSource::Teacher);
}
#[test]
fn batch_future_and_inconsistent_candidates_cannot_suppress_ready_items() {
    let mut future = candidate(PracticeBatchSource::Teacher, Skill::Reading, 0);
    future.due_at_ms = 101;
    assert!(
        BoundedPracticeBatch
            .select(&[future], 3, false, 100)
            .unwrap()
            .is_empty()
    );
    let mut bad = candidate(PracticeBatchSource::Passage, Skill::Reading, 0);
    bad.skill = Skill::Writing;
    assert!(BoundedPracticeBatch.select(&[bad], 3, false, 100).is_err());
    let a = candidate(PracticeBatchSource::Teacher, Skill::Reading, 0);
    let mut b = a.clone();
    b.due_at_ms = -1;
    assert!(BoundedPracticeBatch.select(&[a, b], 3, false, 100).is_err());
}
