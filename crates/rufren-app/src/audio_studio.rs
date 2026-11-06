//! Speech studio UI; the JS adapter handles devices and binary storage, Rust handles DSP.
use base64::{Engine as _, engine::general_purpose::STANDARD};
use dioxus::prelude::*;
use rufren_audio::{AudioAnalysis, AudioError, PcmAudio, analyze_cooperatively, compare};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::rc::Rc;

const BRIDGE: &str = include_str!("audio_bridge.js");
const STYLE: &str = include_str!("audio_studio.css");

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

/// Save one generated original, bound to the exact phrase revision and request.
/// Network/provider configuration is owned by the shared gateway client.
pub async fn save_generated_reference(
    passage_id: &str,
    revision_id: &str,
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
        || generated.revision_id.as_deref() != Some(revision_id)
    {
        return Err(
            "Generated audio does not match this exact phrase and request. Nothing was attached."
                .into(),
        );
    }
    if !uuid::Uuid::parse_str(passage_id).is_ok_and(|id| !id.is_nil())
        || !uuid::Uuid::parse_str(revision_id).is_ok_and(|id| !id.is_nil())
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
    provenance.insert("revision_id".into(), revision_id.into());
    provenance.insert("request_id".into(), expected_request_id.into());
    let voice = provenance
        .get("voice")
        .and_then(|v| v.as_str())
        .unwrap_or("Generated voice");
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
        device: serde_json::json!({"provenance":provenance,"source_mime_type":"audio/wav"}),
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

#[derive(Deserialize)]
struct BridgeEvent {
    kind: String,
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
}

#[derive(Serialize)]
struct PlotMessage<'a> {
    kind: &'static str,
    role: &'a str,
    analysis: PlotAnalysis<'a>,
}
#[derive(Serialize)]
struct PlotAnalysis<'a> {
    duration_seconds: f64,
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
fn plot_message<'a>(role: &'a str, analysis: &'a AudioAnalysis) -> PlotMessage<'a> {
    let spectrum = &analysis.spectrogram;
    let bytes: Vec<u8> = spectrum
        .power_db
        .iter()
        .flat_map(|f| f.to_le_bytes())
        .collect();
    PlotMessage {
        kind: "analysis",
        role,
        analysis: PlotAnalysis {
            duration_seconds: analysis.duration_seconds,
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

async fn yield_analysis(role: &str, asset_id: &str) -> bool {
    let attribute = serde_json::to_string(&format!("data-selected-{role}")).unwrap_or_default();
    let asset_id = serde_json::to_string(asset_id).unwrap_or_default();
    let code = format!(
        "await window.rufrenAudioStudio?.yieldNow(); return document.getElementById('rufren-audio-studio')?.getAttribute({attribute}) === {asset_id};"
    );
    document::eval(&code)
        .await
        .ok()
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}
async fn analyze_event(
    event: &BridgeEvent,
    asset_id: &str,
) -> Result<Option<AudioAnalysis>, String> {
    if event.samples.len() > 90 * 1024 * 1024 {
        return Err("The PCM message exceeds the instrument limit".into());
    }
    let bytes = STANDARD.decode(&event.samples).map_err(|e| e.to_string())?;
    if bytes.len() % 4 != 0 || bytes.len() > 64 * 1024 * 1024 {
        return Err("Invalid PCM byte count".into());
    }
    let samples = bytes
        .chunks_exact(4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect();
    let pcm = match PcmAudio::from_interleaved_cooperatively(
        event.sample_rate,
        event.channels,
        samples,
        || yield_analysis(&event.role, asset_id),
    )
    .await
    {
        Ok(pcm) => pcm,
        Err(AudioError::Cancelled) => return Ok(None),
        Err(e) => return Err(e.to_string()),
    };
    match analyze_cooperatively(&pcm, || yield_analysis(&event.role, asset_id)).await {
        Ok(analysis) => Ok(Some(analysis)),
        Err(AudioError::Cancelled) => Ok(None),
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
) -> Element {
    let mut reference = use_signal(|| None::<Rc<AudioAnalysis>>);
    let mut attempt = use_signal(|| None::<Rc<AudioAnalysis>>);
    let mut error = use_signal(String::new);
    let config = serde_json::json!({"passage_id":passage_id,"revision_id":revision_id}).to_string();
    use_effect(move || {
        spawn(async move {
            // Effects run after mounting. The long-lived eval keeps a bidirectional channel.
            let mut bridge = document::eval(BRIDGE);
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
                        error.set(String::new());
                        let asset_id = event
                            .recording
                            .as_ref()
                            .map(|r| r.asset_id.as_str())
                            .unwrap_or("");
                        if event.role == "reference" {
                            reference.set(None);
                        } else {
                            attempt.set(None);
                        }
                        let decoded = analyze_event(&event, asset_id).await;
                        match decoded {
                            Ok(Some(analysis)) => {
                                let message = plot_message(&event.role, &analysis);
                                if let Err(e) = bridge.send(message) {
                                    error.set(format!("Could not draw the analysis: {e}"));
                                }
                                if event.role == "reference" {
                                    reference.set(Some(Rc::new(analysis)));
                                } else {
                                    attempt.set(Some(Rc::new(analysis)));
                                }
                            }
                            Ok(None) => {}
                            Err(e) => error.set(format!(
                                "The original is available, but analysis could not run: {e}"
                            )),
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
    use_drop(|| {
        let _ = document::eval("window.rufrenAudioStudio?.destroy();");
    });
    let reference_analysis = reference.read().clone();
    let attempt_analysis = attempt.read().clone();
    let comparison = reference_analysis
        .as_ref()
        .zip(attempt_analysis.as_ref())
        .map(|(r, a)| compare(r, a));
    rsx! {
        style {"{STYLE}"}
        section {id:"rufren-audio-studio",class:"audio-studio","data-config":config,
            if !russian.is_empty() {div {class:"panel audio-passage",span {class:"tag","YOUR PHRASE"}p {class:"russian-display",lang:"ru","{russian}"}}}
            div {class:"panel audio-session-bar",
                div {id:"audio-mic-status",class:"audio-mic",role:"status","data-active":"false","Microphone off"}
                button {id:"audio-stop-recording",class:"audio-stop",disabled:true,"data-action":"stop","Stop & keep take"}
                p {id:"audio-status",role:"status",aria_live:"polite","Opening audio storage…"}
                if !error.read().is_empty() {p {class:"audio-error",role:"alert","{error}"}}
            }
            div {class:"audio-tracks",
                for (role,title,description) in [("reference","Reference voice","Generate a Russian example, or add a voice you want to learn from."),("attempt","Your attempt","Say the same thought in your own voice.")] {
                    section {class:"panel audio-track",aria_label:title,
                        div {class:"audio-track-heading",div {span {class:"tag",if role=="reference" {"01 / LISTEN"}else{"02 / SPEAK"}}h2 {"{title}"}}span {class:"audio-track-mark",aria_hidden:"true",if role=="reference" {"A"}else{"B"}}}
                        p {class:"quiet","{description}"}
                        if role=="reference" {if let Some(generate)=on_generate_reference {button {class:"primary",disabled:generating_reference,onclick:move |_|generate.call(()),if generating_reference {"Generating reference…"}else{"Generate reference audio"}}}}
                        div {class:"audio-acquire",
                            button {class:"primary","data-action":"record","data-role":role,"data-record":"true",if role=="reference" {"● Record another voice"}else{"● Record my attempt"}}
                            label {class:"secondary audio-file",r#for:"audio-import-{role}",{"Import audio"} input {id:"audio-import-{role}",name:"audio-import-{role}",onchange:move |event|import_native_file(event,role,error),r#type:"file",accept:"audio/*,.wav,.mp3,.m4a,.ogg,.flac","data-import":role}}
                        }
                        label {r#for:"audio-library-{role}","Saved takes for this phrase"}
                        select {id:"audio-library-{role}","data-library":role,aria_label:"Saved {title}"}
                        p {id:"audio-name-{role}",class:"audio-clip-name","No recording selected"}
                        p {id:"audio-provenance-{role}",class:"quiet audio-source","No source selected"}
                        div {class:"audio-plot",
                            span {class:"audio-plot-title","Waveform · original timing"}
                            canvas {id:"audio-wave-{role}",width:"960",height:"180",role:"img",aria_label:"{title} waveform; recording quality and duration are described below.","Waveform appears after recording or import."}
                            div {id:"audio-time-{role}",class:"audio-time-axis",aria_hidden:"true"}
                        }
                        div {class:"audio-span",
                            label {r#for:"audio-from-{role}",{"Start (s)"} input {id:"audio-from-{role}",r#type:"number",min:"0",step:"0.01",initial_value:"0",inputmode:"decimal"}}
                            label {r#for:"audio-to-{role}",{"End (s)"} input {id:"audio-to-{role}",r#type:"number",min:"0",step:"0.01",initial_value:"0",inputmode:"decimal"}}
                            button {class:"secondary",disabled:true,"data-needs":role,"data-action":"play","data-role":role,"▶ Play span"}
                        }
                        if role=="reference" {if let Some(a)=reference_analysis.as_ref() {{quality_summary(a)}}}
                        else {if let Some(a)=attempt_analysis.as_ref() {{quality_summary(a)}}}
                        details {class:"audio-details",summary {"Inspect sound & add a cue"}
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

const PRACTICE_BRIDGE: &str = r#"
const root=typeof rufrenPlayerRoot==='undefined'?document.getElementById('rufren-audio-practice'):rufrenPlayerRoot;
if(!root) throw Error('Reference player did not mount.');
window.rufrenAudioPlayers ||= new Map();window.rufrenAudioPlayers.get(root.id)?.destroy();
let alive=true,db,record,finish,url,prepareTimer,cancelPrepare,loadGeneration=0,playGeneration=0;
const lifetime=new Promise(resolve=>finish=resolve);
const status=root.querySelector('[data-practice-status]');
const buttons=root.querySelectorAll('[data-practice-rate]'),picker=root.querySelector('[data-reference-select]'),sourceLabel=root.querySelector('[data-reference-source]');
const media=document.createElement('audio');media.preload='auto';media.hidden=true;media.setAttribute('data-practice-media','true');root.append(media);
const preservePitchKey=['preservesPitch','webkitPreservesPitch','mozPreservesPitch'].find(key=>key in media);
if(preservePitchKey)media[preservePitchKey]=true;
const send=value=>{if(alive)dioxus.send(value);};
function stop(){playGeneration++;media.pause();try{media.currentTime=0;}catch(_){}}
function destroy(){if(!alive)return;alive=false;finish();cancelPrepare?.();clearTimeout(prepareTimer);stop();media.removeAttribute('src');media.load();media.remove();if(url)URL.revokeObjectURL(url);db?.close();root.removeEventListener('click',click);root.removeEventListener('change',change);window.removeEventListener('rufren-audio-assets-changed',assetsChanged);window.removeEventListener('pagehide',destroy);document.removeEventListener('visibilitychange',visibility);observer.disconnect();window.rufrenAudioPlayers.delete(root.id);}
function visibility(){if(document.hidden)stop();}
function click(event){
 const button=event.target.closest('button');if(!button)return;
 if(button.dataset.practiceStop!==undefined){stop();status.textContent='Playback stopped.';return;}
 if(!button.dataset.practiceRate||!record)return;
 const rate=Number(button.dataset.practiceRate);
 if(rate!==1&&!preservePitchKey){status.textContent='Pitch-preserving slower playback is unavailable here. Listen at natural speed.';return;}
 for(const [id,api]of window.rufrenAudioPlayers)if(id!==root.id)api.stop?.();
 stop();const generation=playGeneration;media.playbackRate=rate;if(preservePitchKey)media[preservePitchKey]=true;
 // The original is verified and prepared before readiness; play stays in the click.
 media.play().then(()=>{
  if(!alive||generation!==playGeneration)return;
  status.textContent=rate===1?'Playing the selected recording.':'Playing slowly, keeping voice pitch. Time stretching can add artifacts.';send({kind:'played'});
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
  }else{
   const records=await new Promise((resolve,reject)=>{const r=db.transaction('assets').objectStore('assets').getAll();r.onsuccess=()=>resolve(r.result);r.onerror=()=>reject(r.error);});
   const machine=r=>['ai_generated_speech','system_generated_speech'].includes(r.device?.provenance?.kind)?1:0;
   const references=records.filter(r=>r.role==='reference'&&r.revision_id===root.dataset.revision&&!r.interrupted).sort((a,b)=>machine(b)-machine(a)||b.created_at_ms-a.created_at_ms);
   selected=references.find(r=>r.asset_id===root.dataset.selectedReference)||references[0];
   if(picker){picker.replaceChildren(...references.map(r=>new Option(`${referenceSource(r)} · ${r.name}`,r.asset_id)));picker.disabled=!references.length;if(selected)picker.value=selected.asset_id;}
   if(!selected)throw Error('Generate reference audio for this phrase. You can also sync an existing reference or add a provided voice in the speech studio.');
  }
  if(!(selected.blob instanceof Blob)||selected.byte_length!==selected.blob.size||!/^[0-9a-f]{64}$/.test(selected.content_hash))throw Error('The stored audio metadata is invalid. Restore the original from an audio backup.');
  if(!crypto.subtle)throw Error('Use HTTPS or localhost to check and play saved audio.');
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
  if(alive&&generation===loadGeneration){record=selected;if(sourceLabel)sourceLabel.textContent=referenceSource(record);buttons.forEach(b=>b.disabled=Number(b.dataset.practiceRate)!==1&&!preservePitchKey);status.textContent=preservePitchKey?'Reference ready. Listen at natural speed, or replay slowly with voice pitch preserved.':'Reference ready at natural speed. Pitch-preserving slow playback is unavailable here.';send({kind:'ready',ready:true});}
 }catch(error){if(alive&&generation===loadGeneration){status.textContent=error.message;send({kind:'ready',ready:false});}}
}
await prepare();
await lifetime;
"#;

#[derive(Deserialize)]
struct PracticeAudioEvent {
    kind: String,
    #[serde(default)]
    ready: bool,
}

/// Audio-only playback for listening practice. No answer text is rendered.
#[component]
pub fn AudioPractice(
    revision_id: String,
    on_played: EventHandler<()>,
    on_ready: EventHandler<bool>,
    #[props(default)] on_generate: Option<EventHandler<()>>,
    #[props(default)] generating: bool,
) -> Element {
    let mut error = use_signal(String::new);
    use_effect(move || {
        spawn(async move {
            let mut bridge = document::eval(PRACTICE_BRIDGE);
            while let Ok(event) = bridge.recv::<PracticeAudioEvent>().await {
                if event.kind == "played" {
                    on_played.call(());
                } else if event.kind == "ready" {
                    on_ready.call(event.ready);
                }
            }
            error.set(
                "The reference player disconnected. Reopen this practice to reconnect.".into(),
            );
        });
    });
    use_drop(|| {
        let _ = document::eval("window.rufrenAudioPractice?.destroy();");
    });
    rsx! {
        style {"{STYLE}"}
        section {id:"rufren-audio-practice",class:"audio-practice","data-revision":revision_id,aria_label:"Listen to the reference voice",
            if let Some(generate)=on_generate {button {class:"secondary",disabled:generating,onclick:move |_|generate.call(()),if generating {"Generating reference…"}else{"Generate reference audio"}}}
            label {r#for:"practice-reference-voice","Reference voice"}
            select {id:"practice-reference-voice",disabled:true,"data-reference-select":"true"}
            p {class:"quiet audio-source","data-reference-source":"true"}
            div {class:"actions",
                button {class:"primary",disabled:true,"data-practice-rate":"1","▶ Listen / replay"}
                button {class:"secondary",disabled:true,"data-practice-rate":"0.7","Replay slowly · keep voice pitch"}
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
fn AudioReviewPlayer(asset_id: String, label: String) -> Element {
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
            while bridge.recv::<PracticeAudioEvent>().await.is_ok() {}
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
        div {id:element_id,class:"audio-practice audio-review-player","data-asset":asset_id,
            h3 {"{label}"}
            div {class:"actions",
                button {class:"primary",disabled:true,"data-practice-rate":"1","▶ Play this recording"}
                button {class:"secondary",disabled:true,"data-practice-rate":"0.7","Replay slowly · keep voice pitch"}
                button {class:"text-button","data-practice-stop":"true","Stop"}
            }
            p {class:"quiet",role:"status","data-practice-status":"true","Opening the exact saved original…"}
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
