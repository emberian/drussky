//! Lesson provenance and user-controlled practice, separate from rendering.
use crate::{
    model::{AppState, Page},
    repository::{Result, now_ms},
};
use rufren_content::{Exercise, ExerciseKind, Lesson, Phrase};
use rufren_core::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use uuid::Uuid;

const CONTENT_NAMESPACE: Uuid = Uuid::from_u128(0x2774e059_4afc_50c2_8e35_0e5bf8b33bed);
// Authored revision IDs are deterministic across app releases. Their unchanged
// immutable facts must retain the original wire version when the event log grows.
const CONTENT_EVENT_VERSION: u32 = 2;
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Preferences {
    pub show_stress: bool,
    pub chunked: bool,
    pub review_skill: Skill,
    pub coach_name: String,
    pub sync_recordings: bool,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            show_stress: true,
            chunked: true,
            review_skill: Skill::Reading,
            coach_name: String::new(),
            sync_recordings: false,
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct ExerciseSnapshot {
    pub pack_id: String,
    pub pack_revision: String,
    pub lesson_title: String,
    pub exercise: Exercise,
    pub phrases: Vec<Phrase>,
}
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct SessionPlan {
    pub session: Uuid,
    pub lesson_id: Option<String>,
    pub exercise_id: Option<String>,
    pub started_at_ms: i64,
    pub policy: String,
    #[serde(default)]
    pub reference_plays: u32,
    #[serde(default)]
    pub response_recording: Option<AudioAttachment>,
    #[serde(default)]
    pub snapshot: Option<ExerciseSnapshot>,
}
impl AppState {
    pub(crate) fn save_preferences(&self) -> Result<()> {
        self.repo.set("preferences", &self.preferences)
    }
    pub(crate) fn phrase_id(&self, phrase: &Phrase) -> Uuid {
        Uuid::new_v5(
            &CONTENT_NAMESPACE,
            format!("{}:{}", self.content.id, phrase.id).as_bytes(),
        )
    }
    pub(crate) fn phrase_revision(&self, phrase: &Phrase) -> Uuid {
        Uuid::new_v5(
            &self.phrase_id(phrase),
            self.content
                .resolved_phrase_lineage(phrase)
                .revision
                .as_bytes(),
        )
    }
    pub(crate) fn import_lesson(&mut self, id: &str) -> Result<()> {
        let lesson = self
            .content
            .lesson(id)
            .ok_or("That lesson is not in this content pack")?;
        let events: Vec<_> = self
            .content
            .lesson_phrases(lesson)
            .map(|phrase| Event {
                version: CONTENT_EVENT_VERSION,
                id: self.phrase_revision(phrase),
                device: CONTENT_NAMESPACE,
                recorded_at_ms: 0,
                fact: Fact::PassageRevised(Revision {
                    passage: Passage {
                        id: self.phrase_id(phrase),
                        title: phrase.translation.clone(),
                        russian: phrase.russian.clone(),
                        translation: phrase.translation.clone(),
                        source: format!(
                            "Rufren original · editorial draft · {}@{}",
                            phrase.id,
                            self.content.resolved_phrase_lineage(phrase).revision
                        ),
                        coach_reviewed: false,
                    },
                    parents: self
                        .content
                        .resolved_phrase_lineage(phrase)
                        .previous_revisions
                        .iter()
                        .map(|revision| Uuid::new_v5(&self.phrase_id(phrase), revision.as_bytes()))
                        .collect(),
                }),
            })
            .collect();
        self.repo.append(&events)?;
        self.refresh()
    }
    pub(crate) fn choose_lesson(&mut self, id: &str) -> Result<()> {
        if self.content.lesson(id).is_none() {
            return Err("That lesson is not available".into());
        }
        self.repo.set("selected_lesson", &id)?;
        self.selected_lesson = Some(id.into());
        self.navigate(Page::LessonDetail)
    }
    pub(crate) fn lesson(&self) -> Option<&Lesson> {
        self.selected_lesson
            .as_deref()
            .and_then(|id| self.content.lesson(id))
    }
    pub(crate) fn exercise(&self) -> Option<&Exercise> {
        self.plan
            .as_ref()
            .and_then(|p| p.snapshot.as_ref())
            .map(|s| &s.exercise)
    }
    pub(crate) fn stimulus(&self, id: &str) -> Option<&Phrase> {
        self.plan
            .as_ref()
            .and_then(|p| p.snapshot.as_ref())
            .and_then(|s| s.phrases.iter().find(|p| p.id == id))
    }
    pub(crate) fn set_plan(
        &mut self,
        lesson_id: Option<String>,
        exercise_id: Option<String>,
        policy: &str,
    ) -> Result<()> {
        let session = self.session.as_ref().ok_or("No practice is active")?;
        let snapshot = exercise_id
            .as_deref()
            .and_then(|id| self.content.exercise(id))
            .map(|ex| ExerciseSnapshot {
                pack_id: self.content.id.clone(),
                pack_revision: self.content.revision.clone(),
                lesson_title: self
                    .content
                    .lesson(&ex.lesson_id)
                    .map(|l| l.title.clone())
                    .unwrap_or_default(),
                exercise: ex.clone(),
                phrases: ex
                    .stimulus_phrase_ids
                    .iter()
                    .chain(std::iter::once(&ex.reference_phrase_id))
                    .filter_map(|id| self.content.phrase(id).cloned())
                    .collect(),
            });
        let plan = SessionPlan {
            session: session.id,
            lesson_id,
            exercise_id,
            started_at_ms: now_ms(),
            policy: policy.into(),
            reference_plays: 0,
            response_recording: None,
            snapshot,
        };
        self.repo.set("session_plan", &plan)?;
        self.plan = Some(plan);
        Ok(())
    }
    pub(crate) fn start_exercise(&mut self, id: &str) -> Result<()> {
        let ex = self
            .content
            .exercise(id)
            .ok_or("That exercise is unavailable")?
            .clone();
        let phrase = self
            .content
            .phrase(&ex.reference_phrase_id)
            .ok_or("The reference phrase is unavailable")?
            .clone();
        self.import_lesson(&ex.lesson_id)?;
        let revision = self.phrase_revision(&phrase);
        let skill = match ex.kind {
            ExerciseKind::ListeningComprehension => Skill::Listening,
            ExerciseKind::SpokenRetrieval => Skill::Speaking,
            ExerciseKind::WrittenRetrieval
            | ExerciseKind::Composition
            | ExerciseKind::Dictation => Skill::Writing,
            _ => Skill::Reading,
        };
        self.start(revision, skill)?;
        self.set_plan(Some(ex.lesson_id), Some(ex.id), "authored-curriculum-v1")
    }
    pub(crate) fn practice_phrase(&mut self, id: &str, skill: Skill) -> Result<()> {
        let phrase = self.content.phrase(id).ok_or("No such phrase")?.clone();
        let lesson = self
            .content
            .lessons
            .iter()
            .find(|l| l.phrase_ids.iter().any(|p| p == id))
            .ok_or("No lesson for phrase")?
            .id
            .clone();
        self.import_lesson(&lesson)?;
        self.start(self.phrase_revision(&phrase), skill)?;
        self.set_plan(Some(lesson), None, "free-practice-v1")
    }
    pub(crate) fn update_response(&mut self, draft: String) -> Result<()> {
        if draft.len() > MAX_TEXT {
            return Err(
                "Keep one response under 32 KB; split a longer passage into smaller steps.".into(),
            );
        }
        let mut session = self.session.clone().ok_or("No active practice")?;
        if session.phase != Phase::Retrieve {
            return Err("Return to the response step to write an answer.".into());
        }
        session.draft = draft;
        self.repo.set("session", &Some(&session))?;
        self.session = Some(session);
        Ok(())
    }
    pub(crate) fn heard_reference(&mut self) -> Result<()> {
        let Some(mut plan) = self.plan.clone() else {
            return Ok(());
        };
        let Some(mut session) = self.session.clone() else {
            return Ok(());
        };
        if session.phase != Phase::Retrieve {
            return Ok(());
        };
        plan.reference_plays = plan.reference_plays.saturating_add(1);
        let audio_prompt =
            session.skill == Skill::Listening || self.exercise().is_some_and(|e| e.requires_audio);
        if (plan.reference_plays > 1 || !audio_prompt) && !session.cues.contains(&Cue::Replay) {
            session.cues.push(Cue::Replay);
        }
        self.repo.set("session", &Some(&session))?;
        self.repo.set("session_plan", &plan)?;
        self.plan = Some(plan);
        self.session = Some(session);
        self.note_reference_presentation()
    }
    pub(crate) fn attempt_context(&self, attempt: Uuid) -> Result<Option<AttemptContext>> {
        let Some(plan) = &self.plan else {
            return Ok(None);
        };
        let session = self.session.as_ref().ok_or("Missing practice")?;
        let ex = self.exercise();
        let task = match ex.map(|e| e.kind) {
            Some(ExerciseKind::ListeningComprehension)
                if ex.is_some_and(is_authored_sound_discrimination) =>
            {
                TaskKind::SoundDiscrimination
            }
            Some(ExerciseKind::Composition) => TaskKind::Composition,
            Some(ExerciseKind::Contrast) => TaskKind::GrammarGeneralization,
            Some(ExerciseKind::Dictation) => TaskKind::Spelling,
            _ => match session.skill {
                Skill::Listening | Skill::Reading => TaskKind::MeaningRecall,
                Skill::Speaking => TaskKind::FormRecall,
                Skill::Writing => TaskKind::FormRecall,
            },
        };
        let (prompt, reference_answer) = if let Some(exercise) = ex {
            let mut prompt = exercise.instruction.clone();
            if !exercise.stimulus_phrase_ids.is_empty() {
                prompt.push_str(if exercise.requires_audio {
                    "\nAudio input text (the learner heard the recording; text was hidden):"
                } else {
                    "\nVisible Russian task stimuli:"
                });
                for id in &exercise.stimulus_phrase_ids {
                    let phrase = self.stimulus(id).ok_or("The saved question is missing a stimulus; keep this draft and recover its content snapshot.")?;
                    prompt.push_str(&format!("\n{}: {}", id, phrase.russian));
                }
            }
            if !exercise.options.is_empty() {
                prompt.push_str("\nChoice IDs in the learner response refer to these options:");
                for option in &exercise.options {
                    prompt.push_str(&format!("\n{} = {}", option.id, option.text));
                }
            }
            let reference = match &exercise.answer {
                rufren_content::AnswerSpec::Exact { variants, policy } => format!(
                    "Authored acceptable variants:\n{}\nTask-specific matching tolerances: {}",
                    variants.join("\n"),
                    serde_json::to_string(policy).map_err(|e| e.to_string())?
                ),
                rufren_content::AnswerSpec::Choice { correct_option } => {
                    let option = exercise.options.iter().find(|option| &option.id == correct_option)
                        .ok_or("The saved question's answer option is unavailable; keep this draft and recover its content snapshot.")?;
                    format!("Authored correct option: {} = {}", option.id, option.text)
                }
                rufren_content::AnswerSpec::CoachReview { example, criteria } => format!(
                    "Illustrative response, not an exhaustive answer list: {}\nEvaluation criteria:\n{}",
                    example,
                    criteria.join("\n")
                ),
            };
            if prompt.len() > 8192 || reference.len() > 8192 {
                return Err("This authored question exceeds the 8 KB saved-question limit. Your draft is kept; split the task before saving its attempt.".into());
            }
            (Some(prompt), Some(reference))
        } else if plan.exercise_id.is_some() {
            // A legacy authored session without its snapshot has unknown question semantics.
            (None, None)
        } else {
            let revision = self
                .library
                .revision(session.revision)
                .ok_or("The original practice revision is unavailable")?;
            let (prompt, reference) = match session.skill {
                Skill::Reading => (
                    "Explain the meaning of the original Russian passage in your own words.",
                    &revision.passage.translation,
                ),
                Skill::Listening => (
                    "Explain the meaning of the original reference recording in your own words.",
                    &revision.passage.translation,
                ),
                Skill::Writing => (
                    "Write Russian expressing the meaning supplied by the original practice revision.",
                    &revision.passage.russian,
                ),
                Skill::Speaking => (
                    "Speak Russian expressing the meaning supplied by the original practice revision. A transcript supports language-content feedback only.",
                    &revision.passage.russian,
                ),
            };
            // The complete source remains in the exact revision. Never truncate a long source
            // into an apparently complete answer merely to fit the text-teacher request budget.
            (
                Some(prompt.into()),
                (!reference.trim().is_empty() && reference.len() <= 8192)
                    .then(|| reference.clone()),
            )
        };
        Ok(Some(AttemptContext {
            attempt,
            task,
            prompt,
            reference_answer,
            target_features: ex
                .map(|e| e.target_ids.iter().cloned().collect())
                .unwrap_or_default(),
            novel_context: false,
            delayed_probe: plan.policy == SpacedReview.id()
                && SpacedReview
                    .project(
                        &self.library,
                        ReviewKey {
                            revision: session.revision,
                            skill: session.skill,
                        },
                        now_ms(),
                    )
                    .attempts
                    > 0,
            practice_duration_ms: None,
            effort: None,
            grammar_response: None,
            supersedes: BTreeSet::new(),
            content_pack: plan.snapshot.as_ref().map(|s| s.pack_id.clone()),
            content_revision: plan.snapshot.as_ref().map(|s| s.pack_revision.clone()),
            exercise_id: plan.exercise_id.clone(),
        }))
    }
    pub(crate) fn open_studio(&mut self, revision: Uuid) -> Result<()> {
        self.studio_revision = Some(revision);
        self.navigate(Page::Studio)
    }
    pub(crate) fn coach_judgment(
        &mut self,
        attempt: Uuid,
        dimension: JudgmentDimension,
        verdict: Verdict,
        note: String,
        confidence_percent: u8,
        resolve: bool,
    ) -> Result<()> {
        self.refresh()?;
        let coach = self.preferences.coach_name.trim();
        if coach.is_empty() {
            return Err("Enter the coach’s name so the judgment has a source.".into());
        }
        if self.library.attempt(attempt).is_none() {
            return Err("The attempt is unavailable.".into());
        }
        let supersedes = if resolve {
            self.library
                .judgments(attempt, dimension)
                .iter()
                .map(|(e, _)| e.id)
                .collect()
        } else {
            BTreeSet::new()
        };
        let event = Event::new(
            self.device,
            now_ms(),
            Fact::CoachJudgmentRecorded(CoachJudgment {
                attempt,
                dimension,
                verdict,
                confidence_percent,
                provenance: CoachProvenance {
                    coach: coach.into(),
                    method: "Coach entered in person".into(),
                    note,
                },
                supersedes,
            }),
        )
        .map_err(|e| e.to_string())?;
        self.repo.append(&[event])?;
        self.refresh()?;
        self.notice =
            "Coach feedback saved with its source. Earlier feedback remains in the history.".into();
        Ok(())
    }
}

/// These two authored task definitions elicit a specific auditory contrast.
/// A sound target appearing in ordinary meaning comprehension does not establish discrimination.
fn is_authored_sound_discrimination(exercise: &Exercise) -> bool {
    let target = match exercise.id.as_str() {
        "ru.exercise.naming.hear-soft" => "ru.target.hard_soft",
        "ru.exercise.naming.hear-stress" => "ru.target.stress_contrast",
        _ => return false,
    };
    exercise.requires_audio && exercise.target_ids.iter().any(|id| id == target)
}
