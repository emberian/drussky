//! A useful recent-practice view, with separate activity and delayed evidence.
use crate::{
    act,
    model::{AppState, Page},
    progress::ProgressTarget,
    repository::now_ms,
};
use dioxus::prelude::*;
use rufren_core::{DAY_MS, MINUTE_MS};

const STYLE: &str = ".progress-skill-grid{display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:16px}.progress-skill-card{display:flex;flex-direction:column;gap:8px;min-width:0}.progress-skill-card strong{font:42px Georgia}.progress-skill-card p{margin:0}.progress-skill-card .text-button{margin-top:auto;text-align:left}.progress-signals{display:flex;gap:32px;flex-wrap:wrap;margin:20px 0}.progress-signals strong{display:block;font:32px Georgia}.progress-signals p{max-width:23rem;margin:4px 0}.progress-recent{margin-top:24px}.progress-practice-row{display:flex;gap:18px;align-items:center;justify-content:space-between;padding:18px 0;border-top:1px solid var(--line)}.progress-practice-row>div{min-width:0}.progress-practice-row strong,.progress-practice-row small{display:block}.progress-practice-row small{margin-top:6px;color:var(--muted)}.progress-practice-row .secondary{flex-shrink:0}.progress-clock{margin:20px 0}.progress-clock>summary{cursor:pointer;min-height:44px;padding:10px 0}.progress-clock li{margin:10px 0}.progress-chat{margin-top:24px}@media(max-width:1000px){.progress-skill-grid{grid-template-columns:repeat(2,minmax(0,1fr))}}@media(max-width:600px){.progress-page-heading{align-items:flex-start;flex-direction:column;gap:12px}.progress-practice-row{align-items:stretch;flex-direction:column}.progress-signals{gap:18px}.progress-skill-card{padding:18px}.progress-skill-card .tag{font-size:10px}}";

fn age(timestamp: i64, now: i64) -> String {
    let gap = now.saturating_sub(timestamp);
    if gap < DAY_MS {
        "Within the last day".into()
    } else if gap < 2 * DAY_MS {
        "A day ago".into()
    } else {
        format!("{} days ago", gap / DAY_MS)
    }
}
fn ahead(timestamp: i64, now: i64) -> String {
    let gap = timestamp.saturating_sub(now);
    if gap >= DAY_MS {
        format!("About {} days ahead", (gap + DAY_MS / 2) / DAY_MS)
    } else if gap >= 60 * MINUTE_MS {
        format!(
            "About {} hours ahead",
            (gap + 30 * MINUTE_MS) / (60 * MINUTE_MS)
        )
    } else {
        "Less than an hour ahead".into()
    }
}
fn open(app: Signal<AppState>, target: ProgressTarget) {
    act(app, |state| state.open_progress_target(target));
    crate::focus_page_heading();
}

