//! A conversation keeps its source and branch; it is not a graded practice attempt.
use crate::{
    act,
    exposure_view::PresentedRussian,
    gateway,
    model::{AppState, Page},
    repository::Result,
    teacher_dialogue::DialogueDraft,
    teacher_prose::TeacherProse,
};
use dioxus::prelude::*;
use rufren_core::*;
use rufren_tutor::*;
use uuid::Uuid;

const STYLE: &str = include_str!("teacher_dialogue_view.css");

pub(crate) fn show_conversations(state: &mut AppState) -> Result<()> {
    state.navigate(Page::Teacher)?;
    state.repo.set("teacher_surface", &"dialogues")
}

fn focus_composer() {
    spawn(async move {
        let _=document::eval("await new Promise(requestAnimationFrame); const target=document.getElementById('dialogue-message') || document.querySelector('.teacher-dialogue h2'); if(target){if(target.tagName==='H2')target.setAttribute('tabindex','-1');target.focus({preventScroll:true});target.scrollIntoView({block:'nearest'});}").await;
    });
}

#[component]
pub(crate) fn AskAboutThis(
    app: Signal<AppState>,
    document: Uuid,
    focus: TeacherDialogueFocus,
    #[props(default)] disabled: bool,
) -> Element {
    rsx! {button {class:"text-button ask-about-this",disabled,
    "data-dialogue-anchor":document.to_string(),onclick:move |_|act(app,|s| {
        // Navigation persists any active return's assistance before teaching appears.
        show_conversations(s)?;
        let title=s.library.teacher_document(document).map(|d|match &d.response.request {
            TeacherRequest::TeachLesson(r)=>r.title.clone(),
            TeacherRequest::AnswerFeedback(_)=>"A question about this feedback".into(),
        }).ok_or("The saved source is unavailable. Sync it before opening this conversation.")?;
        s.new_anchored_dialogue(document,focus.clone(),DialogueConfig {
            intent:DialogueIntent::Clarification,topic:title,teacher_role:None,learner_role:None,
        },DialogueAction::Ask)?;
        focus_composer();
        Ok(())
    }),"Ask about this"}}
}

