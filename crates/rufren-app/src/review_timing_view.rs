//! Small timing choices beside the work they affect.
use crate::{act, model::AppState, repository::now_ms};
use dioxus::prelude::*;
use rufren_core::*;

const STYLE: &str = include_str!("review_timing_view.css");

pub(crate) fn timing_label(due: Option<i64>, paused: bool, now: i64) -> String {
    if paused {
        return "Paused until you choose to resume".into();
    }
    let Some(due) = due else {
        return "Waiting for saved timing history".into();
    };
    let remaining = due.saturating_sub(now);
    if remaining <= 0 {
        "Ready when you are".into()
    } else if remaining < 60 * MINUTE_MS {
        "In less than an hour".into()
    } else if remaining < DAY_MS {
        let hours = (remaining + 30 * MINUTE_MS) / (60 * MINUTE_MS);
        format!(
            "In about {hours} {}",
            if hours == 1 { "hour" } else { "hours" }
        )
    } else {
        let days = remaining.saturating_add(DAY_MS / 2) / DAY_MS;
        format!("In about {days} {}", if days == 1 { "day" } else { "days" })
    }
}

#[component]
pub(crate) fn TaskTimingControls(
    app: Signal<AppState>,
    target: ReviewTarget,
    baseline_due_at_ms: i64,
    now: i64,
) -> Element {
    let timing = app.read().review_timing(target, baseline_due_at_ms, now);
    let mut timing_open = use_signal(|| false);
    let stamp = timing_label(timing.effective_due_at_ms, timing.paused, now);
    let baseline = timing_label(Some(baseline_due_at_ms), false, now);
    let heads = timing.heads.clone();
    let sooner_heads = heads.clone();
    let later_heads = heads.clone();
    let today_heads = heads.clone();
    let later = timing
        .effective_due_at_ms
        .unwrap_or(now)
        .max(now)
        .saturating_add(DAY_MS);
    rsx! {
        style {"{STYLE}"}
        details {open:timing_open(),class:"review-timing", "data-review-timing":format!("{target:?}"),"data-review-target":serde_json::to_string(&target).unwrap_or_default(),"data-review-heads":serde_json::to_string(&timing.heads).unwrap_or_default(),"data-review-paused":timing.paused.to_string(),"data-review-conflicted":timing.conflicted.to_string(),"data-review-pending":timing.pending.to_string(),"data-review-due":timing.effective_due_at_ms.map(|t|t.to_string()).unwrap_or_default(),
            summary {onclick:move |event|{event.prevent_default();timing_open.toggle();},"Timing"}
            p {class:"review-timing-status",role:"status","{stamp}"}
            if timing.conflicted {p {class:"quiet","Different timing choices were synced. Choose the timing you want to use."}}
            if timing.pending {p {class:"quiet","Some saved timing history has not arrived yet. Sync your notebook to recover it."}}
            div {class:"review-timing-actions",
                button {class:"secondary",disabled:timing.pending,onclick:move |_|act(app,|s|s.change_review_timing(target,ReviewTimingAction::AvailableAt{at_ms:now_ms()},sooner_heads.clone(),now_ms())),"Sooner · now"}
                button {class:"secondary",disabled:timing.pending,onclick:move |_|act(app,|s|s.change_review_timing(target,ReviewTimingAction::AvailableAt{at_ms:later},later_heads.clone(),now_ms())),"Later · one day"}
                button {class:"text-button",disabled:timing.pending,onclick:move |_|act(app,|s|{let now=now_ms();s.change_review_timing(target,ReviewTimingAction::AvailableAt{at_ms:now.saturating_add(DAY_MS)},today_heads.clone(),now)}),"Tomorrow"}
                button {class:if timing.paused {"secondary"}else{"text-button"},disabled:timing.pending,
                    onclick:move |_|act(app,|s|s.change_review_timing(target,if timing.paused {ReviewTimingAction::Resume}else{ReviewTimingAction::Pause},heads.clone(),now_ms())),
                    if timing.paused {"Resume returns"}else{"Pause this task"}
                }
            }
            p {class:"quiet","Tomorrow leaves this task until about this time tomorrow. Later adds a day to its current timing. These choices keep your saved responses."}
            details {class:"review-timing-details",summary {"Original plan"}
                p {class:"quiet","{baseline}. This is the date from your practice history, before your timing choice."}
            }
        }
    }
}

