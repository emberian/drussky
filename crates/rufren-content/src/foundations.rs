//! AI-authored prepared first encounters. Introductions are exposure, not mastery.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

pub const FOUNDATION_ORIGINAL_JSON: &str = include_str!("../content/foundations-original-v1.json");
pub const FOUNDATION_ORIGINAL_REVISION: &str = "foundation-2026-11-06.1";
pub const FOUNDATION_PREVIOUS_JSON: &str = include_str!("../content/foundations-previous-v2.json");
pub const FOUNDATION_PREVIOUS_REVISION: &str = "foundation-2026-11-06.2";
pub const FOUNDATION_CURRENT_REVISION: &str = "foundation-2026-11-06.3";

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

/// Exact seven-episode edition, frozen before the complete alphabet append.
pub fn foundations_previous() -> FoundationPack {
    serde_json::from_str(FOUNDATION_PREVIOUS_JSON).expect("previous foundation must parse")
}

/// Only known immutable editions may supply a saved cursor's old task graph.
pub fn foundations_edition(revision: &str) -> Option<FoundationPack> {
    match revision {
        FOUNDATION_ORIGINAL_REVISION => Some(foundations_legacy()),
        FOUNDATION_PREVIOUS_REVISION => Some(foundations_previous()),
        FOUNDATION_CURRENT_REVISION => Some(foundations()),
        _ => None,
    }
}

pub fn foundation_step_pack_revision(step_id: &str) -> String {
    // Old steps keep the edition in which their immutable target was authored,
    // even when their containing catalog acquires more episodes. Cache the small
    // manifest rather than parsing an entire course for every evidence event.
    static INTRODUCED: OnceLock<BTreeMap<String, &'static str>> = OnceLock::new();
    let introduced = INTRODUCED.get_or_init(|| {
        let mut introduced = BTreeMap::new();
        for (revision, pack) in [
            (FOUNDATION_ORIGINAL_REVISION, foundations_legacy()),
            (FOUNDATION_PREVIOUS_REVISION, foundations_previous()),
            (FOUNDATION_CURRENT_REVISION, foundations()),
        ] {
            for step in pack.episodes.into_iter().flat_map(|e| e.steps) {
                introduced.entry(step.id).or_insert(revision);
            }
        }
        introduced
    });
    introduced
        .get(step_id)
        .copied()
        .unwrap_or(FOUNDATION_CURRENT_REVISION)
        .to_owned()
}

pub fn foundations() -> FoundationPack {
    serde_json::from_str(FOUNDATION_JSON).expect("bundled foundation JSON must parse")
}
impl FoundationPack {
    /// An append may extend a known catalog, never rewrite its old tasks/models.
    pub fn is_append_only_extension_of(&self, previous: &Self) -> bool {
        self.schema_version == previous.schema_version
            && self.revision != previous.revision
            && self.previous_revisions.contains(&previous.revision)
            && self.episodes.len() > previous.episodes.len()
            && self.episodes.get(..previous.episodes.len()) == Some(previous.episodes.as_slice())
            && self.assets.get(..previous.assets.len()) == Some(previous.assets.as_slice())
    }

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
        for previous in [foundations_legacy(), foundations_previous()] {
            if (self.revision == FOUNDATION_CURRENT_REVISION
                || self.revision == FOUNDATION_PREVIOUS_REVISION)
                && self.revision != previous.revision
                && !self.is_append_only_extension_of(&previous)
            {
                errors.push(format!(
                    "Append-only foundation prefix changed for {}",
                    previous.revision
                ));
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
        assert_eq!(pack.episodes.len(), 21);
        assert_eq!(
            pack.episodes.iter().map(|e| e.steps.len()).sum::<usize>(),
            244
        );
        let letters: BTreeSet<_> = pack
            .episodes
            .iter()
            .flat_map(|e| &e.steps)
            .flat_map(|s| s.introduces_graphemes.iter().cloned())
            .collect();
        assert_eq!(letters.len(), 33);
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
        let previous = foundations_previous();
        assert_eq!(&pack.episodes[..3], old.episodes.as_slice());
        assert_eq!(&pack.episodes[..7], previous.episodes.as_slice());
        assert_eq!(foundation_step_pack_revision("f1-a"), old.revision);
        assert_eq!(
            foundation_step_pack_revision("c7-i-audio"),
            previous.revision
        );
        assert_eq!(foundation_step_pack_revision("a8-ya"), pack.revision);
        pack.episodes[0].steps[0].instruction.push_str(" changed");
        assert!(
            pack.validate()
                .unwrap_err()
                .iter()
                .any(|e| e.contains("prefix changed"))
        );
    }
    #[test]
    fn frozen_editions_and_every_step_keep_their_introducing_source() {
        let original = foundations_legacy();
        let previous = foundations_previous();
        let current = foundations();
        assert_eq!(previous.validate(), Ok(()));
        assert!(previous.is_append_only_extension_of(&original));
        assert!(current.is_append_only_extension_of(&original));
        assert!(current.is_append_only_extension_of(&previous));
        assert_eq!(
            foundations_edition(FOUNDATION_ORIGINAL_REVISION),
            Some(original.clone())
        );
        assert_eq!(
            foundations_edition(FOUNDATION_PREVIOUS_REVISION),
            Some(previous.clone())
        );
        assert!(foundations_edition("unknown-edition").is_none());
        let mut introduced = BTreeSet::new();
        for edition in [original, previous, current] {
            for step in edition.episodes.iter().flat_map(|e| &e.steps) {
                if introduced.insert(step.id.clone()) {
                    assert_eq!(foundation_step_pack_revision(&step.id), edition.revision);
                }
            }
        }
        // Changing even a .2-only task or recording invalidates the append.
        let mut changed = foundations();
        changed.episodes[3].steps[0]
            .instruction
            .push_str(" changed");
        assert!(!changed.is_append_only_extension_of(&foundations_previous()));
        assert!(
            changed
                .validate()
                .unwrap_err()
                .iter()
                .any(|e| e.contains(FOUNDATION_PREVIOUS_REVISION))
        );
        let mut changed = foundations();
        changed.assets[19].spoken.push_str(" changed");
        assert!(!changed.is_append_only_extension_of(&foundations_previous()));
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
