//! Speech studio UI; the JS adapter handles devices and binary storage, Rust handles DSP.
use base64::{Engine as _, engine::general_purpose::STANDARD};
use dioxus::prelude::*;
use rufren_audio::{
    AudioAnalysis, AudioError, AudioMetadata, PcmAudio, RegionMetrics, RegionSpan,
    analyze_cooperatively, compare, compare_regions, measure_region_cooperatively,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, rc::Rc};

const BRIDGE: &str = include_str!("audio_bridge.js");
const STYLE: &str = include_str!("audio_studio.css");

/// Standalone JSON export, including on pages which never mounted AudioStudio.
/// Native success means save/share completed; browser success means the download
/// request was handed to the browser, which cannot confirm a filesystem save.
/// Cancellation returns the stable neutral message below, distinct from failure.
pub async fn export_json(text: String, filename: String) -> Result<(), String> {
    use std::{
        future::{Future, poll_fn},
        task::Poll,
    };
    if text.is_empty() || text.len() as u64 > crate::apple_export::MAX_EXPORT_BYTES {
        return Err("Choose a JSON export under 150 MiB. Your original is kept.".into());
    }
    serde_json::from_str::<serde_json::Value>(&text)
        .map_err(|error| format!("The export is not valid JSON: {error}"))?;
    let filename = json_export_filename(&filename);
    let byte_length = text.len() as u64;
    let hash = format!("{:x}", Sha256::digest(text.as_bytes()));
    let native = crate::apple_export::supported();
    let payload = serde_json::json!({"text":text,"filename":filename,"byte_length":byte_length,"sha256":hash,"native":native});
    let script = format!(
        "const payload={payload};\n{}\n{}",
        crate::apple_export::JAVASCRIPT,
        include_str!("json_export.js")
    );
    let mut bridge = document::eval(&script);
    let joined = bridge;
    let mut completion = Box::pin(async move {
        joined
            .join::<JsonExportOutcome>()
            .await
            .map_err(|error| format!("The export could not finish: {error}"))
    });
    enum Step {
        Request(Result<JsonExportRequest, String>),
        Finished(Result<JsonExportOutcome, String>),
    }
    let outcome = loop {
        let mut incoming = Box::pin(async {
            bridge
                .recv::<JsonExportRequest>()
                .await
                .map_err(|error| format!("The export connection closed: {error}"))
        });
        let next = poll_fn(|cx| {
            if let Poll::Ready(result) = completion.as_mut().poll(cx) {
                return Poll::Ready(Step::Finished(result));
            }
            incoming.as_mut().poll(cx).map(Step::Request)
        })
        .await;
        drop(incoming);
        match next {
            Step::Finished(result) => break result?,
            Step::Request(result) => {
                let request = result?.export_request;
                if !native
                    || request.name != filename
                    || request.mime_type != "application/json"
                    || request.byte_length != byte_length
                    || request.sha256 != hash
                {
                    let result = serde_json::json!({"request_id":request.request_id,"status":"failed","message":"The export request differs from the exact JSON original. Your original is kept."});
                    let _ =
                        document::eval(&format!("window.rufrenFileExports?.complete({result});"));
                } else {
                    crate::apple_export::start_export(request);
                }
            }
        }
    };
    json_export_result(native, outcome)
}

#[derive(Deserialize)]
struct JsonExportRequest {
    export_request: crate::apple_export::ExportRequest,
}
#[derive(Deserialize)]
struct JsonExportOutcome {
    status: String,
    #[serde(default)]
    message: String,
}

