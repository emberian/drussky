//! Read an exact saved source, with optional support and no completion gate.
use crate::{
    exposure_view::PresentedRussian, model::AppState, reader::ReaderMode,
    reader::ReaderSupportKind, repository::Result, teacher_prose::TeacherProse,
};
use dioxus::prelude::*;
use rufren_core::{PresentationPurpose, READER_MAX_ORIGINAL_BYTES, ReaderTextLayer, TextSpan};
use uuid::Uuid;

const STYLE: &str = include_str!("reader_view.css");

fn outcome<T>(mut problem: Signal<String>, result: Result<T>) -> bool {
    match result {
        Ok(_) => {
            problem.set(String::new());
            true
        }
        Err(error) => {
            problem.set(error);
            false
        }
    }
}
fn focus_part(document: Uuid, chunk: &str) {
    let id = serde_json::to_string(&format!("reader-original-{document}-{chunk}")).unwrap();
    spawn(async move {
        let _ = document::eval(&format!("await new Promise(requestAnimationFrame);const part=document.getElementById({id});if(part){{part.focus({{preventScroll:true}});part.scrollIntoView({{block:'center'}});}}" )).await;
    });
}
fn safe_url(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    (lower.starts_with("https://") || lower.starts_with("http://"))
        && !url.chars().any(|c| c.is_control() || c.is_whitespace())
}

