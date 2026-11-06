//! Small prepared encounters: instruction, sound, one response and concrete repair.
use crate::{
    act,
    exposure_view::PresentedRussian,
    model::{AppState, Page},
};
use dioxus::prelude::*;
use rufren_content::FoundationAction;
use rufren_content::foundations::{FoundationRecallMode, foundation_step_pack_revision};
use rufren_core::{PresentationPurpose, StimulusPresentation};
use uuid::Uuid;
const STYLE: &str = r#".foundation-card{max-width:680px;margin:0 auto;padding-bottom:1rem}.foundation-card h1{font-size:1.6rem;line-height:1.15;margin:.5rem 0}.foundation-card>p{margin:.5rem 0}.foundation-card .foundation-instruction{font-size:1.08rem;line-height:1.45;font-weight:500;margin:.85rem 0 .55rem}.foundation-card details{margin-top:.65rem}.foundation-card button{white-space:normal;overflow-wrap:anywhere}.foundation-card .actions{margin-top:.8rem}.foundation-card>button{margin-right:.5rem;margin-bottom:.35rem}.foundation-context{font-size:1rem;line-height:1.5}.foundation-symbols{font-size:2.7rem;letter-spacing:.12em;line-height:1.25;text-align:center;padding:.35rem 0;margin:.35rem 0}.foundation-symbols>div,.foundation-symbols .russian,.foundation-script{font:inherit!important;line-height:inherit!important;margin:0!important;padding:0!important}.foundation-tray{display:flex;gap:.6rem;flex-wrap:wrap;margin:1rem 0}.foundation-tray button{font-size:1.4rem;min-width:60px;min-height:50px;max-width:100%}.foundation-progress{font-size:.9rem;color:var(--muted)}.foundation-draft{min-height:72px;padding:1rem;border:1px solid var(--line,#d8ded4);border-radius:8px;font-size:2rem}.foundation-feedback{padding:1rem;border-left:4px solid var(--accent);margin:1rem 0}.foundation-active{background:#e3e9df;border-radius:8px}"#;
#[component]
pub(crate) fn Foundations(app: Signal<AppState>) -> Element {
    use_drop(crate::foundation_audio::stop);
    let mut playing = use_signal(|| None::<String>);
    let mut audio_error = use_signal(String::new);
    let mut displayed_session = use_signal(|| None::<Uuid>);
    use_effect(move || {
        if let Ok(current) = app.read().foundation_state() {
            if *displayed_session.peek() != Some(current.session) {
                displayed_session.set(Some(current.session));
                audio_error.set(String::new());
                playing.set(None);
                spawn(async move {
                    let _=document::eval("await new Promise(requestAnimationFrame);const heading=document.getElementById('foundation-step-heading');heading?.focus({preventScroll:true});heading?.scrollIntoView({block:'start',behavior:'instant'});").await;
                });
            }
        }
    });
    let pack = rufren_content::foundations();
    let s = match app.read().foundation_state() {
        Ok(s) => s,
        Err(e) => return rsx! {p {role:"alert","{e}"}},
    };
    let Some(episode) = pack.episodes.get(s.episode) else {
        return rsx! {p{"Saved episode unavailable."}};
    };
    let Some(step) = episode.steps.get(s.step).cloned() else {
        return rsx! {p{"Saved step unavailable."}};
    };
    let is_read = matches!(step.action, FoundationAction::Read { .. });
    let is_recall = matches!(step.action, FoundationAction::Recall { .. });
    let show_audio = match &step.action {
        FoundationAction::Recall { mode, .. } => {
            matches!(mode, FoundationRecallMode::ListeningMeaning) || s.revealed
        }
        _ => !is_read || s.revealed,
    };
    let feedback = s.feedback.clone();
    let pending = s.pending.is_some();
    let session = s.session;
    let replay_ids = step.audio_ids.clone();
    let can_answer = replay_ids.is_empty() || replay_ids.iter().any(|id| s.played.contains(id));
    let support_step = step.clone();
    let refreshers = crate::foundation::foundation_missing_intros(&s, &step);
    let draft_text = match &step.action {
        FoundationAction::Choose { options, .. } => s
            .draft
            .first()
            .and_then(|id| options.iter().find(|o| &o.id == id))
            .map(|o| o.text.clone())
            .unwrap_or_default(),
        _ => crate::foundation::foundation_tiles_text(&step, &s.draft),
    };
    let model_tiles = match &step.action {
        FoundationAction::Blend { answer, .. } => {
            crate::foundation::foundation_tiles_text(&step, answer)
        }
        _ => String::new(),
    };
    rsx! {
     style {"{STYLE}"}
     section {class:"card foundation-card",id:"foundation-card",
      p {class:"eyebrow",if s.review_resume.is_some(){"RECALL PRACTICE"}else{"FOUNDATIONS · EPISODE {s.episode+1} OF {pack.episodes.len()}"}}
      h1 {id:"page-heading",tabindex:"-1","{episode.title}"}
      if s.review_resume.is_none(){p {class:"foundation-progress","Step {s.step+1} of {episode.steps.len()} · untimed"}}
      if s.finished {
       h2 {"A small beginning, ready to revisit"}
       p {"You reached the end of the current small-step course. Return to these words whenever you like. Later lessons include more untaught letters; use their audio and English support when exploring."}
       button {onclick:move |_|foundation_act(app,session,|a|a.navigate(Page::Lessons)),"Explore later lessons with help"}
       button {class:"secondary",onclick:move |_|foundation_act(app,session,|a|a.foundation_repeat_episode()),"Repeat this episode"}
       button {class:"secondary",onclick:move |_|foundation_act(app,session,|a|a.navigate(Page::Today)),"Stop here"}
      } else {
       if !refreshers.is_empty(){
        details {open:true,summary {"A quick introduction for a step you skipped"}
         p {"These models stay available so you can try this as supported practice."}
         for intro in refreshers {
          p {"{intro.instruction}"}
          if let Some(text)=intro.display.clone(){PresentedRussian {app,text,purpose:PresentationPurpose::Reference,session:Some(session)}}
          if let FoundationAction::Present{note}=&intro.action {p {"{note}"}}
          for id in intro.audio_ids.clone(){
           {let asset=id.clone();let spoken=pack.assets.iter().find(|a|a.id==id).map(|a|a.spoken.clone()).unwrap_or_default();rsx!{
            button {class:"secondary",disabled:playing().is_some(),onclick:move |_|{let asset=asset.clone();let spoken=spoken.clone();playing.set(Some(asset.clone()));spawn(async move {let result=crate::foundation_audio::play(&asset).await;if app.read().foundation_state().ok().is_none_or(|s|s.session!=session){return}playing.set(None);match result{Ok(())=>foundation_act(app,session,|a|{if a.foundation_state()?.session!=session{return Ok(())}a.foundation_replayed(&asset)?;a.note_presentation(Uuid::new_v4(),StimulusPresentation{session:Some(session),revision:None,russian:spoken,purpose:PresentationPurpose::Audio,decision:None,utterance_family:Some("foundation-prerequisite-support".into())})}),Err(e)=>audio_error.set(e)}});},"Listen to this support"}
           }}
          }
         }
        }
       }
       p {class:"foundation-instruction",id:"foundation-step-heading",tabindex:"-1","{step.instruction}"}
       if let Some(display)=step.display.clone() {
        if display.chars().any(|c| ('А'..='я').contains(&c)) {
         div {class:if playing().is_some(){"foundation-symbols foundation-active"}else{"foundation-symbols"},
          PresentedRussian {app,class:"foundation-script",text:display,purpose:PresentationPurpose::TaskInput,session:Some(session)}
         }
        } else {p {class:"foundation-context",lang:"en","{display}"}}
       }
       match &step.action {
        FoundationAction::Present{note}|FoundationAction::Speak{note}=>rsx!{p {"{note}"}},
        FoundationAction::Complete{summary}=>rsx!{p {"{summary}"}},
        _=>rsx!{},
       }
       if show_audio {
        for id in replay_ids.clone() {
         {let asset=id.clone();let spoken=pack.assets.iter().find(|a|a.id==id).map(|a|a.spoken.clone()).unwrap_or_default();
          rsx!{button {class:if can_answer{"secondary"}else{"primary"},disabled:playing().is_some(),onclick:move |_|{
           let asset=asset.clone();let spoken=spoken.clone();playing.set(Some(asset.clone()));audio_error.set(String::new());
           spawn(async move {let result=crate::foundation_audio::play(&asset).await;if app.read().foundation_state().ok().is_none_or(|s|s.session!=session){return}playing.set(None);match result{Ok(())=>foundation_act(app,session,|a|{let current=a.foundation_state()?;if current.session!=session{return Ok(())}let purpose=if matches!(rufren_content::foundations().episodes[current.episode].steps[current.step].action,FoundationAction::Recall{mode:FoundationRecallMode::ListeningMeaning,..})&&!current.played.contains(&asset){PresentationPurpose::TaskInput}else{PresentationPurpose::Audio};a.foundation_replayed(&asset)?;a.note_presentation(Uuid::new_v4(),StimulusPresentation{session:Some(session),revision:None,russian:spoken,purpose,decision:None,utterance_family:Some("foundations-v1".into())})}),Err(e)=>audio_error.set(e)}});
          },if playing().as_deref()==Some(id.as_str()){ "Playing…" }else{ "Listen" }}}
         }
        }

       }
       if !audio_error().is_empty(){p {role:"alert","{audio_error}"}p {"Retry Listen, or Skip for now. Skipping does not count as listening practice."}}
       match &step.action {
        FoundationAction::Choose{options,..}=>rsx!{
         if !can_answer {p {class:"quiet","Listen first, then choose what you heard."}}
         div {class:"foundation-tray",
          for option in options {
           {let id=option.id.clone();let task=step.clone();rsx!{button {class:"secondary",disabled:!can_answer||feedback.is_some()||pending||playing().is_some(),onclick:move |_|{let id=id.clone();foundation_act(app,session,|a|{a.foundation_edit(vec![id])?;a.foundation_submit(&task)})},"{option.text}"}}}
          }
         }
        },
        FoundationAction::Blend{tiles,..}=>rsx!{
         div {class:"foundation-draft",aria_label:"Your message or spelling","{draft_text}"}
         div {class:"foundation-tray",for tile in tiles {
          {let tile=tile.clone();rsx!{button {class:"secondary",disabled:feedback.is_some()||pending,onclick:move |_|{let tile=tile.clone();foundation_act(app,session,|a|{let mut current=a.foundation_state()?;current.draft.push(tile);a.foundation_edit(current.draft)})},"{tile}"}}}
         }}
         if feedback.is_none(){
          button {class:"secondary",disabled:s.draft.is_empty()||pending,onclick:move |_|foundation_act(app,session,|a|{let mut current=a.foundation_state()?;current.draft.pop();a.foundation_edit(current.draft)}),"Undo last tile"}
          {let task=step.clone();rsx!{button {class:"primary",disabled:s.draft.is_empty()||pending,onclick:move |_|foundation_act(app,session,|a|a.foundation_submit(&task)),"Check my tiles"}}}
         }
        },
        FoundationAction::Copy{keyboard,note,..}=>rsx!{
         p {"{note}"}
         FoundationEntry {app,session,draft:draft_text.clone(),keyboard:keyboard.clone(),locked:feedback.is_some()||pending,label:"Your copied word".to_string()}
         if feedback.is_none(){{let task=step.clone();rsx!{button {class:"primary",disabled:draft_text.trim().is_empty()||pending,onclick:move |_|foundation_act(app,session,|a|a.foundation_submit(&task)),"Save my copied word"}}}}
        },
        FoundationAction::Recall{mode,answer,reveal,keyboard,..}=>rsx!{
         if matches!(mode,FoundationRecallMode::Spoken){
          {let revision=Uuid::new_v5(&Uuid::NAMESPACE_URL,format!("rufren-foundation:{}:{}",foundation_step_pack_revision(&step.id),step.id).as_bytes());
           rsx!{details {summary {"Record my voice (optional)"}
            p {"You can record a take, or simply try saying it aloud. The reference appears after you save the attempt."}
            crate::audio_studio::AudioSpeaking {key:"foundation-speaking-{session}",passage_id:revision.to_string(),revision_id:revision.to_string(),workspace_id:format!("foundation-{session}"),selected_asset_id:s.recording.as_ref().map(|r|r.asset_id.clone()),
             on_saved:move |recording:crate::audio_studio::StudioRecording|foundation_act(app,session,|a|{let metadata=serde_json::from_value(serde_json::to_value(recording).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;a.foundation_save_recording(metadata)})
            }
           }}
          }
          if feedback.is_none(){{let task=step.clone();let recorded=s.recording.is_some();rsx!{button {class:"primary",disabled:pending,onclick:move |_|foundation_act(app,session,|a|{a.foundation_edit(vec![if recorded{"Recorded speaking attempt (original audio attached)."}else{"I tried saying it aloud (self-report)."}.into()])?;a.foundation_submit(&task)}),if recorded{"Save my recorded attempt — compare"}else{"I tried aloud — compare"}}}}}
         }else{
          FoundationEntry {app,session,draft:draft_text.clone(),keyboard:keyboard.clone(),locked:feedback.is_some()||pending,label:if matches!(mode,FoundationRecallMode::Written){"Your Russian word".to_string()}else{"Your meaning in English".to_string()}}
          if feedback.is_none(){{let task=step.clone();rsx!{button {class:"primary",disabled:draft_text.trim().is_empty()||pending||(!can_answer&&matches!(mode,FoundationRecallMode::ListeningMeaning)),onclick:move |_|foundation_act(app,session,|a|a.foundation_submit(&task)),"Save my answer — compare"}}}}
         }
         if feedback.is_none(){{let task=step.clone();rsx!{button {class:"secondary",disabled:pending||(!can_answer&&matches!(mode,FoundationRecallMode::ListeningMeaning)),onclick:move |_|foundation_act(app,session,|a|a.foundation_cannot_yet(&task)),"I can't recall it yet — show me"}}}}
         if s.revealed {
          div {class:"foundation-model",p {"{reveal}"}
           if matches!(mode,FoundationRecallMode::Written){PresentedRussian {app,text:answer.clone(),purpose:PresentationPurpose::Answer,session:Some(session)}}
           else if !matches!(mode,FoundationRecallMode::Spoken){p {"Example meaning: {answer}"}}
          }
         }
        },
        FoundationAction::Speak{..}=>{
         let revision=Uuid::new_v5(&Uuid::NAMESPACE_URL,format!("rufren-foundation:{}:{}",foundation_step_pack_revision(&step.id),step.id).as_bytes());
         rsx!{details {summary {"Record and replay my voice (optional)"}
          p {"Use Listen above for the prepared reference, then replay your own take here. You decide what you hear; the app does not judge your accent."}
          crate::audio_studio::AudioSpeaking {key:"foundation-speaking-{session}",passage_id:revision.to_string(),revision_id:revision.to_string(),workspace_id:format!("foundation-{session}"),selected_asset_id:s.recording.as_ref().map(|r|r.asset_id.clone()),
           on_saved:move |recording:crate::audio_studio::StudioRecording|foundation_act(app,session,|a|{let metadata=serde_json::from_value(serde_json::to_value(recording).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;a.foundation_save_recording(metadata)})
          }
         }}
        },
        FoundationAction::Read{reveal_text,note}=>rsx!{
         p {"Try saying it aloud to yourself. No microphone is required."}
         if s.revealed {p {"{note}"}PresentedRussian {app,text:reveal_text.clone(),purpose:PresentationPurpose::Answer,session:Some(session)}}
         else {{let task=step.clone();rsx!{button {class:"primary",onclick:move |_|foundation_act(app,session,|a|a.foundation_read_try(&task)),"I tried it — show the model"}}}}
        },
        _=>rsx!{},
       }
       if let Some(feedback)=feedback.clone(){
        div {class:"foundation-feedback",role:"status",p {"Your saved answer: {draft_text}"}p {"{feedback}"}
         if !s.accepted||is_recall{button {class:"secondary",onclick:move |_|foundation_act(app,session,|a|a.foundation_retry()),"Try again with help"}}
        }
       }
       if pending {button {onclick:move |_|foundation_act(app,session,|a|a.foundation_submit(&support_step)),"Retry saving this response"}p {"Your original response is reserved. Feedback appears after it is safely saved."}}
       if (is_recall&&feedback.is_some())||(!is_recall&&!matches!(step.action,FoundationAction::Choose{..}|FoundationAction::Blend{..}|FoundationAction::Copy{..}))||s.accepted {
        button {class:"primary",disabled:pending||playing().is_some()||(matches!(step.action,FoundationAction::Present{..})&&!can_answer)||(is_read&&!s.revealed),onclick:move |_|foundation_act(app,session,|a|{crate::foundation_audio::stop();a.foundation_next(false)}),if s.review_resume.is_some(){"Back to my course"}else if matches!(step.action,FoundationAction::Complete{..}){"Next episode"}else{"Continue"}}
       }
       if matches!(step.action,FoundationAction::Complete{..}){button {class:"secondary",onclick:move |_|foundation_act(app,session,|a|a.foundation_repeat_episode()),"Repeat this episode"}}
       details {
        summary {"Smaller step / Show me"}
        p {"Replay as often as you like. You can stop or skip without losing your place."}
        button {class:"secondary",onclick:move |_|foundation_act(app,session,|a|a.foundation_help()),"Show me"}
        if s.revealed {
         match &step.action {
          FoundationAction::Choose{options,correct_id,..}=>rsx!{for option in options.iter().filter(|o|&o.id==correct_id){p {"{option.feedback}"}}},
          FoundationAction::Recall{..}=>rsx!{p {"The model is shown with this task. You can try again with it nearby."}},
          FoundationAction::Copy{answer,..}=>rsx!{p {"Copy this word: {answer}. Each letter below works like a keyboard key."}},
          FoundationAction::Blend{answer,feedback,..}=>rsx!{p {"Model: {model_tiles}"}for cue in feedback {p {"{cue}"}}},
          _=>rsx!{p {"Replay the model as often as you like, then Continue when ready."}},
         }
        }
       }
       if !is_recall||s.revealed {details {
        summary { "About the recordings" }
        p { "Prepared audio is stored in the app and works offline. It includes human recordings and synthetic models." }
        if step.audio_first_chunk {p {"The source titles below may use letters you have not met yet. You do not need to read them."}}
        for id in replay_ids.clone() {
         {let credit = crate::foundation_audio::credits(&id);
          rsx! {
           div {
            p { strong { "{credit.title}" } }
            p { "{credit.credit}" }
            p {
             a { href: credit.source_url, target: "_blank", rel: "noopener noreferrer", "Source recording" }
             span { " · " }
             a { href: credit.license_url, target: "_blank", rel: "noopener noreferrer", "{credit.license}" }
            }
            p { class: "quiet", "{credit.changes}" }
           }
          }
         }
        }
       }
       }
       div {class:"actions",
        button {class:"secondary",disabled:pending,onclick:move |_|foundation_act(app,session,|a|{crate::foundation_audio::stop();a.foundation_next(true)}),if s.review_resume.is_some(){"Skip and return to my course"}else{"Skip for now"}}
        button {class:"secondary",onclick:move |_|foundation_act(app,session,|a|a.navigate(Page::Today)),"Stop here · save my place"}
       }
      }
     }
    }
}

#[component]
fn FoundationEntry(
    app: Signal<AppState>,
    session: Uuid,
    draft: String,
    keyboard: Vec<String>,
    locked: bool,
    label: String,
) -> Element {
    let written = !keyboard.is_empty();
    rsx! {
     label {r#for:"foundation-response","{label}"}
     input {id:"foundation-response",r#type:"text",value:draft.clone(),disabled:locked,maxlength:"2000",autocomplete:"off",lang:if written{"ru"}else{"en"},oninput:move |event|{let value=event.value();foundation_act(app,session,|a|a.foundation_edit(vec![value]))}}
     if written {
      p {class:"quiet","Tap a taught letter, or use your keyboard. Backspace removes the last character."}
      div {class:"foundation-tray",for key in keyboard {button {class:"secondary",disabled:locked,onclick:move |_|{let key=key.clone();foundation_act(app,session,|a|{let mut value=a.foundation_state()?.draft.join("");value.push_str(&key);a.foundation_edit(vec![value])})},"{key}"}}}
      button {class:"secondary",disabled:locked||draft.is_empty(),onclick:move |_|foundation_act(app,session,|a|{let mut value=a.foundation_state()?.draft.join("");value.pop();a.foundation_edit(vec![value])}),"⌫ Backspace"}
     }
    }
}

fn foundation_act(
    app: Signal<AppState>,
    session: Uuid,
    f: impl FnOnce(&mut AppState) -> crate::repository::Result<()>,
) {
    act(app, |a| {
        if a.foundation_state()?.session != session {
            return Err("That foundation step has changed. Your current work is preserved.".into());
        }
        f(a)
    })
}
