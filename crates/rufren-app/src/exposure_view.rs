//! Log presentation only after Russian actually enters the visible viewport.
//! This is an opportunity to see text, never a claim that the learner read or learned it.
use crate::{act, model::AppState};
use dioxus::prelude::*;
use rufren_core::{PresentationPurpose, StimulusPresentation};
use uuid::Uuid;

/// Changing displayed content creates a fresh observation identity. A rerender of
/// unchanged text keeps its observer, while swapping lessons cannot reuse a fact ID.
#[component]
pub(crate) fn PresentedRussian(
    app: Signal<AppState>,
    text: String,
    purpose: PresentationPurpose,
    #[props(default)] class: String,
    #[props(default = "ru".to_string())] language: String,
    #[props(default)] children: Option<Element>,
    #[props(default)] revision: Option<Uuid>,
    #[props(default)] session: Option<Uuid>,
    #[props(default)] decision: Option<Uuid>,
    #[props(default)] utterance_family: Option<String>,
) -> Element {
    let identity = serde_json::to_string(&(
        &text,
        purpose,
        revision,
        session,
        decision,
        &utterance_family,
        &language,
    ))
    .expect("presentation identity is serializable");
    // Dioxus consults keys during sibling-list reconciliation. A key on a
    // single static component slot does not remount that component on change.
    rsx! {for identity in [identity] {
        PresentedRussianBlock {key:"{identity}", app, text:text.clone(), purpose,
            class:class.clone(), language:language.clone(), children:children.clone(),
            revision, session, decision, utterance_family:utterance_family.clone()}
    }}
}

#[component]
fn PresentedRussianBlock(
    app: Signal<AppState>,
    text: String,
    purpose: PresentationPurpose,
    #[props(default)] class: String,
    #[props(default = "ru".to_string())] language: String,
    #[props(default)] children: Option<Element>,
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
    match children {
        Some(children) => rsx! {div {id:element_id, class, lang:language, {children}}},
        None => rsx! {p {id:element_id, class, lang:language, "{text}"}},
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use crate::repository::Repository;
    use std::{cell::RefCell, rc::Rc};

    type Display = (String, Option<Uuid>);
    thread_local! {
        static DISPLAY: RefCell<Option<Signal<Display>>> = const { RefCell::new(None) };
    }
    fn harness() -> Element {
        let app = use_signal(|| {
            AppState::from_repository(Rc::new(Repository::open_at(":memory:").unwrap())).unwrap()
        });
        let display = use_signal(|| ("Приве́т".to_owned(), None));
        DISPLAY.with(|slot| *slot.borrow_mut() = Some(display));
        let (text, revision) = display();
        rsx! {PresentedRussian {app,text,revision,purpose:PresentationPurpose::Reference}}
    }
    fn rendered_id(dom: &VirtualDom) -> String {
        let html = dioxus_ssr::render(dom);
        html.split("russian-presentation-")
            .nth(1)
            .expect("observed element")
            .split('"')
            .next()
            .unwrap()
            .to_owned()
    }
    #[test]
    fn changed_visible_text_or_attribution_gets_fresh_observer_identity() {
        // Real component reconciliation, not a reimplementation of its key logic.
        // This catches the prior stable-hook UUID reusing an old snapshot/fact ID.
        let mut dom = VirtualDom::new(harness);
        dom.rebuild_in_place();
        let first = rendered_id(&dom);
        let mut display = DISPLAY.with(|slot| slot.borrow().unwrap());
        dom.in_runtime(|| display.set(("Здравствуйте".into(), None)));
        dom.render_immediate_to_vec();
        let second = rendered_id(&dom);
        assert!(dioxus_ssr::render(&dom).contains("Здравствуйте"));
        assert_ne!(first, second);
        dom.in_runtime(|| display.set(("Здравствуйте".into(), Some(Uuid::new_v4()))));
        dom.render_immediate_to_vec();
        let third = rendered_id(&dom);
        assert_ne!(second, third);
        // Identical content/attribution preserves identity across an ordinary rerender.
        let same = display();
        dom.in_runtime(|| display.set(same));
        dom.render_immediate_to_vec();
        assert_eq!(third, rendered_id(&dom));
        DISPLAY.with(|slot| *slot.borrow_mut() = None);
    }
}