fn json_export_filename(filename: &str) -> String {
    let leaf = filename.rsplit(['/', '\\']).next().unwrap_or_default();
    let clean: String = leaf
        .chars()
        .map(|c| {
            if c.is_control() || "<>:\"|?*".contains(c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    let clean = clean
        .trim()
        .trim_start_matches('.')
        .trim_end_matches(['.', ' ']);
    let stem = clean
        .strip_suffix(".json")
        .or_else(|| clean.strip_suffix(".JSON"))
        .unwrap_or(clean);
    let stem: String = stem.chars().take(75).collect();
    format!(
        "{}.json",
        if stem.trim().is_empty() {
            "export"
        } else {
            stem.as_str()
        }
    )
}
fn json_export_result(native: bool, outcome: JsonExportOutcome) -> Result<(), String> {
    match outcome.status.as_str(){
        "saved"|"shared" if native=>Ok(()),
        "requested" if !native=>Ok(()),
        "cancelled"=>Err("Export cancelled. Your original is kept.".into()),
        "failed"=>Err(if outcome.message.is_empty(){"The export could not be completed. Your original is kept.".into()}else{outcome.message}),
        _=>Err("The export did not report a completed save, share, or browser download request. Your original is kept.".into()),
    }
}

#[cfg(test)]
mod json_export_tests {
    use super::*;
    #[test]
    fn json_export_names_are_portable_leaf_names_with_json_extension() {
        assert_eq!(
            json_export_filename("/tmp/../ink:study?.json"),
            "ink_study_.json"
        );
        assert_eq!(
            json_export_filename("C:\\private\\.ink\nnotes.JSON"),
            "ink_notes.json"
        );
        assert_eq!(json_export_filename("../"), "export.json");
        let name = json_export_filename(&"😀".repeat(100));
        assert!(name.encode_utf16().count() <= 180);
        assert!(!name.contains('/'));
        assert!(name.ends_with(".json"));
    }
    #[test]
    fn json_export_requires_platform_terminal_status_and_keeps_cancel_neutral() {
        let outcome = |status: &str| JsonExportOutcome {
            status: status.into(),
            message: String::new(),
        };
        assert!(json_export_result(true, outcome("saved")).is_ok());
        assert!(json_export_result(true, outcome("shared")).is_ok());
        assert!(json_export_result(true, outcome("requested")).is_err());
        assert!(json_export_result(true, outcome("prepared")).is_err());
        assert!(json_export_result(false, outcome("requested")).is_ok());
        assert_eq!(
            json_export_result(true, outcome("cancelled")).unwrap_err(),
            "Export cancelled. Your original is kept."
        );
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct StudioAnnotation {
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub note: String,
    pub created_at_ms: i64,
}

/// A pointer to a locally stored original. Binary audio stays in IndexedDB.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct StudioRecording {
    pub asset_id: String,
    /// SHA-256 of original encoded file bytes, independent of decoded PCM.
    pub content_hash: String,
    pub role: String,
    pub name: String,
    pub passage_id: String,
    pub revision_id: String,
    pub duration_ms: u64,
    /// Creator's decoded PCM rate; another decoder/device may produce a different rate.
    pub sample_rate: u32,
    pub channels: u16,
    #[serde(default)]
    /// Frames in that decoded PCM observation, not a property implied by the byte hash.
    pub frames: u64,
    pub byte_length: u64,
    pub created_at_ms: i64,
    pub mime_type: String,
    #[serde(default)]
    pub interrupted: bool,
    #[serde(default)]
    pub device: serde_json::Value,
    #[serde(default)]
    pub annotations: Vec<StudioAnnotation>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DialogueRecorderScope {
    pub thread_id: uuid::Uuid,
    pub input_id: uuid::Uuid,
}

/// Originals produced for one explicit Sound Focus intention. This is a
/// recording context, not an assessment or a pronunciation claim.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SoundFocusRecorderScope {
    pub session_id: uuid::Uuid,
    pub intent_id: uuid::Uuid,
    pub member_id: String,
}

/// A local voice original associated with an exact web/native writing artifact.
/// This metadata is not an Attempt or a claim that ink is synchronized.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WritingRecorderScope {
    pub run_id: uuid::Uuid,
    pub round_id: uuid::Uuid,
    pub ink_original_id: uuid::Uuid,
    pub ink_sha256: String,
    pub ink_format: String,
    pub task_id: String,
    pub task_revision: String,
}

/// Save one generated original, bound to the exact phrase revision and request.
/// Network/provider configuration is owned by the shared gateway client.
pub async fn save_generated_reference(
    passage_id: &str,
    revision_id: &str,
    expected_text: &str,
    expected_request_id: &str,
    response: serde_json::Value,
) -> Result<StudioRecording, String> {
    save_generated_audio(
        passage_id,
        revision_id,
        None,
        expected_text,
        expected_request_id,
        response,
    )
    .await
}

pub(crate) async fn save_generated_dialogue_audio(
    thread_id: uuid::Uuid,
    reply_id: uuid::Uuid,
    chunk_index: usize,
    expected_text: &str,
    expected_request_id: &str,
    response: serde_json::Value,
) -> Result<StudioRecording, String> {
    if thread_id.is_nil() || reply_id.is_nil() || chunk_index > 1 {
        return Err("Invalid dialogue utterance binding.".into());
    }
    save_generated_audio("","",Some(serde_json::json!({"thread_id":thread_id,"reply_id":reply_id,"chunk_index":chunk_index})),expected_text,expected_request_id,response).await
}

async fn save_generated_audio(
    passage_id: &str,
    revision_id: &str,
    dialogue_scope: Option<serde_json::Value>,
    expected_text: &str,
    expected_request_id: &str,
    response: serde_json::Value,
) -> Result<StudioRecording, String> {
    #[derive(Deserialize)]
    struct Generated {
        source_text: String,
        request_id: String,
        revision_id: Option<String>,
        wav_base64: String,
        sha256: String,
        sample_rate: u32,
        channels: u16,
        frames: u64,
        provenance: serde_json::Value,
    }
    if expected_text.trim().is_empty()
        || expected_request_id.len() > 128
        || expected_text.len() > 1024
        || expected_text.chars().count() > 256
        || expected_request_id.is_empty()
    {
        return Err("Generate one short phrase at a time.".into());
    }
    let generated: Generated = serde_json::from_value(response)
        .map_err(|e| format!("Invalid generated audio response: {e}"))?;
    if generated.source_text != expected_text
        || generated.request_id != expected_request_id
        || generated.revision_id.as_deref()
            != if dialogue_scope.is_some() {
                None
            } else {
                Some(revision_id)
            }
    {
        return Err(
            "Generated audio does not match this exact phrase and request. Nothing was attached."
                .into(),
        );
    }
    if (dialogue_scope.is_none()
        && (!uuid::Uuid::parse_str(passage_id).is_ok_and(|id| !id.is_nil())
            || !uuid::Uuid::parse_str(revision_id).is_ok_and(|id| !id.is_nil())))
        || generated.wav_base64.len() > 6 * 1024 * 1024
    {
        return Err("Invalid generated audio binding or size.".into());
    }
    let bytes = STANDARD
        .decode(&generated.wav_base64)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 4 * 1024 * 1024 {
        return Err("Generated audio exceeds the recording limit.".into());
    }
    let hash = format!("{:x}", Sha256::digest(&bytes));
    if hash != generated.sha256 {
        return Err("Generated audio failed its SHA-256 check. Nothing was attached.".into());
    }
    let pcm = PcmAudio::from_wav(&bytes).map_err(|e| format!("Invalid generated WAV: {e}"))?;
    let meta = pcm.metadata();
    if meta.sample_rate != generated.sample_rate
        || meta.channels != generated.channels
        || meta.frames as u64 != generated.frames
        || meta.duration_seconds > 45.0
        || meta.sample_rate != 24_000
        || meta.channels != 1
    {
        return Err("Generated WAV metadata does not match the promised audio.".into());
    }
    let mut provenance = generated
        .provenance
        .as_object()
        .cloned()
        .ok_or("Generated speech has no provenance")?;
    if !provenance
        .get("kind")
        .and_then(|v| v.as_str())
        .is_some_and(|kind| matches!(kind, "ai_generated_speech" | "system_generated_speech"))
    {
        return Err("The reference has no recognized machine speech provenance.".into());
    }
    if serde_json::to_vec(&provenance).map_or(true, |v| v.len() > 4096) {
        return Err("Generated speech provenance exceeds its limit.".into());
    }
    for key in ["kind", "provider", "model", "voice"] {
        if !provenance
            .get(key)
            .and_then(|v| v.as_str())
            .is_some_and(|v| !v.is_empty() && v.len() <= 256)
        {
            return Err(format!("Generated speech has no valid {key} provenance"));
        }
    }
    if let Some(input) = provenance
        .get("synthesis_input")
        .filter(|value| !value.is_null())
    {
        let input: rufren_speech::SpeechSynthesisInput = serde_json::from_value(input.clone())
            .map_err(|_| "Generated speech has invalid synthesis-input provenance.")?;
        input
            .validate_for(expected_text)
            .map_err(|error| error.to_string())?;
    }
    provenance.insert("source_text".into(), expected_text.into());
    provenance.insert(
        "source_text_sha256".into(),
        format!("{:x}", Sha256::digest(expected_text.as_bytes())).into(),
    );
    if let Some(scope) = &dialogue_scope {
        provenance.insert("dialogue_scope".into(), scope.clone());
    } else {
        provenance.insert("revision_id".into(), revision_id.into());
    }
    provenance.insert("request_id".into(), expected_request_id.into());
    let voice = provenance
        .get("voice")
        .and_then(|v| v.as_str())
        .unwrap_or("Generated voice");
    let mut device = serde_json::json!({"provenance":provenance,"source_mime_type":"audio/wav"});
    if let Some(scope) = dialogue_scope {
        device["dialogue_scope"] = scope;
    }
    let recording = StudioRecording {
        asset_id: format!("{hash}-{}", uuid::Uuid::new_v4()),
        content_hash: hash,
        role: "reference".into(),
        name: format!("Generated reference · {voice}"),
        passage_id: passage_id.into(),
        revision_id: revision_id.into(),
        duration_ms: (meta.duration_seconds * 1000.0).round() as u64,
        sample_rate: meta.sample_rate,
        channels: meta.channels,
        frames: meta.frames as u64,
        byte_length: bytes.len() as u64,
        created_at_ms: crate::repository::now_ms(),
        mime_type: "audio/wav".into(),
        interrupted: false,
        device,
        annotations: Vec::new(),
    };
    let request = serde_json::json!({"recording":recording,"wav_base64":generated.wav_base64});
    let script = format!(
        "const request={};\n{}",
        request,
        include_str!("generated_audio_store.js")
    );
    let result = document::eval(&script).await.map_err(|e| e.to_string())?;
    if let Some(error) = result.get("error").and_then(|v| v.as_str()) {
        return Err(error.into());
    }
    serde_json::from_value(
        result
            .get("recording")
            .cloned()
            .ok_or("Generated audio was not saved")?,
    )
    .map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn import_native_file(event: FormEvent, role: &'static str, mut error: Signal<String>) {
    let Some(file) = event.files().into_iter().next() else {
        return;
    };
    let limit = if role == "restore" {
        150 * 1024 * 1024
    } else {
        32 * 1024 * 1024
    };
    if file.size() > limit {
        error.set(
            if role == "restore" {
                "Choose an audio backup smaller than 150 MB."
            } else {
                "Choose an audio file smaller than 32 MB."
            }
            .into(),
        );
        return;
    }
    spawn(async move {
        let result=async {
            let bytes=file.read_bytes().await.map_err(|e|e.to_string())?;
            if bytes.len() as u64>limit{return Err("The selected file exceeds its supported size.".into());}
            let request=serde_json::json!({"role":role,"name":file.name(),"mime_type":file.content_type().unwrap_or_default(),"base64":STANDARD.encode(&bytes)});
            let script=format!("const request={request};return await window.rufrenAudioStudio?.importBytes(request);");
            let value=document::eval(&script).await.map_err(|e|e.to_string())?;
            if let Some(message)=value.get("error").and_then(|v|v.as_str()){return Err(message.into());}
            Ok::<(),String>(())
        }.await;
        if let Err(message) = result {
            error.set(message);
        }
    });
}
#[cfg(target_arch = "wasm32")]
fn import_native_file(_event: FormEvent, _role: &'static str, _error: Signal<String>) {}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StudioRegionPlayback {
    pub recording: StudioRecording,
    pub slot: String,
    pub source_pcm_hash: String,
    pub source_kind: String,
    pub sample_rate: u32,
    pub span: RegionSpan,
    pub playback_rate: f64,
}

#[derive(Deserialize)]
struct BridgeEvent {
    kind: String,
    #[serde(default)]
    active: bool,
    #[serde(default)]
    role: String,
    #[serde(default)]
    sample_rate: u32,
    #[serde(default)]
    samples: String,
    #[serde(default)]
    channels: u16,
    #[serde(default)]
    recording: Option<StudioRecording>,
    #[serde(default)]
    wav_base64: String,
    #[serde(default)]
    request_id: String,
    #[serde(default)]
    asset_id: String,
    #[serde(default)]
    content_hash: String,
    #[serde(default)]
    source_pcm_hash: String,
    #[serde(default)]
    start_frame: u64,
    #[serde(default)]
    end_frame: u64,
    #[serde(default)]
    playback_rate: f64,
    #[serde(default)]
    export_request: Option<crate::apple_export::ExportRequest>,
}

struct CachedSource {
    recording: StudioRecording,
    pcm: PcmAudio,
    analysis: Rc<AudioAnalysis>,
    source_kind: &'static str,
}
enum SourceLoad {
    Ready(CachedSource),
    Fallback(String),
    Cancelled,
}

#[derive(Serialize)]
struct PlotMessage<'a> {
    kind: &'static str,
    role: &'a str,
    request_id: &'a str,
    asset_id: &'a str,
    content_hash: &'a str,
    source: PlotSource<'a>,
    analysis: PlotAnalysis<'a>,
}
#[derive(Serialize)]
struct PlotSource<'a> {
    metadata: &'a AudioMetadata,
    kind: &'a str,
}
#[derive(Serialize)]
struct PlotAnalysis<'a> {
    duration_seconds: f64,
    quality: &'a rufren_audio::RecordingQuality,
    waveform: &'a [rufren_audio::EnvelopeBin],
    pitch: &'a [rufren_audio::PitchFrame],
    spectrogram: PlotSpectrum<'a>,
}
#[derive(Serialize)]
struct PlotSpectrum<'a> {
    fft_size: usize,
    hop_size: usize,
    sample_rate: u32,
    frequency_bins: usize,
    frame_times_seconds: &'a [f64],
    power_db_base64: String,
}
fn plot_message<'a>(
    role: &'a str,
    request_id: &'a str,
    source: &'a CachedSource,
) -> PlotMessage<'a> {
    let analysis = &source.analysis;
    let spectrum = &analysis.spectrogram;
    let bytes: Vec<u8> = spectrum
        .power_db
        .iter()
        .flat_map(|f| f.to_le_bytes())
        .collect();
    PlotMessage {
        kind: "analysis",
        role,
        request_id,
        asset_id: &source.recording.asset_id,
        content_hash: &source.recording.content_hash,
        source: PlotSource {
            metadata: source.pcm.metadata(),
            kind: source.source_kind,
        },
        analysis: PlotAnalysis {
            duration_seconds: analysis.duration_seconds,
            quality: &analysis.quality,
            waveform: &analysis.waveform,
            pitch: &analysis.pitch,
            spectrogram: PlotSpectrum {
                fft_size: spectrum.fft_size,
                hop_size: spectrum.hop_size,
                sample_rate: spectrum.sample_rate,
                frequency_bins: spectrum.frequency_bins,
                frame_times_seconds: &spectrum.frame_times_seconds,
                power_db_base64: STANDARD.encode(bytes),
            },
        },
    }
}

