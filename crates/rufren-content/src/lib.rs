//! Versioned original Russian learning material. Content is editorial draft, not a
//! pronunciation reference or proof of competence. No runtime network or core dependency.
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use unicode_normalization::UnicodeNormalization;

pub const SCHEMA_VERSION: u32 = 1;
pub const EMBEDDED_JSON: &str = include_str!("../data/beginner-v1.json");

#[derive(Debug)]
pub enum ContentError {
    Json(serde_json::Error),
    UnsupportedSchema(u32),
    Invalid(Vec<String>),
}
impl std::fmt::Display for ContentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json(e) => write!(f, "Invalid content JSON: {e}"),
            Self::UnsupportedSchema(v) => write!(f, "Unsupported content schema {v}"),
            Self::Invalid(errors) => write!(f, "Invalid content: {}", errors.join("; ")),
        }
    }
}
impl std::error::Error for ContentError {}

macro_rules! model {
    ($name:ident { $($(#[$field_meta:meta])* $field:ident : $ty:ty),* $(,)? }) => {
        #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
        #[serde(deny_unknown_fields)]
        pub struct $name { $($(#[$field_meta])* pub $field: $ty),* }
    };
}
model!(ContentPack {
    schema_version: u32, id: String, revision: String, previous_revisions: Vec<String>, title: String,
    language: String, editorial_status: EditorialStatus, review_notice: String,
    units: Vec<Unit>, lessons: Vec<Lesson>, targets: Vec<Target>,
    phrases: Vec<Phrase>, exercises: Vec<Exercise>, script: Vec<Letter>, sources: Vec<Source>,
});
model!(Unit { id: String, title: String, description: String, lesson_ids: Vec<String> });
model!(Lesson {
    id: String, unit_id: String, title: String, goal: String,
    prerequisites: Vec<String>, target_ids: Vec<String>, phrase_ids: Vec<String>,
    exercise_ids: Vec<String>, explanations: Vec<Explanation>, dialogue: Dialogue,
    #[serde(alias = "coach_activity")]
    conversation_invitation: String, transfer_prompt: String, sound_note: String,
});
model!(Target { id: String, title: String, description: String, prerequisites: Vec<String> });
model!(Phrase {
    id: String, russian: String, stressed: String, translation: String,
    chunks: Vec<String>, target_ids: Vec<String>, note: String,
    audio: AudioStatus, editorial_status: EditorialStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    phrase_revision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    previous_phrase_revisions: Option<Vec<String>>,
});
/// Resolved immutable import provenance. Consumers must use all three fields of
/// the imported event (ID, source attribution, and parent set) from one resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedPhraseLineage<'a> {
    pub revision: &'a str,
    pub previous_revisions: &'a [String],
}
model!(Dialogue { title: String, situation: String, turns: Vec<DialogueTurn> });
model!(DialogueTurn {
    speaker: String,
    phrase_id: String
});
model!(Explanation { title: String, text: String, source_ids: Vec<String> });
model!(Letter { upper: String, lower: String, name: String, cue: String, examples: Vec<String> });
model!(Source {
    id: String,
    title: String,
    url: String,
    accessed: String,
    scope: String,
    access_note: String
});
model!(Exercise {
    id: String, lesson_id: String, kind: ExerciseKind, response_mode: ResponseMode,
    instruction: String, reference_phrase_id: String, stimulus_phrase_ids: Vec<String>, target_ids: Vec<String>,
    options: Vec<ChoiceOption>, answer: AnswerSpec, hint: String, feedback: String,
    requires_audio: bool,
});
model!(ChoiceOption {
    id: String,
    text: String
});

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EditorialStatus {
    GeneratedDraft,
    CoachReviewed,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AudioStatus {
    /// The host generates speech on demand; no audio bytes are embedded in the pack.
    PendingGeneratedSpeech,
    /// Legacy state accepted for older content. Human recording is optional.
    MissingCoachRecording,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExerciseKind {
    ReadingComprehension,
    ListeningComprehension,
    WrittenRetrieval,
    SpokenRetrieval,
    Contrast,
    Composition,
    Dictation,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResponseMode {
    Choice,
    Text,
    Speech,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum AnswerSpec {
    Exact {
        variants: Vec<String>,
        policy: AnswerPolicy,
    },
    Choice {
        correct_option: String,
    },
    /// Legacy wire name for contextual review. The AI teacher is the default reviewer.
    CoachReview {
        example: String,
        criteria: Vec<String>,
    },
}
/// Tolerances describe this task, never a general claim of linguistic equivalence.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AnswerPolicy {
    pub ignore_case: bool,
    pub collapse_whitespace: bool,
    pub ignore_terminal_punctuation: bool,
    pub require_stress: bool,
}
impl Default for AnswerPolicy {
    fn default() -> Self {
        Self {
            ignore_case: true,
            collapse_whitespace: true,
            ignore_terminal_punctuation: true,
            require_stress: false,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnswerMatch {
    /// Matches an explicitly authored answer under this task's tolerances.
    Accepted,
    /// Finite answer lists do not establish that another free answer is wrong.
    NotMatched,
    NeedsReview,
}

pub fn normalize_answer(text: &str, policy: AnswerPolicy) -> String {
    let canonical: String = text.nfc().collect();
    let mut value: String = canonical
        .chars()
        .filter(|c| policy.require_stress || *c != '\u{301}')
        .collect();
    if policy.ignore_case {
        value = value.to_lowercase();
    }
    if policy.collapse_whitespace {
        value = value.split_whitespace().collect::<Vec<_>>().join(" ");
    } else {
        value = value.trim().to_owned();
    }
    if policy.ignore_terminal_punctuation {
        value = value
            .trim_end_matches(['.', '!', '?', '…'])
            .trim_end()
            .to_owned();
    }
    value
}
impl Exercise {
    /// Speech never receives an automatic correctness judgment from text matching.
    pub fn check_answer(&self, response: &str) -> AnswerMatch {
        if self.response_mode == ResponseMode::Speech {
            return AnswerMatch::NeedsReview;
        }
        match &self.answer {
            AnswerSpec::Exact { variants, policy } => {
                let value = normalize_answer(response, *policy);
                if variants
                    .iter()
                    .any(|v| normalize_answer(v, *policy) == value)
                {
                    AnswerMatch::Accepted
                } else {
                    AnswerMatch::NotMatched
                }
            }
            AnswerSpec::Choice { correct_option } => {
                if response.trim() == correct_option {
                    AnswerMatch::Accepted
                } else {
                    AnswerMatch::NotMatched
                }
            }
            AnswerSpec::CoachReview { .. } => AnswerMatch::NeedsReview,
        }
    }
    /// Does the host need an audio asset for this activity? Generated speech is
    /// the default source; an optional human recording also satisfies the need.
    /// This reports embedded-pack state, not the host's generated-audio cache.
    pub fn needs_audio(&self, pack: &ContentPack) -> bool {
        self.requires_audio
            && self.stimulus_phrase_ids.iter().any(|id| {
                pack.phrase(id).is_none_or(|p| {
                    matches!(
                        p.audio,
                        AudioStatus::PendingGeneratedSpeech | AudioStatus::MissingCoachRecording
                    )
                })
            })
    }
    /// Compatibility alias. Needing audio never requires a human to record it.
    pub fn needs_recording(&self, pack: &ContentPack) -> bool {
        self.needs_audio(pack)
    }
}
pub fn load_embedded() -> Result<ContentPack, ContentError> {
    ContentPack::from_json(EMBEDDED_JSON)
}
impl ContentPack {
    pub fn from_json(json: &str) -> Result<Self, ContentError> {
        let pack: Self = serde_json::from_str(json).map_err(ContentError::Json)?;
        pack.validate()?;
        Ok(pack)
    }
    pub fn lesson(&self, id: &str) -> Option<&Lesson> {
        self.lessons.iter().find(|x| x.id == id)
    }
    /// Legacy phrases without either optional field preserve the original pack-based
    /// derivation. Explicit empty predecessors never fall back to pack ancestry.
    /// Validation rejects partial declarations before a pack reaches consumers.
    pub fn resolved_phrase_lineage<'a>(&'a self, phrase: &'a Phrase) -> ResolvedPhraseLineage<'a> {
        match (&phrase.phrase_revision, &phrase.previous_phrase_revisions) {
            (Some(revision), Some(previous_revisions)) => ResolvedPhraseLineage {
                revision,
                previous_revisions,
            },
            _ => ResolvedPhraseLineage {
                revision: &self.revision,
                previous_revisions: &self.previous_revisions,
            },
        }
    }
    pub fn phrase(&self, id: &str) -> Option<&Phrase> {
        self.phrases.iter().find(|x| x.id == id)
    }
    pub fn target(&self, id: &str) -> Option<&Target> {
        self.targets.iter().find(|x| x.id == id)
    }
    pub fn exercise(&self, id: &str) -> Option<&Exercise> {
        self.exercises.iter().find(|x| x.id == id)
    }
    pub fn unit(&self, id: &str) -> Option<&Unit> {
        self.units.iter().find(|x| x.id == id)
    }
    pub fn lesson_phrases<'a>(
        &'a self,
        lesson: &'a Lesson,
    ) -> impl Iterator<Item = &'a Phrase> + 'a {
        lesson.phrase_ids.iter().filter_map(|id| self.phrase(id))
    }
    pub fn lesson_exercises<'a>(
        &'a self,
        lesson: &'a Lesson,
    ) -> impl Iterator<Item = &'a Exercise> + 'a {
        lesson
            .exercise_ids
            .iter()
            .filter_map(|id| self.exercise(id))
    }
    /// Prerequisites are encounter suggestions, not inferred mastery requirements.
    pub fn available_lessons<'a>(
        &'a self,
        encountered: &'a BTreeSet<String>,
    ) -> impl Iterator<Item = &'a Lesson> + 'a {
        self.lessons.iter().filter(move |lesson| {
            !encountered.contains(&lesson.id)
                && lesson
                    .prerequisites
                    .iter()
                    .all(|id| encountered.contains(id))
        })
    }
    pub fn validate(&self) -> Result<(), ContentError> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(ContentError::UnsupportedSchema(self.schema_version));
        }
        let mut errors = Vec::new();
        let mut all = BTreeSet::new();
        for id in self
            .units
            .iter()
            .map(|x| &x.id)
            .chain(self.lessons.iter().map(|x| &x.id))
            .chain(self.targets.iter().map(|x| &x.id))
            .chain(self.phrases.iter().map(|x| &x.id))
            .chain(self.exercises.iter().map(|x| &x.id))
            .chain(self.sources.iter().map(|x| &x.id))
        {
            if id.is_empty() || !id.starts_with("ru.") || !all.insert(id) {
                errors.push(format!("Invalid or duplicate ID: {id}"));
            }
        }
        if !self.id.starts_with("ru.pack.")
            || self.language != "ru"
            || self.revision.trim().is_empty()
            || self.units.is_empty()
            || self.lessons.is_empty()
        {
            errors.push("Invalid pack identity, language, revision, or empty curriculum".into());
        }
        let mut lineage = BTreeSet::new();
        if self.revision.len() > 128
            || self.revision.trim() != self.revision
            || self.previous_revisions.len() > 64
        {
            errors.push("Invalid or oversized revision lineage".into());
        }
        for previous in &self.previous_revisions {
            if previous.trim().is_empty()
                || previous.trim() != previous
                || previous.len() > 128
                || previous == &self.revision
                || !lineage.insert(previous)
            {
                errors.push(format!("Invalid previous revision: {previous:?}"));
            }
        }
        let target_ids: BTreeSet<_> = self.targets.iter().map(|x| x.id.as_str()).collect();
        let source_ids: BTreeSet<_> = self.sources.iter().map(|x| x.id.as_str()).collect();
        for target in &self.targets {
            for id in &target.prerequisites {
                if !target_ids.contains(id.as_str()) || id == &target.id {
                    errors.push(format!("Bad target prerequisite {id}"));
                }
            }
        }
        let mut covered_lessons = BTreeSet::new();
        for unit in &self.units {
            for id in &unit.lesson_ids {
                if !covered_lessons.insert(id) {
                    errors.push(format!("Lesson listed twice: {id}"));
                }
                if self.lesson(id).is_none_or(|l| l.unit_id != unit.id) {
                    errors.push(format!("Bad unit membership {id}"));
                }
            }
        }
        let mut preceding = BTreeSet::new();
        let mut covered_exercises = BTreeSet::new();
        let mut covered_phrases = BTreeSet::new();
        for lesson in &self.lessons {
            if !covered_lessons.contains(&lesson.id)
                || lesson.phrase_ids.is_empty()
                || lesson.exercise_ids.is_empty()
            {
                errors.push(format!("Incomplete lesson {}", lesson.id));
            }
            for id in &lesson.prerequisites {
                if !preceding.contains(id) {
                    errors.push(format!("Nonpreceding lesson prerequisite {id}"));
                }
            }
            preceding.insert(lesson.id.clone());
            for id in &lesson.target_ids {
                if !target_ids.contains(id.as_str()) {
                    errors.push(format!("Unknown target {id}"));
                }
            }
            for id in &lesson.phrase_ids {
                covered_phrases.insert(id);
                if self.phrase(id).is_none() {
                    errors.push(format!("Unknown phrase {id}"));
                }
            }
            for id in &lesson.exercise_ids {
                if !covered_exercises.insert(id)
                    || self.exercise(id).is_none_or(|e| e.lesson_id != lesson.id)
                {
                    errors.push(format!("Bad exercise membership {id}"));
                }
            }
            for turn in &lesson.dialogue.turns {
                if turn.speaker.is_empty() || !lesson.phrase_ids.contains(&turn.phrase_id) {
                    errors.push(format!("Invalid dialogue turn in {}", lesson.id));
                }
            }
            for note in &lesson.explanations {
                for id in &note.source_ids {
                    if !source_ids.contains(id.as_str()) {
                        errors.push(format!("Unknown source {id}"));
                    }
                }
            }
        }
        for phrase in &self.phrases {
            match (&phrase.phrase_revision, &phrase.previous_phrase_revisions) {
                (None, None) => (),
                (Some(revision), Some(previous)) => {
                    let mut unique = BTreeSet::new();
                    let allowed = |value: &str| {
                        value == self.revision || self.previous_revisions.iter().any(|v| v == value)
                    };
                    if revision.trim().is_empty()
                        || revision.trim() != revision
                        || revision.len() > 128
                        || !allowed(revision)
                        || previous.len() > 64
                    {
                        errors.push(format!("Invalid phrase revision {}", phrase.id));
                    }
                    for parent in previous {
                        if parent.trim().is_empty()
                            || parent.trim() != parent
                            || parent.len() > 128
                            || parent == revision
                            || !allowed(parent)
                            || !unique.insert(parent)
                        {
                            errors.push(format!("Invalid phrase predecessor {}", phrase.id));
                        }
                    }
                }
                _ => errors.push(format!("Partial phrase lineage {}", phrase.id)),
            }
            if !covered_phrases.contains(&phrase.id)
                || phrase.russian.is_empty()
                || phrase.translation.is_empty()
                || phrase.chunks.is_empty()
            {
                errors.push(format!("Incomplete phrase {}", phrase.id));
            }
            // NFC preserves ё; only the pedagogical acute is removed.
            let stripped: String = phrase
                .stressed
                .chars()
                .filter(|c| *c != '\u{301}')
                .nfc()
                .collect();
            if phrase.chunks.iter().any(|chunk| chunk.trim().is_empty())
                || phrase.chunks.join(" ") != phrase.stressed
            {
                errors.push(format!("Invalid phrase chunks: {}", phrase.id));
            }
            if stripped != phrase.russian.nfc().collect::<String>() {
                errors.push(format!("Stress text changes spelling: {}", phrase.id));
            }
            for id in &phrase.target_ids {
                if !target_ids.contains(id.as_str()) {
                    errors.push(format!("Unknown target {id}"));
                }
            }
        }
        for ex in &self.exercises {
            if !covered_exercises.contains(&ex.id)
                || ex.instruction.is_empty()
                || ex.target_ids.is_empty()
            {
                errors.push(format!("Incomplete exercise {}", ex.id));
            }
            if self.phrase(&ex.reference_phrase_id).is_none()
                || self
                    .lesson(&ex.lesson_id)
                    .is_none_or(|lesson| !lesson.phrase_ids.contains(&ex.reference_phrase_id))
            {
                errors.push(format!("Invalid exercise reference {}", ex.id));
            }
            for id in &ex.stimulus_phrase_ids {
                if self.phrase(id).is_none() {
                    errors.push(format!("Unknown stimulus {id}"));
                }
            }
            for id in &ex.target_ids {
                if !target_ids.contains(id.as_str()) {
                    errors.push(format!("Unknown target {id}"));
                }
            }
            let option_ids: BTreeSet<_> = ex.options.iter().map(|x| &x.id).collect();
            if option_ids.len() != ex.options.len() {
                errors.push(format!("Duplicate choice options {}", ex.id));
            }
            match &ex.answer {
                AnswerSpec::Exact { variants, .. }
                    if variants.is_empty() || variants.iter().any(|v| v.trim().is_empty()) =>
                {
                    errors.push(format!("Empty answer {}", ex.id))
                }
                AnswerSpec::Choice { correct_option }
                    if ex.response_mode != ResponseMode::Choice
                        || !option_ids.contains(correct_option)
                        || ex.options.len() < 2 =>
                {
                    errors.push(format!("Bad choices {}", ex.id))
                }
                AnswerSpec::CoachReview { example, criteria }
                    if example.is_empty() || criteria.is_empty() =>
                {
                    errors.push(format!("Empty review rubric {}", ex.id))
                }
                _ => (),
            }
            if matches!(
                ex.kind,
                ExerciseKind::ListeningComprehension | ExerciseKind::Dictation
            ) && (!ex.requires_audio || ex.stimulus_phrase_ids.is_empty())
            {
                errors.push(format!("Listening task lacks audio requirement {}", ex.id));
            }
        }
        // Reject target cycles even if IDs resolve; consumers may traverse this graph.
        fn visit<'a>(
            id: &'a str,
            pack: &'a ContentPack,
            active: &mut BTreeSet<&'a str>,
            done: &mut BTreeSet<&'a str>,
        ) -> bool {
            if done.contains(id) {
                return true;
            }
            if !active.insert(id) {
                return false;
            }
            if let Some(t) = pack.target(id) {
                for dep in &t.prerequisites {
                    if !visit(dep, pack, active, done) {
                        return false;
                    }
                }
            }
            active.remove(id);
            done.insert(id);
            true
        }
        let mut done = BTreeSet::new();
        for t in &self.targets {
            if !visit(&t.id, self, &mut BTreeSet::new(), &mut done) {
                errors.push(format!("Target cycle at {}", t.id));
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(ContentError::Invalid(errors))
        }
    }
}
