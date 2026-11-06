//! Frozen sound trials and exact response recovery. Synthetic metadata; no inference.
#![cfg(not(target_arch = "wasm32"))]
#![allow(dead_code)]
#[path = "../src/daily_plan.rs"]
mod daily_plan;
#[path = "../src/drafts.rs"]
mod drafts;
#[path = "../src/editor_drafts.rs"]
mod editor_drafts;
#[path = "../src/exposure.rs"]
mod exposure;
#[path = "../src/learning.rs"]
mod learning;
#[path = "../src/model.rs"]
mod model;
#[path = "../src/native_connection.rs"]
mod native_connection;
#[path = "../src/native_http.rs"]
mod native_http;
#[path = "../src/recognition.rs"]
mod recognition;
#[path = "../src/repository.rs"]
mod repository;
#[path = "../src/sound_focus.rs"]
mod sound_focus;
#[path = "../src/teacher.rs"]
mod teacher;
#[path = "../src/teacher_audio_practice.rs"]
mod teacher_audio_practice;
#[path = "../src/teacher_context.rs"]
mod teacher_context;
#[path = "../src/teacher_notebook.rs"]
mod teacher_notebook;
#[path = "../src/teacher_practice.rs"]
mod teacher_practice;
#[path = "../src/teacher_revisit.rs"]
mod teacher_revisit;
use model::AppState;
use repository::{Repository, now_ms};
use rufren_core::*;
use serde_json::json;
use sound_focus::{SoundFocusDraft, SoundFocusPhase};
use std::rc::Rc;
use uuid::Uuid;
fn app() -> AppState {
    AppState::from_repository(Rc::new(Repository::open_at(":memory:").unwrap())).unwrap()
}
fn models(
    s: &mut AppState,
    which: usize,
) -> (
    rufren_content::SoundContrastDefinition,
    [AudioRecordMetadata; 2],
    [SoundReferenceBinding; 2],
) {
    let definition = rufren_content::sound_contrasts()[which].clone();
    let mut metas = Vec::new();
    let mut bindings = Vec::new();
    for (i, m) in definition.members.iter().enumerate() {
        let (passage, revision) = sound_reference_identity(&m.utterance.id, &m.utterance.version);
        let metadata = AudioRecordMetadata {
            asset_id: format!("synthetic-sound-model-{}", Uuid::new_v4()),
            content_hash: if i == 0 {
                "a".repeat(64)
            } else {
                "b".repeat(64)
            },
            role: "reference".into(),
            name: "Must stay hidden during the blind trial".into(),
            passage_id: passage.to_string(),
            revision_id: revision.to_string(),
            duration_ms: 1000,
            sample_rate: 24000,
            channels: 1,
            frames: 24000,
            byte_length: 48044,
            created_at_ms: 1,
            mime_type: "audio/wav".into(),
            interrupted: false,
            device: json!({"provenance":{"kind":"ai_generated_speech","provider":"public-fixture","model":"public-fixture","voice":"public-fixture","request_id":"public-fixture","source_text":m.utterance.text,"source_text_sha256":dialogue_text_sha256(&m.utterance.text)}}),
            annotations: vec![],
        };
        bindings.push(
            s.register_sound_focus_reference(&definition, &m.id, &metadata)
                .unwrap(),
        );
        metas.push(metadata);
    }
    (
        definition,
        metas.try_into().unwrap(),
        bindings.try_into().unwrap(),
    )
}
fn started(s: &mut AppState) -> (SoundFocusDraft, [AudioRecordMetadata; 2]) {
    let (definition, metadata, bindings) = models(s, 0);
    let draft = s.start_sound_focus(&definition, bindings).unwrap();
    (draft, metadata)
}
fn listening(s: &mut AppState, meta: &[AudioRecordMetadata; 2]) {
    let draft = s.sound_focus_draft().unwrap().unwrap();
    for i in 0..2 {
        let member = draft.contrast.members[i].id.clone();
        let id = s
            .sound_focus_example_started(draft.session_id, &member, &meta[i])
            .unwrap();
        s.sound_focus_example_completed(draft.session_id, &member, id, &meta[i])
            .unwrap();
    }
    s.sound_focus_continue(draft.session_id).unwrap();
}
fn play_trial(s: &mut AppState, meta: &[AudioRecordMetadata; 2]) -> (SoundFocusDraft, Uuid) {
    let draft = s.sound_focus_draft().unwrap().unwrap();
    let trial = &draft.trials[draft.current_trial];
    let i = draft
        .contrast
        .members
        .iter()
        .position(|m| m.id == trial.target_member_id)
        .unwrap();
    let start = s
        .sound_focus_playback_started(draft.session_id, trial.token, &meta[i])
        .unwrap();
    s.sound_focus_playback_completed(draft.session_id, trial.token, start, &meta[i])
        .unwrap();
    (s.sound_focus_draft().unwrap().unwrap(), start)
}
fn speaking(s: &mut AppState, meta: &[AudioRecordMetadata; 2]) {
    listening(s, meta);
    for _ in 0..4 {
        let (d, _) = play_trial(s, meta);
        let trial = &d.trials[d.current_trial];
        s.sound_focus_answer(d.session_id, trial.token, &trial.option_member_ids[0])
            .unwrap();
        s.sound_focus_next(d.session_id).unwrap();
    }
    assert_eq!(
        s.sound_focus_draft().unwrap().unwrap().phase,
        SoundFocusPhase::Speaking
    );
}
fn take(draft: &SoundFocusDraft, hash: &str) -> AudioRecordMetadata {
    let intent = draft.speaking.as_ref().unwrap();
    AudioRecordMetadata {
        asset_id: format!("synthetic-take-{}", Uuid::new_v4()),
        content_hash: hash.repeat(64),
        role: "attempt".into(),
        name: "Public synthetic microphone take".into(),
        passage_id: intent.passage_id.to_string(),
        revision_id: intent.revision_id.to_string(),
        duration_ms: 100,
        sample_rate: 24000,
        channels: 1,
        frames: 2400,
        byte_length: 4844,
        created_at_ms: now_ms(),
        mime_type: "audio/wav".into(),
        interrupted: false,
        device: json!({"sound_focus_scope":{"session_id":draft.session_id,"intent_id":intent.intent_id,"member_id":intent.member_id,"origin":"microphone_capture"}}),
        annotations: vec![],
    }
}
struct Disk(std::path::PathBuf);
impl Disk {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!("rufren-sound-focus-{}.sqlite", Uuid::new_v4())))
    }
    fn app(&self) -> AppState {
        AppState::from_repository(Rc::new(Repository::open_at(&self.0).unwrap())).unwrap()
    }
    fn sql(&self, sql: &str) {
        rusqlite::Connection::open(&self.0)
            .unwrap()
            .execute_batch(sql)
            .unwrap();
    }
}
impl Drop for Disk {
    fn drop(&mut self) {
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{suffix}", self.0.display()));
        }
    }
}

