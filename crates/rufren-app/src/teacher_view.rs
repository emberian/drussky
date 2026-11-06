use crate::{
    act,
    audio_studio::AudioReview,
    exposure_view::PresentedRussian,
    gateway,
    model::{AppState, Page},
    teacher_lesson_view::TeacherLessonView,
    teacher_notebook_view::{TeacherNotebook, step_for_practice},
    teacher_prose::TeacherProse,
};
use dioxus::prelude::*;
use rufren_core::*;
use rufren_tutor::*;
use uuid::Uuid;

#[component]
pub(crate) fn Teacher(mut app: Signal<AppState>) -> Element {
    let conversations = app
        .read()
        .repo
        .get::<String>("teacher_surface")
        .ok()
        .flatten()
        .as_deref()
        == Some("dialogues");
    rsx! {
        style {".teacher-surface-switch {{display:flex;gap:8px;flex-wrap:wrap;margin:12px 0 22px}}.teacher-surface-switch button[aria-pressed=true] {{background:var(--green);color:#fff6da}}"}
        div {class:"page-heading teacher-compact-heading",h1 {"Your Russian teacher"}}
        div {class:"teacher-surface-switch",role:"group",aria_label:"Teacher activities",
            button {class:"secondary",aria_pressed:!conversations,onclick:move |_|act(app,|s|s.repo.set("teacher_surface",&"lessons")),"Lessons"}
            button {class:"secondary",aria_pressed:conversations,onclick:move |_|act(app,|s|s.repo.set("teacher_surface",&"dialogues")),"Conversations"}
        }
        if conversations {crate::teacher_dialogue_view::TeacherDialogues {app}}else{TeacherLessons {app}}
    }
}

