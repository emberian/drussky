use crate::{
    act,
    audio_studio::{AudioPractice, AudioSpeaking, StudioRecording},
    exposure_view::PresentedRussian,
    generate_reference,
    model::{AppState, Page},
    recording_saved,
};
use dioxus::prelude::*;
use rufren_content::{AnswerMatch, AnswerSpec, ResponseMode};
use rufren_core::*;

#[component]
pub(crate) fn Practice(app: Signal<AppState>) -> Element {
    match app.read().sound_focus_is_selected() {
        Ok(true) => {
            let session = app
                .read()
                .sound_focus_draft()
                .ok()
                .flatten()
                .map(|d| d.session_id)
                .unwrap_or_default();
            return rsx! {crate::sound_focus_view::SoundFocusView {key:"sound-focus-{session}",app}};
        }
        Err(problem) => {
            return rsx! {section {class:"panel",h1 {"Your practice needs attention"}p {role:"alert","{problem}"}}};
        }
        Ok(false) => {}
    }
    let active = app.read().active_teacher_revisit();
    match active {
        Ok(Some((answer, snapshot))) => {
            rsx! {for session in [snapshot.id] {crate::teacher_revisit_view::TeacherReturn {key:"active-return-{session}",app,answer:answer.clone(),snapshot:snapshot.clone()}}}
        }
        Ok(None) => {
            let session = app
                .read()
                .session
                .as_ref()
                .map(|s| s.id)
                .unwrap_or_default();
            rsx! {PassagePractice {key:"passage-practice-{session}",app}}
        }
        Err(problem) => {
            rsx! {section {class:"panel",h1 {"Your saved return needs attention"}p {role:"alert","{problem}"}
                p {"Your original task and response are kept. Sync or restore missing material before continuing."}
                button {class:"secondary",onclick:move |_|act(app,|s|s.navigate(Page::Settings)),"Open sync & backups"}
                button {class:"text-button",onclick:move |_|act(app,|s|{s.clear_active_teacher_revisit()?;s.navigate(Page::Today)}),"Return to Today"}
            }}
        }
    }
}