#[component]
pub(crate) fn ReviewTimingSettings(app: Signal<AppState>) -> Element {
    let cadence = app.read().review_cadence(now_ms());
    let selected = if cadence.conflicted || cadence.pending {
        "choose"
    } else {
        match cadence.cadence {
            ReviewCadence::MoreOften => "more-often",
            ReviewCadence::Standard => "standard",
            ReviewCadence::MoreSpace => "more-space",
        }
    };
    let projected_label = match cadence.cadence {
        ReviewCadence::MoreOften => "More often",
        ReviewCadence::Standard => "Standard",
        ReviewCadence::MoreSpace => "More space",
    };
    let heads = cadence.heads.clone();
    rsx! {
        style {"{STYLE}"}
        section {class:"panel review-cadence",aria_label:"Return timing","data-review-heads":serde_json::to_string(&cadence.heads).unwrap_or_default(),"data-review-conflicted":cadence.conflicted.to_string(),"data-review-pending":cadence.pending.to_string(),
            h2 {"Return timing"}
            label {r#for:"review-cadence","Space between future returns"}
            select {id:"review-cadence",value:selected,onchange:move |event|act(app,|s|{
                let choice=match event.value().as_str(){"more-often"=>ReviewCadence::MoreOften,"more-space"=>ReviewCadence::MoreSpace,_=>ReviewCadence::Standard};
                s.change_review_cadence(choice,heads.clone(),now_ms())
            }),
                if cadence.conflicted || cadence.pending {option {value:"choose",disabled:true,"Choose timing · currently {projected_label}"}}
                option {value:"more-often","More often"}
                option {value:"standard","Standard"}
                option {value:"more-space","More space"}
            }
            p {class:"quiet","Applies to practice you start next. Your current return dates and unfinished responses stay as they are."}
            if cadence.conflicted && !cadence.pending {p {class:"quiet",role:"status",
                if cadence.fallback_reason.is_some() {"Different timing preferences were synced. {projected_label} is in use until you choose one."}
                else {"Matching timing choices were synced. {projected_label} is in use; choose it again to confirm a shared choice."}
            }}
            if cadence.pending {p {class:"quiet",role:"status","Some saved timing history is missing. Standard is in use while it is recovered."}}
            details {class:"review-timing-details",summary {"What changes?"}
                p {class:"quiet","More often halves the usual gap; More space doubles it. Gaps stay between ten minutes and 120 days. These are adjustable starting choices, not estimates of what you remember."}
                p {class:"quiet","Each new response keeps the timing choice it started with. To move or pause an existing return, open its Timing controls."}
            }
        }
    }
}

#[component]
pub(crate) fn ReviewStartDetails(
    app: Signal<AppState>,
    session: uuid::Uuid,
    target: ReviewTarget,
) -> Element {
    let decision = app.read().review_start_decision(session, target);
    rsx! {
        style {"{STYLE}"}
        details {class:"review-timing-details","data-review-start-session":session.to_string(),
            summary {"Timing when this practice began"}
            match decision {
                Ok(Some(decision)) => {
                    let cadence = match decision.cadence {ReviewCadence::MoreOften=>"More often",ReviewCadence::Standard=>"Standard",ReviewCadence::MoreSpace=>"More space"};
                    let due = timing_label(decision.effective_due_at_ms,decision.timing_paused,decision.started_at_ms);
                    let sources = decision.evidence_total_count;
                    rsx! {
                        p {"Future gap choice: {cadence}."}
                        p {"Availability at that start: {due}."}
                        p {class:"quiet","{decision.reason}"}
                        if let Some(reason)=decision.cadence_fallback_reason {p {class:"quiet","{reason}"}}
                        details {summary {"Saved sources"}
                            p {class:"quiet","{sources} saved history records were available when this practice began. Its timing choice is kept even if later practice or preferences change."}
                        }
                    }
                },
                Ok(None)=>rsx! {p {class:"quiet","This earlier practice has no saved timing choice from its start. Its responses are kept; a past choice has not been reconstructed."}},
                Err(problem)=>rsx! {p {class:"quiet",role:"alert","{problem}"}},
            }
        }
    }
}

#[component]
pub(crate) fn PassageReturns(app: Signal<AppState>) -> Element {
    let state = app.read().clone();
    let now = now_ms();
    let mut entries: Vec<_> = SpacedReview
        .queue(&state.library, now, &Skill::ALL)
        .into_iter()
        .filter(|entry| entry.state.attempts > 0)
        .collect();
    entries.sort_by_key(|entry| (entry.state.key.revision, entry.state.key.skill));
    rsx! {
        style {"{STYLE}"}
        section {class:"passage-returns",aria_label:"Saved passage returns",
            if entries.is_empty(){p {class:"quiet","Passages you practice will appear here for another try."}}
            for (index,entry) in entries.iter().enumerate() {
                {let key=entry.state.key;let target=PracticeBatchKey::Passage{revision:key.revision,skill:key.skill};let timing=state.review_timing(target,entry.state.due_at_ms,now);let stamp=timing_label(timing.effective_due_at_ms,timing.paused,now);let ordinal=index+1;let row_key=format!("{}-{:?}",key.revision,key.skill);
                rsx! {div {key:"{row_key}",class:"passage-return-row",
                    div {class:"passage-return-description",
                        span {class:"tag","{key.skill.label()}"}
                        h3 {"Saved passage · item {ordinal}"}
                        p {class:"quiet","{stamp}"}
                        TaskTimingControls {app,target,baseline_due_at_ms:entry.state.due_at_ms,now}
                    }
                    div {class:"actions",
                        button {class:"secondary",onclick:move |_|act(app,|s|{s.start(key.revision,key.skill)?;s.phase(Phase::Retrieve)}),"Practice now →"}
                        button {class:"text-button",onclick:move |_|act(app,|s|{s.edit(key.revision)?;s.navigate(crate::model::Page::Library)}),"View source"}
                    }
                }}}
            }
        }
    }
}
