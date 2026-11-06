//! Shared destinations for the sidebar and compact navigation.
use crate::{
    act, focus_page_heading,
    model::{AppState, Page},
    repository::Result,
};
use dioxus::prelude::*;

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Destination {
    Today,
    Learn,
    Talk,
    Read,
    Notebook,
}
impl Destination {
    const ALL: [Self; 5] = [
        Self::Today,
        Self::Learn,
        Self::Talk,
        Self::Read,
        Self::Notebook,
    ];
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Today => "Today",
            Self::Learn => "Learn",
            Self::Talk => "Talk",
            Self::Read => "Read",
            Self::Notebook => "Notebook",
        }
    }
    fn id(self) -> &'static str {
        match self {
            Self::Today => "today",
            Self::Learn => "learn",
            Self::Talk => "talk",
            Self::Read => "read",
            Self::Notebook => "notebook",
        }
    }
}
pub(crate) fn current(state: &AppState) -> Destination {
    match state.page {
        Page::Today => Destination::Today,
        Page::Reader => Destination::Read,
        Page::Library | Page::Progress | Page::Settings => Destination::Notebook,
        Page::Teacher
            if state
                .repo
                .get::<String>("teacher_surface")
                .ok()
                .flatten()
                .as_deref()
                == Some("dialogues") =>
        {
            Destination::Talk
        }
        _ => Destination::Learn,
    }
}
pub(crate) fn open(state: &mut AppState, destination: Destination) -> Result<()> {
    match destination {
        Destination::Today => state.navigate(Page::Today),
        Destination::Learn => state.navigate(Page::Lessons),
        Destination::Talk => crate::teacher_dialogue_view::show_conversations(state),
        Destination::Read => state.navigate(Page::Reader),
        Destination::Notebook => state.navigate(Page::Library),
    }
}
fn go(app: Signal<AppState>, destination: Destination) {
    act(app, |state| {
        open(state, destination)?;
        focus_page_heading();
        Ok(())
    });
}
fn go_page(app: Signal<AppState>, page: Page) {
    act(app, |state| {
        state.navigate(page)?;
        focus_page_heading();
        Ok(())
    });
}
fn go_teacher(app: Signal<AppState>) {
    act(app, |state| {
        state.navigate(Page::Teacher)?;
        state.repo.set("teacher_surface", &"lessons")?;
        focus_page_heading();
        Ok(())
    });
}

#[component]
fn Icon(kind: &'static str) -> Element {
    let path = match kind {
        "today" => "m3 10 9-7 9 7v10a1 1 0 0 1-1 1h-5v-7H9v7H4a1 1 0 0 1-1-1Z",
        "learn" => "M3 5c4-1 6 0 9 2 3-2 5-3 9-2v15c-4-1-6 0-9 2-3-2-5-3-9-2ZM12 7v15",
        "talk" => "M21 11a8 8 0 0 1-8 8H8l-5 3 1-6a8 8 0 1 1 17-5ZM8 10h8M8 14h5",
        "read" => {
            "M6 3h10a3 3 0 0 1 3 3v15H6a3 3 0 0 1-3-3V6a3 3 0 0 1 3-3ZM3 18a3 3 0 0 1 3-3h13M7 7h8M7 11h6"
        }
        "notebook" => "M6 3h13v18H6a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2ZM8 3v18M11 8h5M11 12h5",
        _ => "M4 12h16M12 4v16",
    };
    rsx! {svg {view_box:"0 0 24 24",width:"24",height:"24",fill:"none",stroke:"currentColor",stroke_width:"1.7",stroke_linecap:"round",stroke_linejoin:"round","aria-hidden":"true",path {d:path}}}
}

#[component]
pub(crate) fn PrimaryNavigation(app: Signal<AppState>, compact: bool) -> Element {
    let selected = current(&app.read());
    rsx! {nav {class:if compact {"compact-navigation"}else{"primary-navigation"},aria_label:if compact {"Main navigation"}else{"Main navigation sidebar"},
        for destination in Destination::ALL {button {id:format!("{}-{}",if compact {"tab"}else{"sidebar"},destination.id()),class:if selected==destination {"destination active"}else{"destination"},aria_current:if selected==destination {"page"}else{"false"},onclick:move |_|go(app,destination),Icon {kind:destination.id()}span {"{destination.label()}"}}}
    }}
}

fn page_key(page: Page) -> &'static str {
    match page {
        Page::Today => "today",
        Page::Lessons | Page::LessonDetail => "lessons",
        Page::Foundations => "foundations",
        Page::Teacher => "teacher",
        Page::Reader => "reader",
        Page::Library => "library",
        Page::Practice => "practice",
        Page::Studio => "studio",
        Page::Progress => "progress",
        Page::Settings => "settings",
    }
}

#[component]
pub(crate) fn SecondaryNavigation(app: Signal<AppState>, compact: bool) -> Element {
    let state = app.read();
    let selected = current(&state);
    let page = state.page;
    let learn = !compact || selected == Destination::Learn;
    let notebook = !compact || selected == Destination::Notebook;
    rsx! {div {class:if compact {"context-navigation"}else{"sidebar-sections"},
        if learn {nav {aria_label:"Learning",h2 {"Learn"}
            if compact {button {id:"context-lessons",class:if page==Page::Lessons {"context-link active"}else{"context-link"},aria_current:if page==Page::Lessons {"page"}else{"false"},onclick:move |_|go_page(app,Page::Lessons),"Lessons"}}
            button {id:if compact {"context-teacher"}else{"tools-teacher"},class:if page==Page::Teacher && selected==Destination::Learn {"context-link active"}else{"context-link"},aria_current:if page==Page::Teacher && selected==Destination::Learn {"page"}else{"false"},onclick:move |_|go_teacher(app),"AI lessons & feedback"}
            for (target,label) in [(Page::Practice,"Practice"),(Page::Studio,"Speech studio")] {button {id:format!("{}-{}",if compact {"context"}else{"tools"},page_key(target)),class:if page==target {"context-link active"}else{"context-link"},aria_current:if page==target {"page"}else{"false"},onclick:move |_|go_page(app,target),"{label}"}}
        }}
        if notebook {nav {aria_label:"Notebook",h2 {"Notebook"}
            if compact {button {id:"context-library",class:if page==Page::Library {"context-link active"}else{"context-link"},aria_current:if page==Page::Library {"page"}else{"false"},onclick:move |_|go_page(app,Page::Library),"Passages"}}
            for (target,label) in [(Page::Progress,"Progress"),(Page::Settings,"Settings")] {button {id:format!("{}-{}",if compact {"context"}else{"tools"},page_key(target)),class:if page==target {"context-link active"}else{"context-link"},aria_current:if page==target {"page"}else{"false"},onclick:move |_|go_page(app,target),"{label}"}}
        }}
    }}
}
