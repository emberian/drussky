use rufren_core::*;
use serde_json::json;
use std::collections::BTreeSet;
use uuid::Uuid;

fn id(name: &str) -> Uuid {
    Uuid::new_v5(
        &Uuid::NAMESPACE_URL,
        format!("sound-v6-fixture:{name}").as_bytes(),
    )
}
fn event(name: &str, time: i64, fact: Fact) -> Event {
    Event {
        version: 6,
        id: id(name),
        device: id("device"),
        recorded_at_ms: time,
        fact,
    }
}
fn definition() -> SoundContrastSnapshot {
    let member = |name: &str, label: &str, supported: &str, meaning: &str| SoundMemberSnapshot {
        id: format!("ru.sound.member.{name}"),
        version: "1".into(),
        option_label: label.into(),
        lexeme: "замок".into(),
        supported_lexeme: supported.into(),
        utterance: SoundUtteranceSnapshot {
            id: format!("ru.sound.utterance.{name}.carrier"),
            version: "1".into(),
            text: format!("Это {supported}."),
            meaning: meaning.into(),
            kind: SoundUtteranceShape::CarrierSentence,
            related_phrase_id: format!("ru.phrase.{name}"),
            related_phrase_revision: "2026-11-05.3".into(),
        },
    };
    SoundContrastSnapshot {
        id: "ru.sound.contrast.fixture".into(),
        version: "1".into(),
        lesson_id: "ru.lesson.naming".into(),
        target_id: "ru.target.stress_contrast".into(),
        title: "Castle or lock?".into(),
        teaching_note: "Listen to the whole word.".into(),
        detail_note: "Stress is not simply pitch or loudness.".into(),
        provenance_note: "Synthetic test definitions; no acoustic certification.".into(),
        source_ids: vec!["ru.source.stress-pair".into()],
        members: [
            member("castle", "A castle", "за́мок", "This is a castle."),
            member("lock", "A lock", "замо́к", "This is a lock."),
        ],
        identify_task: SoundTaskSnapshot {
            id: "ru.task.sound.fixture.identify".into(),
            version: "1".into(),
            instruction: "Choose the meaning you heard.".into(),
        },
        produce_task: SoundTaskSnapshot {
            id: "ru.task.sound.fixture.produce".into(),
            version: "1".into(),
            instruction: "Choose an intended meaning, then record.".into(),
        },
        retired_exercise_ids: vec!["ru.exercise.naming.hear-stress".into()],
    }
}
fn refs(snapshot: &SoundContrastSnapshot) -> (Vec<Event>, [SoundReferenceBinding; 2]) {
    let mut events = Vec::new();
    let bindings = std::array::from_fn(|i| {
        let member = &snapshot.members[i];
        let reference = sound_reference_event(snapshot, &member.id).unwrap();
        let recording = Uuid::new_v5(&reference.id, b"fixture-recording");
        let asset_id = format!("sound-reference-{i}");
        let asset = AudioAsset {
            sha256: if i == 0 { "a" } else { "b" }.repeat(64),
            media_type: "audio/wav".into(),
            sample_rate_hz: 24000,
            channels: 1,
            frames: 24000,
        };
        let source = member.utterance.text.clone();
        let hash = dialogue_text_sha256(&source);
        let binding=SoundReferenceBinding {member_id:member.id.clone(),revision:reference.id,recording,asset_id:asset_id.clone(),asset:asset.clone(),
            source_text:source.clone(),source_text_sha256:hash.clone(),provenance_json:json!({"kind":"system_generated_speech","provider":"fixture",
                "model":"fixture-model","voice":"fixture-voice","request_id":format!("reference-{i}"),"source_text":source,"source_text_sha256":hash}).to_string()};
        events.push(reference);
        events.push(Event {
            version: 6,
            id: recording,
            device: id("device"),
            recorded_at_ms: 100,
            fact: Fact::RecordingAttached(AudioAttachment {
                asset_id,
                asset,
                target: AnnotationTarget::Revision(binding.revision),
                role: AudioRole::Reference,
                note: String::new(),
            }),
        });
        binding
    });
    (events, bindings)
}
fn fixture() -> (Vec<Event>, Event) {
    let contrast = definition();
    let (base, references) = refs(&contrast);
    let p = SoundTrialPrepared {
        trial_id: id("trial-0"),
        token: id("token-0"),
        block_id: id("block"),
        position: 0,
        mode: SoundTrialMode::Listening,
        target_member_id: contrast.members[0].id.clone(),
        option_member_ids: [
            contrast.members[0].id.clone(),
            contrast.members[1].id.clone(),
        ],
        contrast,
        references,
    };
    (base, event("prepared-0", 200, Fact::SoundTrialPrepared(p)))
}
fn prepared(e: &Event) -> &SoundTrialPrepared {
    match &e.fact {
        Fact::SoundTrialPrepared(p) => p,
        _ => panic!("fixture prepared"),
    }
}
fn play(
    prepared_event: &Event,
    name: &str,
    time: i64,
    member: usize,
    natural_speed: bool,
) -> (Event, Event) {
    let p = prepared(prepared_event);
    let scope = SoundPlaybackScope::Trial {
        prepared: prepared_event.id,
    };
    let start = event(
        &format!("start-{name}"),
        time,
        Fact::SoundReferencePlayed(SoundReferencePlayed {
            scope: scope.clone(),
            token: p.token,
            reference: p.references[member].clone(),
            stage: SoundPlaybackStage::Started,
            natural_speed,
        }),
    );
    let end = event(
        &format!("end-{name}"),
        time + 10,
        Fact::SoundReferencePlayed(SoundReferencePlayed {
            scope,
            token: p.token,
            reference: p.references[member].clone(),
            stage: SoundPlaybackStage::Completed {
                started_event: start.id,
            },
            natural_speed,
        }),
    );
    (start, end)
}
fn context(p: &SoundTrialPrepared, attempt: Uuid) -> AttemptContext {
    let task = if p.mode == SoundTrialMode::Listening {
        &p.contrast.identify_task
    } else {
        &p.contrast.produce_task
    };
    AttemptContext {
        attempt,
        content_pack: Some("rufren-sound-focus".into()),
        content_revision: Some(p.contrast.version.clone()),
        exercise_id: Some(format!("{}@{}", task.id, task.version)),
        prompt: Some(task.instruction.clone()),
        reference_answer: None,
        task: if p.mode == SoundTrialMode::Listening {
            TaskKind::SoundDiscrimination
        } else {
            TaskKind::Pronunciation
        },
        target_features: [p.contrast.target_id.clone()].into_iter().collect(),
        novel_context: false,
        delayed_probe: false,
        practice_duration_ms: None,
        effort: None,
        grammar_response: None,
        supersedes: BTreeSet::new(),
    }
}
fn listening_packet(prepared_event: &Event, chosen: usize, name: &str) -> Vec<Event> {
    let p = prepared(prepared_event);
    let target = usize::from(p.target_member_id == p.references[1].member_id);
    let (start, end) = play(prepared_event, name, 220, target, true);
    let attempt = event(
        &format!("attempt-{name}"),
        300,
        Fact::AttemptRecorded(Attempt {
            session: p.trial_id,
            revision: p.references[target].revision,
            skill: Skill::Listening,
            response: p.contrast.members[chosen].option_label.clone(),
            cues: Vec::new(),
            self_rating: SelfRating::NotReported,
            policy: "sound-focus-v1".into(),
        }),
    );
    let context = event(
        &format!("context-{name}"),
        301,
        Fact::AttemptContextRecorded(context(p, attempt.id)),
    );
    let completed = event(
        &format!("completed-{name}"),
        302,
        Fact::SoundTrialCompleted(SoundTrialCompleted {
            prepared: prepared_event.id,
            attempt: attempt.id,
            context: context.id,
            response: SoundTrialResponse::Listening {
                chosen_member_id: p.contrast.members[chosen].id.clone(),
            },
            known_playback_ids: vec![start.id, end.id],
            support_presentation_ids: Vec::new(),
            revealed: false,
        }),
    );
    vec![start, end, attempt, context, completed]
}
fn speaking_prepared(base: &Event, name: &str, time: i64, target: usize) -> Event {
    let mut p = prepared(base).clone();
    p.mode = SoundTrialMode::Speaking;
    p.position = 4;
    p.trial_id = id(&format!("speak-trial-{name}"));
    p.token = id(&format!("speak-token-{name}"));
    p.target_member_id = p.contrast.members[target].id.clone();
    event(
        &format!("speaking-{name}"),
        time,
        Fact::SoundTrialPrepared(p),
    )
}
fn speaking_packet(prepared_event: &Event, name: &str, captured_at: i64, sha: &str) -> Vec<Event> {
    let p = prepared(prepared_event);
    let target = p
        .references
        .iter()
        .find(|r| r.member_id == p.target_member_id)
        .unwrap();
    let asset_id = format!("sound-take-{name}");
    let asset = AudioAsset {
        sha256: sha.repeat(64),
        media_type: "audio/wav".into(),
        sample_rate_hz: 48000,
        channels: 1,
        frames: 48000,
    };
    let original = event(
        &format!("original-{name}"),
        captured_at,
        Fact::RecordingAttached(AudioAttachment {
            asset_id: asset_id.clone(),
            asset: asset.clone(),
            target: AnnotationTarget::Revision(target.revision),
            role: AudioRole::Attempt,
            note: String::new(),
        }),
    );
    let attempt = event(
        &format!("attempt-{name}"),
        captured_at + 100,
        Fact::AttemptRecorded(Attempt {
            session: p.trial_id,
            revision: target.revision,
            skill: Skill::Speaking,
            response: format!("Sound Focus recording · {asset_id}"),
            cues: Vec::new(),
            self_rating: SelfRating::NotReported,
            policy: "sound-focus-v1".into(),
        }),
    );
    let copy = event(
        &format!("attempt-recording-{name}"),
        captured_at + 100,
        Fact::RecordingAttached(AudioAttachment {
            asset_id: asset_id.clone(),
            asset: asset.clone(),
            target: AnnotationTarget::Attempt(attempt.id),
            role: AudioRole::Attempt,
            note: String::new(),
        }),
    );
    let ctx = event(
        &format!("context-{name}"),
        captured_at + 101,
        Fact::AttemptContextRecorded(context(p, attempt.id)),
    );
    let completed = event(
        &format!("completed-{name}"),
        captured_at + 102,
        Fact::SoundTrialCompleted(SoundTrialCompleted {
            prepared: prepared_event.id,
            attempt: attempt.id,
            context: ctx.id,
            response: SoundTrialResponse::Speaking {
                intended_member_id: p.target_member_id.clone(),
                original_recording: original.id,
                attempt_recording: copy.id,
                capture_scope: SoundCaptureScope {
                    session_id: p.block_id,
                    intent_id: p.trial_id,
                    member_id: p.target_member_id.clone(),
                },
                captured_at_ms: captured_at,
                asset_id,
                asset,
            },
            known_playback_ids: Vec::new(),
            support_presentation_ids: Vec::new(),
            revealed: false,
        }),
    );
    vec![original, copy, attempt, ctx, completed]
}
fn merged(base: Vec<Event>, extra: Vec<Event>) -> Library {
    let mut library = Library::default();
    library
        .merge(&base.into_iter().chain(extra).collect::<Vec<_>>())
        .unwrap();
    library
}