#[component]
pub(crate) fn TeacherDialogues(mut app: Signal<AppState>) -> Element {
    let state = app.read().clone();
    let active = state.active_dialogue_draft();
    let drafts = state.dialogue_drafts();
    let threads = state.library.dialogue_threads();
    let endpoint = state.endpoint.clone();
    let token = state.token.clone();
    let availability = use_resource(move || {
        let endpoint = endpoint.clone();
        let token = token.clone();
        async move {
            gateway::request::<TutorStatus>(
                &endpoint,
                &token,
                "/api/teacher/status",
                None,
                16 * 1024,
                15_000,
            )
            .await
        }
    });
    let status = availability.read().clone();
    let ready = status.as_ref().is_some_and(|r| {
        r.as_ref()
            .is_ok_and(|s| s.enabled && s.dialogue == Some(DialogueCapability::ThreadedV1))
    });
    let initial_scene=state.repo.get::<String>("teacher_dialogue_new_scene").ok().flatten().unwrap_or_else(||"An AI aboard a quiet starship welcomes me and offers tea. Begin with a simple greeting; I am a complete beginner.".into());
    let mut scene = use_signal(|| initial_scene);
    let have_active = matches!(&active, Ok(Some(_)));
    rsx! {
        style {"{STYLE}"}
        section {class:"dialogue-workspace",aria_label:"Conversations with your teacher",
            details {class:"panel dialogue-library",open:!have_active,
                summary {"Conversations & new questions"}
                p {"Ask in English or Russian, work through a point together, or try a short scene. Your lesson and unfinished answer stay where you left them."}
                div {class:"actions",
                    button {class:"primary",onclick:move |_|act(app,|s| {
                        s.new_dialogue(DialogueConfig {intent:DialogueIntent::Clarification,topic:"A question about Russian".into(),teacher_role:None,learner_role:None},DialogueAction::Ask,None)?;focus_composer();Ok(())
                    }),"New question"}
                }
                details {class:"dialogue-scene-setup",summary {"Try a conversation"}
                    label {r#for:"dialogue-scene","What is happening in the scene?"}
                    textarea {id:"dialogue-scene",value:scene(),maxlength:1024,oninput:move |e| {
                        let value=e.value();scene.set(value.clone());act(app,|s|s.repo.set("teacher_dialogue_new_scene",&value));
                    }}
                    p {class:"quiet","Your AI partner will begin with a short Russian turn. You can ask for a meaning or a way to say something whenever you need it."}
                    button {class:"secondary",disabled:scene().trim().is_empty(),onclick:move |_|act(app,|s| {
                        s.new_dialogue(DialogueConfig {intent:DialogueIntent::Roleplay,topic:scene(),teacher_role:Some("A patient Russian-speaking conversation partner".into()),learner_role:Some("A complete beginner who can ask for English help".into())},DialogueAction::StartScene,None)?;focus_composer();Ok(())
                    }),"Prepare this scene"}
                }
                if let Ok(drafts)=&drafts {for draft in drafts.iter().filter(|d|d.committed_request_id.is_none()) {
                    {let id=draft.id;rsx! {button {class:"dialogue-thread-choice",onclick:move |_|act(app,|s|{s.select_dialogue_draft(id)?;Ok(())}),strong {"{draft.config.topic}"}small {"Unfinished message on this device"}}}}
                }}
                for thread in &threads {
                    {let title=state.library.dialogue_user(thread.thread_id).map(|r|r.config.topic.clone()).unwrap_or_else(||"Conversation awaiting its source".into());
                    rsx! {section {class:"dialogue-thread",h3 {"{title}"}
                        for (index,leaf) in thread.leaf_ids.iter().enumerate() {
                            {let id=*leaf;let is_reply=state.library.dialogue_reply(id).is_some();rsx! {button {class:"secondary","data-open-dialogue-leaf":id.to_string(),onclick:move |_|act(app,|s|{if is_reply{s.open_dialogue_reply(id)?;}else{s.open_dialogue_user(id)?;}Ok(())}),if thread.leaf_ids.len()==1 {"Open conversation"}else{{format!("Open branch {}",index+1)}}}}}
                        }
                        if thread.incomplete {p {class:"quiet","Some source messages have not arrived. Sync to recover the complete conversation."}}
                    }}}
                }
            }
            match active {
                Err(problem)=>rsx!{p {role:"alert","{problem}"}},
                Ok(None)=>rsx!{p {class:"quiet","Start with a question, or open a conversation you have already begun."}},
                Ok(Some(draft))=>rsx!{for id in [draft.thread_id] {DialogueThreadView {key:"dialogue-thread-{id}",app,draft:draft.clone(),ready}}},
            }
            if !ready {p {class:"quiet",role:"status",match status {None=>"Checking your teacher connection…".to_string(),Some(Ok(_))=>"Your questions can stay saved here. Connect a teacher with conversation support when you are ready to send them.".to_string(),Some(Err(problem))=>format!("Your saved conversation is available offline. {problem}")}}}
        }
    }
}

#[component]
fn DialogueThreadView(app: Signal<AppState>, draft: DialogueDraft, ready: bool) -> Element {
    let state = app.read().clone();
    let leaf = state.dialogue_selected_leaf().ok().flatten();
    let path = leaf.and_then(|id| state.library.dialogue_path(id).ok());
    let outbox = state.dialogue_outbox().unwrap_or_default();
    rsx! {
        section {class:"teacher-dialogue","data-dialogue-thread":draft.thread_id.to_string(),"data-dialogue-leaf":leaf.map(|v|v.to_string()).unwrap_or_default(),
            div {class:"dialogue-heading",h2 {if draft.config.intent==DialogueIntent::Roleplay {"Conversation"} else {"{draft.config.topic}"}}
                if let Some(anchor)=&draft.anchor {
                    {let source=anchor.clone();let reader=matches!(&source.focus,TeacherDialogueFocus::ReaderWhole|TeacherDialogueFocus::ReaderChunk{..});rsx! {button {class:"text-button",onclick:move |_|act(app,|s|{s.return_to_dialogue_source(&source)?;crate::focus_page_heading();Ok(())}),if reader {"Return to the passage"}else{"Return to the source lesson or feedback"}}}}
                }
            }
            if draft.config.intent==DialogueIntent::Roleplay {
                details {class:"dialogue-scene-context",summary {"Your scene"}
                    p {"{draft.config.topic}"}
                }
            }
            if let Some(anchor)=&draft.anchor {
                details {class:"dialogue-source","data-dialogue-source-document":anchor.document_id.to_string(),summary {"Source for this conversation"}
                    TeacherProse {app,language:"en",text:anchor.snapshot.clone(),purpose:PresentationPurpose::Reference}
                }
            }
            if let Some(path)=path {
                if !path.is_complete() {p {role:"alert","This branch is missing source material. Its messages are kept; sync before continuing it."}}
                div {class:"dialogue-messages",for message in &path.messages {DialogueMessageView {key:"message-{message.id()}",app,id:message.id()}}}
            }
            if let Some(user)=draft.committed_request_id {
                if let Some(pending)=outbox.iter().find(|p|p.request_id==user) {
                    if let Some(problem)=&pending.problem {p {role:"alert","{problem}"}}
                    p {class:"quiet","Your message is saved. A retry uses that same message; if an earlier reply was lost in transit, another reply may be generated."}
                    button {class:"primary",disabled:state.teacher_busy||!ready,onclick:move |_|send_dialogue(app,user),if state.teacher_busy {"Your teacher is thinking…"}else{"Retry reply"}}
                }
                p {class:"quiet","Open a saved reply to continue from it. Different replies and branches remain separate."}
            } else {
                for id in [draft.id] {DialogueComposer {key:"composer-{id}",app,initial:draft.clone(),ready}}
            }
        }
    }
}

#[component]
fn DialogueMessageView(app: Signal<AppState>, id: Uuid) -> Element {
    let state = app.read().clone();
    if let Some(user) = state.library.dialogue_user(id) {
        return rsx! {article {class:if user.action==DialogueAction::StartScene {"dialogue-message dialogue-scene-start"} else {"dialogue-message learner-message"},"data-message-id":id.to_string(),"data-dialogue-role":"user",
            if user.action==DialogueAction::StartScene {p {class:"quiet","Scene started"}}else{p {class:"dialogue-speaker","You"}PresentedRussian {app,text:user.text.clone(),language:"en",purpose:PresentationPurpose::Reference,p {class:"dialogue-literal","{user.text}"}}}
            if user.confirmed_audio.is_some(){crate::dialogue_audio_view::DialogueUserAudio {app,input_id:user.request_id}}
            if user.omitted_count>0 {details {summary {"Context used for this reply"}p {class:"quiet","{user.ancestors.len()} recent messages were included; {user.omitted_count} earlier messages remain saved here but were not sent with this turn."}}}
        }};
    }
    let Some(reply) = state.library.dialogue_reply(id) else {
        return rsx! {p {role:"alert","This saved message has not arrived yet."}};
    };
    let chunks = teacher_dialogue_reply_chunks(reply);
    rsx! {article {class:"dialogue-message teacher-message","data-message-id":id.to_string(),"data-dialogue-role":"assistant",
        p {class:"dialogue-speaker","Your teacher"}
        if reply.request.action==DialogueAction::Ask {if let Some(text)=&reply.content.explanation_english {TeacherProse {app,language:"en",text:text.clone(),purpose:PresentationPurpose::Instruction}}}
        for (index,chunk) in chunks.iter().enumerate() {
            div {class:"dialogue-spoken-chunk",
                PresentedRussian {app,class:"dialogue-russian",text:chunk.russian.clone(),purpose:PresentationPurpose::Reference}
                crate::dialogue_audio_view::DialogueChunkAudio {app,reply_id:id,chunk_index:index}
                details {summary {"Meaning & help"}TeacherProse {app,language:"en",text:chunk.english.clone(),purpose:PresentationPurpose::Reference}
                    if !chunk.note.is_empty(){TeacherProse {app,language:"en",text:chunk.note.clone(),purpose:PresentationPurpose::Instruction}}
                }
            }
        }
        if reply.request.action!=DialogueAction::Ask {if let Some(text)=&reply.content.explanation_english {
            details {class:"dialogue-reply-help",summary {"Help for this reply"}
                TeacherProse {app,language:"en",text:text.clone(),purpose:PresentationPurpose::Instruction}
            }
        }}
        if let Some(correction)=&reply.content.correction {div {class:"dialogue-small-correction",strong {"One useful adjustment"}
            PresentedRussian {app,text:format!("{} → {}",correction.original,correction.replacement),purpose:PresentationPurpose::Instruction}
            TeacherProse {app,language:"en",text:correction.reason.clone(),purpose:PresentationPurpose::Instruction}
        }}
        if reply.content.scene_state==DialogueSceneState::Closed {p {class:"quiet","A natural place to pause. You can continue if you want to."}}
        div {class:"dialogue-message-footer",small {class:"quiet","{reply.provenance.model} · AI-generated"}
            button {class:"text-button",onclick:move |_|act(app,|s|{s.open_dialogue_reply(id)?;focus_composer();Ok(())}),"Continue from this reply"}
        }
    }}
}

#[component]
fn DialogueComposer(mut app: Signal<AppState>, initial: DialogueDraft, ready: bool) -> Element {
    let mut typed = use_signal(|| initial.text.clone());
    let mut save_problem = use_signal(String::new);
    let mut draft = use_signal(|| initial);
    let mut discard_armed = use_signal(|| false);
    let capture_busy = use_signal(|| false);
    let mut voice_open = use_signal(|| false);
    let snapshot = draft();
    let saved = snapshot.committed_request_id.is_some();
    let busy = app.read().teacher_busy || capture_busy();
    rsx! {section {class:"panel dialogue-composer",aria_label:"Your next message",
        if snapshot.config.intent==DialogueIntent::Roleplay && snapshot.action!=DialogueAction::StartScene {
            label {r#for:"dialogue-action","What would you like to do?"}
            select {id:"dialogue-action",value:if snapshot.action==DialogueAction::Ask{"ask"}else{"respond"},disabled:saved||capture_busy(),onchange:move |e| {
                let mut next=draft();let action=if e.value()=="ask"{DialogueAction::Ask}else{DialogueAction::Respond};let correction=next.correction;
                let result=app.write().set_dialogue_options(&mut next,action,correction);match result {Ok(())=>draft.set(next),Err(problem)=>app.write().notice=problem}
            },option {value:"respond","Speak in the scene"}option {value:"ask","Ask the teacher for help"}}
        }
        if snapshot.action!=DialogueAction::StartScene {
            label {r#for:"dialogue-message",if snapshot.action==DialogueAction::Ask{"Your question"}else{"Your reply"}}
            textarea {id:"dialogue-message",value:typed(),maxlength:DIALOGUE_MAX_TEXT_BYTES,disabled:saved||capture_busy(),placeholder:"English or Russian is welcome. Your draft stays here.",oninput:move |e| {
                let value=e.value();typed.set(value.clone());
                let mut next=draft();let result=app.write().save_dialogue_draft(&mut next,value);match result{Ok(())=>{draft.set(next);save_problem.set(String::new());},Err(problem)=>save_problem.set(problem)}
            }}
            if !save_problem().is_empty() {div {class:"dialogue-save-problem",role:"alert",
                p {"This edit has not been saved. Keep this conversation open while you fix it. {save_problem}"}
                button {class:"secondary",disabled:saved||capture_busy(),onclick:move |_|{
                    let mut next=draft();let result=app.write().save_dialogue_draft(&mut next,typed());match result{Ok(())=>{draft.set(next);save_problem.set(String::new());},Err(problem)=>save_problem.set(problem)}
                },"Retry saving this edit"}
            }}
                    button {class:"text-button",disabled:saved||capture_busy()||!save_problem().is_empty(),onclick:move |_|voice_open.set(!voice_open()),if voice_open(){"Return to the displayed text"}else{"Use your voice"}}
            if voice_open() {crate::dialogue_audio_view::DialogueVoiceInput {app,thread_id:snapshot.thread_id,input_id:snapshot.id,
                on_capture_busy:move |busy|{let mut capture_busy=capture_busy;capture_busy.set(busy);},
                on_clear:move |_|{
                    let mut next=draft();let result=app.read().clear_dialogue_audio(&mut next);match result{Ok(())=>draft.set(next),Err(problem)=>app.write().notice=problem}
                },
                on_adopt:move |adopted:crate::dialogue_audio::DialogueAdoptedSpeech|{
                    let mut next=draft();let result=app.write().adopt_dialogue_audio(&mut next,adopted.text,adopted.source);match result{Ok(())=>{typed.set(next.text.clone());draft.set(next);save_problem.set(String::new());voice_open.set(false);},Err(problem)=>app.write().notice=problem}
                }
            }}
        }
        details {class:"dialogue-preferences",summary {"Corrections"}
            label {r#for:"dialogue-correction","How should the teacher respond?"}
            select {id:"dialogue-correction",value:if snapshot.correction==DialogueCorrection::Off{"off"}else{"one"},disabled:saved||capture_busy(),onchange:move |e| {
                let mut next=draft();let correction=if e.value()=="off"{DialogueCorrection::Off}else{DialogueCorrection::OneTargeted};let action=next.action;
                let result=app.write().set_dialogue_options(&mut next,action,correction);match result{Ok(())=>draft.set(next),Err(problem)=>app.write().notice=problem}
            },option {value:"one","Keep talking; offer one useful correction when needed"}option {value:"off","Focus on the conversation"}}
        }
        div {class:"actions",button {class:"primary",disabled:busy||saved||voice_open()||!save_problem().is_empty()||(snapshot.action!=DialogueAction::StartScene&&snapshot.text.trim().is_empty()),onclick:move |_| {
            let mut next=draft();let result=app.write().prepare_dialogue_turn(&mut next);
            match result {Ok(id)=>{draft.set(next);if ready{send_dialogue(app,id);}else{app.write().notice="Your message is saved. Connect your teacher, then choose Retry reply when you want to send it.".into();}},Err(problem)=>app.write().notice=problem}
        },if !ready {"Save message for later"}else if snapshot.action==DialogueAction::StartScene {"Start scene"}else if snapshot.action==DialogueAction::Ask {"Send question"}else{"Send reply"}}}
        if discard_armed() {div {class:"dialogue-discard",role:"group",aria_label:"Discard this draft",
            p {"Remove this unfinished message? Any saved recordings remain in your library."}
            button {class:"secondary",disabled:busy||saved,onclick:move |_|act(app,|s|s.discard_dialogue_draft(draft().id)),"Discard this draft"}
            button {class:"text-button",onclick:move |_|discard_armed.set(false),"Keep writing"}
        }} else {button {class:"text-button",disabled:busy||saved,onclick:move |_|discard_armed.set(true),"Discard draft…"}}
        p {class:"quiet","The teacher receives this message, its saved source and a limited part of this branch. Nothing is sent automatically when you reopen the app."}
    }}
}

pub(crate) fn send_dialogue(mut app: Signal<AppState>, request: Uuid) {
    if app.read().teacher_busy {
        return;
    }
    let prepared = app.write().begin_dialogue_dispatch(request);
    let dispatch = match prepared {
        Ok(d) => d,
        Err(problem) => {
            app.write().notice = problem;
            return;
        }
    };
    let state = app.read().clone();
    app.write().teacher_busy = true;
    dioxus_core::spawn_forever(async move {
        let response = async {
            let body = serde_json::to_value(&dispatch.request).map_err(|e| e.to_string())?;
            gateway::request::<TeacherDialogueResponse>(
                &state.endpoint,
                &state.token,
                "/api/teacher/dialogue",
                Some(&body),
                64 * 1024,
                120_000,
            )
            .await
        }
        .await;
        let result = match response {
            Ok(response) => app
                .write()
                .keep_dialogue_reply(&dispatch, &response)
                .map(|_| ()),
            Err(problem) => Err(problem),
        };
        match result {
            Ok(()) => {
                let state = app.read().clone();
                let same_thread = state
                    .active_dialogue_draft()
                    .ok()
                    .flatten()
                    .is_some_and(|d| d.thread_id == dispatch.request.thread_id);
                let visible = state.page == Page::Teacher
                    && state
                        .repo
                        .get::<String>("teacher_surface")
                        .ok()
                        .flatten()
                        .as_deref()
                        == Some("dialogues");
                if !same_thread || !visible {
                    app.write().notice = format!(
                        "Your teacher's reply is saved in ‘{}’. Your current work is kept.",
                        dispatch.request.config.topic
                    );
                }
            }
            Err(problem) => {
                let saved = app
                    .write()
                    .fail_dialogue_dispatch(&dispatch, problem.clone());
                if let Err(storage_problem) = saved {
                    app.write().notice = format!("{problem} {storage_problem}");
                }
            }
        }
        app.write().teacher_busy = false;
    });
}
