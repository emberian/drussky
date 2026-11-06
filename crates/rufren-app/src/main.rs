mod model;
mod repository;
use dioxus::prelude::*;
use model::*;
use repository::{Result, now_ms, synchronize};
use rufren_core::*;

const STYLE: &str = include_str!("style.css");
fn main() {
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
    let initial = use_hook(AppState::open);
    match initial {
        Ok(state) => rsx! {Shell {initial:state}},
        Err(error) => rsx! {
            style {"{STYLE}"} main {class:"fatal",h1 {"Your notebook could not be opened."} p {"{error}"} p {"Your saved data has not been replaced. Restore access to storage and reopen Rufren."}}
        },
    }
}
#[component]
fn Shell(initial: AppState) -> Element {
    let mut app = use_signal(|| initial);
    let state = app.read().clone();
    rsx! {
        style {"{STYLE}"}
        div {class:"shell",
            aside {class:"sidebar",
                a {class:"brand",href:"#",onclick:move |_|app.write().page=Page::Today, span {class:"brand-mark","р"} div {strong {"rufren"} small {"a world in another language"}}}
                nav {aria_label:"Main navigation",for page in Page::ALL {button {class:if state.page==page {"nav-item active"}else{"nav-item"},onclick:move |_|{app.write().page=page;act(app,|s|s.refresh());},span {"{page.icon()}"} "{page.label()}"}}}
                div {class:"sidebar-bottom",span {class:"status-dot"} "On this device" p {"Your pace. Your Russian."}}
            }
            main {class:"main",
                header {class:"topbar",span {"YOUR RUSSIAN NOTEBOOK"} span {class:"quiet","{state.page.label()}"}}
                if !state.notice.is_empty() {div {class:"notice",role:"status",span {"{state.notice}"} button {aria_label:"Dismiss message",onclick:move |_|app.write().notice.clear(),"×"}}}
                match state.page {
                    Page::Today=>rsx!{Today {app}},Page::Library=>rsx!{LibraryView {app}},Page::Practice=>rsx!{Practice {app}},
                    Page::Studio=>rsx!{Studio {}},Page::Progress=>rsx!{Progress {app}},Page::Settings=>rsx!{Settings {app}},
                }
            }
        }
    }
}
#[component]
fn Today(mut app: Signal<AppState>) -> Element {
    let state = app.read().clone();
    let count = state.library.heads().len();
    let attempts = state.library.attempts().count();
    let resume = state
        .session
        .as_ref()
        .is_some_and(|s| s.phase != Phase::Complete);
    rsx! {
        section {class:"hero",div {class:"eyebrow","BEGIN WITH CURIOSITY"} h1 {"A little Russian." br {} em {"A bigger world."}}
            p {class:"lede","A conversation with a friend. A sentence from another world. Start with something you want to understand."}
            div {class:"actions",button {class:"primary",onclick:move |_|act(app,|s|s.begin()),if resume {"Continue your practice →"}else if count==0 {"Begin with a small story →"}else{"Practice from your library →"}}
            button {class:"text-button",onclick:move |_|{app.write().editor=Editor::default();app.write().page=Page::Library;},"Add a passage"}}
            div {class:"orbital",aria_hidden:"true",div {class:"orbit orbit-one"} div {class:"orbit orbit-two"} span {class:"star star-one","✦"} span {class:"star star-two","·"} span {class:"planet"}}
        }
        div {class:"section-heading",h2 {"Make it yours"}span {"Small steps, connected."}}
        div {class:"cards three",
            button {class:"feature-card",onclick:move |_|app.write().page=Page::Library,span {class:"card-number","01 / COLLECT"}h3 {"Words worth keeping"}p {"Bring a phrase from your coach, a story, or a question you want to ask."}span {class:"card-link","Open your library ↗"}}
            button {class:"feature-card",onclick:move |_|app.write().page=Page::Studio,span {class:"card-number","02 / LISTEN CLOSELY"}h3 {"Find your Russian voice"}p {"A place for reference audio, your recordings, and focused feedback."}span {class:"card-link","Explore the speech studio ↗"}}
            button {class:"feature-card",onclick:move |_|app.write().page=Page::Progress,span {class:"card-number","03 / RETURN"}h3 {"See what stays with you"}p {"Keep reading, writing, listening, and speaking in view as you practice."}span {class:"card-link","View your practice ↗"}}
        }
        div {class:"quiet-summary",span {"{count} passages in your notebook"}span {"{attempts} saved practice attempts"}}
    }
}
#[component]
fn LibraryView(mut app: Signal<AppState>) -> Element {
    let state = app.read().clone();
    let d = state.editor.clone();
    let heads = state.library.heads();
    let conflicts =
        d.id.and_then(|id| heads.get(&id))
            .is_some_and(|v| v.len() > 1);
    rsx! {
        div {class:"page-heading",div {div {class:"eyebrow","YOUR COLLECTION"}h1 {"A library of possibilities"}p {"Keep the original, its meaning, and where it came from."}}button {class:"secondary",onclick:move |_|app.write().editor=Editor::default(),"New passage"}}
        div {class:"library-layout",
            section {class:"passage-list",aria_label:"Saved passages",
                if heads.is_empty() {div {class:"empty",h3 {"Your first page is waiting."}p {"Add a few words from your coach, or start with our small story."}button {class:"text-button",onclick:move |_|act(app,|s|s.begin()),"Try a starter passage →"}}}
                for revisions in heads.values() {for id in revisions {if let Some(r)=state.library.revision(*id) {
                    {let revision=*id;rsx!{button {class:if d.parent==Some(revision) {"passage-card selected"}else{"passage-card"},onclick:move |_|app.write().edit(revision),
                        span {class:"tag",if revisions.len()>1 {"Competing revision"}else{"RUSSIAN"}}h3 {"{r.passage.title}"}p {lang:"ru","{r.passage.russian}"}small {"{r.passage.source}"}
                    }}}
                }}}
            }
            section {class:"panel editor",h2 {if d.id.is_some() {"Your passage"}else{"Add something you love"}}
                label {r#for:"passage-title","Title"}input {id:"passage-title",value:d.title,oninput:move |e|app.write().editor.title=e.value(),placeholder:"A line worth learning"}
                label {r#for:"passage-russian","Russian text"}textarea {id:"passage-russian",lang:"ru",class:"russian-input",value:d.russian,oninput:move |e|app.write().editor.russian=e.value(),placeholder:"Русский текст…"}
                label {r#for:"passage-meaning","Meaning or translation"}textarea {id:"passage-meaning",value:d.translation,oninput:move |e|app.write().editor.translation=e.value(),placeholder:"What does it mean in this context?"}
                label {r#for:"passage-source","Source or coach note"}input {id:"passage-source",value:d.source,oninput:move |e|app.write().editor.source=e.value(),placeholder:"Book, author, conversation, or your own words"}
                if conflicts {div {class:"notice",p {"This passage was edited in more than one place. Both versions are kept. Select a version to inspect it; resolve when this editor contains the wording you want."}}}
                div {class:"actions",button {class:"primary",onclick:move |_|act(app,|s|s.save_passage(false)),"Save passage"}
                    if conflicts {button {class:"secondary",onclick:move |_|act(app,|s|s.save_passage(true)),"Resolve with this wording"}}
                }
                if let Some(revision)=d.parent {if !conflicts {div {class:"practice-options",p {"Practice the saved version"}button {class:"secondary",onclick:move |_|act(app,|s|s.start(revision,Skill::Reading)),"Read for meaning"}button {class:"secondary",onclick:move |_|act(app,|s|s.start(revision,Skill::Writing)),"Write from memory"}}}}
            }
        }
    }
}
#[component]
fn Practice(mut app: Signal<AppState>) -> Element {
    let state = app.read().clone();
    let Some(session) = state.session.clone() else {
        return rsx! {div {class:"empty large",div {class:"eyebrow","ONE SMALL STEP"}h1 {"Meet a little Russian."}p {"Choose a passage and make it familiar, at your own pace."}button {class:"primary",onclick:move |_|act(app,|s|s.begin()),"Choose my next passage →"}}};
    };
    let Some(revision) = state.library.revision(session.revision) else {
        return rsx! {p {"This practice refers to a passage not yet available on this device. Sync or restore its backup."}};
    };
    let p = &revision.passage;
    let writing = session.skill == Skill::Writing;
    rsx! {
        div {class:"page-heading",div {div {class:"eyebrow","{session.skill.label()} · AT YOUR PACE"}h1 {"{p.title}"}p {"{p.source}"}}}
        ol {class:"steps",for (i,(phase,label)) in [(Phase::Encounter,"Meet"),(Phase::Retrieve,"Try"),(Phase::Reflect,"Reflect"),(Phase::Complete,"Keep")].into_iter().enumerate() {
            li {class:if session.phase==phase {"current"}else{""},span {"{i+1}"} "{label}"}
        }}
        section {class:"panel practice-panel",
            match session.phase {
                Phase::Encounter=>rsx!{
                    span {class:"tag","READ AND CONNECT"}p {class:"russian-display",lang:"ru","{p.russian}"}p {class:"translation","{p.translation}"}
                    p {class:"quiet","Take your time. Next, you’ll try " if writing {"writing the Russian"} else {"explaining the meaning"} " without the answer beside you."}
                    button {class:"primary",onclick:move |_|act(app,|s|s.phase(Phase::Retrieve)),"I'm ready to try →"}
                },
                Phase::Retrieve=>rsx!{
                    span {class:"tag",if writing {"WRITE THE RUSSIAN"}else{"READ FOR MEANING"}}
                    p {class:if writing {"prompt"}else{"russian-display"},if writing {"{p.translation}"}else{"{p.russian}"}}
                    label {r#for:"practice-response",if writing {"Your words in Russian"}else{"What does this mean?"}}
                    textarea {id:"practice-response",class:"response",value:session.draft.clone(),oninput:move |e|act(app,|s|{if let Some(session)=s.session.as_mut(){session.draft=e.value();}s.save_session()}),placeholder:"Take a moment. There’s no timer."}
                    if session.cues.is_empty() {button {class:"text-button",onclick:move |_|act(app,|s|s.hint()),"Give me a hint"}}
                    else {div {class:"hint",span {class:"tag","A LITTLE SUPPORT"}p {if writing {"{p.russian}"}else{"{p.translation}"}}}}
                    div {class:"actions",button {class:"primary",disabled:session.draft.trim().is_empty(),onclick:move |_|act(app,|s|s.phase(Phase::Reflect)),"Compare my response →"}}
                },
                Phase::Reflect=>rsx!{
                    span {class:"tag","YOUR RESPONSE"}p {class:"prompt","{session.draft}"}hr {}p {class:"russian-display",lang:"ru","{p.russian}"}p {class:"translation","{p.translation}"}
                    h2 {"How did that feel?"}p {class:"quiet","Your reflection is saved alongside the hints you used. A coach can help with correctness."}
                    div {class:"rating-buttons",button {class:"secondary",onclick:move |_|act(app,|s|s.record(SelfRating::Again)),"I'd like another look"}button {class:"secondary",onclick:move |_|act(app,|s|s.record(SelfRating::WithHelp)),"I needed some help"}button {class:"primary",onclick:move |_|act(app,|s|s.record(SelfRating::Recalled)),"I recalled it"}}
                },
                Phase::Complete=>rsx!{
                    div {class:"completion-mark","✦"}h2 {"A little more familiar."}p {"Your attempt is saved. Return to the passage, or let it rest for now."}
                    div {class:"actions",button {class:"primary",onclick:move |_|{app.write().page=Page::Today;},"Back to today"}button {class:"secondary",onclick:move |_|act(app,|s|s.begin()),"Practice another passage"}}
                },
            }
        }
    }
}
#[component]
fn Studio() -> Element {
    rsx! {
        div {class:"page-heading",div {div {class:"eyebrow","LISTEN · NOTICE · TRY AGAIN"}h1 {"A place for your voice"}p {"Build a conversation between what you hear and what you say."}}}
        section {class:"panel studio",div {class:"wave",aria_hidden:"true",for height in [18,32,52,28,70,92,48,78,40,62,90,54,34,72,44,24,38] {span {style:"height:{height}px"}}}
            h2 {"Your recording space is taking shape."}p {"Recording and playback will connect your coach’s examples with your own attempts. They aren’t available in this foundation build yet."}
            div {class:"cards three",div {class:"mini-card",h3 {"Reference"}p {"A natural example, with context."}}div {class:"mini-card",h3 {"Your attempt"}p {"Listen side by side, at your pace."}}div {class:"mini-card",h3 {"One useful cue"}p {"A focused correction to try next."}}}
        }
    }
}
#[component]
fn Progress(app: Signal<AppState>) -> Element {
    let state = app.read().clone();
    let mut attempts: Vec<_> = state.library.attempts().collect();
    attempts.sort_by_key(|(event, _)| (event.recorded_at_ms, event.id));
    let independent = attempts
        .iter()
        .filter(|(_, a)| a.cues.is_empty() && a.self_rating == SelfRating::Recalled)
        .count();
    rsx! {
        div {class:"page-heading",div {div {class:"eyebrow","WHAT YOU'VE PRACTICED"}h1 {"Four ways into a language"}p {"A record of your practice, with room to grow in every direction."}}}
        div {class:"cards four",for skill in Skill::ALL {div {class:"panel stat",span {class:"tag","{skill.label()}"}strong {"{attempts.iter().filter(|(_,a)|a.skill==skill).count()}"}p {"saved attempts"}}}}
        section {class:"panel",h2 {"Remembering with and without support"}p {"Marked ‘I recalled it’ without a recorded hint: {independent}."}p {class:"quiet","These are your reflections, not proficiency scores. Listening and speaking will appear here when audio practice is connected."}}
        section {class:"panel",h2 {"Recent practice"}
            if attempts.is_empty() {p {class:"quiet","Your first practice will start this story."}}
            for (event,attempt) in attempts.iter().rev().take(12) {div {class:"history-row",span {"{attempt.skill.label()}"}span {{state.library.revision(attempt.revision).map(|r|r.passage.title.as_str()).unwrap_or("Passage not available")}}small {if attempt.cues.is_empty(){"No hint"}else{"With a hint"} " · {relative_day(event.recorded_at_ms)}"}}}
        }
    }
}
fn relative_day(timestamp: i64) -> String {
    let days = now_ms().saturating_sub(timestamp).max(0) / 86_400_000;
    match days {
        0 => "Within the last day".into(),
        1 => "A day ago".into(),
        n => format!("{n} days ago"),
    }
}
#[component]
fn Settings(mut app: Signal<AppState>) -> Element {
    let state = app.read().clone();
    rsx! {
        div {class:"page-heading",div {div {class:"eyebrow","YOUR NOTEBOOK, YOUR DEVICES"}h1 {"Keep your Russian with you"}p {"Practice saves locally. Connect your own sync library when you’re ready."}}}
        div {class:"cards two",
            section {class:"panel",h2 {"Connect your devices"}p {"Passages and practice records can sync through your private relay. Audio and API credentials are not included."}
                label {r#for:"sync-url","Sync URL"}input {id:"sync-url",value:state.endpoint,oninput:move |e|app.write().endpoint=e.value(),placeholder:"https://your-sync-host"}
                label {r#for:"sync-key","Sync key · kept only for this session"}input {id:"sync-key",r#type:"password",autocomplete:"off",value:state.token,oninput:move |e|app.write().token=e.value()}
                div {class:"actions",button {class:"primary",disabled:state.syncing,onclick:move |_|{
                    let snapshot=app.read().clone();app.write().syncing=true;
                    spawn(async move {let result=synchronize(snapshot.repo,snapshot.endpoint,snapshot.token).await;app.write().syncing=false;
                        match result {Ok(_)=>act(app,|s|{s.refresh()?;s.notice="Your notebook is up to date with the sync library.".into();Ok(())}),Err(e)=>app.write().notice=e}
                    });
                },if state.syncing {"Syncing…"}else{"Sync now"}}
                button {class:"text-button",disabled:state.syncing,onclick:move |_|act(app,|s|{s.repo.disconnect()?;s.token.clear();s.notice="Disconnected. Your local passages and practice are kept.".into();Ok(())}),"Disconnect"}}
            }
            section {class:"panel",h2 {"A portable copy"}p {"Export your passages and practice records as JSON, or restore a previous export. Restoring merges records and keeps competing edits."}
                div {class:"actions",button {class:"secondary",onclick:move |_|act(app,|s|{s.transfer=serde_json::to_string_pretty(&s.repo.library()?).map_err(|e|e.to_string())?;Ok(())}),"Prepare export"}
                button {class:"secondary",onclick:move |_|act(app,|s|{let library:Library=serde_json::from_str(&s.transfer).map_err(|e|format!("Could not read backup: {e}"))?;
                    if library.events.iter().any(|(id,e)|*id!=e.id){return Err("Backup event IDs do not match".into());}
                    s.repo.append(&library.events.into_values().collect::<Vec<_>>())?;s.refresh()?;s.notice="Backup merged into your notebook.".into();Ok(())}),"Restore JSON"}}
                label {r#for:"backup-json","Copy this text to a file, or paste a backup to restore"}textarea {id:"backup-json",class:"backup",value:state.transfer,oninput:move |e|app.write().transfer=e.value(),placeholder:"Your portable library…"}
            }
        }
    }
}
