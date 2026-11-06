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

const STYLE: &str = ".teacher-return-row {display:flex;align-items:center;justify-content:space-between;gap:16px;padding:16px 0;border-top:1px solid var(--line)}.teacher-return-row strong,.teacher-return-row small {display:block;margin-top:6px}.teacher-return-row small {color:var(--muted)}.teacher-revisit-self-report {margin:12px 0}.teacher-revisit-self-report summary {min-height:44px;padding:10px 0;cursor:pointer}.teacher-return-pending {margin:16px 0;padding-bottom:16px}.teacher-return-screen .teacher-exercise {padding:8px 0}.teacher-return-screen .teacher-exercise > details > summary {min-height:44px;padding:10px 0}.teacher-return-screen .teacher-exercise > .text-button {min-height:44px}@media(max-width:600px){.teacher-return-row {align-items:stretch;flex-direction:column}}";

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
fn due_label(due: i64, now: i64) -> String {
    let remaining = due.saturating_sub(now);
    if remaining <= 0 {
        "Ready to revisit".into()
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
    let queue = state.teacher_revisits(now);
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
            if let Some((answer,_))=pending {div {class:"teacher-return-pending",strong {"{response_title(answer)}"}
                p {class:"quiet","Your unfinished return is kept on this device."}
                button {class:"primary",onclick:move |_|act(app,|s|s.navigate(Page::Practice)),"Continue this return →"}
            }}
            for entry in queue.entries.iter().take(if show_all(){usize::MAX}else{3}) {
                {let key=entry.key;let label=task_title(&entry.title);let due=entry.due;let stamp=due_label(entry.state.due_at_ms,now);let row=format!("return-{}-{}",key.document_id,key.practice_index);
                rsx! {div {key:"{row}",class:"teacher-return-row","data-review-document":key.document_id.to_string(),"data-review-index":key.practice_index.to_string(),"data-review-due":entry.state.due_at_ms.to_string(),
                    div {span {class:"tag","{entry.skill.label()}"}strong {"{label}"}small {"{stamp}"}}
                    button {class:"secondary",onclick:move |_|act(app,|s|{s.start_teacher_revisit(key,now_ms())?;s.navigate(Page::Practice)}),if due {"Try this again →"}else{"Practice early →"}}
                }}}
            }
            if queue.entries.len()>3 && !show_all(){button {class:"text-button",onclick:move |_|show_all.set(true),"Show all planned returns"}}
            details {summary {"How these returns are planned"}
                p {"The first saved response comes back about a day later. Later gaps can grow after a response given without help, based on your own report of recall or your AI teacher's judgment that it was correct. Those are different kinds of evidence; neither is a proficiency score."}
                p {class:"quiet","Help and difficulty bring the task back sooner. Replays and recent restudy can hold the gap. An outcome you leave unreported is never treated as failure. The fixed 1, 3, 7, 14, 30, 60 and 120 day steps are a starting design, not a measured optimum for you."}
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
                if let Some(entry)=schedule {p {class:"quiet",role:"status","{due_label(entry.state.due_at_ms,now_ms())}. You can stop here or have another go now."}}
                div {class:"actions",
                    button {class:"primary",disabled:capture_busy(),onclick:move |_|act(app,|s|{s.clear_active_teacher_revisit()?;s.navigate(Page::Today)}),"Finish & return to Today"}
                    button {class:"secondary",disabled:busy,onclick:move |_|act(app,|s|{s.start_teacher_revisit(key,now_ms())?;Ok(())}),"Try again with a fresh response"}
                }
            } else {button {class:"secondary",disabled:busy,onclick:move |_|act(app,|s|s.navigate(Page::Today)),"Pause & return to Today"}}
        }
    }
}
