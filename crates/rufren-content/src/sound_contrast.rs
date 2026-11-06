//! Authored Sound Focus definitions, independent of pack editions and old exercises.
//! Versions identify intended meanings/text, never acoustic certification. Runtime
//! audio bindings and durable trials belong to the core/app, not this catalog.
use crate::{ContentError, ContentPack, Exercise, Lesson};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use unicode_normalization::UnicodeNormalization;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SoundUtteranceKind {
    IsolatedWord,
    CarrierSentence,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SoundUtteranceDefinition {
    pub id: String,
    pub version: String,
    /// Exact recorded/requested utterance, including punctuation and authored stress.
    pub text: String,
    pub meaning: String,
    pub kind: SoundUtteranceKind,
    /// Lexical provenance only. This is not the utterance's audio revision identity.
    pub related_phrase_id: String,
    pub related_phrase_revision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SoundMemberDefinition {
    pub id: String,
    pub version: String,
    /// Safe English choice label: no revealing Russian spelling during a trial.
    pub option_label: String,
    pub lexeme: String,
    pub supported_lexeme: String,
    pub utterance: SoundUtteranceDefinition,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SoundTaskDefinition {
    pub id: String,
    pub version: String,
    pub instruction: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SoundContrastDefinition {
    pub id: String,
    pub version: String,
    pub lesson_id: String,
    pub target_id: String,
    pub title: String,
    pub teaching_note: String,
    pub detail_note: String,
    pub provenance_note: String,
    pub source_ids: Vec<String>,
    pub members: [SoundMemberDefinition; 2],
    pub identify_task: SoundTaskDefinition,
    pub produce_task: SoundTaskDefinition,
    /// Replaced only when offering NEW actions. Raw lookup/history stays intact.
    pub retired_exercise_ids: Vec<String>,
}

/// One shared projection for new lesson buttons and new AI-teacher task material.
/// Saved sessions must continue using their original exercise/task snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LessonPracticeAction<'a> {
    Exercise(&'a Exercise),
    SoundFocus(SoundContrastDefinition),
}

fn task(id: &str, instruction: &str) -> SoundTaskDefinition {
    SoundTaskDefinition {
        id: id.into(),
        version: "1".into(),
        instruction: instruction.into(),
    }
}
fn member(
    name: &str,
    label: &str,
    lexeme: &str,
    supported: &str,
    phrase: &str,
    text: &str,
    meaning: &str,
    kind: SoundUtteranceKind,
) -> SoundMemberDefinition {
    let context = match kind {
        SoundUtteranceKind::IsolatedWord => "isolated",
        SoundUtteranceKind::CarrierSentence => "carrier",
    };
    SoundMemberDefinition {
        id: format!("ru.sound.member.{name}"),
        version: "1".into(),
        option_label: label.into(),
        lexeme: lexeme.into(),
        supported_lexeme: supported.into(),
        utterance: SoundUtteranceDefinition {
            id: format!("ru.sound.utterance.{name}.{context}"),
            version: "1".into(),
            text: text.into(),
            meaning: meaning.into(),
            kind,
            related_phrase_id: phrase.into(),
            related_phrase_revision: "2026-11-05.3".into(),
        },
    }
}

/// Immutable authored v1 definitions. Changing a member, utterance, instruction,
/// option, or teaching note requires a new version, while retaining old snapshots.
pub fn sound_contrasts() -> [SoundContrastDefinition; 2] {
    let provenance = "AI-authored intended meanings and teaching notes, based on research/SOUND_FOCUS.md (2026-11-06). Audio realization is not certified by these labels; preserve each model's actual source and quality status.";
    [
        SoundContrastDefinition {
            id: "ru.sound.contrast.l-hard-soft".into(), version: "1".into(),
            lesson_id: "ru.lesson.naming".into(), target_id: "ru.target.hard_soft".into(),
            title: "Onion or hatch?".into(),
            teaching_note: "Both start with an l sound. For soft л, raise the middle of your tongue toward the roof of your mouth while making the consonant. Listen to люк beside лук.".into(),
            detail_note: "Make the softness during the consonant, rather than adding a full extra й syllable. Here лук means an onion; its other meaning, a bow, is outside this pair.".into(),
            provenance_note: provenance.into(), source_ids: vec!["ru.source.softness".into()],
            members: [
                member("onion", "An onion", "лук", "лук", "ru.phrase.naming.07", "Лук.", "An onion.", SoundUtteranceKind::IsolatedWord),
                member("hatch", "A hatch", "люк", "люк", "ru.phrase.naming.08", "Люк.", "A hatch.", SoundUtteranceKind::IsolatedWord),
            ],
            identify_task: task("ru.task.sound.l-hard-soft.identify", "Listen, then choose the meaning you heard. Both meaning choices stay visible; the playing word stays hidden until you answer or request help."),
            produce_task: task("ru.task.sound.l-hard-soft.produce", "Select the meaning you intend to say here, then record that word. You can listen to the model first if you like."),
            retired_exercise_ids: vec!["ru.exercise.naming.hear-soft".into(), "ru.exercise.naming.say-contrast".into()],
        },
        SoundContrastDefinition {
            id: "ru.sound.contrast.lexical-stress".into(), version: "1".into(),
            lesson_id: "ru.lesson.naming".into(), target_id: "ru.target.stress_contrast".into(),
            title: "Castle or lock?".into(),
            teaching_note: "The strong syllable changes: за́-мок is a castle; за-мо́к is a lock. Listen to the whole word.".into(),
            detail_note: "Stress can affect duration and vowel quality as well as other cues. It is not simply a higher pitch or extra volume. The accent marks help show the intended word; hearing and saying it are separate practice.".into(),
            provenance_note: provenance.into(), source_ids: vec!["ru.source.stress-pair".into()],
            members: [
                member("castle", "A castle", "замок", "за́мок", "ru.phrase.naming.09", "Это за́мок.", "This is a castle.", SoundUtteranceKind::CarrierSentence),
                member("lock", "A lock", "замок", "замо́к", "ru.phrase.naming.10", "Это замо́к.", "This is a lock.", SoundUtteranceKind::CarrierSentence),
            ],
            identify_task: task("ru.task.sound.lexical-stress.identify", "Listen, then choose the meaning you heard. Both meaning choices stay visible; the stressed word stays hidden until you answer or request help."),
            produce_task: task("ru.task.sound.lexical-stress.produce", "Select the meaning you intend to say here, then record its short Это … sentence. You can listen to the model first if you like."),
            retired_exercise_ids: vec!["ru.exercise.naming.hear-stress".into()],
        },
    ]
}

pub fn sound_contrast(id: &str, version: &str) -> Option<SoundContrastDefinition> {
    sound_contrasts()
        .into_iter()
        .find(|definition| definition.id == id && definition.version == version)
}

impl ContentPack {
    /// Validate exact authored bindings without changing or migrating the pack.
    pub fn validate_sound_contrasts(&self) -> Result<(), ContentError> {
        let mut errors = Vec::new();
        let mut ids = BTreeSet::new();
        let mut retired = BTreeSet::new();
        let ordinary = |text: &str| {
            text.chars()
                .filter(|c| *c != '\u{301}')
                .nfc()
                .collect::<String>()
        };
        for contrast in sound_contrasts() {
            if self.lesson(&contrast.lesson_id).is_none()
                || self.target(&contrast.target_id).is_none()
            {
                errors.push(format!(
                    "Sound contrast lesson/target missing: {}",
                    contrast.id
                ));
            }
            for source in &contrast.source_ids {
                if !self.sources.iter().any(|record| &record.id == source) {
                    errors.push(format!("Sound source missing: {source}"));
                }
            }
            for (id, version) in [
                (contrast.id.as_str(), contrast.version.as_str()),
                (&contrast.identify_task.id, &contrast.identify_task.version),
                (&contrast.produce_task.id, &contrast.produce_task.version),
            ] {
                if !ids.insert(id.to_owned()) || version.trim().is_empty() {
                    errors.push(format!("Duplicate/invalid sound identity: {id}"));
                }
            }
            for member in &contrast.members {
                for (id, version) in [
                    (&member.id, &member.version),
                    (&member.utterance.id, &member.utterance.version),
                ] {
                    if !ids.insert(id.clone()) || version.trim().is_empty() {
                        errors.push(format!("Duplicate/invalid sound identity: {id}"));
                    }
                }
                if member.option_label.is_empty()
                    || member
                        .option_label
                        .chars()
                        .any(|c| ('А'..='я').contains(&c) || c == 'ё' || c == 'Ё')
                {
                    errors.push(format!(
                        "Sound meaning label exposes spelling: {}",
                        member.id
                    ));
                }
                let utterance = &member.utterance;
                match self.phrase(&utterance.related_phrase_id) {
                    Some(phrase)
                        if self.resolved_phrase_lineage(phrase).revision
                            == utterance.related_phrase_revision
                            && phrase.target_ids.contains(&contrast.target_id)
                            && self.lesson(&contrast.lesson_id).is_some_and(|lesson| {
                                lesson.phrase_ids.contains(&utterance.related_phrase_id)
                            }) =>
                    {
                        let matches = match utterance.kind {
                            SoundUtteranceKind::IsolatedWord => {
                                ordinary(&utterance.text)
                                    .trim_end_matches('.')
                                    .to_lowercase()
                                    == member.lexeme
                            }
                            SoundUtteranceKind::CarrierSentence => {
                                ordinary(&utterance.text) == phrase.russian
                                    && utterance.text.contains(&member.supported_lexeme)
                            }
                        };
                        if !matches || !phrase.stressed.contains(&member.supported_lexeme) {
                            errors.push(format!(
                                "Sound utterance differs from intended member: {}",
                                utterance.id
                            ));
                        }
                    }
                    _ => errors.push(format!(
                        "Sound lexical provenance unavailable: {}",
                        member.id
                    )),
                }
            }
            for old in &contrast.retired_exercise_ids {
                if !retired.insert(old.clone())
                    || self
                        .exercise(old)
                        .is_none_or(|exercise| exercise.lesson_id != contrast.lesson_id)
                {
                    errors.push(format!("Invalid retired sound action: {old}"));
                }
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(ContentError::Invalid(errors))
        }
    }

    /// New actions only. Historical exercise lookup, raw membership, and resumable
    /// snapshots retain their original questions. Sound Focus is the primary entry.
    pub fn lesson_practice_actions<'a>(
        &'a self,
        lesson: &'a Lesson,
    ) -> Result<Vec<LessonPracticeAction<'a>>, ContentError> {
        if self.lesson(&lesson.id) != Some(lesson) {
            return Err(ContentError::Invalid(vec![
                "New practice needs the current lesson definition".into(),
            ]));
        }
        let contrasts: Vec<_> = sound_contrasts()
            .into_iter()
            .filter(|definition| definition.lesson_id == lesson.id)
            .collect();
        if !contrasts.is_empty() {
            self.validate_sound_contrasts()?;
        }
        let retired: BTreeSet<_> = contrasts
            .iter()
            .flat_map(|definition| definition.retired_exercise_ids.iter().cloned())
            .collect();
        let mut actions: Vec<_> = contrasts
            .into_iter()
            .map(LessonPracticeAction::SoundFocus)
            .collect();
        actions.extend(
            self.lesson_exercises(lesson)
                .filter(|exercise| !retired.contains(&exercise.id))
                .map(LessonPracticeAction::Exercise),
        );
        Ok(actions)
    }
}
