//! Finite offline human and synthetic references; original sources/provenance ship together.
use base64::{Engine as _, engine::general_purpose::STANDARD};
use dioxus::prelude::*;

pub(crate) fn bytes(asset_id: &str) -> Option<&'static [u8]> {
    macro_rules! prepared {
        ($($id:literal),+ $(,)?) => {match asset_id {
            $($id => Some(include_bytes!(concat!("../assets/foundations/", $id, ".wav")).as_slice()),)+
            _ => None,
        }};
    }
    prepared!(
        "foundation-a",
        "foundation-ma",
        "foundation-am",
        "foundation-da",
        "foundation-net",
        "foundation-o",
        "foundation-ka",
        "foundation-ta",
        "foundation-kot",
        "foundation-tam",
        "foundation-tam-kot"
    )
}

pub(crate) struct AudioCredits {
    pub title: &'static str,
    pub credit: &'static str,
    pub source_url: &'static str,
    pub license: &'static str,
    pub license_url: &'static str,
    pub changes: &'static str,
}

pub(crate) fn credits(asset_id: &str) -> AudioCredits {
    let (title, credit, source_url, license, license_url) = match asset_id {
        "foundation-a" => ("Human recording: а", "The Shtooka Project; individual speaker not identified", "https://commons.wikimedia.org/wiki/File:Ru-а.ogg", "CC BY 2.0 France", "https://creativecommons.org/licenses/by/2.0/fr/"),
        "foundation-o" => ("Human recording: о", "Сергей Сахно, Vion Nicolas / The Shtooka Project", "https://commons.wikimedia.org/wiki/File:Ru-о.ogg", "CC BY 3.0 United States", "https://creativecommons.org/licenses/by/3.0/us/"),
        "foundation-kot" => ("Human recording: кот", "The Shtooka Project; individual speaker not identified", "https://commons.wikimedia.org/wiki/File:Ru-кот.ogg", "CC BY 2.0 France", "https://creativecommons.org/licenses/by/2.0/fr/"),
        _ => ("Synthetic reference", "Silero v5_6_ru, Xenia voice", "https://github.com/snakers4/silero-models", "CC BY-NC-SA 4.0", "https://creativecommons.org/licenses/by-nc-sa/4.0/"),
    };
    AudioCredits { title, credit, source_url, license, license_url, changes: if matches!(asset_id, "foundation-a" | "foundation-o" | "foundation-kot") { "Decoded original Ogg Vorbis to PCM16 WAV; no trimming, gain, rate or channel change." } else { "Generated locally; original unmodified PCM16 WAV." } }
}

/// Credit for the exact bundled reference; source/license URLs live in the manifest.
pub(crate) fn attribution(asset_id: &str) -> &'static str {
    match asset_id {
        "foundation-o" => "Human recording: Сергей Сахно, Vion Nicolas / Shtooka — CC BY 3.0 US",
        "foundation-a" | "foundation-kot" => "Human recording: Shtooka Project — CC BY 2.0 FR (speaker not named)",
        _ => "Synthetic reference: Silero v5_6_ru, Xenia voice — CC BY-NC-SA 4.0",
    }
}

/// Original container duration, not a phoneme alignment.
pub(crate) fn duration_seconds(asset_id: &str) -> Option<f64> {
    match asset_id {
        "foundation-a" => Some(41472.0 / 44100.0),
        "foundation-ma" | "foundation-da" | "foundation-tam" => Some(0.325),
        "foundation-am" => Some(0.3625),
        "foundation-net" => Some(0.375),
        "foundation-o" => Some(1.0),
        "foundation-ta" => Some(0.275),
        "foundation-ka" => Some(0.2875),
        "foundation-kot" => Some(43776.0 / 44100.0),
        "foundation-tam-kot" => Some(0.7125),
        _ => None,
    }
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
