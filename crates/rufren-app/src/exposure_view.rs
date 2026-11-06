//! Log presentation only after Russian actually enters the visible viewport.
//! This is an opportunity to see text, never a claim that the learner read or learned it.
use crate::{act, model::AppState};
use dioxus::prelude::*;
use rufren_core::{PresentationPurpose, StimulusPresentation};
use uuid::Uuid;

#[component]
pub(crate) fn PresentedRussian(
    app: Signal<AppState>,
    text: String,
    purpose: PresentationPurpose,
    #[props(default)] class: String,
    #[props(default = "ru".to_string())] language: String,
    #[props(default)] revision: Option<Uuid>,
    #[props(default)] session: Option<Uuid>,
    #[props(default)] decision: Option<Uuid>,
    #[props(default)] utterance_family: Option<String>,
) -> Element {
    let instance = use_hook(Uuid::new_v4);
    let element_id = format!("russian-presentation-{instance}");
    let observer_id = element_id.clone();
    let cleanup_id = element_id.clone();
    let snapshot = StimulusPresentation {
        session,
        revision,
        russian: text.clone(),
        purpose,
        decision,
        utterance_family,
    };
    use_effect(move || {
        let snapshot = snapshot.clone();
        let id = observer_id.clone();
        spawn(async move {
            let script = format!(
                "const presentationId = {};\n{}",
                serde_json::to_string(&id).unwrap(),
                include_str!("exposure_view.js")
            );
            match document::eval(&script).await {
                Ok(serde_json::Value::Bool(true)) => {
                    act(app, |state| state.note_presentation(instance, snapshot))
                }
                // Unmounting, background tabs and unsupported observers produce no exposure.
                _ => (),
            }
        });
    });
    use_drop(move || {
        let id = serde_json::to_string(&cleanup_id).unwrap();
        let _ = document::eval(&format!("window.rufrenPresentations?.get({id})?.();"));
    });
    rsx! {p {id:element_id, class, lang:language, "{text}"}}
}
