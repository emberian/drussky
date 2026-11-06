//! Finite offline references; the validated manifest generates the single catalog.
use base64::{Engine as _, engine::general_purpose::STANDARD};
use dioxus::prelude::*;

#[derive(Clone, Copy)]
pub(crate) struct AudioCredits {
    pub title: &'static str,
    pub credit: &'static str,
    pub source_url: &'static str,
    pub license: &'static str,
    pub license_url: &'static str,
    pub changes: &'static str,
}

struct PreparedAudio {
    id: &'static str,
    bytes: &'static [u8],
    frames: u64,
    sample_rate: u32,
    credits: AudioCredits,
    attribution: &'static str,
}

include!("foundation_audio_catalog.rs");

fn lookup(asset_id: &str) -> Option<&'static PreparedAudio> {
    CATALOG.iter().find(|audio| audio.id == asset_id)
}

pub(crate) fn bytes(asset_id: &str) -> Option<&'static [u8]> {
    lookup(asset_id).map(|audio| audio.bytes)
}

pub(crate) fn credits(asset_id: &str) -> AudioCredits {
    lookup(asset_id)
        .map(|audio| audio.credits)
        .unwrap_or(AudioCredits {
            title: "Recording unavailable",
            credit: "No prepared recording with this ID",
            source_url: "",
            license: "Unavailable",
            license_url: "",
            changes: "",
        })
}

pub(crate) fn attribution(asset_id: &str) -> &'static str {
    lookup(asset_id).map_or("Recording unavailable", |audio| audio.attribution)
}

/// Original container duration, not a phoneme alignment.
pub(crate) fn duration_seconds(asset_id: &str) -> Option<f64> {
    lookup(asset_id).map(|audio| audio.frames as f64 / audio.sample_rate as f64)
}

/// Stop immediately when advancing, skipping, or leaving the foundation route.
pub(crate) fn stop() {
    let _ = document::eval("window.rufrenFoundationAudio?.cancel();");
}

/// Completes only after this exact original model has finished playing.
/// Replacing playback cancels its completion rather than recording a heard model.
pub(crate) async fn play(asset_id: &str) -> Result<(), String> {
    let wav = bytes(asset_id).ok_or("This prepared model is unavailable.")?;
    let source = serde_json::to_string(&format!("data:audio/wav;base64,{}", STANDARD.encode(wav)))
        .map_err(|error| error.to_string())?;
    let script = format!(
        "const source={source};\n{}",
        include_str!("foundation_audio.js")
    );
    let complete: serde_json::Value = document::eval(&script)
        .await
        .map_err(|error| error.to_string())?;
    if complete == serde_json::Value::Bool(true) {
        Ok(())
    } else {
        Err("Audio did not finish. Tap Replay to try again.".into())
    }
}