#[test]
fn sound_app_prepared_block_balances_members_positions_without_creating_practice() {
    let disk = Disk::new();
    let mut s = disk.app();
    let (d, _) = started(&mut s);
    assert_eq!(d.phase, SoundFocusPhase::Examples);
    let cells: std::collections::BTreeSet<_> = d
        .trials
        .iter()
        .map(|t| (t.target_member_id.clone(), t.option_member_ids[0].clone()))
        .collect();
    assert_eq!(cells.len(), 4);
    assert_eq!(s.library.attempts().count(), 0);
    assert!(
        SpacedReview
            .queue(&s.library, now_ms(), &Skill::ALL)
            .is_empty()
    );
    let exact: Vec<_> = d
        .trials
        .iter()
        .map(|t| {
            (
                t.trial_id,
                t.token,
                t.option_member_ids.clone(),
                t.target_member_id.clone(),
            )
        })
        .collect();
    drop(s);
    let s = disk.app();
    let restored = s.sound_focus_draft().unwrap().unwrap();
    assert_eq!(restored.session_id, d.session_id);
    assert_eq!(
        restored
            .trials
            .iter()
            .map(|t| (
                t.trial_id,
                t.token,
                t.option_member_ids.clone(),
                t.target_member_id.clone()
            ))
            .collect::<Vec<_>>(),
        exact
    );
}