#[test]
fn sound_core_preparation_is_neither_playback_attempt_nor_learning() {
    let (mut base, p) = fixture();
    base.push(p.clone());
    let library = merged(base, Vec::new());
    assert_eq!(library.attempts().count(), 0);
    assert_eq!(library.presentations().count(), 0);
    assert_eq!(library.sound_trials().len(), 1);
    assert!(library.sound_trials()[0].incomplete);
    assert_eq!(library.sound_trials()[0].intended_match, None);
    assert!(SpacedReview.queue(&library, 1000, &Skill::ALL).is_empty());
    assert!(library.learner_snapshot(1000).features.is_empty());
    assert_eq!(
        library.sound_prepared(prepared(&p).trial_id).unwrap().1,
        prepared(&p)
    );
}

#[test]
fn sound_core_deterministic_reference_is_exact_and_not_native_certification() {
    let d = definition();
    let a = sound_reference_event(&d, &d.members[0].id).unwrap();
    let b = sound_reference_event(&d, &d.members[0].id).unwrap();
    assert_eq!(a, b);
    assert_eq!(a.version, 6);
    assert_eq!(a.recorded_at_ms, 0);
    assert!(is_sound_focus_reference(&a));
    let (passage, revision) = sound_reference_identity(&d.members[0].utterance.id, "1");
    assert_eq!(a.id, revision);
    if let Fact::PassageRevised(r) = &a.fact {
        assert_eq!(r.passage.id, passage);
        assert_eq!(r.passage.russian, d.members[0].utterance.text);
        assert!(!r.passage.coach_reviewed);
        assert!(r.parents.is_empty());
    }
    let mut bad = a;
    bad.recorded_at_ms = 1;
    assert!(bad.validate().is_err());
}

