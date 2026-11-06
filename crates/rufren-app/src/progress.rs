//! Recent activity and qualified exact-task signals, never a proficiency score.
use crate::{
    model::{AppState, Page},
    repository::{Result, now_ms},
    teacher_revisit::{TeacherRevisitQueue, proven_teacher_task_origin},
};
use rufren_core::*;
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

const WEEK_MS: i64 = 7 * DAY_MS;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProgressTarget {
    Teacher(TeacherRevisitKey),
    SoundFocus { block: Uuid },
    Passage { revision: Uuid, skill: Skill },
}

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub(crate) struct DelayedSignals {
    pub self_reported: usize,
    pub ai_correct: usize,
    pub helped: usize,
    pub replayed: usize,
    pub unrated: usize,
    pub recent_restudy: usize,
    pub contested: usize,
    pub early: usize,
    pub another_look: usize,
}

#[derive(Clone)]
pub(crate) struct SkillActivity {
    pub skill: Skill,
    pub responses: usize,
    pub with_support: usize,
    pub replayed: usize,
    pub no_self_report: usize,
    pub next: Option<ProgressTarget>,
}

#[derive(Clone)]
pub(crate) struct RecentPractice {
    pub attempt_id: Uuid,
    pub occurred_at_ms: i64,
    pub skill: Skill,
    pub title: String,
    pub support: String,
    pub outcome: Option<String>,
    pub target: Option<ProgressTarget>,
    pub sound_trial: Option<Uuid>,
}

#[derive(Clone)]
pub(crate) struct FuturePractice {
    pub occurred_at_ms: i64,
    pub skill: Skill,
    pub title: String,
}

#[derive(Clone)]
pub(crate) struct SoundBlockHistory {
    pub block: Uuid,
    pub title: String,
    pub completed: usize,
    pub unfinished: usize,
    pub incomplete: usize,
}

#[derive(Clone)]
pub(crate) struct ProgressSnapshot {
    pub skills: Vec<SkillActivity>,
    pub recent: Vec<RecentPractice>,
    pub delayed: DelayedSignals,
    pub future_count: usize,
    pub future: Vec<FuturePractice>,
    pub recent_questions: usize,
    pub recent_scene_starts: usize,
    pub sound_blocks: Vec<SoundBlockHistory>,
}

fn safe_title(title: &str, fallback: &str) -> String {
    if title.trim().is_empty()
        || title
            .chars()
            .any(|c| ('\u{0400}'..='\u{052f}').contains(&c))
    {
        fallback.into()
    } else {
        title.to_owned()
    }
}

fn teacher_title(library: &Library, key: TeacherRevisitKey) -> String {
    let name = library
        .teacher_document(key.document_id)
        .map(|d| match &d.response.request {
            rufren_tutor::TeacherRequest::TeachLesson(r) => {
                safe_title(&r.title, "A saved Teacher lesson")
            }
            _ => "A saved Teacher follow-up".into(),
        })
        .unwrap_or_else(|| "A saved Teacher exercise".into());
    format!("{name} · exercise {}", key.practice_index + 1)
}

fn generic_target(library: &Library, revision: Uuid, skill: Skill) -> Option<ProgressTarget> {
    let saved = library.revision(revision)?;
    if library
        .events
        .get(&revision)
        .is_some_and(is_reserved_practice_reference)
        || saved.passage.translation.trim().is_empty()
        || library
            .heads()
            .get(&saved.passage.id)
            .is_none_or(|heads| heads.len() != 1 || heads[0] != revision)
    {
        return None;
    }
    Some(ProgressTarget::Passage { revision, skill })
}

#[derive(Clone)]
struct SoundProgress {
    trial: Uuid,
    block: Uuid,
    title: String,
    incomplete: bool,
    assisted: bool,
    intended_match: Option<bool>,
    mode: SoundTrialMode,
}
fn sound_progress(library: &Library) -> BTreeMap<Uuid, SoundProgress> {
    let mut result = BTreeMap::<Uuid, SoundProgress>::new();
    for trial in library.sound_trials() {
        let title = library
            .sound_prepared(trial.trial_id)
            .map(|(_, prepared)| safe_title(&prepared.contrast.title, "Sound Focus"))
            .unwrap_or_else(|| "Sound Focus".into());
        let attempts: BTreeSet<_> = library
            .sound_trial_completion(trial.trial_id)
            .into_iter()
            .map(|(_, completion)| completion.attempt)
            .collect();
        for attempt in attempts {
            result
                .entry(attempt)
                .and_modify(|old| old.incomplete = true)
                .or_insert(SoundProgress {
                    trial: trial.trial_id,
                    block: trial.block_id,
                    title: format!("Sound Focus · {title}"),
                    incomplete: trial.incomplete,
                    assisted: trial.assisted,
                    intended_match: trial.intended_match,
                    mode: trial.mode,
                });
        }
    }
    result
}

