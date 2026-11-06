//! Sound Focus's consumer uses exact original assets, never a latest-voice lookup.
use crate::{
    act,
    audio_studio::{
        AudioSpeaking, PRACTICE_BRIDGE, PracticeAudioEvent, SoundFocusRecorderScope,
        StudioRecording,
    },
    exposure_view::PresentedRussian,
    model::{AppState, Page},
    repository::Result,
    sound_focus::{SoundFocusDraft, SoundFocusPhase},
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use dioxus::prelude::*;
use rufren_content::SoundContrastDefinition;
use rufren_core::{
    AnnotationTarget, AudioRecordMetadata, Cue, Fact, PresentationPurpose, SoundReferenceBinding,
    StimulusPresentation,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::{cell::Cell, rc::Rc};
use uuid::Uuid;

const INSTALLER: &str = include_str!("sound_focus.js");
const MANIFEST: &str = include_str!("../../../research/artifacts/sound-focus-v1/manifest.json");
const STYLE: &str = include_str!("sound_focus.css");

/// Entry from an authored lesson; no generation or new navigation destination.
pub(crate) async fn open_sound_focus(mut app: Signal<AppState>, contrast_id: String) -> Result<()> {
    let definition = rufren_content::sound_contrasts()
        .into_iter()
        .find(|d| d.id == contrast_id)
        .ok_or("This sound contrast is unavailable.")?;
    let identities = definition.members.each_ref().map(|member| {
        rufren_core::sound_reference_identity(&member.utterance.id, &member.utterance.version)
    });
    let models = install_sound_focus_models(&definition, identities).await?;
    let mut references = Vec::new();
    for (member, record) in definition.members.iter().zip(models) {
        references.push(app.write().register_sound_focus_reference(
            &definition,
            &member.id,
            &record_metadata(&record)?,
        )?);
    }
    let references: [SoundReferenceBinding; 2] = references
        .try_into()
        .map_err(|_| "Both exact sound models are required.")?;
    app.write().start_sound_focus(&definition, references)?;
    Ok(())
}

fn record_metadata(record: &StudioRecording) -> Result<AudioRecordMetadata> {
    let metadata: AudioRecordMetadata =
        serde_json::from_value(serde_json::to_value(record).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    metadata.validate().map_err(|e| e.to_string())?;
    Ok(metadata)
}
fn current(app: Signal<AppState>, session: Uuid) -> Result<SoundFocusDraft> {
    app.read()
        .sound_focus_draft()?
        .filter(|draft| draft.session_id == session)
        .ok_or_else(|| "Reopen the exact saved Sound Focus session to continue.".into())
}
fn focus_step() {
    spawn(async move {
        let _=document::eval("await new Promise(requestAnimationFrame);const heading=document.getElementById('sound-focus-step-heading');heading?.focus({preventScroll:true});heading?.scrollIntoView({block:'start',behavior:'instant'});").await;
    });
}

fn note_replay(
    mut app: Signal<AppState>,
    reference: &SoundReferenceBinding,
    family: &str,
    played: &StudioRecording,
) -> Result<()> {
    if played.asset_id != reference.asset_id || played.content_hash != reference.asset.sha256 {
        return Err("The replay did not match this saved model.".into());
    }
    app.write().note_presentation(
        Uuid::new_v4(),
        StimulusPresentation {
            session: None,
            revision: Some(reference.revision),
            russian: reference.source_text.clone(),
            purpose: PresentationPurpose::Audio,
            decision: None,
            utterance_family: Some(family.into()),
        },
    )
}

#[component]
pub(crate) fn SoundFocusView(mut app: Signal<AppState>) -> Element {
    let capture_busy = use_signal(|| false);
    let loaded = app.read().sound_focus_draft();
    let draft = match loaded {
        Ok(Some(draft)) => draft,
        Ok(None) => {
            return rsx! {section {class:"panel",h1 {"Choose a sound to explore"}p {"Open Sound Focus from your naming lesson."}button {class:"secondary",onclick:move |_|app.write().page=Page::Lessons,"Open lessons"}}};
        }
        Err(error) => {
            return rsx! {section {class:"panel",h1 {"Your sound practice needs attention"}p {role:"alert","{error}"}p {"Your saved sources and responses are kept."}button {class:"secondary",onclick:move |_|app.write().page=Page::Settings,"Open sync & backups"}}};
        }
    };
    let session = draft.session_id;
    rsx! {style {"{STYLE}"}
        section {class:"sound-focus","data-sound-focus-session":session.to_string(),"data-sound-focus-phase":format!("{:?}",draft.phase).to_lowercase(),
            div {class:"sound-focus-heading",div {p {class:"eyebrow","SOUND FOCUS"}h1 {"{draft.contrast.title}"}}button {class:"text-button",disabled:capture_busy(),onclick:move |_|act(app,|state|state.pause_sound_focus()),"Pause & return to Today"}}
            if draft.historical {SoundFocusHistory {app,session}}
            else {match draft.phase {
                SoundFocusPhase::Examples=>rsx!{for id in [session]{SoundFocusExamples {key:"examples-{id}",app,session:id}}},
                SoundFocusPhase::Listening=>rsx!{if let Some(trial)=draft.trials.get(draft.current_trial){for token in [trial.token]{SoundFocusTrial {key:"trial-{token}",app,session,token}}}else{p {role:"alert","The saved question is unavailable. Your work is kept."}}},
                SoundFocusPhase::Speaking=>rsx!{
                    section {class:"panel",h2 {id:"sound-focus-step-heading",tabindex:"-1","Choose what you mean, then say it"}p {"{draft.contrast.produce_task.instruction}"}
                        if draft.speaking.as_ref().is_none_or(|intent|intent.saved_attempt.is_none()){fieldset {class:"choice-list",legend {"My intended meaning"},for member in &draft.contrast.members {{let id=member.id.clone();let selected=draft.speaking.as_ref().is_some_and(|intent|intent.member_id==id);rsx!{button {class:if selected{"choice-option selected"}else{"choice-option"},aria_pressed:selected,disabled:capture_busy(),"data-sound-intent":id.clone(),onclick:move |_|act(app,|state|state.sound_focus_intend(session,&id).map(|_|())),"{member.option_label}"}}}}}}
                        if let Some(intent)=&draft.speaking {for identity in [format!("{}-{}",intent.intent_id,intent.saved_attempt.is_some())]{SoundFocusSpeaking {key:"speak-{identity}",app,session,intent_id:intent.intent_id,capture_busy}}}
                    }
                },
                SoundFocusPhase::Finished=>rsx!{section {class:"panel",h2 {id:"sound-focus-step-heading",tabindex:"-1","Kept for another day"}p {"Your choices and spoken take are saved. Try a new voice or context another time."}div {class:"actions",button {class:"primary",onclick:move |_|act(app,|state|state.pause_sound_focus()),"Return to Today"}button {class:"secondary",onclick:move |_|app.write().page=Page::Lessons,"Return to lessons"}}}SoundFocusHistory {app,session}},
            }}
        }
    }
}

fn support_label(cue: &Cue) -> &'static str {
    match cue {
        Cue::Translation => "meaning shown",
        Cue::OriginalText => "Russian shown",
        Cue::Replay => "audio replay",
        Cue::ReferenceMaterial => "reference material",
        Cue::Coach => "Teacher help",
    }
}

#[component]
fn SoundFocusHistory(mut app: Signal<AppState>, session: Uuid) -> Element {
    let state = app.read().clone();
    let incomplete = state
        .sound_focus_draft()
        .ok()
        .flatten()
        .is_some_and(|draft| draft.session_id == session && draft.history_incomplete);
    let mut packets: Vec<_> = state
        .library
        .events
        .values()
        .filter_map(|event| match &event.fact {
            Fact::SoundTrialPrepared(p) if p.block_id == session => Some((event.id, p.clone())),
            _ => None,
        })
        .collect();
    packets.sort_by_key(|(id, p)| {
        (
            matches!(p.mode, rufren_core::SoundTrialMode::Speaking),
            p.position,
            *id,
        )
    });
    rsx! {section {class:"panel sound-focus-history","data-sound-history":session.to_string(),
        h2 {"Your saved sound practice"}p {class:"quiet","Review the questions, your choices and exact recordings. These examples do not establish a pronunciation grade."}
        if incomplete{p {class:"quiet","Some questions were not completed, or their saved sources are missing. What is available appears below."}}
        if packets.is_empty(){p {role:"alert","The frozen questions are missing. Sync or restore this practice to see its original sources."}}
        for (id,_) in packets {SoundFocusHistoryTrial {key:"history-{id}",app,prepared_id:id}}
        button {class:"text-button",onclick:move |_|app.write().page=Page::Settings,"Sync recordings or restore a backup"}
    }}
}

#[component]
fn SoundFocusHistoryTrial(mut app: Signal<AppState>, prepared_id: Uuid) -> Element {
    let state = app.read().clone();
    let Some(Fact::SoundTrialPrepared(packet)) = state
        .library
        .events
        .get(&prepared_id)
        .map(|event| &event.fact)
    else {
        return rsx! {p {role:"alert","This saved question is missing."}};
    };
    let Some(member) = packet.contrast.member(&packet.target_member_id) else {
        return rsx! {p {role:"alert","This question’s intended meaning is missing."}};
    };
    let Some(reference) = packet
        .references
        .iter()
        .find(|reference| reference.member_id == member.id)
    else {
        return rsx! {p {role:"alert","This question’s exact reference link is missing."}};
    };
    let responses: Vec<_> = state
        .library
        .sound_trial_completion(packet.trial_id)
        .into_iter()
        .filter(|(_, completion)| completion.prepared == prepared_id)
        .map(|(event, completion)| (event.id, completion.clone()))
        .collect();
    let summary = state
        .library
        .sound_trials()
        .into_iter()
        .find(|trial| trial.trial_id == packet.trial_id);
    let incomplete = summary.as_ref().is_none_or(|trial| trial.incomplete);
    let assisted = summary.as_ref().is_some_and(|trial| trial.assisted);
    let conflicting = responses
        .iter()
        .map(|(_, completion)| completion.attempt)
        .collect::<std::collections::BTreeSet<_>>()
        .len()
        > 1;
    let family = format!("sound:{}@{}", packet.contrast.id, packet.contrast.version);
    let replay_reference = reference.clone();
    let speaking = packet.mode == rufren_core::SoundTrialMode::Speaking;
    rsx! {article {class:"sound-focus-history-trial","data-sound-prepared":prepared_id.to_string(),"data-sound-trial":packet.trial_id.to_string(),"data-sound-position":packet.position.to_string(),"data-sound-mode":if speaking{"speaking"}else{"listening"},"data-sound-incomplete":incomplete.to_string(),"data-sound-conflict":conflicting.to_string(),"data-sound-assisted":assisted.to_string(),
        h3 {if speaking{"Your intended spoken meaning"}else{"Listening question {packet.position+1}"}}
        p {strong {"Intended meaning: "}"{member.option_label}"}
        PresentedRussian {app,text:member.utterance.text.clone(),purpose:PresentationPurpose::Reference,revision:Some(reference.revision),utterance_family:Some(member.utterance.id.clone())}
        if !speaking{details {summary {"Original choice order"}ol {for id in &packet.option_member_ids {li {"data-sound-option-order":id.clone(),{packet.contrast.member(id).map(|m|m.option_label.as_str()).unwrap_or("Missing meaning")}}}}}}
        if responses.is_empty(){p {class:"quiet","No response was kept for this question."}}
        if conflicting{p {role:"alert","Different responses were saved for this question. All are kept below; no single result is assumed."}}
        p {class:"quiet",if assisted{"Reference help is recorded for this question."}else if incomplete{"Support status is incomplete until its sources are available."}else{"No reference help is recorded for this question."}}
        for (id,completion) in responses {{
            let attempt=state.library.attempt(completion.attempt);
            let has_context=matches!(state.library.events.get(&completion.context).map(|event|&event.fact),Some(Fact::AttemptContextRecorded(_)));
            let support=attempt.map(|a|a.cues.iter().map(support_label).collect::<Vec<_>>().join(", ")).unwrap_or_default();
            let starts=completion.known_playback_ids.iter().filter(|id|matches!(state.library.events.get(id).map(|e|&e.fact),Some(Fact::SoundReferencePlayed(p))if matches!(p.stage,rufren_core::SoundPlaybackStage::Started))).count();
            rsx!{div {key:"response-{id}",class:"sound-focus-history-response","data-sound-completion":id.to_string(),"data-sound-attempt":completion.attempt.to_string(),
                match &completion.response {
                    rufren_core::SoundTrialResponse::Listening{chosen_member_id}=>rsx!{p {"data-sound-chosen":chosen_member_id.clone(),strong {"Your choice: "}{packet.contrast.member(chosen_member_id).map(|m|m.option_label.as_str()).unwrap_or("Missing meaning")}}},
                    rufren_core::SoundTrialResponse::Speaking{asset_id,asset,attempt_recording,..}=>rsx!{
                        if !matches!(state.library.events.get(attempt_recording).map(|e|&e.fact),Some(Fact::RecordingAttached(_))){p {role:"alert","The saved take’s attachment is missing. Sync its source before continuing."}}
                        SoundFocusSavedTake {app,attempt:completion.attempt,asset_id:asset_id.clone(),sha256:asset.sha256.clone()}
                    },
                }
                if attempt.is_some(){details {summary {"Support kept with this response"}p {class:"quiet","At save: "{if support.is_empty(){"no cue reported"}else{support.as_str()}}". Model starts before this response: {starts}."}}if !has_context{p {role:"alert","The saved question context is missing. Sync or restore it to recover the complete response."}}}
                else{p {role:"alert","The response is still incomplete on this device. Sync its saved attempt and context."}}
            }}
        }}
        SoundFocusExactPlayer {asset_id:reference.asset_id.clone(),sha256:reference.asset.sha256.clone(),label:"Replay this model".to_string(),model_example:true,on_started:move |played:FocusPlayback|{if let Err(error)=note_replay(app,&replay_reference,&family,&played.recording){app.write().notice=error;}}}
    }}
}

#[component]
fn SoundFocusSavedTake(
    app: Signal<AppState>,
    attempt: Uuid,
    asset_id: String,
    sha256: String,
) -> Element {
    let state = app.read().clone();
    let hypotheses = state.library.speech_transcriptions(attempt);
    rsx! {div {"data-sound-saved-asset":asset_id.clone(),SoundFocusExactPlayer {asset_id:asset_id.clone(),sha256,label:"Replay your exact take".to_string()}
        for (event,hypothesis) in hypotheses {details {key:"hypothesis-{event.id}",summary {"Recognized & confirmed words"}p {"Recognized: "}p {lang:"ru","{hypothesis.transcript}"}
            for (_,confirmation) in state.library.transcript_confirmations(event.id){p {"Confirmed / edited: "}p {lang:"ru","{confirmation.text}"}}
            p {class:"quiet","{hypothesis.provenance.model}"}
        }}
    }}
}

#[component]
fn SoundFocusExamples(app: Signal<AppState>, session: Uuid) -> Element {
    let Ok(draft) = current(app, session) else {
        return rsx! {};
    };
    let all_done = draft.labelled_examples_done.iter().all(|done| *done);
    rsx! {section {class:"panel",
        h2 {id:"sound-focus-step-heading",tabindex:"-1","Meet the two sounds"}p {"{draft.contrast.teaching_note}"}
        div {class:"sound-focus-models",for (index,member) in draft.contrast.members.iter().enumerate(){SoundFocusModel {key:"model-{session}-{index}",app,session,member_id:member.id.clone(),example:true}}}
        details {summary {"A little more about this contrast"}p {"{draft.contrast.detail_note}"}p {class:"quiet","The voices are generated examples with authored meanings. They are not a native-speaker quality benchmark."}}
        p {class:"quiet","Listen to both complete examples. Next you’ll hear one at a time with its word hidden."}
        button {class:"primary",disabled:!all_done,onclick:move |_|{act(app,|state|state.sound_focus_continue(session).map(|_|()));focus_step();},"Try listening →"}
    }}
}

#[component]
fn SoundFocusModel(
    mut app: Signal<AppState>,
    session: Uuid,
    member_id: String,
    #[props(default)] example: bool,
) -> Element {
    let starts = use_hook(|| Rc::new(std::cell::RefCell::new(BTreeMap::<String, Uuid>::new())));
    let Ok(draft) = current(app, session) else {
        return rsx! {};
    };
    let Some((index, member)) = draft
        .contrast
        .members
        .iter()
        .enumerate()
        .find(|(_, m)| m.id == member_id)
    else {
        return rsx! {};
    };
    let Some(reference) = draft.references.iter().find(|r| r.member_id == member.id) else {
        return rsx! {p {role:"alert","This exact model link is missing."}};
    };
    let start_member = member_id.clone();
    let completed_member = member_id.clone();
    let start_map = starts.clone();
    let complete_map = starts.clone();
    let kept = !example
        && draft
            .speaking
            .as_ref()
            .is_some_and(|intent| intent.saved_attempt.is_some());
    let token = if example {
        format!("example-{session}-{member_id}")
    } else {
        draft
            .speaking
            .as_ref()
            .map(|i| i.token.to_string())
            .unwrap_or_default()
    };
    let intent_token = draft.speaking.as_ref().map(|i| i.token);
    let replay_reference = reference.clone();
    let family = format!("sound:{}@{}", draft.contrast.id, draft.contrast.version);
    rsx! {article {class:"sound-focus-model","data-sound-model":member_id,
        h3 {"{member.option_label}"}
        PresentedRussian {app,text:member.utterance.text.clone(),class:"russian-display".to_string(),purpose:if example{PresentationPurpose::Instruction}else{PresentationPurpose::Reference},session:Some(session),revision:Some(reference.revision),utterance_family:Some(member.utterance.id.clone())}
        p {"{member.utterance.meaning}"}
        SoundFocusExactPlayer {asset_id:reference.asset_id.clone(),sha256:reference.asset.sha256.clone(),label:"Spoken example".to_string(),model_example:true,token:(!kept).then_some(token),
            on_started:move |played:FocusPlayback|{
                if kept{if let Err(error)=note_replay(app,&replay_reference,&family,&played.recording){app.write().notice=error;}return;}
                let result=(||{let meta=record_metadata(&played.recording)?;if example{app.write().sound_focus_example_started(session,&start_member,&meta)}else{app.write().sound_focus_playback_started(session,intent_token.ok_or("Choose your intended meaning first.")?,&meta)}})();
                match result{Ok(id)=>{start_map.borrow_mut().clear();start_map.borrow_mut().insert(played.playback_id,id);},Err(error)=>app.write().notice=error}
            },
            on_completed:move |played:FocusPlayback|{
                if kept{return;}
                let Some(start)=complete_map.borrow_mut().remove(&played.playback_id)else{return;};
                let result=(||{let meta=record_metadata(&played.recording)?;if example{app.write().sound_focus_example_completed(session,&completed_member,start,&meta)}else{app.write().sound_focus_playback_completed(session,intent_token.ok_or("Choose your intended meaning first.")?,start,&meta)}})();
                if let Err(error)=result{app.write().notice=error;}
            }
        }
        if example&&draft.labelled_examples_done[index]{p {class:"quiet","Example played through"}}
        details {summary {"Voice source"}p {class:"quiet","Generated Russian speech; this exact original is kept with the practice."}pre {class:"audio-provenance","{reference.provenance_json}"}}
    }}
}

#[component]
fn SoundFocusTrial(mut app: Signal<AppState>, session: Uuid, token: Uuid) -> Element {
    let starts = use_hook(|| Rc::new(std::cell::RefCell::new(BTreeMap::<String, Uuid>::new())));
    let Ok(draft) = current(app, session) else {
        return rsx! {};
    };
    let Some(trial) = draft
        .trials
        .get(draft.current_trial)
        .filter(|trial| trial.token == token)
    else {
        return rsx! {};
    };
    let has_saved = !app
        .read()
        .library
        .sound_trial_completion(trial.trial_id)
        .is_empty();
    if trial.conflicted || (has_saved && trial.incomplete) {
        return rsx! {section {class:"panel",h2 {id:"sound-focus-step-heading",tabindex:"-1","This saved question needs attention"}p {role:"alert","Its responses differ, or some saved sources are missing. Your records are kept; a new choice cannot replace them."}button {class:"secondary",onclick:move |_|act(app,|state|state.open_sound_focus_history(session)),"See the saved responses"}}};
    }
    let Some(index) = draft
        .contrast
        .members
        .iter()
        .position(|m| m.id == trial.target_member_id)
    else {
        return rsx! {};
    };
    let member = &draft.contrast.members[index];
    let Some(reference) = draft.references.iter().find(|r| r.member_id == member.id) else {
        return rsx! {p {role:"alert","This exact question model is missing."}};
    };
    let start_map = starts.clone();
    let complete_map = starts.clone();
    let answered = trial.choice_member_id.is_some();
    let show = answered || trial.revealed;
    let chosen = trial
        .choice_member_id
        .as_deref()
        .and_then(|id| draft.contrast.member(id));
    let replay_reference = reference.clone();
    let family = format!("sound:{}@{}", draft.contrast.id, draft.contrast.version);
    rsx! {section {class:"panel","data-sound-trial":trial.trial_id.to_string(),"data-trial-token":token.to_string(),"data-sound-answered":answered.to_string(),
        p {class:"sound-focus-progress","Listening · {draft.current_trial+1} of {draft.trials.len()}"}
        h2 {id:"sound-focus-step-heading",tabindex:"-1","Which meaning did you hear?"}
        p {"Listen at natural speed. Then choose one meaning. There’s no timer."}
        for player_key in [format!("{token}-{answered}")] {{let start_map=start_map.clone();let complete_map=complete_map.clone();let replay_reference=replay_reference.clone();let family=family.clone();rsx!{SoundFocusExactPlayer {key:"{player_key}",asset_id:reference.asset_id.clone(),sha256:reference.asset.sha256.clone(),label:if answered{"Replay this question".to_string()}else{"Hidden word".to_string()},model_example:answered,token:(!answered).then(||token.to_string()),
            on_started:move |played:FocusPlayback|{if answered{if let Err(error)=note_replay(app,&replay_reference,&family,&played.recording){app.write().notice=error;}return;}let result=record_metadata(&played.recording).and_then(|meta|app.write().sound_focus_playback_started(session,token,&meta));match result{Ok(id)=>{start_map.borrow_mut().clear();start_map.borrow_mut().insert(played.playback_id,id);},Err(error)=>app.write().notice=error}},
            on_completed:move |played:FocusPlayback|{if answered{return;}let Some(start)=complete_map.borrow_mut().remove(&played.playback_id)else{return;};let result=record_metadata(&played.recording).and_then(|meta|app.write().sound_focus_playback_completed(session,token,start,&meta));if let Err(error)=result{app.write().notice=error;}}
        }}}}
        if !answered {fieldset {class:"choice-list",legend {"Choose the meaning you heard"},for id in &trial.option_member_ids {if let Some(option)=draft.contrast.member(id){{let choice=id.clone();rsx!{button {class:"choice-option",disabled:!trial.can_answer,"data-sound-choice":choice.clone(),onclick:move |_|act(app,|state|state.sound_focus_answer(session,token,&choice).map(|_|())),"{option.option_label}"}}}}}}
            if !trial.can_answer{p {class:"quiet","Play the whole sound before choosing. Stop or pause whenever you need."}}
            if !trial.revealed{button {class:"text-button",onclick:move |_|act(app,|state|state.sound_focus_reveal(session,token)),"Show me the word · use help"}}
        }
        if show {div {class:"sound-focus-feedback",role:"status",
            if let Some(chosen)=chosen{p {strong {"You chose: "}"{chosen.option_label}"}p {if chosen.id==member.id{"That matches this example’s intended meaning."}else{"This example was made for the other meaning. Compare the sound and try the next one."}}}
            PresentedRussian {app,text:member.utterance.text.clone(),class:"russian-display".to_string(),purpose:if answered{PresentationPurpose::Answer}else{PresentationPurpose::Reference},session:Some(trial.trial_id),revision:Some(reference.revision),utterance_family:Some(member.utterance.id.clone())}
            p {"{member.utterance.meaning}"}
            if !answered{p {class:"quiet","The revealed word stays recorded as help for this question."}}
        }}
        if answered {button {class:"primary",onclick:move |_|{act(app,|state|state.sound_focus_next(session).map(|_|()));focus_step();},if draft.current_trial+1==draft.trials.len(){"Choose a meaning to say →"}else{"Next sound →"}}}
    }}
}

#[component]
fn SoundFocusSpeaking(
    mut app: Signal<AppState>,
    session: Uuid,
    intent_id: Uuid,
    mut capture_busy: Signal<bool>,
) -> Element {
    let mut show_model = use_signal(|| false);
    let Ok(draft) = current(app, session) else {
        return rsx! {};
    };
    let Some(intent) = draft
        .speaking
        .as_ref()
        .filter(|intent| intent.intent_id == intent_id)
    else {
        return rsx! {};
    };
    let Some(index) = draft
        .contrast
        .members
        .iter()
        .position(|m| m.id == intent.member_id)
    else {
        return rsx! {};
    };
    let Some(reference) = draft
        .references
        .iter()
        .find(|r| r.member_id == intent.member_id)
    else {
        return rsx! {p {role:"alert","This intended meaning’s model is missing."}};
    };
    let kept = intent.saved_attempt.is_some();
    let member_id = intent.member_id.clone();
    let token = intent.token;
    let attempt_recording = intent.saved_attempt.and_then(|attempt| {
        app.read()
            .library
            .recordings(AnnotationTarget::Attempt(attempt))
            .into_iter()
            .find(|(_, record)| {
                Some(record.asset_id.as_str()) == intent.selected_asset_id.as_deref()
            })
            .map(|(event, _)| event.id)
    });
    rsx! {div {class:"sound-focus-speaking","data-sound-intent":intent_id.to_string(),"data-sound-kept":kept.to_string(),
        p {"Your intention: {draft.contrast.members[index].option_label}."}
        if !kept{p {class:"quiet","Make a new recording here. Imported or earlier recordings stay available as originals, but do not stand for a fresh production in this turn."}}
        AudioSpeaking {passage_id:intent.passage_id.to_string(),revision_id:intent.revision_id.to_string(),sound_focus_scope:Some(SoundFocusRecorderScope{session_id:session,intent_id,member_id:member_id.clone()}),frozen_reference_asset_id:Some(reference.asset_id.clone()),frozen_reference_hash:Some(reference.asset.sha256.clone()),review_only:kept,
            workspace_id:if cfg!(target_arch="wasm32"){String::new()}else{app.read().device.to_string()},selected_asset_id:intent.selected_asset_id.clone(),on_recording_state:move |busy|capture_busy.set(busy),
            on_saved:move |record:StudioRecording|act(app,|state|{let meta=record_metadata(&record)?;let attachment=state.register_sound_focus_take(session,intent_id,&meta)?;if record.interrupted{state.sound_focus_select_take(session,intent_id,None,false)?;state.notice="Your partial take is kept. Choose it explicitly if it contains your intended response.".into();Ok(())}else{state.sound_focus_select_take(session,intent_id,Some((attachment,meta)),false)}}),
            on_selected:move |record:Option<StudioRecording>|act(app,|state|{let selected=record.as_ref().map(|record|{let meta=record_metadata(record)?;let attachment=state.register_sound_focus_take(session,intent_id,&meta)?;Ok::<_,String>((attachment,meta))}).transpose()?;state.sound_focus_select_take(session,intent_id,selected,true)}),
            on_reference_played:move |_|{if !kept{act(app,|state|state.sound_focus_support(session,token,rufren_core::Cue::ReferenceMaterial));}},
            on_region_played:move |_|{if !kept{act(app,|state|state.sound_focus_support(session,token,rufren_core::Cue::ReferenceMaterial));}}
        }
        if !kept {div {class:"actions",
            button {class:"primary",disabled:capture_busy()||intent.selected_attachment.is_none(),onclick:move |_|act(app,|state|state.sound_focus_keep_take(session,intent_id).map(|_|())),"Keep this fresh take"}
            button {class:"text-button",disabled:capture_busy(),onclick:move |_|{let result=app.write().sound_focus_reveal(session,token);match result{Ok(())=>show_model.set(true),Err(error)=>app.write().notice=error}},"Show / hear my model · use help"}
        }}
        if show_model()||kept {SoundFocusModel {app,session,member_id,example:false}}
        if kept {p {class:"quiet","Your exact take is kept. Play the model and your take at natural speed; notice one difference you want to try next time. Sound tools offer optional original-frame A/B comparison."}
            if let (Some(attempt),Some(recording))=(intent.saved_attempt,attempt_recording){SoundFocusWords {app,attempt,recording}}
            button {class:"primary",disabled:capture_busy(),onclick:move |_|{act(app,|state|state.finish_sound_focus(session));focus_step();},"Finish for now"}
        }
    }}
}

#[derive(Clone)]
struct FocusPlayback {
    recording: StudioRecording,
    playback_id: String,
}

/// Exact, opaque trial playback. A token opts into natural-speed completion;
/// no filename, provider, duration or target text is added to the blind DOM.
#[component]
fn SoundFocusExactPlayer(
    asset_id: String,
    sha256: String,
    label: String,
    #[props(default)] model_example: bool,
    #[props(default)] token: Option<String>,
    #[props(default)] on_started: EventHandler<FocusPlayback>,
    #[props(default)] on_completed: EventHandler<FocusPlayback>,
) -> Element {
    let id = use_hook(|| format!("sound-focus-player-{}", Uuid::new_v4()));
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    let mut error = use_signal(String::new);
    let effect_id = id.clone();
    let effect_alive = alive.clone();
    let expected_asset = asset_id.clone();
    let expected_hash = sha256.clone();
    let expected_token = token.clone();
    use_effect(move || {
        let script = format!(
            "const rufrenPlayerRoot=document.getElementById({});\n{}",
            serde_json::to_string(&effect_id).unwrap_or_default(),
            PRACTICE_BRIDGE
        );
        let alive = effect_alive.clone();
        let asset = expected_asset.clone();
        let hash = expected_hash.clone();
        let token = expected_token.clone();
        spawn(async move {
            let mut bridge = document::eval(&script);
            while let Ok(event) = bridge.recv::<PracticeAudioEvent>().await {
                if !alive.get() {
                    break;
                }
                if event.playback_token != token {
                    continue;
                }
                let Some(recording) = event.recording else {
                    continue;
                };
                if recording.asset_id != asset || recording.content_hash != hash {
                    continue;
                }
                let proof = FocusPlayback {
                    recording,
                    playback_id: event.playback_id.unwrap_or_default(),
                };
                match event.kind.as_str() {
                    "played" => on_started.call(proof),
                    "completed" if token.is_some() => on_completed.call(proof),
                    _ => {}
                }
            }
            if alive.get() {
                error.set(
                    "The player disconnected. Pause and reopen this saved focus to reconnect."
                        .into(),
                );
            }
        });
    });
    let drop_id = id.clone();
    use_drop(move || {
        alive.set(false);
        let id = serde_json::to_string(&drop_id).unwrap_or_default();
        let _ = document::eval(&format!("window.rufrenAudioPlayers?.get({id})?.destroy();"));
    });
    rsx! {div {id,class:"sound-focus-player","data-asset":asset_id,"data-expected-hash":sha256,"data-playback-token":token.clone(),"data-playback-context":if model_example{"example"}else{"recording"},
        h3 {"{label}"}
        div {class:"actions",
            button {class:"primary",disabled:true,"data-practice-rate":"1",if token.is_some(){"▶ Listen"}else{"▶ Play / replay"}}
            if token.is_none(){button {class:"secondary",disabled:true,"data-practice-rate":"0.7","Replay slowly"}}
            button {class:"text-button","data-practice-stop":"true","Stop"}
        }
        p {class:"quiet",role:"status","data-practice-status":"true","Preparing the saved sound…"}
        if !error().is_empty(){p {role:"alert","{error}"}}
    }}
}

/// Word recognition is optional after a real take has been kept. It does not
/// assess this sound contrast, and confirmation never sends a Teacher request.
#[component]
fn SoundFocusWords(
    mut app: Signal<crate::model::AppState>,
    attempt: Uuid,
    recording: Uuid,
) -> Element {
    let state = app.read().clone();
    let latest = state
        .library
        .speech_transcriptions(attempt)
        .into_iter()
        .filter(|(_, t)| t.recording == recording)
        .max_by_key(|(e, _)| (e.recorded_at_ms, e.id));
    let pending = state
        .recognition_job
        .as_ref()
        .is_some_and(|(id, _)| *id == recording);
    rsx! {section {class:"sound-focus-words",
        h3 {"Check the words, if useful"}
        p {class:"quiet","Transcription can help you check which words were recognized. It cannot judge tongue position, stress or accent. Compare the recordings by ear."}
        button {class:"secondary",disabled:state.recognition_job.is_some(),onclick:move |_|transcribe_focus_take(app,recording),if pending{"Recognizing this take…"}else{"Find the words in my take"}}
        if pending {button {class:"text-button",disabled:state.recognition_cancel_requested,onclick:move |_|crate::recognition_view::cancel(app),"Cancel transcription"}}
        if let Some((event,_))=latest {for id in [event.id] {SoundFocusTranscript {key:"sound-words-{id}",app,transcription:id}}}
    }}
}

#[component]
fn SoundFocusTranscript(mut app: Signal<crate::model::AppState>, transcription: Uuid) -> Element {
    let initial = use_hook(|| {
        let state = app.read();
        let heads = state.library.transcript_confirmations(transcription);
        let fallback = if heads.len() == 1 {
            heads[0].1.text.clone()
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
            .map(|value| value.unwrap_or(fallback))
    });
    let mut text = use_signal(|| initial.clone().unwrap_or_default());
    let mut problem = use_signal(|| initial.err().unwrap_or_default());
    let state = app.read().clone();
    let Some(observation) = state.library.speech_transcription(transcription) else {
        return rsx! {p {role:"alert","This take’s transcription is missing. Your original is kept."}};
    };
    let heads = state.library.transcript_confirmations(transcription);
    let saved = heads.len() == 1 && heads[0].1.text == text();
    let field = format!("sound-focus-transcript-{transcription}");
    rsx! {div {class:"sound-focus-transcript",
        p {strong {"Recognized words: "}span {lang:"ru","{observation.transcript}"}}
        label {r#for:field.clone(),"Confirm the words, or edit what it misheard"}
        textarea {id:field,lang:"ru",value:text(),oninput:move |event|{
            let value=event.value();if value.len()>8192{problem.set("Keep the words under 8 KB.".into());return;}
            text.set(value.clone());match app.read().repo.set(&format!("transcript_draft:{transcription}"),&value){Ok(())=>problem.set(String::new()),Err(error)=>problem.set(error)}
        }}
        button {class:"secondary",disabled:text.read().trim().is_empty()||saved,onclick:move |_|{
            match app.write().confirm_transcription(transcription,text()){Ok(_)=>problem.set(String::new()),Err(error)=>problem.set(error)}
        },if saved{"Words confirmed"}else{"Keep these confirmed words"}}
        if !problem().is_empty(){p {role:"alert","{problem}"}}
        details {summary {"Transcription source"}p {"{observation.provenance.model}"}p {"Independent Russian word recognition; no expected answer was supplied."}}
    }}
}

fn transcribe_focus_take(mut app: Signal<crate::model::AppState>, recording: Uuid) {
    if app.read().recognition_job.is_some() {
        return;
    }
    let snapshot = app.read().clone();
    let Some(rufren_core::Fact::RecordingAttached(attachment)) = snapshot
        .library
        .events
        .get(&recording)
        .map(|e| e.fact.clone())
    else {
        app.write().notice = "This exact saved take is unavailable.".into();
        return;
    };
    let request_id = Uuid::new_v4().to_string();
    app.write().recognition_job = Some((recording, request_id.clone()));
    app.write().recognition_cancel_requested = false;
    dioxus_core::spawn_forever(async move {
        let result = async {
            let (metadata, bytes) = crate::asset_sync::read_original(
                &attachment.asset_id,
                &attachment.asset.sha256,
                rufren_speech::RECOGNITION_MAX_AUDIO_BYTES,
            )
            .await?;
            let request = snapshot.recognition_request(recording, &bytes, request_id.clone())?;
            if metadata.role != "attempt"
                || Some(metadata.revision_id.as_str()) != request.revision_id.as_deref()
            {
                return Err("The original belongs to another response.".into());
            }
            if app.read().recognition_cancel_requested {
                return Err("Transcription cancelled.".into());
            }
            let body = serde_json::to_value(&request).map_err(|e| e.to_string())?;
            let response = crate::gateway::request::<rufren_speech::RecognitionResponse>(
                &snapshot.endpoint,
                &snapshot.token,
                "/api/speech/transcribe",
                Some(&body),
                rufren_speech::RECOGNITION_MAX_RESPONSE_BYTES,
                140_000,
            )
            .await?;
            if app.read().recognition_cancel_requested {
                return Err("Transcription cancelled.".into());
            }
            app.write()
                .keep_transcription(recording, &request, &response)?;
            Ok::<_, String>(())
        }
        .await;
        if app
            .read()
            .recognition_job
            .as_ref()
            .is_some_and(|(_, id)| id == &request_id)
        {
            app.write().recognition_job = None;
            app.write().recognition_cancel_requested = false;
            app.write().notice = match result {
                Ok(()) => "The recognized words are ready to check.".into(),
                Err(error) => error,
            };
        }
    });
}

fn original(member: &str) -> Result<(&'static [u8], &'static str)> {
    match member {
        "ru.sound.member.onion" => Ok((
            include_bytes!("../../../research/artifacts/reference-quality-slot1/RQ02.wav"),
            include_str!("../../../research/artifacts/reference-quality-slot1/RQ02.json"),
        )),
        "ru.sound.member.hatch" => Ok((
            include_bytes!("../../../research/artifacts/reference-quality-slot1/RQ03.wav"),
            include_str!("../../../research/artifacts/reference-quality-slot1/RQ03.json"),
        )),
        "ru.sound.member.castle" => Ok((
            include_bytes!("../../../research/artifacts/sound-focus-v1/castle-carrier-v1.wav"),
            include_str!("../../../research/artifacts/sound-focus-v1/castle-carrier-v1.json"),
        )),
        "ru.sound.member.lock" => Ok((
            include_bytes!("../../../research/artifacts/sound-focus-v1/lock-carrier-v1.wav"),
            include_str!("../../../research/artifacts/sound-focus-v1/lock-carrier-v1.json"),
        )),
        _ => Err("This contrast has no packaged original. Existing responses are kept.".into()),
    }
}

/// Install the packaged byte originals at the backend's exact source identities.
/// Assigned publication time is deterministic across replicas; it is explicitly
/// not a claim about when a voice was recorded or synthesized.
fn packaged_sound_models(
    definition: &SoundContrastDefinition,
    identities: [(Uuid, Uuid); 2],
) -> Result<Vec<Value>> {
    let manifest: Value = serde_json::from_str(MANIFEST).map_err(|e| e.to_string())?;
    let published = manifest["published_at_ms"]
        .as_i64()
        .ok_or("Missing model publication time")?;
    let mut models = Vec::new();
    for (member, (passage, revision)) in definition.members.iter().zip(identities) {
        if (passage, revision)
            != rufren_core::sound_reference_identity(
                &member.utterance.id,
                &member.utterance.version,
            )
        {
            return Err("Invalid exact sound source identity".into());
        }
        let (bytes, source) = original(&member.id)?;
        let source: Value = serde_json::from_str(source).map_err(|e| e.to_string())?;
        let hash = format!("{:x}", Sha256::digest(bytes));
        let entry = manifest["originals"]
            .as_array()
            .and_then(|items| items.iter().find(|v| v["member_id"] == member.id))
            .ok_or("Missing packaged model identity")?;
        if source["source_text"] != member.utterance.text
            || entry["source_text"] != member.utterance.text
            || source["sha256"] != hash
            || entry["original_sha256"] != hash
            || entry["utterance_id"] != member.utterance.id
            || entry["utterance_version"] != member.utterance.version
            || bytes.len() > 512 * 1024
            || entry["byte_length"].as_u64() != Some(bytes.len() as u64)
        {
            return Err("The packaged sound does not match this exact authored utterance.".into());
        }
        let pcm = rufren_audio::PcmAudio::from_wav(bytes).map_err(|e| e.to_string())?;
        let shape = pcm.metadata();
        if source["sample_rate"].as_u64() != Some(u64::from(shape.sample_rate))
            || source["channels"].as_u64() != Some(u64::from(shape.channels))
            || source["frames"].as_u64() != Some(shape.frames as u64)
        {
            return Err("The packaged sound's original shape differs from its provenance.".into());
        }
        let mut provenance = source["provenance"]
            .as_object()
            .cloned()
            .ok_or("Missing archived voice provenance")?;
        let source_hash = format!("{:x}", Sha256::digest(member.utterance.text.as_bytes()));
        if let Some(input) = provenance.get("synthesis_input") {
            let input: rufren_speech::SpeechSynthesisInput = serde_json::from_value(input.clone())
                .map_err(|_| "Invalid archived synthesis input")?;
            input
                .validate_for(&member.utterance.text)
                .map_err(|e| e.to_string())?;
        }
        // Provider fields remain verbatim. These three normalized binding fields
        // come from the archived response envelope and verified exact UTF-8 text,
        // not from a newly invented generation request.
        for (key, value) in [
            ("source_text", json!(member.utterance.text)),
            ("source_text_sha256", json!(source_hash)),
            ("request_id", source["request_id"].clone()),
        ] {
            if provenance.get(key).is_some_and(|old| old != &value) {
                return Err(
                    "Archived voice provenance conflicts with its original source envelope.".into(),
                );
            }
            provenance.insert(key.into(), value);
        }
        let recording = StudioRecording {
            asset_id: format!(
                "sound-focus-v1-{}",
                Uuid::new_v5(&revision, hash.as_bytes())
            ),
            content_hash: hash,
            role: "reference".into(),
            name: format!("Sound Focus · {}", member.option_label),
            passage_id: passage.to_string(),
            revision_id: revision.to_string(),
            duration_ms: (shape.duration_seconds * 1000.0).round() as u64,
            sample_rate: shape.sample_rate,
            channels: shape.channels,
            frames: shape.frames as u64,
            byte_length: bytes.len() as u64,
            created_at_ms: published,
            mime_type: "audio/wav".into(),
            interrupted: false,
            device: json!({"provenance":provenance,"source_text":member.utterance.text,"source_request_id":source["request_id"],"packaged_original":{"publication_id":manifest["publication_id"],"timestamp_semantics":manifest["time_semantics"],"utterance_id":member.utterance.id,"utterance_version":member.utterance.version}}),
            annotations: Vec::new(),
        };
        let metadata: rufren_core::AudioRecordMetadata =
            serde_json::from_value(serde_json::to_value(&recording).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        metadata.validate().map_err(|e| e.to_string())?;
        // Refute a consumer-contract mismatch before any binary storage write.
        SoundReferenceBinding {
            member_id: member.id.clone(),
            revision,
            recording: Uuid::new_v5(
                &revision,
                format!("audio:{}:reference", recording.asset_id).as_bytes(),
            ),
            asset_id: recording.asset_id.clone(),
            asset: rufren_core::AudioAsset {
                sha256: recording.content_hash.clone(),
                media_type: recording.mime_type.clone(),
                sample_rate_hz: recording.sample_rate,
                channels: recording.channels,
                frames: recording.frames,
            },
            source_text: member.utterance.text.clone(),
            source_text_sha256: source_hash,
            provenance_json: serde_json::to_string(&recording.device["provenance"])
                .map_err(|e| e.to_string())?,
        }
        .validate()
        .map_err(|e| e.to_string())?;
        models.push(json!({"recording":recording,"wav_base64":STANDARD.encode(bytes)}));
    }
    Ok(models)
}

pub(crate) async fn install_sound_focus_models(
    definition: &SoundContrastDefinition,
    identities: [(Uuid, Uuid); 2],
) -> Result<[StudioRecording; 2]> {
    let models = packaged_sound_models(definition, identities)?;
    let request = json!({"models":models});
    let result = dioxus::document::eval(&format!("const request={request};\n{INSTALLER}"))
        .await
        .map_err(|e| e.to_string())?;
    if let Some(problem) = result.get("error").and_then(Value::as_str) {
        return Err(problem.into());
    }
    serde_json::from_value(result["records"].clone()).map_err(|e| e.to_string())
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    #[test]
    fn bundled_originals_register_and_prepare_both_actual_contrasts() {
        let repo = Rc::new(crate::repository::Repository::open_at(":memory:").unwrap());
        let mut state = AppState::from_repository(repo).unwrap();
        let mut registered = 0;
        for definition in rufren_content::sound_contrasts() {
            let ids = definition.members.each_ref().map(|member| {
                rufren_core::sound_reference_identity(
                    &member.utterance.id,
                    &member.utterance.version,
                )
            });
            let models = packaged_sound_models(&definition, ids).unwrap();
            let mut references = Vec::new();
            for (member, item) in definition.members.iter().zip(models) {
                let record: StudioRecording =
                    serde_json::from_value(item["recording"].clone()).unwrap();
                let (_, original_source) = original(&member.id).unwrap();
                let original_source: Value = serde_json::from_str(original_source).unwrap();
                for (key, value) in original_source["provenance"].as_object().unwrap() {
                    assert_eq!(
                        &record.device["provenance"][key], value,
                        "original provider field {key} must survive normalization"
                    );
                }
                assert_eq!(
                    record.device["provenance"]["request_id"],
                    original_source["request_id"]
                );
                assert_eq!(
                    record.device["provenance"]["source_text"],
                    member.utterance.text
                );
                assert_eq!(
                    record.device["provenance"]["source_text_sha256"],
                    rufren_core::dialogue_text_sha256(&member.utterance.text)
                );
                let binding = state
                    .register_sound_focus_reference(
                        &definition,
                        &member.id,
                        &record_metadata(&record).unwrap(),
                    )
                    .unwrap();
                binding.validate().unwrap();
                references.push(binding);
                registered += 1;
            }
            let references: [SoundReferenceBinding; 2] = references.try_into().unwrap();
            let draft = state.start_sound_focus(&definition, references).unwrap();
            assert_eq!(draft.trials.len(), 4);
            assert_eq!(draft.phase, SoundFocusPhase::Examples);
        }
        assert_eq!(registered, 4);
    }
}
