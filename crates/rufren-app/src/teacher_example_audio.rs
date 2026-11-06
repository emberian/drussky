//! Inline replay reuses the exact original-audio player, with a distinct DOM root.
use crate::{
    act,
    audio_studio::{PRACTICE_BRIDGE, PracticeAudioEvent},
    generate_reference,
    model::AppState,
    teacher_examples::example_reference,
};
use dioxus::prelude::*;
use rufren_tutor::TeacherResponse;

#[component]
pub(crate) fn TeacherExampleAudio(
    app: Signal<AppState>,
    answer: TeacherResponse,
    index: usize,
) -> Element {
    let revision = match example_reference(&answer, index) {
        Ok(event) => event.id,
        Err(problem) => return rsx! {p {class:"quiet", "{problem}"}},
    };
    let element_id = use_hook(|| format!("teacher-example-audio-{}", uuid::Uuid::new_v4()));
    let voice_id = format!("{element_id}-voice");
    let mut ready = use_signal(|| false);
    let mut error = use_signal(String::new);
    let effect_id = element_id.clone();
    use_effect(move || {
        let script = format!(
            "const rufrenPlayerRoot=document.getElementById({});\n{}",
            serde_json::to_string(&effect_id).unwrap(),
            PRACTICE_BRIDGE
        );
        spawn(async move {
            let mut bridge = document::eval(&script);
            while let Ok(event) = bridge.recv::<PracticeAudioEvent>().await {
                match event.kind.as_str() {
                    "ready" => ready.set(event.ready),
                    "played" => act(app, |s| s.heard_teacher_example(revision)),
                    _ => (),
                }
            }
            error.set("Reopen the Teacher to reconnect this example’s player.".into());
        });
    });
    let drop_id = element_id.clone();
    use_drop(move || {
        let id = serde_json::to_string(&drop_id).unwrap();
        let _ = document::eval(&format!("window.rufrenAudioPlayers?.get({id})?.destroy();"));
    });
    let busy = app.read().speech_busy;
    rsx! {
        div {class:"teacher-example-voice",
            if !ready() {
                button {class:"secondary",disabled:busy,onclick:move |_| {
                    let result=app.write().ensure_teacher_example(&answer,index);
                    match result {Ok(revision)=>generate_reference(app,revision),Err(problem)=>error.set(problem)}
                },if busy {"Generating audio…"}else{"Create spoken example"}}
            }
            div {id:element_id,class:"teacher-example-player",hidden:!ready(),"data-revision":revision.to_string(),aria_label:"Spoken teacher example",
                label {class:"quiet",r#for:voice_id.clone(),"Reference voice"}
                select {id:voice_id,disabled:true,"data-reference-select":"true"}
                div {class:"actions",
                    button {class:"secondary",disabled:true,"data-practice-rate":"1","▶ Listen"}
                    button {class:"text-button",disabled:true,"data-practice-rate":"0.7","Slowly"}
                    button {class:"text-button","data-practice-stop":"true","Stop"}
                }
                p {class:"quiet","data-reference-source":"true"}
                p {class:"quiet",role:"status","data-practice-status":"true","Opening saved audio…"}
            }
            if !error().is_empty() {p {class:"quiet",role:"alert","{error}"}}
        }
    }
}