async fn yield_selection(role: &str, asset_id: &str, request_id: &str, region: bool) -> bool {
    let attribute = serde_json::to_string(&format!("data-selected-{role}")).unwrap_or_default();
    let token_attribute = serde_json::to_string(&format!(
        "data-{}-request-{role}",
        if region { "region" } else { "analysis" }
    ))
    .unwrap_or_default();
    let asset = serde_json::to_string(asset_id).unwrap_or_default();
    let token = serde_json::to_string(request_id).unwrap_or_default();
    let code = format!(
        "await window.rufrenAudioStudio?.yieldNow();const root=document.getElementById('rufren-audio-studio');return !!root && root.getAttribute({attribute})==={asset} && root.getAttribute({token_attribute})==={token};"
    );
    document::eval(&code)
        .await
        .ok()
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}
async fn analyze_event(event: &BridgeEvent) -> Result<SourceLoad, String> {
    let recording = event
        .recording
        .clone()
        .ok_or("Audio source metadata is missing")?;
    let yield_now = || yield_selection(&event.role, &recording.asset_id, &event.request_id, false);
    let (pcm, source_kind) = if !event.wav_base64.is_empty() {
        if event.wav_base64.len() > 90 * 1024 * 1024 {
            return Err("Original WAV exceeds its bounded size".into());
        }
        let bytes = STANDARD
            .decode(&event.wav_base64)
            .map_err(|e| e.to_string())?;
        let mut byte_hash = Sha256::new();
        for chunk in bytes.chunks(262144) {
            byte_hash.update(chunk);
            if !yield_now().await {
                return Ok(SourceLoad::Cancelled);
            }
        }
        if format!("{:x}", byte_hash.finalize()) != recording.content_hash {
            return Err("Original WAV failed its byte-hash check".into());
        }
        match PcmAudio::from_wav_cooperatively(&bytes, yield_now).await {
            Ok(pcm) => (pcm, "original_wav"),
            Err(AudioError::Cancelled) => return Ok(SourceLoad::Cancelled),
            Err(error) => return Ok(SourceLoad::Fallback(error.to_string())),
        }
    } else {
        if event.samples.len() > 90 * 1024 * 1024 {
            return Err("PCM message exceeds the instrument limit".into());
        }
        let bytes = STANDARD.decode(&event.samples).map_err(|e| e.to_string())?;
        if bytes.len() % 4 != 0 || bytes.len() > 64 * 1024 * 1024 {
            return Err("Invalid PCM byte count".into());
        }
        let samples = bytes
            .chunks_exact(4)
            .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .collect();
        match PcmAudio::from_interleaved_cooperatively(
            event.sample_rate,
            event.channels,
            samples,
            yield_now,
        )
        .await
        {
            Ok(pcm) => (pcm, "decoded_pcm"),
            Err(AudioError::Cancelled) => return Ok(SourceLoad::Cancelled),
            Err(e) => return Err(e.to_string()),
        }
    };
    let analyzed = analyze_cooperatively(&pcm, yield_now).await;
    match analyzed {
        Ok(analysis) => Ok(SourceLoad::Ready(CachedSource {
            recording,
            pcm,
            analysis: Rc::new(analysis),
            source_kind,
        })),
        Err(AudioError::Cancelled) => Ok(SourceLoad::Cancelled),
        Err(e) => Err(e.to_string()),
    }
}

