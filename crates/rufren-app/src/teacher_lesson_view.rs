//! A saved teacher lesson is navigated one step at a time; navigation is not a grade.
use crate::{
    model::{AppState, Page},
    teacher_example_view::TeacherExampleView,
    teacher_practice::saved_teacher_practice_origin,
    teacher_practice_view::TeacherPractice,
    teacher_prose::TeacherProse,
};
use dioxus::prelude::*;
use rufren_core::{PresentationPurpose, teacher_document_id};
use rufren_tutor::{ExamplePresentation, TeacherRequest, TeacherResponse};
use uuid::Uuid;

const STYLE: &str = include_str!("teacher_lesson_view.css");

#[derive(Clone, PartialEq)]
struct LessonStep {
    id: String,
    goal: String,
    orientation: String,
    detail: Option<String>,
    models: Vec<usize>,
    support: Vec<usize>,
    practice: Option<usize>,
}

fn lesson_steps(answer: &TeacherResponse) -> Vec<LessonStep> {
    if let Some(flow) = &answer.content.lesson_flow {
        return flow
            .steps
            .iter()
            .map(|step| LessonStep {
                id: step.id.clone(),
                goal: step.goal.clone(),
                orientation: step.orientation.clone(),
                detail: step.detail.clone(),
                models: step
                    .examples
                    .iter()
                    .filter(|example| example.presentation == ExamplePresentation::Model)
                    .map(|example| example.example_index)
                    .collect(),
                support: step
                    .examples
                    .iter()
                    .filter(|example| example.presentation == ExamplePresentation::OptionalSupport)
                    .map(|example| example.example_index)
                    .collect(),
                practice: step.practice_index,
            })
            .collect();
    }
    // Old responses did not specify a sequence. Keep their original practice order,
    // and leave the complete explanation intact behind an explicit disclosure.
    let orientation = match &answer.request {
        TeacherRequest::TeachLesson(request) => request.objective.clone(),
        TeacherRequest::AnswerFeedback(_) => String::new(),
    };
    let mut steps = vec![LessonStep {
        id: "orientation".into(),
        goal: "Start here".into(),
        orientation,
        detail: None,
        models: if answer.content.examples.is_empty() {
            vec![]
        } else {
            vec![0]
        },
        support: vec![],
        practice: None,
    }];
    steps.extend(
        answer
            .content
            .practice
            .iter()
            .enumerate()
            .map(|(index, _)| LessonStep {
                id: format!("practice-{index}"),
                goal: format!("Practice {}", index + 1),
                orientation: String::new(),
                detail: None,
                models: vec![],
                support: vec![],
                practice: Some(index),
            }),
    );
    steps
}

#[component]
pub(crate) fn TeacherLessonView(
    app: Signal<AppState>,
    answer: TeacherResponse,
    active_speaking: Signal<Option<String>>,
    capture_busy: Signal<bool>,
) -> Element {
    let document = teacher_document_id(&answer);
    // A changed immutable response must not inherit another response's cursor or form.
    rsx! {for id in [document] {
        LessonCursor {key:"lesson-{id}",app,answer:answer.clone(),active_speaking,capture_busy}
    }}
}