#[component]
pub(crate) fn ReaderView(mut app: Signal<AppState>) -> Element {
    let state = app.read();
    let place = state.reader_state();
    let shelf = state.reader_shelf();
    drop(state);
    let initial = use_hook(|| app.read().reader_import_draft());
    let mut title = use_signal(|| {
        initial
            .as_ref()
            .map(|draft| draft.title.clone())
            .unwrap_or_default()
    });
    let mut original = use_signal(|| {
        initial
            .as_ref()
            .map(|draft| draft.original.clone())
            .unwrap_or_default()
    });
    let mut source_note = use_signal(|| {
        initial
            .as_ref()
            .map(|draft| draft.source_note.clone())
            .unwrap_or_default()
    });
    let mut choosing = use_signal(|| false);
    let mut pasting = use_signal(|| false);
    let mut problem = use_signal(String::new);
    let selected = place
        .as_ref()
        .ok()
        .and_then(|place| place.as_ref())
        .map(|place| place.document_id);
    rsx! {
        style {"{STYLE}"}
        if selected.is_none() {div {class:"page-heading reader-heading",div {
            div {class:"eyebrow","A LITTLE RUSSIAN, A WHOLE WORLD"}h1 {"Read something worth staying with"}p {"Follow a scene or an idea. Bring in support when you want it."}
        }}}
        if let Err(error)=&place {p {class:"reader-error",role:"alert","{error}"}}
        if let Err(error)=&initial {p {class:"reader-error",role:"alert","{error}"}}
        if !problem().is_empty() {p {class:"reader-error",role:"alert","{problem}"}button {class:"text-button",onclick:move |_|{outcome(problem,app.write().flush_pending_reader_edits());},"Try saving again"}}
        if selected.is_none() || choosing() {
            section {class:"reader-shelf",aria_label:"Choose something to read",
                match shelf {
                    Ok(entries)=>rsx! {for entry in entries {article {class:"panel reader-book","data-reader-source":entry.document_id.to_string(),
                        PresentedRussian {app,language:"en",text:entry.title.clone(),purpose:PresentationPurpose::Reference,h2 {"{entry.title}"}}
                        PresentedRussian {app,language:"en",class:"quiet",text:entry.source_title.clone(),purpose:PresentationPurpose::Reference}
                        button {class:"text-button",onclick:move |_| {
                            let result=app.write().open_reader_document(entry.document_id,None).map(|_|());
                            if outcome(problem,result){choosing.set(false);}
                        },if entry.imported {"Return to this text →"}else{"Open this text →"}}
                    }}},
                    Err(error)=>rsx! {p {class:"reader-error",role:"alert","{error}"}},
                }
            }
            section {class:"panel reader-paste",
                button {class:"text-button",aria_expanded:pasting(),onclick:move |_|pasting.toggle(),"Or bring your own Russian text"}
                if pasting() {
                    label {r#for:"reader-import-title","Title"}
                    input {id:"reader-import-title",value:title(),oninput:move |event| {
                        let next=event.value();title.set(next.clone());
                        outcome(problem,app.write().edit_reader_import_draft(&next,&original(),&source_note()));
                    }}
                    label {r#for:"reader-import-original","Russian text"}
                    textarea {id:"reader-import-original",lang:"ru",rows:"8",value:original(),oninput:move |event| {
                        let next=event.value();original.set(next.clone());
                        outcome(problem,app.write().edit_reader_import_draft(&title(),&next,&source_note()));
                    }}
                    p {class:if original.read().len()>READER_MAX_ORIGINAL_BYTES {"reader-error"}else{"quiet"},"{original.read().len()} / {READER_MAX_ORIGINAL_BYTES} UTF-8 bytes. Your original wording and line breaks are kept."}
                    label {r#for:"reader-import-source","Source or author (optional)"}
                    input {id:"reader-import-source",value:source_note(),placeholder:"Book, author, link, or where you found it",oninput:move |event| {
                        let next=event.value();source_note.set(next.clone());
                        outcome(problem,app.write().edit_reader_import_draft(&title(),&original(),&next));
                    }}
                    button {class:"primary",disabled:title.read().trim().is_empty()||original.read().trim().is_empty()||original.read().len()>READER_MAX_ORIGINAL_BYTES,onclick:move |_| {
                        let result=(|| {let id=app.write().import_reader_with_source(&title(),&original(),&source_note())?;app.write().open_reader_document(id,None)?;Ok(())})();
                        if outcome(problem,result){choosing.set(false);pasting.set(false);}
                    },"Open my text →"}
                }
            }
        }
        if let Some(document)=selected {
            for document in [document] {ReaderDocumentView {key:"reader-document-{document}",app,document,choosing:choosing(),on_choose:move |_|choosing.toggle()}}
        }
    }
}

#[component]
fn ReaderDocumentView(
    mut app: Signal<AppState>,
    document: Uuid,
    choosing: bool,
    on_choose: EventHandler<()>,
) -> Element {
    let mut problem = use_signal(String::new);
    let mut source_open = use_signal(|| false);
    let result = (|| {
        let state = app.read();
        let place = state
            .reader_state()?
            .filter(|place| place.document_id == document)
            .ok_or("Reopen this text to restore your place.")?;
        Ok::<_, String>((state.reader_definition(document)?, place))
    })();
    let (definition, place) = match result {
        Ok(value) => value,
        Err(error) => return rsx! {p {class:"reader-error",role:"alert","{error}"}},
    };
    let position = definition
        .chunks
        .iter()
        .position(|chunk| chunk.id == place.chunk_id);
    let Some(position) = position else {
        return rsx! {p {class:"reader-error",role:"alert","Your saved part is unavailable. Your text and draft have been kept."}};
    };
    let count = definition.chunks.len();
    let chunk_id = place.chunk_id.clone();
    let previous = position
        .checked_sub(1)
        .map(|index| definition.chunks[index].id.clone());
    let next = definition
        .chunks
        .get(position + 1)
        .map(|chunk| chunk.id.clone());
    let source_kind = match serde_json::to_value(&definition.source.kind)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .as_deref()
    {
        Some("ai_authored") => "AI-written for Rufren",
        Some("authentic_excerpt") => "Published excerpt",
        _ => "Your own text",
    };
    rsx! {
        section {class:"reader-document","data-reader-document":document.to_string(),
            header {class:"reader-document-heading",
                div {class:"reader-title-actions",span {class:"tag","{source_kind}"}button {class:"text-button",aria_expanded:choosing,onclick:move |_|on_choose.call(()),"Choose text"}}
                PresentedRussian {app,language:"en",text:definition.title.clone(),purpose:PresentationPurpose::Reference,h1 {"{definition.title}"}}
            }
            div {class:"reader-utilities",
                button {class:"text-button",aria_expanded:source_open(),onclick:move |_|source_open.toggle(),"About this text & its support"}
            details {class:"reader-options",
                summary {"Reading options"}
                div {class:"reader-toolbar",
                div {class:"reader-mode",role:"group",aria_label:"Reading layout",
                    for (mode,label) in [(ReaderMode::Continuous,"Continuous text"),(ReaderMode::Focus,"One part at a time")] {
                        button {class:if place.mode==mode {"secondary selected"}else{"secondary"},aria_pressed:place.mode==mode,onclick:move |_|{outcome(problem,app.write().set_reader_mode(mode));},"{label}"}
                    }
                }
                label {class:"reader-stress-choice",input {r#type:"checkbox",checked:place.show_stress,onchange:move |event|{outcome(problem,app.write().set_reader_stress(event.checked()));}}" Show stress support alongside the original"}
            }
            }
            }
                if source_open() {div {class:"reader-source-details",
                    PresentedRussian {app,language:"en",class:"reader-byline",text:definition.source.author.clone(),purpose:PresentationPurpose::Reference}
                    TeacherProse {app,language:"en",text:definition.source.title.clone(),purpose:PresentationPurpose::Reference}
                    if !definition.source.edition.is_empty(){TeacherProse {app,language:"en",text:definition.source.edition.clone(),purpose:PresentationPurpose::Reference}}
                    TeacherProse {app,language:"en",text:definition.source.rights.clone(),purpose:PresentationPurpose::Reference}
                    TeacherProse {app,language:"en",text:definition.source.provenance.clone(),purpose:PresentationPurpose::Reference}
                    if let Some(url)=&definition.source.url {if safe_url(url) {a {href:url.clone(),target:"_blank",rel:"noopener noreferrer","View the source ↗"}}}
                    h3 {"About the reading support"}
                    TeacherProse {app,language:"en",text:definition.help_provenance.clone(),purpose:PresentationPurpose::Reference}
                }}
            nav {class:if place.mode==ReaderMode::Continuous {"reader-part-navigation reader-with-tools"}else{"reader-part-navigation"},aria_label:"Parts of this text",
                button {class:"text-button",aria_label:"← Previous part",disabled:previous.is_none(),onclick:move |_|{if let Some(id)=&previous {if outcome(problem,app.write().set_reader_chunk(id)){focus_part(document,id);}}},span {aria_hidden:"true","←"}}
                div {label {class:"reader-sr-only",r#for:"reader-part-select","Choose a part"}select {id:"reader-part-select",value:chunk_id.clone(),onchange:move |event|{let id=event.value();if outcome(problem,app.write().set_reader_chunk(&id)){focus_part(document,&id);}},
                    for (index,chunk) in definition.chunks.iter().enumerate(){option {value:chunk.id.clone(),"Part {index+1} of {count}"}}
                }}
                button {class:"text-button",aria_label:"Next part →",disabled:next.is_none(),onclick:move |_|{if let Some(id)=&next {if outcome(problem,app.write().set_reader_chunk(id)){focus_part(document,id);}}},span {aria_hidden:"true","→"}}
            if place.mode==ReaderMode::Continuous {button {class:"text-button reader-tools-link",aria_label:format!("Reading tools for part {}",position+1),onclick:move |_| {
                spawn(async move {let _=document::eval("await new Promise(requestAnimationFrame);const heading=document.querySelector('.reader-current-support .reader-part-heading');if(heading){heading.focus({preventScroll:true});heading.scrollIntoView({block:'start'});}").await;});
            },"Help"}}
            }
            if !problem().is_empty(){p {class:"reader-error",role:"alert","{problem}"}button {class:"text-button",onclick:move |_|{outcome(problem,app.write().flush_pending_reader_edits());},"Try saving again"}}
            div {class:if place.mode==ReaderMode::Continuous {"reader-reading-layout reader-continuous-layout"}else{"reader-reading-layout"},
                if place.mode==ReaderMode::Continuous {
                    div {class:"reader-original reader-continuous-original",lang:"ru","data-reader-original":document.to_string(),
                        for chunk in &definition.chunks {
                            {let id=chunk.id.clone();rsx! {PresentedRussian {key:"continuous-{document}-{chunk.id}",app,inline:true,text:chunk.text.clone(),purpose:PresentationPurpose::Reference,
                                span {id:format!("reader-original-{document}-{}",chunk.id),tabindex:"-1",class:if chunk.id==place.chunk_id {"reader-text-chunk reader-text-current"}else{"reader-text-chunk"},"data-reader-text-chunk":chunk.id.clone(),onclick:move |_|{outcome(problem,app.write().set_reader_chunk(&id));},"{chunk.text}"}
                            }}}
                        }
                    }
                }
                div {class:if place.mode==ReaderMode::Continuous {"reader-current-support"}else{"reader-pages"},
                    for chunk in definition.chunks.iter().filter(|chunk|chunk.id==place.chunk_id) {
                        ReaderChunkView {key:"reader-{document}-{chunk.id}",app,document,chunk_id:chunk.id.clone(),ordinal:position+1,active:true,show_stress:place.show_stress,show_original:place.mode==ReaderMode::Focus}
                    }
                }
            }
        }
    }
}

#[component]
fn ReaderChunkView(
    mut app: Signal<AppState>,
    document: Uuid,
    chunk_id: String,
    ordinal: usize,
    active: bool,
    show_stress: bool,
    show_original: bool,
) -> Element {
    let mut problem = use_signal(String::new);
    let mut reflection = use_signal(|| false);
    let mut audio = use_signal(|| false);
    let mut kept = use_signal(|| false);
    let initial = use_hook(|| {
        app.read().reader_state().map(|state| {
            state
                .and_then(|state| state.own_words.get(&chunk_id).cloned())
                .unwrap_or_default()
        })
    });
    let mut own_words = use_signal(|| initial.clone().unwrap_or_default());
    let open = match app.read().reader_state() {
        Ok(Some(state)) if state.document_id == document => {
            state.supports.get(&chunk_id).cloned().unwrap_or_default()
        }
        Ok(_) => {
            return rsx! {p {class:"reader-error",role:"alert","Reopen this reading to restore its support panels."}};
        }
        Err(error) => return rsx! {p {class:"reader-error",role:"alert","{error}"}},
    };
    let definition = match app.read().reader_definition(document) {
        Ok(value) => value,
        Err(error) => return rsx! {p {class:"reader-error",role:"alert","{error}"}},
    };
    let Some(chunk) = definition
        .chunks
        .into_iter()
        .find(|chunk| chunk.id == chunk_id)
    else {
        return rsx! {p {class:"reader-error",role:"alert","This part is unavailable. Your draft is kept."}};
    };
    let original_id = format!("reader-original-{document}-{chunk_id}");
    let selection_id = original_id.clone();
    let original_text = chunk.text.clone();
    let chunk_start = chunk.span.start;
    let keep_chunk = chunk_id.clone();
    let keep_whole_chunk = chunk_id.clone();
    let whole_span = chunk.span.clone();
    let edit_chunk = chunk_id.clone();
    let ask_chunk = chunk_id.clone();
    let audio_chunk = chunk_id.clone();
    let mark_chunk = chunk_id.clone();
    let field = format!("reader-own-words-{document}-{chunk_id}");
    rsx! {
        article {id:format!("reader-part-{document}-{chunk_id}"),class:if active {"reader-chunk reader-current"}else{"reader-chunk"},"data-reader-chunk":chunk_id.clone(),
            div {class:"reader-part-heading-row",h3 {class:"reader-part-heading",tabindex:"-1","Part {ordinal}"}
                if active {span {class:"reader-place","Your place"}}else {button {class:"text-button",onclick:move |_|{if outcome(problem,app.write().set_reader_chunk(&mark_chunk)){focus_part(document,&mark_chunk);}},"Keep my place here"}}
            }
            if !show_original {button {class:"text-button",onclick:{let chunk=chunk_id.clone();move |_|focus_part(document,&chunk)},"Back to this part of the text ↑"}}
            if show_original {PresentedRussian {app,text:chunk.text.clone(),purpose:PresentationPurpose::Reference,class:"reader-original",
                p {id:original_id,lang:"ru",tabindex:"-1","{chunk.text}"}
            }}
            if show_stress {if let Some(stress)=&chunk.stress {div {class:"reader-stress-support",span {class:"reader-support-label","Stress support"}PresentedRussian {app,text:stress.clone(),purpose:PresentationPurpose::Reference}}}}
            div {class:"reader-support",role:"group",aria_label:format!("Support for part {ordinal}"),
                for (label,kind,available) in [
                    ("Meaning",ReaderSupportKind::Meaning,chunk.translation.is_some()),
                    ("Words in context",ReaderSupportKind::Gloss,!chunk.glosses.is_empty()),
                    ("How the sentence works",ReaderSupportKind::Grammar,!chunk.grammar_notes.is_empty())] {
                    if available {{let id=chunk_id.clone();let show=!open.contains(&kind);rsx! {button {class:"text-button",aria_expanded:!show,onclick:move |_| {
                        outcome(problem,app.write().set_reader_support(document,&id,kind,show));
                    },"{label}"}}}}
                }
                button {class:"text-button",aria_expanded:audio(),onclick:move |_|audio.toggle(),"Listen to this part"}
                button {class:"text-button",aria_expanded:reflection(),onclick:move |_|reflection.toggle(),"In your own words / ask a question"}
            }
            if open.contains(&ReaderSupportKind::Meaning) {if let Some(translation)=&chunk.translation {section {class:"reader-help reader-meaning",h4 {"Meaning"}TeacherProse {app,language:"en",text:translation.clone(),purpose:PresentationPurpose::Reference}}}}
            if open.contains(&ReaderSupportKind::Gloss) {section {class:"reader-help reader-glosses",h4 {"Words in this context"}
                for note in &chunk.glosses {div {class:"reader-note",PresentedRussian {app,text:note.text.clone(),purpose:PresentationPurpose::Reference,strong {"{note.text}"}}TeacherProse {app,language:"en",text:note.explanation.clone(),purpose:PresentationPurpose::Reference}}}
            }}
            if open.contains(&ReaderSupportKind::Grammar) {section {class:"reader-help reader-grammar",h4 {"How the sentence works"}
                for note in &chunk.grammar_notes {div {class:"reader-note",PresentedRussian {app,text:note.text.clone(),purpose:PresentationPurpose::Reference,strong {"{note.text}"}}TeacherProse {app,language:"en",text:note.explanation.clone(),purpose:PresentationPurpose::Reference}}}
            }}
            if audio() {section {class:"reader-help reader-audio",h4 {"Hear the saved text"}
                crate::reader_audio::ReaderAudio {app,document_id:document,chunk_id:audio_chunk,layer:ReaderTextLayer::Original}
                if show_stress&&chunk.stress.is_some(){crate::reader_audio::ReaderAudio {app,document_id:document,chunk_id:chunk_id.clone(),layer:ReaderTextLayer::StressScaffold}}
            }}
            div {class:"reader-keep",button {class:"text-button",onclick:move |_| {
                let id=selection_id.clone();let chunk=keep_chunk.clone();let expected=original_text.clone();
                spawn(async move {
                    let quoted=serde_json::to_string(&id).unwrap();
                    let script=format!("const root=document.getElementById({quoted});const selection=getSelection();if(!root||!selection||selection.rangeCount!==1||!root.contains(selection.anchorNode)||!root.contains(selection.focusNode))return null;const range=selection.getRangeAt(0);const prefix=range.cloneRange();prefix.selectNodeContents(root);prefix.setEnd(range.startContainer,range.startOffset);const text=range.toString();const start=new TextEncoder().encode(prefix.toString()).length;return {{text,start,end:start+new TextEncoder().encode(text).length}};");
                    let result=async {
                        let selected=document::eval(&script).await.map_err(|_|"Select a phrase in this part first.")?;
                        let text=selected.get("text").and_then(|value|value.as_str()).ok_or("Select a phrase in this part first.")?;
                        let start=selected.get("start").and_then(|value|value.as_u64()).and_then(|value|usize::try_from(value).ok()).ok_or("The selected phrase could not be located in the original.")?;
                        let end=selected.get("end").and_then(|value|value.as_u64()).and_then(|value|usize::try_from(value).ok()).ok_or("The selected phrase could not be located in the original.")?;
                        if text.trim().is_empty(){return Err("Select a phrase in this part first.".to_string());}
                        if expected.get(start..end)!=Some(text){return Err("Select words in the unchanged original, rather than its support text.".to_string());}
                        let current=app.read().reader_state()?.ok_or("Reopen the selected original before keeping this phrase.")?;
                        if current.document_id!=document||current.chunk_id!=chunk {return Err("Your reading changed before this selection was saved. Select the words again in the current part.".to_string());}
                        app.write().keep_reader_span(document,&chunk,TextSpan{start:chunk_start+start,end:chunk_start+end})?;Ok(())
                    }.await;
                    if outcome(problem,result){kept.set(true);}
                });
            },"Keep selected words for later"}
                button {class:"text-button",onclick:move |_| {
                    if outcome(problem,app.write().keep_reader_span(document,&keep_whole_chunk,whole_span.clone())){kept.set(true);}
                },"Keep this part"}
                if kept(){span {class:"quiet",role:"status","Saved in your library."}}
            }
            if reflection()||!own_words.read().is_empty() {section {class:"reader-help reader-reflection",h4 {"An idea to take with you"}
                if !chunk.prompts.is_empty(){p {class:"quiet","A question to consider, if you like:"}for prompt in &chunk.prompts {TeacherProse {app,language:"en",text:prompt.question.clone(),purpose:PresentationPurpose::Reference}}}
                label {r#for:field.clone(),"What did you take from this part?"}
                textarea {id:field,rows:"3",value:own_words(),placeholder:"English or Russian is fine.",oninput:move |event|{
                    let next=event.value();own_words.set(next.clone());outcome(problem,app.write().edit_reader_own_words(document,&edit_chunk,&next));
                }}
                p {class:"quiet","This is a place for your thought, not a test. You can also use it to ask about something unfamiliar."}
                button {class:"secondary",onclick:move |_|{
                    let result=(|| {app.write().edit_reader_own_words(document,&ask_chunk,&own_words())?;
                        app.write().prepare_reader_question(document,&ask_chunk,&own_words())?;
                        crate::teacher_dialogue_view::show_conversations(&mut app.write())?;Ok(())})();
                    outcome(problem,result);
                },"Prepare a question for your teacher →"}
                p {class:"quiet","Opens an editable question. Nothing is sent until you choose Send."}
            }}
            if let Err(error)=&initial {p {class:"reader-error",role:"alert","{error}"}}
            if !problem().is_empty(){p {class:"reader-error",role:"alert","{problem}"}button {class:"text-button",onclick:move |_|{outcome(problem,app.write().flush_pending_reader_edits());},"Try saving again"}}
        }
    }
}