#[component]
fn PassagePractice(mut app: Signal<AppState>) -> Element {
    let mut ready = use_signal(|| None::<String>);
    let mut capture_busy = use_signal(|| false);
    let state = app.read().clone();
    let heading = format!(
        "passage-practice-heading-{}",
        state.session.as_ref().map(|s| s.id).unwrap_or_default()
    );
    let focus_heading = heading.clone();
    use_effect(move || {
        let id = serde_json::to_string(&focus_heading).expect("generated heading ID");
        spawn(async move {
            let _ = document::eval(&format!("await new Promise(requestAnimationFrame);const heading=document.getElementById({id});if(heading){{heading.focus({{preventScroll:true}});heading.scrollIntoView({{block:'start'}});}}")).await;
        });
    });
    let Some(session) = state.session.clone() else {
        return rsx! {div {class:"empty large",h1 {"One phrase at a time."}p {"Choose a lesson or return to a passage you want to remember."}button {class:"primary",onclick:move |_|act(app,|s|s.begin()),"Choose my next step →"}}};
    };
    let Some(revision) = state.library.revision(session.revision) else {
        return rsx! {section {class:"panel",h1 {"This practice needs its reference."}p {"Sync or restore the passage before continuing. Your draft is kept."}button {class:"secondary",onclick:move |_|app.write().page=Page::Settings,"Open sync & backups"}}};
    };
    let p = &revision.passage;
    let ex = state.exercise().cloned();
    let writing = session.skill == Skill::Writing;
    let speaking = session.skill == Skill::Speaking;
    let audio_prompt =
        session.skill == Skill::Listening || ex.as_ref().is_some_and(|e| e.requires_audio);
    let choice = ex
        .as_ref()
        .is_some_and(|e| e.response_mode == ResponseMode::Choice);
    let revision_id = session.revision.to_string();
    let ready_id = revision_id.clone();
    let instruction = ex
        .as_ref()
        .map(|e| e.instruction.clone())
        .unwrap_or_else(|| match session.skill {
            Skill::Reading => "Explain the meaning in your own words.".into(),
            Skill::Writing => "Write the Russian from the meaning.".into(),
            Skill::Listening => "Listen, then explain what you understood.".into(),
            Skill::Speaking => {
                "Express this meaning aloud. Save a recording to listen back and explore.".into()
            }
        });
    let has_audio = ready().as_deref() == Some(revision_id.as_str());
    let played = state.plan.as_ref().is_some_and(|p| p.reference_plays > 0);
    let can_reflect =
        !capture_busy() && !session.draft.trim().is_empty() && (!audio_prompt || played);
    let response_display = ex
        .as_ref()
        .and_then(|e| e.options.iter().find(|o| o.id == session.draft))
        .map(|o| o.text.clone())
        .unwrap_or_else(|| session.draft.clone());
    let answer = ex.as_ref().map(|e| match &e.answer {
        AnswerSpec::Exact { variants, .. } => variants.join(" / "),
        AnswerSpec::Choice { correct_option } => e
            .options
            .iter()
            .find(|o| &o.id == correct_option)
            .map(|o| o.text.clone())
            .unwrap_or_default(),
        AnswerSpec::CoachReview { example, .. } => example.clone(),
    });
    rsx! {
        div {class:"page-heading",div {div {class:"eyebrow","{session.skill.label()} · AT YOUR PACE"}h1 {id:heading.clone(),tabindex:"-1",if session.phase==Phase::Retrieve {"Your turn"}else if let Some(plan)=&state.plan {if let Some(lesson)=plan.lesson_id.as_deref().and_then(|id|state.content.lesson(id)) {"{lesson.title}"}else{"{p.title}"}}else{"{p.title}"}}if session.phase!=Phase::Retrieve {p {"{p.source}"}}}}
        ol {class:"steps",for (i,(phase,label)) in [(Phase::Encounter,"Meet"),(Phase::Retrieve,"Try"),(Phase::Reflect,"Reflect"),(Phase::Complete,"Keep")].into_iter().enumerate() {li {class:if session.phase==phase {"current"}else{""},aria_current:if session.phase==phase {"step"}else{"false"},span {"{i+1}"}"{label}"}}}
        section {class:"panel practice-panel",
            match session.phase {
                Phase::Encounter=>rsx! {
                    span {class:"tag","CONNECT WORDS AND MEANING"}{rsx!{PresentedRussian {key:"encounter-{session.id}",app,class:"russian-display",text:p.russian.clone(),purpose:PresentationPurpose::Instruction,revision:Some(session.revision),session:Some(session.id),decision:state.presentation_decision()}}}p {class:"translation","{p.translation}"}
                    p {class:"prompt","{instruction}"}p {class:"quiet","Take as long as you like. The next step gives you room to try, with support whenever you need it."}
                    if audio_prompt {p {"This exercise uses a spoken reference. Generate its voice in the next step, or use a recording you already have."}}
                    button {class:"primary",onclick:move |_|act(app,|s|s.phase(Phase::Retrieve)),"I'm ready to try →"}
                },
                Phase::Retrieve=>rsx! {
                    span {class:"tag","YOUR TURN"}h2 {"{instruction}"}
                    if audio_prompt {
                        AudioPractice {key:"listen-{session.id}",revision_id:revision_id.clone(),on_ready:move |is_ready:bool|{ready.set(is_ready.then(||ready_id.clone()));},on_played:move |_|act(app,|s|s.heard_reference()),generating:state.speech_busy,on_generate:move |_|generate_reference(app,session.revision)}
                        if !has_audio {button {class:"secondary",onclick:move |_|act(app,|s|s.open_studio(session.revision)),"Add a reference recording"}}
                    } else if writing || speaking {p {class:"prompt","{p.translation}"}}
                    else if let Some(ex)=&ex {for id in &ex.stimulus_phrase_ids {if let Some(phrase)=state.stimulus(id) {PresentedRussian {key:"input-{session.id}-{id}",app,class:"russian-display",text:phrase.russian.clone(),purpose:PresentationPurpose::TaskInput,session:Some(session.id),decision:state.presentation_decision()}}}}
                    else {PresentedRussian {key:"input-{session.id}",app,class:"russian-display",text:p.russian.clone(),purpose:PresentationPurpose::TaskInput,revision:Some(session.revision),session:Some(session.id),decision:state.presentation_decision()}}
                    if choice {
                        if let Some(ex)=&ex {fieldset {class:"choice-list",legend {"Choose the meaning"}
                            for option in &ex.options {{let id=option.id.clone();rsx! {button {class:if session.draft==option.id {"choice-option selected"}else{"choice-option"},aria_pressed:session.draft==option.id,onclick:move |_|act(app,|s|s.update_response(id.clone())),"{option.text}"}}}}
                        }}
                    } else if speaking {
                        AudioSpeaking {key:"speak-{session.id}",passage_id:p.id.to_string(),revision_id:revision_id.clone(),selected_asset_id:state.plan.as_ref().and_then(|plan|plan.response_recording.as_ref()).map(|record|record.asset_id.clone()),workspace_id:if cfg!(target_arch="wasm32") {String::new()}else{state.device.to_string()},on_region_played:move |played|act(app,|s|s.heard_region(played)),on_saved:move |record|act(app,|s|recording_saved(s,record)),on_selected:move |record|act(app,|s|select_spoken_take(s,session.id,record)),on_recording_state:move |busy|capture_busy.set(busy),on_reference_played:move |_|act(app,|s|s.heard_reference()),generating_reference:state.speech_busy,on_generate_reference:move |_|generate_reference(app,session.revision)}
                        if !session.draft.is_empty() {p {class:"quiet","{session.draft}"}}
                    } else {
                        label {r#for:"practice-response",if writing {"Your words in Russian"}else{"What did you understand?"}}
                        textarea {id:"practice-response",class:"response",value:session.draft.clone(),oninput:move |e|act(app,|s|s.update_response(e.value())),placeholder:"There’s no timer. Your draft is kept."}
                    }
                    if !session.cues.contains(&Cue::OriginalText) && !session.cues.contains(&Cue::Translation) {button {class:"text-button",onclick:move |_|act(app,|s|s.hint()),"Give me a hint"}}
                    else {div {class:"hint",span {class:"tag","A LITTLE SUPPORT"}
                        if let Some(ex)=&ex {p {"{ex.hint}"}}
                        if session.skill==Skill::Reading {if let Some(ex)=&ex {for id in &ex.stimulus_phrase_ids {if let Some(phrase)=state.stimulus(id) {p {span {lang:"ru","{phrase.russian}"}" — {phrase.translation}"}}}}else{p {"{p.translation}"}}}
                        else if writing || speaking {PresentedRussian {key:"hint-{session.id}",app,text:p.russian.split_whitespace().next().unwrap_or(&p.russian).to_string(),purpose:PresentationPurpose::Reference,revision:Some(session.revision),session:Some(session.id)}}
                        else {PresentedRussian {key:"hint-{session.id}",app,text:p.russian.clone(),purpose:PresentationPurpose::Reference,revision:Some(session.revision),session:Some(session.id)}}
                    }}
                    div {class:"actions",button {class:"primary",disabled:!can_reflect,onclick:move |_|act(app,|s|s.phase(Phase::Reflect)),"Compare my response →"}}
                },
                Phase::Reflect=>rsx! {
                    span {class:"tag","YOUR RESPONSE"}p {class:"prompt","{response_display}"}hr {}{rsx!{PresentedRussian {key:"answer-{session.id}",app,class:"russian-display",text:p.russian.clone(),purpose:PresentationPurpose::Answer,revision:Some(session.revision),session:Some(session.id),decision:state.presentation_decision()}}}p {class:"translation","{p.translation}"}
                    if let Some(ex)=&ex {div {class:"authored-feedback",p {match ex.check_answer(&session.draft) {AnswerMatch::Accepted=>"Your answer matches an authored answer for this task.",AnswerMatch::NotMatched=>"Your answer differs from the authored example. There may be another valid way to say it; ask the teacher to explore your answer.",AnswerMatch::NeedsReview=>"This is an open response. Use the example to reflect, then ask the teacher about your answer."}}
                        if let Some(answer)=answer {p {strong {"Authored example: "}"{answer}"}}
                        p {"{ex.feedback}"}
                        if let AnswerSpec::CoachReview{criteria,..}=&ex.answer {ul {for criterion in criteria {li {"{criterion}"}}}}
                    }}
                    h2 {"How did that feel?"}p {class:"quiet","This is your reflection. Hints and reference replays stay recorded alongside it."}
                    div {class:"rating-buttons",button {class:"secondary",onclick:move |_|act(app,|s|s.record(SelfRating::Again)),"I'd like another look"}button {class:"secondary",onclick:move |_|act(app,|s|s.record(SelfRating::WithHelp)),"I needed some help"}button {class:"primary",onclick:move |_|act(app,|s|s.record(SelfRating::Recalled)),"I recalled it"}}
                },
                Phase::Complete=>rsx! {
                    div {class:"completion-mark","✦"}h2 {"A little more familiar."}p {"Your response and the support you used are saved. Give it some time, then find out what stays with you."}
                    p {class:"quiet","{next_review(&state,session.revision,session.skill)}"}
                    crate::review_timing_view::ReviewStartDetails {app,session:session.id,target:PracticeBatchKey::Passage {revision:session.revision,skill:session.skill}}
                    crate::daily_plan_view::SessionNext {app}
                    div {class:"actions",button {class:"primary",onclick:move |_|app.write().page=Page::Today,"Back to today"}button {class:"secondary",onclick:move |_|app.write().page=Page::Lessons,"Return to your lesson"}button {class:"text-button",onclick:move |_|app.write().page=Page::Teacher,"Get teacher feedback"}}
                },
            }
        }
    }
}

fn select_spoken_take(
    state: &mut AppState,
    session: uuid::Uuid,
    record: Option<StudioRecording>,
) -> crate::repository::Result<()> {
    let active = state
        .session
        .as_ref()
        .ok_or("Reopen your speaking practice to choose a take.")?;
    if active.id != session || active.skill != Skill::Speaking || active.phase != Phase::Retrieve {
        return Err("This take belongs to a different speaking practice.".into());
    }
    let (attachment, label) = if let Some(record) = record {
        if record.role != "attempt" || record.revision_id != active.revision.to_string() {
            return Err("Choose a response recorded for this passage.".into());
        }
        let id = crate::register_recording(state, &record)?;
        let Some(Fact::RecordingAttached(attachment)) =
            state.library.events.get(&id).map(|event| &event.fact)
        else {
            return Err("Reopen the saved take to select it.".into());
        };
        (
            Some(attachment.clone()),
            format!(
                "Recorded response · {:.1} seconds{}",
                record.duration_ms as f64 / 1000.0,
                if record.interrupted {
                    " · partial take"
                } else {
                    ""
                }
            ),
        )
    } else {
        (None, String::new())
    };
    if let Some(mut plan) = state.plan.clone() {
        plan.response_recording = attachment;
        state.repo.set("session_plan", &plan)?;
        state.plan = Some(plan);
    }
    state.update_response(label)
}

fn next_review(state: &AppState, revision: uuid::Uuid, skill: Skill) -> String {
    if state
        .library
        .events
        .get(&revision)
        .is_some_and(is_teacher_generated_reference)
    {
        return "Teacher exercises return with their saved question. This general passage practice is kept separately.".into();
    }
    let now = crate::repository::now_ms();
    let review = SpacedReview.project(&state.library, ReviewKey { revision, skill }, now);
    let timing = state.review_timing(PracticeBatchKey::Passage { revision, skill }, review.due_at_ms, now);
    if timing.paused { return "Automatic returns for this task are paused. Your response is saved.".into(); }
    let Some(due) = timing.effective_due_at_ms else {
        return "Some saved timing history is still missing. Your response is saved.".into();
    };
    let minutes = due.saturating_sub(now).max(0).saturating_add(59_999) / 60_000;
    if minutes < 60 {
        format!("Suggested revisit in about {minutes} minutes.")
    } else if minutes < 1440 {
        format!("Suggested revisit in about {} hours.", minutes / 60)
    } else {
        format!("Suggested revisit in about {} days.", minutes / 1440)
    }
}
