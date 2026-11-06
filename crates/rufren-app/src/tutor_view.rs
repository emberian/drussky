use crate::{exposure_view::PresentedRussian, model::AppState, teacher_prose::TeacherProse};
use dioxus::prelude::*;
use rufren_core::PresentationPurpose;
use rufren_tutor::{
    MAX_CONTEXT_BYTES, MAX_TEXT_BYTES, ReviewStatus, TutorExample, TutorRequest, TutorResponse,
    TutorStatus, TutorTask,
};
use serde::de::DeserializeOwned;

const STYLE: &str = include_str!("tutor_view.css");
const RESPONSE_LIMIT: usize = 80 * 1024;

/// A text-only tutor. Mounting never sends text or contacts the relay.
/// The parent may persist responses through `on_response` and restore their JSON later.
#[component]
pub fn TutorView(
    app: Signal<AppState>,
    endpoint: String,
    token: String,
    on_add: Option<EventHandler<TutorExample>>,
    on_response: Option<EventHandler<TutorResponse>>,
    restored_response_json: Option<String>,
) -> Element {
    let mut text = use_signal(String::new);
    let mut context = use_signal(String::new);
    let mut starter_example = use_signal(|| None::<String>);
    let mut task = use_signal(|| TutorTask::Explain);
    let mut status = use_signal(|| None::<TutorStatus>);
    let mut checked_connection = use_signal(|| None::<(String, String)>);
    let mut checking = use_signal(|| false);
    let mut sending = use_signal(|| false);
    let mut error = use_signal(String::new);
    let mut response = use_signal(move || {
        restored_response_json
            .as_deref()
            .filter(|json| json.len() <= RESPONSE_LIMIT)
            .and_then(|json| serde_json::from_str::<TutorResponse>(json).ok())
            .filter(valid_response)
    });
    let connection_matches =
        checked_connection.read().as_ref() == Some(&(endpoint.clone(), token.clone()));
    let current_status = if connection_matches {
        status.read().clone()
    } else {
        None
    };
    let available = current_status
        .as_ref()
        .is_some_and(|s| s.enabled && s.remaining_reservations > 0);
    let busy = checking() || sending();
    let input = TutorRequest {
        task: task(),
        text: text(),
        learner_context: (!context().trim().is_empty()).then(|| context()),
    };
    let valid_input = input.validate().is_ok();
    let text_size = text.read().len();
    let context_size = context.read().len();
    let answer = response.read().clone();
    let check_endpoint = endpoint.clone();
    let check_token = token.clone();
    let send_endpoint = endpoint.clone();
    let send_token = token.clone();
    rsx! {
        style {"{STYLE}"}
        div {class:"page-heading tutor-heading",
            div {div {class:"eyebrow","MAKE SENSE OF THE WORDS"} h1 {"A little help with Russian"}
                p {"Bring a sentence, a question, or something you’re trying to say."}}
            span {class:"tutor-text-badge","Text only"}
        }
        div {class:"tutor-layout",
            section {class:"panel tutor-compose", aria_label:"Ask the text tutor",
                h2 {"What are you working on?"}
                div {class:"tutor-task-picker", role:"group", aria_label:"Choose the kind of help",
                    for (value, label) in [(TutorTask::Explain,"Understand"),(TutorTask::WritingFeedback,"Try my words"),(TutorTask::PrepareContent,"Make practice")] {
                        button {class:if task()==value {"tutor-task selected"}else{"tutor-task"}, disabled:busy,
                            aria_pressed:if task()==value {"true"}else{"false"}, onclick:move |_|task.set(value.clone()), "{label}"}
                    }
                }
                p {class:"tutor-task-help",match task() {
                    TutorTask::Explain=>"Untangle a sentence, a grammar pattern, or a meaning.",
                    TutorTask::WritingFeedback=>"Explore wording, meaning, and register in something you wrote.",
                    TutorTask::PrepareContent=>"Turn a topic or short passage into a small practice draft.",
                }}
                label {r#for:"tutor-text","Your text or question"}
                textarea {id:"tutor-text",class:"tutor-input",value:text(),disabled:busy,
                    oninput:move |e|text.set(e.value()),placeholder:"Ask about a sentence or a pattern you noticed."}
                if let Some(example)=starter_example() {PresentedRussian {app,language:"en",class:"quiet",text:example,purpose:PresentationPurpose::Reference}}
                else {PresentedRussian {app,language:"en",class:"quiet",text:"For example: Why does «Я иду в школу» use школу, but «Я в школе» uses школе?",purpose:PresentationPurpose::Reference}}
                div {class:if text_size>MAX_TEXT_BYTES {"tutor-count over"}else{"tutor-count"},"{text_size} / {MAX_TEXT_BYTES} bytes"}
                details {class:"tutor-context", summary {"Add a little context"}
                    label {r#for:"tutor-context","What would help?"}
                    textarea {id:"tutor-context",value:context(),disabled:busy,oninput:move |e|context.set(e.value()),
                        placeholder:"I’m beginning Russian. Keep the explanation short and show how meaning changes."}
                    div {class:if context_size>MAX_CONTEXT_BYTES {"tutor-count over"}else{"tutor-count"},"{context_size} / {MAX_CONTEXT_BYTES} bytes"}
                }
                div {class:"tutor-disclosure",
                    span {class:"tutor-disclosure-mark",aria_hidden:"true","↗"}
                    p {"Sending shares this text and optional context with your configured AI teacher through your private relay."}
                }
                div {class:"actions tutor-actions",
                    button {class:"primary",disabled:busy || !available || !valid_input,
                        onclick:move |_|{
                            let request = input.clone();
                            let endpoint = send_endpoint.clone();
                            let token = send_token.clone();
                            error.set(String::new());sending.set(true);
                            spawn(async move {
                                let result = relay_json::<TutorResponse>(&endpoint,&token,"/api/tutor/suggest",Some(&request)).await;
                                sending.set(false);
                                match result {
                                    Ok(answer) if valid_response(&answer)=>{
                                        if let Some(callback)=on_response {callback.call(answer.clone());}
                                        response.set(Some(answer));
                                        // Local display follows the service's conservative reservation policy.
                                        if let Some(s)=status.write().as_mut(){
                                            s.remaining_reservations=s.remaining_reservations.saturating_sub(1);
                                            s.remaining_budget_micro_usd=s.remaining_budget_micro_usd.saturating_sub(rufren_tutor::RESERVATION_MICRO_USD);
                                        }
                                    }
                                    Ok(_)=>{checked_connection.set(None);error.set("The relay returned a suggestion with invalid provenance or content. Please check its setup.".into());},
                                    Err(problem)=>{checked_connection.set(None);error.set(problem);},
                                }
                            });
                        },
                        if sending() {span {class:"tutor-spinner",aria_hidden:"true"} "Thinking through your text…"}else{"Send text to Anthropic ↗"}
                    }
                    button {class:"text-button",disabled:busy,
                        onclick:move |_|{
                            let endpoint=check_endpoint.clone();let token=check_token.clone();
                            checking.set(true);error.set(String::new());
                            spawn(async move {
                                let result=relay_json::<TutorStatus>(&endpoint,&token,"/api/tutor/status",None).await;
                                checking.set(false);
                                match result {
                                    Ok(result)=>{checked_connection.set(Some((endpoint,token)));status.set(Some(result));}
                                    Err(problem)=>error.set(problem),
                                }
                            });
                        },if checking(){"Checking…"}else{"Check availability"}
                    }
                }
                div {class:"tutor-availability",role:"status",aria_live:"polite",
                    if let Some(s)=current_status {
                        if s.enabled && s.remaining_reservations>0 {
                            p {class:"tutor-ready",span {class:"status-dot"} "Ready for text suggestions"}
                            p {class:"quiet",{format!("{} requests left in this relay session · ${:.2} estimated budget remaining",s.remaining_reservations,s.remaining_budget_micro_usd as f64/1_000_000.0)}}
                        }else if s.enabled {p {"This relay session’s request budget is used up. Check the relay’s budget settings before sending more."}}
                        else {p {"Text tutoring is switched off on your private relay."}
                            details {summary {"Set up the relay"} p {"Set RUFREN_TUTOR_PROVIDER=anthropic and ANTHROPIC_API_KEY on the relay server, then restart it. Keep provider credentials out of this app."}}}
                    }else {p {"Check availability first. This contacts your private relay without sending the text above."}}
                }
                if !error().is_empty(){p {class:"tutor-error",role:"alert","{error}"}}
            }
            section {class:"tutor-result",aria_label:"Tutor suggestions",aria_live:"polite",
                if let Some(answer)=answer {
                    div {class:"tutor-answer-heading",span {class:"tag tutor-review-tag","UNREVIEWED AI SUGGESTION"}
                        p {"A starting point to explore and check with your coach."}}
                    article {class:"panel tutor-answer",
                        h2 {"{task_title(&answer.task)}"}
                        TeacherProse {app,language:"en",class:"tutor-prose",text:answer.content.explanation.clone(),purpose:PresentationPurpose::Instruction}
                        if !answer.content.examples.is_empty(){
                            div {class:"tutor-example-list",for (index,example) in answer.content.examples.iter().enumerate(){
                                {let draft=example.clone();rsx!{
                                    section {class:"tutor-example",key:"{index}",
                                        PresentedRussian {app,class:"tutor-russian",text:example.russian.clone(),purpose:PresentationPurpose::Instruction}
                                        p {class:"tutor-meaning","{example.english}"}
                                        TeacherProse {app,language:"en",class:"quiet",text:example.note.clone(),purpose:PresentationPurpose::Instruction}
                                        if let Some(callback)=on_add {button {class:"text-button",onclick:move |_|callback.call(draft.clone()),"Open as library draft →"}}
                                    }
                                }}
                            }}
                        }
                        div {class:"tutor-next-step",span {class:"eyebrow","TRY THIS NEXT"} TeacherProse {app,language:"en",text:answer.content.next_step.clone(),purpose:PresentationPurpose::Instruction}}
                        if !answer.content.caveats.is_empty(){details {class:"tutor-caveats",summary {"Things to check"}ul {for caveat in &answer.content.caveats {li {TeacherProse {app,language:"en",text:caveat.clone(),purpose:PresentationPurpose::Instruction}}}}}}
                        footer {class:"tutor-answer-footer",
                            span {"{answer.provenance.provider} · {answer.provenance.model}"}
                            span {{match answer.usage.accounting { rufren_tutor::UsageAccounting::SubscriptionQuota=>format!("{} output tokens · subscription usage",answer.usage.output_tokens), rufren_tutor::UsageAccounting::ApiTokenEstimate=>format!("{} input + {} output tokens · estimated ${:.4}",answer.usage.input_tokens,answer.usage.output_tokens,answer.usage.estimated_micro_usd as f64/1_000_000.0) }}}
                            small {"Text feedback concerns wording and meaning. Speech feedback needs the recording itself."}
                        }
                    }
                }else{
                    div {class:"tutor-welcome",
                        span {class:"tutor-welcome-symbol",aria_hidden:"true","Ж"}
                        h2 {"Follow a small curiosity."}
                        p {"Find the shape of an idea, test a sentence, or make a few examples to return to."}
                        div {class:"tutor-starters",h3 {"A place to begin"}
                            for (starter_task,label,prompt) in [
                                (TutorTask::Explain,"Where, or where to?","Explain the difference between «Я в школе» and «Я иду в школу» with two short examples."),
                                (TutorTask::WritingFeedback,"Find the words","I want to say “I’m learning Russian because I want to read stories.” My draft is «Я учу русский потому что я хочу читать истории». Explain any useful wording or punctuation suggestions."),
                                (TutorTask::PrepareContent,"A little mathematics","Draft three very short beginner-friendly Russian sentences using “for every”, “there exists”, and “if … then”. Explain the meaning and flag terminology that a Russian mathematics reader should verify."),
                            ]{button {class:"tutor-starter",disabled:busy,onclick:move |_|{task.set(starter_task.clone());text.set(prompt.into());starter_example.set(Some(prompt.into()));},span {"{label}"}span {aria_hidden:"true","↗"}}}
                        }
                        p {class:"tutor-welcome-note","Choosing a prompt only fills the text box. You decide when to send it."}
                    }
                }
            }
        }
    }
}

fn task_title(task: &TutorTask) -> &'static str {
    match task {
        TutorTask::Explain => "Finding the meaning",
        TutorTask::WritingFeedback => "Working with your words",
        TutorTask::PrepareContent => "A small practice draft",
    }
}

fn valid_response(response: &TutorResponse) -> bool {
    response.provenance.review_status == ReviewStatus::UnreviewedAiSuggestion
        && !response.provenance.provider.is_empty()
        && !response.provenance.model.is_empty()
        && response.content.validate().is_ok()
        && response.usage.input_tokens <= rufren_tutor::MAX_INPUT_TOKENS
        && response.usage.output_tokens <= rufren_tutor::MAX_OUTPUT_TOKENS
}

async fn relay_json<T: DeserializeOwned>(
    endpoint: &str,
    token: &str,
    path: &str,
    body: Option<&TutorRequest>,
) -> Result<T, String> {
    let body = body
        .map(serde_json::to_value)
        .transpose()
        .map_err(|_| "Could not prepare this text.")?;
    crate::gateway::request(
        endpoint,
        token,
        path,
        body.as_ref(),
        RESPONSE_LIMIT,
        120_000,
    )
    .await
}