#[test]
fn sound_core_full_graph_is_order_independent_and_nominal_choice_is_not_a_grade() {
    let (mut base, p) = fixture();
    let packet = listening_packet(&p, 0, "one");
    base.push(p.clone());
    let forward = merged(base.clone(), packet.clone());
    let mut reversed: Vec<_> = base.into_iter().chain(packet).collect();
    reversed.reverse();
    let reverse = merged(reversed, Vec::new());
    assert_eq!(forward, reverse);
    let summaries = forward.sound_trials();
    let summary = &summaries[0];
    assert!(!summary.incomplete);
    assert!(!summary.assisted);
    assert_eq!(summary.intended_match, Some(true));
    assert_eq!(forward.attempts().count(), 1);
    assert_eq!(
        forward.sound_trial_completion(prepared(&p).trial_id).len(),
        1
    );
    assert!(!forward.events.values().any(|e| matches!(
        e.fact,
        Fact::CoachJudgmentRecorded(_) | Fact::AiAssessmentRecorded(_)
    )));
}

#[test]
fn sound_core_missing_links_are_incomplete_and_late_wrong_types_reject_atomically() {
    let (base, p) = fixture();
    let packet = listening_packet(&p, 0, "late");
    let mut library = Library::default();
    library
        .merge(&[
            p.clone(),
            packet[2].clone(),
            packet[3].clone(),
            packet[4].clone(),
        ])
        .unwrap();
    assert!(library.sound_trials()[0].incomplete);
    let before = library.clone();
    let mut wrong = base[0].clone();
    wrong.id = packet[1].id;
    assert!(library.merge(&[wrong]).is_err());
    assert_eq!(library, before);
    library.merge(&[packet[1].clone()]).unwrap();
    assert!(library.sound_trials()[0].incomplete);
    library.merge(&base).unwrap();
    assert!(library.sound_trials()[0].incomplete);
    library.merge(&[packet[0].clone()]).unwrap();
    assert!(!library.sound_trials()[0].incomplete);
}

