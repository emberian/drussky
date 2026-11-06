//! Shared language-content feedback, independent of the lesson or revisit navigation.
use crate::{
    exposure_view::PresentedRussian, model::AppState, teacher_example_view::TeacherExampleView,
    teacher_practice_view::TeacherPractice as TeacherPracticeForm, teacher_prose::TeacherProse,
};
use dioxus::prelude::*;
use rufren_core::*;
use rufren_tutor::{AiVerdict, TeacherResponse};

#[component]
pub(crate) fn TeacherFeedback(
    app: Signal<AppState>,
    answer: TeacherResponse,
    active_speaking: Signal<Option<String>>,
    capture_busy: Signal<bool>,
    #[props(default = true)] show_practice: bool,
) -> Element {
    rsx! {
                TeacherProse {key:"explanation-{answer.provenance.response_id}",app,class:"teacher-explanation",language:"en",text:answer.content.explanation.clone(),purpose:PresentationPurpose::Instruction}
                if let Some(feedback)=&answer.content.feedback {
                    p {strong {match feedback.verdict {AiVerdict::Correct=>"This works for the task.",AiVerdict::PartlyCorrect=>"Part of this works; let’s refine it.",AiVerdict::Incorrect=>"There’s something useful to change.",AiVerdict::Uncertain=>"There may be more than one interpretation here."}}}
                    if let Some(text)=&feedback.suggested_response {PresentedRussian {key:"{text}",app,class:"dialogue-russian",text:text.clone(),purpose:PresentationPurpose::Reference}}
                    for (index,correction) in feedback.corrections.iter().enumerate() {div {key:"correction-{answer.provenance.response_id}-{index}",class:"teacher-correction",PresentedRussian {app,text:format!("{} → {}",correction.original,correction.replacement),purpose:PresentationPurpose::Instruction,p {span {lang:"ru","{correction.original}"}" → " strong {lang:"ru","{correction.replacement}"}}}TeacherProse {app,language:"en",text:correction.reason.clone(),purpose:PresentationPurpose::Instruction}}}
                }
                for (index,_) in answer.content.examples.iter().enumerate() {TeacherExampleView {key:"example-{answer.provenance.response_id}-{index}",app,answer:answer.clone(),index}}
                if show_practice && !answer.content.practice.is_empty() {h3 {"A small thing to try"}
                    for (index,_) in answer.content.practice.iter().enumerate() {
                        TeacherPracticeForm {key:"{answer.provenance.response_id}-{index}",app,answer:answer.clone(),index,active_speaking,capture_busy}
                    }
                }
                div {class:"hint",span {class:"tag","YOUR NEXT STEP"}TeacherProse {app,language:"en",text:answer.content.next_step.clone(),purpose:PresentationPurpose::Instruction}}
    }
}
