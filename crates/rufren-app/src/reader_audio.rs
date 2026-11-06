//! Inline audio belongs to one exact Reader chunk and text layer. Preparing or
//! storing a reference is not an opportunity to hear it; only successful play is.
use crate::{
    audio_studio::{PRACTICE_BRIDGE, PracticeAudioEvent, StudioRecording},
    generate_reference,
    model::AppState,
    repository::Result,
};
use dioxus::prelude::*;
use rufren_core::*;
use std::{cell::Cell, rc::Rc};
use uuid::Uuid;

fn layer_name(layer: ReaderTextLayer) -> &'static str {
    match layer {
        ReaderTextLayer::Original => "original",
        ReaderTextLayer::StressScaffold => "stress_scaffold",
    }
}

impl AppState {
    /// Verify the actual original and canonical derivative chain before writing
    /// an audio presentation. No source reference is silently repaired here.
    fn heard_reader_audio(
        &mut self,
        document_id: Uuid,
        chunk_id: &str,
        layer: ReaderTextLayer,
        record: &StudioRecording,
    ) -> Result<()> {
        self.refresh()?;
        let document = self
            .library
            .reader_document(document_id)
            .ok_or("The exact reading is missing. Sync its original before playing this voice.")?;
        let (reference, link) =
            reader_reference_events(document, chunk_id, layer).map_err(|e| e.to_string())?;
        if self.library.events.get(&reference.id) != Some(&reference)
            || self.library.events.get(&link.id) != Some(&link)
        {
            return Err(
                "This voice’s original reading link is missing. Sync it before continuing.".into(),
            );
        }
        let Fact::PassageRevised(revision) = &reference.fact else {
            return Err("The selected voice has no exact text reference.".into());
        };
        let metadata: AudioRecordMetadata =
            serde_json::from_value(serde_json::to_value(record).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        metadata.validate().map_err(|e| e.to_string())?;
        let source_hash = reader_text_sha256(&revision.passage.russian);
        let provenance = &record.device["provenance"];
        if record.role != "reference"
            || record.interrupted
            || record.revision_id != reference.id.to_string()
            || record.passage_id != revision.passage.id.to_string()
            || provenance["source_text"].as_str() != Some(revision.passage.russian.as_str())
            || provenance["source_text_sha256"].as_str() != Some(source_hash.as_str())
            || provenance["revision_id"].as_str() != Some(record.revision_id.as_str())
            || provenance["request_id"]
                .as_str()
                .is_none_or(|id| id.is_empty() || id.len() > 256)
            || !matches!(
                provenance["kind"].as_str(),
                Some("ai_generated_speech" | "system_generated_speech")
            )
        {
            return Err(
                "The played original does not match this exact Reader text. Its audio was kept."
                    .into(),
            );
        }
        let asset = AudioAsset {
            sha256: record.content_hash.clone(),
            media_type: record.mime_type.clone(),
            sample_rate_hz: record.sample_rate,
            channels: record.channels,
            frames: record.frames,
        };
        if !self
            .library
            .recordings(AnnotationTarget::Revision(reference.id))
            .iter()
            .any(|(_, a)| {
                a.role == AudioRole::Reference && a.asset_id == record.asset_id && a.asset == asset
            })
        {
            return Err(
                "The played original’s exact attachment is missing. Sync its recording link first."
                    .into(),
            );
        }
        self.note_presentation(
            Uuid::new_v4(),
            StimulusPresentation {
                session: None,
                revision: Some(reference.id),
                russian: revision.passage.russian.clone(),
                purpose: PresentationPurpose::Audio,
                decision: None,
                utterance_family: Some(format!(
                    "reader:{}",
                    Uuid::new_v5(&document_id, chunk_id.as_bytes())
                )),
            },
        )
    }
}

/// The parent chooses Original or StressScaffold. A layer/source change remounts
/// the inner player, stopping old sound and invalidating its pending callbacks.
#[component]
pub(crate) fn ReaderAudio(
    app: Signal<AppState>,
    document_id: Uuid,
    chunk_id: String,
    #[props(default = ReaderTextLayer::Original)] layer: ReaderTextLayer,
) -> Element {
    let source = (|| {
        let definition = app.read().reader_definition(document_id)?;
        let document = ReaderDocument {
            document_id,
            definition,
        };
        let (event, _) =
            reader_reference_events(&document, &chunk_id, layer).map_err(|e| e.to_string())?;
        let Fact::PassageRevised(revision) = event.fact else {
            return Err("The selected reading has no audio text.".into());
        };
        if revision.passage.russian.len() > 1024 || revision.passage.russian.chars().count() > 256 {
            return Err("This chunk is too long for one spoken reference. Choose a shorter chunk; the original is unchanged.".into());
        }
        Ok::<_, String>((event.id, revision.passage.russian))
    })();
    let (revision, text) = match source {
        Ok(source) => source,
        Err(error) => return rsx! {p {class:"quiet",role:"alert","{error}"}},
    };
    let identity = format!("{document_id}-{chunk_id}-{}-{revision}", layer_name(layer));
    rsx! {for identity in [identity] {
        ReaderAudioPlayer {key:"{identity}",app,document_id,chunk_id:chunk_id.clone(),layer,revision,source_hash:reader_text_sha256(&text)}
    }}
}

#[component]
fn ReaderAudioPlayer(
    mut app: Signal<AppState>,
    document_id: Uuid,
    chunk_id: String,
    layer: ReaderTextLayer,
    revision: Uuid,
    source_hash: String,
) -> Element {
    let element_id = use_hook(|| format!("reader-audio-{}", Uuid::new_v4()));
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    let mut ready = use_signal(|| false);
    let mut error = use_signal(String::new);
    let effect_id = element_id.clone();
    let effect_alive = alive.clone();
    let playback_chunk = chunk_id.clone();
    use_effect(move || {
        let script = format!(
            "const rufrenPlayerRoot=document.getElementById({});\n{}",
            serde_json::to_string(&effect_id).unwrap_or_default(),
            PRACTICE_BRIDGE
        );
        let alive = effect_alive.clone();
        let chunk = playback_chunk.clone();
        spawn(async move {
            let mut bridge = document::eval(&script);
            while let Ok(event) = bridge.recv::<PracticeAudioEvent>().await {
                if !alive.get() {
                    break;
                }
                match event.kind.as_str() {
                    "ready" => ready.set(event.ready),
                    "played" => {
                        if let Some(recording) = event.recording {
                            match app.write().heard_reader_audio(
                                document_id,
                                &chunk,
                                layer,
                                &recording,
                            ) {
                                Ok(()) => error.set(String::new()),
                                Err(problem) => error.set(problem),
                            }
                        }
                    }
                    _ => (),
                }
            }
            if alive.get() {
                error.set("The audio player disconnected. Close and reopen this chunk’s audio to reconnect.".into());
            }
        });
    });
    let drop_id = element_id.clone();
    use_drop(move || {
        alive.set(false);
        let id = serde_json::to_string(&drop_id).unwrap_or_default();
        let _ = document::eval(&format!("window.rufrenAudioPlayers?.get({id})?.destroy();"));
    });
    let busy = app.read().speech_busy;
    let generate_chunk = chunk_id.clone();
    rsx! {section {class:"reader-audio teacher-example-voice","data-reader-document":document_id.to_string(),"data-reader-chunk":chunk_id,"data-reader-layer":layer_name(layer),
        h3 {match layer {ReaderTextLayer::Original=>"Listen to the original text",ReaderTextLayer::StressScaffold=>"Listen with stress support"}}
        if layer==ReaderTextLayer::StressScaffold {p {class:"quiet","This voice uses the stress-marked reading scaffold."}}
        if !ready(){button {class:"secondary",disabled:busy,"data-reader-create-audio":"true",onclick:move |_|{
            let result=app.write().prepare_reader_audio(document_id,&generate_chunk,layer);
            match result{Ok(id)if id==revision=>{error.set(String::new());generate_reference(app,id);},Ok(_)=>error.set("This reading changed its exact audio source. Reopen the chunk.".into()),Err(problem)=>error.set(problem)}
        },if busy{"Creating spoken audio…"}else if layer==ReaderTextLayer::Original{"Create spoken original"}else{"Create spoken stress support"}}}
        div {id:element_id,class:"reader-audio-player teacher-example-player","data-revision":revision.to_string(),"data-source-text-sha256":source_hash,"data-playback-context":"example",aria_label:"Selected reading chunk audio",
            div {class:"actions",
                button {class:"secondary",disabled:true,"data-practice-rate":"1","▶ Listen"}
                button {class:"text-button",disabled:true,"data-practice-rate":"0.7","Slowly"}
                button {class:"text-button","data-practice-stop":"true","Stop"}
            }
            p {class:"quiet",role:"status","data-practice-status":"true","Checking for this text’s saved voice…"}
            details {summary {"Voice source"}p {class:"quiet","data-reference-source":"true"}}
        }
        if !error().is_empty(){p {class:"quiet",role:"alert","{error}"}}
    }}
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    fn record_for(state: &AppState, revision: Uuid, asset_id: &str) -> StudioRecording {
        let source = &state.library.revision(revision).unwrap().passage;
        let bytes = include_bytes!("../../../research/artifacts/reference-quality-slot1/RQ02.wav");
        StudioRecording {
            asset_id: asset_id.into(),
            content_hash: format!("{:x}", Sha256::digest(bytes)),
            role: "reference".into(),
            name: "Synthetic callback fixture".into(),
            passage_id: source.id.to_string(),
            revision_id: revision.to_string(),
            duration_ms: 625,
            sample_rate: 24000,
            channels: 1,
            frames: 15000,
            byte_length: bytes.len() as u64,
            created_at_ms: 1,
            mime_type: "audio/wav".into(),
            interrupted: false,
            annotations: Vec::new(),
            device: serde_json::json!({"provenance":{"kind":"ai_generated_speech","provider":"fixture","model":"fixture","voice":"fixture","request_id":asset_id,"revision_id":revision,"source_text":source.russian,"source_text_sha256":reader_text_sha256(&source.russian)}}),
        }
    }
    fn attach(state: &mut AppState, record: &StudioRecording) {
        let revision = Uuid::parse_str(&record.revision_id).unwrap();
        let event = rufren_core::Event::new(
            state.device,
            1,
            Fact::RecordingAttached(AudioAttachment {
                asset_id: record.asset_id.clone(),
                asset: AudioAsset {
                    sha256: record.content_hash.clone(),
                    media_type: record.mime_type.clone(),
                    sample_rate_hz: record.sample_rate,
                    channels: record.channels,
                    frames: record.frames,
                },
                target: AnnotationTarget::Revision(revision),
                role: AudioRole::Reference,
                note: String::new(),
            }),
        )
        .unwrap();
        state.repo.append(&[event]).unwrap();
        state.refresh().unwrap();
    }

    #[test]
    fn play_ack_requires_attached_original_and_keeps_stress_scaffold_distinct() {
        let mut state = AppState::from_repository(Rc::new(
            crate::repository::Repository::open_at(":memory:").unwrap(),
        ))
        .unwrap();
        let definition = rufren_content::load_reader_anthology()
            .unwrap()
            .documents
            .remove(0);
        let document = reader_document(&definition).unwrap();
        let chunk = &definition.chunks[0];
        state
            .open_reader_document(document.document_id, Some(&chunk.id))
            .unwrap();
        let original = state
            .prepare_reader_audio(document.document_id, &chunk.id, ReaderTextLayer::Original)
            .unwrap();
        let scaffold = state
            .prepare_reader_audio(
                document.document_id,
                &chunk.id,
                ReaderTextLayer::StressScaffold,
            )
            .unwrap();
        assert_ne!(original, scaffold);
        assert_eq!(
            state.library.presentations().count(),
            0,
            "preparation does not create audio exposure"
        );
        let original_record = record_for(&state, original, "original-reader-take");
        assert!(
            state
                .heard_reader_audio(
                    document.document_id,
                    &chunk.id,
                    ReaderTextLayer::Original,
                    &original_record
                )
                .is_err()
        );
        assert_eq!(
            state.library.presentations().count(),
            0,
            "an unattached callback cannot fabricate exposure"
        );
        attach(&mut state, &original_record);
        state
            .heard_reader_audio(
                document.document_id,
                &chunk.id,
                ReaderTextLayer::Original,
                &original_record,
            )
            .unwrap();
        assert!(
            state
                .heard_reader_audio(
                    document.document_id,
                    &chunk.id,
                    ReaderTextLayer::StressScaffold,
                    &original_record
                )
                .is_err()
        );
        let mut wrong = original_record.clone();
        wrong.content_hash = "b".repeat(64);
        assert!(
            state
                .heard_reader_audio(
                    document.document_id,
                    &chunk.id,
                    ReaderTextLayer::Original,
                    &wrong
                )
                .is_err()
        );
        let scaffold_record = record_for(&state, scaffold, "scaffold-reader-take");
        attach(&mut state, &scaffold_record);
        state
            .heard_reader_audio(
                document.document_id,
                &chunk.id,
                ReaderTextLayer::StressScaffold,
                &scaffold_record,
            )
            .unwrap();
        let exposures: Vec<_> = state.library.presentations().map(|(_, p)| p).collect();
        assert_eq!(exposures.len(), 2);
        assert!(
            exposures
                .iter()
                .all(|p| p.purpose == PresentationPurpose::Audio)
        );
        assert!(
            exposures
                .iter()
                .any(|p| p.revision == Some(original) && p.russian == chunk.text)
        );
        assert!(
            exposures.iter().any(|p| p.revision == Some(scaffold)
                && Some(p.russian.as_str()) == chunk.stress.as_deref())
        );
        assert!(
            !state
                .library
                .events
                .values()
                .any(|e| matches!(e.fact, Fact::AttemptRecorded(_)))
        );
    }
}
