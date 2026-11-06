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
        "foundation-tam-kot",
        "foundation-i",
        "foundation-na",
        "foundation-okno",
        "foundation-on",
        "foundation-ona",
        "foundation-oni",
        "foundation-ono",
        "foundation-ty",
        "foundation-ya",
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
        "foundation-a" => (
            "Human recording: а",
            "The Shtooka Project; individual speaker not identified",
            "https://commons.wikimedia.org/wiki/File:Ru-а.ogg",
            "CC BY 2.0 France",
            "https://creativecommons.org/licenses/by/2.0/fr/",
        ),
        "foundation-o" => (
            "Human recording: о",
            "Сергей Сахно, Vion Nicolas / The Shtooka Project",
            "https://commons.wikimedia.org/wiki/File:Ru-о.ogg",
            "CC BY 3.0 United States",
            "https://creativecommons.org/licenses/by/3.0/us/",
        ),
        "foundation-kot" => (
            "Human recording: кот",
            "The Shtooka Project; individual speaker not identified",
            "https://commons.wikimedia.org/wiki/File:Ru-кот.ogg",
            "CC BY 2.0 France",
            "https://creativecommons.org/licenses/by/2.0/fr/",
        ),
        "foundation-i" => (
            "Human recording: и",
            "The Shtooka Project; individual speaker not identified on source page",
            "https://commons.wikimedia.org/wiki/File:Ru-и.ogg",
            "CC BY 2.0 fr",
            "https://creativecommons.org/licenses/by/2.0/fr/deed.en",
        ),
        "foundation-na" => (
            "Human recording: на",
            "Сергей Сахно, Vion Nicolas / The Shtooka Project",
            "https://commons.wikimedia.org/wiki/File:Ru-на.ogg",
            "CC BY 3.0 us",
            "https://creativecommons.org/licenses/by/3.0/us/deed.en",
        ),
        "foundation-okno" => (
            "Human recording: окно",
            "The Shtooka Project; individual speaker not identified on source page",
            "https://commons.wikimedia.org/wiki/File:Ru-окно.ogg",
            "CC BY 2.0 fr",
            "https://creativecommons.org/licenses/by/2.0/fr/deed.en",
        ),
        "foundation-on" => (
            "Human recording: он",
            "The Shtooka Project; individual speaker not identified on source page",
            "https://commons.wikimedia.org/wiki/File:Ru-он.ogg",
            "CC BY 2.0 fr",
            "https://creativecommons.org/licenses/by/2.0/fr/deed.en",
        ),
        "foundation-ona" => (
            "Human recording: она",
            "Сергей Сахно, Vion Nicolas / The Shtooka Project",
            "https://commons.wikimedia.org/wiki/File:Ru-она.ogg",
            "CC BY 3.0 us",
            "https://creativecommons.org/licenses/by/3.0/us/deed.en",
        ),
        "foundation-oni" => (
            "Human recording: они",
            "Сергей Сахно, Vion Nicolas / The Shtooka Project",
            "https://commons.wikimedia.org/wiki/File:Ru-они.ogg",
            "CC BY 3.0 us",
            "https://creativecommons.org/licenses/by/3.0/us/deed.en",
        ),
        "foundation-ono" => (
            "Human recording: оно",
            "1640max / Forvo",
            "https://commons.wikimedia.org/wiki/File:Ru-оно.ogg",
            "CC BY-SA 4.0",
            "https://creativecommons.org/licenses/by-sa/4.0",
        ),
        "foundation-ty" => (
            "Human recording: ты",
            "Сергей Сахно, Vion Nicolas / The Shtooka Project",
            "https://commons.wikimedia.org/wiki/File:Ru-ты.ogg",
            "CC BY 3.0 us",
            "https://creativecommons.org/licenses/by/3.0/us/deed.en",
        ),
        "foundation-ya" => (
            "Human recording: я",
            "Сергей Сахно, Vion Nicolas / The Shtooka Project",
            "https://commons.wikimedia.org/wiki/File:Ru-я.ogg",
            "CC BY 3.0 us",
            "https://creativecommons.org/licenses/by/3.0/us/deed.en",
        ),
        _ => (
            "Synthetic reference",
            "Silero v5_6_ru, Xenia voice",
            "https://github.com/snakers4/silero-models",
            "CC BY-NC-SA 4.0",
            "https://creativecommons.org/licenses/by-nc-sa/4.0/",
        ),
    };
    AudioCredits {
        title,
        credit,
        source_url,
        license,
        license_url,
        changes: if matches!(
            asset_id,
            "foundation-a"
                | "foundation-o"
                | "foundation-kot"
                | "foundation-i"
                | "foundation-na"
                | "foundation-okno"
                | "foundation-on"
                | "foundation-ona"
                | "foundation-oni"
                | "foundation-ono"
                | "foundation-ty"
                | "foundation-ya"
        ) {
            "Decoded original Ogg Vorbis to PCM16 WAV; no trimming, gain, rate or channel change."
        } else {
            "Generated locally; original unmodified PCM16 WAV."
        },
    }
}

/// Credit for the exact bundled reference; source/license URLs live in the manifest.
pub(crate) fn attribution(asset_id: &str) -> &'static str {
    match asset_id {
        "foundation-o" => "Human recording: Сергей Сахно, Vion Nicolas / Shtooka — CC BY 3.0 US",
        "foundation-a" | "foundation-kot" => {
            "Human recording: Shtooka Project — CC BY 2.0 FR (speaker not named)"
        }
        "foundation-i" => {
            "Human recording: The Shtooka Project; individual speaker not identified on source page — CC BY 2.0 fr"
        }
        "foundation-na" => {
            "Human recording: Сергей Сахно, Vion Nicolas / The Shtooka Project — CC BY 3.0 us"
        }
        "foundation-okno" => {
            "Human recording: The Shtooka Project; individual speaker not identified on source page — CC BY 2.0 fr"
        }
        "foundation-on" => {
            "Human recording: The Shtooka Project; individual speaker not identified on source page — CC BY 2.0 fr"
        }
        "foundation-ona" => {
            "Human recording: Сергей Сахно, Vion Nicolas / The Shtooka Project — CC BY 3.0 us"
        }
        "foundation-oni" => {
            "Human recording: Сергей Сахно, Vion Nicolas / The Shtooka Project — CC BY 3.0 us"
        }
        "foundation-ono" => "Human recording: 1640max / Forvo — CC BY-SA 4.0",
        "foundation-ty" => {
            "Human recording: Сергей Сахно, Vion Nicolas / The Shtooka Project — CC BY 3.0 us"
        }
        "foundation-ya" => {
            "Human recording: Сергей Сахно, Vion Nicolas / The Shtooka Project — CC BY 3.0 us"
        }
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
        "foundation-i" => Some(40320.0 / 44100.0),
        "foundation-na" => Some(48400.0 / 44000.0),
        "foundation-okno" => Some(46080.0 / 44100.0),
        "foundation-on" => Some(40320.0 / 44100.0),
        "foundation-ona" => Some(55000.0 / 44000.0),
        "foundation-oni" => Some(57200.0 / 44000.0),
        "foundation-ono" => Some(21888.0 / 44100.0),
        "foundation-ty" => Some(44000.0 / 44000.0),
        "foundation-ya" => Some(50600.0 / 44000.0),
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
