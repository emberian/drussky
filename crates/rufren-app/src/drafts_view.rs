use crate::{act, model::AppState};
use dioxus::prelude::*;

#[component]
pub(crate) fn PausedPractices(app: Signal<AppState>) -> Element {
    let state = app.read();
    match state.paused_practices() {
        Err(problem) => {
            rsx! {section {class:"panel",h2 {"Your paused practice"}p {role:"alert","{problem}"}}}
        }
        Ok(records) if records.is_empty() => rsx! {},
        Ok(records) => rsx! {
            section {class:"panel",h2 {"Pick up an earlier thought"}p {class:"quiet","Your responses and the support you used are kept when you switch tasks."}
                for record in records {
                    {let id=record.session.id;
                    let same_active_reference=state.session.as_ref().is_some_and(|s|s.phase==rufren_core::Phase::Retrieve && s.revision==record.session.revision);
                    let title=if same_active_reference {"Earlier practice of this passage".to_owned()}else{record.plan.as_ref().and_then(|p|p.snapshot.as_ref()).map(|s|s.lesson_title.as_str())
                        .or_else(||state.library.revision(record.session.revision).map(|r|r.passage.title.as_str()))
                        .unwrap_or("Saved practice").to_owned()};
                    rsx! {button {class:"attempt-choice",onclick:move |_|act(app,|s|s.resume_practice(id)),
                        span {class:"tag","{record.session.skill.label()}"}strong {"{title}"}
                        p {if record.session.draft.is_empty() {"Saved listening and support"}else if same_active_reference {"Your earlier response is kept"}else{"{record.session.draft}"}}
                        small {"Continue this practice →"}
                    }}}
                }
            }
        },
    }
}