fn sound_blocks(library: &Library) -> Vec<SoundBlockHistory> {
    let mut blocks = BTreeMap::<Uuid, SoundBlockHistory>::new();
    for trial in library.sound_trials() {
        let title = library
            .sound_prepared(trial.trial_id)
            .map(|(_, prepared)| safe_title(&prepared.contrast.title, "Saved sound practice"))
            .unwrap_or_else(|| "Saved sound practice".into());
        let block = blocks.entry(trial.block_id).or_insert(SoundBlockHistory {
            block: trial.block_id,
            title,
            completed: 0,
            unfinished: 0,
            incomplete: 0,
        });
        if library.sound_trial_completion(trial.trial_id).is_empty() {
            block.unfinished += 1;
        } else if trial.incomplete {
            block.incomplete += 1;
        } else {
            block.completed += 1;
        }
    }
    blocks.into_values().collect()
}

fn evidence_label(evidence: TeacherRevisitEvidence) -> &'static str {
    match evidence {
        TeacherRevisitEvidence::GuidedSeed => "First lesson response",
        TeacherRevisitEvidence::UnassistedSelfReport => "You reported remembering after the gap",
        TeacherRevisitEvidence::AiCorrect => "Your AI teacher marked this delayed response correct",
        TeacherRevisitEvidence::ClearError => "You or your teacher asked for another look",
        TeacherRevisitEvidence::AnswerHelp => "Help was recorded for this return",
        TeacherRevisitEvidence::ReplayOnly => "This return included a replay",
        TeacherRevisitEvidence::Unrated => "No reported recall or clear positive teacher judgment",
        TeacherRevisitEvidence::ContestedAi => "Saved teacher judgments disagree",
        TeacherRevisitEvidence::RecentRestudy => {
            "Recent practice or restudy limits the delay evidence"
        }
    }
}

/// Reuse the cadence classifier over complete occurrence groups, with all opinions
/// currently available. A late opinion on an old attempt is not new weekly activity.
fn delayed_signals(
    queue: &TeacherRevisitQueue,
    now: i64,
    since: i64,
) -> (DelayedSignals, BTreeMap<Uuid, String>) {
    let mut result = DelayedSignals::default();
    let mut labels = BTreeMap::new();
    for entry in &queue.entries {
        let mut groups = BTreeMap::<i64, Vec<TeacherRevisitObservation>>::new();
        for observation in &entry.observations {
            if observation.occurred_at_ms <= now {
                groups
                    .entry(observation.occurred_at_ms)
                    .or_default()
                    .push(observation.clone());
            }
        }
        let recent_groups = groups.split_off(&since);
        let mut prefix: Vec<_> = groups.into_values().flatten().collect();
        let Ok(mut before) =
            TeacherRevisitPolicy.project(entry.key, entry.state.seed, &prefix, now)
        else {
            continue;
        };
        for (_time, group) in recent_groups {
            prefix.extend(group.iter().cloned());
            let Ok(after) = TeacherRevisitPolicy.project(entry.key, entry.state.seed, &prefix, now)
            else {
                break;
            };
            let early = after
                .ignored_early_returns
                .saturating_sub(before.ignored_early_returns);
            let label = if early > 0 {
                "Early practice; the planned gap was not completed"
            } else {
                evidence_label(after.last_evidence)
            };
            let label = if group.len() > 1 {
                format!("Same-time returns: {label}")
            } else {
                label.into()
            };
            for observation in &group {
                labels.insert(observation.attempt_id, label.clone());
            }
            let advanced = after.self_report_advances > before.self_report_advances
                || after.ai_correct_advances > before.ai_correct_advances;
            if advanced {
                // The core qualifies the gap/support/group. Both distinct
                // evidence channels may describe that same qualified step.
                result.self_reported += usize::from(
                    group
                        .iter()
                        .any(|o| o.self_rating == SelfRating::Recalled && o.cues.is_empty()),
                );
                result.ai_correct += usize::from(group.iter().any(|o| {
                    o.cues.is_empty()
                        && o.ai_judgments
                            .iter()
                            .any(|j| j.recorded_at_ms <= now && j.verdict == Verdict::Correct)
                }));
            }
            result.helped += after
                .answer_help_returns
                .saturating_sub(before.answer_help_returns);
            result.replayed += after
                .replay_only_returns
                .saturating_sub(before.replay_only_returns);
            result.unrated += after.unrated_returns.saturating_sub(before.unrated_returns);
            result.recent_restudy += after
                .recent_restudy_returns
                .saturating_sub(before.recent_restudy_returns);
            result.contested += after
                .contested_returns
                .saturating_sub(before.contested_returns);
            result.another_look += after
                .clear_error_returns
                .saturating_sub(before.clear_error_returns);
            result.early += early;
            before = after;
        }
    }
    (result, labels)
}

