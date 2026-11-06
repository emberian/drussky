//! AI-authored prepared first encounters. Introductions are exposure, not mastery.
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const FOUNDATION_JSON: &str = include_str!("../content/foundations-v1.json");
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FoundationPack {
    pub schema_version: u32,
    pub revision: String,
    pub editorial_notice: String,
    pub assets: Vec<FoundationAsset>,
    pub episodes: Vec<FoundationEpisode>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FoundationAsset { pub id: String, pub spoken: String }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FoundationEpisode {
    pub id: String, pub title: String, pub goal: String, pub steps: Vec<FoundationStep>,
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
    pub audio_first_chunk: bool,
    pub action: FoundationAction,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FoundationAction {
    Present { note: String },
    Choose { options: Vec<FoundationChoice>, correct_id: String, success: String },
    Blend { tiles: Vec<String>, answer: Vec<String>, feedback: Vec<String>, success: String },
    Read { reveal_text: String, note: String },
    Speak { note: String },
    Complete { summary: String },
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FoundationChoice { pub id: String, pub text: String, pub feedback: String }

pub fn foundations() -> FoundationPack {
    serde_json::from_str(FOUNDATION_JSON).expect("bundled foundation JSON must parse")
}
impl FoundationPack {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.schema_version != 1 { errors.push("Unsupported foundation schema".into()); }
        let mut ids = BTreeSet::new();
        let assets: BTreeSet<_> = self.assets.iter().map(|a| a.id.as_str()).collect();
        if assets.len() != self.assets.len() || self.assets.iter().any(|a| a.spoken.trim().is_empty()) {
            errors.push("Duplicate or empty foundation audio asset".into());
        }
        let mut graphemes = BTreeSet::<String>::new();
        let mut meanings = BTreeSet::<String>::new();
        for episode in &self.episodes {
            if !ids.insert(episode.id.clone()) { errors.push(format!("Duplicate ID {}", episode.id)); }
            for step in &episode.steps {
                if step.revision.trim().is_empty() { errors.push(format!("{} has no revision", step.id)); }
                if !ids.insert(step.id.clone()) { errors.push(format!("Duplicate ID {}", step.id)); }
                for g in &step.requires_graphemes {
                    if !graphemes.contains(g) { errors.push(format!("{} requires untaught grapheme {g}",step.id)); }
                }
                for m in &step.requires_meanings {
                    if !meanings.contains(m) { errors.push(format!("{} tests unpresented meaning {m}",step.id)); }
                }
                for a in &step.audio_ids {
                    if !assets.contains(a.as_str()) { errors.push(format!("{} has unknown audio {a}",step.id)); }
                }
                graphemes.extend(step.introduces_graphemes.iter().cloned());
                meanings.extend(step.introduces_meanings.iter().cloned());
                let mut written = step.display.clone().unwrap_or_default();
                match &step.action {
                    FoundationAction::Choose {options,correct_id,..} => {
                        if options.len()<2 || !options.iter().any(|o| &o.id==correct_id) || options.iter().any(|o|o.feedback.trim().is_empty()) {
                            errors.push(format!("{} has incomplete choices/feedback",step.id));
                        }
                        for o in options { written.push_str(&o.text); }
                    }
                    FoundationAction::Blend {tiles,answer,feedback,..} => {
                        if answer.is_empty() || answer.len()!=feedback.len() || answer.iter().any(|a|!tiles.contains(a)) { errors.push(format!("{} has invalid construction",step.id)); }
                        written.push_str(&tiles.join(""));
                    }
                    FoundationAction::Read{reveal_text,..} => written.push_str(reveal_text),
                    _ => (),
                }
                // Every visible or response-bearing Cyrillic shape must have been introduced.
                for c in written.chars().filter(|c| ('А'..='я').contains(c) || *c=='ё' || *c=='Ё') {
                    if !graphemes.contains(&c.to_uppercase().to_string()) { errors.push(format!("{} displays untaught shape {c}",step.id)); }
                }
                if step.audio_first_chunk && step.display.is_some() { errors.push(format!("{} reveals audio-first script",step.id)); }
            }
        }
        if errors.is_empty(){Ok(())}else{Err(errors)}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bundled_sequence_has_valid_prerequisites_and_assets() {
        let pack = foundations();
        assert_eq!(pack.episodes.len(), 3);
        assert_eq!(pack.validate(), Ok(()));
    }
    #[test]
    fn rejects_untaught_response_shape_and_unpresented_meaning() {
        let mut pack = foundations();
        pack.episodes[0].steps[0].display = Some("Н".into());
        pack.episodes[0].steps[0].requires_meanings.push("cat".into());
        let errors = pack.validate().unwrap_err();
        assert!(errors.iter().any(|e| e.contains("untaught shape Н")));
        assert!(errors.iter().any(|e| e.contains("unpresented meaning cat")));
    }
    #[test]
    fn rejects_missing_audio_and_answer_bearing_chunk_script() {
        let mut pack = foundations();
        let step = &mut pack.episodes[0].steps[0];
        step.audio_ids.push("absent-model".into());
        step.audio_first_chunk = true;
        let errors = pack.validate().unwrap_err();
        assert!(errors.iter().any(|e| e.contains("unknown audio absent-model")));
        assert!(errors.iter().any(|e| e.contains("reveals audio-first script")));
    }
}