#[test]
fn sound_app_start_is_not_end_wrong_source_and_stale_token_do_not_unlock_choice() {
    let mut s = app();
    let (_, meta) = started(&mut s);
    listening(&mut s, &meta);
    let d = s.sound_focus_draft().unwrap().unwrap();
    let t = &d.trials[0];
    let i = d
        .contrast
        .members
        .iter()
        .position(|m| m.id == t.target_member_id)
        .unwrap();
    assert!(
        s.sound_focus_answer(d.session_id, t.token, &t.option_member_ids[0])
            .is_err()
    );
    let start = s
        .sound_focus_playback_started(d.session_id, t.token, &meta[i])
        .unwrap();
    assert!(!s.sound_focus_draft().unwrap().unwrap().trials[0].can_answer);
    assert!(
        s.sound_focus_playback_completed(d.session_id, t.token, start, &meta[1 - i])
            .is_err()
    );
    assert!(
        s.sound_focus_playback_completed(d.session_id, Uuid::new_v4(), start, &meta[i])
            .is_err()
    );
    s.sound_focus_playback_completed(d.session_id, t.token, start, &meta[i])
        .unwrap();
    assert!(s.sound_focus_draft().unwrap().unwrap().trials[0].can_answer);
    let attempt = s
        .sound_focus_answer(d.session_id, t.token, &t.option_member_ids[0])
        .unwrap();
    assert_eq!(
        s.library.attempt(attempt).unwrap().self_rating,
        SelfRating::NotReported
    );
    assert_eq!(
        s.library.contexts(attempt)[0].1.task,
        TaskKind::SoundDiscrimination
    );
    s.sound_focus_next(d.session_id).unwrap();
    assert!(
        s.sound_focus_playback_completed(d.session_id, t.token, start, &meta[i])
            .is_err()
    );
    assert!(!s.sound_focus_draft().unwrap().unwrap().trials[1].can_answer);
}

#[test]
fn sound_app_failed_response_commit_freezes_choice_and_retries_same_actual_packet_after_restart() {
    let disk = Disk::new();
    let mut s = disk.app();
    let (_, meta) = started(&mut s);
    listening(&mut s, &meta);
    let (d, _) = play_trial(&mut s, &meta);
    let t = &d.trials[0];
    disk.sql("CREATE TRIGGER fail_response BEFORE INSERT ON events WHEN json_extract(NEW.json,'$.fact.kind')='sound_trial_completed' BEGIN SELECT RAISE(ABORT,'public synthetic response failure'); END;");
    assert!(
        s.sound_focus_answer(d.session_id, t.token, &t.option_member_ids[0])
            .is_err()
    );
    assert_eq!(s.library.attempts().count(), 0);
    let packet: serde_json::Value = s.repo.get("sound_focus_active").unwrap().unwrap();
    let before = packet["response_packets"][t.trial_id.to_string()].clone();
    assert!(!before.is_null());
    assert!(
        s.sound_focus_answer(d.session_id, t.token, &t.option_member_ids[1])
            .is_err()
    );
    disk.sql("DROP TRIGGER fail_response;");
    drop(s);
    let mut s = disk.app();
    let actual = s
        .sound_focus_answer(d.session_id, t.token, &t.option_member_ids[0])
        .unwrap();
    let again = s
        .sound_focus_answer(d.session_id, t.token, &t.option_member_ids[0])
        .unwrap();
    assert_eq!(actual, again);
    assert_eq!(s.library.attempts().count(), 1);
    let after: serde_json::Value = s.repo.get("sound_focus_active").unwrap().unwrap();
    assert_eq!(before, after["response_packets"][t.trial_id.to_string()]);
    assert!(
        s.library
            .learner_snapshot(now_ms())
            .features
            .iter()
            .all(|f| f.judgments.is_empty())
    );
}

#[test]
fn sound_app_replay_reveal_and_reference_navigation_are_retained_as_support() {
    let mut s = app();
    let (_, meta) = started(&mut s);
    listening(&mut s, &meta);
    let (d, _) = play_trial(&mut s, &meta);
    let t = &d.trials[0];
    let i = d
        .contrast
        .members
        .iter()
        .position(|m| m.id == t.target_member_id)
        .unwrap();
    s.sound_focus_playback_started(d.session_id, t.token, &meta[i])
        .unwrap();
    s.sound_focus_reveal(d.session_id, t.token).unwrap();
    s.sound_focus_mark_active_support(Cue::ReferenceMaterial)
        .unwrap();
    let id = s
        .sound_focus_answer(d.session_id, t.token, &t.option_member_ids[0])
        .unwrap();
    let a = s.library.attempt(id).unwrap();
    for cue in [
        Cue::Replay,
        Cue::OriginalText,
        Cue::Translation,
        Cue::ReferenceMaterial,
    ] {
        assert!(a.cues.contains(&cue));
    }
    assert!(
        s.library
            .sound_trials()
            .iter()
            .find(|r| r.trial_id == t.trial_id)
            .unwrap()
            .assisted
    );
}

