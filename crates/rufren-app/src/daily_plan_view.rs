//! A small amount of practice, with an explicit stopping point.
use crate::{
    act,
    daily_plan::{DailyPlanItemState, DailyPlanSettings},
    model::{AppState, Page},
    repository::now_ms,
};
use dioxus::prelude::*;

const STYLE: &str = ".daily-plan {margin:24px 0}.daily-plan-list {list-style:none;padding:0;margin:16px 0}.daily-plan-item {border-top:1px solid var(--line);padding:16px 0;display:flex;justify-content:space-between;gap:18px}.daily-plan-item h3 {font-size:1rem;margin:8px 0}.daily-plan-item p {margin:6px 0}.daily-plan-item .actions {flex-shrink:0;align-self:center;margin-top:8px}.daily-plan-controls {display:flex;gap:20px;align-items:center;flex-wrap:wrap;margin:16px 0}.daily-plan-controls select {width:auto;min-width:120px}.daily-plan-status {font-weight:600}.daily-plan summary {cursor:pointer;min-height:44px;padding:12px 0}@media(max-width:600px){.daily-plan-item{flex-direction:column;gap:6px}.daily-plan-item .actions{align-self:stretch}.daily-plan-controls{align-items:flex-start;flex-direction:column;gap:8px}}";

#[component]
pub(crate) fn DailyPractice(app: Signal<AppState>) -> Element {
    let state = app.read().clone();
    let now = now_ms();
    let settings = state.daily_plan_settings();
    let plan = state.daily_plan(now);
    let (settings, plan) = match (settings, plan) {
        (Ok(settings), Ok(plan)) => (settings, plan),
        (Err(problem), _) | (_, Err(problem)) => {
            return rsx! {section {class:"panel daily-plan",h2 {"Your short session"}p {role:"alert","{problem}"}p {"Your saved practice is kept. You can still choose a lesson or passage."}}};
        }
    };
    let budget = settings.response_budget;
    let quiet = settings.quiet;
    let finished = plan.as_ref().is_some_and(|plan| plan.is_finished());
    let empty = plan.as_ref().is_some_and(|plan| plan.items.is_empty());
    let plan_id = plan
        .as_ref()
        .map(|plan| plan.id.to_string())
        .unwrap_or_default();
    let created = plan
        .as_ref()
        .map(|plan| plan.created_at_ms.to_string())
        .unwrap_or_default();
    rsx! {
        style {"{STYLE}"}
        section {class:"panel daily-plan",aria_label:"Your short practice session","data-daily-plan":plan_id,"data-daily-plan-created":created,
            h2 {if empty {"Room for something new"}else if finished {"A good place to stop"}else{"A little practice, at your pace"}}
            if let Some(plan)=&plan {
                if !empty {p {class:"daily-plan-status",role:"status","{plan.completed_count()} of {plan.items.len()} responses saved"}}
                if empty {
                    p {if plan.settings.quiet {"No reading or writing returns were selected for this session. You can explore a lesson, or turn quiet practice off for your next session."}else{"No practiced tasks were ready when you chose this session. Explore a lesson, have a conversation, or choose a passage freely."}}
                } else if finished {
                    p {"Your session is finished. The rest can wait; there’s no daily quota to catch up on."}
                } else {
                    p {class:"quiet","A mix of practiced material, with room for different skills. Pause whenever you like; this session stays here."}
                }
                if plan.settings.quiet {p {class:"quiet","Quiet session · reading and writing"}}
                ol {class:"daily-plan-list",
                    for item in &plan.items {
                        {let index=item.index;let ready=matches!(item.state,DailyPlanItemState::Ready);let unavailable=matches!(item.state,DailyPlanItemState::Unavailable{..});
                        rsx! {li {key:"daily-item-{index}",class:"daily-plan-item","data-daily-plan-index":index.to_string(),"data-daily-source":match item.key {rufren_core::PracticeBatchKey::Teacher{..}=>"teacher",rufren_core::PracticeBatchKey::Passage{..}=>"passage"},
                            div {span {class:"tag","{item.skill.label()}"}h3 {"{item.title}"}
                                match &item.state {
                                    DailyPlanItemState::Ready=>rsx!{details {summary {"Why this practice?"}p {class:"quiet","{item.reason}"}}},
                                    DailyPlanItemState::Completed{..}=>rsx!{p {class:"quiet","Response saved"}},
                                    DailyPlanItemState::Skipped=>rsx!{p {class:"quiet","Left for another time"}},
                                    DailyPlanItemState::Unavailable{reason}=>rsx!{p {class:"quiet","{reason}"}},
                                }
                            }
                            if ready || unavailable {div {class:"actions",
                                if ready {button {class:"secondary",onclick:move |_|act(app,|s|s.start_daily_plan_item(index,now_ms())),"Try this →"}}
                                button {class:"text-button",onclick:move |_|act(app,|s|s.skip_daily_plan_item(index)),"Leave for later"}
                            }}
                        }}}
                    }
                }
                if finished {div {class:"actions",
                    button {class:"secondary",onclick:move |_|act(app,|s|{s.new_daily_plan(now_ms())?;s.begin()}),"Choose another short session"}
                    button {class:"text-button",onclick:move |_|act(app,|s|s.navigate(Page::Lessons)),"Explore a lesson"}
                    button {class:"text-button",onclick:move |_|act(app,|s|s.navigate(Page::Teacher)),"Talk with your teacher"}
                }} else if plan.next_ready_index().is_some() {button {class:"primary",onclick:move |_|act(app,|s|s.begin()),"Continue this session →"}}
            } else {
                p {"Choose a few returns from things you’ve practiced. New lessons stay available whenever curiosity takes you there."}
                button {class:"primary",onclick:move |_|act(app,|s|s.begin()),"Choose my next step →"}
            }
            details {summary {if plan.is_some(){"For your next session"}else{"Make room for today"}}
                div {class:"daily-plan-controls",
                    div {label {r#for:"daily-plan-budget","Up to this many responses"}
                        select {id:"daily-plan-budget",value:budget.to_string(),onchange:move |event|act(app,|s|{let mut settings=s.daily_plan_settings()?;settings.response_budget=event.value().parse().map_err(|_|"Choose one, three, or five responses")?;s.save_daily_plan_settings(settings)}),
                            for count in [1,3,5] {option {value:count.to_string(),"{count}"}}
                        }
                    }
                    div {class:"preference-row",input {id:"daily-plan-quiet",r#type:"checkbox",checked:quiet,onchange:move |event|act(app,|s|{let mut settings:DailyPlanSettings=s.daily_plan_settings()?;settings.quiet=event.checked();s.save_daily_plan_settings(settings)})}label {r#for:"daily-plan-quiet","Quiet practice: reading and writing"}}
                }
                p {class:"quiet","There’s no timer. Your choices apply when you start a new session; the current one stays as you left it."}
            }
        }
    }
}

/// Used only after an actual response has been saved. Completing a plan is not a grade.
#[component]
pub(crate) fn SessionNext(app: Signal<AppState>, #[props(default)] disabled: bool) -> Element {
    let plan = app.read().daily_plan(now_ms());
    match plan {
        Ok(Some(plan)) if plan.is_finished() => rsx! {
            p {class:"quiet",role:"status","Your short session is finished. You can stop here."}
        },
        Ok(Some(plan)) if plan.next_ready_index().is_some() => rsx! {
            button {class:"primary",disabled,onclick:move |_|act(app,|s|s.begin()),"Next in this session →"}
        },
        _ => rsx! {},
    }
}
