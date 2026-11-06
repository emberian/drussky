//! Exact archived Teacher tasks revisited in fresh sessions; local drafts are never sync facts.
use crate::{
    model::AppState,
    repository::{Result, now_ms},
    teacher_practice::TeacherPracticeDraft,
};
use rufren_core::*;
use rufren_tutor::{TeacherRequest, TeacherResponse, TeacherSkill};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

pub(crate) const TEACHER_REVISIT_POLICY: &str = TEACHER_REVISIT_POLICY_ID;
const PROOF_MARKER: &str = "\nRufren teacher revisit v1: ";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct TeacherRevisitDraft {
    pub document_id: Uuid,
    pub practice_index: usize,
    pub source_attempt_id: Uuid,
    pub source_context_id: Uuid,
    pub self_rating: SelfRating,
    pub cues: Vec<Cue>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct TeacherRevisitProof {
    pub format_version: u32,
    pub document_id: Uuid,
    pub practice_index: usize,
    pub source_attempt_id: Uuid,
    pub source_context_id: Uuid,
    pub started_at_ms: i64,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ActiveRevisit {
    document_id: Uuid,
    practice_index: usize,
    draft_id: Uuid,
}
#[derive(Clone)]
pub(crate) struct TeacherRevisitEntry {
    pub key: TeacherRevisitKey,
    pub title: String,
    pub prompt: String,
    pub skill: Skill,
    pub state: TeacherRevisitState,
    pub due: bool,
    /// Derived from exact presentation facts; these are not rewritten learner-reported cues.
    pub derived_support: Vec<TeacherRevisitDerivedSupport>,
}
#[derive(Clone)]
pub(crate) struct TeacherRevisitDerivedSupport {
    pub attempt_id: Uuid,
    pub cue: Cue,
    pub presentation_fact_ids: Vec<Uuid>,
    pub omitted_source_count: usize,
}
#[derive(Clone, Default)]
pub(crate) struct TeacherRevisitQueue {
    pub entries: Vec<TeacherRevisitEntry>,
    pub diagnostics: Vec<String>,
}
#[derive(Clone)]
pub(crate) struct ProvenTeacherTaskOrigin {
    pub key: TeacherRevisitKey,
    pub lesson_id: Option<String>,
    pub review: bool,
}
fn unique_cues(cues: &[Cue]) -> bool {
    !cues
        .iter()
        .enumerate()
        .any(|(i, cue)| cues[i + 1..].contains(cue))
}
fn draft_key(document: Uuid, index: usize) -> String {
    format!("teacher_revisit:{document}:{index}")
}
pub(crate) fn teacher_revisit_storage_key(draft: &TeacherPracticeDraft) -> Result<String> {
    let review = draft
        .review
        .as_ref()
        .ok_or("This is not a scheduled Teacher revisit.")?;
    if review.document_id.is_nil()
        || review.practice_index >= 4
        || review.source_attempt_id.is_nil()
        || review.source_context_id.is_nil()
        || draft.source != review.document_id.to_string()
        || draft.index != review.practice_index
        || draft.id.is_nil()
        || draft.started_at_ms < 0
        || draft
            .saved_attempt
            .is_some_and(|id| id != Uuid::new_v5(&draft.id, b"attempt"))
        || review.cues.len() > 32
        || !unique_cues(&review.cues)
    {
        return Err(
            "This revisit has invalid saved task/session bindings; its earlier draft is kept."
                .into(),
        );
    }
    Ok(draft_key(review.document_id, review.practice_index))
}
pub(crate) fn teacher_revisit_exercise_id(draft: &TeacherPracticeDraft) -> Result<String> {
    teacher_revisit_storage_key(draft)?;
    let r = draft.review.as_ref().unwrap();
    Ok(format!(
        "teacher:revisit:v1:{}:{}",
        r.document_id, r.practice_index
    ))
}
pub(crate) fn teacher_revisit_context_reference(
    base: &str,
    draft: &TeacherPracticeDraft,
) -> Result<String> {
    teacher_revisit_storage_key(draft)?;
    let r = draft.review.as_ref().unwrap();
    let proof = TeacherRevisitProof {
        format_version: 1,
        document_id: r.document_id,
        practice_index: r.practice_index,
        source_attempt_id: r.source_attempt_id,
        source_context_id: r.source_context_id,
        started_at_ms: draft.started_at_ms,
    };
    let value = format!(
        "{base}{PROOF_MARKER}{}",
        serde_json::to_string(&proof).map_err(|e| e.to_string())?
    );
    if value.len() > 8192 {
        return Err("This revisit reference exceeds its saved task budget.".into());
    }
    Ok(value)
}
fn reference_without_audio(context: &AttemptContext) -> Option<&str> {
    crate::teacher_audio_practice::audio_reference_base(context)
        .or_else(|| context.reference_answer.as_deref())
}
pub(crate) fn teacher_revisit_proof(
    context: &AttemptContext,
) -> Option<(&str, TeacherRevisitProof)> {
    let (base, json) = reference_without_audio(context)?.rsplit_once(PROOF_MARKER)?;
    let proof: TeacherRevisitProof = serde_json::from_str(json).ok()?;
    (proof.format_version == 1
        && proof.practice_index < 4
        && !proof.document_id.is_nil()
        && !proof.source_attempt_id.is_nil()
        && !proof.source_context_id.is_nil()
        && proof.started_at_ms >= 0)
        .then_some((base, proof))
}
fn task_skill(answer: &TeacherResponse, index: usize) -> Option<Skill> {
    Some(match answer.content.practice.get(index)?.skill.as_ref()? {
        TeacherSkill::Reading => Skill::Reading,
        TeacherSkill::Writing => Skill::Writing,
        TeacherSkill::Listening => Skill::Listening,
        TeacherSkill::Speaking => Skill::Speaking,
    })
}
fn task_kind(skill: Skill) -> TaskKind {
    match skill {
        Skill::Reading | Skill::Listening => TaskKind::MeaningRecall,
        Skill::Writing => TaskKind::Composition,
        Skill::Speaking => TaskKind::FormRecall,
    }
}
fn common_exact_task(
    library: &Library,
    key: TeacherRevisitKey,
    attempt_id: Uuid,
    context_id: Uuid,
) -> Option<()> {
    let document = library.teacher_document(key.document_id)?;
    let (_, revision, fact) =
        teacher_practice_reference_fact(&document.response, key.practice_index).ok()?;
    let (prompt, _) =
        teacher_practice_prompt_reference(&document.response, key.practice_index).ok()?;
    let event = library.events.get(&revision)?;
    let attempt = library.attempt(attempt_id)?;
    let Fact::AttemptContextRecorded(context) = &library.events.get(&context_id)?.fact else {
        return None;
    };
    (event.version == 2
        && event.device == key.document_id
        && event.recorded_at_ms == 0
        && event.fact == fact
        && attempt_id == Uuid::new_v5(&attempt.session, b"attempt")
        && context_id == Uuid::new_v5(&attempt.session, b"context")
        && context.attempt == attempt_id
        && attempt.revision == revision
        && Some(attempt.skill) == task_skill(&document.response, key.practice_index)
        && context.task == task_kind(attempt.skill)
        && context.prompt.as_deref() == Some(prompt.as_str())
        && context.content_pack.is_none()
        && context.content_revision.is_none()
        && context.target_features.is_empty()
        && !context.novel_context
        && !context.delayed_probe
        && context.practice_duration_ms.is_none()
        && context.effort.is_none()
        && context.grammar_response.is_none()
        && context.supersedes.is_empty())
    .then_some(())
}
fn exact_guided_origin(
    library: &Library,
    key: TeacherRevisitKey,
    attempt_id: Uuid,
    context_id: Uuid,
) -> Option<()> {
    common_exact_task(library, key, attempt_id, context_id)?;
    let document = library.teacher_document(key.document_id)?;
    let a = library.attempt(attempt_id)?;
    let Fact::AttemptContextRecorded(c) = &library.events.get(&context_id)?.fact else {
        return None;
    };
    let (_, reference) =
        teacher_practice_prompt_reference(&document.response, key.practice_index).ok()?;
    let passage = library.revision(a.revision)?.passage.id;
    let legacy = format!("teacher:{passage}");
    let qualified = match &document.response.request {
        TeacherRequest::TeachLesson(lesson) => Some(format!(
            "teacher:v2:{}:{}:{}",
            lesson.lesson_id, key.document_id, key.practice_index
        )),
        _ => None,
    };
    let identity = c.exercise_id.as_deref()?;
    if identity != legacy && qualified.as_deref() != Some(identity) {
        return None;
    }
    let oral = matches!(a.skill, Skill::Listening | Skill::Speaking);
    (a.self_rating == SelfRating::NotReported
        && if oral {
            a.policy == crate::teacher_audio_practice::TEACHER_AUDIO_POLICY
                && reference_without_audio(c) == Some(reference.as_str())
                && crate::teacher_audio_practice::valid_submitted_audio(library, attempt_id, c)
        } else {
            a.policy == "teacher-guided-text-v1"
                && a.cues == [Cue::ReferenceMaterial]
                && c.reference_answer.as_deref() == Some(reference.as_str())
        })
    .then_some(())
}
pub(crate) fn saved_teacher_revisit_origin(
    library: &Library,
    attempt_id: Uuid,
    context_id: Uuid,
) -> Option<ProvenTeacherTaskOrigin> {
    let a = library.attempt(attempt_id)?;
    let event = library.events.get(&attempt_id)?;
    let Fact::AttemptContextRecorded(c) = &library.events.get(&context_id)?.fact else {
        return None;
    };
    if a.policy != TEACHER_REVISIT_POLICY {
        return None;
    }
    let (base, proof) = teacher_revisit_proof(c)?;
    let key = TeacherRevisitKey {
        document_id: proof.document_id,
        practice_index: proof.practice_index,
    };
    common_exact_task(library, key, attempt_id, context_id)?;
    exact_guided_origin(
        library,
        key,
        proof.source_attempt_id,
        proof.source_context_id,
    )?;
    let source = library.events.get(&proof.source_attempt_id)?;
    let document = library.teacher_document(key.document_id)?;
    let (_, reference) =
        teacher_practice_prompt_reference(&document.response, key.practice_index).ok()?;
    if base != reference
        || source.recorded_at_ms > proof.started_at_ms
        || event.recorded_at_ms < proof.started_at_ms
        || c.exercise_id.as_deref()
            != Some(
                format!(
                    "teacher:revisit:v1:{}:{}",
                    key.document_id, key.practice_index
                )
                .as_str(),
            )
        || a.cues.len() > 32
        || !unique_cues(&a.cues)
        || (matches!(a.skill, Skill::Listening | Skill::Speaking)
            && !crate::teacher_audio_practice::valid_submitted_audio(library, attempt_id, c))
    {
        return None;
    }
    Some(ProvenTeacherTaskOrigin {
        key,
        review: true,
        lesson_id: match &document.response.request {
            TeacherRequest::TeachLesson(r) => Some(r.lesson_id.clone()),
            _ => None,
        },
    })
}
pub(crate) fn proven_teacher_task_origin(
    library: &Library,
    attempt_id: Uuid,
    context_id: Uuid,
) -> Option<ProvenTeacherTaskOrigin> {
    if let Some(origin) = saved_teacher_revisit_origin(library, attempt_id, context_id) {
        return Some(origin);
    }
    for summary in library.teacher_documents() {
        let document = library.teacher_document(summary.document_id)?;
        for practice_index in 0..document.response.content.practice.len() {
            let key = TeacherRevisitKey {
                document_id: summary.document_id,
                practice_index,
            };
            if exact_guided_origin(library, key, attempt_id, context_id).is_some() {
                return Some(ProvenTeacherTaskOrigin {
                    key,
                    review: false,
                    lesson_id: match &document.response.request {
                        TeacherRequest::TeachLesson(r) => Some(r.lesson_id.clone()),
                        _ => None,
                    },
                });
            }
        }
    }
    None
}
fn target_matches(library: &Library, key: TeacherRevisitKey, text: &str) -> bool {
    let Some(document) = library.teacher_document(key.document_id) else {
        return false;
    };
    let Some(exercise) = document.response.content.practice.get(key.practice_index) else {
        return false;
    };
    [
        exercise.reference_russian.as_deref(),
        exercise.expected_response.as_deref(),
    ]
    .into_iter()
    .flatten()
    .any(|target| !target.trim().is_empty() && stimulus_contains(text, target))
}
fn in_session_support(
    library: &Library,
    key: TeacherRevisitKey,
    session: Uuid,
    started: i64,
    completed: i64,
) -> Vec<(Cue, Uuid)> {
    library
        .presentations()
        .filter_map(|(event, presentation)| {
            if event.recorded_at_ms < started
                || event.recorded_at_ms > completed
                || !target_matches(library, key, &presentation.russian)
            {
                return None;
            }
            let cue = if presentation.session == Some(session) {
                match presentation.purpose {
                    // First required listening input is TaskInput; later plays/model audio
                    // are durable Audio facts even if the accompanying draft save failed.
                    PresentationPurpose::TaskInput => return None,
                    PresentationPurpose::Audio => Cue::Replay,
                    _ => Cue::ReferenceMaterial,
                }
            } else {
                Cue::ReferenceMaterial
            };
            Some((cue, event.id))
        })
        .collect()
}
fn prior_target_exposure(
    library: &Library,
    key: TeacherRevisitKey,
    session: Uuid,
    started: i64,
) -> Option<i64> {
    library
        .presentations()
        .filter(|(event, p)| {
            event.recorded_at_ms <= started
                && p.session != Some(session)
                && target_matches(library, key, &p.russian)
        })
        .map(|(event, _)| event.recorded_at_ms)
        .max()
}
fn as_of_ai(
    library: &Library,
    attempt_id: Uuid,
    context_id: Uuid,
    now: i64,
) -> Vec<TeacherRevisitAiJudgment> {
    let Some(a) = library.attempt(attempt_id) else {
        return vec![];
    };
    let Some(Fact::AttemptContextRecorded(c)) = library.events.get(&context_id).map(|e| &e.fact)
    else {
        return vec![];
    };
    let expected_dimension = if matches!(a.skill, Skill::Reading | Skill::Listening) {
        JudgmentDimension::Comprehension
    } else {
        JudgmentDimension::Grammar
    };
    let eligible: Vec<_> = library
        .events
        .values()
        .filter_map(|event| {
            let Fact::AiAssessmentRecorded(j) = &event.fact else {
                return None;
            };
            let words_ok = if let Some(id) = j.confirmed_transcript {
                let confirm = library.transcript_confirmation(id)?;
                let transcript = library.speech_transcription(confirm.transcription)?;
                a.skill == Skill::Speaking
                    && j.evidence == AiTextEvidence::Transcript
                    && transcript.attempt == attempt_id
                    && transcript.revision == a.revision
                    && confirm.text == j.response
            } else {
                a.skill != Skill::Speaking
                    && j.evidence == AiTextEvidence::WrittenText
                    && j.response == a.response
            };
            (event.recorded_at_ms <= now
                && j.attempt == attempt_id
                && j.revision == a.revision
                && j.context == Some(context_id)
                && j.skill == a.skill
                && j.task == c.task
                && j.dimension == expected_dimension
                && words_ok)
                .then_some((event, j))
        })
        .collect();
    let superseded: BTreeSet<_> = eligible
        .iter()
        .flat_map(|(_, j)| j.supersedes.iter().copied())
        .collect();
    eligible
        .into_iter()
        .filter(|(e, _)| !superseded.contains(&e.id))
        .map(|(e, j)| TeacherRevisitAiJudgment {
            fact_id: e.id,
            recorded_at_ms: e.recorded_at_ms,
            verdict: j.verdict,
        })
        .collect()
}

impl AppState {
    pub(crate) fn teacher_revisits(&self, now: i64) -> TeacherRevisitQueue {
        let mut queue = TeacherRevisitQueue::default();
        let mut seeds = BTreeMap::<TeacherRevisitKey, Vec<TeacherRevisitSeed>>::new();
        let mut observations = BTreeMap::<TeacherRevisitKey, Vec<TeacherRevisitObservation>>::new();
        let mut derived_support =
            BTreeMap::<TeacherRevisitKey, Vec<TeacherRevisitDerivedSupport>>::new();
        for (event, a) in self.library.attempts() {
            if !a.policy.starts_with("teacher-") {
                continue;
            }
            let contexts = self.library.contexts(event.id);
            let origin = if contexts.len() == 1 {
                proven_teacher_task_origin(&self.library, event.id, contexts[0].0.id)
            } else {
                None
            };
            let Some(origin) = origin else {
                if queue.diagnostics.len() < 8 {
                    queue.diagnostics.push("Some generated practice has missing, competing or mismatched archived task evidence and was kept outside scheduled revisits.".into());
                }
                continue;
            };
            if !origin.review {
                if event.recorded_at_ms > now {
                    if queue.diagnostics.len() < 8 {
                        queue.diagnostics.push(
                            "Future-dated guided practice is excluded until its recorded time."
                                .into(),
                        );
                    }
                    continue;
                }
                seeds
                    .entry(origin.key)
                    .or_default()
                    .push(TeacherRevisitSeed {
                        attempt_id: event.id,
                        context_id: contexts[0].0.id,
                        occurred_at_ms: event.recorded_at_ms,
                    });
            } else {
                let Some((_, proof)) = teacher_revisit_proof(contexts[0].1) else {
                    if queue.diagnostics.len() < 8 {
                        queue.diagnostics.push("A revisit is missing its exact retrieval-start evidence and was kept outside cadence.".into());
                    }
                    continue;
                };
                let source_ids = in_session_support(
                    &self.library,
                    origin.key,
                    a.session,
                    proof.started_at_ms,
                    event.recorded_at_ms,
                );
                let mut qualified_cues = a.cues.clone();
                for cue in [Cue::Replay, Cue::ReferenceMaterial] {
                    let cue_sources: Vec<_> = source_ids
                        .iter()
                        .filter(|(kind, _)| *kind == cue)
                        .map(|(_, id)| *id)
                        .collect();
                    if !cue_sources.is_empty() && !qualified_cues.contains(&cue) {
                        qualified_cues.push(cue);
                        let omitted_source_count = cue_sources.len().saturating_sub(16);
                        derived_support.entry(origin.key).or_default().push(
                            TeacherRevisitDerivedSupport {
                                attempt_id: event.id,
                                cue,
                                presentation_fact_ids: cue_sources.into_iter().take(16).collect(),
                                omitted_source_count,
                            },
                        );
                    }
                }
                observations
                    .entry(origin.key)
                    .or_default()
                    .push(TeacherRevisitObservation {
                        attempt_id: event.id,
                        context_id: contexts[0].0.id,
                        occurred_at_ms: event.recorded_at_ms,
                        started_at_ms: proof.started_at_ms,
                        self_rating: a.self_rating,
                        cues: qualified_cues,
                        prior_target_exposure_at_ms: prior_target_exposure(
                            &self.library,
                            origin.key,
                            a.session,
                            proof.started_at_ms,
                        ),
                        ai_judgments: as_of_ai(&self.library, event.id, contexts[0].0.id, now),
                    });
            }
        }
        for (key, mut candidates) in seeds {
            candidates.sort_by_key(|s| (s.occurred_at_ms, s.attempt_id));
            let seed = candidates[0];
            let Some(document) = self.library.teacher_document(key.document_id) else {
                continue;
            };
            match TeacherRevisitPolicy.project(
                key,
                seed,
                observations.get(&key).map_or(&[], Vec::as_slice),
                now,
            ) {
                Ok(state) => queue.entries.push(TeacherRevisitEntry {
                    key,
                    title: match &document.response.request {
                        TeacherRequest::TeachLesson(r) => r.title.clone(),
                        _ => "A saved teacher follow-up".into(),
                    },
                    prompt: document.response.content.practice[key.practice_index]
                        .prompt
                        .clone(),
                    skill: task_skill(&document.response, key.practice_index).unwrap(),
                    due: state.is_due(now),
                    derived_support: derived_support.remove(&key).unwrap_or_default(),
                    state,
                }),
                Err(_) => {
                    if queue.diagnostics.len() < 8 {
                        queue.diagnostics.push("A revisit's timing/evidence is inconsistent; its original facts are preserved for review.".into());
                    }
                }
            }
        }
        if queue
            .entries
            .iter()
            .any(|entry| !entry.derived_support.is_empty())
            && queue.diagnostics.len() < 8
        {
            queue.diagnostics.push("Recorded target exposure during a past return qualifies its planned gap conservatively. Its saved response and self-report are unchanged.".into());
        }
        queue
            .entries
            .sort_by_key(|e| (!e.due, e.state.due_at_ms, e.key));
        queue.diagnostics.sort();
        queue.diagnostics.dedup();
        queue
    }
    /// Re-read durable support and cross-window exposure before committing response facts.
    pub(crate) fn qualify_teacher_revisit_support(
        &mut self,
        answer: &TeacherResponse,
        draft: &mut TeacherPracticeDraft,
    ) -> Result<()> {
        if draft.review.is_none() {
            return Ok(());
        }
        self.refresh()?;
        *draft = self.merged_teacher_revisit_draft(draft)?;
        self.validate_teacher_revisit(answer, draft)?;
        if draft.saved_attempt.is_some() {
            return Ok(());
        }
        let key = TeacherRevisitKey {
            document_id: draft.review.as_ref().unwrap().document_id,
            practice_index: draft.index,
        };
        let evidence =
            in_session_support(&self.library, key, draft.id, draft.started_at_ms, now_ms());
        let mut changed = false;
        for (cue, _) in evidence {
            if !draft.review.as_ref().unwrap().cues.contains(&cue) {
                draft.review.as_mut().unwrap().cues.push(cue);
                if let Some(audio) = &mut draft.audio {
                    if !audio.cues.contains(&cue) {
                        audio.cues.push(cue);
                    }
                }
                changed = true;
            }
        }
        if changed {
            self.save_teacher_practice_draft(draft)?;
        }
        Ok(())
    }
    pub(crate) fn start_teacher_revisit(
        &mut self,
        key: TeacherRevisitKey,
        now: i64,
    ) -> Result<TeacherPracticeDraft> {
        self.refresh()?;
        let entry = self
            .teacher_revisits(now)
            .entries
            .into_iter()
            .find(|e| e.key == key)
            .ok_or("This exact completed teacher task is unavailable for a scheduled revisit.")?;
        let answer = self
            .library
            .teacher_document(key.document_id)
            .ok_or("Sync the original teacher document first.")?
            .response
            .clone();
        if let Some(saved) = self
            .repo
            .get::<TeacherPracticeDraft>(&draft_key(key.document_id, key.practice_index))?
        {
            if saved.saved_attempt.is_none() {
                self.validate_teacher_revisit(&answer, &saved)?;
                self.select_teacher_revisit(&saved)?;
                return Ok(saved);
            }
        }
        let draft = TeacherPracticeDraft {
            id: Uuid::new_v4(),
            source: key.document_id.to_string(),
            index: key.practice_index,
            response: String::new(),
            started_at_ms: now,
            saved_attempt: None,
            audio: None,
            review: Some(TeacherRevisitDraft {
                document_id: key.document_id,
                practice_index: key.practice_index,
                source_attempt_id: entry.state.seed.attempt_id,
                source_context_id: entry.state.seed.context_id,
                self_rating: SelfRating::NotReported,
                cues: vec![],
            }),
        };
        self.validate_teacher_revisit(&answer, &draft)?;
        self.repo
            .set(&teacher_revisit_storage_key(&draft)?, &draft)?;
        self.select_teacher_revisit(&draft)?;
        Ok(draft)
    }
    pub(crate) fn teacher_revisit_draft(
        &self,
        answer: &TeacherResponse,
        index: usize,
    ) -> Result<TeacherPracticeDraft> {
        let draft = self
            .repo
            .get::<TeacherPracticeDraft>(&draft_key(teacher_document_id(answer), index))?
            .ok_or("Open this scheduled revisit from your review list first.")?;
        self.validate_teacher_revisit(answer, &draft)?;
        self.merged_teacher_revisit_draft(&draft)
    }
    pub(crate) fn validate_teacher_revisit(
        &self,
        answer: &TeacherResponse,
        draft: &TeacherPracticeDraft,
    ) -> Result<()> {
        teacher_revisit_storage_key(draft)?;
        let r = draft.review.as_ref().unwrap();
        let document = self
            .library
            .teacher_document(r.document_id)
            .ok_or("Sync the exact original teacher document before revisiting it.")?;
        if document.response != *answer
            || r.document_id != teacher_document_id(answer)
            || exact_guided_origin(
                &self.library,
                TeacherRevisitKey {
                    document_id: r.document_id,
                    practice_index: r.practice_index,
                },
                r.source_attempt_id,
                r.source_context_id,
            )
            .is_none()
            || self
                .library
                .events
                .get(&r.source_attempt_id)
                .is_none_or(|e| e.recorded_at_ms > draft.started_at_ms)
        {
            return Err("This revisit no longer matches its exact archived task and original completed practice. Its draft is kept.".into());
        }
        Ok(())
    }
    pub(crate) fn merged_teacher_revisit_draft(
        &self,
        draft: &TeacherPracticeDraft,
    ) -> Result<TeacherPracticeDraft> {
        let key = teacher_revisit_storage_key(draft)?;
        let mut merged = draft.clone();
        if let Some(old) = self.repo.get::<TeacherPracticeDraft>(&key)? {
            if old.id != draft.id {
                return Err(
                    "This revisit was replaced by a fresh session. Its newer draft is kept.".into(),
                );
            }
            if old.saved_attempt.is_some() && old.saved_attempt != draft.saved_attempt {
                return Err("This revisit is already saved. Start another scheduled revisit for a fresh response.".into());
            }
            let old_review = old
                .review
                .as_ref()
                .ok_or("The saved revisit is missing its task binding.")?;
            let review = merged.review.as_mut().unwrap();
            if old_review.document_id != review.document_id
                || old_review.practice_index != review.practice_index
                || old_review.source_attempt_id != review.source_attempt_id
                || old_review.source_context_id != review.source_context_id
                || old.started_at_ms != draft.started_at_ms
            {
                return Err("This saved revisit has different source bindings.".into());
            }
            for cue in &old_review.cues {
                if !review.cues.contains(cue) {
                    review.cues.push(*cue);
                }
            }
        }
        if let Some(audio) = &mut merged.audio {
            let review = merged.review.as_mut().unwrap();
            for cue in &audio.cues {
                if !review.cues.contains(cue) {
                    review.cues.push(*cue);
                }
            }
            for cue in &review.cues {
                if !audio.cues.contains(cue) {
                    audio.cues.push(*cue);
                }
            }
        }
        if let Some(old) = self.repo.get::<TeacherPracticeDraft>(&key)? {
            if old.saved_attempt.is_some()
                && serde_json::to_value(&old).map_err(|e| e.to_string())?
                    != serde_json::to_value(&merged).map_err(|e| e.to_string())?
            {
                return Err("This revisit is already saved. Its original response and self-report are immutable.".into());
            }
        }
        teacher_revisit_storage_key(&merged)?;
        Ok(merged)
    }
    pub(crate) fn select_teacher_revisit(&self, draft: &TeacherPracticeDraft) -> Result<()> {
        let saved = self
            .repo
            .get::<TeacherPracticeDraft>(&teacher_revisit_storage_key(draft)?)?
            .ok_or("Save this revisit before selecting it.")?;
        if saved.id != draft.id {
            return Err("This exact revisit session is unavailable.".into());
        }
        self.repo.set(
            "teacher_revisit_active",
            &ActiveRevisit {
                document_id: draft.review.as_ref().unwrap().document_id,
                practice_index: draft.index,
                draft_id: draft.id,
            },
        )
    }
    pub(crate) fn active_teacher_revisit(
        &self,
    ) -> Result<Option<(TeacherResponse, TeacherPracticeDraft)>> {
        let Some(active) = self.repo.get::<ActiveRevisit>("teacher_revisit_active")? else {
            return Ok(None);
        };
        let answer = self
            .library
            .teacher_document(active.document_id)
            .ok_or(
                "The active revisit's original document is unavailable. Sync it before continuing.",
            )?
            .response
            .clone();
        let draft = self.teacher_revisit_draft(&answer, active.practice_index)?;
        if draft.id != active.draft_id {
            return Err("The active revisit points to a different saved session. Choose the intended revisit again.".into());
        }
        Ok(Some((answer, draft)))
    }
    pub(crate) fn clear_active_teacher_revisit(&self) -> Result<()> {
        self.repo
            .set("teacher_revisit_active", &None::<ActiveRevisit>)
    }
    pub(crate) fn set_teacher_revisit_rating(
        &self,
        draft: &mut TeacherPracticeDraft,
        rating: SelfRating,
    ) -> Result<()> {
        let mut next = self.merged_teacher_revisit_draft(draft)?;
        if next.saved_attempt.is_some() {
            return Err(
                "This revisit is already saved; its original self-report is immutable.".into(),
            );
        }
        next.review.as_mut().unwrap().self_rating = rating;
        self.save_teacher_practice_draft(&next)?;
        *draft = next;
        Ok(())
    }
    pub(crate) fn teacher_revisit_mark_active_support(&self, cue: Cue) -> Result<()> {
        let Some((_, mut draft)) = self.active_teacher_revisit()? else {
            return Ok(());
        };
        if draft.saved_attempt.is_some() {
            return Ok(());
        }
        if !draft.review.as_ref().unwrap().cues.contains(&cue) {
            draft.review.as_mut().unwrap().cues.push(cue);
            self.save_teacher_practice_draft(&draft)?;
        }
        Ok(())
    }
    pub(crate) fn teacher_revisit_reveal(
        &self,
        draft: &mut TeacherPracticeDraft,
        cue: Cue,
    ) -> Result<()> {
        let mut next = self.merged_teacher_revisit_draft(draft)?;
        if next.saved_attempt.is_some() {
            return Err("This revisit is already saved. Start another scheduled revisit for a fresh response.".into());
        }
        if !next.review.as_ref().unwrap().cues.contains(&cue) {
            next.review.as_mut().unwrap().cues.push(cue);
        }
        self.save_teacher_practice_draft(&next)?;
        *draft = self.merged_teacher_revisit_draft(&next)?;
        Ok(())
    }
}
