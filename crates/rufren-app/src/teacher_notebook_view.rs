//! Reopen immutable teaching without regenerating it or changing its exercise identities.
use crate::{
    act,
    exposure_view::PresentedRussian,
    model::{AppState, Page},
    teacher_notebook::TeacherNotebookEntry,
};
use dioxus::prelude::*;
use rufren_core::{PresentationPurpose, teacher_document_id};
use rufren_tutor::{TeacherRequest, TeacherResponse};
use uuid::Uuid;

const STYLE: &str = ".teacher-notebook-feedback input {width:auto;margin-right:8px}.teacher-notebook-entry {margin-top:12px}.teacher-notebook-entry .card-link {display:block;margin-top:8px}.teacher-notebook-entry p {margin:8px 0}";

pub(crate) fn lesson_step_count(answer: &TeacherResponse) -> usize {
    answer
        .content
        .lesson_flow
        .as_ref()
        .map_or(1 + answer.content.practice.len(), |flow| flow.steps.len())
}

pub(crate) fn step_for_practice(answer: &TeacherResponse, index: usize) -> usize {
    answer
        .content
        .lesson_flow
        .as_ref()
        .and_then(|flow| {
            flow.steps
                .iter()
                .position(|step| step.practice_index == Some(index))
        })
        .unwrap_or(index + 1)
}

fn title(entry: &TeacherNotebookEntry) -> String {
    match &entry.response.request {
        TeacherRequest::TeachLesson(request) => request.title.clone(),
        TeacherRequest::AnswerFeedback(request) => format!(
            "Feedback · {}",
            match request.skill {
                rufren_tutor::TeacherSkill::Reading => "Reading",
                rufren_tutor::TeacherSkill::Writing => "Writing",
                rufren_tutor::TeacherSkill::Listening => "Listening",
                rufren_tutor::TeacherSkill::Speaking => "Speaking",
            }
        ),
    }
}

fn age(entry: &TeacherNotebookEntry) -> String {
    let Some(received) = entry.received_at_ms else {
        return "Saved earlier".into();
    };
    let elapsed = crate::repository::now_ms().saturating_sub(received);
    if elapsed < 0 {
        return "Saved on another device".into();
    }
    let days = elapsed / 86_400_000;
    match days {
        0 => "Saved today".into(),
        1 => "Saved about a day ago".into(),
        _ => format!("Saved about {days} days ago"),
    }
}

#[component]
pub(crate) fn TeacherNotebook(
    app: Signal<AppState>,
    #[props(default)] on_selected: EventHandler<Uuid>,
    #[props(default)] disabled: bool,
) -> Element {
    let mut expanded = use_signal(|| false);
    let mut query = use_signal(String::new);
    let mut include_feedback = use_signal(|| false);
    let mut limit = use_signal(|| 12usize);
    let state = app.read().clone();
    let documents = state.teacher_notebook();
    if documents.is_empty() {
        return rsx! {};
    }
    let text = query().trim().to_lowercase();
    let matching: Vec<_> = documents
        .iter()
        .filter(|entry| {
            if !include_feedback()
                && !matches!(entry.response.request, TeacherRequest::TeachLesson(_))
            {
                return false;
            }
            if text.is_empty() {
                return true;
            }
            let searchable = match &entry.response.request {
                TeacherRequest::TeachLesson(request) => format!(
                    "{} {} {}",
                    request.title, request.objective, entry.response.content.explanation
                ),
                TeacherRequest::AnswerFeedback(request) => format!(
                    "{} {} {}",
                    title(entry),
                    request.prompt,
                    entry.response.content.explanation
                ),
            };
            searchable.to_lowercase().contains(&text)
        })
        .collect();
    rsx! {
        section {class:"panel teacher-notebook",aria_label:"Saved teaching",
            style {"{STYLE}"}
            button {class:"secondary",aria_expanded:expanded(),disabled,onclick:move |_|expanded.set(!expanded()),"Your saved lessons"}
            if expanded() {
                p {class:"quiet","Reopen a lesson with its examples and exercises. Your current work is kept."}
                label {r#for:"teacher-notebook-search","Find a lesson"}
                input {id:"teacher-notebook-search",r#type:"search",value:query(),placeholder:"Search by topic or words",oninput:move |event|{query.set(event.value());limit.set(12);}}
                label {class:"teacher-notebook-feedback",
                    input {r#type:"checkbox",checked:include_feedback(),onchange:move |event|{include_feedback.set(event.checked());limit.set(12);}}
                    " Include saved feedback"
                }
                if matching.is_empty() {p {"No saved teaching matches this search."}}
                for entry in matching.iter().take(limit()) {
                    {let id=entry.document_id;let label=title(entry);let stamp=age(entry);let lesson=matches!(entry.response.request,TeacherRequest::TeachLesson(_));
                    rsx! {button {key:"document-{id}",class:"attempt-choice teacher-notebook-entry","data-document-id":id.to_string(),disabled,
                        onclick:move |_| {
                            let result=app.read().select_teacher_document(id);
                            match result {Ok(())=>{on_selected.call(id);expanded.set(false);act(app,|s|s.navigate(Page::Teacher));},Err(problem)=>{let mut app=app;app.write().notice=problem;}}
                        },
                        span {class:"tag",if lesson {"LESSON"}else{"FEEDBACK"}}
                        PresentedRussian {app,text:label.clone(),language:"en",purpose:PresentationPurpose::Instruction,strong {"{label}"}}
                        small {"{stamp}"}
                        span {class:"card-link","Open saved teaching →"}
                    }}}
                }
                if matching.len()>limit() {button {class:"text-button",onclick:move |_|limit.set(limit()+12),"Show more saved teaching"}}
            }
        }
    }
}

#[component]
pub(crate) fn ContinueTeacher(app: Signal<AppState>) -> Element {
    let state = app.read().clone();
    let documents = state.teacher_notebook();
    let selected = state.selected_teacher_document().ok().flatten();
    let preferred = selected.as_ref().and_then(|answer| match &answer.request {
        TeacherRequest::TeachLesson(_) => Some(teacher_document_id(answer)),
        TeacherRequest::AnswerFeedback(_) => {
            state.teacher_lesson_for_feedback(answer).map(|(id, _)| id)
        }
    });
    let unfinished = |entry: &&TeacherNotebookEntry| {
        matches!(entry.response.request, TeacherRequest::TeachLesson(_))
            && state
                .teacher_lesson_cursor(entry.document_id)
                .is_ok_and(|cursor| cursor < lesson_step_count(&entry.response))
    };
    let selected = preferred
        .and_then(|id| {
            documents
                .iter()
                .filter(unfinished)
                .find(|entry| entry.document_id == id)
        })
        .or_else(|| documents.iter().filter(unfinished).next());
    let Some(entry) = selected else {
        return rsx! {};
    };
    let id = entry.document_id;
    let cursor = state.teacher_lesson_cursor(id).unwrap_or(0);
    let steps = lesson_step_count(&entry.response);
    let label = title(entry);
    rsx! {
        section {class:"panel teacher-resume",aria_label:"Continue your teaching",
            span {class:"tag","PICK UP WHERE YOU LEFT OFF"}
            PresentedRussian {app,text:label.clone(),language:"en",purpose:PresentationPurpose::Instruction,h2 {"{label}"}}
            p {class:"quiet","Step {cursor+1} of {steps}. Your responses and recordings are kept."}
            button {class:"primary",onclick:move |_|act(app,|state|{state.select_teacher_document(id)?;state.navigate(Page::Teacher)}),"Continue this lesson →"}
        }
    }
}
