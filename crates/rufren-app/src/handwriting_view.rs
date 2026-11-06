//! Handwriting UI and a narrow save-before-navigation bridge.
use crate::{
    handwriting::*,
    model::{AppState, Page},
    repository::Result,
};
use dioxus::prelude::*;
use serde::Deserialize;
use uuid::Uuid;
const BRIDGE: &str = include_str!("handwriting.js");
const UNSAVED_DRAWING_NOTICE: &str =
    "Your drawing still needs saving. Keep this pad open and retry.";
/// Parent awaits this before switching tasks/pages. A false result keeps the visible ink in place.
pub(crate) async fn flush(scope: &str) -> Result<()> {
    let scope = serde_json::to_string(scope).map_err(|e| e.to_string())?;
    let value = document::eval(&format!(
        "const pad=window.rufrenInkPads?.get({scope});if(!pad)return true;return await pad.flush();"
    ))
    .await
    .map_err(|e| e.to_string())?;
    if value == serde_json::Value::Bool(true) {
        Ok(())
    } else {
        Err(UNSAVED_DRAWING_NOTICE.into())
    }
}
#[derive(Deserialize)]
struct InkEvent {
    kind: String,
    #[serde(default)]
    request_id: String,
    #[serde(default)]
    draft: Option<HandwritingSubmission>,
    #[serde(default)]
    submission: Option<HandwritingSubmission>,
    #[serde(default)]
    message: String,
    #[serde(default)]
    dirty: bool,
    #[serde(default)]
    audio_id: String,
    #[serde(default)]
    attempt_id: Option<Uuid>,
}
#[component]
pub(crate) fn HandwritingPad(
    app: Signal<AppState>,
    scope_id: String,
    task: HandwritingTask,
    #[props(default=InkMode::Trace)] initial_mode: InkMode,
    on_saved: EventHandler<HandwritingSubmission>,
    #[props(default)] on_dirty: EventHandler<bool>,
    #[props(default)] on_audio_completed: EventHandler<String>,
    #[props(default)] attempt_id: Option<Uuid>,
) -> Element {
    let instance = use_hook(|| Uuid::new_v4().to_string());
    let root_id = format!("handwriting-{instance}");
    let mut error = use_signal(String::new);
    let mut audio_error = use_signal(String::new);
    let mut persistence_error = use_signal(String::new);
    let effect_root = root_id.clone();
    let effect_scope = scope_id.clone();
    let effect_task = task.clone();
    use_effect(move || {
        let root = effect_root.clone();
        let scope = effect_scope.clone();
        let task = effect_task.clone();
        let initial_mode = initial_mode.clone();
        spawn(async move {
            let initial = match app.read().handwriting_draft(&scope, &task) {
                Ok(d) => d,
                Err(e) => {
                    error.set(e);
                    return;
                }
            };
            let config = serde_json::json!({"model_version":task.model_version,"scope_id":scope,"task_id":task.id,"task_revision":task.revision,"target":task.target,"audio_id":task.reference_audio_id,"audio_role":task.audio_role,"initial_mode":initial_mode,"initial":initial,"paper":task.paper,"attempt_id":attempt_id,"external_flow":true,"meaning_visible":task.meaning.is_some(),"sound_visible":task.reference_audio_id.is_some()});
            let script = format!(
                r#"{BRIDGE}
const root=document.getElementById({root});if(!root)throw Error('The handwriting pad did not mount.');
const config={config};const pending=new Map();
const request=(kind,payload)=>new Promise((resolve,reject)=>{{const request_id=crypto.randomUUID();const timer=setTimeout(()=>{{pending.delete(request_id);reject(Error('The drawing could not be saved yet. Keep this pad open and retry.'));}},10000);pending.set(request_id,{{resolve:()=>{{clearTimeout(timer);resolve();}},reject:error=>{{clearTimeout(timer);reject(error);}}}});dioxus.send({{kind,request_id,...payload}});}});
const adapter={{saveDraft:draft=>request('persist_draft',{{draft}}),saveOriginal:submission=>request('persist_original',{{submission}})}};
window.rufrenInkPads??=new Map();if(window.rufrenInkPads.has(config.scope_id))throw Error('This handwriting task is already open.');
const pad=window.RufrenHandwriting.install(root,config,adapter,event=>dioxus.send(event));window.rufrenInkPads.set(config.scope_id,pad);
try{{while(true){{const message=await dioxus.recv();const waiter=pending.get(message.request_id);if(waiter){{pending.delete(message.request_id);if(message.ok)waiter.resolve();else waiter.reject(Error(message.error||'Ink could not be saved.'));}}}}}}finally{{pad.dispose();window.rufrenInkPads.delete(config.scope_id);for(const waiter of pending.values())waiter.reject(Error('The drawing pad closed.'));}}
"#,
                root = serde_json::to_string(&root).unwrap()
            );
            let mut bridge = document::eval(&script);
            while let Ok(event) = bridge.recv::<InkEvent>().await {
                match event.kind.as_str() {
                    "persist_draft" | "persist_original" => {
                        let result = if event.kind == "persist_draft" {
                            event
                                .draft
                                .as_ref()
                                .ok_or_else(|| "The ink draft is missing.".into())
                                .and_then(|draft| {
                                    app.read().handwriting_store_draft(&scope, &task, draft)
                                })
                        } else {
                            event
                                .submission
                                .as_ref()
                                .ok_or_else(|| "The original ink is missing.".into())
                                .and_then(|original| {
                                    app.read()
                                        .handwriting_store_original(&scope, &task, original)
                                })
                        };
                        if result.is_ok() && app.read().notice == UNSAVED_DRAWING_NOTICE {
                            app.write().notice.clear();
                        }
                        let _=bridge.send(serde_json::json!({"request_id":event.request_id,"ok":result.is_ok(),"error":result.err()}));
                    }
                    "saved" => {
                        if let Some(submission) = event.submission {
                            error.set(String::new());
                            on_saved.call(submission)
                        }
                    }
                    "persistence_error" => persistence_error.set(event.message),
                    "persistence_recovered" => persistence_error.set(String::new()),
                    "status" => on_dirty.call(event.dirty),
                    "audio_completed" => on_audio_completed.call(event.audio_id),
                    "error" => error.set(event.message),
                    "stop_audio" => crate::foundation_audio::stop(),
                    "listen" => {
                        if let Some(asset) = task.reference_audio_id.clone() {
                            let scope = scope.clone();
                            let attempt = event.attempt_id;
                            spawn(async move {
                                let result = crate::foundation_audio::play(&asset).await;
                                let script = format!(
                                    "return await window.rufrenInkPads?.get({})?.audioResult({},{}) ?? false;",
                                    serde_json::to_string(&scope).unwrap(),
                                    serde_json::to_string(&attempt).unwrap(),
                                    result.is_ok()
                                );
                                let current = document::eval(&script).await.ok()
                                    == Some(serde_json::Value::Bool(true));
                                if current {
                                    if let Err(e) = result {
                                        audio_error.set(e)
                                    } else {
                                        audio_error.set(String::new());
                                    }
                                }
                            });
                        }
                    }
                    _ => (),
                }
            }
        });
    });
    let drop_scope = scope_id.clone();
    use_drop(move || {
        crate::foundation_audio::stop();
        let scope = serde_json::to_string(&drop_scope).unwrap();
        let _ = document::eval(&format!(
            "window.rufrenInkPads?.get({scope})?.dispose();window.rufrenInkPads?.delete({scope});"
        ));
    });
    rsx! {
      style {"{INK_STYLE}"}
      section {id:root_id,class:"handwriting-pad",aria_label:"Handwritten Russian practice",
       p {class:"ink-instruction","{task.instruction}"}
       if !error().is_empty(){p {role:"alert","{error}"}}
       if !audio_error().is_empty(){p {role:"alert","{audio_error}"}}
       if !persistence_error().is_empty(){p {role:"alert","{persistence_error}"}}
          div {class:"ink-context","data-ink-meaning":"true",if let Some(meaning)=task.meaning.clone(){p {"{meaning}"}}}
       div {class:"ink-sound","data-ink-sound":"true",if task.reference_audio_id.is_some(){button {class:"primary","data-ink-listen":"true","Listen"}}}
       div {class:"ink-pad-heading",span {"Write here"}div {"data-ink-device-options":"true",button {class:"text-button","data-ink-input":"pen","data-ink-control":"true","Use Pencil"}span {" · "}button {class:"text-button","data-ink-input":"finger","data-ink-control":"true","Use finger"}}button {class:"secondary","data-ink-show":"true","data-ink-control":"true","Show model"}button {class:"secondary","data-ink-hide":"true","data-ink-control":"true","Hide model"}}
    div {class:"ink-workspace",div {class:"ink-model","data-ink-model":"true"}div {class:"ink-paper",span {class:"ink-pane-label","Your drawing"}canvas {width:"400",height:"300",aria_label:"Draw your Russian letters here"}}div {class:"ink-comparison","data-ink-comparison":"true",hidden:true}}
       div {class:"actions",button {class:"secondary","data-ink-undo":"true","data-ink-edit":"true","Undo"}button {class:"secondary","data-ink-clear":"true","data-ink-edit":"true","Clear"}button {class:"primary","data-ink-compare":"true","Compare"}button {class:"secondary","data-ink-new":"true",hidden:true,"Try again"}}
       p {role:"status","data-ink-status":"true"}button {class:"secondary","data-ink-retry-save":"true",hidden:true,"Retry saving"}
       details {summary {"Drawing and cues"}
        p {class:"ink-guidance","data-ink-guidance":"true",hidden:initial_mode==InkMode::Hidden,"{task.formation_hint}"}
        fieldset {class:"ink-mode-options",legend {"Model help"}for (mode,label) in [("hidden","Hidden"),("copy","Copy"),("trace","Trace")]{label {input {r#type:"radio",name:format!("ink-model-{instance}"),"data-ink-mode":mode,"data-ink-control":"true"}"{label}"}}}
     button {class:"secondary","data-ink-replay":"true","data-ink-control":"true","Watch the strokes"}
     if task.meaning.is_some(){button {class:"secondary","data-ink-toggle-meaning":"true","data-ink-control":"true","Show / hide meaning"}}
     if task.reference_audio_id.is_some()&&task.audio_role==InkAudioRole::Model{button {class:"secondary","data-ink-toggle-sound":"true","data-ink-control":"true","Show / hide sound cue"}}
     p {"Use the page outside the pad to scroll. These are unjoined hand-printed forms; cursive can be learned later."}

       }
      }
     }
}
const INK_STYLE: &str = r#".handwriting-pad{max-width:760px}.ink-mode-options{border:0;padding:0;display:flex;gap:16px;flex-wrap:wrap}.ink-mode-options label{display:flex;align-items:center;gap:6px;min-height:44px}.ink-mode-options input{width:20px;height:20px}.ink-instruction{font-size:1.1rem;line-height:1.45}.ink-pad-heading{display:flex;gap:10px;align-items:center;flex-wrap:wrap;margin:8px 0}.ink-pad-heading>span{font-weight:600;margin-right:auto}.ink-pad-heading button[aria-pressed=true]{font-weight:700;text-decoration:underline}.ink-workspace{position:relative;width:100%;height:220px}.ink-paper{height:100%;min-width:0}.ink-workspace canvas{display:block;width:100%;height:100%;position:relative;touch-action:none;user-select:none;-webkit-user-select:none;border:1px solid #aab9ac;border-radius:10px;background:transparent;box-sizing:border-box}.ink-model svg,.ink-comparison svg{display:block;width:100%;height:220px}.ink-model{color:#adb8ad}.handwriting-pad[data-visual=copy] .ink-model{color:#596e5e}.handwriting-pad[data-visual=trace] .ink-model{position:absolute;inset:0;pointer-events:none}.ink-pane-label{display:none;font-size:.9rem;font-weight:600;margin-bottom:6px}.handwriting-pad[data-visual=copy] .ink-workspace,.handwriting-pad[data-sealed=true] .ink-workspace{height:auto;display:grid;grid-template-columns:repeat(auto-fit,minmax(min(100%,240px),1fr));gap:14px;align-items:start}.handwriting-pad[data-visual=copy] .ink-paper,.handwriting-pad[data-sealed=true] .ink-paper{height:auto}.handwriting-pad[data-visual=copy] canvas,.handwriting-pad[data-sealed=true] canvas{height:220px}.handwriting-pad[data-visual=copy] .ink-pane-label,.handwriting-pad[data-sealed=true] .ink-pane-label{display:block}.handwriting-pad[data-sealed=true] .ink-workspace>.ink-model{display:none}.ink-comparison{color:#596e5e}.handwriting-pad button,.handwriting-pad select{min-height:44px;white-space:normal}.ink-guidance{font-size:.95rem;line-height:1.5}.handwriting-pad [hidden]{display:none!important}@media(max-width:600px){.handwriting-pad[data-visual=copy] .ink-workspace,.handwriting-pad[data-sealed=true] .ink-workspace{grid-template-columns:1fr}.ink-workspace{height:200px}}"#;

async fn resume_drawing(scope: &str) {
    let scope = serde_json::to_string(scope).unwrap();
    let _ = document::eval(&format!(
        "window.rufrenInkPads?.get({scope})?.resumeDrawing();"
    ))
    .await;
}
/// Only actual shell navigation uses this gate; ordinary application mutations stay synchronous.
pub(crate) fn nav_act(
    mut app: Signal<AppState>,
    f: impl FnOnce(&mut AppState) -> Result<()> + 'static,
) {
    if app.read().page != Page::Handwriting {
        crate::act(app, f);
        return;
    }
    if !app.read().writing_navigation_needs_flush() {
        crate::act(app, f);
        return;
    }
    if app.read().handwriting_navigation_busy {
        return;
    }
    if app.read().writing_speech_busy {
        app.write().notice =
            "Stop recording or finish listening before leaving this speaking turn.".into();
        return;
    }
    let selected = {
        let state = app.read();
        state.active_handwriting()
    };
    let expected = match selected {
        Ok(activity) => activity,
        Err(e) => {
            app.write().notice = e;
            return;
        }
    };
    let scope = expected.scope();
    app.write().handwriting_navigation_busy = true;
    if let Some(instance) = expected.native_instance {
        if handwriting_native::is_open() {
            NATIVE_LEAVES.with(|q| {
                q.borrow_mut().insert(
                    instance,
                    Box::new(move || {
                        let valid = {
                            let a = app.read();
                            a.page == Page::Handwriting
                                && a.active_handwriting().ok().is_some_and(|r| {
                                    r.id == expected.id
                                        && r.round_id == expected.round_id
                                        && r.stage == expected.stage
                                })
                        };
                        if valid {
                            crate::foundation_audio::stop();
                            crate::act(app, f)
                        } else {
                            app.write().notice =
                                "The writing task changed. Its drawing is kept.".into()
                        }
                        app.write().handwriting_navigation_busy = false;
                    }),
                )
            });
            if let Err(e) = handwriting_native::request_pause(instance) {
                native_error(app, instance, e)
            }
            return;
        }
    }
    spawn(async move {
        match flush(&scope).await {
            Ok(()) => {
                let still_current = {
                    let a = app.read();
                    a.page == Page::Handwriting
                        && a.active_handwriting().ok().is_some_and(|current| {
                            current.id == expected.id
                                && current.round_id == expected.round_id
                                && current.stage == expected.stage
                        })
                };
                if still_current {
                    crate::foundation_audio::stop();
                    crate::act(app, f)
                } else {
                    app.write().notice = "The writing task changed. Your drawing is kept.".into()
                };
                if app.read().page == Page::Handwriting {
                    resume_drawing(&scope).await
                }
            }
            Err(e) => {
                app.write().notice = e;
                resume_drawing(&scope).await
            }
        }
        app.write().handwriting_navigation_busy = false;
    });
}

use crate::handwriting_native::{
    self, NativeCheckpointReason, NativePaper, NativeWritingAudio, NativeWritingCheckpoint,
    NativeWritingEvent, NativeWritingRequest,
};
use rufren_content::handwriting_tasks::{
    FoundationHandwritingSequence as WritingSequence, handwriting_tasks,
};
use sha2::{Digest, Sha256};
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
thread_local! {static NATIVE_LEAVES:RefCell<BTreeMap<Uuid,Box<dyn FnOnce()>>>=RefCell::new(BTreeMap::new());}
fn native_error(mut app: Signal<AppState>, instance: Uuid, error: String) {
    NATIVE_LEAVES.with(|q| {
        q.borrow_mut().remove(&instance);
    });
    app.write().handwriting_navigation_busy = false;
    app.write().notice = error;
}
fn native_event(
    mut app: Signal<AppState>,
    run_id: Uuid,
    round: Uuid,
    instance: Uuid,
    event: NativeWritingEvent,
) {
    let event_instance = match &event {
        NativeWritingEvent::Checkpoint { checkpoint, .. } => checkpoint.instance_id,
        NativeWritingEvent::Ready { instance_id }
        | NativeWritingEvent::Closed { instance_id }
        | NativeWritingEvent::CompareReady { instance_id, .. }
        | NativeWritingEvent::ContinueToSpeaking { instance_id, .. }
        | NativeWritingEvent::Paused { instance_id, .. }
        | NativeWritingEvent::ModelRevealed { instance_id, .. }
        | NativeWritingEvent::AudioStarted { instance_id, .. }
        | NativeWritingEvent::AudioCompleted { instance_id, .. }
        | NativeWritingEvent::AudioStopped { instance_id, .. }
        | NativeWritingEvent::Failure { instance_id, .. } => *instance_id,
    };
    if event_instance != instance {
        return;
    }
    let run = match app.read().active_handwriting() {
        Ok(r) if r.id == run_id && r.round_id == round && r.native_instance == Some(instance) => r,
        _ => return,
    };
    match event {
        NativeWritingEvent::Checkpoint { reason, checkpoint } => {
            let result = (|| -> Result<()> {
                checkpoint.validate()?;
                let task = run.task();
                if checkpoint.instance_id != instance
                    || checkpoint.scope_id != run.scope()
                    || checkpoint.attempt_id != run.attempt_id
                    || checkpoint.task_id != task.id
                    || checkpoint.task_revision != task.revision
                {
                    return Err("This drawing checkpoint belongs to another activity. Its recovery copy is preserved.".into());
                }
                app.read().repo.set(
                    &format!("handwriting.native.latest:{}", run.scope()),
                    &checkpoint,
                )?;
                if reason == NativeCheckpointReason::Compare {
                    if checkpoint.canonical.strokes.is_empty() {
                        return Err(
                            "Write something first, or go back to keep this activity for later."
                                .into(),
                        );
                    }
                    let reference = app.read().writing_store_artifact(
                        checkpoint.checkpoint_id,
                        &run.scope(),
                        &task.id,
                        &task.revision,
                        "apple-pencilkit-control-points-v1",
                        serde_json::to_value(&checkpoint).map_err(|e| e.to_string())?,
                    )?;
                    app.read()
                        .writing_check_original(run_id, round, &reference)?;
                    let key = format!("handwriting.native.checkpoints:{}", run.scope());
                    let mut ids: Vec<Uuid> = app.read().repo.get(&key)?.unwrap_or_default();
                    if !ids.contains(&checkpoint.checkpoint_id) {
                        ids.push(checkpoint.checkpoint_id);
                        app.read().repo.set(&key, &ids)?
                    }
                }
                Ok(())
            })();
            let ack = handwriting_native::ack(
                instance,
                checkpoint.sequence,
                result.is_ok(),
                result.as_ref().err().cloned(),
            );
            if let Err(e) = result.and(ack) {
                native_error(app, instance, e)
            }
        }
        NativeWritingEvent::CompareReady {
            checkpoint_id,
            sequence,
            archive_sha256,
            ..
        } => {
            let result = (|| -> Result<()> {
                let reference = app.read().writing_artifact_ref(checkpoint_id)?;
                let stored: serde_json::Value = app
                    .read()
                    .repo
                    .get(&reference.local_key)?
                    .ok_or("The compared drawing is unavailable")?;
                if stored["payload"]["sequence"] != sequence
                    || stored["payload"]["archive_sha256"] != archive_sha256
                {
                    return Err(
                        "The compared drawing no longer matches its exact checkpoint.".into(),
                    );
                }
                app.read().writing_accept_original(run_id, round, reference)
            })();
            if let Err(e) = result {
                native_error(app, instance, e)
            } else {
                app.write().notice.clear()
            }
        }
        NativeWritingEvent::ContinueToSpeaking { checkpoint_id, .. } => {
            let result = (|| -> Result<()> {
                let current = app.read().active_handwriting()?;
                if current.original.as_ref().map(|r| r.id) != Some(checkpoint_id) {
                    return Err("The saved comparison changed. Its original is kept.".into());
                }
                app.read().writing_go_say(run_id, round)?;
                app.read().writing_native_instance(run_id, round, None)
            })();
            if let Err(e) = result {
                native_error(app, instance, e)
            } else {
                app.write().notice.clear()
            }
        }
        NativeWritingEvent::AudioCompleted { audio_id, .. } => {
            let result = { app.read().writing_audio_completed(run_id, round, &audio_id) };
            if let Err(e) = result {
                native_error(app, instance, e)
            } else {
                app.write().notice.clear();
            }
        }
        NativeWritingEvent::ModelRevealed { .. } => {
            let result = { app.read().writing_model_revealed(run_id, round) };
            if let Err(e) = result {
                native_error(app, instance, e)
            } else {
                app.write().notice.clear();
            }
        }
        NativeWritingEvent::Paused { .. } => {
            let _ = app.read().writing_native_instance(run_id, round, None);
            if let Some(action) = NATIVE_LEAVES.with(|q| q.borrow_mut().remove(&instance)) {
                action()
            } else {
                app.write().notice.clear()
            }
        }
        NativeWritingEvent::Failure { message, .. } => native_error(app, instance, message),
        NativeWritingEvent::Closed { .. } => {
            let _ = app.read().writing_native_instance(run_id, round, None);
            app.write().handwriting_navigation_busy = false;
        }
        _ => (),
    }
}
fn attach_native(
    mut app: Signal<AppState>,
    run: HandwritingActivity,
    on_event: Callback<(Uuid, Uuid, Uuid, NativeWritingEvent)>,
    alive: Rc<std::cell::Cell<bool>>,
    host: handwriting_native::NativeWritingHostRect,
) -> Result<Uuid> {
    if handwriting_native::is_open() {
        return Err("Another writing page is still saving. Try again in a moment.".into());
    }
    (|| -> Result<Uuid> {
        let current = app.read().active_handwriting()?;
        if current.id != run.id || current.round_id != run.round_id || current.stage != run.stage {
            return Err("The writing activity changed. Its original is kept.".into());
        }
        let task = run.task();
        let instance = Uuid::new_v4();
        let bytes = crate::foundation_audio::bytes(
            task.reference_audio_id
                .as_deref()
                .ok_or("This activity has no prepared spoken model")?,
        )
        .ok_or("The spoken model is unavailable")?
        .to_vec();
        let initial: Option<NativeWritingCheckpoint> = if run.original.is_some() {
            if let Some(reference) = &run.original {
                let value: serde_json::Value = app
                    .read()
                    .repo
                    .get(&reference.local_key)?
                    .ok_or("The saved drawing is unavailable")?;
                Some(serde_json::from_value(value["payload"].clone()).map_err(|e| e.to_string())?)
            } else {
                None
            }
        } else {
            app.read()
                .repo
                .get(&format!("handwriting.native.latest:{}", run.scope()))?
        };
        let request = NativeWritingRequest {
            instance_id: instance,
            scope_id: run.scope(),
            attempt_id: run.attempt_id,
            task_id: task.id.clone(),
            task_revision: task.revision.clone(),
            prompt: task.instruction,
            meaning: task.meaning,
            model_text: task.target,
            comparison_hint: run.definition()?.model_help,
            model_visible: run.model_revealed || run.original.is_some(),
            allow_model_reveal: true,
            allow_listen: true,
            paper: initial
                .as_ref()
                .map(|checkpoint| checkpoint.canonical.paper.clone())
                .unwrap_or(NativePaper {
                    width_points: task.paper.width as f64,
                    height_points: task.paper.height as f64,
                    line_height_points: task.paper.model_size as f64,
                }),
            audio: Some(NativeWritingAudio {
                id: task.reference_audio_id.unwrap(),
                sha256: format!("{:x}", Sha256::digest(&bytes)),
                wav_bytes: bytes,
            }),
            initial,
            initial_compared: run.original.is_some(),
        };
        app.read()
            .writing_native_instance(run.id, run.round_id, Some(instance))?;
        if let Err(error) = handwriting_native::attach_inline(
            request,
            Rc::new(move |event| {
                if alive.get() {
                    on_event.call((run.id, run.round_id, instance, event))
                }
            }),
            host,
        ) {
            app.read()
                .writing_native_instance(run.id, run.round_id, None)?;
            return Err(error);
        }
        Ok(instance)
    })()
}

#[component]
fn NativeWritingPaper(
    mut app: Signal<AppState>,
    run: HandwritingActivity,
    on_event: Callback<(Uuid, Uuid, Uuid, NativeWritingEvent)>,
) -> Element {
    let host_id = use_hook(|| format!("native-paper-{}", Uuid::new_v4()));
    let alive = use_hook(|| Rc::new(std::cell::Cell::new(true)));
    let attached = use_hook(|| Rc::new(std::cell::Cell::new(None::<Uuid>)));
    let mut ready = use_signal(|| false);
    let mut playing = use_signal(|| false);
    let mut finger = use_signal(|| false);
    let mut problem = use_signal(String::new);
    let mut retry = use_signal(|| 0usize);
    let callback_alive = alive.clone();
    let event_handler = use_callback(move |event: (Uuid, Uuid, Uuid, NativeWritingEvent)| {
        if !callback_alive.get() {
            return;
        }
        match &event.3 {
            NativeWritingEvent::Ready { .. } => ready.set(true),
            NativeWritingEvent::AudioStarted { .. } => playing.set(true),
            NativeWritingEvent::AudioCompleted { .. } | NativeWritingEvent::AudioStopped { .. } => {
                playing.set(false)
            }
            NativeWritingEvent::Failure { message, .. } => {
                playing.set(false);
                problem.set(message.clone());
            }
            _ => (),
        }
        on_event.call(event);
    });
    let effect_id = host_id.clone();
    let effect_alive = alive.clone();
    let effect_attached = attached.clone();
    let initial_run = use_hook(|| run.clone());
    use_effect(move || {
        let _ = retry();
        let id = effect_id.clone();
        let alive = effect_alive.clone();
        let attached = effect_attached.clone();
        let run = initial_run.clone();
        spawn(async move {
            let script = format!(
                "const hostId={};\n{}",
                serde_json::to_string(&id).unwrap(),
                include_str!("handwriting_native_host.js")
            );
            let mut bridge = document::eval(&script);
            while let Ok(rect) = bridge
                .recv::<handwriting_native::NativeWritingHostRect>()
                .await
            {
                if !alive.get() {
                    break;
                }
                let result = if let Some(instance) = attached.get() {
                    handwriting_native::update_inline_rect(instance, rect)
                } else if rect.width > 0.0 && rect.height > 0.0 {
                    attach_native(app, run.clone(), event_handler, alive.clone(), rect)
                        .map(|instance| attached.set(Some(instance)))
                } else {
                    Ok(())
                };
                if let Err(error) = result {
                    problem.set(error);
                    break;
                }
            }
        });
    });
    let drop_id = host_id.clone();
    let drop_alive = alive.clone();
    let drop_attached = attached.clone();
    use_drop(move || {
        drop_alive.set(false);
        let id = serde_json::to_string(&drop_id).unwrap();
        let _ = document::eval(&format!("window.drusskyNativePaperHosts?.get({id})?.();"));
        if let Some(instance) = drop_attached.get() {
            if handwriting_native::is_open() {
                // Normal navigation has already flushed and detached. This is
                // recovery for an unexpected unmount, preserving the journal.
                if let Err(error) = handwriting_native::detach_inline(instance) {
                    tracing_notice(&error);
                }
            }
        }
    });
    let writing = run.stage == WritingStage::Writing;
    let visible = run.model_revealed || !writing;
    let task = run.task();
    let listen_id = attached.clone();
    let reveal_id = attached.clone();
    let undo_id = attached.clone();
    let redo_id = attached.clone();
    let finger_id = attached.clone();
    let compare_id = attached.clone();
    rsx! {section {class:"native-writing-paper",
        if writing {p {class:"writing-prompt","{task.instruction}"}}
        if visible {div {class:"writing-example",lang:"ru","{task.target}"}}
        if writing {div {class:"actions",
            button {class:"secondary",disabled:!ready()||playing(),onclick:move |_|{if let Some(id)=listen_id.get(){if let Err(e)=handwriting_native::native_listen(id){problem.set(e)}}},if playing(){"Playing…"}else{"Listen"}}
            if !visible {button {class:"text-button",disabled:!ready(),onclick:move |_|{if let Some(id)=reveal_id.get(){if let Err(e)=handwriting_native::native_reveal(id){problem.set(e)}}},"Show example"}}
        }}
        div {id:host_id,class:"native-paper-host",role:"group",aria_label:"Write here with Apple Pencil"}
        if !ready() && problem().is_empty(){p {role:"status",class:"quiet","Preparing paper…"}}
        if writing {div {class:"actions writing-tools",
            button {class:"text-button",disabled:!ready(),onclick:move |_|{if let Some(id)=undo_id.get(){if let Err(e)=handwriting_native::native_undo(id){problem.set(e)}}},"Undo"}
            button {class:"text-button",disabled:!ready(),onclick:move |_|{if let Some(id)=redo_id.get(){if let Err(e)=handwriting_native::native_redo(id){problem.set(e)}}},"Redo"}
            button {class:"text-button",aria_pressed:finger(),disabled:!ready(),onclick:move |_|{if let Some(id)=finger_id.get(){let next=!finger();match handwriting_native::native_finger(id,next){Ok(())=>finger.set(next),Err(e)=>problem.set(e)}}},if finger(){"Finger drawing on"}else{"Use finger"}}
            button {class:"primary",disabled:!ready()||playing(),onclick:move |_|{if let Some(id)=compare_id.get(){problem.set(String::new());if let Err(e)=handwriting_native::native_compare(id){problem.set(e)}}},"Compare"}
        }}
        if !problem().is_empty(){p {role:"alert","{problem}"}if !ready(){button {class:"secondary",onclick:move |_|{problem.set(String::new());retry+=1;},"Try opening the paper again"}}}
    }}
}

fn tracing_notice(error: &str) {
    // No UI signal may outlive its component during an emergency teardown.
    eprintln!("Native writing recovery: {error}");
}

#[component]
pub(crate) fn Writing(mut app: Signal<AppState>) -> Element {
    let alive = use_hook(|| Rc::new(std::cell::Cell::new(true)));
    let drop_alive = alive.clone();
    use_drop(move || drop_alive.set(false));
    let mut dirty = use_signal(|| false);
    let mut tick = use_signal(|| 0usize);
    let mut catalog_query = use_signal(String::new);
    let mut catalog_group = use_signal(|| "all".to_string());
    let _ = tick();
    let surface = app
        .read()
        .repo
        .get::<String>("handwriting.surface")
        .ok()
        .flatten();
    let catalog = surface.as_deref() == Some("catalog");
    let run = app.read().active_handwriting();
    let on_native = use_callback(
        move |(id, round, instance, event): (Uuid, Uuid, Uuid, NativeWritingEvent)| {
            native_event(app, id, round, instance, event)
        },
    );
    if surface.as_deref() == Some("history") {
        return rsx! {crate::writing_history_view::WritingHistory {app}};
    }
    if catalog || run.is_err() {
        let mut tasks: Vec<_> = handwriting_tasks()
            .into_iter()
            .filter(|task| app.read().handwriting_task_available(task).unwrap_or(false))
            .collect();
        let available_count = tasks.len();
        let query = catalog_query().trim().to_lowercase();
        let group = catalog_group();
        tasks.retain(|task| {
            let category = match task.layout {
                rufren_content::handwriting_tasks::FoundationHandwritingLayout::LetterBox => {
                    "letters"
                }
                rufren_content::handwriting_tasks::FoundationHandwritingLayout::WordLine => "words",
                rufren_content::handwriting_tasks::FoundationHandwritingLayout::PhraseLine => {
                    "messages"
                }
            };
            (group == "all" || group == category)
                && (query.is_empty()
                    || task.target.to_lowercase().contains(&query)
                    || task
                        .meaning
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&query))
        });
        let empty = tasks.is_empty();
        let sections: Vec<_> = [
            (
                "Letters",
                rufren_content::handwriting_tasks::FoundationHandwritingLayout::LetterBox,
            ),
            (
                "Words",
                rufren_content::handwriting_tasks::FoundationHandwritingLayout::WordLine,
            ),
            (
                "Messages",
                rufren_content::handwriting_tasks::FoundationHandwritingLayout::PhraseLine,
            ),
        ]
        .into_iter()
        .map(|(title, layout)| {
            (
                title,
                tasks
                    .iter()
                    .filter(|task| task.layout == layout)
                    .cloned()
                    .collect::<Vec<_>>(),
            )
        })
        .filter(|(_, items)| !items.is_empty())
        .collect();
        let first_group = sections.first().map(|(title, _)| *title).unwrap_or("");
        return rsx! {section {class:"card writing-catalog",style {"{WRITING_CATALOG_STYLE}"}h1 {id:"page-heading","Write and say"}p {"Use a word you have met. Hear it and write, or copy it first; then say the same word or message."}
               button {class:"secondary",onclick:move |_|nav_act(app,|a|a.open_writing_history_list()),"Your writing"}
               label {"Find writing" input {r#type:"search",value:"{catalog_query}",placeholder:"Russian or meaning",oninput:move |event|catalog_query.set(event.value())}}
               fieldset {class:"writing-catalog-filters",legend {"Show"}for (value,label) in [("all","All"),("letters","Letters"),("words","Words"),("messages","Messages")] {button {class:"secondary",r#type:"button",aria_pressed:group == value,onclick:move |_|catalog_group.set(value.to_string()),"{label}"}}}
               p {"{tasks.len()} matching activities · {available_count} available to practise"}
               if empty && available_count > 0 {p {"No matching activities. Try another word or group."}}
               if available_count == 0 {p {"First meet a few sounds and letters. Writing activities appear here as you meet their words."}button {class:"primary",onclick:move |_|nav_act(app,|a|a.start_foundations()),"Start sounds & letters"}}
        for (title, items) in sections {details {class:"writing-catalog-group",open:group != "all" || !query.is_empty() || title == first_group,summary {"{title} · {items.len()}"}for task in items {div {class:"writing-choice",h2 {"{task.target}"}if let Some(meaning)=&task.meaning{p {"{meaning}"}}
                {let id=task.id.clone();rsx!{button {class:"primary",onclick:move |_|{let id=id.clone();nav_act(app,move|a|a.open_writing_task(&id,WritingSequence::HearWriteCompareSay))},"Hear and write"}}}
                {let id=task.id.clone();rsx!{button {class:"secondary",onclick:move |_|{let id=id.clone();nav_act(app,move|a|a.open_writing_task(&id,WritingSequence::SeeCopyHearSay))},"See and copy"}}}
               }}}}
               button {class:"secondary",onclick:move |_|nav_act(app,|a|a.navigate(Page::Foundations)),"Back to my course"}
              }};
    }
    let run = run.unwrap();
    let id = run.id;
    let round = run.round_id;
    let scope = run.scope();
    let task = run.task();
    let definition = run.definition().unwrap();
    let native = handwriting_native::supported();
    let busy = app.read().handwriting_navigation_busy;
    let completed = run.stage == WritingStage::Complete;
    let speech_busy = app.read().writing_speech_busy;
    let compared = matches!(run.stage, WritingStage::Compare | WritingStage::Say);
    rsx! {
     style {"{WRITING_WORKSPACE_STYLE}"}
     section {class:"card writing-lesson",
      header {class:"writing-header",h1 {id:"page-heading",tabindex:"-1","Write and say"}
       button {class:"text-button",disabled:busy||speech_busy,onclick:move |_|nav_act(app,|a|a.navigate(Page::Foundations)),"Pause"}}
      if native {
       NativeWritingPaper {key:"native-{scope}",app,run:run.clone(),on_event:on_native}
      } else {
       HandwritingPad {key:"{scope}",app,scope_id:scope.clone(),task:task.clone(),attempt_id:Some(run.attempt_id),initial_mode:if run.sequence==WritingSequence::SeeCopyHearSay{InkMode::Copy}else{InkMode::Hidden},on_saved:move|original|{crate::act(app,|a|a.writing_accept_web(id,round,&original));tick+=1;dirty.set(false)},on_dirty:move|value|dirty.set(value),on_audio_completed:move|asset:String|{crate::act(app,|a|a.writing_audio_completed(id,round,&asset));}}
      }
      if compared || completed {
       section {class:"writing-response",
        if let Some(meaning)=&definition.meaning{p {class:"writing-meaning","{meaning}"}}
        if !completed {p {"{definition.say_instruction}"}}
        if let Ok(binding)=crate::writing_speech::WritingSpeechBinding::from_run(&run){crate::writing_speech::WritingSpeech {app,binding,on_busy:move|value|app.write().writing_speech_busy=value}}
        details {summary {"Writing tips"}p {"{definition.model_help}"}if let Some(grammar)=&definition.grammar_note{p {"{grammar}"}}}
       }
       if compared {footer {class:"actions writing-next",
        button {class:"primary",disabled:busy||dirty()||speech_busy||!run.audio_completed,onclick:move |_|nav_act(app,move|a|a.writing_finish_and_continue(id,round,SayDisposition::SelfReportedSaid)),"I said it · Next"}
        button {class:"text-button",disabled:busy||dirty()||speech_busy,onclick:move |_|nav_act(app,move|a|a.writing_finish_and_continue(id,round,SayDisposition::Skipped)),"Skip speaking"}
       }}
       if completed {footer {class:"actions",
        button {class:"primary",disabled:busy||speech_busy,onclick:move |_|nav_act(app,move|a|a.writing_finish_and_continue(id,round,SayDisposition::Skipped)),"Continue"}
        button {class:"secondary",disabled:busy||speech_busy,onclick:move |_|nav_act(app,move|a|a.writing_new_round(id,round,WritingSequence::HearWriteCompareSay)),"Try from listening"}
        button {class:"text-button",disabled:busy||speech_busy,onclick:move |_|nav_act(app,|a|a.open_writing_history_list()),"Your writing"}
       }}
      }
     }
    }
}

const WRITING_CATALOG_STYLE: &str = r#".writing-catalog-filters{display:flex;gap:8px;flex-wrap:wrap;border:0;padding:0;margin:16px 0}.writing-catalog-filters legend{margin-bottom:8px}.writing-catalog-filters button{min-height:44px;min-width:64px}.writing-catalog-filters button[aria-pressed=true]{background:#244f42;color:white}.writing-catalog-group>summary{min-height:44px;display:list-item;align-content:center;cursor:pointer}.writing-catalog-group .writing-choice{padding:12px 0;border-top:1px solid #d4ddd4}"#;

const WRITING_WORKSPACE_STYLE: &str = r#"
.writing-lesson{max-width:900px;margin:0 auto;padding:24px}
.writing-header{display:flex;align-items:center;justify-content:space-between;gap:16px;margin-bottom:12px}
.writing-header h1{font-size:1.5rem;margin:0}.writing-prompt{margin:8px 0 14px}
.writing-example{font-size:clamp(2rem,5vw,3.3rem);line-height:1.25;margin:12px 0;white-space:pre-wrap}
.native-paper-host{height:clamp(240px,34vh,360px);width:100%;border:1px solid var(--line,#ccd4cc);border-radius:12px;background:var(--paper,#faf9f3);overflow:hidden}
.writing-tools{display:flex;align-items:center;flex-wrap:wrap;gap:8px}.writing-tools .primary{margin-left:auto}
.writing-response{margin-top:16px}.writing-response>p{margin:8px 0}.writing-response details{font-size:.95rem;margin:12px 0}
.writing-next{display:flex;align-items:center;gap:12px;margin-top:16px}
.writing-lesson button{min-height:44px}.writing-meaning{font-size:1.1rem;color:var(--muted)}
.writing-lesson .handwriting-pad{max-width:none}.writing-lesson .handwriting-pad[data-sealed=true] .ink-instruction,
.writing-lesson .handwriting-pad[data-sealed=true] .ink-pad-heading,
.writing-lesson .handwriting-pad[data-sealed=true] .ink-guidance,
.writing-lesson .handwriting-pad[data-sealed=true] .ink-sound,
.writing-lesson .handwriting-pad[data-sealed=true] .ink-context,
.writing-lesson .handwriting-pad[data-sealed=true]>div.actions,
.writing-lesson .handwriting-pad[data-sealed=true]>[data-ink-status],
.writing-lesson .handwriting-pad[data-sealed=true]>details{display:none}
@media(max-width:600px){.writing-lesson{padding:16px}.native-paper-host{height:260px}.writing-lesson .ink-model svg,.writing-lesson .ink-comparison svg{height:112px}}
"#;
