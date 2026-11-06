use crate::exposure_view::PresentedRussian;
use crate::{
    act,
    model::{AppState, Page},
};
use dioxus::prelude::*;
use rufren_content::{ExerciseKind, LessonPracticeAction, ResponseMode};
use rufren_core::{PresentationPurpose, Skill};

#[component]
pub(crate) fn Lessons(mut app: Signal<AppState>) -> Element {
    let state = app.read().clone();
    let selected = state.lesson().cloned();
    let mut query = use_signal(String::new);
    let needle = query().trim().to_lowercase();
    let mut detail_open = use_signal(|| false);
    let mut focus_request = use_signal(|| None::<String>);
    use_effect(move || {
        let Some(target) = focus_request() else {
            return;
        };
        focus_request.set(None);
        spawn(async move {
            let selector = if target == "detail" {
                "#lesson-detail-heading"
            } else {
                "#lesson-search"
            };
            let script = format!(
                "await new Promise(requestAnimationFrame);const el=document.querySelector({});if(el){{el.focus({{preventScroll:true}});const top=el.id==='lesson-detail-heading'&&matchMedia('(max-width:1000px)').matches?el.closest('.lesson-body'):el;top.scrollIntoView({{block:'start'}});}}",
                serde_json::to_string(selector).unwrap_or_default()
            );
            let _ = document::eval(&script).await;
        });
    });
    let mut preparing_sound = use_signal(|| None::<String>);
    rsx! {
        style {".lesson-link strong,.lesson-link small {{display:block}}.lesson-link small {{margin-top:4px;font-size:12px;font-weight:400;line-height:1.4;color:var(--muted)}}.lesson-index input {{width:100%;margin-bottom:16px}}"}
        div {class:if detail_open() {"page-heading lesson-catalog-heading detail-open"}else{"page-heading lesson-catalog-heading"}, div {h1 {"Learn"}p {"Choose something you want to understand. Your AI teacher will help you read, listen, write, and speak."}}}
        div {class:if detail_open() {"lesson-layout detail-open"}else{"lesson-layout"},
            aside {class:"lesson-index",aria_label:"Lesson sequence",
                label {r#for:"lesson-search","Find a lesson"}
                input {id:"lesson-search",r#type:"search",placeholder:"Search topics or goals",value:query(),oninput:move |event|query.set(event.value())}
                if !state.content.lessons.iter().any(|l|format!("{} {}",l.title,l.goal).to_lowercase().contains(&needle)) {p {"No lessons match this search."}}
                for unit in state.content.units.iter().filter(|unit|unit.lesson_ids.iter().any(|id|state.content.lesson(id).is_some_and(|l|format!("{} {}",l.title,l.goal).to_lowercase().contains(&needle)))) {section {h2 {"{unit.title}"}p {class:"quiet","{unit.description}"}
                    for id in &unit.lesson_ids {if let Some(lesson)=state.content.lesson(id).filter(|l|format!("{} {}",l.title,l.goal).to_lowercase().contains(&needle)) {
                        {let id=id.clone();rsx! {button {class:if state.selected_lesson.as_deref()==Some(id.as_str()) {"lesson-link selected"}else{"lesson-link"},aria_pressed:state.selected_lesson.as_deref()==Some(id.as_str()),onclick:move |_| {let result=app.write().choose_lesson(&id);match result {Ok(())=>{detail_open.set(true);focus_request.set(Some("detail".into()));},Err(error)=>app.write().notice=error,}}, strong {"{lesson.title}"} small {"{lesson.goal}"}}}}
                    }}
                }}
            }
            div {class:"lesson-body",
                button {class:"text-button lesson-catalog-back",onclick:move |_|{detail_open.set(false);focus_request.set(Some("list".into()));},"← Back to lessons"}
                if let Some(lesson)=selected {
                    section {class:"panel",span {class:"tag","LESSON"}h2 {id:"lesson-detail-heading",tabindex:"-1","{lesson.title}"}p {class:"lesson-goal","{lesson.goal}"}
                        p {class:"draft-note","AI-authored learning material · editorial draft. Hear an AI speech model, explore the pattern, and try a short response at your own pace."}
                        div {class:"actions",button {class:"primary",onclick:move |_|act(app,|s|{s.repo.set("teacher_surface", &"lessons")?;s.navigate(Page::Teacher)}),"Learn with the AI teacher →"}
                        button {class:"secondary",onclick:{let id=lesson.id.clone();move |_|act(app,|s|{s.import_lesson(&id)?;s.notice="Lesson phrases added to your library for listening, practice, and your own notes.".into();Ok(())})},"Keep these phrases"}}
                    }
                    section {class:"panel",h2 {"{lesson.dialogue.title}"}PresentedRussian {app,language:"en",class:"quiet",text:lesson.dialogue.situation.clone(),purpose:PresentationPurpose::Instruction}
                        for turn in &lesson.dialogue.turns {if let Some(phrase)=state.content.phrase(&turn.phrase_id) {
                            div {class:"dialogue-turn",span {class:"speaker","{turn.speaker}"}div {PresentedRussian {key:"dialogue-{lesson.id}-{phrase.id}",app,class:"dialogue-russian",text:if state.preferences.show_stress {phrase.stressed.clone()}else{phrase.russian.clone()},purpose:PresentationPurpose::Instruction}p {class:"quiet","{phrase.translation}"}}}
                        }}
                    }
                    section {class:"panel",h2 {"Notice the pattern"}
                        for explanation in &lesson.explanations {article {class:"explanation",PresentedRussian {app,language:"en",text:explanation.title.clone(),purpose:PresentationPurpose::Instruction,h3 {"{explanation.title}"}}PresentedRussian {key:"explanation-{lesson.id}-{explanation.title}",app,text:explanation.text.clone(),language:"en",purpose:PresentationPurpose::Instruction}}}
                        PresentedRussian {app,language:"en",class:"sound-note",text:lesson.sound_note.clone(),purpose:PresentationPurpose::Instruction}
                    }
                    section {class:"panel",h2 {"Make the words familiar"}
                        for phrase in state.content.lesson_phrases(&lesson) {
                            {let id=phrase.id.clone();let audio_id=phrase.id.clone();rsx! {article {class:"phrase-study",
                                PresentedRussian {key:"study-{lesson.id}-{phrase.id}",app,class:"dialogue-russian",text:if state.preferences.show_stress {phrase.stressed.clone()}else{phrase.russian.clone()},purpose:PresentationPurpose::Instruction}
                                p {"{phrase.translation}"}
                                if state.preferences.chunked {div {class:"phrase-chunks",for chunk in &phrase.chunks {PresentedRussian {app,text:chunk.clone(),purpose:PresentationPurpose::Instruction,span {"{chunk}"}}}}}
                                if !phrase.note.is_empty() {PresentedRussian {app,language:"en",class:"quiet",text:phrase.note.clone(),purpose:PresentationPurpose::Instruction}}
                                div {class:"actions",button {class:"secondary",onclick:move |_|act(app,|s|s.practice_phrase(&id,Skill::Reading)),"Read & recall"}
                                    button {class:"text-button",onclick:move |_|act(app,|s|{s.practice_phrase(&audio_id,Skill::Speaking)?;let rev=s.session.as_ref().unwrap().revision;s.open_studio(rev)?;Ok(())}),"Hear & say it"}}
                            }}}
                        }
                    }
                    section {class:"panel",h2 {"Try it in different ways"}p {class:"quiet","Try a short response with the AI teacher. Ask for a missing word, a simpler step, or one useful correction whenever you need it."}
                        match state.content.lesson_practice_actions(&lesson) {
                            Ok(actions) => rsx! {
                        for action in actions {match action {
                            LessonPracticeAction::Exercise(exercise) => rsx! {
                            {let id=exercise.id.clone();rsx! {button {class:"exercise-choice",onclick:move |_|act(app,|s|s.start_exercise(&id)),
                                span {class:"tag","{exercise_label(exercise.kind)}"}PresentedRussian {app,language:"en",text:exercise.instruction.clone(),purpose:PresentationPurpose::Instruction,strong {"{exercise.instruction}"}}
                                if exercise.requires_audio {small {"Uses an AI speech model; a human voice is optional"}}
                                else if exercise.response_mode==ResponseMode::Speech {small {"Try it aloud and compare your response"}}
                            }}}
                        },
                            LessonPracticeAction::SoundFocus(contrast) => {let id=contrast.id.clone();rsx! {
                                button {class:"exercise-choice",disabled:preparing_sound.read().is_some(),onclick:move |_| {
                                    let id=id.clone();preparing_sound.set(Some(id.clone()));
                                    spawn(async move {
                                        let result=crate::sound_focus_view::open_sound_focus(app,id).await;
                                        preparing_sound.set(None);
                                        if let Err(error)=result {app.write().notice=error;}
                                    });
                                },span {class:"tag","SOUND FOCUS"}strong {"{contrast.title}"}
                                    small {if preparing_sound.read().as_deref()==Some(contrast.id.as_str()) {"Preparing sound practice…"}else{"Listen for meaning, then choose what you want to say."}}
                                }
                            }},
                        }}                            },
                            Err(error) => rsx! {p {role:"alert","Sound practice is unavailable: {error}"}},
                        }

                    }
                    details {class:"panel",summary {"Optional conversation beyond the lesson"}p {class:"quiet","An optional moment with your friend. No lesson preparation, recordings, or correction duties."}PresentedRussian {app,language:"en",text:lesson.conversation_invitation.clone(),purpose:PresentationPurpose::Instruction}h3 {"A fresh idea for the AI teacher"}PresentedRussian {app,language:"en",text:lesson.transfer_prompt.clone(),purpose:PresentationPurpose::Instruction}}
                } else {
                    section {class:"panel empty",h2 {"Start with a conversation."}p {"Open the first lesson with your AI teacher. You can hear, read, and try one useful phrase before learning the whole alphabet."}
                        button {class:"primary",onclick:move |_|act(app,|s|{let id=s.content.lessons.first().ok_or("No first lesson")?.id.clone();s.choose_lesson(&id)}),"Open the first lesson →"}}
                }
                details {class:"panel alphabet",summary {"Cyrillic reference · letters, sounds, and examples"}
                    p {class:"quiet","A letter’s name and its sound in a word are different things. Listen to the AI speech model and use this panel when a letter catches your attention."}
                    div {class:"alphabet-grid",for letter in &state.content.script {article {PresentedRussian {app,text:format!("{} {}",letter.upper,letter.lower),purpose:PresentationPurpose::Reference,strong {"{letter.upper} {letter.lower}"}}PresentedRussian {app,language:"en",text:letter.name.clone(),purpose:PresentationPurpose::Reference,span {"{letter.name}"}}PresentedRussian {app,language:"en",text:letter.cue.clone(),purpose:PresentationPurpose::Reference}for example in &letter.examples {PresentedRussian {app,text:example.clone(),purpose:PresentationPurpose::Reference,small {"{example}"}}}}}}
                }
            }
        }
    }
}
pub(crate) fn exercise_label(kind: ExerciseKind) -> &'static str {
    match kind {
        ExerciseKind::ReadingComprehension => "Read for meaning",
        ExerciseKind::ListeningComprehension => "Listen for meaning",
        ExerciseKind::WrittenRetrieval => "Write from meaning",
        ExerciseKind::SpokenRetrieval => "Speak from meaning",
        ExerciseKind::Contrast => "Notice a contrast",
        ExerciseKind::Composition => "Make your own sentence",
        ExerciseKind::Dictation => "Listen and write",
    }
}
