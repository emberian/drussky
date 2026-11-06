use crate::exposure_view::PresentedRussian;
use crate::{act, model::AppState};
use dioxus::prelude::*;
use rufren_content::{ExerciseKind, ResponseMode};
use rufren_core::{PresentationPurpose, Skill};

#[component]
pub(crate) fn Lessons(mut app: Signal<AppState>) -> Element {
    let state = app.read().clone();
    let selected = state.lesson().cloned();
    rsx! {
        div {class:"page-heading", div {div {class:"eyebrow","FROM YOUR FIRST WORDS TO OTHER WORLDS"} h1 {"A path into Russian"}p {"Learn with your AI teacher, from your first exchange to ideas worth reading."}}}
        div {class:"lesson-layout",
            aside {class:"lesson-index",aria_label:"Lesson sequence",
                for unit in &state.content.units {section {h2 {"{unit.title}"}p {class:"quiet","{unit.description}"}
                    for id in &unit.lesson_ids {if let Some(lesson)=state.content.lesson(id) {
                        {let id=id.clone();rsx! {button {class:if state.selected_lesson.as_deref()==Some(id.as_str()) {"lesson-link selected"}else{"lesson-link"},onclick:move |_|act(app,|s|s.choose_lesson(&id)), "{lesson.title}"}}}
                    }}
                }}
            }
            div {class:"lesson-body",
                if let Some(lesson)=selected {
                    section {class:"panel",span {class:"tag","LESSON"}h2 {"{lesson.title}"}p {class:"lesson-goal","{lesson.goal}"}
                        p {class:"draft-note","AI-authored learning material · editorial draft. Hear an AI speech model, explore the pattern, and try a short response at your own pace."}
                        div {class:"actions",button {class:"primary",onclick:{let id=lesson.id.clone();move |_|act(app,|s|{s.import_lesson(&id)?;s.notice="Lesson phrases added to your library for listening, practice, and your own notes.".into();Ok(())})},"Keep these phrases"}}
                    }
                    section {class:"panel",h2 {"{lesson.dialogue.title}"}p {class:"quiet","{lesson.dialogue.situation}"}
                        for turn in &lesson.dialogue.turns {if let Some(phrase)=state.content.phrase(&turn.phrase_id) {
                            div {class:"dialogue-turn",span {class:"speaker","{turn.speaker}"}div {PresentedRussian {key:"dialogue-{lesson.id}-{phrase.id}",app,class:"dialogue-russian",text:if state.preferences.show_stress {phrase.stressed.clone()}else{phrase.russian.clone()},purpose:PresentationPurpose::Instruction}p {class:"quiet","{phrase.translation}"}}}
                        }}
                    }
                    section {class:"panel",h2 {"Notice the pattern"}
                        for explanation in &lesson.explanations {article {class:"explanation",h3 {"{explanation.title}"}PresentedRussian {key:"explanation-{lesson.id}-{explanation.title}",app,text:explanation.text.clone(),language:"en",purpose:PresentationPurpose::Instruction}}}
                        p {class:"sound-note","{lesson.sound_note}"}
                    }
                    section {class:"panel",h2 {"Make the words familiar"}
                        for phrase in state.content.lesson_phrases(&lesson) {
                            {let id=phrase.id.clone();let audio_id=phrase.id.clone();rsx! {article {class:"phrase-study",
                                PresentedRussian {key:"study-{lesson.id}-{phrase.id}",app,class:"dialogue-russian",text:if state.preferences.show_stress {phrase.stressed.clone()}else{phrase.russian.clone()},purpose:PresentationPurpose::Instruction}
                                p {"{phrase.translation}"}
                                if state.preferences.chunked {div {class:"phrase-chunks",for chunk in &phrase.chunks {span {lang:"ru","{chunk}"}}}}
                                if !phrase.note.is_empty() {p {class:"quiet","{phrase.note}"}}
                                div {class:"actions",button {class:"secondary",onclick:move |_|act(app,|s|s.practice_phrase(&id,Skill::Reading)),"Read & recall"}
                                    button {class:"text-button",onclick:move |_|act(app,|s|{s.practice_phrase(&audio_id,Skill::Speaking)?;let rev=s.session.as_ref().unwrap().revision;s.open_studio(rev)?;Ok(())}),"Hear & say it"}}
                            }}}
                        }
                    }
                    section {class:"panel",h2 {"Try it in different ways"}p {class:"quiet","Try a short response with the AI teacher. Ask for a missing word, a simpler step, or one useful correction whenever you need it."}
                        for exercise in state.content.lesson_exercises(&lesson) {
                            {let id=exercise.id.clone();rsx! {button {class:"exercise-choice",onclick:move |_|act(app,|s|s.start_exercise(&id)),
                                span {class:"tag","{exercise_label(exercise.kind)}"}strong {"{exercise.instruction}"}
                                if exercise.requires_audio {small {"Uses an AI speech model; a human voice is optional"}}
                                else if exercise.response_mode==ResponseMode::Speech {small {"Try it aloud and compare your response"}}
                            }}}
                        }
                    }
                    section {class:"panel",h2 {"If it fits over a meal"}p {class:"quiet","An optional moment with your friend. No lesson preparation, recordings, or correction duties."}p {"{lesson.conversation_invitation}"}h3 {"A fresh idea for the AI teacher"}p {"{lesson.transfer_prompt}"}}
                } else {
                    section {class:"panel empty",h2 {"Start with a conversation."}p {"Open the first lesson with your AI teacher. You can hear, read, and try one useful phrase before learning the whole alphabet."}
                        button {class:"primary",onclick:move |_|act(app,|s|{let id=s.content.lessons.first().ok_or("No first lesson")?.id.clone();s.choose_lesson(&id)}),"Open the first lesson →"}}
                }
                details {class:"panel alphabet",summary {"Cyrillic reference · letters, sounds, and examples"}
                    p {class:"quiet","A letter’s name and its sound in a word are different things. Listen to the AI speech model and use this panel when a letter catches your attention."}
                    div {class:"alphabet-grid",for letter in &state.content.script {article {strong {lang:"ru","{letter.upper} {letter.lower}"}span {"{letter.name}"}p {"{letter.cue}"}small {{letter.examples.join(" · ")}}}}}
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