fn project(library: &Library, queue: &TeacherRevisitQueue, now: i64) -> ProgressSnapshot {
    let since = now.saturating_sub(WEEK_MS);
    let (delayed, outcomes) = delayed_signals(queue, now, since);
    let mut skills: Vec<_> = Skill::ALL
        .into_iter()
        .map(|skill| SkillActivity {
            skill,
            responses: 0,
            with_support: 0,
            replayed: 0,
            no_self_report: 0,
            next: None,
        })
        .collect();
    let mut derived = BTreeMap::<Uuid, Vec<Cue>>::new();
    for support in queue
        .entries
        .iter()
        .flat_map(|entry| &entry.derived_support)
    {
        derived
            .entry(support.attempt_id)
            .or_default()
            .push(support.cue);
    }
    let queue_keys: BTreeSet<_> = queue.entries.iter().map(|entry| entry.key).collect();
    let mut recent = Vec::new();
    let mut future = Vec::new();
    let sounds = sound_progress(library);
    let mut attempts: Vec<_> = library.attempts().collect();
    attempts.sort_by_key(|(event, _)| (std::cmp::Reverse(event.recorded_at_ms), event.id));
    for (event, attempt) in attempts {
        let contexts = library.contexts(event.id);
        let origin = if attempt.policy.starts_with("teacher-") && contexts.len() == 1 {
            proven_teacher_task_origin(library, event.id, contexts[0].0.id)
        } else {
            None
        };
        let sound = sounds.get(&event.id);
        let sound_reserved = sound.is_some()
            || attempt.policy.starts_with("sound-focus")
            || library
                .events
                .get(&attempt.revision)
                .is_some_and(is_sound_focus_reference);
        let sound_complete = sound.is_some_and(|sound| !sound.incomplete);
        let title = sound
            .map(|sound| sound.title.clone())
            .or_else(|| sound_reserved.then(|| "Sound Focus · saved response".into()))
            .unwrap_or_else(|| {
                origin
                    .as_ref()
                    .map(|o| teacher_title(library, o.key))
                    .unwrap_or_else(|| {
                        library
                            .revision(attempt.revision)
                            .map(|r| safe_title(&r.passage.title, "A saved passage"))
                            .unwrap_or_else(|| "Saved practice".into())
                    })
            });
        if event.recorded_at_ms > now {
            future.push(FuturePractice {
                occurred_at_ms: event.recorded_at_ms,
                skill: attempt.skill,
                title,
            });
            continue;
        }
        let target = if sound_reserved {
            sound
                .filter(|sound| !sound.incomplete)
                .map(|sound| ProgressTarget::SoundFocus { block: sound.block })
        } else {
            origin
                .as_ref()
                .filter(|o| queue_keys.contains(&o.key))
                .map(|o| ProgressTarget::Teacher(o.key))
                .or_else(|| {
                    if attempt.policy.starts_with("teacher-") {
                        None
                    } else {
                        generic_target(library, attempt.revision, attempt.skill)
                    }
                })
        };
        let activity = skills
            .iter_mut()
            .find(|s| s.skill == attempt.skill)
            .unwrap();
        if activity.next.is_none() {
            activity.next = target;
        }
        if event.recorded_at_ms < since {
            continue;
        }
        let mut cues = attempt.cues.clone();
        let later_support = derived.get(&event.id);
        for cue in later_support.into_iter().flatten() {
            if !cues.contains(cue) {
                cues.push(*cue);
            }
        }
        let supported = !cues.is_empty()
            || attempt.self_rating == SelfRating::WithHelp
            || sound.is_some_and(|sound| sound.assisted);
        if !sound_reserved || sound_complete {
            activity.responses += 1;
            activity.with_support += usize::from(supported);
            activity.replayed += usize::from(cues.contains(&Cue::Replay));
            activity.no_self_report += usize::from(attempt.self_rating == SelfRating::NotReported);
        }
        let support = if sound_reserved && !sound_complete {
            "Incomplete sound trial history; not counted as completed practice".into()
        } else if later_support.is_some() {
            "Later recorded support also applies".into()
        } else if cues.contains(&Cue::Replay) && cues.iter().all(|c| *c == Cue::Replay) {
            "Reference replay recorded".into()
        } else if supported {
            "Help recorded".into()
        } else {
            "No help recorded on this response".into()
        };
        let outcome = if sound_reserved {
            sound
                .filter(|sound| !sound.incomplete)
                .map(|sound| match sound.mode {
                    SoundTrialMode::Listening => {
                        if sound.intended_match == Some(true) {
                            "Choice matched the intended meaning; no phonetic grade".into()
                        } else {
                            "Choice differed from the intended meaning; no phonetic grade".into()
                        }
                    }
                    SoundTrialMode::Speaking => {
                        "Spoken response saved for the chosen meaning; no pronunciation grade"
                            .into()
                    }
                })
        } else {
            outcomes.get(&event.id).cloned().or_else(|| {
                origin
                    .as_ref()
                    .filter(|o| !o.review)
                    .map(|_| "First lesson response".into())
            })
        };
        recent.push(RecentPractice {
            attempt_id: event.id,
            occurred_at_ms: event.recorded_at_ms,
            skill: attempt.skill,
            title,
            support,
            outcome,
            target,
            sound_trial: sound.map(|sound| sound.trial),
        });
    }
    // A due exact task is a useful next action; otherwise preserve a valid saved
    // task. We do not enroll unpracticed library material as a new obligation here.
    for activity in &mut skills {
        if let Some(entry) = queue
            .entries
            .iter()
            .find(|e| e.skill == activity.skill && e.due)
        {
            activity.next = Some(ProgressTarget::Teacher(entry.key));
        }
    }
    recent.sort_by_key(|row| (std::cmp::Reverse(row.occurred_at_ms), row.attempt_id));
    future.sort_by_key(|row| row.occurred_at_ms);
    let future_count = future.len();
    future.truncate(8);
    let mut recent_questions = 0;
    let mut recent_scene_starts = 0;
    for event in library
        .events
        .values()
        .filter(|e| e.recorded_at_ms >= since && e.recorded_at_ms <= now)
    {
        if let Fact::TeacherDialogueUserRecorded(request) = &event.fact {
            if request.action == rufren_tutor::DialogueAction::StartScene {
                recent_scene_starts += 1;
            } else {
                recent_questions += 1;
            }
        }
    }
    ProgressSnapshot {
        skills,
        recent,
        delayed,
        future_count,
        future,
        recent_questions,
        recent_scene_starts,
        sound_blocks: sound_blocks(library),
    }
}

