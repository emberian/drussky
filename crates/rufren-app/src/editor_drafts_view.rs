use crate::{act, model::AppState};
use dioxus::prelude::*;

#[component]
pub(crate) fn EditorDrafts(app: Signal<AppState>) -> Element {
    let state = app.read();
    match state.editor_drafts() {
        Err(problem) => {
            rsx! {section {class:"panel",h2 {"Your passage drafts"}p {role:"alert","{problem}"}}}
        }
        Ok(records) if records.is_empty() => rsx! {},
        Ok(records) => {
            rsx! {section {class:"panel",h2 {"Passages in progress"}p {class:"quiet","Unfinished passages stay here when you open another. These drafts are saved on this device."}
                for record in records {{let id=record.editor.draft_id;let title=if record.editor.title.trim().is_empty(){"Untitled passage".to_owned()}else{record.editor.title.clone()};rsx! {button {class:"attempt-choice",onclick:move |_|act(app,|s|s.resume_editor_draft(id)),strong {"{title}"}p {lang:"ru","{record.editor.russian}"}small {"Continue this passage →"}}}}}
            }}
        }
    }
}
