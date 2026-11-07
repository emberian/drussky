mod apple_export;
mod asset_sync;
mod audio_studio;
mod coach;
mod daily_plan;
mod daily_plan_view;
mod dialogue_audio;
mod dialogue_audio_view;
mod drafts;
mod drafts_view;
mod editor_drafts;
mod editor_drafts_view;
mod exposure;
mod exposure_view;
mod foundation;
mod foundation_audio;
mod foundation_review;
mod foundation_view;
mod gateway;
mod handwriting;
mod handwriting_native;
mod handwriting_view;
mod learning;
mod lessons;
mod model;
mod navigation;
mod practice;
mod progress;
mod progress_view;
mod reader;
mod reader_audio;
mod reader_view;
mod recognition;
mod recognition_view;
mod review_timing;
mod review_timing_view;
mod sound_focus;
mod sound_focus_view;
mod speech_playback;
mod teacher;
mod teacher_audio_practice;
mod teacher_audio_practice_view;
mod teacher_context;
mod teacher_dialogue;
mod teacher_dialogue_view;
mod teacher_example_audio;
mod teacher_example_view;
mod teacher_examples;
mod teacher_feedback_view;
mod teacher_lesson_view;
mod teacher_notebook;
mod teacher_notebook_view;
mod teacher_practice;
mod teacher_practice_view;
mod teacher_prose;
mod teacher_revisit;
mod teacher_revisit_view;
mod teacher_view;
mod tutor_view;
mod workspace_view;
mod writing_history_view;
mod writing_speech;
use audio_studio::{AudioStudio, StudioRecording};
use exposure_view::PresentedRussian;
use lessons::Lessons;
use practice::Practice;
use progress_view::Progress;
use teacher_view::Teacher;
#[cfg(not(target_arch = "wasm32"))]
mod native_connection;
#[cfg(not(target_arch = "wasm32"))]
mod native_http;
mod repository;
use dioxus::prelude::*;
use dioxus_core::spawn_forever;
use model::*;
use repository::{Result, synchronize};
use rufren_core::*;

const STYLE: &str = include_str!("style.css");
const FEEDBACK_STYLE: &str = include_str!("teacher_feedback_view.css");
fn main() {
    #[cfg(all(feature = "desktop", not(target_arch = "wasm32")))]
    dioxus::LaunchBuilder::desktop()
        .with_cfg(
            dioxus::desktop::Config::new()
                .with_window(dioxus::desktop::WindowBuilder::new().with_title("Drussky")),
        )
        .launch(App);
    #[cfg(not(all(feature = "desktop", not(target_arch = "wasm32"))))]
    dioxus::launch(App);
}

fn act(mut app: Signal<AppState>, f: impl FnOnce(&mut AppState) -> Result<()>) {
    let result = {
        let mut state = app.write();
        f(&mut state)
    };
    if let Err(e) = result {
        app.write().notice = e;
    }
}