/// Mount one studio at a time. Key this component by revision when its passage changes.
#[component]
pub fn AudioStudio(
    #[props(default)] passage_id: String,
    #[props(default)] revision_id: String,
    #[props(default)] russian: String,
    on_saved: EventHandler<StudioRecording>,
    #[props(default)] on_reference_played: EventHandler<()>,
    #[props(default)] on_cue_saved: EventHandler<StudioRecording>,
    #[props(default)] on_generate_reference: Option<EventHandler<()>>,
    #[props(default)] generating_reference: bool,
    #[props(default)] workspace_id: String,
    #[props(default)] on_region_played: EventHandler<StudioRegionPlayback>,
    #[props(default)] compact: bool,
    #[props(default)] review_only: bool,
    #[props(default)] selected_asset_id: Option<String>,
    #[props(default)] selected_content_hash: Option<String>,
    #[props(default)] on_selected: EventHandler<Option<StudioRecording>>,
    #[props(default)] on_recording_state: EventHandler<bool>,
    #[props(default)] dialogue_scope: Option<DialogueRecorderScope>,
    #[props(default)] sound_focus_scope: Option<SoundFocusRecorderScope>,
    #[props(default)] writing_scope: Option<WritingRecorderScope>,
    #[props(default)] frozen_reference_asset_id: Option<String>,
    #[props(default)] frozen_reference_hash: Option<String>,
) -> Element {
    let instance_id = use_hook(|| uuid::Uuid::new_v4().to_string());
    let effect_instance = instance_id.clone();
    let mut reference = use_signal(|| None::<Rc<AudioAnalysis>>);
    let mut attempt = use_signal(|| None::<Rc<AudioAnalysis>>);
    let mut error = use_signal(String::new);
    let config = serde_json::json!({"passage_id":passage_id,"revision_id":revision_id,"workspace_id":workspace_id,"compact":compact,"review_only":review_only,"selected_asset_id":selected_asset_id,"selected_content_hash":selected_content_hash,"dialogue_scope":dialogue_scope,"sound_focus_scope":sound_focus_scope,"writing_scope":writing_scope,"frozen_reference_asset_id":frozen_reference_asset_id,"frozen_reference_hash":frozen_reference_hash,"native_exports":crate::apple_export::supported()}).to_string();
    use_effect(move || {
        let script = format!(
            "const rufrenStudioRoot=document.getElementById('rufren-audio-studio');if(rufrenStudioRoot?.dataset.instance!=={})return;\n{}\n{}",
            serde_json::to_string(&effect_instance).unwrap_or_default(),
            crate::apple_export::JAVASCRIPT,
            BRIDGE
        );
        spawn(async move {
            // Effects run after mounting. The long-lived eval keeps a bidirectional channel.
            let mut bridge = document::eval(&script);
            let mut sources: HashMap<String, Rc<CachedSource>> = HashMap::new();
            let mut regions: HashMap<String, (String, RegionMetrics)> = HashMap::new();
            loop {
                let event = match bridge.recv::<BridgeEvent>().await {
                    Ok(event) => event,
                    Err(e) => {
                        error.set(format!(
                            "The audio connection stopped: {e}. Reopen the studio to reconnect."
                        ));
                        break;
                    }
                };
                match event.kind.as_str() {
                    "pcm" => {
                        if !matches!(
                            event.role.as_str(),
                            "reference" | "attempt" | "region_a" | "region_b"
                        ) {
                            continue;
                        }
                        error.set(String::new());
                        regions.remove(&event.role);
                        if event.role == "reference" {
                            reference.set(None);
                        } else if event.role == "attempt" {
                            attempt.set(None);
                        }
                        let Some(recording) = event.recording.as_ref() else {
                            continue;
                        };
                        let reused = sources
                            .values()
                            .find(|source| {
                                source.recording.asset_id == recording.asset_id
                                    && source.recording.content_hash == recording.content_hash
                                    && (source.source_kind == "original_wav"
                                        || source.pcm.metadata().sample_rate == event.sample_rate
                                            && source.pcm.metadata().channels == event.channels)
                            })
                            .cloned();
                        let loaded = if let Some(source) = reused {
                            if !yield_selection(
                                &event.role,
                                &recording.asset_id,
                                &event.request_id,
                                false,
                            )
                            .await
                            {
                                continue;
                            }
                            Ok(Some(source))
                        } else {
                            match analyze_event(&event).await {
                                Ok(SourceLoad::Ready(source)) => Ok(Some(Rc::new(source))),
                                Ok(SourceLoad::Cancelled) => Ok(None),
                                Ok(SourceLoad::Fallback(reason)) => {
                                    let _=bridge.send(serde_json::json!({"kind":"request_decoded","role":event.role,"asset_id":recording.asset_id,"request_id":event.request_id,"reason":reason}));
                                    Ok(None)
                                }
                                Err(e) => Err(e),
                            }
                        };
                        match loaded {
                            Ok(Some(source)) => {
                                if let Err(e) = bridge.send(plot_message(
                                    &event.role,
                                    &event.request_id,
                                    &source,
                                )) {
                                    error.set(format!("Could not draw analysis: {e}"));
                                }
                                if event.role == "reference" {
                                    reference.set(Some(source.analysis.clone()));
                                } else if event.role == "attempt" {
                                    attempt.set(Some(source.analysis.clone()));
                                }
                                sources.insert(event.role.clone(), source);
                            }
                            Ok(None) => {}
                            Err(e) => error.set(format!(
                                "Original audio is available, but analysis could not run: {e}"
                            )),
                        }
                    }
                    "source_clear" => {
                        sources.remove(&event.role);
                        regions.remove(&event.role);
                        if event.role == "attempt" {
                            attempt.set(None);
                        } else if event.role == "reference" {
                            reference.set(None);
                        }
                    }
                    "region" => {
                        if !matches!(event.role.as_str(), "region_a" | "region_b") {
                            continue;
                        }
                        let Some(source) = sources.get(&event.role).cloned() else {
                            continue;
                        };
                        if source.recording.asset_id != event.asset_id
                            || source.recording.content_hash != event.content_hash
                            || source.pcm.metadata().content_hash != event.source_pcm_hash
                        {
                            continue;
                        }
                        let span = RegionSpan {
                            start_frame: event.start_frame,
                            end_frame: event.end_frame,
                        };
                        match measure_region_cooperatively(&source.pcm, span, || {
                            yield_selection(&event.role, &event.asset_id, &event.request_id, true)
                        })
                        .await
                        {
                            Ok(metrics) => {
                                let _=bridge.send(serde_json::json!({"kind":"region_metrics","role":event.role,"asset_id":event.asset_id,"content_hash":event.content_hash,"request_id":event.request_id,"metrics":metrics}));
                                regions.insert(
                                    event.role.clone(),
                                    (event.request_id.clone(), metrics),
                                );
                                if let (Some((a_token, a)), Some((b_token, b))) =
                                    (regions.get("region_a"), regions.get("region_b"))
                                {
                                    if let Ok(comparison) = compare_regions(a, b) {
                                        let _=bridge.send(serde_json::json!({"kind":"region_comparison","a_request_id":a_token,"b_request_id":b_token,"comparison":comparison}));
                                    }
                                }
                            }
                            Err(AudioError::Cancelled) => {}
                            Err(e) => {
                                let _=bridge.send(serde_json::json!({"kind":"region_error","role":event.role,"request_id":event.request_id,"error":e.to_string()}));
                            }
                        }
                    }
                    "region_played" => {
                        if !matches!(event.role.as_str(), "region_a" | "region_b")
                            || !event.playback_rate.is_finite()
                            || !(0.25..=2.0).contains(&event.playback_rate)
                        {
                            continue;
                        }
                        let Some(source) = sources.get(&event.role) else {
                            continue;
                        };
                        let span = RegionSpan {
                            start_frame: event.start_frame,
                            end_frame: event.end_frame,
                        };
                        if source.recording.asset_id == event.asset_id
                            && source.recording.content_hash == event.content_hash
                            && source.pcm.metadata().content_hash == event.source_pcm_hash
                            && span.validate_for(&source.pcm).is_ok()
                        {
                            on_region_played.call(StudioRegionPlayback {
                                recording: source.recording.clone(),
                                slot: if event.role == "region_a" { "A" } else { "B" }.into(),
                                source_pcm_hash: event.source_pcm_hash,
                                source_kind: source.source_kind.into(),
                                sample_rate: source.pcm.metadata().sample_rate,
                                span,
                                playback_rate: event.playback_rate,
                            });
                        }
                    }
                    "selected" => on_selected.call(event.recording),
                    "recording_state" => on_recording_state.call(event.active),
                    "export_file" => {
                        if let Some(request) = event.export_request {
                            crate::apple_export::start_export(request);
                        }
                    }
                    "cue_saved" => {
                        if let Some(recording) = event.recording {
                            on_cue_saved.call(recording);
                        }
                    }
                    "reference_played" => on_reference_played.call(()),
                    "saved" => {
                        if let Some(recording) = event.recording {
                            on_saved.call(recording);
                        }
                    }
                    _ => {}
                }
            }
        });
    });
    let drop_instance = instance_id.clone();
    use_drop(move || {
        let token = serde_json::to_string(&drop_instance).unwrap_or_default();
        let _ = document::eval(&format!("window.rufrenAudioStudio?.destroy({token});"));
    });
    let reference_analysis = reference.read().clone();
    let attempt_analysis = attempt.read().clone();
    let comparison = reference_analysis
        .as_ref()
        .zip(attempt_analysis.as_ref())
        .map(|(r, a)| compare(r, a));
    rsx! {
        style {"{STYLE}"}
        section {id:"rufren-audio-studio",class:if compact {"audio-studio audio-speaking"}else{"audio-studio"},"data-config":config,"data-instance":instance_id,"data-advanced":"false",
            if writing_scope.is_some() {
                div {class:"writing-voice-controls",
                    button {class:"secondary","data-action":"record","data-role":"attempt","data-record":"true","Record my voice"}
                    button {id:"audio-stop-recording",class:"audio-stop",disabled:true,hidden:true,"data-action":"stop","Stop & keep"}
                    button {class:"secondary",disabled:true,"data-needs":"attempt","data-action":"play","data-role":"attempt","Replay my voice"}
                    button {class:"text-button",hidden:true,"data-writing-stop-playback":"true","data-action":"stopPlayback","Stop replay"}
                }
                div {id:"audio-mic-status",class:"audio-mic",role:"status","data-active":"false","Microphone off"}
                p {id:"audio-status",role:"status",aria_live:"polite","Opening saved voices…"}
                if !error.read().is_empty() {p {class:"audio-error",role:"alert","{error}"}}
                div {id:"writing-voice-recovery",hidden:true,
                    p {"This take has not saved yet. Keep this page open to retry or save a copy."}
                    div {class:"actions",
                        button {class:"secondary","data-action":"retryWritingSave","Retry saving"}
                        button {class:"secondary","data-action":"download","data-role":"attempt","Save audio copy"}
                    }
                }
                details {id:"writing-voice-history",hidden:true,
                    summary {"Earlier takes"}
                    div {id:"writing-voice-takes",role:"group",aria_label:"Choose a voice for this drawing"}
                }
            } else {
            if compact {div {class:"audio-speaking-heading",h3 {if dialogue_scope.is_some(){"Your voice message"}else{"Your spoken response"}}button {class:"text-button","data-action":"advanced",aria_expanded:"false","Show sound tools"}}}
            if !russian.is_empty() {div {class:"panel audio-passage",span {class:"tag","YOUR PHRASE"}p {class:"russian-display",lang:"ru","{russian}"}}}
            div {class:"panel audio-session-bar",
                div {id:"audio-mic-status",class:"audio-mic",role:"status","data-active":"false","Microphone off"}
                button {id:"audio-stop-recording",class:"audio-stop",disabled:true,"data-action":"stop","Stop & keep take"}
                p {id:"audio-status",role:"status",aria_live:"polite","Opening saved recordings…"}
                if !error.read().is_empty() {p {class:"audio-error",role:"alert","{error}"}}
            }
            div {class:"audio-tracks",
                for (role,title,description) in [("reference","Reference voice","Generate a Russian example, or add a voice you want to learn from."),("attempt","Your attempt","Say the same thought in your own voice.")] {
                    section {class:"panel audio-track","data-track":role,aria_label:title,
                        div {class:"audio-track-heading",div {span {class:"tag",if role=="reference" {"01 / LISTEN"}else{"02 / SPEAK"}}h2 {"{title}"}}span {class:"audio-track-mark",aria_hidden:"true",if role=="reference" {"A"}else{"B"}}}
                        p {class:"quiet","{description}"}
                        if role=="reference" {if let Some(generate)=on_generate_reference {button {class:"primary",disabled:generating_reference,onclick:move |_|generate.call(()),if generating_reference {"Generating reference…"}else{"Generate reference audio"}}}}
                        div {class:"audio-acquire",
                            button {class:"primary","data-action":"record","data-role":role,"data-record":"true",if role=="reference" {"● Record another voice"}else{if dialogue_scope.is_some(){"Record my message"}else{"Record my voice"}}}
                            if writing_scope.is_none(){label {class:"secondary audio-file",r#for:"audio-import-{role}",{if dialogue_scope.is_some(){"Import WAV"}else{"Import audio"}} input {id:"audio-import-{role}",name:"audio-import-{role}",onchange:move |event|import_native_file(event,role,error),r#type:"file",accept:if dialogue_scope.is_some(){"audio/wav,.wav"}else{"audio/*,.wav,.mp3,.m4a,.ogg,.flac"},"data-import":role}}}
                        }
                        label {r#for:"audio-library-{role}",if dialogue_scope.is_some(){"Saved takes for this message"}else{"Saved takes for this phrase"}}
                        select {id:"audio-library-{role}","data-library":role,aria_label:"Saved {title}"}
                        p {id:"audio-name-{role}",class:"audio-clip-name","No recording selected"}
                        div {class:"audio-plot",
                            span {class:"audio-plot-title","Waveform · original timing"}
                            canvas {id:"audio-wave-{role}",width:"960",height:"180",role:"img",aria_label:"{title} waveform; recording quality and duration are described below.","Waveform appears after recording or import."}
                            div {id:"audio-time-{role}",class:"audio-time-axis",aria_hidden:"true"}
                        }
                        div {class:"audio-span",
                            label {r#for:"audio-from-{role}",{"Start (s)"} input {id:"audio-from-{role}",r#type:"number",min:"0",step:"0.01",initial_value:"0",inputmode:"decimal"}}
                            label {r#for:"audio-to-{role}",{"End (s)"} input {id:"audio-to-{role}",r#type:"number",min:"0",step:"0.01",initial_value:"0",inputmode:"decimal"}}
                            button {class:"secondary",disabled:true,"data-needs":role,"data-action":"play","data-role":role,if compact {"Replay"}else{"▶ Play span"}}
                            if compact && role=="attempt" {button {class:"text-button","data-action":"stopPlayback","Stop playback"}}
                        }
                        details {class:"audio-details",summary {"Recording details & sound tools"}
                            p {id:"audio-provenance-{role}",class:"quiet audio-source","No source selected"}
                            if role=="reference" {if let Some(a)=reference_analysis.as_ref() {{quality_summary(a)}}}
                            else {if let Some(a)=attempt_analysis.as_ref() {{quality_summary(a)}}}
                            p {class:"quiet","Time runs left to right. Brighter bands mean more energy at a frequency. Voices and microphones differ; matching this picture is not the goal."}
                            div {class:"audio-plot",span {id:"audio-spectrum-label-{role}",class:"audio-plot-title","Spectrogram · frequency increases upward"}canvas {id:"audio-spectrum-{role}",width:"960",height:"180",role:"img",aria_label:"{title} spectrogram. Use playback and focused review to interpret frequency patterns.","Spectrogram appears after recording or import."}}
                            div {class:"audio-plot",span {class:"audio-plot-title","Estimated pitch · 50–400 Hz, logarithmic · confident voiced frames only"}canvas {id:"audio-pitch-{role}",width:"960",height:"180",role:"img",aria_label:"{title} estimated pitch. Gaps can mean unvoiced sound or uncertain estimation.","Pitch display appears after recording or import."}}
                            p {class:"quiet","Pitch can be wrong in breathy, noisy, creaky, or high voices. An octave jump is a reason to listen again. A different voice’s habitual pitch is not a target."}
                            label {r#for:"audio-note-{role}","One observation or practice cue for the selected span"}
                            textarea {id:"audio-note-{role}",rows:"2",maxlength:"2000",placeholder:"For example: listen to the stressed vowel, then try again."}
                            button {class:"secondary",disabled:true,"data-needs":role,"data-action":"cue","data-role":role,"Keep cue with this span"}
                            ul {id:"audio-cues-{role}",class:"audio-cues",aria_label:"Saved timed cues"}
                            button {class:"text-button",disabled:true,"data-needs":role,"data-action":"download","data-role":role,"Download original audio ↗"}
                        }
                    }
                }
            }
            details {id:"audio-region-lab",class:"panel audio-region-lab",
                summary {h2 {"Compare two regions"}span {"Choose a sound, replay it, and compare its timing and level."}}
                p {class:"quiet","Choose any saved recording for A and B, including two regions of the same recording. Drag across the waveform or spectrum. The highlighted boundaries are your selections, not automatic sound labels."}
                div {class:"audio-region-tools",
                    button {class:"secondary","data-region-action":"use-current","Use current recordings"}
                    button {class:"text-button","data-region-action":"copy-a","Use A’s recording for B"}
                    label {class:"audio-checkbox",input {id:"audio-region-zoom",r#type:"checkbox"}"Zoom around both selections"}
                    label {class:"audio-checkbox",input {id:"audio-region-pitch",r#type:"checkbox"}"Show contextual pitch"}
                }
                div {class:"audio-region-panes",
                    for (slot,label) in [("region_a","A"),("region_b","B")] {
                        section {class:"audio-region-pane","data-region-slot":slot,
                            h3 {"Region {label}"}
                            label {r#for:"audio-region-source-{slot}","Recording"}select {id:"audio-region-source-{slot}","data-region-source":slot,option {value:"","Choose a saved recording…"}}
                            p {id:"audio-region-identity-{slot}",class:"audio-region-identity","No source selected"}
                            for (kind,title) in [("wave","Waveform"),("spectrum","Spectrogram"),("pitch","Contextual pitch · 50–400 Hz")] {
                                div {class:"audio-region-plot","data-region-track":kind,
                                    span {class:"audio-plot-title","{title}"}
                                    div {class:"audio-region-surface","data-region-surface":slot,"data-region-kind":kind,
                                        canvas {id:"audio-{kind}-{slot}",width:"960",height:"180",role:"img",aria_label:"Region {label} {title}"}
                                        div {class:"audio-region-highlight","data-region-highlight":slot,aria_hidden:"true"}
                                        for edge in ["start","end"] {button {class:"audio-region-handle",disabled:true,r#type:"button",role:"slider",tabindex:if kind=="wave"{0}else{-1},aria_hidden:if kind=="wave"{"false"}else{"true"},"data-region-edge":edge,"data-region-owner":slot,aria_label:"Region {label} {edge} boundary",aria_describedby:"audio-region-help-{slot}",aria_valuemin:"0",aria_valuemax:"0",aria_valuenow:"0",span {aria_hidden:"true","⋮"}}}
                                    }
                                }
                            }
                            div {id:"audio-time-{slot}",class:"audio-time-axis",aria_hidden:"true"}
                            p {id:"audio-region-scale-{slot}",class:"quiet audio-region-scale"}
                            div {class:"audio-region-boundaries",
                                label {r#for:"audio-region-from-{slot}",{"Start (seconds)"} input {id:"audio-region-from-{slot}",r#type:"number",min:"0",step:"0.001",initial_value:"0","data-region-seconds":slot}}
                                label {r#for:"audio-region-to-{slot}",{"End (seconds)"} input {id:"audio-region-to-{slot}",r#type:"number",min:"0",step:"0.001",initial_value:"0","data-region-seconds":slot}}
                            }
                            details {class:"audio-region-precise",summary {"Exact frame boundaries & keyboard help"}
                                p {id:"audio-region-help-{slot}",class:"quiet","Each boundary is a source PCM frame. End is exclusive. Focus a boundary handle, then use arrows to move one frame; Shift moves 10, Alt moves 100. Home and End move to the available edge."}
                                div {class:"audio-region-boundaries",
                                    label {r#for:"audio-region-first-{slot}",{"Start frame"} input {id:"audio-region-first-{slot}",r#type:"number",min:"0",step:"1",initial_value:"0","data-region-frames":slot}}
                                    label {r#for:"audio-region-last-{slot}",{"End frame"} input {id:"audio-region-last-{slot}",r#type:"number",min:"1",step:"1",initial_value:"0","data-region-frames":slot}}
                                }
                                button {class:"text-button","data-region-action":"reset","data-region-role":slot,"Select whole recording"}
                            }
                            p {id:"audio-region-range-{slot}",class:"audio-region-range"}
                            p {id:"audio-region-metrics-{slot}",class:"audio-region-metrics",role:"status",aria_live:"polite","Choose a recording to measure a region."}
                            div {class:"actions",
                                button {class:"secondary",disabled:true,"data-region-play":slot,"▶ Play {label}"}
                                button {class:"text-button",disabled:true,"data-region-export":slot,"Save boundaries"}
                            }
                        }
                    }
                }
                div {class:"audio-region-tools",
                    label {r#for:"audio-region-rate","Playback speed"}select {id:"audio-region-rate",option {value:"1","Natural · 1×"}option {value:"0.85","0.85× · keep voice pitch"}option {value:"0.7","0.7× · keep voice pitch"}option {value:"0.5","0.5× · keep voice pitch"}}
                    label {class:"audio-checkbox",input {id:"audio-region-loop",r#type:"checkbox"}"Loop one region"}
                    button {id:"audio-region-compare",class:"primary",disabled:true,"data-region-action":"compare","A → B · selected regions"}
                    button {class:"secondary","data-action":"stopPlayback","Stop playback"}
                }
                p {id:"audio-region-comparison",class:"audio-region-result",role:"status",aria_live:"polite","Select both regions to compare."}
                p {id:"audio-region-whole",class:"quiet"}
                p {class:"audio-principle","Both plots use the same seconds-per-pixel, frequency and level scales. Zoom translates the view around each selection; it does not stretch sounds to equal widths. Level depends on microphone, voice and gain. A longer or louder region does not by itself identify stress or a correct vowel."}
                p {id:"audio-region-persistence",class:"quiet"}
            }
            section {class:"panel audio-comparison",h2 {"Listen side by side"}
                div {class:"audio-playback-controls",
                    label {r#for:"audio-rate","Playback speed"}select {id:"audio-rate",option {value:"1",initial_selected:true,"Natural · 1×"}option {value:"0.85","0.85× · keep voice pitch"}option {value:"0.7","0.7× · keep voice pitch"}option {value:"0.5","0.5× · keep voice pitch"}}
                    label {class:"audio-checkbox",input {id:"audio-loop",r#type:"checkbox"}"Loop a single span"}
                    button {id:"audio-compare",class:"primary",disabled:true,"data-action":"compare","A → B · reference, then me"}
                    button {class:"secondary","data-action":"stopPlayback","Stop playback"}
                }
                p {id:"audio-speed-support",class:"quiet","A → B uses each recording’s selected span, with a short pause. Slower playback keeps voice pitch using your device’s audio engine. Time stretching can add artifacts; return to natural speed to check the sound."}
                if let Some(c) = comparison {
                    div {class:"audio-measures",span {"Whole-take duration: {c.reference_duration_seconds:.2} s → {c.attempt_duration_seconds:.2} s"}span {"Relative duration: {c.duration_ratio:.2}×"}span {"Recording level difference: {c.rms_delta_db:+.1} dB"}}
                    for warning in c.warnings {p {class:"quiet","{warning}"}}
                }
                p {class:"audio-principle","These are listening aids, not an accent score. Compare stress, timing, and a specific sound with a useful reference; voice, room, and microphone differences also appear in the plots."}
                details {class:"audio-details",summary {"Spectrogram display settings"}
                    div {class:"audio-display-settings",label {r#for:"audio-frequency","Frequency ceiling"}select {id:"audio-frequency",option {value:"4000","4,000 Hz"}option {value:"8000",initial_selected:true,"8,000 Hz"}}label {r#for:"audio-floor","Display floor"}select {id:"audio-floor",option {value:"-60","−60 dBFS"}option {value:"-80",initial_selected:true,"−80 dBFS"}option {value:"-100","−100 dBFS"}}}
                    p {class:"quiet","Both takes use the same scale and their actual durations. Changing the display does not change your originals or measurements."}
                }
            }
            section {class:"panel audio-storage",h2 {"Keep your voices"}p {"Recordings and timed cues save on this device. Include originals in Settings when you want to sync them, and download a backup to keep a separate copy."}
                div {class:"actions",button {class:"secondary","data-action":"backup","Download audio backup"}label {class:"secondary audio-file",r#for:"audio-restore",{"Restore audio backup"} input {id:"audio-restore",name:"audio-restore",onchange:move |event|import_native_file(event,"restore",error),r#type:"file",accept:"application/json,.json","data-restore":"true"}}}
                p {class:"quiet","Short takes work best: recording stops after 30 seconds; imported clips may be up to 60 seconds. Leaving this screen releases the microphone. Microphone availability depends on your browser or app’s device permissions."}
            }
            }
        }
    }
}

/// One compact speaking editor at a time. Key by task/revision when its binding changes.
/// Existing takes are selected explicitly; new originals arrive only through `on_saved`.
#[component]
pub fn AudioSpeaking(
    passage_id: String,
    revision_id: String,
    on_saved: EventHandler<StudioRecording>,
    #[props(default)] selected_asset_id: Option<String>,
    #[props(default)] selected_content_hash: Option<String>,
    #[props(default)] on_selected: EventHandler<Option<StudioRecording>>,
    #[props(default)] on_recording_state: EventHandler<bool>,
    #[props(default)] on_reference_played: EventHandler<()>,
    #[props(default)] on_region_played: EventHandler<StudioRegionPlayback>,
    #[props(default)] on_generate_reference: Option<EventHandler<()>>,
    #[props(default)] generating_reference: bool,
    #[props(default)] dialogue_scope: Option<DialogueRecorderScope>,
    #[props(default)] sound_focus_scope: Option<SoundFocusRecorderScope>,
    #[props(default)] writing_scope: Option<WritingRecorderScope>,
    #[props(default)] frozen_reference_asset_id: Option<String>,
    #[props(default)] frozen_reference_hash: Option<String>,
    #[props(default)] review_only: bool,
    #[props(default)] workspace_id: String,
) -> Element {
    rsx! {AudioStudio {passage_id,revision_id,workspace_id,compact:true,review_only,selected_asset_id,selected_content_hash,on_saved,on_selected,on_recording_state,on_reference_played,on_region_played,on_generate_reference,generating_reference,dialogue_scope,sound_focus_scope,writing_scope,frozen_reference_asset_id,frozen_reference_hash}}
}

fn quality_summary(analysis: &AudioAnalysis) -> Element {
    let q = &analysis.quality;
    let clipped_percent = q.clipping_fraction * 100.0;
    rsx! {
        div {class:"audio-quality",aria_label:"Recording quality",
            span {"{analysis.duration_seconds:.2} s"}span {"Level {q.rms_dbfs:.1} dBFS"}span {"Near-full-scale samples {clipped_percent:.2}%"}
            if q.flags.is_empty() {span {class:"audio-quality-ok","No basic level warning"}}
            for flag in &q.flags {span {class:"audio-quality-warning","{flag.label()}"}}
        }
    }
}

pub(crate) const PRACTICE_BRIDGE: &str = r#"
const root=typeof rufrenPlayerRoot==='undefined'?document.getElementById('rufren-audio-practice'):rufrenPlayerRoot;
if(!root) throw Error('Reference player did not mount.');
window.rufrenAudioPlayers ||= new Map();window.rufrenAudioPlayers.get(root.id)?.destroy();
let alive=true,db,record,finish,url,prepareTimer,cancelPrepare,loadGeneration=0,playGeneration=0;
// Completion is opt-in for blind Sound Focus trials. Ordinary players retain
// their existing successful-start acknowledgment semantics.
const playbackToken=root.dataset.playbackToken,expectedHash=root.dataset.expectedHash,expectedSourceHash=root.dataset.sourceTextSha256;
const modelExample=root.dataset.playbackContext==='example';
let activePlayback;
const lifetime=new Promise(resolve=>finish=resolve);
const status=root.querySelector('[data-practice-status]');
const buttons=root.querySelectorAll('[data-practice-rate]'),picker=root.querySelector('[data-reference-select]'),sourceLabel=root.querySelector('[data-reference-source]');
const media=document.createElement('audio');media.preload='auto';media.hidden=true;media.setAttribute('data-practice-media','true');root.append(media);
const preservePitchKey=['preservesPitch','webkitPreservesPitch','mozPreservesPitch'].find(key=>key in media);
if(preservePitchKey)media[preservePitchKey]=true;
const send=value=>{if(alive)dioxus.send(value);};
function stop(){playGeneration++;activePlayback=null;media.onended=null;media.pause();try{media.currentTime=0;}catch(_){}}
function destroy(){if(!alive)return;alive=false;finish();cancelPrepare?.();clearTimeout(prepareTimer);stop();media.removeAttribute('src');media.load();media.remove();if(url)URL.revokeObjectURL(url);db?.close();root.removeEventListener('click',click);root.removeEventListener('change',change);window.removeEventListener('rufren-audio-assets-changed',assetsChanged);window.removeEventListener('pagehide',destroy);document.removeEventListener('visibilitychange',visibility);observer.disconnect();window.rufrenAudioPlayers.delete(root.id);}
function visibility(){if(document.hidden)stop();}
function click(event){
 const button=event.target.closest('button');if(!button)return;
 if(button.dataset.practiceStop!==undefined){stop();status.textContent='Playback stopped.';return;}
 if(!button.dataset.practiceRate||!record)return;
 const rate=Number(button.dataset.practiceRate);
 if(playbackToken!==undefined&&rate!==1){status.textContent='Listen at natural speed for this question.';return;}
 if(rate!==1&&!preservePitchKey){status.textContent='Pitch-preserving slower playback is unavailable here. Listen at natural speed.';return;}
 for(const [id,api]of window.rufrenAudioPlayers)if(id!==root.id)api.stop?.();
 window.rufrenAudioStudio?.stopPlayback?.();
 stop();const generation=playGeneration,load=loadGeneration,playedRecord=record;media.playbackRate=rate;if(preservePitchKey)media[preservePitchKey]=true;
 const playbackId=crypto.randomUUID();const proof={id:playbackId,started:false,completed:false};activePlayback=proof;
 const completed=()=>{
  if(playbackToken===undefined||!alive||activePlayback!==proof||!proof.started||proof.completed||generation!==playGeneration||load!==loadGeneration||root.dataset.playbackToken!==playbackToken||!media.ended||media.playbackRate!==1||rate!==1)return;
  proof.completed=true;const {blob,...recording}=playedRecord;status.textContent=modelExample?'Example played through.':'Playback finished. Choose the meaning you heard.';send({kind:'completed',recording,playback_token:playbackToken,playback_id:playbackId});
 };
 if(playbackToken!==undefined)media.onended=completed;
 // The original is verified and prepared before readiness; play stays in the click.
 media.play().then(()=>{
  if(!alive||generation!==playGeneration||load!==loadGeneration)return;
  if(playbackToken!==undefined&&root.dataset.playbackToken!==playbackToken)return;
  proof.started=true;status.textContent=rate===1?'Playing the selected recording.':'Playing slowly.';const {blob,...recording}=playedRecord;send({kind:'played',recording,...(playbackToken===undefined?{}:{playback_token:playbackToken,playback_id:playbackId})});completed();
 }).catch(error=>{if(alive&&generation===playGeneration)status.textContent=`Playback could not start. Tap Listen again, or check this app’s audio permission. ${error.message}`;});
}
function change(event){if(event.target===picker){root.dataset.selectedReference=picker.value;prepare();}}
function assetsChanged(event){const d=event.detail;if(!d)return;if(root.dataset.asset!==undefined){if(d.asset_id===root.dataset.asset)prepare();}else if(d.revision_id===root.dataset.revision){root.dataset.selectedReference=d.asset_id;prepare();}}
root.addEventListener('click',click);root.addEventListener('change',change);window.addEventListener('rufren-audio-assets-changed',assetsChanged);window.addEventListener('pagehide',destroy);document.addEventListener('visibilitychange',visibility);
const observer=new MutationObserver(()=>{if(!root.isConnected)destroy();});observer.observe(document.body,{childList:true,subtree:true});const playerApi={destroy,stop};window.rufrenAudioPlayers.set(root.id,playerApi);if(root.id==='rufren-audio-practice')window.rufrenAudioPractice=playerApi;
function referenceSource(r){const p=r.device?.provenance;if(p?.kind==='ai_generated_speech')return `AI speech · ${p.provider} · ${p.voice}`;if(p?.kind==='system_generated_speech')return `System speech · ${p.voice}`;return 'Provided voice';}
async function prepare(){
 const generation=++loadGeneration;cancelPrepare?.();clearTimeout(prepareTimer);stop();record=null;
 const cancellation=new Promise(resolve=>cancelPrepare=resolve);
 buttons.forEach(b=>b.disabled=true);send({kind:'ready',ready:false});
 try{
  if(!window.indexedDB)throw Error('Saved audio is unavailable in this WebView. Use a supported browser or app.');
  if(!db)db=await new Promise((resolve,reject)=>{const r=indexedDB.open('rufren-audio-v1',1);r.onupgradeneeded=()=>r.result.createObjectStore('assets',{keyPath:'asset_id'});r.onsuccess=()=>{if(!alive){r.result.close();reject(Error('Player closed.'));}else resolve(r.result);};r.onerror=()=>reject(r.error);r.onblocked=()=>reject(Error('Close another recording tab, then retry.'));});
  if(!alive||generation!==loadGeneration)return;
  let selected;
  if(root.dataset.asset!==undefined){
   selected=await new Promise((resolve,reject)=>{const r=db.transaction('assets').objectStore('assets').get(root.dataset.asset);r.onsuccess=()=>resolve(r.result);r.onerror=()=>reject(r.error);});
   if(!selected)throw Error('This exact recording is not stored on this device. Sync recordings in Settings or restore an audio backup.');
   if(selected.asset_id!==root.dataset.asset)throw Error('The selected recording identity did not match. Existing audio was kept.');
   if(expectedHash!==undefined&&selected.content_hash!==expectedHash)throw Error('This question’s exact audio identity did not match. Reopen its saved source.');
  }else{
   const records=await new Promise((resolve,reject)=>{const r=db.transaction('assets').objectStore('assets').getAll();r.onsuccess=()=>resolve(r.result);r.onerror=()=>reject(r.error);});
   const machine=r=>['ai_generated_speech','system_generated_speech'].includes(r.device?.provenance?.kind)?1:0;
   const references=records.filter(r=>r.role==='reference'&&r.revision_id===root.dataset.revision&&!r.interrupted&&(expectedSourceHash===undefined||(machine(r)&&r.device?.provenance?.source_text_sha256===expectedSourceHash))).sort((a,b)=>machine(b)-machine(a)||b.created_at_ms-a.created_at_ms);
   selected=references.find(r=>r.asset_id===root.dataset.selectedReference)||references[0];
   if(picker){picker.replaceChildren(...references.map((r,i)=>new Option(root.dataset.hideReferenceName==='true'?`${referenceSource(r)} · Voice ${i+1} · ${(r.duration_ms/1000).toFixed(1)}s`:`${referenceSource(r)} · ${r.name}`,r.asset_id)));picker.disabled=!references.length;if(selected)picker.value=selected.asset_id;}
   if(!selected)throw Error(expectedSourceHash===undefined?'Generate reference audio for this phrase. You can also sync an existing reference or add a provided voice in the speech studio.':'Create audio for this exact text, or sync its saved original in Settings.');
  }
  if(!(selected.blob instanceof Blob)||selected.byte_length!==selected.blob.size||!/^[0-9a-f]{64}$/.test(selected.content_hash))throw Error('The stored audio metadata is invalid. Restore the original from an audio backup.');
  if(!crypto.subtle)throw Error('Use HTTPS or localhost to check and play saved audio.');
  if(expectedSourceHash!==undefined){
   const p=selected.device?.provenance;
   if(typeof p?.source_text!=='string'||p.source_text_sha256!==expectedSourceHash)throw Error('This voice does not match the selected text. Its original was kept.');
   const textHash=Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',new TextEncoder().encode(p.source_text))),n=>n.toString(16).padStart(2,'0')).join('');
   if(textHash!==expectedSourceHash)throw Error('This recording could not be checked against its text. Create the audio again.');
  }
  const bytes=await selected.blob.arrayBuffer();
  const hash=Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',bytes)),n=>n.toString(16).padStart(2,'0')).join('');
  if(hash!==selected.content_hash)throw Error('This audio failed its integrity check. Restore your audio backup.');
  if(!alive||generation!==loadGeneration)return;
  if(url)URL.revokeObjectURL(url);url=URL.createObjectURL(selected.blob);
  await Promise.race([new Promise((resolve,reject)=>{
   prepareTimer=setTimeout(()=>reject(Error('The reference could not be prepared. Reopen this practice to retry.')),10000);
   media.onloadedmetadata=()=>{clearTimeout(prepareTimer);media.onloadedmetadata=media.onerror=null;resolve();};
   media.onerror=()=>{clearTimeout(prepareTimer);reject(Error('This device could not play the saved original. Import a WAV or MP3 reference.'));};
   media.src=url;media.load();
  }),lifetime.then(()=>{throw Error('Reference player closed.');}),cancellation.then(()=>{throw Error('Reference selection changed.');})]);
  if(alive&&generation===loadGeneration){record=selected;if(sourceLabel)sourceLabel.textContent=referenceSource(record);buttons.forEach(b=>b.disabled=Number(b.dataset.practiceRate)!==1&&!preservePitchKey);const readyLabel=root.dataset.asset!==undefined?'Recording':'Reference';status.textContent=modelExample?'Ready. Listen to the complete example.':playbackToken!==undefined?'Ready. Listen to the whole sound, then choose its meaning.':preservePitchKey?`${readyLabel} ready.`:`${readyLabel} ready. Slow replay is unavailable on this device.`;send({kind:'ready',ready:true});}
 }catch(error){if(alive&&generation===loadGeneration){status.textContent=error.message;send({kind:'ready',ready:false});}}
}
await prepare();
await lifetime;
"#;

#[derive(Deserialize)]
pub(crate) struct PracticeAudioEvent {
    pub(crate) kind: String,
    #[serde(default)]
    pub(crate) ready: bool,
    #[serde(default)]
    pub(crate) recording: Option<StudioRecording>,
    #[serde(default)]
    pub(crate) playback_token: Option<String>,
    #[serde(default)]
    pub(crate) playback_id: Option<String>,
}

/// Audio-only playback for listening practice. No answer text is rendered.
#[component]
pub fn AudioPractice(
    revision_id: String,
    on_played: EventHandler<()>,
    on_ready: EventHandler<bool>,
    #[props(default)] on_played_recording: EventHandler<StudioRecording>,
    #[props(default)] on_generate: Option<EventHandler<()>>,
    #[props(default)] generating: bool,
    #[props(default)] instance_id: Option<String>,
) -> Element {
    let element_id = use_hook(|| {
        instance_id
            .filter(|id| !id.is_empty())
            .unwrap_or_else(|| "rufren-audio-practice".into())
    });
    let voice_id = format!("{element_id}-reference-voice");
    let mut error = use_signal(String::new);
    let effect_id = element_id.clone();
    use_effect(move || {
        let script = format!(
            "const rufrenPlayerRoot=document.getElementById({});\n{}",
            serde_json::to_string(&effect_id).unwrap_or_default(),
            PRACTICE_BRIDGE
        );
        spawn(async move {
            let mut bridge = document::eval(&script);
            while let Ok(event) = bridge.recv::<PracticeAudioEvent>().await {
                if event.kind == "played" {
                    on_played.call(());
                    if let Some(recording) = event.recording {
                        on_played_recording.call(recording);
                    }
                } else if event.kind == "ready" {
                    on_ready.call(event.ready);
                }
            }
            error.set(
                "The reference player disconnected. Reopen this practice to reconnect.".into(),
            );
        });
    });
    let drop_id = element_id.clone();
    use_drop(move || {
        let id = serde_json::to_string(&drop_id).unwrap_or_default();
        let _ = document::eval(&format!("window.rufrenAudioPlayers?.get({id})?.destroy();"));
    });
    rsx! {
        style {"{STYLE}"}
        section {id:element_id,class:"audio-practice","data-revision":revision_id,"data-hide-reference-name":"true",aria_label:"Listen to the reference voice",
            if let Some(generate)=on_generate {button {class:"secondary",disabled:generating,onclick:move |_|generate.call(()),if generating {"Generating reference…"}else{"Generate reference audio"}}}
            label {r#for:voice_id.clone(),"Reference voice"}
            select {id:voice_id,disabled:true,"data-reference-select":"true"}
            details {summary {"Voice source"}p {class:"quiet audio-source","data-reference-source":"true"}}
            div {class:"actions",
                button {class:"primary",disabled:true,"data-practice-rate":"1","Listen"}
                button {class:"secondary",disabled:true,"data-practice-rate":"0.7","Replay slowly"}
                button {class:"text-button","data-practice-stop":"true","Stop"}
            }
            p {class:"quiet",role:"status","data-practice-status":"true","Opening reference audio…"}
            if !error.read().is_empty() {p {role:"alert","{error}"}}
        }
    }
}

/// Replay the exact original linked to a selected attempt. It never falls back
/// to another take or records learning evidence.
#[component]
pub fn AudioReview(
    asset_id: String,
    #[props(default)] reference_asset_id: Option<String>,
) -> Element {
    rsx! {
        section {class:"audio-review",aria_label:"Exact attempt recording",
            AudioReviewPlayer {key:"attempt-{asset_id}",asset_id,label:"This attempt".to_string()}
            if let Some(reference) = reference_asset_id {
                AudioReviewPlayer {key:"reference-{reference}",asset_id:reference,label:"Selected reference voice".to_string()}
            }
        }
    }
}

#[component]
pub(crate) fn AudioReviewPlayer(
    asset_id: String,
    label: String,
    #[props(default)] expected_hash: Option<String>,
    #[props(default)] on_played: EventHandler<StudioRecording>,
) -> Element {
    let element_id = use_hook(|| format!("rufren-audio-review-{}", uuid::Uuid::new_v4()));
    let mut error = use_signal(String::new);
    let effect_id = element_id.clone();
    use_effect(move || {
        let script = format!(
            "const rufrenPlayerRoot=document.getElementById({});\n{}",
            serde_json::to_string(&effect_id).unwrap_or_default(),
            PRACTICE_BRIDGE
        );
        spawn(async move {
            let mut bridge = document::eval(&script);
            while let Ok(event) = bridge.recv::<PracticeAudioEvent>().await {
                if event.kind == "played" {
                    if let Some(recording) = event.recording {
                        on_played.call(recording);
                    }
                }
            }
            error.set("The recording player disconnected. Reopen this review to reconnect.".into());
        });
    });
    let drop_id = element_id.clone();
    use_drop(move || {
        let script = format!(
            "window.rufrenAudioPlayers?.get({})?.destroy();",
            serde_json::to_string(&drop_id).unwrap_or_default()
        );
        let _ = document::eval(&script);
    });
    rsx! {
        style {"{STYLE}"}
        div {id:element_id,class:"audio-practice audio-review-player","data-asset":asset_id,"data-expected-hash":expected_hash,
            h3 {"{label}"}
            div {class:"actions",
                button {class:"primary",disabled:true,"data-practice-rate":"1","Replay"}
                button {class:"secondary",disabled:true,"data-practice-rate":"0.7","Replay slowly"}
                button {class:"text-button","data-practice-stop":"true","Stop"}
            }
            p {class:"quiet",role:"status","data-practice-status":"true","Opening recording…"}
            if !error.read().is_empty() {p {role:"alert","{error}"}}
        }
    }
}

#[cfg(test)]
mod generated_reference_tests {
    use super::*;
    use std::{
        future::Future,
        task::{Context, Poll, Waker},
    };
    fn envelope() -> serde_json::Value {
        let pcm = PcmAudio::from_mono(24_000, vec![0.1; 2400]).unwrap();
        let bytes = pcm.to_wav();
        serde_json::json!({"source_text":"Приве́т!","request_id":"request-1","revision_id":"22222222-2222-4222-8222-222222222222","wav_base64":STANDARD.encode(&bytes),"sha256":format!("{:x}",Sha256::digest(&bytes)),"sample_rate":24_000,"channels":1,"frames":2400,"provenance":{"kind":"system_generated_speech","provider":"macos_say","model":"Milena","voice":"Milena"}})
    }
    fn rejected(response: serde_json::Value) -> String {
        let mut future = std::pin::pin!(save_generated_reference(
            "11111111-1111-4111-8111-111111111111",
            "22222222-2222-4222-8222-222222222222",
            "Приве́т!",
            "request-1",
            response
        ));
        match future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
        {
            Poll::Ready(Err(error)) => error,
            _ => panic!("Invalid generated audio reached the renderer/storage boundary"),
        }
    }
    #[test]
    fn generated_reference_rejects_stale_text_request_and_revision_before_storage() {
        for (key, value) in [
            ("source_text", "Пока!"),
            ("request_id", "another-request"),
            ("revision_id", "33333333-3333-4333-8333-333333333333"),
        ] {
            let mut response = envelope();
            response[key] = value.into();
            assert!(rejected(response).contains("exact phrase and request"));
        }
    }
    #[test]
    fn generated_reference_rejects_modified_bytes_and_forged_pcm_metadata() {
        let mut response = envelope();
        response["sha256"] = "0".repeat(64).into();
        assert!(rejected(response).contains("SHA-256"));
        let mut response = envelope();
        response["frames"] = 2401.into();
        assert!(rejected(response).contains("WAV metadata"));
        let mut response = envelope();
        response["provenance"]["kind"] = "unlabelled_voice".into();
        assert!(rejected(response).contains("machine speech provenance"));
    }
}
