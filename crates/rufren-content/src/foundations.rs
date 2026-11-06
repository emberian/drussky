//! AI-authored prepared first encounters. Introductions are exposure, not mastery.
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const FOUNDATION_ORIGINAL_JSON: &str = include_str!("../content/foundations-original-v1.json");
pub const FOUNDATION_ORIGINAL_REVISION: &str = "foundation-2026-11-06.1";
pub const FOUNDATION_CURRENT_REVISION: &str = "foundation-2026-11-06.2";

pub const FOUNDATION_JSON: &str = include_str!("../content/foundations-v1.json");
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FoundationPack {
    pub schema_version: u32,
    pub revision: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub previous_revisions: Vec<String>,
    pub editorial_notice: String,
    pub assets: Vec<FoundationAsset>,
    pub episodes: Vec<FoundationEpisode>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FoundationAsset {
    pub id: String,
    pub spoken: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FoundationEpisode {
    pub id: String,
    pub title: String,
    pub goal: String,
    pub steps: Vec<FoundationStep>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FoundationStep {
    pub id: String,
    pub revision: String,
    pub instruction: String,
    pub display: Option<String>,
    pub audio_ids: Vec<String>,
    pub requires_graphemes: Vec<String>,
    pub introduces_graphemes: Vec<String>,
    pub requires_meanings: Vec<String>,
    pub introduces_meanings: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires_samples: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub introduces_samples: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires_conventions: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub introduces_conventions: Vec<String>,
    pub audio_first_chunk: bool,
    pub action: FoundationAction,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FoundationAction {
    Present {
        note: String,
    },
    Choose {
        options: Vec<FoundationChoice>,
        correct_id: String,
        success: String,
    },
    Blend {
        tiles: Vec<String>,
        answer: Vec<String>,
        feedback: Vec<String>,
        success: String,
    },
    Read {
        reveal_text: String,
        note: String,
    },
    Speak {
        note: String,
    },
    Complete {
        summary: String,
    },
    Copy {
        answer: String,
        keyboard: Vec<String>,
        note: String,
    },
    Recall {
        mode: FoundationRecallMode,
        answer: String,
        accepted: Vec<String>,
        feedback: Vec<FoundationRecallFeedback>,
        reveal: String,
        keyboard: Vec<String>,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FoundationChoice {
    pub id: String,
    pub text: String,
    pub feedback: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FoundationRecallMode {
    ListeningMeaning,
    ReadingMeaning,
    Written,
    Spoken,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FoundationRecallFeedback {
    pub response: String,
    pub feedback: String,
}

/// Resolve immutable source identity, independent of the append-only catalog revision.
pub fn foundations_legacy() -> FoundationPack {
    serde_json::from_str(FOUNDATION_ORIGINAL_JSON).expect("legacy foundation must parse")
}

pub fn foundation_step_pack_revision(step_id: &str) -> String {
    if foundations_legacy()
        .episodes
        .iter()
        .flat_map(|e| &e.steps)
        .any(|s| s.id == step_id)
    {
        FOUNDATION_ORIGINAL_REVISION.to_owned()
    } else {
        FOUNDATION_CURRENT_REVISION.to_owned()
    }
}

pub fn foundations() -> FoundationPack {
    serde_json::from_str(FOUNDATION_JSON).expect("bundled foundation JSON must parse")
}
impl FoundationPack {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.schema_version != 1 {
            errors.push("Unsupported foundation schema".into());
        }
        let mut ids = BTreeSet::new();
        let assets: BTreeSet<_> = self.assets.iter().map(|a| a.id.as_str()).collect();
        if assets.len() != self.assets.len()
            || self.assets.iter().any(|a| a.spoken.trim().is_empty())
        {
            errors.push("Duplicate or empty foundation audio asset".into());
        }
        let mut graphemes = BTreeSet::<String>::new();
        let mut meanings = BTreeSet::<String>::new();
        let mut samples = BTreeSet::<String>::new();
        let mut conventions = BTreeSet::<String>::new();
        for episode in &self.episodes {
            if !ids.insert(episode.id.clone()) {
                errors.push(format!("Duplicate ID {}", episode.id));
            }
            for step in &episode.steps {
                if step.revision.trim().is_empty() {
                    errors.push(format!("{} has no revision", step.id));
                }
                if !ids.insert(step.id.clone()) {
                    errors.push(format!("Duplicate ID {}", step.id));
                }
                for g in &step.requires_graphemes {
                    if !graphemes.contains(g) {
                        errors.push(format!("{} requires untaught grapheme {g}", step.id));
                    }
                }
                for m in &step.requires_meanings {
                    if !meanings.contains(m) {
                        errors.push(format!("{} tests unpresented meaning {m}", step.id));
                    }
                }
                for (kind, required, known) in [
                    ("sample", &step.requires_samples, &samples),
                    ("convention", &step.requires_conventions, &conventions),
                ] {
                    for item in required {
                        if !known.contains(item) {
                            errors.push(format!("{} requires untaught {kind} {item}", step.id));
                        }
                    }
                }
                for a in &step.audio_ids {
                    if !assets.contains(a.as_str()) {
                        errors.push(format!("{} has unknown audio {a}", step.id));
                    }
                }
                graphemes.extend(step.introduces_graphemes.iter().cloned());
                meanings.extend(step.introduces_meanings.iter().cloned());
                samples.extend(step.introduces_samples.iter().cloned());
                conventions.extend(step.introduces_conventions.iter().cloned());
                let mut written = step.display.clone().unwrap_or_default();
                written.push_str(&step.instruction);
                match &step.action {
                    FoundationAction::Present { note }
                    | FoundationAction::Read { note, .. }
                    | FoundationAction::Speak { note }
                    | FoundationAction::Copy { note, .. } => written.push_str(note),
                    FoundationAction::Complete { summary } => written.push_str(summary),
                    FoundationAction::Choose {
                        options, success, ..
                    } => {
                        written.push_str(success);
                        for option in options {
                            written.push_str(&option.feedback);
                        }
                    }
                    FoundationAction::Blend {
                        feedback, success, ..
                    } => {
                        written.push_str(success);
                        written.push_str(&feedback.join(" "));
                    }
                    FoundationAction::Recall {
                        feedback, reveal, ..
                    } => {
                        written.push_str(reveal);
                        for item in feedback {
                            written.push_str(&item.feedback);
                        }
                    }
                }
                match &step.action {
                    FoundationAction::Choose {
                        options,
                        correct_id,
                        ..
                    } => {
                        if options.len() < 2
                            || !options.iter().any(|o| &o.id == correct_id)
                            || options.iter().any(|o| o.feedback.trim().is_empty())
                        {
                            errors.push(format!("{} has incomplete choices/feedback", step.id));
                        }
                        for o in options {
                            written.push_str(&o.text);
                        }
                    }
                    FoundationAction::Blend {
                        tiles,
                        answer,
                        feedback,
                        ..
                    } => {
                        if answer.is_empty()
                            || answer.len() != feedback.len()
                            || answer.iter().any(|a| !tiles.contains(a))
                        {
                            errors.push(format!("{} has invalid construction", step.id));
                        }
                        written.push_str(&tiles.join(""));
                    }
                    FoundationAction::Read { reveal_text, .. } => written.push_str(reveal_text),
                    FoundationAction::Copy {
                        answer,
                        keyboard,
                        note,
                    } => {
                        if answer.trim().is_empty()
                            || keyboard.is_empty()
                            || note.trim().is_empty()
                            || step.display.as_ref() != Some(answer)
                        {
                            errors.push(format!("{} lacks visible copy model/keyboard", step.id));
                        }
                        written.push_str(answer);
                        written.push_str(&keyboard.join(""));
                    }
                    FoundationAction::Recall {
                        mode,
                        answer,
                        accepted,
                        feedback,
                        reveal,
                        keyboard,
                    } => {
                        if answer.trim().is_empty()
                            || reveal.trim().is_empty()
                            || feedback.iter().any(|f| {
                                f.response.trim().is_empty() || f.feedback.trim().is_empty()
                            })
                        {
                            errors.push(format!(
                                "{} has incomplete recall reference/feedback",
                                step.id
                            ));
                        }
                        if matches!(
                            mode,
                            FoundationRecallMode::ListeningMeaning
                                | FoundationRecallMode::ReadingMeaning
                                | FoundationRecallMode::Written
                        ) && accepted.is_empty()
                        {
                            errors.push(format!("{} lacks authored recall answers", step.id));
                        }
                        if !matches!(mode, FoundationRecallMode::Written) && !keyboard.is_empty() {
                            errors.push(format!(
                                "{} exposes keyboard outside written recall",
                                step.id
                            ));
                        }
                        if matches!(mode, FoundationRecallMode::Written)
                            && (keyboard.is_empty() || step.display.is_some())
                        {
                            errors.push(format!("{} lacks independent written response", step.id));
                        }
                        if matches!(mode, FoundationRecallMode::Spoken)
                            && (!accepted.is_empty() || step.display.is_some())
                        {
                            errors
                                .push(format!("{} gives spoken score or visible answer", step.id));
                        }
                        if matches!(mode, FoundationRecallMode::ListeningMeaning)
                            && step.display.is_some()
                        {
                            errors.push(format!("{} exposes listening answer script", step.id));
                        }
                        if matches!(mode, FoundationRecallMode::ReadingMeaning)
                            && step.display.is_none()
                        {
                            errors.push(format!("{} lacks reading stimulus", step.id));
                        }
                        if step.audio_ids.is_empty() {
                            errors.push(format!("{} lacks recall audio", step.id));
                        }
                        written.push_str(&keyboard.join(""));
                        // Audio-first references remain internal; their untaught script must never render.
                        if !step.audio_first_chunk {
                            written.push_str(answer);
                        }
                        if matches!(mode, FoundationRecallMode::Written) {
                            written.push_str(&accepted.join(" "));
                            for c in answer.chars().filter(|c| c.is_alphabetic()) {
                                if !keyboard
                                    .iter()
                                    .any(|k| k.to_uppercase() == c.to_uppercase().to_string())
                                {
                                    errors.push(format!(
                                        "{} answer cannot use its keyboard",
                                        step.id
                                    ));
                                }
                            }
                        }
                    }
                    _ => (),
                }
                if written.contains('\u{301}') && !conventions.contains("stress-acute-annotation") {
                    errors.push(format!("{} uses an untaught stress mark", step.id));
                }
                if !step.introduces_meanings.is_empty()
                    && !matches!(step.action, FoundationAction::Present { .. })
                {
                    errors.push(format!(
                        "{} tests a meaning on its introduction card",
                        step.id
                    ));
                }
                // Every visible or response-bearing Cyrillic shape must have been introduced.
                for c in written
                    .chars()
                    .filter(|c| ('А'..='я').contains(c) || *c == 'ё' || *c == 'Ё')
                {
                    if !graphemes.contains(&c.to_uppercase().to_string()) {
                        errors.push(format!("{} displays untaught shape {c}", step.id));
                    }
                }
                if step.audio_first_chunk && step.display.is_some() {
                    errors.push(format!("{} reveals audio-first script", step.id));
                }
            }
        }
        if self.revision == FOUNDATION_CURRENT_REVISION {
            let original: Self =
                serde_json::from_str(FOUNDATION_ORIGINAL_JSON).expect("legacy foundation JSON");
            if !self.previous_revisions.contains(&original.revision)
                || self.episodes.get(..original.episodes.len())
                    != Some(original.episodes.as_slice())
                || self.assets.get(..original.assets.len()) != Some(original.assets.as_slice())
            {
                errors.push("Append-only foundation prefix changed".into());
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bundled_sequence_has_valid_prerequisites_and_assets() {
        let pack = foundations();
        assert_eq!(pack.episodes.len(), 7);
        assert_eq!(pack.validate(), Ok(()));
    }
    #[test]
    fn rejects_untaught_response_shape_and_unpresented_meaning() {
        let mut pack = foundations();
        pack.episodes[0].steps[0].display = Some("Н".into());
        pack.episodes[0].steps[0]
            .requires_meanings
            .push("cat".into());
        let errors = pack.validate().unwrap_err();
        assert!(errors.iter().any(|e| e.contains("untaught shape Н")));
        assert!(errors.iter().any(|e| e.contains("unpresented meaning cat")));
    }
    #[test]
    fn append_keeps_original_sources_and_rejects_changed_prefix() {
        let mut pack = foundations();
        let old = foundations_legacy();
        assert_eq!(&pack.episodes[..3], old.episodes.as_slice());
        assert_eq!(foundation_step_pack_revision("f1-a"), old.revision);
        assert_eq!(foundation_step_pack_revision("c7-i-audio"), pack.revision);
        pack.episodes[0].steps[0].instruction.push_str(" changed");
        assert!(
            pack.validate()
                .unwrap_err()
                .iter()
                .any(|e| e.contains("prefix changed"))
        );
    }
    #[test]
    fn recall_requires_sample_instruction_and_an_unexposed_target() {
        let mut pack = foundations();
        let step = pack
            .episodes
            .iter_mut()
            .flat_map(|e| &mut e.steps)
            .find(|s| s.id == "c6-they-read")
            .unwrap();
        step.requires_samples
            .push("unintroduced-soft-context".into());
        let errors = pack.validate().unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.contains("unintroduced-soft-context"))
        );
        let step = pack
            .episodes
            .iter_mut()
            .flat_map(|e| &mut e.steps)
            .find(|s| s.id == "c5-window-write")
            .unwrap();
        step.display = Some("окно".into());
        assert!(
            pack.validate()
                .unwrap_err()
                .iter()
                .any(|e| e.contains("independent written response"))
        );
    }
    #[test]
    fn rejects_missing_audio_and_answer_bearing_chunk_script() {
        let mut pack = foundations();
        let step = &mut pack.episodes[0].steps[0];
        step.audio_ids.push("absent-model".into());
        step.audio_first_chunk = true;
        let errors = pack.validate().unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.contains("unknown audio absent-model"))
        );
        assert!(
            errors
                .iter()
                .any(|e| e.contains("reveals audio-first script"))
        );
    }
}
