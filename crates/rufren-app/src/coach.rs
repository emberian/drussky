use crate::audio_studio::AudioReview;
use crate::{
    act,
    model::{AppState, Page},
};
use dioxus::prelude::*;
use rufren_core::*;

#[component]
pub(crate) fn ConversationNotes(mut app: Signal<AppState>) -> Element {
    let state = app.read().clone();
    let mut selected = use_signal(|| None::<uuid::Uuid>);
    let mut reference_recording = use_signal(|| None::<String>);
    let mut dimension = use_signal(|| JudgmentDimension::Recall);
    let mut verdict = use_signal(|| Verdict::Uncertain);
    let mut confidence = use_signal(|| 70u8);
    let mut note = use_signal(String::new);
    let mut correction = use_signal(String::new);
    let mut attempts: Vec<_> = state.library.attempts().collect();
    attempts.sort_by_key(|(e, _)| std::cmp::Reverse(e.recorded_at_ms));
    let current = selected().and_then(|id| state.library.attempt(id).map(|a| (id, a)));
    rsx! {
        p {"If a useful phrase or correction comes up in a conversation, you can keep it here. These notes are entirely optional."}
        button {class:"secondary",onclick:move |_|act(app,|s|s.navigate(Page::Studio)),"Open saved recordings"}
        div {class:"library-layout",
            section {class:"panel",h2 {"Recent attempts"}p {class:"quiet","Choose a response if you want to attach a conversation note."}
                if attempts.is_empty() {p {"Complete a practice to bring it here."}}
                for (event,attempt) in attempts.iter().take(50) {{let id=event.id;rsx! {button {class:if selected()==Some(id) {"attempt-choice selected"}else{"attempt-choice"},onclick:move |_|{selected.set(Some(id));reference_recording.set(None);note.set(String::new());correction.set(String::new());verdict.set(Verdict::Uncertain);},
                    span {class:"tag","{attempt.skill.label()}"}strong {{state.library.revision(attempt.revision).map(|r|r.passage.title.as_str()).unwrap_or("Reference not available")}}
                    p {"{attempt.response}"}small {if attempt.cues.is_empty(){"No recorded cue"}else{"Used support"}}
                }}}}
            }
            section {class:"panel",h2 {"Feedback with a source"}
                if let Some((id,attempt))=current {
                    if let Some(revision)=state.library.revision(attempt.revision) {crate::exposure_view::PresentedRussian {app,class:"dialogue-russian",text:revision.passage.russian.clone(),revision:Some(attempt.revision),purpose:PresentationPurpose::Reference}p {class:"quiet","{revision.passage.translation}"}}
                    blockquote {"{attempt.response}"}
                    {let recordings=state.library.recordings(AnnotationTarget::Attempt(id));
                        let references=state.library.recordings(AnnotationTarget::Revision(attempt.revision));
                        rsx! {
                            if recordings.iter().any(|(_,a)|a.role==AudioRole::Attempt) {
                                label {r#for:"coach-reference","Compare with a saved reference (optional)"}
                                select {id:"coach-reference",value:reference_recording().unwrap_or_default(),onchange:move |e|reference_recording.set((!e.value().is_empty()).then(||e.value())),
                                    option {value:"","This attempt only"}
                                    for (index,(_,recording)) in references.iter().filter(|(_,a)|a.role==AudioRole::Reference).enumerate() {
                                        option {value:recording.asset_id.clone(),"Reference {index + 1} · {recording.note}"}
                                    }
                                }
                                for (_,recording) in recordings.into_iter().filter(|(_,a)|a.role==AudioRole::Attempt) {
                                    AudioReview {key:"{recording.asset_id}",asset_id:recording.asset_id.clone(),reference_asset_id:reference_recording()}
                                }
                            }
                        }
                    }
                    label {r#for:"coach-name","Who offered this feedback?"}input {id:"coach-name",value:state.preferences.coach_name,oninput:move |e|act(app,|s|{s.preferences.coach_name=e.value();s.save_preferences()})}
                    label {r#for:"coach-dimension","What are you assessing?"}select {id:"coach-dimension",value:dimension_key(dimension()),onchange:move |e|dimension.set(parse_dimension(&e.value())),
                        for d in [JudgmentDimension::Recall,JudgmentDimension::Comprehension,JudgmentDimension::Grammar,JudgmentDimension::Pronunciation,JudgmentDimension::Fluency,JudgmentDimension::Intelligibility,JudgmentDimension::Spelling] {option {value:dimension_key(d),"{dimension_label(d)}"}}
                    }
                    label {r#for:"coach-verdict","Their impression"}select {id:"coach-verdict",value:verdict_key(verdict()),onchange:move |e|verdict.set(parse_verdict(&e.value())),
                        option {value:"uncertain","Not sure yet"}option {value:"correct","Correct for this task"}option {value:"partly_correct","Partly correct"}option {value:"incorrect","Needs correction"}
                    }
                    label {r#for:"coach-confidence","Confidence in that impression · {confidence()}%"}input {id:"coach-confidence",r#type:"range",min:0,max:100,step:10,value:"{confidence()}",oninput:move |e|{if let Ok(v)=e.value().parse(){confidence.set(v);}}}
                    label {r#for:"coach-note","One useful observation"}textarea {id:"coach-note",value:note(),oninput:move |e|note.set(e.value()),placeholder:"What should we listen for or try next?"}
                    label {r#for:"coach-correction","Suggested wording (optional)"}textarea {id:"coach-correction",lang:"ru",value:correction(),oninput:move |e|correction.set(e.value())}
                    div {class:"actions",button {class:"primary",onclick:move |_|act(app,|s|{s.coach_judgment(id,dimension(),verdict(),note(),confidence(),false)?;save_annotation(s,id,note(),correction())}),"Save new feedback"}}
                    {let heads=state.library.judgments(id,dimension());rsx! {
                        if !heads.is_empty() {div {class:"feedback-history",h3 {"Active judgments"}
                            for (_,j) in &heads {p {strong {"{j.provenance.coach}: "} "{verdict_label(j.verdict)} · {j.confidence_percent}%"}p {class:"quiet","{j.provenance.note}"}}
                            p {class:"quiet","Save new feedback keeps these judgments active. Use the action below only when the current form should replace them; their history is preserved."}
                            button {class:"secondary",onclick:move |_|act(app,|s|s.coach_judgment(id,dimension(),verdict(),note(),confidence(),true)),"Replace these active judgments"}
                        }}
                    }}
                    for (_,annotation) in state.library.annotations(AnnotationTarget::Attempt(id)) {article {class:"coach-note",p {"{annotation.note}"}if let Some(text)=&annotation.suggested_correction {p {lang:"ru","{text}"}}small {"{annotation.provenance.coach}"}}}
                } else {p {class:"quiet","Select an attempt to review. Feedback records what a named person judged; it does not turn a self-rating or AI suggestion into a coach assessment."}}
            }
        }
    }
}
fn save_annotation(
    s: &mut AppState,
    id: uuid::Uuid,
    note: String,
    correction: String,
) -> crate::repository::Result<()> {
    if note.trim().is_empty() && correction.trim().is_empty() {
        return Ok(());
    }
    let event = rufren_core::Event::new(
        s.device,
        crate::repository::now_ms(),
        Fact::CoachAnnotationRecorded(CoachAnnotation {
            target: AnnotationTarget::Attempt(id),
            span: None,
            note: if note.trim().is_empty() {
                "Suggested wording".into()
            } else {
                note
            },
            suggested_correction: (!correction.trim().is_empty()).then_some(correction),
            provenance: CoachProvenance {
                coach: s.preferences.coach_name.trim().into(),
                method: "Coach entered in person".into(),
                note: String::new(),
            },
            supersedes: Default::default(),
        }),
    )
    .map_err(|e| e.to_string())?;
    s.repo.append(&[event])?;
    s.refresh()
}
fn dimension_key(d: JudgmentDimension) -> &'static str {
    match d {
        JudgmentDimension::Recall => "recall",
        JudgmentDimension::Comprehension => "comprehension",
        JudgmentDimension::Grammar => "grammar",
        JudgmentDimension::Pronunciation => "pronunciation",
        JudgmentDimension::Fluency => "fluency",
        JudgmentDimension::Intelligibility => "intelligibility",
        JudgmentDimension::Spelling => "spelling",
    }
}
fn dimension_label(d: JudgmentDimension) -> &'static str {
    match d {
        JudgmentDimension::Recall => "Recall",
        JudgmentDimension::Comprehension => "Meaning",
        JudgmentDimension::Grammar => "Grammar",
        JudgmentDimension::Pronunciation => "Pronunciation",
        JudgmentDimension::Fluency => "Fluency",
        JudgmentDimension::Intelligibility => "Intelligibility",
        JudgmentDimension::Spelling => "Spelling",
    }
}
fn parse_dimension(s: &str) -> JudgmentDimension {
    match s {
        "comprehension" => JudgmentDimension::Comprehension,
        "grammar" => JudgmentDimension::Grammar,
        "pronunciation" => JudgmentDimension::Pronunciation,
        "fluency" => JudgmentDimension::Fluency,
        "intelligibility" => JudgmentDimension::Intelligibility,
        "spelling" => JudgmentDimension::Spelling,
        _ => JudgmentDimension::Recall,
    }
}
fn verdict_key(v: Verdict) -> &'static str {
    match v {
        Verdict::Correct => "correct",
        Verdict::PartlyCorrect => "partly_correct",
        Verdict::Incorrect => "incorrect",
        Verdict::Uncertain => "uncertain",
    }
}
fn verdict_label(v: Verdict) -> &'static str {
    match v {
        Verdict::Correct => "Correct",
        Verdict::PartlyCorrect => "Partly correct",
        Verdict::Incorrect => "Needs correction",
        Verdict::Uncertain => "Uncertain",
    }
}
fn parse_verdict(s: &str) -> Verdict {
    match s {
        "correct" => Verdict::Correct,
        "partly_correct" => Verdict::PartlyCorrect,
        "incorrect" => Verdict::Incorrect,
        _ => Verdict::Uncertain,
    }
}