#[component]
pub(crate) fn Progress(app: Signal<AppState>) -> Element {
    let mut clock = use_signal(now_ms);
    use_future(move || async move {
        loop {
            if document::eval("await new Promise(resolve => setTimeout(resolve, 30000));")
                .await
                .is_err()
            {
                break;
            }
            clock.set(now_ms());
        }
    });
    let now = clock();
    let snapshot = app.read().progress_snapshot(now);
    let delayed = &snapshot.delayed;
    rsx! {
        style {"{STYLE}"}
        div {class:"page-heading progress-page-heading",div {
            div {class:"eyebrow","YOUR LAST SEVEN DAYS"}
            h1 {"Keep all four skills in view"}
            p {"See what you have tried, then choose a useful next step. These records are not proficiency scores."}
        }button {class:"secondary",onclick:move |_|{act(app,|s|s.navigate(Page::Today));crate::focus_page_heading();},"See today’s plan →"}}
        div {class:"progress-skill-grid",
            for activity in &snapshot.skills {
                {let next=activity.next;let skill=activity.skill;rsx! {
                    section {class:"panel progress-skill-card","data-progress-skill":skill.label().to_lowercase(),
                        span {class:"tag","{skill.label()}"}
                        strong {"{activity.responses}"}
                        p {if activity.responses==1 {"completed response"}else{"completed responses"}}
                        p {class:"quiet","{activity.with_support} with recorded support · {activity.no_self_report} without a self-report"}
                        if let Some(target)=next {
                            button {class:"text-button",onclick:move |_|open(app,target),"Revisit {skill.label()} →"}
                        }else {button {class:"text-button",onclick:move |_|{act(app,|s|s.navigate(Page::Today));crate::focus_page_heading();},"Choose some practice →"}}
                    }
                }}
            }
        }
        section {class:"panel progress-recent",aria_label:"Delayed Teacher evidence",
            h2 {"Coming back after a gap"}
            p {"Recent returns to the same saved Teacher questions, using their recorded timing and support."}
            div {class:"progress-signals",
                div {strong {"{delayed.self_reported}"}p {"You reported remembering after the planned gap"}}
                div {strong {"{delayed.ai_correct}"}p {"Your AI teacher marked the delayed response correct"}}
            }
            p {class:"quiet","These describe these saved questions; they are not a proficiency score."}
            details {summary {"How to read this"}
                p {"Your own impression and your AI teacher’s language judgment are different signals; they may describe the same return. Each channel counts at most one qualified step for responses saved at the same instant."}
                if delayed.helped+delayed.replayed+delayed.unrated+delayed.recent_restudy+delayed.contested+delayed.another_look>0 {
                    p {class:"quiet","Other returns: {delayed.helped} with help · {delayed.replayed} with replay · {delayed.unrated} left unrated · {delayed.recent_restudy} after recent restudy · {delayed.contested} with differing judgments · {delayed.another_look} needing another look."}
                }
                if delayed.early>0 {p {class:"quiet","{delayed.early} early responses are useful practice; they do not count as completing the planned gap."}}
            }
        }
        details {class:"panel progress-recent",summary {"Browse all your saved Teacher questions"}
            crate::teacher_revisit_view::TeacherReturns {app,expanded:true}
        }
        section {class:"panel progress-recent",aria_label:"Recent completed practice",
            h2 {"Pick up a useful thread"}
            if snapshot.recent.is_empty() {p {"No completed practice is recorded in the last seven days. Start small; earlier work remains saved."}}
            for row in snapshot.recent.iter().take(12) {
                {let target=row.target;rsx! {
                    div {class:"progress-practice-row","data-progress-attempt":row.attempt_id.to_string(),
                        div {span {class:"tag","{row.skill.label()}"}strong {"{row.title}"}
                            small {"{row.support} · {age(row.occurred_at_ms,now)}"}
                            if let Some(outcome)=&row.outcome {small {"{outcome}"}}
                        }
                        if let Some(target)=target {button {class:"secondary",onclick:move |_|open(app,target),
                            match target {ProgressTarget::Teacher(_)=>"Return to this question →",ProgressTarget::Passage{..}=>"Practice this passage →"}
                        }}else {small {class:"quiet","Its saved evidence is kept. Sync or resolve the exact source before reopening it."}}
                    }
                }}
            }
        }
        if snapshot.recent_questions+snapshot.recent_scene_starts>0 {
            section {class:"panel progress-chat",h2 {"Conversations count as use, too"}
                p {"{snapshot.recent_questions} messages and {snapshot.recent_scene_starts} scenes started in the last seven days. Ordinary conversation stays separate from completed exercises and memory evidence."}
                button {class:"text-button",onclick:move |_|act(app,|s|{s.navigate(Page::Teacher)?;s.repo.set("teacher_surface",&"dialogues")}),"Open your conversations →"}
            }
        }
        if snapshot.future_count>0 {
            details {class:"panel progress-clock",summary {"{snapshot.future_count} saved practice dates are ahead of this device"}
                p {"These responses remain saved, but are not counted in current activity or recent memory evidence. Check the dates on your devices before interpreting them."}
                ul {for row in &snapshot.future {li {"{row.skill.label()} · {row.title} · {ahead(row.occurred_at_ms,now)}"}}}
            }
        }
    }
}