#[component]
fn LessonCursor(
    app: Signal<AppState>,
    answer: TeacherResponse,
    active_speaking: Signal<Option<String>>,
    capture_busy: Signal<bool>,
) -> Element {
    let document = teacher_document_id(&answer);
    let steps = lesson_steps(&answer);
    let count = steps.len();
    let initial = use_hook(|| {
        let state = app.read();
        if state.library.teacher_document(document).is_none() {
            // Old injected/cached responses can be read without pretending a place was saved.
            Ok(0)
        } else {
            state.teacher_lesson_cursor(document)
        }
    });
    let cursor = use_signal(|| initial);
    let problem = use_signal(String::new);
    let mut focus_request = use_signal(|| None::<usize>);
    use_effect(move || {
        let Some(target) = focus_request() else {
            return;
        };
        focus_request.set(None);
        let document = document.to_string();
        spawn(async move {
            let script = format!(
                "await new Promise(requestAnimationFrame);const root=[...document.querySelectorAll('.teacher-lesson-flow')].find(e=>e.dataset.lessonDocument==={}&&e.dataset.lessonStep==={});const heading=root?.querySelector('.teacher-lesson-goal, .teacher-lesson-finish h3');if(heading){{heading.focus({{preventScroll:true}});heading.scrollIntoView({{block:'start'}});}}",
                serde_json::to_string(&document).unwrap_or_default(),
                serde_json::to_string(&target.to_string()).unwrap_or_default()
            );
            let _ = document::eval(&script).await;
        });
    });
    let position = cursor.read().as_ref().ok().copied();
    let current = position.and_then(|index| steps.get(index));
    let state = app.read();
    let busy = state.teacher_busy || state.speech_busy || capture_busy();
    let saved = state
        .library
        .attempts()
        .filter(|(event, _)| {
            state.library.contexts(event.id).iter().any(|(context, _)| {
                saved_teacher_practice_origin(&state.library, event.id, context.id).is_some_and(
                    |origin| {
                        origin.source == document && origin.index < answer.content.practice.len()
                    },
                )
            })
        })
        .count();
    let missing_archive = state.library.teacher_document(document).is_none();
    drop(state);
    let jump_id = use_hook(|| format!("teacher-lesson-jump-{}", Uuid::new_v4()));
    rsx! {
        style {"{STYLE}"}
        div {class:"teacher-lesson-flow","data-lesson-document":document.to_string(),"data-lesson-step":position.map(|i|i.to_string()).unwrap_or_default(),
            div {class:"teacher-lesson-position",
                p {class:"quiet",role:"status",aria_live:"polite",match position {
                    Some(index) if index<count=>format!("Step {} of {count}",index+1),
                    Some(_)=>"Finished for now".into(),None=>"Your saved place needs attention".into(),
                }}
                button {class:"text-button",disabled:busy,onclick:move |_|change_place(app,cursor,problem,active_speaking,capture_busy,document,count,position,true,focus_request),"Pause & return to Today"}
            }
            if let Err(error)=&*cursor.read() {p {class:"teacher-lesson-error",role:"alert","{error} Choose a step to continue; your saved place has not been reset."}}
            if !problem.read().is_empty() {p {class:"teacher-lesson-error",role:"alert","{problem}"}}
            if let Some(step)=current {
                for step in [step.clone()] {
                    LessonStepView {key:"{document}-step-{step.id}",app,answer:answer.clone(),step,active_speaking,capture_busy,busy}
                }
            } else if position==Some(count) {
                section {class:"teacher-lesson-finish",
                    h3 {tabindex:"-1","Finished for now"}
                    p {"Your place is saved. Return to Today or revisit any step."}
                    p {class:"quiet",if saved==0 {"No responses saved for this lesson yet."}else{{format!("{saved} {} saved for this lesson.",if saved==1 {"response"}else{"responses"})}}}
                }
            }
            nav {class:"teacher-lesson-navigation",aria_label:"Lesson steps",
                div {class:"actions",
                    button {class:"secondary",disabled:busy||position.is_none_or(|i|i==0),onclick:move |_|change_place(app,cursor,problem,active_speaking,capture_busy,document,count,position.map(|i|i.saturating_sub(1)),false,focus_request),"← Back"}
                    button {class:if current.is_some_and(|step|step.practice.is_some()) {"secondary"}else{"primary"},disabled:busy||position.is_none_or(|i|i>=count),onclick:move |_|change_place(app,cursor,problem,active_speaking,capture_busy,document,count,position.map(|i|(i+1).min(count)),false,focus_request),if position==Some(count.saturating_sub(1)) {"Finish for now →"}else{"Next step →"}}
                }
                div {class:"teacher-lesson-jump",
                    label {r#for:jump_id.clone(),"Go to a step"}
                    select {id:jump_id,value:position.map(|i|i.to_string()).unwrap_or_default(),disabled:busy,onchange:move |event|change_place(app,cursor,problem,active_speaking,capture_busy,document,count,event.value().parse().ok(),false,focus_request),
                        if position.is_none() {option {value:"",disabled:true,"Choose a step…"}}
                        for (index,step) in steps.iter().enumerate() {option {value:index.to_string(),{format!("{}. {}",index+1,step.goal)}}}
                        option {value:count.to_string(),"Finished for now"}
                    }
                }
            }
            if missing_archive {p {class:"quiet","This lesson is readable here. Saving a place requires its original in your teacher notebook."}}
            for source_key in [format!("original-{document}-{:?}",position)] {LessonDisclosure {key:"{source_key}",title:"Read the full saved lesson".to_string(),busy,
                TeacherProse {app,class:"teacher-explanation",language:"en",text:answer.content.explanation.clone(),purpose:PresentationPurpose::Instruction}
                for (index,_) in answer.content.examples.iter().enumerate() {TeacherExampleView {key:"original-{document}-example-{index}",app,answer:answer.clone(),index}}
                div {class:"hint",span {class:"tag","YOUR NEXT STEP"}TeacherProse {app,language:"en",text:answer.content.next_step.clone(),purpose:PresentationPurpose::Instruction}}
                if !answer.content.caveats.is_empty() {h4 {"Nuance and uncertainty"}ul {for (index,caveat) in answer.content.caveats.iter().enumerate() {li {key:"caveat-{index}",TeacherProse {app,language:"en",text:caveat.clone(),purpose:PresentationPurpose::Instruction}}}}}
            }}
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn change_place(
    mut app: Signal<AppState>,
    mut cursor: Signal<Result<usize, String>>,
    mut problem: Signal<String>,
    mut active_speaking: Signal<Option<String>>,
    capture_busy: Signal<bool>,
    document: Uuid,
    count: usize,
    target: Option<usize>,
    pause: bool,
    mut focus_request: Signal<Option<usize>>,
) {
    if capture_busy() || app.read().teacher_busy || app.read().speech_busy {
        return;
    }
    let result = (|| {
        if let Some(target) = target {
            if target > count {
                return Err("That step is unavailable. Your earlier place was kept.".into());
            }
            app.read().set_teacher_lesson_cursor(document, target)?;
            cursor.set(Ok(target));
            if !pause {
                focus_request.set(Some(target));
            }
        } else if !pause {
            return Err("Choose one of this lesson’s steps.".into());
        }
        active_speaking.set(None);
        if pause {
            app.write().navigate(Page::Today)?;
        }
        Ok::<_, String>(())
    })();
    match result {
        Ok(()) => problem.set(String::new()),
        Err(error) => problem.set(error),
    }
}

#[component]
fn LessonStepView(
    app: Signal<AppState>,
    answer: TeacherResponse,
    step: LessonStep,
    active_speaking: Signal<Option<String>>,
    capture_busy: Signal<bool>,
    busy: bool,
) -> Element {
    let document = teacher_document_id(&answer);
    rsx! {
        section {class:"teacher-lesson-current","data-step-id":step.id.clone(),
            div {class:"teacher-lesson-goal",role:"heading",aria_level:"3",tabindex:"-1",TeacherProse {app,language:"en",text:step.goal,purpose:PresentationPurpose::Instruction}}
            if !step.orientation.is_empty() {TeacherProse {app,language:"en",text:step.orientation,purpose:PresentationPurpose::Instruction}}
            if let Some(detail)=step.detail {LessonDisclosure {title:"Why".to_string(),busy,TeacherProse {app,language:"en",text:detail,purpose:PresentationPurpose::Instruction}}}
            for index in step.models {TeacherExampleView {key:"model-{document}-{index}",app,answer:answer.clone(),index}}
            if !step.support.is_empty() {LessonDisclosure {title:"See optional examples".to_string(),busy,
                for index in step.support {TeacherExampleView {key:"support-{document}-{index}",app,answer:answer.clone(),index,purpose:PresentationPurpose::Reference}}
            }}
            if let Some(index)=step.practice {TeacherPractice {key:"practice-{document}-{index}",app,answer,index,active_speaking,capture_busy}}
        }
    }
}

#[component]
fn LessonDisclosure(title: String, busy: bool, children: Element) -> Element {
    let mut open = use_signal(|| false);
    let content_id = use_hook(|| format!("teacher-lesson-detail-{}", Uuid::new_v4()));
    rsx! {
        section {class:"teacher-lesson-disclosure",
            button {class:"text-button",disabled:busy,aria_expanded:open(),aria_controls:content_id.clone(),onclick:move |_|open.toggle(),if open() {"▾ "}else{"▸ "}"{title}"}
            if open() {div {id:content_id.clone(),class:"teacher-lesson-detail",{children}}}
        }
    }
}
