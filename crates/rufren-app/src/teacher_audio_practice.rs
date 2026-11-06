//! Durable oral Teacher tasks. Playback/selection refer to exact stored originals, not latest takes.
use crate::{
    model::AppState,
    repository::{Result, now_ms},
    teacher_practice::{TeacherPracticeDraft, practice_source},
    teacher_revisit::{
        TEACHER_REVISIT_POLICY, teacher_revisit_context_reference, teacher_revisit_exercise_id,
    },
};
use rufren_core::*;
use rufren_tutor::{TeacherResponse, TeacherSkill};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;
pub(crate) const TEACHER_AUDIO_POLICY: &str = "teacher-guided-audio-v1";
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct TeacherAudioBinding {
    pub passage_id: Uuid,
    pub revision_id: Uuid,
    pub skill: Skill,
    pub task_fingerprint: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct TeacherPlayedReference {
    pub attachment: Uuid,
    pub presentation: Uuid,
    pub asset_id: String,
    pub sha256: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct TeacherSelectedRecording {
    pub attachment: Uuid,
    pub asset_id: String,
    pub sha256: String,
    pub interrupted: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct TeacherAudioDraft {
    pub binding: TeacherAudioBinding,
    pub reference_plays: u32,
    pub cues: Vec<Cue>,
    pub played_reference: Option<TeacherPlayedReference>,
    pub selected_recording: Option<TeacherSelectedRecording>,
}
impl TeacherPracticeDraft {
    pub(crate) fn selected_asset_id(&self) -> Option<&str> {
        self.audio
            .as_ref()?
            .selected_recording
            .as_ref()
            .map(|r| r.asset_id.as_str())
    }
    pub(crate) fn selected_attachment(&self) -> Option<Uuid> {
        self.audio
            .as_ref()?
            .selected_recording
            .as_ref()
            .map(|r| r.attachment)
    }
}
fn valid_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
impl TeacherAudioDraft {
    pub(crate) fn validate(&self) -> Result<()> {
        if self.binding.passage_id.is_nil()
            || self.binding.revision_id.is_nil()
            || !matches!(self.binding.skill, Skill::Listening | Skill::Speaking)
            || !valid_hash(&self.binding.task_fingerprint)
            || self.cues.len() > 32
            || (self.binding.skill == Skill::Listening && self.selected_recording.is_some())
            || (self.reference_plays == 0) != self.played_reference.is_none()
            || self.played_reference.as_ref().is_some_and(|p| {
                p.attachment.is_nil()
                    || p.presentation.is_nil()
                    || p.asset_id.is_empty()
                    || p.asset_id.len() > 256
                    || !valid_hash(&p.sha256)
            })
            || self.selected_recording.as_ref().is_some_and(|p| {
                p.attachment.is_nil()
                    || p.asset_id.is_empty()
                    || p.asset_id.len() > 256
                    || !valid_hash(&p.sha256)
            })
        {
            return Err("This oral exercise draft has invalid source bindings. Its earlier saved response is kept.".into());
        }
        Ok(())
    }
}
const BINDING_MARKER: &str = "\nRufren oral evidence v1: ";
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct SubmittedAudioEvidence {
    pub binding: TeacherAudioBinding,
    pub question_sha256: String,
    pub played_reference: Option<TeacherPlayedReference>,
    pub selected_recording: Option<TeacherSelectedRecording>,
    pub reference_plays: u32,
}
pub(crate) fn audio_reference_base(context: &AttemptContext) -> Option<&str> {
    context
        .reference_answer
        .as_deref()?
        .rsplit_once(BINDING_MARKER)
        .map(|(base, _)| base)
}
fn review_editable(draft: &TeacherPracticeDraft) -> Result<()> {
    if draft.review.is_some() && draft.saved_attempt.is_some() {
        return Err(
            "This revisit is already saved. Start another scheduled revisit for a fresh response."
                .into(),
        );
    }
    Ok(())
}
fn sync_review_cues(draft: &mut TeacherPracticeDraft) {
    if let (Some(review), Some(audio)) = (&mut draft.review, &draft.audio) {
        review.cues = audio.cues.clone();
    }
}
fn previously_submitted_original(library: &Library, sha256: &str, session: Uuid) -> bool {
    let current = Uuid::new_v5(&session, b"attempt");
    let cutoff = library
        .events
        .get(&current)
        .map_or_else(now_ms, |event| event.recorded_at_ms);
    library.events.values().any(|event| {
        let Fact::RecordingAttached(a) = &event.fact else {
            return false;
        };
        let AnnotationTarget::Attempt(id) = a.target else {
            return false;
        };
        a.role == AudioRole::Attempt
            && a.asset.sha256 == sha256
            && id != current
            && library.events.get(&id).is_some_and(|source| {
                matches!(source.fact, Fact::AttemptRecorded(_)) && source.recorded_at_ms <= cutoff
            })
    })
}
pub(crate) fn submitted_audio_evidence(context: &AttemptContext) -> Option<SubmittedAudioEvidence> {
    let (_, json) = context
        .reference_answer
        .as_deref()?
        .rsplit_once(BINDING_MARKER)?;
    serde_json::from_str(json).ok()
}
pub(crate) fn audio_reference_answer(
    reference: &str,
    prompt: &str,
    audio: &TeacherAudioDraft,
) -> Result<String> {
    let evidence = SubmittedAudioEvidence {
        binding: audio.binding.clone(),
        question_sha256: question_identity(prompt, reference, audio.binding.skill),
        played_reference: audio.played_reference.clone(),
        selected_recording: audio.selected_recording.clone(),
        reference_plays: audio.reference_plays,
    };
    let value = format!(
        "{reference}{BINDING_MARKER}{}",
        serde_json::to_string(&evidence).map_err(|e| e.to_string())?
    );
    if value.len() > 8192 {
        return Err(
            "This oral question exceeds the saved reference budget; your draft is kept.".into(),
        );
    }
    Ok(value)
}
fn bind(answer: &TeacherResponse, draft: &TeacherPracticeDraft) -> Result<TeacherAudioBinding> {
    answer.request.validate().map_err(|e| e.to_string())?;
    answer
        .content
        .validate_for(&answer.request)
        .map_err(|e| e.to_string())?;
    if draft.source != practice_source(answer) || draft.id.is_nil() {
        return Err("This draft belongs to a different teacher question.".into());
    }
    let exercise = answer
        .content
        .practice
        .get(draft.index)
        .ok_or("This teacher exercise is unavailable.")?;
    let skill = match exercise.skill {
        Some(TeacherSkill::Listening) => Skill::Listening,
        Some(TeacherSkill::Speaking) => Skill::Speaking,
        _ => return Err("Choose a listening or speaking exercise.".into()),
    };
    let source = Uuid::parse_str(&draft.source).map_err(|e| e.to_string())?;
    let passage_id = Uuid::new_v5(&source, draft.index.to_string().as_bytes());
    let task_fingerprint = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(exercise).map_err(|e| e.to_string())?)
    );
    Ok(TeacherAudioBinding {
        passage_id,
        revision_id: Uuid::new_v5(&passage_id, b"reference-v1"),
        skill,
        task_fingerprint,
    })
}
fn checked_attachment<'a>(
    library: &'a Library,
    binding: &TeacherAudioBinding,
    id: Uuid,
    role: AudioRole,
    metadata: &AudioRecordMetadata,
) -> Result<&'a AudioAttachment> {
    metadata.validate().map_err(|e| e.to_string())?;
    let Some(Fact::RecordingAttached(record)) = library.events.get(&id).map(|e| &e.fact) else {
        return Err("Sync the exact recording attachment first.".into());
    };
    if record.target != AnnotationTarget::Revision(binding.revision_id)
        || record.role != role
        || metadata.role
            != if role == AudioRole::Reference {
                "reference"
            } else {
                "attempt"
            }
        || metadata.passage_id != binding.passage_id.to_string()
        || metadata.revision_id != binding.revision_id.to_string()
        || record.asset_id != metadata.asset_id
        || record.asset.sha256 != metadata.content_hash
        || record.asset.sample_rate_hz != metadata.sample_rate
        || record.asset.channels != metadata.channels
        || record.asset.frames != metadata.frames
        || record.asset.media_type != metadata.mime_type
    {
        return Err(
            "This recording does not match the exact teacher exercise, role and original.".into(),
        );
    }
    Ok(record)
}
impl AppState {
    pub(crate) fn prepare_teacher_audio_practice(
        &mut self,
        answer: &TeacherResponse,
        draft: &mut TeacherPracticeDraft,
    ) -> Result<TeacherAudioBinding> {
        let binding = bind(answer, draft)?;
        self.refresh()?;
        if draft.review.is_some() {
            let merged = self.merged_teacher_revisit_draft(draft)?;
            self.validate_teacher_revisit(answer, &merged)?;
            *draft = merged;
        }
        let (source, revision_id, reference) =
            crate::teacher_practice::teacher_reference_fact(answer, draft.index)?;
        if revision_id != binding.revision_id {
            return Err("Teacher audio reference identity mismatch.".into());
        }
        if let Some(existing) = self.library.events.get(&revision_id) {
            if existing.fact != reference {
                return Err(
                    "The saved teacher reference has different content; the original is kept."
                        .into(),
                );
            }
        } else {
            self.repo.append(&[Event {
                version: 2,
                id: revision_id,
                device: source,
                recorded_at_ms: 0,
                fact: reference,
            }])?;
            self.refresh()?;
        }
        let mut next = draft.clone();
        if let Some(audio) = &next.audio {
            audio.validate()?;
            if draft
                .review
                .as_ref()
                .map_or(!audio.cues.contains(&Cue::ReferenceMaterial), |review| {
                    review.cues != audio.cues
                })
            {
                return Err(
                    "This oral exercise has inconsistent help cues; its earlier draft is kept."
                        .into(),
                );
            }
            if audio.binding != binding {
                return Err("This saved oral draft belongs to a changed question. Start a fresh teacher exercise.".into());
            }
        } else {
            next.audio = Some(TeacherAudioDraft {
                binding: binding.clone(),
                reference_plays: 0,
                cues: next.review.as_ref().map_or_else(
                    || vec![Cue::ReferenceMaterial],
                    |review| review.cues.clone(),
                ),
                played_reference: None,
                selected_recording: None,
            });
        }
        self.save_teacher_practice_draft(&next)?;
        *draft = next;
        Ok(binding)
    }
    pub(crate) fn teacher_audio_reference_played(
        &mut self,
        answer: &TeacherResponse,
        draft: &mut TeacherPracticeDraft,
        attachment: Uuid,
        record: &AudioRecordMetadata,
    ) -> Result<()> {
        review_editable(draft)?;
        let binding = self.prepare_teacher_audio_practice(answer, draft)?;
        checked_attachment(
            &self.library,
            &binding,
            attachment,
            AudioRole::Reference,
            record,
        )?;
        if record.interrupted {
            return Err("Choose a complete reference recording for this listening input.".into());
        }
        let mut next = draft.clone();
        self.fork_teacher_answer_for_edit(answer, &mut next)?;
        let audio = next.audio.as_mut().unwrap();
        let review = next.review.is_some();
        let audio_family = format!("teacher-oral-reference:{}", binding.revision_id);
        let previous = self
            .library
            .presentations()
            .filter(|(_, p)| {
                p.session == Some(next.id)
                    && p.revision == Some(binding.revision_id)
                    && (if review {
                        p.utterance_family.as_deref() == Some(audio_family.as_str())
                            && matches!(
                                p.purpose,
                                PresentationPurpose::Audio | PresentationPurpose::TaskInput
                            )
                    } else {
                        p.purpose == PresentationPurpose::Audio
                    })
            })
            .count()
            .min(u32::MAX as usize) as u32;
        audio.reference_plays = audio.reference_plays.max(previous).saturating_add(1);
        if audio.reference_plays > 1 || binding.skill == Skill::Speaking {
            if !audio.cues.contains(&Cue::Replay) {
                audio.cues.push(Cue::Replay);
            }
        }
        let russian = self
            .library
            .revision(binding.revision_id)
            .unwrap()
            .passage
            .russian
            .clone();
        let presented = Event::new(
            self.device,
            now_ms(),
            Fact::StimulusPresented(StimulusPresentation {
                session: Some(next.id),
                revision: Some(binding.revision_id),
                russian,
                purpose: if review
                    && binding.skill == Skill::Listening
                    && audio.reference_plays == 1
                {
                    PresentationPurpose::TaskInput
                } else {
                    PresentationPurpose::Audio
                },
                decision: None,
                utterance_family: review.then_some(audio_family),
            }),
        )
        .map_err(|e| e.to_string())?;
        audio.played_reference = Some(TeacherPlayedReference {
            attachment,
            presentation: presented.id,
            asset_id: record.asset_id.clone(),
            sha256: record.content_hash.clone(),
        });
        self.repo.append(&[presented])?;
        sync_review_cues(&mut next);
        self.save_teacher_practice_draft(&next)?;
        *draft = next;
        self.refresh()
    }
    pub(crate) fn teacher_audio_select_recording(
        &mut self,
        answer: &TeacherResponse,
        draft: &mut TeacherPracticeDraft,
        attachment: Uuid,
        record: &AudioRecordMetadata,
        accept_interrupted: bool,
    ) -> Result<()> {
        review_editable(draft)?;
        let binding = self.prepare_teacher_audio_practice(answer, draft)?;
        if binding.skill != Skill::Speaking {
            return Err(
                "This task expects a listening meaning response, not a learner recording.".into(),
            );
        }
        checked_attachment(
            &self.library,
            &binding,
            attachment,
            AudioRole::Attempt,
            record,
        )?;
        if record.interrupted && !accept_interrupted {
            return Err("This take was interrupted. Choose it explicitly if the captured portion is your intended response.".into());
        }
        if draft.review.is_some() {
            let resuming = draft
                .audio
                .as_ref()
                .and_then(|audio| audio.selected_recording.as_ref())
                .is_some_and(|selected| {
                    selected.attachment == attachment
                        && selected.asset_id == record.asset_id
                        && selected.sha256 == record.content_hash
                });
            if previously_submitted_original(&self.library, &record.content_hash, draft.id)
                || (!resuming && record.created_at_ms < draft.started_at_ms)
            {
                // The recorder can still offer this preserved original for replay. Keep
                // that opportunity as support even when it cannot serve as new production.
                let mut supported = draft.clone();
                let cues = &mut supported.audio.as_mut().unwrap().cues;
                if !cues.contains(&Cue::ReferenceMaterial) {
                    cues.push(Cue::ReferenceMaterial);
                }
                sync_review_cues(&mut supported);
                self.save_teacher_practice_draft(&supported)?;
                *draft = supported;
                return Err("Make a new take for this revisit. The earlier take stays with its original attempt; opening it is kept as reference help for this response.".into());
            }
        }
        let selected = TeacherSelectedRecording {
            attachment,
            asset_id: record.asset_id.clone(),
            sha256: record.content_hash.clone(),
            interrupted: record.interrupted,
        };
        if draft
            .audio
            .as_ref()
            .and_then(|a| a.selected_recording.as_ref())
            == Some(&selected)
        {
            return Ok(());
        }
        let mut next = draft.clone();
        self.fork_teacher_answer_for_edit(answer, &mut next)?;
        next.audio.as_mut().unwrap().selected_recording = Some(selected);
        next.response = format!(
            "Selected spoken recording · {}{}",
            record.asset_id,
            if record.interrupted {
                " · interrupted captured portion"
            } else {
                ""
            }
        );
        self.save_teacher_practice_draft(&next)?;
        *draft = next;
        Ok(())
    }
    pub(crate) fn teacher_audio_clear_recording(
        &mut self,
        answer: &TeacherResponse,
        draft: &mut TeacherPracticeDraft,
    ) -> Result<()> {
        review_editable(draft)?;
        let binding = self.prepare_teacher_audio_practice(answer, draft)?;
        if binding.skill != Skill::Speaking {
            return Err("Only speaking tasks have a selected learner take.".into());
        }
        if draft.audio.as_ref().unwrap().selected_recording.is_none() {
            return Ok(());
        }
        let mut next = draft.clone();
        self.fork_teacher_answer_for_edit(answer, &mut next)?;
        next.audio.as_mut().unwrap().selected_recording = None;
        next.response.clear();
        self.save_teacher_practice_draft(&next)?;
        *draft = next;
        Ok(())
    }
    pub(crate) fn teacher_audio_reveal(
        &mut self,
        answer: &TeacherResponse,
        draft: &mut TeacherPracticeDraft,
        cue: Cue,
    ) -> Result<()> {
        review_editable(draft)?;
        self.prepare_teacher_audio_practice(answer, draft)?;
        if !matches!(
            cue,
            Cue::OriginalText | Cue::Translation | Cue::ReferenceMaterial
        ) {
            return Err("Choose a text, meaning or reference reveal.".into());
        }
        if draft.audio.as_ref().unwrap().cues.contains(&cue) {
            return Ok(());
        }
        let mut next = draft.clone();
        self.fork_teacher_answer_for_edit(answer, &mut next)?;
        next.audio.as_mut().unwrap().cues.push(cue);
        sync_review_cues(&mut next);
        self.save_teacher_practice_draft(&next)?;
        *draft = next;
        Ok(())
    }
}