#[test]
fn sound_app_speaking_intent_changes_scope_old_takes_cannot_be_relabelled_and_keep_is_idempotent() {
    let mut s = app();
    let (d, meta) = started(&mut s);
    speaking(&mut s, &meta);
    let member0 = d.contrast.members[0].id.clone();
    let member1 = d.contrast.members[1].id.clone();
    s.sound_focus_intend(d.session_id, &member0).unwrap();
    let first = s.sound_focus_draft().unwrap().unwrap();
    let old_intent = first.speaking.as_ref().unwrap().intent_id;
    let old = take(&first, "c");
    let old_attachment = s
        .register_sound_focus_take(d.session_id, old_intent, &old)
        .unwrap();
    s.sound_focus_select_take(
        d.session_id,
        old_intent,
        Some((old_attachment, old.clone())),
        false,
    )
    .unwrap();
    s.sound_focus_intend(d.session_id, &member1).unwrap();
    let next = s.sound_focus_draft().unwrap().unwrap();
    let intent = next.speaking.as_ref().unwrap();
    assert_ne!(intent.intent_id, old_intent);
    assert!(intent.selected_attachment.is_none());
    assert!(
        s.sound_focus_select_take(
            d.session_id,
            intent.intent_id,
            Some((old_attachment, old)),
            true
        )
        .is_err()
    );
    let fresh = take(&next, "d");
    let attachment = s
        .register_sound_focus_take(d.session_id, intent.intent_id, &fresh)
        .unwrap();
    s.sound_focus_select_take(
        d.session_id,
        intent.intent_id,
        Some((attachment, fresh.clone())),
        false,
    )
    .unwrap();
    let attempt = s
        .sound_focus_keep_take(d.session_id, intent.intent_id)
        .unwrap();
    assert_eq!(
        s.sound_focus_keep_take(d.session_id, intent.intent_id)
            .unwrap(),
        attempt
    );
    assert_eq!(s.library.attempt(attempt).unwrap().skill, Skill::Speaking);
    assert_eq!(
        s.library.contexts(attempt)[0].1.task,
        TaskKind::Pronunciation
    );
    assert_eq!(
        s.library.recordings(AnnotationTarget::Attempt(attempt))[0]
            .1
            .asset
            .sha256,
        fresh.content_hash
    );
    s.finish_sound_focus(d.session_id).unwrap();
    assert_eq!(s.library.attempts().count(), 5);
    assert!(
        SpacedReview
            .queue(&s.library, now_ms(), &Skill::ALL)
            .is_empty()
    );
}

#[test]
fn sound_app_empty_client_reopens_fact_only_completed_or_partial_history_without_new_attempts() {
    let mut source = app();
    let (d, meta) = started(&mut source);
    listening(&mut source, &meta);
    let (current, _) = play_trial(&mut source, &meta);
    let t = &current.trials[0];
    source
        .sound_focus_answer(d.session_id, t.token, &t.option_member_ids[0])
        .unwrap();
    let raw = repository::export_library(source.library.clone()).unwrap();
    let library = repository::import_library(&raw).unwrap();
    let mut restored = app();
    restored
        .repo
        .append(&library.events.values().cloned().collect::<Vec<_>>())
        .unwrap();
    restored.refresh().unwrap();
    assert!(restored.sound_focus_draft().unwrap().is_none());
    restored.open_sound_focus_history(d.session_id).unwrap();
    let history = restored.sound_focus_draft().unwrap().unwrap();
    assert!(history.historical);
    assert_eq!(history.trials[0].option_member_ids, t.option_member_ids);
    assert_eq!(
        history.trials[0].choice_member_id,
        Some(t.option_member_ids[0].clone())
    );
    assert_eq!(restored.library.attempts().count(), 1);
    assert!(
        restored
            .sound_focus_answer(d.session_id, t.token, &t.option_member_ids[0])
            .is_err()
    );
    let one = source.library.events.get(&t.prepared_id).unwrap().clone();
    let mut partial = app();
    partial.repo.append(&[one]).unwrap();
    partial.refresh().unwrap();
    partial.open_sound_focus_history(d.session_id).unwrap();
    let h = partial.sound_focus_draft().unwrap().unwrap();
    assert!(h.historical && h.history_incomplete);
    assert_eq!(h.trials.len(), 1);
    assert!(h.trials[0].saved_attempt.is_none());
    assert_eq!(partial.library.attempts().count(), 0);
}