#[component]
fn TeacherLessons(mut app: Signal<AppState>) -> Element {
    let mut history_for = use_signal(|| None::<String>);
    let active_speaking = use_signal(|| None::<String>);
    let capture_busy = use_signal(|| false);
    let state = app.read().clone();
    let mut attempts: Vec<_> = state.library.attempts().collect();
    attempts.sort_by_key(|(e, _)| std::cmp::Reverse(e.recorded_at_ms));
    let latest = attempts.first().map(|(e, _)| e.id);
    let mut selected = use_signal(move || latest);
    let selected_id = selected().or(latest);
    let current = selected_id.and_then(|id| state.library.attempt(id).map(|a| (id, a)));
    let endpoint = state.endpoint.clone();
    let token = state.token.clone();
    let mut availability = use_resource(move || {
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
    let selected_document = state.selected_teacher_document();
    let answer = selected_document
        .as_ref()
        .ok()
        .cloned()
        .flatten()
        .or_else(|| {
            state
                .repo
                .get::<TeacherResponse>("teacher_response")
                .ok()
                .flatten()
        });
    let response_key = answer
        .as_ref()
        .map(|response| response.provenance.response_id.clone())
        .unwrap_or_else(|| "no-teacher-response".into());
    let reviewing_history = history_for.read().as_deref() == Some(response_key.as_str());
    let return_lesson = answer
        .as_ref()
        .and_then(|answer| state.teacher_lesson_for_feedback(answer));
    let return_review = answer
        .as_ref()
        .and_then(|answer| {
            let key = state.teacher_revisit_for_feedback(answer)?;
            let (_, draft) = state.active_teacher_revisit().ok().flatten()?;
            let TeacherRequest::AnswerFeedback(request) = &answer.request else {
                return None;
            };
            (draft.source == key.document_id.to_string()
                && draft.index == key.practice_index
                && draft.saved_attempt.map(|id| id.to_string()).as_deref()
                    == request.attempt_id.as_deref())
            .then_some(())
        })
        .is_some();
    let feedback_document = answer
        .as_ref()
        .filter(|answer| matches!(answer.request, TeacherRequest::TeachLesson(_)))
        .and_then(|lesson| {
            let id = teacher_document_id(lesson);
            state
                .teacher_notebook()
                .into_iter()
                .find(|entry| {
                    state
                        .teacher_lesson_for_feedback(&entry.response)
                        .is_some_and(|(source, _)| source == id)
                })
                .map(|entry| entry.document_id)
        });
    let supports_guided = status.as_ref().is_some_and(|status| {
        status
            .as_ref()
            .is_ok_and(|status| status.lesson_flow == Some(LessonFlowCapability::GuidedV1))
    });
    let lesson = state.lesson().or_else(|| state.content.lessons.first());
    let recent_practice = state.lesson_practice_summary().ok();
    let prepared_context = answer.as_ref().and_then(|answer| match &answer.request {
        TeacherRequest::TeachLesson(request) => request.learner_context.clone(),
        _ => None,
    });
    let free_answer = state
        .repo
        .get::<TutorResponse>("tutor_response")
        .ok()
        .flatten()
        .filter(|r| r.content.validate().is_ok());
    let history_key = response_key.clone();
    let management = rsx! {
        TeacherNotebook {app,disabled:state.teacher_busy || state.speech_busy || capture_busy(),on_selected:move |_|history_for.set(None)}
        details {class:"panel teacher-start",open:answer.is_none(),
            summary {if answer.is_some(){"Start another lesson"}else{"Choose your first lesson"}}
            h2 {"What shall we work on?"}
            label {r#for:"teacher-lesson","Your lesson"}
            select {id:"teacher-lesson",value:lesson.map(|l|l.id.clone()).unwrap_or_default(),disabled:state.teacher_busy,onchange:move |e|act(app,|s|{s.selected_lesson=Some(e.value());s.repo.set("selected_lesson",&s.selected_lesson)}),
                for lesson in &state.content.lessons {option {value:lesson.id.clone(),"{lesson.title}"}}
            }
            if let Some(lesson)=lesson {p {class:"lesson-goal","{lesson.goal}"}}
            if let Some(recent)=recent_practice {
                details {class:"teacher-context-preview",
                    summary {"Recent practice sent with this lesson"}
                    p {class:"quiet","Includes a short summary of your saved work on this lesson: {recent.observations.len()} recent responses."}
                    PresentedRussian {app,text:recent.provider_context(),language:"en",purpose:PresentationPurpose::Instruction,pre {class:"teacher-context-text","{recent.provider_context()}"}}
                }
            }
            div {class:"actions",button {class:"primary",disabled:state.teacher_busy || state.speech_busy || capture_busy(),onclick:move |_|{let mut request=app.read().lesson_teaching_request();if !supports_guided {if let Ok(TeacherRequest::TeachLesson(lesson))=&mut request {lesson.lesson_flow=None;}}send_teacher(app,request);},if state.teacher_busy {"Your teacher is thinking…"}else{"Teach me this lesson →"}}
                button {class:"secondary",onclick:move |_|act(app,|s|s.navigate(Page::Lessons)),"Open lesson & practice"}}
            p {class:"quiet","Start with a short explanation, try the words, then bring them into your next conversation."}
            match status {
                Some(Ok(status))=>rsx! {p {class:"quiet",role:"status",if status.enabled {"Ready" if let Some(model)=&status.model {" · {model}"}}else{"{status.message}"}}},
                Some(Err(problem))=>rsx! {p {class:"quiet",role:"status","{problem}"}button {class:"text-button",onclick:move |_|act(app,|s|s.navigate(Page::Settings)),"Open teacher connection"}},
                None=>rsx! {p {class:"quiet",role:"status","Checking your teacher connection…"}},
            }
            button {class:"text-button",disabled:state.teacher_busy,onclick:move |_|availability.restart(),"Check connection again"}
        }
        if !attempts.is_empty() {
            div {class:"actions teacher-history-toggle",
                button {class:"secondary",aria_expanded:reviewing_history,disabled:state.teacher_busy || state.speech_busy || capture_busy(),onclick:move |_|history_for.set(if reviewing_history {None}else{Some(history_key.clone())}),
                    if reviewing_history {"Return to current teaching"}else{"Review saved practice"}
                }
            }
        }
    };
    rsx! {
        style {".teacher-compact-heading {{margin-bottom:12px}}.teacher-compact-heading h1 {{font-size:1.75rem;margin:0}}.teacher-tools {{margin-bottom:16px}}.teacher-tools > summary {{min-height:44px;display:flex;align-items:center;cursor:pointer;font-weight:600}}.teacher-tools > summary::before {{content:\"\";width:0;height:0;border-left:5px solid currentColor;border-top:4px solid transparent;border-bottom:4px solid transparent;margin-right:10px}}.teacher-tools[open] > summary::before {{transform:rotate(90deg)}}.teacher-tools[open] > summary {{margin-bottom:12px}}.teacher-tools .teacher-start:not([open]) {{padding:16px 24px}}.teacher-context-action {{margin:8px 0}}"}
        for tools_key in [response_key.clone()] {details {key:"tools-{tools_key}",class:"teacher-tools",open:answer.is_none(),
            summary {"Lessons & history"}
            {management.clone()}
        }}
        if let Err(problem)=selected_document {p {role:"alert","{problem}"}}
        if reviewing_history && !attempts.is_empty() {
            section {class:"panel",
                h2 {"Work with your answer"}
                label {r#for:"teacher-attempt","Saved practice"}
                select {id:"teacher-attempt",value:selected_id.map(|id|id.to_string()).unwrap_or_default(),disabled:state.teacher_busy,onchange:move |e|selected.set(Uuid::parse_str(&e.value()).ok()),
                    for (event,attempt) in attempts.iter().take(80) {option {value:event.id.to_string(),"{attempt.skill.label()} · "{state.library.revision(attempt.revision).map(|r|r.passage.title.as_str()).unwrap_or("Saved passage")}}}
                }
                if let Some((id,attempt))=current {
                    if attempt.skill!=Skill::Speaking {blockquote {"{attempt.response}"}}
                    if attempt.skill==Skill::Speaking {
                        for (recording_event,recording) in state.library.recordings(AnnotationTarget::Attempt(id)).into_iter().filter(|(_,a)|a.role==AudioRole::Attempt) {
                            div {key:"take-{recording_event.id}",class:"spoken-take",
                                AudioReview {asset_id:recording.asset_id.clone()}
                                crate::recognition_view::SpokenWords {app,attempt:id,recording:recording_event.id}
                            }
                        }
                    } else {
                        button {class:"primary",disabled:state.teacher_busy,onclick:move |_|{let request=app.read().answer_feedback_request(id);send_teacher(app,request);},"Help me improve this answer →"}
                    }
                    for dimension in [JudgmentDimension::Comprehension,JudgmentDimension::Grammar,JudgmentDimension::Spelling,JudgmentDimension::Recall] {
                        for (_,feedback) in state.library.ai_assessments(id,dimension) {
                            details {class:"teacher-saved-feedback",summary {"Saved teacher feedback · {feedback.provenance.model}"}
                                TeacherProse {app,language:"en",text:feedback.explanation.clone(),purpose:PresentationPurpose::Instruction}
                                if let Some(text)=&feedback.suggested_response {PresentedRussian {key:"{text}",app,text:text.clone(),purpose:PresentationPurpose::Reference}}
                                TeacherProse {app,language:"en",text:feedback.next_step.clone(),purpose:PresentationPurpose::Instruction}
                            }
                        }
                    }
                }
            }
        }
        if !reviewing_history {
            if return_review {div {class:"actions teacher-return",button {class:"primary",disabled:state.teacher_busy || state.speech_busy || capture_busy(),onclick:move |_|act(app,|state|state.navigate(Page::Practice)),"Return to this saved review →"}}}
            if let Some((document,index))=return_lesson {
                div {class:"actions teacher-return",button {class:"secondary",disabled:state.teacher_busy || state.speech_busy || capture_busy(),onclick:move |_|act(app,|state| {
                    let answer=state.library.teacher_document(document).ok_or("The original lesson is unavailable. Sync it, then try again.")?.response.clone();
                    state.set_teacher_lesson_cursor(document,step_for_practice(&answer,index))?;
                    state.select_teacher_document(document)
                }),"Return to this lesson & exercise →"}}
            }
        }
        if !reviewing_history {if let Some(answer)=answer.as_ref() {
            section {class:"panel teacher-answer",aria_label:"Your teacher’s response",
                span {class:"tag","AI TEACHER"}
                {let title=match &answer.request {TeacherRequest::TeachLesson(r)=>r.title.clone(),TeacherRequest::AnswerFeedback(_)=>"Working with your response".into()};rsx! {PresentedRussian {app,text:title.clone(),language:"en",purpose:PresentationPurpose::Instruction,h2 {"{title}"}}}}
                if let Some(document)=feedback_document {div {class:"teacher-context-action",button {class:"text-button",disabled:state.teacher_busy || state.speech_busy || capture_busy(),onclick:move |_|act(app,|state|state.select_teacher_document(document)),"Read the latest feedback for this lesson"}}}
                if matches!(answer.request,TeacherRequest::TeachLesson(_)) {
                    TeacherLessonView {key:"guided-{answer.provenance.response_id}",app,answer:answer.clone(),active_speaking,capture_busy}
                } else {
                crate::teacher_feedback_view::TeacherFeedback {app,answer:answer.clone(),active_speaking,capture_busy}

                }
                if !answer.content.caveats.is_empty() {details {summary {"Nuance and uncertainty"}ul {for caveat in &answer.content.caveats {li {TeacherProse {app,language:"en",text:caveat.clone(),purpose:PresentationPurpose::Instruction}}}}}}
                p {class:"quiet","{answer.provenance.model} · AI-generated teaching"}
                if let Some(prepared)=prepared_context {
                    details {class:"teacher-context-preview",
                        summary {"Practice used in this explanation"}
                        PresentedRussian {app,text:prepared.clone(),language:"en",purpose:PresentationPurpose::Instruction,pre {class:"teacher-context-text","{prepared}"}}
                    }
                }
            }
        }}
        if let Some(earlier)=free_answer {details {class:"panel",summary {"Earlier one-off answer"}
            p {class:"quiet","The original question was not saved with this older answer. It remains here separately from your conversations."}
            TeacherProse {app,language:"en",text:earlier.content.explanation.clone(),purpose:PresentationPurpose::Instruction}
            for example in &earlier.content.examples {PresentedRussian {app,text:example.russian.clone(),purpose:PresentationPurpose::Reference}TeacherProse {app,language:"en",text:example.english.clone(),purpose:PresentationPurpose::Reference}TeacherProse {app,language:"en",text:example.note.clone(),purpose:PresentationPurpose::Instruction}}
            TeacherProse {app,language:"en",text:earlier.content.next_step.clone(),purpose:PresentationPurpose::Instruction}
            for caveat in &earlier.content.caveats {TeacherProse {app,language:"en",text:caveat.clone(),purpose:PresentationPurpose::Instruction}}
            p {class:"quiet","{earlier.provenance.model} · Earlier AI suggestion"}
        }}
        details {class:"panel",summary {"Optional notes from a conversation"}crate::coach::ConversationNotes {app}}
    }
}

pub(crate) fn send_teacher(
    mut app: Signal<AppState>,
    request: crate::repository::Result<TeacherRequest>,
) {
    if app.read().teacher_busy {
        return;
    }
    let request = match request {
        Ok(request) => request,
        Err(problem) => {
            app.write().notice = problem;
            return;
        }
    };
    let snapshot = app.read().clone();
    if let Err(problem) = snapshot.repo.set("teacher_request", &request) {
        app.write().notice = problem;
        return;
    }
    app.write().teacher_busy = true;
    dioxus_core::spawn_forever(async move {
        let result = async {
            let body = serde_json::to_value(&request).map_err(|e| e.to_string())?;
            gateway::request::<TeacherResponse>(
                &snapshot.endpoint,
                &snapshot.token,
                "/api/teacher/teach",
                Some(&body),
                128 * 1024,
                120_000,
            )
            .await
        }
        .await;
        app.write().teacher_busy = false;
        match result {
            Ok(response) => act(app, |s| s.keep_teacher_response(&request, &response)),
            Err(problem) => app.write().notice = problem,
        }
    });
}
