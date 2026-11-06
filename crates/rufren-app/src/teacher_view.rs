use crate::{
    act,
    audio_studio::AudioReview,
    exposure_view::PresentedRussian,
    gateway,
    model::{AppState, Page},
    teacher_practice_view::TeacherPractice as TeacherPracticeForm,
    tutor_view::TutorView,
};
use dioxus::prelude::*;
use rufren_core::*;
use rufren_tutor::*;
use uuid::Uuid;

#[component]
pub(crate) fn Teacher(mut app: Signal<AppState>) -> Element {
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
    let answer = state
        .repo
        .get::<TeacherResponse>("teacher_response")
        .ok()
        .flatten();
    let lesson = state.lesson().or_else(|| state.content.lessons.first());
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
            div {class:"actions",button {class:"primary",disabled:state.teacher_busy,onclick:move |_|{let request=app.read().lesson_teaching_request();send_teacher(app,request);},if state.teacher_busy {"Your teacher is thinking…"}else{"Teach me this lesson →"}}
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
            section {class:"panel",
                h2 {"Work with your answer"}
                label {r#for:"teacher-attempt","Saved practice"}
                select {id:"teacher-attempt",value:selected_id.map(|id|id.to_string()).unwrap_or_default(),disabled:state.teacher_busy,onchange:move |e|selected.set(Uuid::parse_str(&e.value()).ok()),
                    for (event,attempt) in attempts.iter().take(80) {option {value:event.id.to_string(),"{attempt.skill.label()} · "{state.library.revision(attempt.revision).map(|r|r.passage.title.as_str()).unwrap_or("Saved passage")}}}
                }
                if let Some((id,attempt))=current {
                    blockquote {"{attempt.response}"}
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
                            details {class:"teacher-saved-feedback",summary {"Saved teacher feedback · {feedback.provenance.model}"}p {"{feedback.explanation}"}if let Some(text)=&feedback.suggested_response {p {lang:"ru","{text}"}}p {"{feedback.next_step}"}}
                        }
                    }
                }
            }
        }
        if let Some(answer)=answer {
            section {class:"panel teacher-answer",aria_label:"Your teacher’s response",
                span {class:"tag","AI TEACHER"}
                h2 {match &answer.request {TeacherRequest::TeachLesson(r)=>r.title.clone(),TeacherRequest::AnswerFeedback(_)=>"Working with your response".into()}}
                PresentedRussian {key:"explanation-{answer.provenance.response_id}",app,class:"teacher-explanation",language:"en",text:answer.content.explanation.clone(),purpose:PresentationPurpose::Instruction}
                if let Some(feedback)=&answer.content.feedback {
                    p {strong {match feedback.verdict {AiVerdict::Correct=>"This works for the task.",AiVerdict::PartlyCorrect=>"Part of this works; let’s refine it.",AiVerdict::Incorrect=>"There’s something useful to change.",AiVerdict::Uncertain=>"There may be more than one interpretation here."}}}
                    if let Some(text)=&feedback.suggested_response {p {class:"dialogue-russian",lang:"ru","{text}"}}
                    for correction in &feedback.corrections {div {class:"teacher-correction",p {span {lang:"ru","{correction.original}"}" → " strong {lang:"ru","{correction.replacement}"}}p {"{correction.reason}"}}}
                }
                for (index,example) in answer.content.examples.iter().enumerate() {{let example=example.clone();rsx! {article {class:"phrase-study",PresentedRussian {key:"example-{answer.provenance.response_id}-{index}",app,class:"dialogue-russian",text:example.russian.clone(),purpose:PresentationPurpose::Instruction}p {"{example.english}"}p {class:"quiet","{example.note}"}button {class:"text-button",onclick:move |_|act(app,|s|s.keep_teacher_example(example.clone())),"Keep this phrase for practice →"}}}}}
                if !answer.content.practice.is_empty() {h3 {"A small thing to try"}
                    for (index,_) in answer.content.practice.iter().enumerate() {
                        TeacherPracticeForm {key:"{answer.provenance.response_id}-{index}",app,answer:answer.clone(),index}
                    }
                }
                div {class:"hint",span {class:"tag","YOUR NEXT STEP"}p {"{answer.content.next_step}"}}
                if !answer.content.caveats.is_empty() {details {summary {"Nuance and uncertainty"}ul {for caveat in &answer.content.caveats {li {"{caveat}"}}}}}
                p {class:"quiet","{answer.provenance.model} · AI-generated teaching"}
            }
        }
        details {class:"panel",summary {"Ask a different question"}
            TutorView {endpoint:state.endpoint.clone(),token:state.token.clone(),restored_response_json:free_answer,
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