#[test]
fn sound_app_history_archives_unfinished_scope_and_resume_preserves_order_and_marks_reference_help()
{
    let mut s = app();
    let (first, meta) = started(&mut s);
    listening(&mut s, &meta);
    let order = first.trials.iter().map(|t| t.token).collect::<Vec<_>>();
    let (other, _, bindings) = models(&mut s, 1);
    let second = s.start_sound_focus(&other, bindings).unwrap();
    s.open_sound_focus_history(second.session_id).unwrap();
    assert!(
        s.paused_sound_focus()
            .unwrap()
            .iter()
            .any(|(id, _)| *id == first.session_id)
    );
    s.resume_sound_focus(first.session_id).unwrap();
    let resumed = s.sound_focus_draft().unwrap().unwrap();
    assert!(!resumed.historical);
    assert_eq!(
        resumed.trials.iter().map(|t| t.token).collect::<Vec<_>>(),
        order
    );
    let (d, _) = play_trial(&mut s, &meta);
    let t = &d.trials[0];
    let id = s
        .sound_focus_answer(d.session_id, t.token, &t.option_member_ids[0])
        .unwrap();
    assert!(
        s.library
            .attempt(id)
            .unwrap()
            .cues
            .contains(&Cue::ReferenceMaterial)
    );
}

#[test]
fn sound_app_surface_changes_preserve_ordinary_and_daily_plan_work() {
    let mut s = app();
    let revision = starter(s.device, now_ms() - 5 * DAY_MS);
    let attempt = Event::new(
        s.device,
        now_ms() - 4 * DAY_MS,
        Fact::AttemptRecorded(Attempt {
            session: Uuid::new_v4(),
            revision: revision.id,
            skill: Skill::Writing,
            response: "Public earlier answer".into(),
            cues: vec![],
            self_rating: SelfRating::Recalled,
            policy: "free-practice-v1".into(),
        }),
    )
    .unwrap();
    s.repo.append(&[revision, attempt]).unwrap();
    s.refresh().unwrap();
    s.prepare_daily_plan(now_ms()).unwrap();
    s.start_daily_plan_item(0, now_ms()).unwrap();
    let id = s.session.as_ref().unwrap().id;
    let mut ordinary = s.session.clone().unwrap();
    ordinary.draft = "Unrelated public draft".into();
    s.repo.set("session", &Some(&ordinary)).unwrap();
    s.session = Some(ordinary);
    let plan: serde_json::Value = s.repo.get("daily_plan_active").unwrap().unwrap();
    let (sound, _) = started(&mut s);
    assert!(s.sound_focus_is_selected().unwrap());
    s.pause_sound_focus().unwrap();
    assert!(!s.sound_focus_is_selected().unwrap());
    s.begin().unwrap();
    assert_eq!(s.session.as_ref().unwrap().id, id);
    assert_eq!(s.session.as_ref().unwrap().draft, "Unrelated public draft");
    assert_eq!(
        s.repo
            .get::<serde_json::Value>("daily_plan_active")
            .unwrap()
            .unwrap(),
        plan
    );
    s.resume_sound_focus(sound.session_id).unwrap();
    assert_eq!(
        s.sound_focus_draft().unwrap().unwrap().session_id,
        sound.session_id
    );
}

