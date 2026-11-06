//! Native paper is a renderer of a Rust-owned writing activity. Drawing checkpoints
//! contain PencilKit control points, never relabelled browser hardware samples.
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::rc::Rc;
use uuid::Uuid;

pub(crate) const MAX_NATIVE_ARCHIVE_BYTES: usize = 16 * 1024 * 1024;
const MAX_CANONICAL_BYTES: usize = 32 * 1024 * 1024;
pub(crate) type NativeWritingCallback = Rc<dyn Fn(NativeWritingEvent)>;
pub(crate) type NativeWritingResult<T> = Result<T, String>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NativePaper {
    pub width_points: f64,
    pub height_points: f64,
    pub line_height_points: f64,
}
impl NativePaper {
    pub(crate) fn validate(&self) -> NativeWritingResult<()> {
        if ![
            self.width_points,
            self.height_points,
            self.line_height_points,
        ]
        .iter()
        .all(|n| n.is_finite())
            || !(320.0..=8192.0).contains(&self.width_points)
            || !(200.0..=16384.0).contains(&self.height_points)
            || !(24.0..=512.0).contains(&self.line_height_points)
        {
            return Err("The native paper dimensions are invalid.".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NativeInkPoint {
    pub x: f64,
    pub y: f64,
    pub time_offset_seconds: f64,
    pub width: f64,
    pub height: f64,
    pub force: f64,
    pub opacity: f64,
    pub azimuth_radians: f64,
    pub altitude_radians: f64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NativeInkStroke {
    pub creation_unix_ms: f64,
    pub transform: [f64; 6],
    pub ink_type: String,
    pub color_rgba: [f64; 4],
    /// PencilKit path-parameter intervals that remain visible after masking.
    pub visible_ranges: Vec<[f64; 2]>,
    pub points: Vec<NativeInkPoint>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NativeInkProjection {
    pub format: String,
    pub paper: NativePaper,
    pub strokes: Vec<NativeInkStroke>,
}
impl NativeInkProjection {
    pub(crate) fn validate(&self) -> NativeWritingResult<()> {
        self.paper.validate()?;
        if self.format != "pencilkit-control-points-v1" || self.strokes.len() > 1000 {
            return Err("The native ink format or stroke count is unsupported.".into());
        }
        let mut total = 0usize;
        for s in &self.strokes {
            total = total
                .checked_add(s.points.len())
                .ok_or("The ink has too many points.")?;
            if total > 200_000
                || s.points.is_empty()
                || s.points.len() > 100_000
                || !s.creation_unix_ms.is_finite()
                || s.creation_unix_ms.abs() > 1.0e15
                || s.ink_type.is_empty()
                || s.ink_type.len() > 128
                || !s
                    .transform
                    .iter()
                    .all(|n| n.is_finite() && n.abs() <= 1.0e6)
                || !s
                    .color_rgba
                    .iter()
                    .all(|n| n.is_finite() && (0.0..=1.0).contains(n))
                || s.visible_ranges.len() > 10_000
            {
                return Err("The native stroke has invalid or excessive data.".into());
            }
            let last = s.points.len().saturating_sub(1) as f64;
            if s.visible_ranges.iter().any(|r| {
                !r.iter().all(|n| n.is_finite()) || r[0] < 0.0 || r[1] < r[0] || r[1] > last
            }) {
                return Err("The native stroke mask is invalid.".into());
            }
            let mut time = 0.0;
            for p in &s.points {
                if ![
                    p.x,
                    p.y,
                    p.time_offset_seconds,
                    p.width,
                    p.height,
                    p.force,
                    p.opacity,
                    p.azimuth_radians,
                    p.altitude_radians,
                ]
                .iter()
                .all(|n| n.is_finite())
                    || p.x.abs() > 1.0e6
                    || p.y.abs() > 1.0e6
                    || p.time_offset_seconds < time
                    || p.time_offset_seconds > 86400.0
                    || !(0.0..=4096.0).contains(&p.width)
                    || !(0.0..=4096.0).contains(&p.height)
                    || !(0.0..=10000.0).contains(&p.force)
                    || !(0.0..=1.0).contains(&p.opacity)
                    || p.azimuth_radians.abs() > std::f64::consts::TAU + 0.01
                    || p.altitude_radians.abs() > std::f64::consts::FRAC_PI_2 + 0.01
                {
                    return Err("The native control point cannot be read safely.".into());
                }
                time = p.time_offset_seconds;
            }
        }
        if serde_json::to_vec(self).map_err(|e| e.to_string())?.len() > MAX_CANONICAL_BYTES {
            return Err("The native ink projection is too large.".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NativeWritingCheckpoint {
    pub version: u32,
    pub checkpoint_id: Uuid,
    pub instance_id: Uuid,
    pub scope_id: String,
    pub attempt_id: Uuid,
    pub task_id: String,
    pub task_revision: String,
    pub sequence: u64,
    pub captured_at_ms: i64,
    pub archive_base64: String,
    pub archive_sha256: String,
    pub canonical: NativeInkProjection,
    pub canonical_sha256: String,
    pub zoom_scale: f64,
    pub content_offset: [f64; 2],
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn bounded(s: &str, max: usize) -> bool {
    !s.trim().is_empty()
        && s.len() <= max
        && !s
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
}
impl NativeWritingCheckpoint {
    pub(crate) fn archive(&self) -> NativeWritingResult<Vec<u8>> {
        if self.archive_base64.len() > MAX_NATIVE_ARCHIVE_BYTES.div_ceil(3) * 4 {
            return Err("The native drawing archive is too large.".into());
        }
        let bytes = STANDARD
            .decode(&self.archive_base64)
            .map_err(|_| "The native drawing archive is invalid.")?;
        if bytes.is_empty()
            || bytes.len() > MAX_NATIVE_ARCHIVE_BYTES
            || hash(&bytes) != self.archive_sha256
        {
            return Err("The native drawing archive does not match its saved hash.".into());
        }
        Ok(bytes)
    }
    pub(crate) fn validate(&self) -> NativeWritingResult<()> {
        if self.version != 1
            || self.checkpoint_id.is_nil()
            || self.instance_id.is_nil()
            || self.attempt_id.is_nil()
            || !bounded(&self.scope_id, 512)
            || !bounded(&self.task_id, 256)
            || !bounded(&self.task_revision, 128)
            || self.sequence == 0
            || !self.zoom_scale.is_finite()
            || !(0.1..=8.0).contains(&self.zoom_scale)
            || !self
                .content_offset
                .iter()
                .all(|v| v.is_finite() && v.abs() < 1.0e6)
        {
            return Err("The native drawing checkpoint identity or viewport is invalid.".into());
        }
        self.canonical.validate()?;
        if hash(&serde_json::to_vec(&self.canonical).map_err(|e| e.to_string())?)
            != self.canonical_sha256
        {
            return Err("The native ink does not match its saved hash.".into());
        }
        self.archive()?;
        Ok(())
    }
}
#[derive(Clone, Debug)]
pub(crate) struct NativeWritingAudio {
    pub id: String,
    pub sha256: String,
    pub wav_bytes: Vec<u8>,
}
#[derive(Clone, Debug)]
pub(crate) struct NativeWritingRequest {
    pub instance_id: Uuid,
    pub scope_id: String,
    pub attempt_id: Uuid,
    pub task_id: String,
    pub task_revision: String,
    pub prompt: String,
    pub meaning: Option<String>,
    pub model_text: String,
    pub comparison_hint: String,
    pub model_visible: bool,
    pub allow_model_reveal: bool,
    pub allow_listen: bool,
    pub paper: NativePaper,
    pub audio: Option<NativeWritingAudio>,
    pub initial: Option<NativeWritingCheckpoint>,
    /// Resume an already committed comparison without changing the original.
    pub initial_compared: bool,
}
impl NativeWritingRequest {
    pub(crate) fn validate(&self) -> NativeWritingResult<()> {
        self.paper.validate()?;
        if self.instance_id.is_nil()
            || self.attempt_id.is_nil()
            || !bounded(&self.scope_id, 512)
            || !bounded(&self.task_id, 256)
            || !bounded(&self.task_revision, 128)
            || !bounded(&self.prompt, 4096)
            || !bounded(&self.model_text, 4096)
            || !bounded(&self.comparison_hint, 512)
            || self.meaning.as_ref().is_some_and(|m| !bounded(m, 4096))
        {
            return Err("The native writing activity is missing its exact task binding.".into());
        }
        if let Some(a) = &self.audio {
            if !bounded(&a.id, 256)
                || a.wav_bytes.len() < 44
                || a.wav_bytes.len() > 8 * 1024 * 1024
                || &a.wav_bytes[..4] != b"RIFF"
                || &a.wav_bytes[8..12] != b"WAVE"
                || hash(&a.wav_bytes) != a.sha256
            {
                return Err("The spoken prompt does not match its exact WAV source.".into());
            }
        }
        if self.initial_compared
            && self
                .initial
                .as_ref()
                .is_none_or(|c| c.canonical.strokes.is_empty())
        {
            return Err("A comparison needs its exact saved original drawing.".into());
        }
        if let Some(c) = &self.initial {
            c.validate()?;
            if c.scope_id != self.scope_id
                || c.attempt_id != self.attempt_id
                || c.task_id != self.task_id
                || c.task_revision != self.task_revision
                || c.canonical.paper != self.paper
            {
                return Err("The saved drawing belongs to a different writing activity.".into());
            }
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum NativeCheckpointReason {
    Autosave,
    Compare,
    Back,
}
#[derive(Clone, Debug)]
pub(crate) enum NativeWritingEvent {
    Ready {
        instance_id: Uuid,
    },
    Checkpoint {
        reason: NativeCheckpointReason,
        checkpoint: NativeWritingCheckpoint,
    },
    CompareReady {
        instance_id: Uuid,
        checkpoint_id: Uuid,
        sequence: u64,
        archive_sha256: String,
    },
    ContinueToSpeaking {
        instance_id: Uuid,
        checkpoint_id: Uuid,
        sequence: u64,
    },
    Paused {
        instance_id: Uuid,
        checkpoint_id: Uuid,
        sequence: u64,
    },
    ModelRevealed {
        instance_id: Uuid,
        at_ms: i64,
    },
    AudioStarted {
        instance_id: Uuid,
        audio_id: String,
        sha256: String,
        at_ms: i64,
    },
    AudioCompleted {
        instance_id: Uuid,
        audio_id: String,
        sha256: String,
        at_ms: i64,
    },
    AudioStopped {
        instance_id: Uuid,
        audio_id: String,
        at_ms: i64,
    },
    Failure {
        instance_id: Uuid,
        message: String,
    },
    Closed {
        instance_id: Uuid,
    },
}
#[cfg(all(target_os = "ios", feature = "mobile"))]
#[path = "handwriting_native_ios.rs"]
mod ios;
#[cfg(all(target_os = "ios", feature = "mobile"))]
pub(crate) use ios::{ack, is_open, native_drawing_preview, open_native, request_pause};
pub(crate) const fn supported() -> bool {
    cfg!(all(target_os = "ios", feature = "mobile"))
}

/// Transient history image, rendered from the saved PencilKit archive on iOS.
/// Call only for an explicitly viewed original, on the app's main thread.
/// Other platforms cannot render the native archive faithfully.
#[cfg(not(all(target_os = "ios", feature = "mobile")))]
pub(crate) fn native_drawing_preview(
    checkpoint: &NativeWritingCheckpoint,
) -> NativeWritingResult<Option<String>> {
    checkpoint.validate()?;
    Ok(None)
}

/// Paper and native rendered bounds are combined before selecting a bitmap scale.
/// Keep both edges bounded, including long paper and transformed outlying ink.
pub(crate) fn native_preview_geometry(
    paper: &NativePaper,
    ink_bounds: Option<[f64; 4]>,
) -> NativeWritingResult<([f64; 4], f64)> {
    paper.validate()?;
    const MAX_EXTENT: f64 = 1_000_000.0;
    let mut rect = [0.0, 0.0, paper.width_points, paper.height_points];
    if let Some([x, y, width, height]) = ink_bounds {
        if ![x, y, width, height]
            .iter()
            .all(|n| n.is_finite() && n.abs() <= MAX_EXTENT)
            || width <= 0.0
            || height <= 0.0
        {
            return Err("The saved drawing has unsupported preview bounds.".into());
        }
        let left = x.min(0.0);
        let top = y.min(0.0);
        rect = [
            left,
            top,
            (x + width).max(paper.width_points) - left,
            (y + height).max(paper.height_points) - top,
        ];
    }
    let [_, _, width, height] = rect;
    if width > MAX_EXTENT || height > MAX_EXTENT {
        return Err("The saved drawing is too spread out for a history preview.".into());
    }
    // One pixel of rounding room keeps UIKit's raster dimensions within
    // 1024 × 2048 (about 8 MiB of RGBA), independent of device screen scale.
    let scale = (1023.0 / width).min(2047.0 / height).min(1.0);
    if width * scale < 1.0 || height * scale < 1.0 {
        return Err("The saved drawing is too narrow for a history preview.".into());
    }
    Ok((rect, scale))
}

#[cfg(not(all(target_os = "ios", feature = "mobile")))]
pub(crate) fn open_native(
    request: NativeWritingRequest,
    _callback: NativeWritingCallback,
) -> NativeWritingResult<()> {
    request.validate()?;
    Err("Native PencilKit paper is available in the iPad app. Use the writing pad on this platform.".into())
}
#[cfg(not(all(target_os = "ios", feature = "mobile")))]
pub(crate) fn ack(
    _instance: Uuid,
    _sequence: u64,
    _ok: bool,
    _error: Option<String>,
) -> NativeWritingResult<()> {
    Err("There is no native drawing controller on this platform.".into())
}

/// App-private recovery journal is independent of the renderer callback queue.
/// A caller can reconcile it with its durable activity; it does not seal a response.
#[cfg(not(target_arch = "wasm32"))]
fn recovery_path(request: &NativeWritingRequest) -> NativeWritingResult<std::path::PathBuf> {
    let root = directories::ProjectDirs::from("org", "ember", "rufren")
        .ok_or("The app's private data folder is unavailable.")?;
    let key = hash(
        format!(
            "{}\n{}\n{}\n{}",
            request.scope_id, request.attempt_id, request.task_id, request.task_revision
        )
        .as_bytes(),
    );
    Ok(root
        .data_dir()
        .join("native-ink-recovery")
        .join(format!("{key}.json")))
}
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn native_recovery(
    request: &NativeWritingRequest,
) -> NativeWritingResult<Option<NativeWritingCheckpoint>> {
    use std::io::Read;
    let path = recovery_path(request)?;
    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("Could not open the native ink recovery: {e}")),
    };
    if file.metadata().map_err(|e| e.to_string())?.len() > 64 * 1024 * 1024 {
        return Err("The native ink recovery exceeds its byte bound.".into());
    }
    let mut bytes = vec![];
    file.take(64 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 64 * 1024 * 1024 {
        return Err("The native ink recovery changed while it was read.".into());
    }
    let c: NativeWritingCheckpoint = serde_json::from_slice(&bytes)
        .map_err(|_| "The native ink recovery cannot be read. Its original is kept.")?;
    c.validate()?;
    if c.scope_id != request.scope_id
        || c.attempt_id != request.attempt_id
        || c.task_id != request.task_id
        || c.task_revision != request.task_revision
        || c.canonical.paper != request.paper
    {
        return Err(
            "The native ink recovery belongs to a different activity. Its original is kept.".into(),
        );
    }
    Ok(Some(c))
}
#[cfg(all(target_os = "ios", feature = "mobile"))]
fn journal_native(
    request: &NativeWritingRequest,
    checkpoint: &NativeWritingCheckpoint,
) -> NativeWritingResult<()> {
    let path = recovery_path(request)?;
    let directory = path.parent().ok_or("The native ink folder is invalid.")?;
    std::fs::create_dir_all(directory)
        .map_err(|e| format!("Could not prepare native ink recovery: {e}"))?;
    let temporary = directory.join(format!(".{}.tmp", Uuid::new_v4()));
    let result = (|| {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|e| e.to_string())?;
        serde_json::to_writer(&file, checkpoint).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        std::fs::rename(&temporary, &path).map_err(|e| e.to_string())?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}
#[cfg(target_arch = "wasm32")]
pub(crate) fn native_recovery(
    _request: &NativeWritingRequest,
) -> NativeWritingResult<Option<NativeWritingCheckpoint>> {
    Ok(None)
}

#[cfg(not(all(target_os = "ios", feature = "mobile")))]
pub(crate) fn is_open() -> bool {
    false
}
#[cfg(not(all(target_os = "ios", feature = "mobile")))]
pub(crate) fn request_pause(_instance: Uuid) -> NativeWritingResult<()> {
    Err("There is no native writing controller on this platform.".into())
}

/// Throughput batching, never a claim that delayed pressure has finished.
pub(crate) fn native_checkpoint_interval(tool_active: bool, explicit: bool) -> f64 {
    if tool_active {
        1.0
    } else if explicit {
        0.0
    } else {
        0.25
    }
}

#[cfg(test)]
mod preview_tests {
    use super::*;

    fn paper(width_points: f64, height_points: f64) -> NativePaper {
        NativePaper {
            width_points,
            height_points,
            line_height_points: 80.0,
        }
    }

    #[test]
    fn preview_bounds_include_transformed_ink_outside_all_paper_edges() {
        let (rect, scale) =
            native_preview_geometry(&paper(1024.0, 768.0), Some([-100.0, -40.0, 1300.0, 900.0]))
                .unwrap();
        assert_eq!(rect, [-100.0, -40.0, 1300.0, 900.0]);
        assert!(rect[2] * scale <= 1024.0);
        assert!(rect[3] * scale <= 2048.0);
    }

    #[test]
    fn preview_raster_stays_bounded_for_every_extreme_valid_paper_shape() {
        for width in [320.0, 8192.0] {
            for height in [200.0, 16384.0] {
                let (rect, scale) = native_preview_geometry(&paper(width, height), None).unwrap();
                assert_eq!(rect, [0.0, 0.0, width, height]);
                assert!((1.0..=1024.0).contains(&(width * scale).ceil()));
                assert!((1.0..=2048.0).contains(&(height * scale).ceil()));
                assert!((width * scale).ceil() * (height * scale).ceil() <= 1024.0 * 2048.0);
            }
        }
    }

    #[test]
    fn preview_rejects_nonfinite_excessive_and_subpixel_geometry() {
        let paper = paper(1024.0, 768.0);
        for bounds in [
            [f64::NAN, 0.0, 10.0, 10.0],
            [0.0, f64::INFINITY, 10.0, 10.0],
            [0.0, 0.0, f64::MAX, 10.0],
            [0.0, 0.0, 0.0, 10.0],
            [0.0, 0.0, -1.0, 10.0],
            [-1_000_000.0, 0.0, 10.0, 10.0],
            [0.0, 0.0, 1_000_000.0, 10.0],
        ] {
            assert!(native_preview_geometry(&paper, Some(bounds)).is_err());
        }
    }
}