impl AppState {
    pub(crate) fn progress_snapshot(&self, now: i64) -> ProgressSnapshot {
        project(&self.library, &self.teacher_revisits(now), now)
    }

    pub(crate) fn open_progress_target(&mut self, target: ProgressTarget) -> Result<()> {
        self.refresh()?;
        match target {
            ProgressTarget::Teacher(key) => {
                self.start_teacher_revisit(key, now_ms())?;
                self.navigate(Page::Practice)
            }
            ProgressTarget::SoundFocus { block } => self.open_sound_focus_history(block),
            ProgressTarget::Passage { revision, skill } => {
                if generic_target(&self.library, revision, skill).is_none() {
                    return Err("That passage has changed or needs its exact reference. Your saved practice is kept; open the library to choose a current version.".into());
                }
                self.start(revision, skill)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture_library() -> (Library, Uuid) {
        let event = Event::new(
            Uuid::new_v4(),
            0,
            Fact::PassageRevised(Revision {
                passage: Passage {
                    id: Uuid::new_v4(),
                    title: "Synthetic passage".into(),
                    russian: "Я читаю.".into(),
                    translation: "I am reading.".into(),
                    source: "Synthetic test".into(),
                    coach_reviewed: false,
                },
                parents: Default::default(),
            }),
        )
        .unwrap();
        let revision = event.id;
        let mut library = Library::default();
        library.merge(&[event]).unwrap();
        (library, revision)
    }
    fn attempt(library: &mut Library, revision: Uuid, time: i64, cues: Vec<Cue>) -> Uuid {
        let event = Event::new(
            Uuid::new_v4(),
            time,
            Fact::AttemptRecorded(Attempt {
                session: Uuid::new_v4(),
                revision,
                skill: Skill::Writing,
                response: "Я читаю.".into(),
                cues,
                self_rating: SelfRating::NotReported,
                policy: "synthetic-free-practice".into(),
            }),
        )
        .unwrap();
        let id = event.id;
        library.merge(&[event]).unwrap();
        id
    }
    #[test]
    fn reserved_sound_reference_without_companions_never_becomes_generic_practice() {
        let definition = rufren_content::sound_contrasts()
            .into_iter()
            .next()
            .unwrap();
        let contrast: SoundContrastSnapshot =
            serde_json::from_value(serde_json::to_value(definition).unwrap()).unwrap();
        let reference = sound_reference_event(&contrast, &contrast.members[0].id).unwrap();
        let revision = reference.id;
        let mut library = Library::default();
        library.merge(&[reference]).unwrap();
        let id = attempt(&mut library, revision, DAY_MS, vec![]);
        let snapshot = project(&library, &TeacherRevisitQueue::default(), 2 * DAY_MS);
        assert!(generic_target(&library, revision, Skill::Writing).is_none());
        assert_eq!(
            snapshot
                .skills
                .iter()
                .map(|skill| skill.responses)
                .sum::<usize>(),
            0
        );
        let raw = snapshot
            .recent
            .iter()
            .find(|row| row.attempt_id == id)
            .unwrap();
        assert!(raw.target.is_none());
        assert!(raw.outcome.is_none());
        assert!(raw.support.contains("Incomplete sound trial history"));
        assert!(snapshot.sound_blocks.is_empty());
        assert_eq!(library.attempts().count(), 1);
    }
    #[test]
    fn progress_excludes_future_and_old_activity_without_losing_inspection() {
        let now = 20 * DAY_MS;
        let (mut library, revision) = fixture_library();
        attempt(&mut library, revision, now - DAY_MS, vec![]);
        attempt(&mut library, revision, now - 8 * DAY_MS, vec![]);
        attempt(&mut library, revision, now + DAY_MS, vec![]);
        let snapshot = project(&library, &TeacherRevisitQueue::default(), now);
        assert_eq!(
            snapshot
                .skills
                .iter()
                .find(|s| s.skill == Skill::Writing)
                .unwrap()
                .responses,
            1
        );
        assert_eq!(snapshot.recent.len(), 1);
        assert_eq!(snapshot.future_count, 1);
        assert_eq!(snapshot.future[0].occurred_at_ms, now + DAY_MS);
        assert_eq!(library.attempts().count(), 3);
    }
    #[test]
    fn progress_applies_derived_support_without_rewriting_the_attempt() {
        let now = 20 * DAY_MS;
        let (mut library, revision) = fixture_library();
        let id = attempt(&mut library, revision, now - DAY_MS, vec![]);
        let key = TeacherRevisitKey {
            document_id: Uuid::new_v4(),
            practice_index: 0,
        };
        let seed = TeacherRevisitSeed {
            attempt_id: Uuid::new_v4(),
            context_id: Uuid::new_v4(),
            occurred_at_ms: now - 3 * DAY_MS,
        };
        let state = TeacherRevisitPolicy.project(key, seed, &[], now).unwrap();
        let queue = TeacherRevisitQueue {
            entries: vec![crate::teacher_revisit::TeacherRevisitEntry {
                key,
                title: "Synthetic lesson".into(),
                prompt: "Synthetic question".into(),
                skill: Skill::Writing,
                due: true,
                state,
                observations: vec![],
                derived_support: vec![crate::teacher_revisit::TeacherRevisitDerivedSupport {
                    attempt_id: id,
                    cue: Cue::ReferenceMaterial,
                    presentation_fact_ids: vec![Uuid::new_v4()],
                    omitted_source_count: 0,
                }],
            }],
            diagnostics: vec![],
        };
        let snapshot = project(&library, &queue, now);
        assert_eq!(
            snapshot
                .skills
                .iter()
                .find(|s| s.skill == Skill::Writing)
                .unwrap()
                .with_support,
            1
        );
        assert_eq!(
            snapshot.recent[0].support,
            "Later recorded support also applies"
        );
        assert!(library.attempt(id).unwrap().cues.is_empty());
    }

    fn queue_for(observations: Vec<TeacherRevisitObservation>, now: i64) -> TeacherRevisitQueue {
        let key = TeacherRevisitKey {
            document_id: Uuid::new_v4(),
            practice_index: 0,
        };
        let seed = TeacherRevisitSeed {
            attempt_id: Uuid::new_v4(),
            context_id: Uuid::new_v4(),
            occurred_at_ms: 0,
        };
        let state = TeacherRevisitPolicy
            .project(key, seed, &observations, now)
            .unwrap();
        TeacherRevisitQueue {
            entries: vec![crate::teacher_revisit::TeacherRevisitEntry {
                key,
                title: "Synthetic lesson".into(),
                prompt: "Synthetic question".into(),
                skill: Skill::Writing,
                due: state.is_due(now),
                state,
                observations,
                derived_support: vec![],
            }],
            diagnostics: vec![],
        }
    }
    fn observed(
        time: i64,
        rating: SelfRating,
        correct_at: Option<i64>,
    ) -> TeacherRevisitObservation {
        TeacherRevisitObservation {
            attempt_id: Uuid::new_v4(),
            context_id: Uuid::new_v4(),
            started_at_ms: time,
            occurred_at_ms: time,
            self_rating: rating,
            cues: vec![],
            prior_target_exposure_at_ms: None,
            ai_judgments: correct_at
                .into_iter()
                .map(|recorded_at_ms| TeacherRevisitAiJudgment {
                    fact_id: Uuid::new_v4(),
                    recorded_at_ms,
                    verdict: Verdict::Correct,
                })
                .collect(),
        }
    }
    #[test]
    fn progress_delayed_channels_remain_distinct_and_may_share_a_return() {
        let now = 5 * DAY_MS;
        let queue = queue_for(
            vec![observed(DAY_MS, SelfRating::Recalled, Some(DAY_MS))],
            now,
        );
        let (signals, _) = delayed_signals(&queue, now, now - WEEK_MS);
        assert_eq!(signals.self_reported, 1);
        assert_eq!(signals.ai_correct, 1);
    }
    #[test]
    fn progress_late_opinion_does_not_turn_an_old_attempt_into_recent_activity() {
        let now = 20 * DAY_MS;
        let queue = queue_for(
            vec![observed(DAY_MS, SelfRating::NotReported, Some(now - 1))],
            now,
        );
        assert_eq!(queue.entries[0].state.ai_correct_advances, 1);
        let (signals, labels) = delayed_signals(&queue, now, now - WEEK_MS);
        assert_eq!(signals.ai_correct, 0);
        assert!(labels.is_empty());
    }
    #[test]
    fn progress_future_opinion_and_co_timed_help_cannot_be_positive_evidence() {
        let now = 5 * DAY_MS;
        let queue = queue_for(
            vec![observed(DAY_MS, SelfRating::NotReported, Some(now + 1))],
            now,
        );
        let (signals, _) = delayed_signals(&queue, now, now - WEEK_MS);
        assert_eq!(signals.ai_correct, 0);
        assert_eq!(signals.unrated, 1);
        let mut help = observed(DAY_MS, SelfRating::NotReported, None);
        help.cues.push(Cue::ReferenceMaterial);
        let queue = queue_for(
            vec![observed(DAY_MS, SelfRating::Recalled, Some(DAY_MS)), help],
            now,
        );
        let (signals, labels) = delayed_signals(&queue, now, now - WEEK_MS);
        assert_eq!((signals.ai_correct, signals.self_reported), (0, 0));
        assert_eq!(signals.helped, 1);
        assert!(
            labels
                .values()
                .all(|label| label.starts_with("Same-time returns:"))
        );
    }
    #[test]
    fn progress_generic_link_rejects_reserved_teacher_reference_and_old_revision() {
        let (mut library, revision) = fixture_library();
        assert!(generic_target(&library, revision, Skill::Writing).is_some());
        let old = library.revision(revision).unwrap().clone();
        let mut replacement = old.clone();
        replacement.parents.insert(revision);
        let event = Event::new(Uuid::new_v4(), DAY_MS, Fact::PassageRevised(replacement)).unwrap();
        library.merge(&[event]).unwrap();
        assert!(generic_target(&library, revision, Skill::Writing).is_none());
        let (mut library, revision) = fixture_library();
        let Fact::PassageRevised(revision_fact) =
            &mut library.events.get_mut(&revision).unwrap().fact
        else {
            unreachable!()
        };
        revision_fact.passage.source = "AI teacher exercise · synthetic reserved source".into();
        assert!(generic_target(&library, revision, Skill::Writing).is_none());
    }
}
