//! Shared language-content feedback, independent of the lesson or revisit navigation.
use crate::{
    exposure_view::PresentedRussian, model::AppState, teacher_example_view::TeacherExampleView,
    teacher_practice_view::TeacherPractice as TeacherPracticeForm, teacher_prose::TeacherProse,
};
use dioxus::prelude::*;
use rufren_core::*;
use rufren_tutor::{AiVerdict, TeacherResponse};
use rufren_tutor::{TeacherRequest, TeacherSkill, TextEvidence};
use uuid::Uuid;

#[component]
pub(crate) fn TeacherFeedback(
    mut app: Signal<AppState>,
    answer: TeacherResponse,
    active_speaking: Signal<Option<String>>,
    capture_busy: Signal<bool>,
    #[props(default = true)] show_practice: bool,
) -> Element {
    let mut repair = use_signal(|| None::<(TeacherResponse, usize, Uuid)>);
    let state = app.read().clone();
    let task = match &answer.request {
        TeacherRequest::AnswerFeedback(request) => Some(request),
        _ => None,
    };
    let origin = state.teacher_feedback_task_origin(&answer);
    let repair_available = origin.is_some();
    let reading_question = origin.as_ref().and_then(|origin| {
        let source = state.library.teacher_document(origin.key.document_id)?;
        let exercise = source
            .response
            .content
            .practice
            .get(origin.key.practice_index)?;
        (exercise.skill == Some(TeacherSkill::Reading)).then(|| {
            (
                exercise.prompt.clone(),
                exercise.reference_russian.clone().unwrap_or_default(),
            )
        })
    });
    let recording = task.and_then(|request| {
        if request.evidence != TextEvidence::Transcript {
            return None;
        }
        let attempt = Uuid::parse_str(request.attempt_id.as_deref()?).ok()?;
        let confirmation = Uuid::parse_str(request.confirmed_transcript_id.as_deref()?).ok()?;
        if state.confirmed_words(confirmation, attempt).ok()? != request.response {
            return None;
        }
        let confirmation = state.library.transcript_confirmation(confirmation)?;
        let transcription = state
            .library
            .speech_transcription(confirmation.transcription)?;
        Some(transcription.asset_id.clone())
    });
    let busy = state.teacher_busy || state.speech_busy || capture_busy();
    let repair_answer = answer.clone();
    let response_id = answer.provenance.response_id.clone();
    let focus_id = format!("teacher-repair-{}", teacher_document_id(&answer));
    let focus_target = focus_id.clone();
    use_effect(move || {
        if repair().is_none() {
            return;
        }
        let id = serde_json::to_string(&focus_target).expect("generated repair heading ID");
        spawn(async move {
            let _ = document::eval(&format!("await new Promise(requestAnimationFrame);const heading=document.getElementById({id});if(heading){{heading.focus({{preventScroll:true}});heading.scrollIntoView({{block:'start'}});}}")).await;
        });
    });
    rsx! {
                if let Some(request)=task {
                    section {class:"teacher-feedback-task",aria_label:"The question and your saved response",
                        h3 {"The question you answered"}
                        if let Some((instruction,text))=&reading_question {
                            TeacherProse {app,language:"en",text:instruction.clone(),purpose:PresentationPurpose::Reference}
                            PresentedRussian {app,class:"dialogue-russian",text:text.clone(),purpose:PresentationPurpose::Reference}
                        } else {
                            TeacherProse {app,language:"en",text:request.prompt.clone(),purpose:PresentationPurpose::Reference}
                        }
                        h3 {if request.evidence==TextEvidence::Transcript {"Your saved words to discuss"}else{"Your saved response"}}
                        PresentedRussian {app,language:if matches!(request.skill,TeacherSkill::Writing|TeacherSkill::Speaking){"ru"}else{"en"},text:request.response.clone(),purpose:PresentationPurpose::Reference,
                            blockquote {style:"white-space:pre-wrap","{request.response}"}
                        }
                        if let Some(asset)=recording {crate::audio_studio::AudioReview {asset_id:asset}}
                        if request.evidence==TextEvidence::Transcript {p {class:"quiet","These are the words you confirmed or edited from the recording. This feedback concerns their meaning and language."}}
                    }
                }
                TeacherProse {key:"explanation-{answer.provenance.response_id}",app,class:"teacher-explanation",language:"en",text:answer.content.explanation.clone(),purpose:PresentationPurpose::Instruction}
                if let Some(feedback)=&answer.content.feedback {
                    p {strong {match feedback.verdict {AiVerdict::Correct=>"This works for the task.",AiVerdict::PartlyCorrect=>"Part of this works; let’s refine it.",AiVerdict::Incorrect=>"There’s something useful to change.",AiVerdict::Uncertain=>"There may be more than one interpretation here."}}}
                    if let Some(text)=&feedback.suggested_response {PresentedRussian {key:"{text}",app,class:"dialogue-russian",text:text.clone(),purpose:PresentationPurpose::Reference}}
                    for (index,correction) in feedback.corrections.iter().enumerate() {div {key:"correction-{answer.provenance.response_id}-{index}",class:"teacher-correction",PresentedRussian {app,text:format!("{} → {}",correction.original,correction.replacement),purpose:PresentationPurpose::Instruction,p {span {lang:"ru","{correction.original}"}" → " strong {lang:"ru","{correction.replacement}"}}}TeacherProse {app,language:"en",text:correction.reason.clone(),purpose:PresentationPurpose::Instruction}}}
                }
                for (index,_) in answer.content.examples.iter().enumerate() {TeacherExampleView {key:"example-{answer.provenance.response_id}-{index}",app,answer:answer.clone(),index}}
                div {class:"hint",span {class:"tag","YOUR NEXT STEP"}TeacherProse {app,language:"en",text:answer.content.next_step.clone(),purpose:PresentationPurpose::Instruction}}
                if repair_available {
                    if let Some((source,index,session))=repair() {
                        section {class:"teacher-feedback-repair",aria_label:"Try the same question again",
                            h3 {id:focus_id,tabindex:"-1","Try this question again"}
                            p {class:"quiet","Use the explanation as you try again. Your earlier answer is kept."}
                            TeacherPracticeForm {key:"repair-{response_id}-{session}",app,answer:source,index,active_speaking,capture_busy}
                        }
                    } else {
                        button {class:"secondary",disabled:busy,onclick:move |_| {
                            if app.read().teacher_busy || app.read().speech_busy || capture_busy() { return; }
                            let result=app.write().start_teacher_feedback_repair(&repair_answer);
                            match result {Ok(next)=>{app.write().notice.clear();repair.set(Some(next));},Err(error)=>app.write().notice=error}
                        },"Try this question again"}
                    }
                }
                if show_practice && !answer.content.practice.is_empty() {h3 {"More practice"}
                    for (index,_) in answer.content.practice.iter().enumerate() {
                        TeacherPracticeForm {key:"{answer.provenance.response_id}-{index}",app,answer:answer.clone(),index,active_speaking,capture_busy}
                    }
                }
                crate::teacher_dialogue_view::AskAboutThis {app,document:teacher_document_id(&answer),focus:rufren_tutor::TeacherDialogueFocus::Feedback,disabled:capture_busy()}
    }
}
