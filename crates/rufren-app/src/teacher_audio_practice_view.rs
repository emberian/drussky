//! Listening and speaking retain the generated question and exact original audio.
use crate::{
    audio_studio::{AudioPractice, AudioReview, AudioSpeaking, StudioRecording},
    exposure_view::PresentedRussian,
    generate_reference,
    model::AppState,
    recognition_view::SpokenWords,
    register_recording,
    repository::Result,
    teacher_practice::TeacherPracticeDraft,
    teacher_prose::TeacherProse,
    teacher_view::send_teacher,
};
use dioxus::prelude::*;
use rufren_core::*;
use rufren_tutor::TeacherResponse;

#[component]
pub(crate) fn TeacherAudioPractice(
    mut app: Signal<AppState>,
    answer: TeacherResponse,
    index: usize,
    mut active_speaking: Signal<Option<String>>,
    mut capture_busy: Signal<bool>,
) -> Element {
    let initial = use_hook(|| {
        let mut state = app.write();
        let mut draft = state.teacher_practice_draft(&answer, index)?;
        let binding = state.prepare_teacher_audio_practice(&answer, &mut draft)?;
        Ok::<_, String>((draft, binding))
    });
    let draft = use_signal(|| initial.as_ref().ok().map(|(draft, _)| draft.clone()));
    let Ok((_, binding)) = &initial else {
        return rsx! {p {role:"alert", "{initial.as_ref().err().cloned().unwrap_or_default()}"}};
    };
    let Some(snapshot) = draft.read().clone() else {
        return rsx! {};
    };
    let Some(exercise) = answer.content.practice.get(index) else {
        return rsx! {};
    };
    let state = app.read().clone();
    let listening = binding.skill == Skill::Listening;
    let revision = binding.revision_id;
    let passage = binding.passage_id;
    let identity = format!("{}-{index}", snapshot.source);
    let editor_active = active_speaking.read().as_deref() == Some(identity.as_str());
    let played = snapshot
        .audio
        .as_ref()
        .is_some_and(|a| a.reference_plays > 0);
    let revealed = snapshot
        .audio
        .as_ref()
        .is_some_and(|a| a.cues.contains(&Cue::OriginalText));
    let ready_to_submit = if listening {
        played && !snapshot.response.trim().is_empty()
    } else {
        snapshot.selected_attachment().is_some()
    };
    let playback_answer = answer.clone();
    let model_playback_answer = answer.clone();
    let saved_answer = answer.clone();
    let selected_answer = answer.clone();
    let edit_answer = answer.clone();
    let submit_answer = answer.clone();
    let reveal_answer = answer.clone();
    let field = format!("teacher-oral-response-{identity}");
    rsx! {
        article {class:"phrase-study teacher-exercise teacher-oral-exercise", "data-teacher-skill":if listening {"listening"}else{"speaking"}, "data-exercise-index":index.to_string(),
            span {class:"tag",if listening {"LISTEN & UNDERSTAND"}else{"SAY SOMETHING"}}
            TeacherProse {app,language:"en",text:exercise.prompt.clone(),purpose:PresentationPurpose::TaskInput,session:Some(snapshot.id),revision:Some(revision)}
            if listening {
                AudioPractice {key:"listen-{identity}",instance_id:Some(format!("teacher-listening-{identity}")),revision_id:revision.to_string(),
                    on_played:move |_|{},on_ready:move |_|{},
                    on_played_recording:move |record| change_draft(app,draft,|state,draft| {
                        let metadata = recording_metadata(&record)?;
                        let attachment = register_recording(state,&record)?;
                        state.teacher_audio_reference_played(&playback_answer,draft,attachment,&metadata)
                    }),
                    generating:state.speech_busy,on_generate:move |_|generate_reference(app,revision)
                }
                label {r#for:field.clone(),"What did you understand?"}
                textarea {id:field,value:snapshot.response.clone(),disabled:state.teacher_busy,
                    placeholder:"English is fine. Listen as often as you need.",
                    oninput:move |event|change_draft(app,draft,|state,draft|state.teacher_practice_edit_response(&edit_answer,draft,event.value()))
                }
                if !played {p {class:"quiet","Play the reference before discussing your answer."}}
            } else {
                if editor_active {
                    AudioSpeaking {key:"speak-{identity}",passage_id:passage.to_string(),revision_id:revision.to_string(),
                        workspace_id:if cfg!(target_arch="wasm32") {String::new()}else{state.device.to_string()},
                        selected_asset_id:snapshot.selected_asset_id().map(str::to_owned),
                        on_recording_state:move |busy|capture_busy.set(busy),
                        on_saved:move |record|change_draft(app,draft,|state,draft| {
                            let metadata=recording_metadata(&record)?;
                            let attachment=register_recording(state,&record)?;
                            if record.interrupted {
                                state.teacher_audio_clear_recording(&saved_answer,draft)?;
                                state.notice="Your partial take is kept. Select it if you want to use it, or record again.".into();
                                Ok(())
                            } else {state.teacher_audio_select_recording(&saved_answer,draft,attachment,&metadata,false)}
                        }),
                        on_selected:move |record:Option<StudioRecording>|change_draft(app,draft,|state,draft| {
                            if let Some(record)=record {
                                let metadata=recording_metadata(&record)?;
                                let attachment=register_recording(state,&record)?;
                                state.teacher_audio_select_recording(&selected_answer,draft,attachment,&metadata,true)
                            } else {state.teacher_audio_clear_recording(&selected_answer,draft)}
                        })
                    }
                } else {
                    button {class:"secondary",disabled:capture_busy(),onclick:move |_|active_speaking.set(Some(identity.clone())),"Open recorder for this exercise"}
                    if let Some(asset)=snapshot.selected_asset_id() {AudioReview {asset_id:asset.to_owned()}}
                }
                if snapshot.selected_attachment().is_some() {p {class:"quiet","Your selected take is ready. Keep it, then check the words it heard."}}
            }
            button {class:"primary",disabled:state.teacher_busy || capture_busy() || !ready_to_submit,
                onclick:move |_| {
                    let Some(mut next)=draft.read().clone() else{return};
                    let result=app.write().submit_teacher_practice(&submit_answer,&mut next);
                    match result {
                        Ok(id)=>{
                            let mut draft=draft;
                            draft.set(Some(next));
                            if listening {let request=app.read().answer_feedback_request(id);send_teacher(app,request);}
                            else {app.write().notice="Your spoken response is kept. Find and check its words below.".into();}
                        },
                        Err(problem)=>app.write().notice=problem,
                    }
                },
                if listening {if state.teacher_busy {"Your teacher is thinking…"}else{"Discuss what I heard →"}}else{"Keep this spoken response →"}
            }
            if !listening {
                if let Some(attempt)=snapshot.saved_attempt {
                    for (event,recording) in state.library.recordings(AnnotationTarget::Attempt(attempt)).into_iter().filter(|(_,recording)|recording.role==AudioRole::Attempt) {
                        div {key:"oral-take-{event.id}",class:"spoken-take",
                            AudioReview {asset_id:recording.asset_id.clone()}
                            SpokenWords {app,attempt,recording:event.id}
                        }
                    }
                }
            }
            if !revealed {
                button {class:"text-button",disabled:capture_busy(),onclick:move |_|change_draft(app,draft,|state,draft| {
                    state.teacher_audio_reveal(&reveal_answer,draft,Cue::OriginalText)?;
                    state.teacher_audio_reveal(&reveal_answer,draft,Cue::Translation)
                }),"Show a model & explanation"}
            } else {
                div {class:"hint teacher-oral-model",
                    if let Some(text)=&exercise.reference_russian {PresentedRussian {app,class:"dialogue-russian",text:text.clone(),purpose:PresentationPurpose::Reference,revision:Some(revision),session:Some(snapshot.id)}}
                    if !listening {
                        AudioPractice {instance_id:Some(format!("teacher-speaking-model-{}-{index}",snapshot.source)),revision_id:revision.to_string(),
                            on_played:move |_|{},on_ready:move |_|{},
                            on_played_recording:move |record|change_draft(app,draft,|state,draft| {
                                let metadata=recording_metadata(&record)?;
                                let attachment=register_recording(state,&record)?;
                                state.teacher_audio_reference_played(&model_playback_answer,draft,attachment,&metadata)
                            }),generating:state.speech_busy,on_generate:move |_|generate_reference(app,revision)
                        }
                    }
                    if let Some(text)=&exercise.reference_english {TeacherProse {app,language:"en",text:text.clone(),purpose:PresentationPurpose::Reference,session:Some(snapshot.id),revision:Some(revision)}}
                    if let Some(text)=&exercise.expected_response {TeacherProse {app,language:"en",text:text.clone(),purpose:PresentationPurpose::Answer,session:Some(snapshot.id),revision:Some(revision)}}
                    TeacherProse {app,language:"en",text:exercise.teaching_note.clone(),purpose:PresentationPurpose::Instruction,session:Some(snapshot.id),revision:Some(revision)}
                }
            }
        }
    }
}

fn recording_metadata(record: &StudioRecording) -> Result<AudioRecordMetadata> {
    serde_json::to_value(record)
        .and_then(serde_json::from_value)
        .map_err(|e| e.to_string())
}

fn change_draft(
    mut app: Signal<AppState>,
    mut draft: Signal<Option<TeacherPracticeDraft>>,
    change: impl FnOnce(&mut AppState, &mut TeacherPracticeDraft) -> Result<()>,
) {
    let Some(mut next) = draft.read().clone() else {
        return;
    };
    let result = change(&mut app.write(), &mut next);
    match result {
        Ok(()) => draft.set(Some(next)),
        Err(problem) => app.write().notice = problem,
    }
}