#[test]
fn sound_core_started_slowed_wrong_member_and_examples_do_not_unlock_listening() {
    for case in ["started", "slowed", "wrong-member", "example"] {
        let (mut base, p) = fixture();
        let mut packet = listening_packet(&p, 0, case);
        match case {
            "started" => {
                let end = packet.remove(1);
                if let Fact::SoundTrialCompleted(c) = &mut packet.last_mut().unwrap().fact {
                    c.known_playback_ids.retain(|id| *id != end.id);
                }
            }
            "slowed" => {
                for e in &mut packet[..2] {
                    if let Fact::SoundReferencePlayed(r) = &mut e.fact {
                        r.natural_speed = false;
                    }
                }
            }
            "wrong-member" => {
                for e in &mut packet[..2] {
                    if let Fact::SoundReferencePlayed(r) = &mut e.fact {
                        r.reference = prepared(&p).references[1].clone();
                    }
                }
            }
            "example" => {
                for e in &mut packet[..2] {
                    if let Fact::SoundReferencePlayed(r) = &mut e.fact {
                        r.scope = SoundPlaybackScope::Example {
                            block_id: prepared(&p).block_id,
                            member_id: r.reference.member_id.clone(),
                        };
                        r.token = prepared(&p).block_id;
                    }
                }
            }
            _ => unreachable!(),
        }
        base.push(p);
        let mut library = Library::default();
        assert!(
            library
                .merge(&base.into_iter().chain(packet).collect::<Vec<_>>())
                .is_err(),
            "{case}"
        );
    }
}

