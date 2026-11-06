//! One immutable teacher example, shared by guided steps and the saved original.
use crate::{
    act, exposure_view::PresentedRussian, model::AppState,
    teacher_example_audio::TeacherExampleAudio, teacher_prose::TeacherProse,
};
use dioxus::prelude::*;
use rufren_core::PresentationPurpose;
use rufren_tutor::TeacherResponse;

#[component]
pub(crate) fn TeacherExampleView(
    app: Signal<AppState>,
    answer: TeacherResponse,
    index: usize,
    #[props(default = PresentationPurpose::Instruction)] purpose: PresentationPurpose,
) -> Element {
    let Some(example) = answer.content.examples.get(index).cloned() else {
        return rsx! {p {role:"alert", "This saved example is unavailable."}};
    };
    let keep = example.clone();
    rsx! {
        article {class:"phrase-study teacher-lesson-example","data-example-index":index.to_string(),
            PresentedRussian {app,class:"dialogue-russian",text:example.russian,purpose}
            TeacherProse {app,language:"en",text:example.english,purpose}
            if !example.note.is_empty() {TeacherProse {app,class:"quiet",language:"en",text:example.note,purpose}}
            TeacherExampleAudio {app,answer,index}
            button {class:"text-button",onclick:move |_|act(app,|s|s.keep_teacher_example(keep.clone())),"Keep this phrase for practice →"}
        }
    }
}
