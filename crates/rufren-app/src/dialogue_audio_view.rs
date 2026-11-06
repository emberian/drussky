//! Voice is an input method for dialogue, never a substitute practice attempt.
use crate::{
    asset_sync,
    audio_studio::{
        AudioReviewPlayer, AudioSpeaking, DialogueRecorderScope, StudioRecording,
        save_generated_dialogue_audio,
    },
    dialogue_audio::{DialogueAdoptedSpeech, DialogueVoiceDraft},
    exposure_view::PresentedRussian,
    gateway,
    model::AppState,
};
use dioxus::prelude::*;
use rufren_core::*;
use rufren_speech::{
    RECOGNITION_MAX_AUDIO_BYTES, RECOGNITION_MAX_RESPONSE_BYTES, RecognitionResponse,
};
use uuid::Uuid;
const STYLE: &str = include_str!("dialogue_audio_view.css");

#[component]
pub(crate) fn DialogueVoiceInput(
    mut app: Signal<AppState>,
    thread_id: Uuid,
    input_id: Uuid,
    on_adopt: EventHandler<DialogueAdoptedSpeech>,
    #[props(default)] on_clear: EventHandler<()>,
    #[props(default)] on_capture_busy: EventHandler<bool>,
) -> Element {
    let initial = use_hook(|| app.read().dialogue_voice_draft(thread_id, input_id));
    let mut voice = use_signal(|| initial.clone().unwrap_or_default());
    let mut problem = use_signal(|| initial.err().unwrap_or_default());
    let mut recording_busy = use_signal(|| false);
    let mut reported_busy = use_signal(|| false);
    use_effect(move || {
        let selected = voice().recording;
        let pending = recording_busy()
            || app
                .read()
                .recognition_job
                .as_ref()
                .is_some_and(|(id, _)| Some(*id) == selected);
        if reported_busy() != pending {
            reported_busy.set(pending);
            on_capture_busy.call(pending);
        }
    });
    let state = app.read().clone();
    let snapshot = voice();
    let recording = snapshot.recording.and_then(|id| {
        state
            .dialogue_input_recording(id, Some((thread_id, input_id)))
            .ok()
            .map(|record| (id, record.clone()))
    });
    let selected_asset = recording
        .as_ref()
        .map(|(_, record)| record.asset_id.clone());
    let hypotheses = state.library.dialogue_speech_transcriptions(input_id);
    let selected_transcription = snapshot
        .transcription
        .filter(|id| {
            hypotheses
                .iter()
                .any(|(event, t)| event.id == *id && Some(t.recording) == snapshot.recording)
        })
        .or_else(|| {
            hypotheses
                .iter()
                .filter(|(_, t)| Some(t.recording) == snapshot.recording)
                .max_by_key(|(event, _)| (event.recorded_at_ms, event.id))
                .map(|(event, _)| event.id)
        });
    let this_job = state
        .recognition_job
        .as_ref()
        .is_some_and(|(id, _)| Some(*id) == snapshot.recording);
    let committed = state.library.dialogue_user(input_id).is_some();
    rsx! {
        style {"{STYLE}"}
        section {class:"dialogue-voice-input","data-dialogue-input":input_id.to_string(),
            p {class:"quiet","Speak Russian, then check the words before adding them to your message. A recording is kept separately from the words you adopt."}
            if !problem().is_empty(){p {role:"alert","{problem}"}}
            if !committed {
                AudioSpeaking {key:"dialogue-recorder-{input_id}",passage_id:String::new(),revision_id:String::new(),dialogue_scope:Some(DialogueRecorderScope{thread_id,input_id}),
                    selected_asset_id:selected_asset.clone(),workspace_id:if cfg!(target_arch="wasm32"){String::new()}else{state.device.to_string()},
                    on_recording_state:move |busy|recording_busy.set(busy),
                    on_saved:move |record:StudioRecording|{
                        let result=(||{let id=app.write().register_dialogue_recording(thread_id,input_id,&record)?;
                            let next=DialogueVoiceDraft{recording:Some(id),transcription:None,confirmation:None};
                            app.read().save_dialogue_voice_draft(thread_id,input_id,&next)?;Ok::<_,String>(next)
                        })();
                        match result{Ok(next)=>{voice.set(next);problem.set(String::new());on_clear.call(());},Err(error)=>problem.set(error)}
                    },
                    on_selected:move |record:Option<StudioRecording>|{
                        let result=(||{
                            let recording=record.as_ref().map(|record|app.write().register_dialogue_recording(thread_id,input_id,record)).transpose()?;
                            let next=DialogueVoiceDraft{recording,transcription:None,confirmation:None};app.read().save_dialogue_voice_draft(thread_id,input_id,&next)?;Ok::<_,String>(next)
                        })();
                        match result{Ok(next)=>{voice.set(next);problem.set(String::new());on_clear.call(());},Err(error)=>problem.set(error)}
                    }
                }
            } else if let Some(asset)=selected_asset {AudioReviewPlayer {asset_id:asset,label:"Your recorded message".to_string()}}
            if let Some((recording,_))=recording {
                div {class:"actions",
                    button {class:"secondary",disabled:state.recognition_job.is_some()||committed,onclick:move |_|{on_clear.call(());transcribe_dialogue(app,thread_id,input_id,recording);},
                        if this_job {"Listening for the words…"}else if selected_transcription.is_some(){"Transcribe this take again"}else{"Find the words in this take"}
                    }
                    if this_job {button {class:"text-button",disabled:state.recognition_cancel_requested,onclick:move |_|crate::recognition_view::cancel(app),if state.recognition_cancel_requested {"Cancelling…"}else{"Cancel transcription"}}}
                }
            }
            p {class:"quiet","Transcription uses your private speech service. Supported input: PCM WAV, mono or stereo, 8–48 kHz, up to 45 seconds and 9 MB. Your original stays on this device and can be synced in Settings."}
            if let Some(transcription)=selected_transcription {
                DialogueTranscriptEditor {key:"dialogue-transcript-{transcription}",app,thread_id,input_id,transcription,on_adopt,
                    on_confirmed:move |confirmation|{let mut next=voice();next.transcription=Some(transcription);next.confirmation=Some(confirmation);let result=app.read().save_dialogue_voice_draft(thread_id,input_id,&next);match result{Ok(())=>voice.set(next),Err(error)=>problem.set(error)}}
                }
            }
        }
    }
}