fn question_identity(prompt: &str, reference: &str, skill: Skill) -> String {
    format!(
        "{:x}",
        Sha256::digest(
            serde_json::to_vec(&(prompt, reference, skill)).expect("question strings serialize")
        )
    )
}
fn checked_compact<'a>(
    library: &'a Library,
    binding: &TeacherAudioBinding,
    id: Uuid,
    asset_id: &str,
    sha256: &str,
    role: AudioRole,
) -> Option<&'a AudioAttachment> {
    match &library.events.get(&id)?.fact {
        Fact::RecordingAttached(record)
            if record.target == AnnotationTarget::Revision(binding.revision_id)
                && record.role == role
                && record.asset_id == asset_id
                && record.asset.sha256 == sha256 =>
        {
            Some(record)
        }
        _ => None,
    }
}
fn valid_played_presentation(
    library: &Library,
    binding: &TeacherAudioBinding,
    played: &TeacherPlayedReference,
    session: Uuid,
    plays: u32,
    review: bool,
) -> bool {
    let Some(reference) = library.revision(binding.revision_id) else {
        return false;
    };
    let purpose = if review && binding.skill == Skill::Listening && plays == 1 {
        PresentationPurpose::TaskInput
    } else {
        PresentationPurpose::Audio
    };
    matches!(library.events.get(&played.presentation).map(|event|&event.fact),Some(Fact::StimulusPresented(p))
        if p.purpose==purpose&&p.revision==Some(binding.revision_id)&&p.russian==reference.passage.russian
        &&(if review {p.session==Some(session)&&p.utterance_family.as_deref()==Some(format!("teacher-oral-reference:{}",binding.revision_id).as_str())}else{p.session.is_some()}))
}
pub(crate) fn valid_submitted_audio(
    library: &Library,
    attempt: Uuid,
    context: &AttemptContext,
) -> bool {
    let Some(a) = library.attempt(attempt) else {
        return false;
    };
    let review = a.policy == TEACHER_REVISIT_POLICY;
    if !review && a.policy != TEACHER_AUDIO_POLICY {
        return false;
    }
    let Some(evidence) = submitted_audio_evidence(context) else {
        return false;
    };
    let Some(reference) = library.revision(a.revision) else {
        return false;
    };
    let Some((base, _)) = context
        .reference_answer
        .as_deref()
        .and_then(|value| value.rsplit_once(BINDING_MARKER))
    else {
        return false;
    };
    if evidence.binding.revision_id != a.revision
        || evidence.binding.passage_id != reference.passage.id
        || evidence.binding.skill != a.skill
        || !valid_hash(&evidence.binding.task_fingerprint)
        || context.prompt.is_none()
        || evidence.question_sha256
            != question_identity(context.prompt.as_deref().unwrap(), base, a.skill)
        || (!review && !a.cues.contains(&Cue::ReferenceMaterial))
        || ((evidence.reference_plays > 1
            || a.skill == Skill::Speaking && evidence.reference_plays > 0)
            && !a.cues.contains(&Cue::Replay))
        || a.cues.len() > 32
    {
        return false;
    }
    if let Some(played) = &evidence.played_reference {
        if checked_compact(
            library,
            &evidence.binding,
            played.attachment,
            &played.asset_id,
            &played.sha256,
            AudioRole::Reference,
        )
        .is_none()
        {
            return false;
        }
        if !valid_played_presentation(
            library,
            &evidence.binding,
            played,
            a.session,
            evidence.reference_plays,
            review,
        ) {
            return false;
        }
        if evidence.reference_plays == 0 {
            return false;
        }
    } else if evidence.reference_plays != 0 {
        return false;
    }
    match a.skill {
        Skill::Listening => {
            evidence.played_reference.is_some() && evidence.selected_recording.is_none()
        }
        Skill::Speaking => {
            if review
                && previously_submitted_original(
                    library,
                    evidence
                        .selected_recording
                        .as_ref()
                        .map_or("", |s| s.sha256.as_str()),
                    a.session,
                )
            {
                return false;
            }
            let Some(selected) = &evidence.selected_recording else {
                return false;
            };
            let Some(original) = checked_compact(
                library,
                &evidence.binding,
                selected.attachment,
                &selected.asset_id,
                &selected.sha256,
                AudioRole::Attempt,
            ) else {
                return false;
            };
            let id = Uuid::new_v5(&a.session, b"recording");
            match library.events.get(&id).map(|e| &e.fact) {
                Some(Fact::RecordingAttached(copy))
                    if copy.target == AnnotationTarget::Attempt(attempt)
                        && copy.asset_id == original.asset_id
                        && copy.asset == original.asset
                        && copy.role == AudioRole::Attempt
                        && copy.note == original.note =>
                {
                    true
                }
                _ => false,
            }
        }
        _ => false,
    }
}
impl AppState {
    pub(crate) fn submit_teacher_audio_practice(
        &mut self,
        answer: &TeacherResponse,
        draft: &mut TeacherPracticeDraft,
    ) -> Result<Uuid> {
        let binding = self.prepare_teacher_audio_practice(answer, draft)?;
        if draft.review.is_some() {
            self.qualify_teacher_revisit_support(answer, draft)?;
        }
        let audio = draft
            .audio
            .as_ref()
            .ok_or("Prepare this oral exercise first.")?
            .clone();
        audio.validate()?;
        if draft.review.is_some() {
            self.validate_teacher_revisit(answer, draft)?;
            if draft.review.as_ref().unwrap().cues != audio.cues {
                return Err("The saved revisit help cues do not match this oral response.".into());
            }
        }
        let review = draft.review.is_some();
        let exercise = answer
            .content
            .practice
            .get(draft.index)
            .ok_or("This teacher exercise is unavailable.")?;
        if draft.response.trim().is_empty() || draft.response.len() > 8192 {
            return Err(if binding.skill == Skill::Speaking {
                "Choose a saved recording for this response."
            } else {
                "Listen to the reference and write what you understood."
            }
            .into());
        }
        let selected = if binding.skill == Skill::Speaking {
            let selected = audio
                .selected_recording
                .as_ref()
                .ok_or("Choose the exact speaking take before submitting.")?;
            if review && previously_submitted_original(&self.library, &selected.sha256, draft.id) {
                return Err("Make a new take for this revisit; the previous response cannot stand in for a new production.".into());
            }
            Some(
                checked_compact(
                    &self.library,
                    &binding,
                    selected.attachment,
                    &selected.asset_id,
                    &selected.sha256,
                    AudioRole::Attempt,
                )
                .ok_or("Sync the exact selected recording before submitting.")?
                .clone(),
            )
        } else {
            None
        };
        if binding.skill == Skill::Listening && audio.played_reference.is_none() {
            return Err("Play the correct reference before saving this listening response.".into());
        }
        if let Some(played) = &audio.played_reference {
            if checked_compact(
                &self.library,
                &binding,
                played.attachment,
                &played.asset_id,
                &played.sha256,
                AudioRole::Reference,
            )
            .is_none()
            {
                return Err("The played reference no longer matches this exact exercise.".into());
            }
            if !valid_played_presentation(
                &self.library,
                &binding,
                played,
                draft.id,
                audio.reference_plays,
                review,
            ) {
                return Err(
                    "This draft has no durable acknowledgment of its actual reference playback."
                        .into(),
                );
            }
        }
        let attempt_id = Uuid::new_v5(&draft.id, b"attempt");
        let context_id = Uuid::new_v5(&draft.id, b"context");
        let source = Uuid::parse_str(&draft.source).map_err(|e| e.to_string())?;
        let reference = format!(
            "Example answer: {}\nReference Russian: {}\nReference meaning: {}\nTeaching note: {}",
            exercise
                .expected_response
                .as_deref()
                .unwrap_or("An open response; accept valid alternatives."),
            exercise.reference_russian.as_deref().unwrap(),
            exercise.reference_english.as_deref().unwrap(),
            exercise.teaching_note
        );
        let reference = if review {
            teacher_revisit_context_reference(&reference, draft)?
        } else {
            reference
        };
        let reference = audio_reference_answer(&reference, &exercise.prompt, &audio)?;
        let context = AttemptContext {
            attempt: attempt_id,
            content_pack: None,
            content_revision: None,
            exercise_id: Some(if review {
                teacher_revisit_exercise_id(draft)?
            } else {
                self.teacher_practice_exercise_id(
                    answer,
                    draft.index,
                    source,
                    binding.passage_id,
                    context_id,
                    attempt_id,
                )
            }),
            prompt: Some(exercise.prompt.clone()),
            reference_answer: Some(reference),
            task: if binding.skill == Skill::Listening {
                TaskKind::MeaningRecall
            } else {
                TaskKind::FormRecall
            },
            target_features: Default::default(),
            novel_context: false,
            delayed_probe: false,
            practice_duration_ms: None,
            effort: None,
            grammar_response: None,
            supersedes: Default::default(),
        };
        let response = if let Some(selected) = &audio.selected_recording {
            format!(
                "Selected spoken recording · {}{}",
                selected.asset_id,
                if selected.interrupted {
                    " · interrupted captured portion"
                } else {
                    ""
                }
            )
        } else {
            draft.response.clone()
        };
        if binding.skill == Skill::Speaking && draft.response != response {
            return Err("The speaking response label does not match the selected take; choose that original again.".into());
        }
        let attempt = Attempt {
            session: draft.id,
            revision: binding.revision_id,
            skill: binding.skill,
            response,
            cues: audio.cues.clone(),
            self_rating: draft
                .review
                .as_ref()
                .map_or(SelfRating::NotReported, |review| review.self_rating),
            policy: if review {
                TEACHER_REVISIT_POLICY
            } else {
                TEACHER_AUDIO_POLICY
            }
            .into(),
        };
        let mut facts = vec![
            (attempt_id, Fact::AttemptRecorded(attempt)),
            (context_id, Fact::AttemptContextRecorded(context)),
        ];
        if let Some(mut selected) = selected {
            selected.target = AnnotationTarget::Attempt(attempt_id);
            facts.push((
                Uuid::new_v5(&draft.id, b"recording"),
                Fact::RecordingAttached(selected),
            ));
        }
        let mut events = Vec::new();
        for (id, fact) in facts {
            if let Some(old) = self.library.events.get(&id) {
                if old.fact != fact {
                    return Err("This saved oral response has different contents. Start a fresh answer; the earlier attempt is kept.".into());
                }
                continue;
            }
            events.push(Event {
                version: EVENT_VERSION,
                id,
                device: self.device,
                recorded_at_ms: now_ms(),
                fact,
            });
        }
        self.repo.append(&events)?;
        self.refresh()?;
        let Some(Fact::AttemptContextRecorded(saved_context)) = self
            .library
            .events
            .get(&context_id)
            .map(|event| &event.fact)
        else {
            return Err(
                "The exact saved oral context is unavailable; its source facts are kept.".into(),
            );
        };
        if !valid_submitted_audio(&self.library, attempt_id, saved_context) {
            return Err(
                "The saved oral source chain could not be verified. Its facts remain preserved."
                    .into(),
            );
        }
        let mut next = draft.clone();
        next.saved_attempt = Some(attempt_id);
        self.save_teacher_practice_draft(&next)?;
        *draft = next;
        Ok(attempt_id)
    }
}
