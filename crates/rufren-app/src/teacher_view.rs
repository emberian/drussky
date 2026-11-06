use crate::{
    act,
    audio_studio::AudioReview,
    exposure_view::PresentedRussian,
    gateway,
    model::{AppState, Page},
    teacher_example_audio::TeacherExampleAudio,
    teacher_practice_view::TeacherPractice as TeacherPracticeForm,
    teacher_prose::TeacherProse,
    tutor_view::TutorView,
};
use dioxus::prelude::*;
use rufren_core::*;
use rufren_tutor::*;
use uuid::Uuid;

#[component]
pub(crate) fn Teacher(mut app: Signal<AppState>) -> Element {
    let mut returned_from = use_signal(|| None::<String>);
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
    let latest_answer = state
        .repo
        .get::<TeacherResponse>("teacher_response")
        .ok()
        .flatten();
    let saved_lesson = state
        .repo
        .get::<TeacherResponse>("teacher_lesson_response")
        .ok()
        .flatten()
        .filter(|response| matches!(response.request, TeacherRequest::TeachLesson(_)));
    let response_key = latest_answer
        .as_ref()
        .map(|response| response.provenance.response_id.clone())
        .unwrap_or_else(|| "no-teacher-response".into());
    let reviewing_history = history_for.read().as_deref() == Some(response_key.as_str());
    let feedback_id = latest_answer
        .as_ref()
        .filter(|response| matches!(response.request, TeacherRequest::AnswerFeedback(_)))
        .map(|response| response.provenance.response_id.clone());
    let showing_lesson = feedback_id
        .as_ref()
        .is_some_and(|id| returned_from.read().as_ref() == Some(id))
        && saved_lesson.is_some();
    let answer = if showing_lesson {
        saved_lesson.clone()
    } else {
        latest_answer.clone()
    };
    let lesson = state.lesson().or_else(|| state.content.lessons.first());
    let recent_practice = state.lesson_practice_summary().ok();
    let prepared_context = answer.as_ref().and_then(|answer| match &answer.request {
        TeacherRequest::TeachLesson(request) => state
            .prepared_lesson_context(&request.request_id)
            .ok()
            .flatten(),
        _ => None,
    });
    let free_answer = state
        .repo
        .get::<TutorResponse>("tutor_response")
        .ok()
        .flatten()
        .and_then(|r| serde_json::to_string(&r).ok());
    rsx! {
        div {class:"page-heading",div {div {class:"eyebrow","EXPLAIN · TRY · UNDERSTAND"}h1 {"Your Russian teacher"}p {"Learn one useful thing, try it, and work through what feels unfamiliar."}}}
        section {class:"panel teacher-start",
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
                    pre {class:"teacher-context-text","{recent.provider_context()}"}
                }
            }
            div {class:"actions",button {class:"primary",disabled:state.teacher_busy || capture_busy(),onclick:move |_|{let request=app.read().lesson_teaching_request();send_teacher(app,request);},if state.teacher_busy {"Your teacher is thinking…"}else{"Teach me this lesson →"}}
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
                button {class:"secondary",aria_expanded:reviewing_history,disabled:capture_busy(),onclick:move |_|history_for.set(if reviewing_history {None}else{Some(response_key.clone())}),
                    if reviewing_history {"Return to current teaching"}else{"Review saved practice"}
                }
            }
        }
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
        if !reviewing_history {if let Some(id)=feedback_id {
            if saved_lesson.is_some() {
                div {class:"actions teacher-return",
                    if showing_lesson {button {class:"secondary",disabled:capture_busy(),onclick:move |_|returned_from.set(None),"Read the latest feedback"}}
                    else {button {class:"secondary",disabled:capture_busy(),onclick:move |_|returned_from.set(Some(id.clone())),"Return to your lesson & exercises →"}}
                }
            }
        }}
        if !reviewing_history {if let Some(answer)=answer {
            section {class:"panel teacher-answer",aria_label:"Your teacher’s response",
                span {class:"tag","AI TEACHER"}
                h2 {match &answer.request {TeacherRequest::TeachLesson(r)=>r.title.clone(),TeacherRequest::AnswerFeedback(_)=>"Working with your response".into()}}
                TeacherProse {key:"explanation-{answer.provenance.response_id}",app,class:"teacher-explanation",language:"en",text:answer.content.explanation.clone(),purpose:PresentationPurpose::Instruction}
                if let Some(feedback)=&answer.content.feedback {
                    p {strong {match feedback.verdict {AiVerdict::Correct=>"This works for the task.",AiVerdict::PartlyCorrect=>"Part of this works; let’s refine it.",AiVerdict::Incorrect=>"There’s something useful to change.",AiVerdict::Uncertain=>"There may be more than one interpretation here."}}}
                    if let Some(text)=&feedback.suggested_response {PresentedRussian {key:"{text}",app,class:"dialogue-russian",text:text.clone(),purpose:PresentationPurpose::Reference}}
                    for (index,correction) in feedback.corrections.iter().enumerate() {div {key:"correction-{answer.provenance.response_id}-{index}",class:"teacher-correction",PresentedRussian {app,text:format!("{} → {}",correction.original,correction.replacement),purpose:PresentationPurpose::Instruction,p {span {lang:"ru","{correction.original}"}" → " strong {lang:"ru","{correction.replacement}"}}}TeacherProse {app,language:"en",text:correction.reason.clone(),purpose:PresentationPurpose::Instruction}}}
                }
                for (index,example) in answer.content.examples.iter().enumerate() {{let example=example.clone();rsx! {article {key:"example-{answer.provenance.response_id}-{index}",class:"phrase-study",
                    PresentedRussian {app,class:"dialogue-russian",text:example.russian.clone(),purpose:PresentationPurpose::Instruction}
                    p {"{example.english}"}
                    TeacherProse {app,class:"quiet",language:"en",text:example.note.clone(),purpose:PresentationPurpose::Instruction}
                    TeacherExampleAudio {app,answer:answer.clone(),index}
                    button {class:"text-button",onclick:move |_|act(app,|s|s.keep_teacher_example(example.clone())),"Keep this phrase for practice →"}
                }}}}
                if !answer.content.practice.is_empty() {h3 {"A small thing to try"}
                    for (index,_) in answer.content.practice.iter().enumerate() {
                        TeacherPracticeForm {key:"{answer.provenance.response_id}-{index}",app,answer:answer.clone(),index,active_speaking,capture_busy}
                    }
                }
                div {class:"hint",span {class:"tag","YOUR NEXT STEP"}TeacherProse {app,language:"en",text:answer.content.next_step.clone(),purpose:PresentationPurpose::Instruction}}
                if !answer.content.caveats.is_empty() {details {summary {"Nuance and uncertainty"}ul {for caveat in &answer.content.caveats {li {TeacherProse {app,language:"en",text:caveat.clone(),purpose:PresentationPurpose::Instruction}}}}}}
                p {class:"quiet","{answer.provenance.model} · AI-generated teaching"}
                if let Some(prepared)=prepared_context {
                    details {class:"teacher-context-preview",
                        summary {"Practice used in this explanation"}
                        pre {class:"teacher-context-text","{prepared.summary.provider_context()}"}
                    }
                }
            }
        }}
        details {class:"panel",summary {"Ask a different question"}
            TutorView {app,endpoint:state.endpoint.clone(),token:state.token.clone(),restored_response_json:free_answer,
                on_response:move |response|act(app,|s|s.repo.set("tutor_response",&response)),
                on_add:move |example: TutorExample|act(app,|s|s.keep_teacher_example(example))}
        }
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