#[component]
fn App() -> Element {
    let initial = use_resource(|| async {
        repository::initialize_browser().await?;
        let mut state = AppState::open()?;
        if let Err(problem) = state.recover_cached_teacher_responses() {
            state.notice = problem;
        }
        match state.reconcile_dialogue() {
            Ok(report) if !report.diagnostics.is_empty() => {
                state.notice = report.diagnostics.join(" ")
            }
            Err(problem) => state.notice = problem,
            _ => {}
        }
        Ok::<_, String>(state)
    });
    let snapshot = initial.read().clone();
    match snapshot {
        None => {
            rsx! {style {"{STYLE}"} main {class:"fatal",h1 {"Opening your notebook…"}p {"Loading your saved practice and recordings."}}}
        }
        Some(Ok(state)) => rsx! {Shell {initial:state}},
        Some(Err(error)) => rsx! {
            style {"{STYLE}"} main {class:"fatal",h1 {"Your notebook could not be opened."} p {"{error}"} p {"Your saved data has not been replaced. Restore access to storage and reopen Drussky."}}
        },
    }
}
#[component]
fn Shell(initial: AppState) -> Element {
    let mut app = use_signal(|| initial);
    let state = app.read().clone();
    let destination = navigation::current(&state).label();
    rsx! {
        style {"{STYLE}"}
        style {"{FEEDBACK_STYLE}"}
        div {class:if state.page==Page::Foundations{"shell lesson-active"}else{"shell"},
            aside {class:"sidebar",
                a {class:"brand",href:"#",onclick:move |event|{event.prevent_default();handwriting_view::nav_act(app,|s|navigation::open(s,navigation::Destination::Today));}, span {class:"brand-mark","р"} strong {"Drussky"}}
                navigation::PrimaryNavigation {app,compact:false}
                navigation::SecondaryNavigation {app,compact:false}
                div {class:"sidebar-bottom",span {class:"status-dot"} "On this device"}
            }
            main {class:"main",
                header {class:"topbar",span {"Drussky"} span {class:"quiet","{destination}"}}
                if !matches!(state.page,Page::Foundations|Page::Handwriting) {navigation::SecondaryNavigation {app,compact:true}}
                if !state.notice.is_empty() {div {class:"notice",role:"status",span {"{state.notice}"} button {aria_label:"Dismiss message",onclick:move |_|app.write().notice.clear(),"×"}}}
                match state.page {
                    Page::Today=>rsx!{Today {app}},Page::Foundations=>rsx!{foundation_view::Foundations {app}},Page::Handwriting=>rsx!{handwriting_view::Writing {app}},Page::Lessons | Page::LessonDetail=>rsx!{Lessons {app}},Page::Teacher=>rsx!{Teacher {app}},Page::Library=>rsx!{LibraryView {app}},Page::Practice=>rsx!{Practice {app}},
                    Page::Reader=>rsx!{reader_view::ReaderView {app}},Page::Studio=>rsx!{Studio {app}},Page::Progress=>rsx!{Progress {app}},Page::Settings=>rsx!{Settings {app}},
                }
            }
            navigation::PrimaryNavigation {app,compact:true}
        }
    }
}

