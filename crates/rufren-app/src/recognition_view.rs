use crate::{asset_sync, gateway, model::AppState, teacher_view::send_teacher};
use dioxus::prelude::*;
use rufren_core::*;
use rufren_speech::{
    RECOGNITION_MAX_AUDIO_BYTES, RECOGNITION_MAX_RESPONSE_BYTES, RecognitionResponse,
};
use uuid::Uuid;

#[component]
pub(crate) fn SpokenWords(mut app: Signal<AppState>, attempt: Uuid, recording: Uuid) -> Element {
    let state = app.read().clone();
    let latest = state
        .library
        .speech_transcriptions(attempt)
        .into_iter()
        .filter(|(_, t)| t.recording == recording)
        .max_by_key(|(e, _)| (e.recorded_at_ms, e.id));
    let this_job = state
        .recognition_job
        .as_ref()
        .is_some_and(|(id, _)| *id == recording);
    rsx! {
        div {class:"spoken-words",
            button {class:"secondary", disabled:state.recognition_job.is_some(), onclick:move |_|transcribe(app,recording),
                if this_job {"Listening for the words…"}else if latest.is_some(){"Transcribe this take again"}else{"Find the words in this take"}}
            if this_job {button {class:"text-button",disabled:state.recognition_cancel_requested,onclick:move |_|cancel(app),
                if state.recognition_cancel_requested {"Cancelling…"}else{"Cancel transcription"}}}
            p {class:"quiet","This sends the saved take to your private speech service. Check the words it heard, then ask your teacher about their meaning and grammar."}
            if let Some((event,_))=latest {
                TranscriptWords {key:"{event.id}",app,transcription:event.id}
            }
        }
    }
}

#[component]
fn TranscriptWords(mut app: Signal<AppState>, transcription: Uuid) -> Element {
    let initial = use_hook(|| {
        let state = app.read();
        let confirmations = state.library.transcript_confirmations(transcription);
        let fallback = if confirmations.len() == 1 {
            confirmations[0].1.text.clone()
        } else {
            state
                .library
                .speech_transcription(transcription)
                .map(|t| t.transcript.clone())
                .unwrap_or_default()
        };
        state
            .repo
            .get::<String>(&format!("transcript_draft:{transcription}"))
            .map(|s| s.unwrap_or(fallback))
    });
    let mut text = use_signal(|| initial.clone().unwrap_or_default());
    let state = app.read().clone();
    let Some(observation) = state.library.speech_transcription(transcription) else {
        return rsx! {};
    };
    let confirmations = state.library.transcript_confirmations(transcription);
    let field = format!("transcript-words-{transcription}");
    rsx! {
        section {class:"transcript-words",
            h3 {"The words it heard"}
            if observation.transcript.is_empty() {p {"No words were recognized. Replay the take, or try a shorter phrase."}}
            else {blockquote {lang:"ru","{observation.transcript}"}}
            if confirmations.len()>1 {p {"This transcript has different saved edits. Choose the words you want to discuss below."}}
            if let Err(error)=&initial {p {role:"alert","{error}"}}
            label {r#for:field.clone(),"Words to discuss · edit anything it misheard"}
            textarea {id:field,lang:"ru",value:text(),disabled:state.teacher_busy,oninput:move |event| {
                let value=event.value();
                if value.len()>8192 {app.write().notice="Choose a shorter passage for one discussion.".into();return}
                let result=app.read().repo.set(&format!("transcript_draft:{transcription}"),&value);
                match result {Ok(())=>text.set(value),Err(error)=>app.write().notice=error}
            }}
            button {class:"primary",disabled:state.teacher_busy || text.read().trim().is_empty(),onclick:move |_| {
                let result=app.write().confirm_transcription(transcription,text());
                match result {Ok(confirmation)=>{
                    let request=app.read().confirmed_transcript_feedback_request(confirmation);
                    send_teacher(app,request);
                },Err(error)=>app.write().notice=error}
            }, "Use these words & discuss them →"}
            p {class:"quiet","This discussion concerns the words you choose here. Pronunciation and rhythm need the recording itself."}
            details {summary {"About this transcription"}p {"{observation.provenance.model}"}p {"Independent Russian transcription; no pronunciation score or word alignment."}}
        }
    }
}

fn transcribe(mut app: Signal<AppState>, recording: Uuid) {
    if app.read().recognition_job.is_some() {
        return;
    }
    let snapshot = app.read().clone();
    let Some(Fact::RecordingAttached(attachment)) = snapshot
        .library
        .events
        .get(&recording)
        .map(|e| e.fact.clone())
    else {
        app.write().notice = "That saved recording is unavailable.".into();
        return;
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
            let request = snapshot.recognition_request(recording, &bytes, request_id.clone())?;
            if metadata.role != "attempt"
                || Some(metadata.revision_id.as_str()) != request.revision_id.as_deref()
            {
                return Err(
                    "The local recording belongs to a different speaking response.".to_string(),
                );
            }
            if app.read().recognition_cancel_requested {
                return Err("Transcription cancelled.".into());
            }
            let body = serde_json::to_value(&request).map_err(|e| e.to_string())?;
            let response = gateway::request::<RecognitionResponse>(
                &snapshot.endpoint,
                &snapshot.token,
                "/api/speech/transcribe",
                Some(&body),
                RECOGNITION_MAX_RESPONSE_BYTES,
                140_000,
            )
            .await?;
            if app.read().recognition_cancel_requested {
                return Err("Transcription cancelled.".into());
            }
            app.write()
                .keep_transcription(recording, &request, &response)?;
            Ok::<(), String>(())
        }
        .await;
        let cancelled = app.read().recognition_cancel_requested;
        app.write().recognition_job = None;
        app.write().recognition_cancel_requested = false;
        app.write().notice = if cancelled {
            "Transcription cancelled. Your recording is kept.".into()
        } else {
            match result {
                Ok(()) => "The words are ready to check.".into(),
                Err(error) => error,
            }
        };
    });
}

fn cancel(mut app: Signal<AppState>) {
    let state = app.read().clone();
    let Some((_, request_id)) = state.recognition_job else {
        return;
    };
    app.write().recognition_cancel_requested = true;
    dioxus_core::spawn_forever(async move {
        let body = serde_json::json!({"request_id":request_id});
        if let Err(error) = gateway::request::<serde_json::Value>(
            &state.endpoint,
            &state.token,
            "/api/speech/transcribe/cancel",
            Some(&body),
            4096,
            15_000,
        )
        .await
        {
            if app.read().recognition_job.is_some() {
                app.write().notice = format!("Could not confirm cancellation: {error}");
            }
        }
    });
}
