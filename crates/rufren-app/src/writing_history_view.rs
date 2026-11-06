//! Read-only ink and voice history; reviewing never replaces the current round.
use crate::{
    audio_studio::AudioReviewPlayer,
    handwriting::*,
    handwriting_native::{NativeWritingCheckpoint, native_drawing_preview},
    handwriting_view::nav_act,
    model::AppState,
};
use dioxus::prelude::*;
use uuid::Uuid;

const STYLE: &str = r#"
.writing-history-list{display:grid;gap:12px;margin:20px 0}
.writing-history-item{display:flex;align-items:center;justify-content:space-between;gap:16px;padding:16px;border:1px solid var(--line,#d8ddd4);border-radius:12px}
.writing-history-item h2,.writing-history-item p{margin:0 0 4px}
.writing-history-actions{display:flex;gap:10px;flex-wrap:wrap;margin:18px 0}
.writing-history-rounds{display:flex;gap:8px;flex-wrap:wrap;margin:12px 0}
.writing-history-rounds [aria-pressed=true]{outline:2px solid var(--green,#375e50);outline-offset:-2px}
.writing-original{border:1px solid var(--line,#d8ddd4);border-radius:12px;padding:12px;background:#fff;margin:16px 0}
.writing-original canvas,.writing-original img{display:block;width:100%;height:auto;max-height:65vh;object-fit:contain}
"#;

fn route(
    sequence: rufren_content::handwriting_tasks::FoundationHandwritingSequence,
) -> &'static str {
    use rufren_content::handwriting_tasks::FoundationHandwritingSequence::*;
    match sequence {
        SeeCopyHearSay => "Copying",
        HearWriteCompareSay => "Listening and writing",
    }
}

#[component]
pub(crate) fn WritingHistory(app: Signal<AppState>) -> Element {
    let mut selected = use_signal(|| None::<Uuid>);
    let loaded = app.read().handwriting_history();
    let runs = match loaded {
        Ok(runs) => runs,
        Err(error) => {
            return rsx! {section {class:"card",h1 {"Your writing"}p {role:"alert","{error}"}button {class:"secondary",onclick:move |_|nav_act(app,|a|a.open_writing_catalog()),"Back to writing activities"}}};
        }
    };
    if let Some(id) = selected() {
        if let Some(run) = runs.iter().find(|run| run.id == id) {
            return rsx! {style {"{STYLE}"}section {class:"card",
                button {class:"text-button",onclick:move |_|selected.set(None),"← Your writing"}
                WritingHistoryDetail {key:"{id}",app,run:run.clone()}
            }};
        }
    }
    rsx! {style {"{STYLE}"}section {class:"card",
        h1 {id:"page-heading",tabindex:"-1","Your writing"}
        p {"Revisit a drawing, hear your saved voice, or pick up an unfinished activity."}
        if runs.is_empty(){p {"Your writing will appear here after you start an activity."}}
        div {class:"writing-history-list",
            for run in runs {
                {let id=run.id;let label=run.definition().map(|t|t.target).unwrap_or_else(|_|"Saved writing".into());
                 let state=if run.stage==WritingStage::Complete{"Saved practice"}else{"In progress"};
                 rsx! {article {class:"writing-history-item","data-history-run":id.to_string(),
                    div {h2 {lang:"ru","{label}"}p {"{state} · {route(run.sequence)}"}}
                    button {class:"secondary",onclick:move |_|selected.set(Some(id)),"Open"}
                 }}}
            }
        }
        button {class:"secondary",onclick:move |_|nav_act(app,|a|a.open_writing_catalog()),"Back to writing activities"}
    }}
}

#[component]
fn WritingHistoryDetail(mut app: Signal<AppState>, run: HandwritingActivity) -> Element {
    let mut selected_round = use_signal(|| run.round_id);
    let id = run.id;
    let review = use_memo(move || app.read().handwriting_round_review(id, selected_round()));
    let title = run
        .definition()
        .map(|t| t.target)
        .unwrap_or_else(|_| "Saved writing".into());
    let current = run.round_id;
    rsx! {
        h1 {id:"page-heading",tabindex:"-1",lang:"ru","{title}"}
        if !run.history.is_empty(){
            div {class:"writing-history-rounds",role:"group",aria_label:"Choose a writing try",
                for (index,round) in run.history.iter().enumerate(){
                    {let round_id=round.round_id;rsx!{button {class:"secondary",aria_pressed:selected_round()==round_id,onclick:move |_|selected_round.set(round_id),"Try {index+1} · {route(round.sequence)}"}}}
                }
                button {class:"secondary",aria_pressed:selected_round()==current,onclick:move |_|selected_round.set(current),"Latest try · {route(run.sequence)}"}
            }
        }
        {match &*review.read(){
            Ok(review)=>rsx!{WritingRoundHistory {key:"{review.round_id}",app,review:review.clone()}},
            Err(error)=>rsx!{p {role:"alert","{error}"}},
        }}
        div {class:"writing-history-actions",
            if run.stage!=WritingStage::Complete {button {class:"primary",onclick:move |_|nav_act(app,move|a|a.open_handwriting_history(id)),"Continue this activity"}}
            button {class:"secondary",onclick:move |_|{
                let result=app.read().handwriting_export_run(id);
                match result{Ok(value)=>{let text=serde_json::to_string_pretty(&value).unwrap();spawn(async move{if let Err(error)=crate::audio_studio::export_json(text,format!("drussky-writing-{id}.json")).await{app.write().notice=error;}});},Err(error)=>app.write().notice=error}
            },"Save activity copy"}
        }
    }
}

#[component]
fn WritingRoundHistory(app: Signal<AppState>, review: WritingRoundReview) -> Element {
    let initial = review
        .original
        .as_ref()
        .map(|o| o.id)
        .or_else(|| review.originals.first().map(|o| o.reference.id));
    let mut selected = use_signal(|| initial);
    let chosen = review
        .originals
        .iter()
        .find(|o| Some(o.reference.id) == selected());
    let voice = app.read().writing_speech_for_review(&review);
    rsx! {
        p {class:"quiet","{route(review.sequence)}"}
        if let Some(original)=chosen{SavedInk {key:"{original.reference.id}:{original.reference.sha256}",original:original.clone(),task:review.task.clone()}}
        else {p {"This try has no compared drawing yet. Continue the activity to finish it."}}
        if review.originals.len()>1{
            details {summary {"Other saved drawings from this try"}div {class:"writing-history-rounds",
                for (index,original) in review.originals.iter().enumerate(){
                    {let id=original.reference.id;let accepted=review.original.as_ref().is_some_and(|o|o.id==id);rsx!{button {class:"secondary",aria_pressed:selected()==Some(id),onclick:move |_|selected.set(Some(id)),if accepted{"Compared drawing"}else{"Saved drawing {index+1}"}}}}
                }
            }}
        }
        if chosen.is_some_and(|o|review.original.as_ref().is_some_and(|accepted|accepted.id==o.reference.id)){
            match voice {
                Ok(Some(record))=>rsx!{AudioReviewPlayer {asset_id:record.asset_id,expected_hash:Some(record.content_hash),label:"Your voice for this drawing".to_string()}},
                Ok(None)=>rsx!{p {class:"quiet","No voice was saved with this drawing."}},
                Err(error)=>rsx!{p {role:"alert","{error}"}},
            }
        }
    }
}

#[component]
fn SavedInk(original: WritingOriginalReview, task: HandwritingTask) -> Element {
    let canvas = use_hook(|| format!("saved-ink-{}", Uuid::new_v4()));
    let mut error = use_signal(String::new);
    let native = original.reference.format == "apple-pencilkit-control-points-v1";
    let preview = use_hook(|| {
        if !native {
            return Ok(None);
        }
        let checkpoint: NativeWritingCheckpoint =
            serde_json::from_value(original.payload.clone()).map_err(|e| e.to_string())?;
        native_drawing_preview(&checkpoint)
    });
    let node = canvas.clone();
    use_effect(move || {
        if native {
            return;
        }
        let node = node.clone();
        let original = original.clone();
        let task = task.clone();
        spawn(async move {
            let config = serde_json::json!({"model_version":task.model_version,"scope_id":original.reference.scope_id,"target":task.target,"task_id":task.id,"task_revision":task.revision,"paper":task.paper});
            let script = format!(
                "{}\nconst canvas=document.getElementById({});if(!canvas)return false;return window.RufrenHandwriting.preview(canvas,{},{});",
                include_str!("handwriting.js"),
                serde_json::to_string(&node).unwrap(),
                original.payload,
                config
            );
            if let Err(problem) = document::eval(&script).await {
                error.set(problem.to_string());
            }
        });
    });
    rsx! {figure {class:"writing-original",
        if native {match preview {
            Ok(Some(source))=>rsx!{img {src:source,alt:"Your original Pencil drawing"}},
            Ok(None)=>rsx!{p {"This Pencil drawing is preserved. Open it on your iPad to view it."}},
            Err(problem)=>rsx!{p {role:"alert","{problem}"}},
        }} else {canvas {id:canvas,role:"img",aria_label:"Your saved handwriting",width:"400",height:"300","Your saved drawing"}}
        if !error().is_empty(){p {role:"alert","{error}"}}
    }}
}