fn focus_page_heading() {
    spawn(async move {
        let _ = document::eval("await new Promise(requestAnimationFrame); const heading = document.querySelector('main.main h1'); if (heading) { heading.setAttribute('tabindex', '-1'); heading.focus({preventScroll:true}); heading.scrollIntoView({block:'start'}); }").await;
    });
}
#[component]
fn Today(mut app: Signal<AppState>) -> Element {
    let state = app.read().clone();
    let count = state
        .library
        .heads()
        .values()
        .filter(|ids| {
            !ids.iter().all(|id| {
                state.library.events.get(id).is_some_and(|event| {
                    is_reader_audio_reference(event) || is_foundation_reference(event)
                })
            })
        })
        .count();
    let now = repository::now_ms();
    let attempts = state
        .library
        .attempts()
        .filter(|(event, _)| event.recorded_at_ms <= now)
        .count();
    let resume = state
        .active_teacher_revisit()
        .ok()
        .flatten()
        .is_some_and(|(_, draft)| draft.saved_attempt.is_none())
        || state
            .session
            .as_ref()
            .is_some_and(|s| s.phase != Phase::Complete);
    let sound_resume = state.sound_focus_unfinished().unwrap_or(false);
    let resume = resume || (sound_resume && state.sound_focus_is_selected().unwrap_or(false));
    let teaching = teacher_notebook_view::unfinished_lesson(&state);
    let teaching_id = teaching.as_ref().map(|entry| entry.document_id);
    let foundation = state.foundation_state();
    let foundation_started = state.foundation_has_started();
    let foundation_ready = foundation.as_ref().is_ok_and(|s| !s.finished);
    let foundation_review_active = foundation.as_ref().is_ok_and(|s| s.review_resume.is_some());
    let foundation_returns = if foundation_review_active {
        Vec::new()
    } else {
        state.foundation_review_queue(now)
    };
    let writing_resume = state.handwriting_resume_available();
    let foundation_primary = !resume && foundation_ready;
    let teaching_title = teaching
        .as_ref()
        .map(teacher_notebook_view::title)
        .unwrap_or_default();
    let returning =
        resume || sound_resume || teaching.is_some() || attempts > 0 || foundation_started;
    let teacher_returns = state.teacher_revisits(now);
    let show_teacher_returns =
        !teacher_returns.entries.is_empty() || !teacher_returns.diagnostics.is_empty();
    let show_passage_returns = SpacedReview
        .queue(&state.library, now, &Skill::ALL)
        .iter()
        .any(|entry| entry.state.attempts > 0);
    let finished = !resume
        && !foundation_primary
        && state
            .daily_plan(now)
            .ok()
            .flatten()
            .is_some_and(|plan| !plan.items.is_empty() && plan.is_finished());
    rsx! {
        section {class:"hero hero-compact",
            div {class:"eyebrow","YOUR RUSSIAN, AT YOUR PACE"}
            h1 {if returning {"Welcome back."}else{"Make yourself at home."}}
            p {class:"lede",if foundation_review_active {"Try a familiar word, then compare with its model. Your place in the course is kept."}else if foundation_primary {"Start with a sound, meet its letters, and try a little Russian. Everything you need for these first steps is already here."}else{"A conversation, a passage, a little practice. Follow what interests you."}}
            div {class:"actions",
                button {id:"today-primary",class:"primary",onclick:move |_|act(app,|s|{
                    if resume {s.begin()}else if foundation_primary {s.start_foundations()}else if writing_resume {s.resume_handwriting()}else if finished {s.navigate(Page::Progress)}else{s.begin()}
                }),if resume {"Continue learning"}else if foundation_primary && foundation_review_active {"Continue recall practice"}else if foundation_primary && foundation_started {"Continue sounds & letters"}else if foundation_primary {"Start sounds & letters"}else if writing_resume {"Continue writing and speaking"}else if finished {"See your progress"}else{"Continue learning"}}
                button {class:"secondary",onclick:move |_|act(app,|s|navigation::open(s,navigation::Destination::Talk)),"Let’s talk"}
                button {class:"text-button",onclick:move |_|act(app,|s|s.navigate(Page::Reader)),"Find something to read"}
            }
        }
        if let Err(problem)=foundation {p {role:"alert","{problem}"}}
        if !foundation_returns.is_empty() {
            section {class:"panel",id:"foundation-returns",
                h2 {"A little recall practice"}
                p {"Return to a few words you have met. Try first, then listen or look again."}
                div {class:"actions",
                    for entry in foundation_returns {
                        button {class:"secondary",onclick:move |_|act(app,|s|s.foundation_start_review(&entry.step_id)),"{entry.label}"}
                    }
                }
            }
        }
        if let Some(id)=teaching_id {section {class:"panel",h2 {"Your saved AI lesson"}p {PresentedRussian {app,text:teaching_title,language:"en",inline:true,purpose:PresentationPurpose::Instruction}}button {class:"secondary",onclick:move |_|act(app,|s|{s.navigate(Page::Teacher)?;s.select_teacher_document(id)?;s.repo.set("teacher_surface",&"lessons")}),"Return to this AI lesson"}}}
        details {class:"today-session",summary {"Your practice session"}daily_plan_view::DailyPractice {app}}
        ContinueSoundFocus {app}
        PausedSoundFocus {app}
        if show_teacher_returns {details {class:"panel",summary {"Browse your saved Teacher returns"}teacher_revisit_view::TeacherReturns {app,expanded:true}}}
        if show_passage_returns {details {class:"panel",summary {"Browse your saved passage returns"}review_timing_view::PassageReturns {app}}}
        drafts_view::PausedPractices {app}
        div {class:"quiet-summary",span {"{count} passages in your notebook"}span {"{attempts} saved practice attempts"}}
    }
}
#[component]
fn ContinueSoundFocus(app: Signal<AppState>) -> Element {
    let state = app.read();
    let loaded = state.sound_focus_draft();
    match loaded {
        Ok(Some(draft))
            if !draft.historical && draft.phase != sound_focus::SoundFocusPhase::Finished =>
        {
            let session = draft.session_id;
            rsx! {section {class:"panel","data-sound-continue":session.to_string(),
                h2 {"Continue your sound practice"}p {"{draft.contrast.title}"}
                p {class:"quiet","Your place, choices and recordings are kept."}
                button {class:"secondary",onclick:move |_|act(app,|s|s.resume_sound_focus(session)),"Continue Sound Focus →"}
            }}
        }
        Err(problem) => {
            rsx! {section {class:"panel",h2 {"Your sound practice needs attention"}p {role:"alert","{problem}"}
                button {class:"secondary",onclick:move |_|act(app,|s|s.navigate(Page::Settings)),"Open sync & backups"}
            }}
        }
        _ => rsx! {},
    }
}
#[component]
fn PausedSoundFocus(app: Signal<AppState>) -> Element {
    match app.read().paused_sound_focus() {
        Ok(paused) if !paused.is_empty() => {
            rsx! {section {class:"panel",h2 {"Other sound practice you paused"}
                for (id,title) in paused {div {class:"history-row","data-sound-paused":id.to_string(),
                    strong {"{title}"}button {class:"secondary",onclick:move |_|act(app,|s|s.resume_sound_focus(id)),"Resume this sound practice"}
                }}
            }}
        }
        Err(problem) => rsx! {p {role:"alert","{problem}"}},
        _ => rsx! {},
    }
}
#[component]
fn LibraryView(mut app: Signal<AppState>) -> Element {
    let state = app.read().clone();
    let d = state.editor.clone();
    let heads: std::collections::BTreeMap<_, _> = state
        .library
        .heads()
        .into_iter()
        .filter(|(_, revisions)| {
            !revisions.iter().all(|id| {
                state.library.events.get(id).is_some_and(|event| {
                    is_reader_audio_reference(event) || is_foundation_reference(event)
                })
            })
        })
        .collect();
    let sound_reference = d
        .parent
        .and_then(|id| state.library.events.get(&id))
        .is_some_and(is_sound_focus_reference);
    let conflicts =
        d.id.and_then(|id| heads.get(&id))
            .is_some_and(|v| v.len() > 1);
    rsx! {
        div {class:"page-heading",div {div {class:"eyebrow","YOUR COLLECTION"}h1 {"Your notebook"}p {"Keep the original, its meaning, and where it came from."}}button {class:"secondary",onclick:move |_|act(app,|s|s.new_passage()),"New passage"}}
        editor_drafts_view::EditorDrafts {app}
        div {class:"library-layout",
            section {class:"passage-list",aria_label:"Saved passages",
                if heads.is_empty() {div {class:"empty",h3 {"Your first page is waiting."}p {"Keep a phrase you care about, or start with your first lesson."}button {class:"text-button",onclick:move |_|{act(app,|s|s.navigate(Page::Lessons));},"Explore the first lessons →"}}}
                for revisions in heads.values() {for id in revisions {if let Some(r)=state.library.revision(*id) {
                    {let revision=*id;rsx!{button {class:if d.parent==Some(revision) {"passage-card selected"}else{"passage-card"},onclick:move |_|act(app,|s|s.edit(revision)),
                        span {class:"tag",if revisions.len()>1 {"Competing revision"}else{"RUSSIAN"}}h3 {"{r.passage.title}"}PresentedRussian {key:"library-{id}",app,text:r.passage.russian.clone(),revision:Some(*id),purpose:PresentationPurpose::Reference}small {"{r.passage.source}"}
                    }}}
                }}}
            }
            section {class:"panel editor",h2 {if d.id.is_some() {"Your passage"}else{"Add something you love"}}
                p {class:"quiet",if d.pending_save.is_some() {"A save was interrupted. Choose Save passage to finish it; your words are kept."}else{"Draft saved as you type on this device. Save passage to add this version to your library."}}
                label {r#for:"passage-title","Title"}input {id:"passage-title",value:d.title,oninput:move |e|act(app,|s|s.update_editor(editor_drafts::EditorField::Title,e.value())),placeholder:"A line worth learning"}
                label {r#for:"passage-russian","Russian text"}textarea {id:"passage-russian",lang:"ru",class:"russian-input",value:d.russian,oninput:move |e|act(app,|s|s.update_editor(editor_drafts::EditorField::Russian,e.value())),placeholder:"Russian text…"}
                label {r#for:"passage-meaning","Meaning or translation"}textarea {id:"passage-meaning",value:d.translation,oninput:move |e|act(app,|s|s.update_editor(editor_drafts::EditorField::Translation,e.value())),placeholder:"What does it mean in this context?"}
                label {r#for:"passage-source","Source or conversation note"}input {id:"passage-source",value:d.source,oninput:move |e|act(app,|s|s.update_editor(editor_drafts::EditorField::Source,e.value())),placeholder:"Book, author, conversation, or your own words"}
                if conflicts {div {class:"notice",p {"This passage was edited in more than one place. Both versions are kept. Select a version to inspect it; resolve when this editor contains the wording you want."}}}
                div {class:"actions",button {class:"primary",onclick:move |_|act(app,|s|s.save_passage(false)),"Save passage"}
                    if conflicts {button {class:"secondary",onclick:move |_|act(app,|s|s.save_passage(true)),"Resolve with this wording"}}
                }
                if let Some(revision)=d.parent {
                    for link in state.reader_sources_for_revision(revision) {
                        {let document=link.document_id;let chunk=link.chunk_id.clone();rsx!{button {class:"text-button",onclick:move |_|act(app,|s|{s.open_reader_document(document,chunk.as_deref())?;s.navigate(Page::Reader)?;focus_page_heading();Ok(())}),"View original source →"}}}
                    }
                    if !conflicts {div {class:"practice-options",p {"Practice the saved version"}
                    if sound_reference {
                        button {class:"secondary",onclick:move |_|act(app,|s|s.choose_lesson("ru.lesson.naming")),"Explore Sound Focus"}
                    } else {
                    button {class:"secondary",onclick:move |_|act(app,|s|s.start(revision,Skill::Reading)),"Read for meaning"}
                    button {class:"secondary",onclick:move |_|act(app,|s|s.start(revision,Skill::Writing)),"Write from memory"}
                    button {class:"secondary",onclick:move |_|act(app,|s|s.start(revision,Skill::Listening)),"Listen for meaning"}
                    button {class:"secondary",onclick:move |_|act(app,|s|s.start(revision,Skill::Speaking)),"Speak from meaning"}
                    }
                    button {class:"text-button",onclick:move |_|act(app,|s|s.open_studio(revision)),"Hear & explore the voice"}
                }}}
            }
        }
    }
}
#[component]
fn Settings(mut app: Signal<AppState>) -> Element {
    let state = app.read().clone();
    rsx! {
        div {class:"page-heading",div {div {class:"eyebrow","YOUR NOTEBOOK, YOUR DEVICES"}h1 {"Keep your Russian with you"}p {"Practice saves locally. Connect your own sync library when you’re ready."}}}
        section {class:"panel",h2 {"How you like to learn"}
            div {class:"preference-row",input {id:"show-stress",r#type:"checkbox",checked:state.preferences.show_stress,onchange:move |e|act(app,|s|{s.preferences.show_stress=e.checked();s.save_preferences()})}label {r#for:"show-stress","Show stress marks in lesson references"}}
            div {class:"preference-row",input {id:"show-chunks",r#type:"checkbox",checked:state.preferences.chunked,onchange:move |e|act(app,|s|{s.preferences.chunked=e.checked();s.save_preferences()})}label {r#for:"show-chunks","Break lesson phrases into short chunks"}}
            p {class:"quiet","Choose the size of your next short session on Today. It mixes returns across skills you’ve practiced, with a quiet option for reading and writing. Every practice stays untimed; Teacher exercises keep their original question and skill."}
        }
        review_timing_view::ReviewTimingSettings {app}
        workspace_view::SavedWindows {}
        div {class:"cards two",
            section {class:"panel",h2 {"Connect your devices"}p {"Passages, practice, and teacher feedback sync through your private relay. Choose separately whether to transfer recordings; model API credentials stay on the relay."}
                label {r#for:"sync-url","Sync URL"}input {id:"sync-url",value:state.endpoint,oninput:move |e|app.write().endpoint=e.value(),placeholder:"https://your-sync-host"}
                div {class:"preference-row",input {id:"sync-recordings",r#type:"checkbox",checked:state.preferences.sync_recordings,disabled:state.syncing,onchange:move |e|act(app,|s|{s.preferences.sync_recordings=e.checked();s.save_preferences()})}label {r#for:"sync-recordings","Include original recordings and timed cues"}}
                label {r#for:"sync-key","Private connection key"}input {id:"sync-key",r#type:"password",autocomplete:"off",value:state.token,oninput:move |e|app.write().token=e.value()}
                if cfg!(target_arch="wasm32") {p {class:"quiet","This browser keeps the key in memory until you close Drussky."}}else{p {class:"quiet","The installed connection stays in this device’s Keychain. Disconnect removes it."}}
                div {class:"actions",button {class:"primary",disabled:state.syncing,onclick:move |_|{
                    let snapshot=app.read().clone();app.write().syncing=true;
                    spawn_forever(async move {let result=async {synchronize(snapshot.repo.clone(),snapshot.endpoint.clone(),snapshot.token.clone()).await?;
                        if snapshot.preferences.sync_recordings {asset_sync::synchronize_audio(snapshot.repo,snapshot.endpoint,snapshot.token).await.map(Some).map_err(|e|format!("Passages and practice synced; recording sync stopped: {e}"))} else {Ok(None)}}.await;app.write().syncing=false;
                        match result {Ok(count)=>act(app,|s|{s.refresh()?;let recovered=s.reconcile_dialogue()?;if !recovered.diagnostics.is_empty(){s.notice=recovered.diagnostics.join(" ");return Ok(());}s.notice=count.map_or("Your notebook is up to date with the sync library.".into(),|n|format!("Notebook synced. {n} recordings transferred."));Ok(())}),Err(e)=>app.write().notice=e}
                    });
                },if state.syncing {"Syncing…"}else{"Sync now"}}
                button {class:"text-button",disabled:state.syncing,onclick:move |_|act(app,|s|{s.repo.disconnect()?;
                    #[cfg(not(target_arch="wasm32"))]
                    crate::native_connection::forget()?;
                    s.token.clear();s.notice="Disconnected. Your local passages and practice are kept.".into();Ok(())}),"Disconnect"}}
            }
            section {class:"panel",h2 {"A portable copy"}p {"Export your passages and practice records as JSON, or restore a previous export. Restoring merges records and keeps competing edits."}
                div {class:"actions",button {class:"secondary",onclick:move |_|act(app,|s|{s.transfer=repository::export_library(s.repo.library()?)?;Ok(())}),"Prepare export"}
                button {class:"secondary",onclick:move |_|act(app,|s|{let library=repository::import_library(&s.transfer)?;
                    if library.events.iter().any(|(id,e)|*id!=e.id){return Err("Backup event IDs do not match".into());}
                    s.repo.append(&library.events.into_values().collect::<Vec<_>>())?;s.refresh()?;let recovered=s.reconcile_dialogue()?;if !recovered.diagnostics.is_empty(){s.notice=recovered.diagnostics.join(" ");return Ok(());}s.notice="Backup merged into your notebook.".into();Ok(())}),"Restore JSON"}}
                label {r#for:"backup-json","Copy this text to a file, or paste a backup to restore"}textarea {id:"backup-json",class:"backup",value:state.transfer,oninput:move |e|app.write().transfer=e.value(),placeholder:"Your portable library…"}
            }
        }
    }
}

#[component]
fn Studio(app: Signal<AppState>) -> Element {
    let state = app.read().clone();
    let revision_id = state
        .studio_revision
        .or_else(|| state.session.as_ref().map(|s| s.revision));
    let revision = revision_id.and_then(|id| state.library.revision(id));
    rsx! {
        div {class:"page-heading",div {div {class:"eyebrow","LISTEN · NOTICE · TRY AGAIN"}h1 {"Find your Russian voice"}p {"Give a phrase a voice and keep your own attempts. Listen first; use the instruments to investigate a particular question."}}}
        if let Some(r)=revision {p {class:"quiet","Linked to: {r.passage.title}"}}
        AudioStudio {passage_id:revision.map(|r|r.passage.id.to_string()).unwrap_or_default(),revision_id:revision_id.map(|id|id.to_string()).unwrap_or_default(),russian:revision.map(|r|r.passage.russian.clone()).unwrap_or_default(),workspace_id:if cfg!(target_arch="wasm32") {String::new()}else{state.device.to_string()},on_region_played:move |played|act(app,|s|s.heard_region(played)),on_saved:move |record|act(app,|s|recording_saved(s,record)),generating_reference:state.speech_busy,on_generate_reference:move |_|{if let Some(id)=revision_id {generate_reference(app,id)}else{act(app,|s|{s.notice="Choose a passage or lesson phrase to give it a voice.".into();s.navigate(Page::Library)})}}}
        if let Some(id)=revision_id {section {class:"panel",
            h2 {"Work out how to say it"}p {"Ask your teacher to write numbers as spoken words, explain an ambiguous word, or mark its stress."}
            button {class:"secondary",onclick:move |_|act(app,|s|prepare_spoken_wording_question(s,id)),"Ask how to say this"}
        }}
    }
}
fn prepare_spoken_wording_question(state: &mut AppState, revision: uuid::Uuid) -> Result<()> {
    use rufren_tutor::{DialogueAction, DialogueConfig, DialogueIntent};
    let source = state
        .library
        .revision(revision)
        .ok_or("Open or sync this source first.")?
        .passage
        .russian
        .clone();
    let question = format!(
        "Help me say this Russian source aloud. Keep its meaning and explain any ambiguity. Give a separate short spoken wording with numbers written as correctly inflected Russian words and stress marks on ambiguous words. Explain your choices in English. Keep the original source unchanged.\n\nSource:\n{source}"
    );
    if question.len() > rufren_tutor::DIALOGUE_MAX_TEXT_BYTES {
        return Err(
            "Choose a shorter source phrase for this question. Your passage is kept.".into(),
        );
    }
    teacher_dialogue_view::show_conversations(state)?;
    let mut draft = state.new_dialogue(
        DialogueConfig {
            intent: DialogueIntent::Clarification,
            topic: "How to say this source aloud".into(),
            teacher_role: None,
            learner_role: None,
        },
        DialogueAction::Ask,
        None,
    )?;
    state.save_dialogue_draft(&mut draft, question)?;
    Ok(())
}
fn generate_reference(mut app: Signal<AppState>, revision: uuid::Uuid) {
    if app.read().speech_busy {
        return;
    }
    let snapshot = app.read().clone();
    let Some(passage) = snapshot
        .library
        .revision(revision)
        .map(|r| r.passage.clone())
    else {
        app.write().notice = "This passage is unavailable. Open or sync it first.".into();
        return;
    };
    let text = snapshot
        .content
        .phrases
        .iter()
        .find(|p| snapshot.phrase_revision(p) == revision)
        .map(|p| p.stressed.clone())
        .or_else(|| {
            snapshot
                .plan
                .as_ref()
                .filter(|p| {
                    snapshot
                        .session
                        .as_ref()
                        .is_some_and(|s| s.id == p.session && s.revision == revision)
                })
                .and_then(|p| p.snapshot.as_ref())
                .and_then(|s| {
                    s.phrases
                        .iter()
                        .find(|p| p.id == s.exercise.reference_phrase_id)
                })
                .map(|p| p.stressed.clone())
        })
        .unwrap_or(passage.russian);
    if text.chars().count() > 256 || text.len() > 1024 {
        app.write().notice =
            "Give one short phrase a voice at a time. Split this passage into smaller phrases."
                .into();
        return;
    }
    let request_id = uuid::Uuid::new_v4().to_string();
    let revision_id = revision.to_string();
    let body = serde_json::json!({"text":text,"request_id":request_id,"revision_id":revision_id});
    app.write().speech_busy = true;
    dioxus_core::spawn_forever(async move {
        let result = async {
            let response = gateway::request::<serde_json::Value>(
                &snapshot.endpoint,
                &snapshot.token,
                "/api/speech/generate",
                Some(&body),
                4 * 1024 * 1024,
                90_000,
            )
            .await?;
            audio_studio::save_generated_reference(
                &passage.id.to_string(),
                &revision_id,
                &text,
                &request_id,
                response,
            )
            .await
        }
        .await;
        app.write().speech_busy = false;
        match result {
            Ok(record) => act(app, |s| {
                recording_saved(s, record)?;
                s.notice =
                    "Your spoken reference is ready. Listen, then try the meaning yourself.".into();
                Ok(())
            }),
            Err(problem) => app.write().notice = problem,
        }
    });
}
fn recording_saved(s: &mut AppState, record: StudioRecording) -> Result<()> {
    let Ok(revision) = uuid::Uuid::parse_str(&record.revision_id) else {
        s.notice = "Recording kept in your local speech studio.".into();
        return Ok(());
    };
    let id = register_recording(s, &record)?;
    if record.role == "attempt"
        && !record.interrupted
        && record.device.get("sound_focus_scope").is_none()
        && record.device.get("dialogue_scope").is_none()
        && s.session.as_ref().is_some_and(|session| {
            session.revision == revision
                && session.skill == Skill::Speaking
                && session.phase == Phase::Retrieve
        })
    {
        let Some(Fact::RecordingAttached(attachment)) = s.library.events.get(&id).map(|e| &e.fact)
        else {
            return Err("The saved take could not be linked. Select it again.".into());
        };
        let attachment = attachment.clone();
        s.update_response(format!(
            "Recorded response · {:.1} seconds",
            record.duration_ms as f64 / 1000.0
        ))?;
        if let Some(mut plan) = s.plan.clone() {
            plan.response_recording = Some(attachment);
            s.repo.set("session_plan", &plan)?;
            s.plan = Some(plan);
        }
    }
    s.notice="Recording saved with its passage. The original audio stays on this device until you choose to sync recordings.".into();
    Ok(())
}

/// Register one exact durable original without changing any active practice.
fn register_recording(s: &mut AppState, record: &StudioRecording) -> Result<uuid::Uuid> {
    let revision = uuid::Uuid::parse_str(&record.revision_id).map_err(|e| e.to_string())?;
    if s.library.revision(revision).is_none() {
        return Err(
            "Recording saved locally; its passage is not available for linking yet.".into(),
        );
    }
    let attachment = AudioAttachment {
        asset_id: record.asset_id.clone(),
        asset: AudioAsset {
            sha256: record.content_hash.clone(),
            media_type: record.mime_type.clone(),
            sample_rate_hz: record.sample_rate,
            channels: record.channels,
            frames: record.frames,
        },
        target: AnnotationTarget::Revision(revision),
        role: if record.role == "reference" {
            AudioRole::Reference
        } else {
            AudioRole::Attempt
        },
        note: if record.interrupted {
            "Recording interrupted; captured portion preserved".into()
        } else {
            String::new()
        },
    };
    let mut event = rufren_core::Event::new(
        s.device,
        record.created_at_ms,
        Fact::RecordingAttached(attachment.clone()),
    )
    .map_err(|e| e.to_string())?;
    // Re-linking the same immutable original remains idempotent across retries.
    event.id = uuid::Uuid::new_v5(
        &revision,
        format!("audio:{}:{}", record.asset_id, record.role).as_bytes(),
    );
    event.device = uuid::Uuid::new_v5(&revision, b"audio-attachment-writer");
    let id = event.id;
    s.repo.append(&[event])?;
    s.refresh()?;
    Ok(id)
}
