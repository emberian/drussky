//! Returning to an exact saved task, without reopening its answer before the learner tries.
use crate::{
    act,
    model::{AppState, Page},
    repository::now_ms,
    teacher_feedback_view::TeacherFeedback,
    teacher_practice::TeacherPracticeDraft,
    teacher_practice_view::TeacherPractice,
    teacher_view::send_teacher,
};
use dioxus::prelude::*;
use rufren_core::*;
use rufren_tutor::{TeacherRequest, TeacherResponse};
use uuid::Uuid;

const STYLE: &str = ".teacher-return-row {display:flex;align-items:center;justify-content:space-between;gap:16px;padding:16px 0;border-top:1px solid var(--line)}.teacher-return-description {min-width:0;flex:1}.teacher-return-why {max-width:58ch;margin-top:8px}.teacher-return-why summary {cursor:pointer;min-height:44px;display:flex;align-items:center}.teacher-return-why p {margin:8px 0}.teacher-return-row strong,.teacher-return-row small {display:block;margin-top:6px}.teacher-return-row small {color:var(--muted)}.teacher-revisit-self-report {margin:12px 0}.teacher-revisit-self-report summary {min-height:44px;padding:10px 0;cursor:pointer}.teacher-return-pending {margin:16px 0;padding-bottom:16px}.teacher-return-screen .teacher-exercise {padding:8px 0}.teacher-return-screen .teacher-exercise > details > summary {min-height:44px;padding:10px 0}.teacher-return-screen .teacher-exercise > .text-button {min-height:44px}@media(max-width:600px){.teacher-return-row {align-items:stretch;flex-direction:column}}";

fn task_title(title: &str) -> String {
    // A queue label must not supply Russian which might itself be the answer.
    if title
        .chars()
        .any(|c| ('\u{0400}'..='\u{052f}').contains(&c))
    {
        "A saved Teacher exercise".into()
    } else {
        title.to_owned()
    }
}
fn response_title(answer: &TeacherResponse) -> String {
    match &answer.request {
        TeacherRequest::TeachLesson(lesson) => task_title(&lesson.title),
        TeacherRequest::AnswerFeedback(_) => "A follow-up with your teacher".into(),
    }
}
// Use authored topic labels rather than provider-written questions or model text:
// even an English prompt/title can give away a listening or retrieval answer.
fn canonical_lesson_title(state: &AppState, answer: Option<&TeacherResponse>) -> String {
    answer
        .and_then(|answer| match &answer.request {
            TeacherRequest::TeachLesson(request) => state.content.lesson(&request.lesson_id),
            TeacherRequest::AnswerFeedback(_) => None,
        })
        .map(|lesson| task_title(&lesson.title))
        .unwrap_or_else(|| "Saved Teacher lesson".into())
}

fn return_reason(evidence: TeacherRevisitEvidence) -> &'static str {
    match evidence {
        TeacherRevisitEvidence::GuidedSeed => {
            "You saved your first response to this exercise. This is its first planned return."
        }
        TeacherRevisitEvidence::UnassistedSelfReport => {
            "You reported remembering the last response, with no help recorded. Your own impression helped set this gap."
        }
        TeacherRevisitEvidence::AiCorrect => {
            "Your AI teacher judged the last response correct, with no help recorded. This is a language judgment, separate from your own impression."
        }
        TeacherRevisitEvidence::ClearError => {
            "You asked for another try, or your AI teacher flagged something to work on. A shorter gap gives you another opportunity."
        }
        TeacherRevisitEvidence::AnswerHelp => {
            "Help was recorded for the last return, or you said you needed help. The next gap stays short."
        }
        TeacherRevisitEvidence::ReplayOnly => {
            "The last return included a replay. That response is kept, and the gap has not grown from it."
        }
        TeacherRevisitEvidence::Unrated => {
            "The last response has no reported recall or clear positive teacher judgment. It is kept without treating an unreported outcome as failure."
        }
        TeacherRevisitEvidence::ContestedAi => {
            "Your AI teacher's saved judgments disagree. The plan stays cautious rather than choosing a favorable judgment."
        }
        TeacherRevisitEvidence::RecentRestudy => {
            "You had recently practiced or seen this material before the last return. That response is kept without treating it as recall across the full gap."
        }
    }
}

