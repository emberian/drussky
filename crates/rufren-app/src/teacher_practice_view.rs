use crate::exposure_view::PresentedRussian;
use crate::{model::AppState, teacher_prose::TeacherProse, teacher_view::send_teacher};
use dioxus::prelude::*;
use rufren_core::{Cue, PresentationPurpose, SelfRating};
use rufren_tutor::{TeacherResponse, TeacherSkill};

#[component]
pub(crate) fn TeacherPractice(
    app: Signal<AppState>,
    answer: TeacherResponse,
    index: usize,
    active_speaking: Signal<Option<String>>,
    capture_busy: Signal<bool>,
    #[props(default)] review: bool,
) -> Element {
    if answer.content.practice.get(index).is_some_and(|exercise| {
        matches!(
            exercise.skill,
            Some(TeacherSkill::Listening | TeacherSkill::Speaking)
        )
    }) {
        rsx! {crate::teacher_audio_practice_view::TeacherAudioPractice {app,answer,index,active_speaking,capture_busy,review}}
    } else {
        rsx! {TextTeacherPractice {app,answer,index,review}}
    }
}

#[component]
fn TextTeacherPractice(
    mut app: Signal<AppState>,
    answer: TeacherResponse,
    index: usize,
    review: bool,
) -> Element {
    let initial = use_hook(|| {
        if review {
            app.read().teacher_revisit_draft(&answer, index)
        } else {
            app.read().teacher_practice_draft(&answer, index)
        }
    });
    let mut revealed = use_signal(|| false);
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
    let saved = review && snapshot.saved_attempt.is_some();
    let rating = snapshot
        .review
        .as_ref()
        .map_or(SelfRating::NotReported, |review| review.self_rating);
    let answer_for_submit = answer.clone();
    let answer_for_edit = answer.clone();
    rsx! {
        article {class:"phrase-study teacher-exercise", "data-teacher-review":review.to_string(),"data-review-session":if review {snapshot.id.to_string()}else{String::new()},
            TeacherProse {app,language:"en",text:exercise.prompt.clone(),purpose:PresentationPurpose::TaskInput,session:Some(snapshot.id)}
            if exercise.skill==Some(TeacherSkill::Reading) {
                if let Some(text)=&exercise.reference_russian {PresentedRussian {app,class:"dialogue-russian",text:text.clone(),purpose:PresentationPurpose::TaskInput,session:Some(snapshot.id)}}
            }
            if editable {
                label {r#for:field.clone(), "Your response"}
                textarea {id:field, value:snapshot.response.clone(), disabled:(busy && !review) || saved, placeholder:"Try a short answer. Your draft is kept.",
                    oninput:move |event| {
                        let Some(mut next)=draft.read().clone() else{return};
                        let result=app.read().teacher_practice_edit_response(&answer_for_edit,&mut next,event.value());
                        match result {Ok(())=>draft.set(Some(next)),Err(error)=>app.write().notice=error}
                    }
                }
                if review {crate::teacher_revisit_view::ReviewSelfReport {value:rating,disabled:saved,on_change:move |rating| {
                    let Some(mut next)=draft.read().clone() else{return};
                    let result=app.read().set_teacher_revisit_rating(&mut next,rating);
                    match result {Ok(())=>draft.set(Some(next)),Err(error)=>app.write().notice=error}
                }}}
                if !saved {button {class:"primary", disabled:busy || snapshot.response.trim().is_empty(),
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
                    if busy {"Your teacher is thinking…"}else if review {"Save & discuss this return →"}else{"Discuss my answer →"}
                }} else {p {class:"quiet",role:"status","Your response is saved. You can finish here even if the teacher is unavailable."}}
            }
            if review {
                button {class:"text-button",aria_expanded:revealed(),onclick:move |_| {
                    if revealed(){revealed.set(false);return;}
                    if !saved {
                        let Some(mut next)=draft.read().clone() else{return};
                        let result=app.write().teacher_revisit_reveal(&mut next,Cue::ReferenceMaterial);
                        match result {Ok(())=>draft.set(Some(next)),Err(error)=>{app.write().notice=error;return;}}
                    }
                    revealed.set(true);
                },"Show an example and explanation"}
                if revealed() {div {class:"hint teacher-revisit-model",
                    if let Some(example)=&exercise.expected_response {PresentedRussian {app,text:example.clone(),purpose:PresentationPurpose::Answer,session:Some(snapshot.id)}}
                    TeacherProse {app,language:"en",text:exercise.teaching_note.clone(),purpose:PresentationPurpose::Instruction,session:Some(snapshot.id)}
                }}
            } else {details {summary {"Show an example and explanation"}
                if let Some(example)=&exercise.expected_response {PresentedRussian {app,text:example.clone(),purpose:PresentationPurpose::Answer,session:Some(snapshot.id)}}
                TeacherProse {app,language:"en",text:exercise.teaching_note.clone(),purpose:PresentationPurpose::Instruction,session:Some(snapshot.id)}
            }}
        }
    }
}