#[component]
fn DialogueTranscriptEditor(
    mut app: Signal<AppState>,
    thread_id: Uuid,
    input_id: Uuid,
    transcription: Uuid,
    on_adopt: EventHandler<DialogueAdoptedSpeech>,
    on_confirmed: EventHandler<Uuid>,
) -> Element {
    let initial = use_hook(|| {
        let state = app.read();
        let heads = state
            .library
            .dialogue_transcript_confirmations(transcription);
        let fallback = if heads.len() == 1 {
            heads[0].1.text.clone()
        } else {
            state
                .library
                .dialogue_speech_transcription(transcription)
                .map(|t| t.transcript.clone())
                .unwrap_or_default()
        };
        state
            .repo
            .get::<String>(&format!("dialogue_transcript_draft:{transcription}"))
            .map(|draft| draft.unwrap_or(fallback))
    });
    let mut text = use_signal(|| initial.clone().unwrap_or_default());
    let mut problem = use_signal(|| initial.err().unwrap_or_default());
    let state = app.read().clone();
    let Some(observation) = state.library.dialogue_speech_transcription(transcription) else {
        return rsx! {p {role:"alert","The original dialogue transcription is unavailable. Sync it before continuing."}};
    };
    let heads = state
        .library
        .dialogue_transcript_confirmations(transcription);
    let committed = state.library.dialogue_user(input_id).is_some();
    let family = format!("dialogue-input:{input_id}");
    rsx! {
        section {class:"dialogue-transcript-words","data-dialogue-transcription":transcription.to_string(),
            h3 {"The words it heard"}
            if observation.transcript.is_empty(){p {"No words were recognized. Replay the original or record a shorter message."}}
            else {PresentedRussian {app,text:observation.transcript.clone(),purpose:PresentationPurpose::Answer,utterance_family:Some(family.clone()),blockquote {"{observation.transcript}"}}}
            if heads.len()>1 {p {class:"quiet","This transcript has different saved edits. Choose the intended words below to resolve them explicitly."}}
            for (event,confirmation) in heads {div {key:"adopted-{event.id}",p {class:"quiet","Saved adopted words"}PresentedRussian {app,text:confirmation.text.clone(),purpose:PresentationPurpose::Answer,utterance_family:Some(family.clone()),blockquote {"{confirmation.text}"}}}}
            if !problem().is_empty(){p {role:"alert","{problem}"}}
            if !committed {
                label {r#for:"dialogue-transcript-{transcription}","Words to add to your message · edit anything it misheard"}
                textarea {id:"dialogue-transcript-{transcription}",lang:"ru",value:text(),disabled:state.teacher_busy,
                    oninput:move |event|{let value=event.value();if value.len()>2048{problem.set("Keep one message under 2 KB.".into());return;}text.set(value.clone());let result=app.read().repo.set(&format!("dialogue_transcript_draft:{transcription}"),&value);match result{Ok(())=>problem.set(String::new()),Err(error)=>problem.set(format!("Your edit is still here, but could not be saved: {error}"))}}
                }
                button {class:"primary",disabled:state.teacher_busy||text.read().trim().is_empty(),onclick:move |_|{
                    let result=(||{let confirmation=app.write().confirm_dialogue_transcription(transcription,text())?;let adopted=app.read().dialogue_audio_input_reference(thread_id,input_id,confirmation)?;Ok::<_,String>((confirmation,adopted))})();
                    match result {Ok((confirmation,adopted))=>{on_confirmed.call(confirmation);on_adopt.call(adopted);},Err(error)=>problem.set(error)}
                },"Use these words in my message"}
            }
            details {summary {"About this transcription"}p {"{observation.provenance.model}"}p {class:"quiet","An independent Russian transcript is a hypothesis about words. It does not assess pronunciation or rhythm."}}
        }
    }
}

fn transcribe_dialogue(mut app: Signal<AppState>, thread: Uuid, input: Uuid, recording: Uuid) {
    if app.read().recognition_job.is_some() {
        return;
    }
    let snapshot = app.read().clone();
    let attachment = match snapshot.dialogue_input_recording(recording, Some((thread, input))) {
        Ok(record) => record.clone(),
        Err(error) => {
            app.write().notice = error;
            return;
        }
    };
    let request_id = Uuid::new_v4().to_string();
    app.write().recognition_job = Some((recording, request_id.clone()));
    app.write().recognition_cancel_requested = false;
    dioxus_core::spawn_forever(async move {
        let result = async {
            let (metadata, bytes) = asset_sync::read_original(
                &attachment.asset_id,
                &attachment.asset.sha256,
                RECOGNITION_MAX_AUDIO_BYTES,
            )
            .await?;
            let scope = &metadata.device["dialogue_scope"];
            if metadata.role != "attempt"
                || !metadata.passage_id.is_empty()
                || !metadata.revision_id.is_empty()
                || scope["thread_id"].as_str() != Some(thread.to_string().as_str())
                || scope["input_id"].as_str() != Some(input.to_string().as_str())
            {
                return Err("The local original belongs to a different dialogue input.".into());
            }
            let request =
                snapshot.dialogue_recognition_request(recording, &bytes, request_id.clone())?;
            if app.read().recognition_cancel_requested {
                return Err("Transcription cancelled.".into());
            }
            let response = gateway::request::<RecognitionResponse>(
                &snapshot.endpoint,
                &snapshot.token,
                "/api/speech/transcribe",
                Some(&serde_json::to_value(&request).map_err(|e| e.to_string())?),
                RECOGNITION_MAX_RESPONSE_BYTES,
                140_000,
            )
            .await?;
            if app.read().recognition_cancel_requested {
                return Err("Transcription cancelled.".into());
            }
            let transcription = app
                .write()
                .keep_dialogue_transcription(recording, &request, &response)?;
            let mut selected = app.read().dialogue_voice_draft(thread, input)?;
            if selected.recording == Some(recording) {
                selected.transcription = Some(transcription);
                selected.confirmation = None;
                app.read()
                    .save_dialogue_voice_draft(thread, input, &selected)?;
            }
            Ok::<_, String>(())
        }
        .await;
        let cancelled = app.read().recognition_cancel_requested;
        app.write().recognition_job = None;
        app.write().recognition_cancel_requested = false;
        app.write().notice = if cancelled {
            "Transcription cancelled. Your original is kept.".into()
        } else {
            match result {
                Ok(()) => "The words are ready to check.".into(),
                Err(error) => error,
            }
        };
    });
}

#[component]
pub(crate) fn DialogueChunkAudio(
    mut app: Signal<AppState>,
    reply_id: Uuid,
    chunk_index: usize,
) -> Element {
    let state = app.read().clone();
    let Some(reply) = state.library.dialogue_reply(reply_id) else {
        return rsx! {};
    };
    let chunks = teacher_dialogue_reply_chunks(reply);
    let Some(chunk) = chunks.get(chunk_index) else {
        return rsx! {};
    };
    let owner = DialogueAudioOwner::AssistantChunk {
        reply_id,
        chunk_index,
        source_text_sha256: dialogue_text_sha256(&chunk.russian),
    };
    let mut voices = state.library.dialogue_audio_attachments(&owner);
    voices.sort_by_key(|(event, _)| (std::cmp::Reverse(event.recorded_at_ms), event.id));
    let mut selected = use_signal(|| None::<String>);
    let asset = selected()
        .filter(|id| voices.iter().any(|(_, audio)| &audio.asset_id == id))
        .or_else(|| voices.first().map(|(_, audio)| audio.asset_id.clone()));
    let select_id = use_hook(|| format!("dialogue-voice-{}", Uuid::new_v4()));
    let voice_source = asset
        .as_ref()
        .and_then(|id| voices.iter().find(|(_, voice)| &voice.asset_id == id))
        .and_then(|(_, voice)| voice.provenance_json.as_deref())
        .and_then(|json| serde_json::from_str::<serde_json::Value>(json).ok())
        .map(|p| {
            format!(
                "{} · {} · {}",
                if p["kind"].as_str() == Some("system_generated_speech") {
                    "System speech"
                } else {
                    "AI speech"
                },
                p["provider"].as_str().unwrap_or("Saved voice"),
                p["voice"].as_str().unwrap_or("Saved voice")
            )
        });
    rsx! {
        style {"{STYLE}"}
        div {class:"dialogue-chunk-audio","data-dialogue-reply":reply_id.to_string(),"data-dialogue-chunk":chunk_index.to_string(),
            if !voices.is_empty(){
                if voices.len()>1 {label {r#for:select_id.clone(),"Spoken voice"}select {id:select_id,value:asset.clone().unwrap_or_default(),onchange:move |event|selected.set(Some(event.value())),for(index,(_,audio))in voices.iter().enumerate(){option{value:audio.asset_id.clone(),{format!("Voice {}",index+1)}}}}}
                if let Some(asset_id)=asset {AudioReviewPlayer {key:"dialogue-voice-{asset_id}",asset_id,label:"Spoken teacher message".to_string(),on_played:move |record|{let result=app.write().heard_dialogue_chunk(reply_id,chunk_index,&record);if let Err(error)=result{app.write().notice=error;}}}}
            }
            if let Some(source)=voice_source {p {class:"quiet","{source}"}}
            button {class:"secondary",disabled:state.speech_busy,onclick:move |_|generate_chunk(app,reply_id,chunk_index),if state.speech_busy {"Creating spoken audio…"}else if voices.is_empty(){"Create spoken audio"}else{"Create another voice"}}
        }
    }
}
fn generate_chunk(mut app: Signal<AppState>, reply: Uuid, index: usize) {
    if app.read().speech_busy {
        return;
    }
    let snapshot = app.read().clone();
    let source = (|| {
        let response = snapshot
            .library
            .dialogue_reply(reply)
            .ok_or("The exact dialogue reply is unavailable.")?;
        let chunks = teacher_dialogue_reply_chunks(response);
        let chunk = chunks
            .get(index)
            .ok_or("The Russian utterance is unavailable.")?;
        Ok::<_, String>((response.request.thread_id, chunk.russian.clone()))
    })();
    let (thread, text) = match source {
        Ok(source) => source,
        Err(error) => {
            app.write().notice = error;
            return;
        }
    };
    let request_id = Uuid::new_v4().to_string();
    app.write().speech_busy = true;
    dioxus_core::spawn_forever(async move {
        let result = async {
            let body = serde_json::json!({"text":text,"request_id":request_id,"revision_id":null});
            let response = gateway::request::<serde_json::Value>(
                &snapshot.endpoint,
                &snapshot.token,
                "/api/speech/generate",
                Some(&body),
                4 * 1024 * 1024,
                90_000,
            )
            .await?;
            let record =
                save_generated_dialogue_audio(thread, reply, index, &text, &request_id, response)
                    .await?;
            app.write()
                .register_dialogue_generated_audio(reply, index, &record)?;
            Ok::<_, String>(())
        }
        .await;
        app.write().speech_busy = false;
        app.write().notice = match result {
            Ok(()) => "The spoken teacher message is ready.".into(),
            Err(error) => error,
        };
    });
}

/// Replay the exact immutable original and confirmation used by a sent message.
#[component]
pub(crate) fn DialogueUserAudio(app: Signal<AppState>, input_id: Uuid) -> Element {
    let state = app.read();
    let Some(user) = state.library.dialogue_user(input_id) else {
        return rsx! {};
    };
    let Some(source) = &user.confirmed_audio else {
        return rsx! {};
    };
    let Some(Fact::DialogueAudioAttached(recording)) = state
        .library
        .events
        .get(&source.recording)
        .map(|event| &event.fact)
    else {
        return rsx! {p {class:"quiet",role:"status","Sync this voice message’s original recording link to replay it."}};
    };
    let Some(observation) = state
        .library
        .dialogue_speech_transcription(source.transcription)
    else {
        return rsx! {p {class:"quiet",role:"status","Sync this message’s independent transcription to inspect its source."}};
    };
    let Some(confirmation) = state
        .library
        .dialogue_transcript_confirmation(source.confirmation)
    else {
        return rsx! {p {class:"quiet",role:"status","Sync this message’s exact adopted words to inspect its source."}};
    };
    if let Err(error) = validate_dialogue_input_source(user, &state.library.events) {
        return rsx! {p {role:"alert","{error}"}};
    }
    let family = format!("dialogue-input:{input_id}");
    rsx! {
        style {"{STYLE}"}
        details {class:"dialogue-user-audio","data-dialogue-original":source.recording.to_string(),
            summary {"Recording & adopted words"}
            AudioReviewPlayer {key:"sent-dialogue-{source.recording}",asset_id:recording.asset_id.clone(),label:"Your original voice message".to_string()}
            p {class:"quiet","Independent transcription"}
            if observation.transcript.is_empty(){p {"No words were recognized."}}else{PresentedRussian {app,text:observation.transcript.clone(),purpose:PresentationPurpose::Answer,utterance_family:Some(family.clone()),blockquote {"{observation.transcript}"}}}
            p {class:"quiet",match confirmation.operation {TranscriptOperation::Confirm=>"Words adopted unchanged for this message",TranscriptOperation::Edit=>"Words edited and adopted for this message"}}
            PresentedRussian {app,text:confirmation.text.clone(),purpose:PresentationPurpose::Answer,utterance_family:Some(family),blockquote {"{confirmation.text}"}}
            details {summary {"Transcription source & audit"}
                p {class:"quiet","{observation.provenance.model}"}
                p {class:"quiet","Original SHA-256: {observation.audio_sha256}"}
                pre {class:"dialogue-audio-audit","{observation.audit_json}"}
            }
        }
    }
}