fn planned_gap(library: &Library, state: &TeacherRevisitState) -> String {
    let anchor = state
        .last_attempt
        .and_then(|id| library.events.get(&id))
        .map_or(state.seed.occurred_at_ms, |event| event.recorded_at_ms);
    let gap = state.due_at_ms.saturating_sub(anchor);
    let duration = if gap < 60 * MINUTE_MS {
        "less than an hour".into()
    } else if gap < DAY_MS {
        let hours = (gap + 30 * MINUTE_MS) / (60 * MINUTE_MS);
        format!(
            "about {hours} {}",
            if hours == 1 { "hour" } else { "hours" }
        )
    } else {
        let days = gap.saturating_add(DAY_MS / 2) / DAY_MS;
        format!("about {days} {}", if days == 1 { "day" } else { "days" })
    };
    format!("Planned gap: {duration} after that response. You can return when it suits you.")
}

fn latest_recorded_support(
    library: &Library,
    entry: &crate::teacher_revisit::TeacherRevisitEntry,
) -> Option<&'static str> {
    let last_time = library
        .events
        .get(&entry.state.last_attempt?)?
        .recorded_at_ms;
    // last_attempt represents a co-timed group, not an evidence winner. Check
    // all members of that latest group, while leaving older help in its history.
    let current: Vec<_> = entry
        .derived_support
        .iter()
        .filter(|support| {
            library
                .events
                .get(&support.attempt_id)
                .is_some_and(|event| event.recorded_at_ms == last_time)
        })
        .collect();
    if current
        .iter()
        .any(|support| support.cue == Cue::ReferenceMaterial)
    {
        Some(
            "Saved exposure history also shows reference material during that return. Your original response and own impression are kept.",
        )
    } else if current.iter().any(|support| support.cue == Cue::Replay) {
        Some(
            "Saved playback history also shows a replay during that return. Your original response and own impression are kept.",
        )
    } else {
        None
    }
}