#[test]
fn sound_app_same_block_history_cannot_replace_or_hide_its_unfinished_workspace() {
    let mut s = app();
    let (initial, meta) = started(&mut s);
    listening(&mut s, &meta);
    let (before, _) = play_trial(&mut s, &meta);
    s.open_sound_focus_history(initial.session_id).unwrap();
    assert!(s.sound_focus_draft().unwrap().unwrap().historical);
    assert!(
        s.paused_sound_focus()
            .unwrap()
            .iter()
            .any(|(id, _)| *id == initial.session_id)
    );
    s.resume_sound_focus(initial.session_id).unwrap();
    let resumed = s.sound_focus_draft().unwrap().unwrap();
    assert!(!resumed.historical);
    assert_eq!(resumed.phase, SoundFocusPhase::Listening);
    assert_eq!(
        resumed.trials.iter().map(|t| t.token).collect::<Vec<_>>(),
        before.trials.iter().map(|t| t.token).collect::<Vec<_>>()
    );
    assert!(resumed.trials[0].can_answer);
    let trial = &resumed.trials[0];
    let attempt = s
        .sound_focus_answer(initial.session_id, trial.token, &trial.option_member_ids[0])
        .unwrap();
    assert!(
        s.library
            .attempt(attempt)
            .unwrap()
            .cues
            .contains(&Cue::ReferenceMaterial)
    );
    // Leaving a historical view must likewise preserve another paused editable block.
    let (other, _, bindings) = models(&mut s, 1);
    let b = s.start_sound_focus(&other, bindings).unwrap();
    s.open_sound_focus_history(b.session_id).unwrap();
    s.resume_sound_focus(initial.session_id).unwrap();
    assert!(
        s.paused_sound_focus()
            .unwrap()
            .iter()
            .any(|(id, _)| *id == b.session_id)
    );
    s.resume_sound_focus(b.session_id).unwrap();
    let restored_b = s.sound_focus_draft().unwrap().unwrap();
    assert!(!restored_b.historical);
    assert_eq!(restored_b.phase, SoundFocusPhase::Examples);
    assert_eq!(
        restored_b
            .trials
            .iter()
            .map(|t| t.token)
            .collect::<Vec<_>>(),
        b.trials.iter().map(|t| t.token).collect::<Vec<_>>()
    );
}

#[test]
fn sound_app_kept_or_queued_speaking_response_blocks_intent_change_without_orphaning_packet() {
    for fail_commit in [false, true] {
        let disk = Disk::new();
        let mut s = disk.app();
        let (d, meta) = started(&mut s);
        speaking(&mut s, &meta);
        let first = d.contrast.members[0].id.clone();
        let other = d.contrast.members[1].id.clone();
        s.sound_focus_intend(d.session_id, &first).unwrap();
        let intent = s
            .sound_focus_draft()
            .unwrap()
            .unwrap()
            .speaking
            .unwrap()
            .intent_id;
        let original = take(&s.sound_focus_draft().unwrap().unwrap(), "e");
        let attachment = s
            .register_sound_focus_take(d.session_id, intent, &original)
            .unwrap();
        s.sound_focus_select_take(d.session_id, intent, Some((attachment, original)), false)
            .unwrap();
        if fail_commit {
            disk.sql("CREATE TRIGGER fail_frozen_speaking BEFORE INSERT ON events WHEN json_extract(NEW.json,'$.fact.kind')='sound_trial_completed' BEGIN SELECT RAISE(ABORT,'public synthetic queued take failure'); END;");
            assert!(s.sound_focus_keep_take(d.session_id, intent).is_err());
        } else {
            s.sound_focus_keep_take(d.session_id, intent).unwrap();
        }
        let before: serde_json::Value = s.repo.get("sound_focus_active").unwrap().unwrap();
        let facts = s.library.clone();
        for meaning in [&first, &other] {
            let error = s.sound_focus_intend(d.session_id, meaning).unwrap_err();
            assert!(error.contains("new Sound practice"));
            assert_eq!(
                s.repo
                    .get::<serde_json::Value>("sound_focus_active")
                    .unwrap()
                    .unwrap(),
                before
            );
            assert_eq!(s.library, facts);
        }
        if fail_commit {
            disk.sql("DROP TRIGGER fail_frozen_speaking;");
            let attempt = s.sound_focus_keep_take(d.session_id, intent).unwrap();
            assert_eq!(s.library.attempt(attempt).unwrap().skill, Skill::Speaking);
        }
    }
}
