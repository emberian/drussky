//! Pure native-boundary tests. Opaque bytes here are transport fixtures, not a
//! claim that a test host parsed a PKDrawing or exercised a physical Pencil.
#[path = "../src/handwriting_native.rs"]
mod native;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use native::*;
use sha2::{Digest, Sha256};
use uuid::Uuid;
fn sha(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn paper() -> NativePaper {
    NativePaper {
        width_points: 1024.0,
        height_points: 768.0,
        line_height_points: 80.0,
    }
}
fn point(time: f64) -> NativeInkPoint {
    NativeInkPoint {
        // Nontrivial coordinates that drift with serde_json's default parser.
        x: 51.248178375505404,
        y: 52.314008204106244,
        time_offset_seconds: time,
        width: 5.0,
        height: 5.0,
        force: 2.7,
        opacity: 1.0,
        azimuth_radians: 0.5,
        altitude_radians: 1.2,
    }
}
fn projection() -> NativeInkProjection {
    NativeInkProjection {
        format: "pencilkit-control-points-v1".into(),
        paper: paper(),
        strokes: vec![NativeInkStroke {
            creation_unix_ms: 1000.0,
            transform: [1.0, 0.0, 0.0, 1.0, 42.0, 24.0],
            ink_type: "com.apple.ink.pen".into(),
            color_rgba: [0.1, 0.2, 0.3, 1.0],
            visible_ranges: vec![[0.0, 1.0]],
            points: vec![point(0.0), point(0.1)],
        }],
    }
}
fn checkpoint() -> NativeWritingCheckpoint {
    let archive = b"opaque native archive transport fixture";
    let canonical = projection();
    NativeWritingCheckpoint {
        version: 1,
        checkpoint_id: Uuid::new_v4(),
        instance_id: Uuid::new_v4(),
        scope_id: "activity.public.word".into(),
        attempt_id: Uuid::new_v4(),
        task_id: "word.cat".into(),
        task_revision: "1".into(),
        sequence: 3,
        captured_at_ms: 1000,
        archive_base64: STANDARD.encode(archive),
        archive_sha256: sha(archive),
        canonical_sha256: sha(&serde_json::to_vec(&canonical).unwrap()),
        canonical,
        zoom_scale: 1.25,
        content_offset: [50.0, 0.0],
    }
}
fn request(c: &NativeWritingCheckpoint) -> NativeWritingRequest {
    NativeWritingRequest {
        instance_id: Uuid::new_v4(),
        scope_id: c.scope_id.clone(),
        attempt_id: c.attempt_id,
        task_id: c.task_id.clone(),
        task_revision: c.task_revision.clone(),
        prompt: "Hear the whole word, then write it.".into(),
        meaning: Some("cat".into()),
        model_text: "кот".into(),
        comparison_hint: "Compare all three letters with the printed example.".into(),
        model_visible: false,
        allow_model_reveal: true,
        allow_listen: true,
        paper: paper(),
        audio: None,
        initial: Some(c.clone()),
        initial_compared: false,
    }
}
#[test]
fn native_ink_projection_retains_processed_force_timing_transform_and_mask_coordinates() {
    let p = projection();
    p.validate().unwrap();
    let json = serde_json::to_vec(&p).unwrap();
    let decoded: NativeInkProjection = serde_json::from_slice(&json).unwrap();
    assert_eq!(decoded, p);
    assert_eq!(decoded.strokes[0].points[0].force, 2.7);
    assert_eq!(decoded.strokes[0].transform[4], 42.0);
    assert_eq!(decoded.strokes[0].visible_ranges, vec![[0.0, 1.0]]);
    assert!(String::from_utf8(json)
        .unwrap()
        .contains("pencilkit-control-points-v1"));
}
#[test]
fn native_checkpoint_binds_both_exact_archive_and_canonical_projection_hashes() {
    let c = checkpoint();
    c.validate().unwrap();
    assert_eq!(
        c.archive().unwrap(),
        b"opaque native archive transport fixture"
    );
    let mut wrong = c.clone();
    wrong.archive_base64 = STANDARD.encode(b"changed archive");
    assert!(wrong.validate().is_err());
    let mut wrong = c.clone();
    wrong.canonical.strokes[0].points[0].x += 1.0;
    assert!(wrong.validate().is_err());
    let json = serde_json::to_vec(&c).unwrap();
    let restored: NativeWritingCheckpoint = serde_json::from_slice(&json).unwrap();
    restored.validate().unwrap();
    assert_eq!(restored, c);
    assert_eq!(serde_json::to_vec(&restored).unwrap(), json);
}
#[test]
fn native_saved_paper_can_reopen_in_new_controller_but_not_different_task_or_attempt() {
    let c = checkpoint();
    let r = request(&c);
    assert_ne!(r.instance_id, c.instance_id);
    r.validate().unwrap();
    let mut wrong = r.clone();
    wrong.attempt_id = Uuid::new_v4();
    assert!(wrong.validate().is_err());
    let mut wrong = r.clone();
    wrong.task_revision = "2".into();
    assert!(wrong.validate().is_err());
    let mut wrong = r.clone();
    wrong.scope_id.push_str(".other");
    assert!(wrong.validate().is_err());
    let mut wrong = r;
    wrong.paper.width_points += 1.0;
    assert!(wrong.validate().is_err());
}
#[test]
fn native_ink_rejects_nonfinite_points_out_of_order_time_and_invalid_path_ranges() {
    let p = projection();
    let mut bad = p.clone();
    bad.strokes[0].points[0].x = f64::NAN;
    assert!(bad.validate().is_err());
    let mut bad = p.clone();
    bad.strokes[0].points[1].time_offset_seconds = -0.1;
    assert!(bad.validate().is_err());
    let mut bad = p.clone();
    bad.strokes[0].transform[1] = f64::INFINITY;
    assert!(bad.validate().is_err());
    let mut bad = p.clone();
    bad.strokes[0].visible_ranges[0][1] = 2.0;
    assert!(bad.validate().is_err());
    let mut bad = p;
    bad.strokes[0].points.clear();
    assert!(bad.validate().is_err());
    let mut bad = paper();
    bad.height_points = f64::INFINITY;
    assert!(bad.validate().is_err());
}
#[test]
fn native_audio_prompt_requires_exact_bound_wav_bytes_and_hash() {
    let c = checkpoint();
    let mut r = request(&c);
    let mut bytes = vec![0; 44];
    bytes[..4].copy_from_slice(b"RIFF");
    bytes[8..12].copy_from_slice(b"WAVE");
    r.audio = Some(NativeWritingAudio {
        id: "foundation.cat".into(),
        sha256: sha(&bytes),
        wav_bytes: bytes,
    });
    r.validate().unwrap();
    let mut wrong = r.clone();
    wrong.audio.as_mut().unwrap().wav_bytes[20] = 1;
    assert!(wrong.validate().is_err());
    let mut wrong = r;
    wrong.audio.as_mut().unwrap().wav_bytes.truncate(4);
    assert!(wrong.validate().is_err());
}
#[test]
fn native_checkpoint_missing_identity_sequence_and_hostile_archive_fail_before_opening() {
    let c = checkpoint();
    let mut bad = c.clone();
    bad.sequence = 0;
    assert!(bad.validate().is_err());
    let mut bad = c.clone();
    bad.checkpoint_id = Uuid::nil();
    assert!(bad.validate().is_err());
    let mut bad = c.clone();
    bad.archive_base64 = "!notbase64!".into();
    assert!(bad.validate().is_err());
    let mut bad = c;
    bad.zoom_scale = f64::NAN;
    assert!(bad.validate().is_err());
}
#[test]
fn native_comparison_resume_requires_exact_existing_original() {
    let c = checkpoint();
    let mut r = request(&c);
    r.initial_compared = true;
    r.validate().unwrap();
    r.initial = None;
    assert!(r.validate().is_err());
}
#[test]
fn native_checkpoint_cadence_batches_changes_but_explicit_intents_wait_only_for_tool_end() {
    assert_eq!(native_checkpoint_interval(true, false), 1.0);
    assert_eq!(native_checkpoint_interval(false, false), 0.25);
    assert_eq!(native_checkpoint_interval(true, true), 1.0);
    assert_eq!(native_checkpoint_interval(false, true), 0.0);
}