#[test]
fn sound_core_partial_balance_duplicate_trials_and_immutable_definition_collisions() {
    let (mut base, p) = fixture();
    base.push(p.clone());
    let mut library = merged(base, Vec::new());
    let mut duplicate = p.clone();
    duplicate.id = id("identical-prepared-copy");
    library.merge(&[duplicate]).unwrap();
    assert_eq!(library.sound_trials().len(), 1);
    let mut wrong = p.clone();
    wrong.id = id("position-collision");
    if let Fact::SoundTrialPrepared(p) = &mut wrong.fact {
        p.trial_id = id("different-trial");
        p.token = id("different-token");
    }
    assert!(library.merge(&[wrong]).is_err());
    for position in 1..4 {
        let mut value = prepared(&p).clone();
        value.trial_id = id(&format!("trial-{position}"));
        value.token = id(&format!("token-{position}"));
        value.position = position;
        value.target_member_id = value.contrast.members[position / 2].id.clone();
        if position % 2 == 1 {
            value.option_member_ids.reverse();
        }
        let good = event(
            &format!("prepared-{position}"),
            200 + position as i64,
            Fact::SoundTrialPrepared(value),
        );
        if position == 3 {
            let mut bad = good.clone();
            if let Fact::SoundTrialPrepared(p) = &mut bad.fact {
                p.target_member_id = p.contrast.members[0].id.clone();
            }
            assert!(library.merge(&[bad]).is_err());
        }
        library.merge(&[good]).unwrap();
    }
    assert_eq!(library.sound_trials().len(), 4);
    let mut changed = p.clone();
    changed.id = id("changed-definition");
    if let Fact::SoundTrialPrepared(p) = &mut changed.fact {
        p.trial_id = id("new-definition-trial");
        p.token = id("new-definition-token");
        p.block_id = id("another-block");
        p.contrast.detail_note = "Changed without a version bump.".into();
    }
    assert!(library.merge(&[changed]).is_err());
}

#[test]
fn sound_core_late_preanswer_replays_and_support_requalify_without_rewriting_cues() {
    let (mut base, p) = fixture();
    base.push(p.clone());
    let mut library = merged(base, listening_packet(&p, 0, "initial"));
    let attempt = library.sound_trial_completion(prepared(&p).trial_id)[0]
        .1
        .attempt;
    let after = play(&p, "after", 400, 0, true);
    library.merge(&[after.0, after.1]).unwrap();
    assert!(!library.sound_trials()[0].assisted);
    let before = play(&p, "before", 250, 0, true);
    library.merge(&[before.0, before.1]).unwrap();
    assert!(library.sound_trials()[0].assisted);
    assert!(library.attempt(attempt).unwrap().cues.is_empty());
    let (mut base, p) = fixture();
    base.push(p.clone());
    let mut library = merged(base, listening_packet(&p, 0, "support"));
    let support = event(
        "visible-reference",
        260,
        Fact::StimulusPresented(StimulusPresentation {
            session: Some(prepared(&p).trial_id),
            revision: Some(prepared(&p).references[0].revision),
            russian: prepared(&p).references[0].source_text.clone(),
            purpose: PresentationPurpose::Reference,
            decision: None,
            utterance_family: Some(prepared(&p).contrast.id.clone()),
        }),
    );
    library.merge(&[support]).unwrap();
    assert!(library.sound_trials()[0].assisted);
}

#[test]
fn sound_core_multiple_speaking_intents_preserve_unsubmitted_takes_and_old_meanings() {
    let (base, p) = fixture();
    let a = speaking_prepared(&p, "a", 200, 0);
    let b = speaking_prepared(&p, "b", 300, 1);
    let c = speaking_prepared(&p, "c", 500, 0);
    let mut library = merged(base, vec![a.clone(), b]);
    let unsubmitted = speaking_packet(&a, "unsubmitted", 400, "c").remove(0);
    library.merge(&[unsubmitted, c]).unwrap();
    assert_eq!(library.sound_trials().len(), 3);
    assert_eq!(library.attempts().count(), 0);
    assert!(
        library
            .sound_trials()
            .iter()
            .all(|summary| summary.incomplete)
    );
}

