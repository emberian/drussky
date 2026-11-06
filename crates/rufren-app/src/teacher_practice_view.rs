use crate::exposure_view::PresentedRussian;
use crate::{model::AppState, teacher_view::send_teacher};
use dioxus::prelude::*;
use rufren_core::PresentationPurpose;
use rufren_tutor::{TeacherResponse, TeacherSkill};
use uuid::Uuid;

#[component]
pub(crate) fn TeacherPractice(
    mut app: Signal<AppState>,
    answer: TeacherResponse,
    index: usize,
) -> Element {
    let initial = use_hook(|| app.read().teacher_practice_draft(&answer, index));
    let mut draft = use_signal(|| initial.clone().ok());
    let Some(snapshot) = draft.read().clone() else {
        let problem = initial.as_ref().err().cloned().unwrap_or_default();
        return rsx! {p {role:"alert", "{problem}"}};
    };
    let Some(exercise) = answer.content.practice.get(index) else {
        return rsx! {};
    };
    let editable = exercise.reference_russian.is_some()
        && exercise.reference_english.is_some()
        && matches!(
            exercise.skill,
            Some(TeacherSkill::Reading | TeacherSkill::Writing)
        );
    let field = format!("teacher-practice-{}-{index}", snapshot.source);
    let busy = app.read().teacher_busy;
    let answer_for_submit = answer.clone();
    rsx! {
        article {class:"phrase-study teacher-exercise",
            p {"{exercise.prompt}"}
            if exercise.skill==Some(TeacherSkill::Reading) {
                if let Some(text)=&exercise.reference_russian {PresentedRussian {app,class:"dialogue-russian",text:text.clone(),purpose:PresentationPurpose::TaskInput,session:Some(snapshot.id)}}
            }
            if editable {
                label {r#for:field.clone(), "Your response"}
                textarea {id:field, value:snapshot.response.clone(), disabled:busy, placeholder:"Try a short answer. Your draft is kept.",
                    oninput:move |event| {
                        let Some(mut next)=draft.read().clone() else{return};
                        if next.saved_attempt.take().is_some() {next.id=Uuid::new_v4();}
                        next.response=event.value();
                        let result=app.read().save_teacher_practice_draft(&next);
                        match result {Ok(())=>draft.set(Some(next)),Err(error)=>app.write().notice=error}
                    }
                }
                button {class:"primary", disabled:busy || snapshot.response.trim().is_empty(),
                    onclick:move |_| {
                        let Some(mut next)=draft.read().clone() else{return};
                        let result=app.write().submit_teacher_practice(&answer_for_submit,&mut next);
                        match result {
                            Ok(id)=>{
                                draft.set(Some(next));
                                let request=app.read().answer_feedback_request(id);
                                send_teacher(app,request);
                            },
                            Err(error)=>app.write().notice=error,
                        }
                    },
                    if busy {"Your teacher is thinking…"}else{"Discuss my answer →"}
                }
            }
            details {summary {"Show an example and explanation"}
                if let Some(example)=&exercise.expected_response {PresentedRussian {app,text:example.clone(),purpose:PresentationPurpose::Answer,session:Some(snapshot.id)}}
                p {"{exercise.teaching_note}"}
            }
        }
    }
}
