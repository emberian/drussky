//! Authored writing-and-speaking activities; source lessons remain immutable.
//! Dictation exposes audio, not spelling. Model hints appear only after saving ink
//! or explicitly requesting help. Completing a route does not grade ink or speech.
use crate::foundations::{FoundationAction, FoundationPack, foundation_step_pack_revision};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const HANDWRITING_TASKS_JSON: &str = include_str!("../content/handwriting-v1.json");
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FoundationHandwritingSequence {
    HearWriteCompareSay,
    SeeCopyHearSay,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FoundationHandwritingInputFocus {
    WholeTarget,
    InitialSegmentOfSyllable,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FoundationHandwritingCase {
    Uppercase,
    Lowercase,
    Sentence,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FoundationHandwritingLayout {
    LetterBox,
    WordLine,
    PhraseLine,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FoundationHandwritingTask {
    pub id: String,
    pub revision: String,
    pub foundation_step_id: String,
    pub foundation_step_revision: String,
    pub source_pack_revision: String,
    pub audio_id: String,
    pub target: String,
    pub meaning: Option<String>,
    pub requires_graphemes: Vec<String>,
    pub requires_meanings: Vec<String>,
    pub requires_samples: Vec<String>,
    pub requires_conventions: Vec<String>,
    pub prerequisite_step_ids: Vec<String>,
    pub required_models: Vec<String>,
    pub sequence: FoundationHandwritingSequence,
    pub sequences: Vec<FoundationHandwritingSequence>,
    pub input_focus: FoundationHandwritingInputFocus,
    pub case: FoundationHandwritingCase,
    pub layout: FoundationHandwritingLayout,
    pub formation_hint: String,
    pub script_note: String,
    pub model_help: String,
    pub say_instruction: String,
    pub grammar_note: Option<String>,
}

pub fn handwriting_tasks() -> Vec<FoundationHandwritingTask> {
    serde_json::from_str(HANDWRITING_TASKS_JSON).expect("bundled handwriting activities must parse")
}
pub fn handwriting_task_for_step(step_id: &str) -> Option<FoundationHandwritingTask> {
    handwriting_tasks()
        .into_iter()
        .find(|t| t.foundation_step_id == step_id)
}

/// Structural authoring check only. Runtime still requires actual source encounters
/// and the host's authored glyph models; file order is not learner knowledge.
pub fn validate_handwriting_tasks(
    pack: &FoundationPack,
    tasks: &[FoundationHandwritingTask],
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    let mut ids = BTreeSet::new();
    for task in tasks {
        if !ids.insert(&task.id) || task.revision.trim().is_empty() {
            errors.push(format!("Duplicate or unversioned writing task {}", task.id));
        }
        let mut graphemes = BTreeSet::new();
        let mut meanings = BTreeSet::new();
        let mut samples = BTreeSet::new();
        let mut conventions = BTreeSet::new();
        let mut preceding = BTreeSet::new();
        let mut source = None;
        for step in pack.episodes.iter().flat_map(|e| &e.steps) {
            graphemes.extend(step.introduces_graphemes.iter().cloned());
            meanings.extend(step.introduces_meanings.iter().cloned());
            samples.extend(step.introduces_samples.iter().cloned());
            conventions.extend(step.introduces_conventions.iter().cloned());
            preceding.insert(step.id.as_str());
            if step.id == task.foundation_step_id {
                source = Some(step);
                break;
            }
        }
        let Some(source) = source else {
            errors.push(format!("{} has unknown source", task.id));
            continue;
        };
        if !matches!(source.action, FoundationAction::Present { .. })
            || source.revision != task.foundation_step_revision
            || foundation_step_pack_revision(&source.id) != task.source_pack_revision
        {
            errors.push(format!(
                "{} has unsuitable or changed teaching source",
                task.id
            ));
        }
        if !task
            .prerequisite_step_ids
            .contains(&task.foundation_step_id)
            || task
                .prerequisite_step_ids
                .iter()
                .any(|id| !preceding.contains(id.as_str()))
        {
            errors.push(format!(
                "{} has missing or future step prerequisites",
                task.id
            ));
        }
        for (kind, needed, known) in [
            ("grapheme", &task.requires_graphemes, &graphemes),
            ("meaning", &task.requires_meanings, &meanings),
            ("sample", &task.requires_samples, &samples),
            ("convention", &task.requires_conventions, &conventions),
        ] {
            for value in needed {
                if !known.contains(value) {
                    errors.push(format!("{} requires untaught {kind} {value}", task.id));
                }
            }
        }
        let shapes: BTreeSet<_> = task
            .target
            .chars()
            .filter(|c| ('А'..='я').contains(c) || *c == 'Ё' || *c == 'ё')
            .map(|c| c.to_uppercase().to_string())
            .collect();
        if shapes.iter().any(|g| {
            !graphemes.contains(g)
                || !task.requires_graphemes.contains(g)
                || !task.required_models.contains(g)
        }) {
            errors.push(format!(
                "{} has untaught or unmodeled target script",
                task.id
            ));
        }
        let visible = format!(
            "{} {} {} {} {} {} {}",
            task.target,
            task.formation_hint,
            task.script_note,
            task.model_help,
            task.grammar_note.as_deref().unwrap_or(""),
            task.say_instruction,
            task.meaning.as_deref().unwrap_or("")
        );
        for c in visible
            .chars()
            .filter(|c| ('А'..='я').contains(c) || *c == 'Ё' || *c == 'ё')
        {
            if !graphemes.contains(&c.to_uppercase().to_string()) {
                errors.push(format!("{} has untaught script in model help", task.id));
            }
        }
        if task.target.contains('\u{301}') || task.target.trim().is_empty() {
            errors.push(format!("{} target must use ordinary spelling", task.id));
        }
        let audio = pack.assets.iter().find(|a| a.id == task.audio_id);
        if !source.audio_ids.contains(&task.audio_id) {
            errors.push(format!(
                "{} audio does not belong to teaching source",
                task.id
            ));
        }
        match audio {
            None => errors.push(format!("{} has unknown audio", task.id)),
            Some(audio) => {
                let spoken = audio.spoken.to_lowercase();
                let target = task.target.to_lowercase();
                let matches = match task.input_focus {
                    FoundationHandwritingInputFocus::WholeTarget => spoken == target,
                    FoundationHandwritingInputFocus::InitialSegmentOfSyllable => {
                        target.chars().count() == 1
                            && spoken.starts_with(&target)
                            && spoken.chars().count() > 1
                    }
                };
                if !matches {
                    errors.push(format!("{} audio/target focus mismatch", task.id));
                }
            }
        }
        if !task.sequences.contains(&task.sequence)
            || task.formation_hint.trim().is_empty()
            || task.say_instruction.trim().is_empty()
            || task.model_help.trim().is_empty()
        {
            errors.push(format!("{} lacks a complete teaching route", task.id));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_seventeen_activities_resolve() {
        let tasks = handwriting_tasks();
        assert_eq!(tasks.len(), 17);
        assert_eq!(
            validate_handwriting_tasks(&crate::foundations::foundations(), &tasks),
            Ok(())
        );
        let phrase = tasks.iter().find(|t| t.target == "Там кот.").unwrap();
        assert_eq!(phrase.layout, FoundationHandwritingLayout::PhraseLine);
        assert_eq!(phrase.audio_id, "foundation-tam-kot");
    }
    #[test]
    fn every_activity_has_a_bundled_whole_wav() {
        let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../rufren-app/assets/foundations");
        for task in handwriting_tasks() {
            let bytes = std::fs::read(directory.join(format!("{}.wav", task.audio_id)))
                .expect("writing reference must be bundled");
            assert!(bytes.len() > 44, "{}", task.id);
            assert_eq!(&bytes[..4], b"RIFF", "{}", task.id);
            assert_eq!(&bytes[8..12], b"WAVE", "{}", task.id);
        }
    }
    #[test]
    fn rejects_untaught_script_and_mismatched_whole_audio() {
        let mut tasks = handwriting_tasks();
        tasks[0].target = "Я".into();
        let errors =
            validate_handwriting_tasks(&crate::foundations::foundations(), &tasks).unwrap_err();
        assert!(errors.iter().any(|e| e.contains("unmodeled target script")));
        assert!(
            errors
                .iter()
                .any(|e| e.contains("audio/target focus mismatch"))
        );
    }
    #[test]
    fn rejects_unsubmitted_recall_as_entry_and_future_prerequisite() {
        let mut tasks = handwriting_tasks();
        let task = tasks.iter_mut().find(|t| t.target == "кот").unwrap();
        task.foundation_step_id = "f2-read-cat".into();
        task.prerequisite_step_ids.push("c6-ni-context".into());
        let errors =
            validate_handwriting_tasks(&crate::foundations::foundations(), &tasks).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.contains("unsuitable or changed teaching source"))
        );
        assert!(
            errors
                .iter()
                .any(|e| e.contains("future step prerequisites"))
        );
    }
}