#[test]
fn sound_core_speaking_binds_intent_capture_scope_actual_attempt_and_exact_original_copy() {
    let (base, p) = fixture();
    let speaking = speaking_prepared(&p, "good", 200, 0);
    let packet = speaking_packet(&speaking, "good", 400, "c");
    let library = merged(
        base.into_iter().chain([speaking.clone()]).collect(),
        packet.clone(),
    );
    let summary = &library.sound_trials()[0];
    assert!(!summary.incomplete);
    assert_eq!(summary.intended_match, None);
    assert_eq!(summary.choice_member_id, None);
    assert!(!summary.assisted);
    for case in [
        "scope",
        "time",
        "original-time",
        "copy",
        "attempt",
        "context",
    ] {
        let (base, p) = fixture();
        let s = speaking_prepared(&p, "good", 200, 0);
        let mut changed = packet.clone();
        match case {
            "scope" => {
                if let Fact::SoundTrialCompleted(c) = &mut changed[4].fact {
                    if let SoundTrialResponse::Speaking { capture_scope, .. } = &mut c.response {
                        capture_scope.intent_id = id("stale-intent");
                    }
                }
            }
            "time" => {
                if let Fact::SoundTrialCompleted(c) = &mut changed[4].fact {
                    if let SoundTrialResponse::Speaking { captured_at_ms, .. } = &mut c.response {
                        *captured_at_ms = 199;
                    }
                }
            }
            "original-time" => changed[0].recorded_at_ms = 399,
            "copy" => {
                if let Fact::RecordingAttached(a) = &mut changed[1].fact {
                    a.asset.sha256 = "d".repeat(64);
                }
            }
            "attempt" => {
                if let Fact::AttemptRecorded(a) = &mut changed[2].fact {
                    a.session = id("different-session");
                }
            }
            "context" => {
                if let Fact::AttemptContextRecorded(c) = &mut changed[3].fact {
                    c.target_features.insert("unrelated-target".into());
                }
            }
            _ => unreachable!(),
        }
        let mut lib = Library::default();
        assert!(
            lib.merge(
                &base
                    .into_iter()
                    .chain([s])
                    .chain(changed)
                    .collect::<Vec<_>>()
            )
            .is_err(),
            "{case}"
        );
    }
}

#[test]
fn sound_core_original_sha_cannot_be_submitted_under_another_attempt() {
    let (base, p) = fixture();
    let a = speaking_prepared(&p, "first", 200, 0);
    let first = speaking_packet(&a, "first", 400, "c");
    let mut library = merged(base.into_iter().chain([a]).collect(), first);
    let b = speaking_prepared(&p, "second", 600, 1);
    let second = speaking_packet(&b, "second", 800, "c");
    let before = library.clone();
    assert!(library.merge(&[vec![b], second].concat()).is_err());
    assert_eq!(library, before);
}

#[test]
fn sound_core_concurrent_offline_responses_are_preserved_without_a_winner() {
    let (mut base, p) = fixture();
    base.push(p.clone());
    let left = listening_packet(&p, 0, "offline-a");
    let right = listening_packet(&p, 1, "offline-b");
    let library = merged(base, left.into_iter().chain(right).collect());
    let summary = &library.sound_trials()[0];
    assert_eq!(library.attempts().count(), 2);
    assert_eq!(
        library.sound_trial_completion(prepared(&p).trial_id).len(),
        2
    );
    assert!(summary.incomplete);
    assert_eq!(summary.choice_member_id, None);
    assert_eq!(summary.intended_match, None);
}

#[test]
fn sound_core_playback_acknowledgements_and_generated_provenance_cannot_cross_sources() {
    let (mut base, p) = fixture();
    base.push(p.clone());
    let packet = listening_packet(&p, 0, "source");
    for case in [
        "token",
        "ended-rate",
        "ended-start",
        "source-text",
        "acoustic-claim",
    ] {
        let mut prepared = p.clone();
        let mut events = packet.clone();
        let wrong_start = events[2].id;
        match case {
            "token" => {
                if let Fact::SoundReferencePlayed(p) = &mut events[0].fact {
                    p.token = id("wrong-token");
                }
            }
            "ended-rate" => {
                if let Fact::SoundReferencePlayed(p) = &mut events[1].fact {
                    p.natural_speed = false;
                }
            }
            "ended-start" => {
                if let Fact::SoundReferencePlayed(p) = &mut events[1].fact {
                    p.stage = SoundPlaybackStage::Completed {
                        started_event: wrong_start,
                    };
                }
            }
            "source-text" => {
                if let Fact::SoundTrialPrepared(p) = &mut prepared.fact {
                    p.references[0].source_text = "Другие слова.".into();
                }
            }
            "acoustic-claim" => {
                if let Fact::SoundTrialPrepared(p) = &mut prepared.fact {
                    let mut audit: serde_json::Value =
                        serde_json::from_str(&p.references[0].provenance_json).unwrap();
                    audit["acoustic_assessment"] = json!({"score":100});
                    p.references[0].provenance_json = audit.to_string();
                }
            }
            _ => unreachable!(),
        }
        let mut library = Library::default();
        let refs = base.iter().filter(|e| e.id != p.id).cloned();
        assert!(
            library
                .merge(&refs.chain([prepared]).chain(events).collect::<Vec<_>>())
                .is_err(),
            "{case}"
        );
    }
}