#[component]
pub(crate) fn ReviewSelfReport(
    value: SelfRating,
    disabled: bool,
    on_change: EventHandler<SelfRating>,
) -> Element {
    let id = use_hook(|| format!("review-self-report-{}", Uuid::new_v4()));
    let selected = match value {
        SelfRating::NotReported => "none",
        SelfRating::Recalled => "recalled",
        SelfRating::WithHelp => "help",
        SelfRating::Again => "again",
    };
    rsx! {
        details {class:"teacher-revisit-self-report",
            summary {"Your own impression (optional)"}
            label {r#for:id.clone(),"How did that response feel?"}
            select {id:id.clone(),value:selected,disabled,onchange:move |event|on_change.call(match event.value().as_str(){"recalled"=>SelfRating::Recalled,"help"=>SelfRating::WithHelp,"again"=>SelfRating::Again,_=>SelfRating::NotReported}),
                option {value:"none","No self-report"}
                option {value:"recalled","I remembered it"}
                option {value:"help","I needed some help"}
                option {value:"again","I'd like another try"}
            }
        }
    }
}

#[component]
pub(crate) fn TeacherReturns(app: Signal<AppState>, #[props(default)] expanded: bool) -> Element {
    let state = app.read().clone();
    let mut clock = use_signal(now_ms);
    use_future(move || async move {
        loop {
            // The displayed queue refreshes after idle time; no background network work.
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
    let mut queue = state.teacher_revisits(now);
    // Browsing is a stable place to make choices. Availability still orders the
    // scheduling queue, but a timing edit must not move the row being edited.
    queue.entries.sort_by_cached_key(|entry| {
        (
            canonical_lesson_title(
                &state,
                state
                    .library
                    .teacher_document(entry.key.document_id)
                    .map(|d| &d.response),
            ),
            entry.key.document_id,
            entry.key.practice_index,
        )
    });
    let active = state.active_teacher_revisit();
    let pending = active
        .as_ref()
        .ok()
        .and_then(|a| a.as_ref())
        .filter(|(_, draft)| draft.saved_attempt.is_none());
    let mut show_all = use_signal(move || expanded);
    if queue.entries.is_empty()
        && queue.diagnostics.is_empty()
        && pending.is_none()
        && active.is_ok()
    {
        return rsx! {};
    }
    let count = queue.entries.iter().filter(|entry| entry.due).count();
    rsx! {
        section {class:"panel teacher-returns",aria_label:"Return to your Teacher exercises",
            style {"{STYLE}"}
            h2 {"Bring it back"}
            p {class:"quiet",if count==0 {"Your saved exercises will come back after a little time. You can also choose an early return."}else{{format!("{count} {} ready for another try.",if count==1 {"exercise is"}else{"exercises are"})}}}
            if let Err(problem)=&active {p {role:"alert","{problem}"}}
            if let Some((answer,draft))=pending {div {class:"teacher-return-pending",strong {{format!("{} · Exercise {}",canonical_lesson_title(&state,Some(answer)),draft.index+1)}}
                p {class:"quiet","Your unfinished return is kept on this device."}
                button {class:"primary",onclick:move |_|act(app,|s|s.navigate(Page::Practice)),"Continue this return →"}
            }}
            for entry in queue.entries.iter().take(if show_all(){usize::MAX}else{3}) {
                {let key=entry.key;let label=canonical_lesson_title(&state,state.library.teacher_document(key.document_id).map(|document|&document.response));let ordinal=key.practice_index+1;let reason=return_reason(entry.state.last_evidence);let gap=planned_gap(&state.library,&entry.state);let support=latest_recorded_support(&state.library,entry);let due=entry.due;let stamp=crate::review_timing_view::timing_label(entry.timing.effective_due_at_ms,entry.timing.paused,now);let row=format!("return-{}-{}",key.document_id,key.practice_index);
                rsx! {div {key:"{row}",class:"teacher-return-row","data-review-document":key.document_id.to_string(),"data-review-index":key.practice_index.to_string(),"data-review-due":entry.timing.effective_due_at_ms.map(|t|t.to_string()).unwrap_or_default(),"data-review-baseline-due":entry.state.due_at_ms.to_string(),
                    div {class:"teacher-return-description",span {class:"tag","{entry.skill.label()}"}strong {"{label} · Exercise {ordinal}"}small {"{stamp}"}
                        details {class:"teacher-return-why",summary {"Why now?"}p {"{reason}"}p {class:"quiet","{gap}"}if let Some(support)=support {p {class:"quiet","{support}"}}}
                        crate::review_timing_view::TaskTimingControls {app,target:PracticeBatchKey::Teacher {document_id:key.document_id,practice_index:key.practice_index},baseline_due_at_ms:entry.state.due_at_ms,now}
                    }
                    button {class:"secondary",onclick:move |_|act(app,|s|{s.start_teacher_revisit(key,now_ms())?;s.navigate(Page::Practice)}),if due {"Try this again →"}else{"Practice early →"}}
                }}}
            }
            if queue.entries.len()>3 && !show_all(){button {class:"text-button",onclick:move |_|show_all.set(true),"Show all planned returns"}}
            details {summary {"How these returns are planned"}
                p {"Return gaps use the timing preference chosen when you start a response. Later gaps can grow after a response given without help, based on your own report of recall or your AI teacher's judgment that it was correct. Those are different kinds of evidence; neither is a proficiency score."}
                p {class:"quiet","Help and difficulty bring the task back sooner. Replays and recent restudy can hold the gap. An outcome you leave unreported is never treated as failure. The standard 1, 3, 7, 14, 30, 60 and 120 day steps are adjustable starting choices, not a measured optimum for you."}
            }
            if !queue.diagnostics.is_empty(){details {summary {"Some saved tasks need attention"}ul {for problem in &queue.diagnostics {li {"{problem}"}}}}}
        }
    }
}

#[component]
pub(crate) fn TeacherReturn(
    mut app: Signal<AppState>,
    answer: TeacherResponse,
    snapshot: TeacherPracticeDraft,
) -> Element {
    let active_speaking = use_signal(|| None::<String>);
    let capture_busy = use_signal(|| false);
    let heading = format!("teacher-return-heading-{}", snapshot.id);
    let focus_heading = heading.clone();
    use_effect(move || {
        let id =
            serde_json::to_string(&focus_heading).expect("a generated heading ID is serializable");
        spawn(async move {
            let _=document::eval(&format!("await new Promise(requestAnimationFrame);const heading=document.getElementById({id});if(heading){{heading.focus({{preventScroll:true}});heading.scrollIntoView({{block:'start'}});}}" )).await;
        });
    });
    let state = app.read().clone();
    let Some(review) = snapshot.review.as_ref() else {
        return rsx! {p {role:"alert","This saved practice is missing its return information."}};
    };
    let key = TeacherRevisitKey {
        document_id: review.document_id,
        practice_index: review.practice_index,
    };
    let title = response_title(&answer);
    let feedback = snapshot
        .saved_attempt
        .and_then(|attempt| state.teacher_revisit_feedback(attempt));
    let schedule = state
        .teacher_revisits(now_ms())
        .entries
        .into_iter()
        .find(|entry| entry.key == key);
    let busy = state.teacher_busy || state.speech_busy || capture_busy();
    let saved = snapshot.saved_attempt;
    rsx! {
        style {"{STYLE}"}
        div {class:"page-heading",div {div {class:"eyebrow","RETURN · TRY · UNDERSTAND"}h1 {id:heading.clone(),tabindex:"-1","A fresh try"}p {"{title}"}}}
        section {class:"panel teacher-return-screen","data-active-review-document":key.document_id.to_string(),"data-active-review-index":key.practice_index.to_string(),"data-active-review-session":snapshot.id.to_string(),
            if saved.is_none(){p {class:"quiet","Try the original question before reopening its answer. Help is available whenever you want it."}}
            for session in [snapshot.id] {TeacherPractice {key:"return-session-{session}",app,answer:answer.clone(),index:key.practice_index,active_speaking,capture_busy,review:true}}
            if let Some(attempt)=saved {
                if let Some(feedback)=feedback {
                    section {class:"teacher-return-feedback",aria_label:"Feedback on this return",
                        h2 {"Work with this response"}
                        TeacherFeedback {app,answer:feedback.clone(),active_speaking,capture_busy,show_practice:false}
                        p {class:"quiet","{feedback.provenance.model} · AI-generated feedback"}
                    }
                } else if state.teacher_busy {p {role:"status","Your teacher is thinking. Your response is already saved."}}
                else if answer.content.practice[key.practice_index].skill!=Some(rufren_tutor::TeacherSkill::Speaking) {
                    button {class:"secondary",disabled:busy,onclick:move |_|{let request=app.read().answer_feedback_request(attempt);send_teacher(app,request);},"Ask the teacher about this saved response →"}
                }
                if let Some(entry)=schedule {p {class:"quiet",role:"status","{crate::review_timing_view::timing_label(entry.timing.effective_due_at_ms,entry.timing.paused,now_ms())}. You can stop here or have another go now."}}
                crate::review_timing_view::ReviewStartDetails {app,session:snapshot.id,target:PracticeBatchKey::Teacher {document_id:key.document_id,practice_index:key.practice_index}}
                crate::daily_plan_view::SessionNext {app,disabled:busy}
                div {class:"actions",
                    button {class:"primary",disabled:capture_busy(),onclick:move |_|act(app,|s|{s.clear_active_teacher_revisit()?;s.navigate(Page::Today)}),"Finish & return to Today"}
                    button {class:"secondary",disabled:busy,onclick:move |_|act(app,|s|{s.start_teacher_revisit(key,now_ms())?;Ok(())}),"Try again with a fresh response"}
                }
            } else {button {class:"secondary",disabled:busy,onclick:move |_|act(app,|s|s.navigate(Page::Today)),"Pause & return to Today"}}
        }
    }
}
