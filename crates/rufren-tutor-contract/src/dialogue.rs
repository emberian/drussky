//! Ordinary learner-authored dialogue. Messages are not graded practice attempts.
use crate::{ReviewStatus, TextCorrection, TutorError, TutorExample, TutorProvenance, TutorUsage};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const DIALOGUE_MAX_TEXT_BYTES: usize = 2048;
pub const DIALOGUE_MAX_ANCESTORS: usize = 8;
pub const DIALOGUE_MAX_HISTORY_BYTES: usize = 10 * 1024;
pub const DIALOGUE_MAX_ANCHOR_BYTES: usize = 8 * 1024;
pub const DIALOGUE_MAX_REQUEST_BYTES: usize = 24_576;
pub const DIALOGUE_MAX_CONTENT_BYTES: usize = 6 * 1024;
pub const DIALOGUE_ANCHOR_SNAPSHOT_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DialogueCapability {
    ThreadedV1,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DialogueIntent {
    Clarification,
    Roleplay,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DialogueConfig {
    pub intent: DialogueIntent,
    pub topic: String,
    pub teacher_role: Option<String>,
    pub learner_role: Option<String>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DialogueAction {
    Ask,
    Respond,
    StartScene,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DialogueCorrection {
    Off,
    OneTargeted,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DialogueRole {
    User,
    Assistant,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DialogueAncestor {
    pub message_id: Uuid,
    pub role: DialogueRole,
    pub text: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TeacherDialogueFocus {
    Explanation,
    Step { step_id: String },
    Practice { index: usize },
    Feedback,
    ReaderWhole,
    ReaderChunk { chunk_id: String },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TeacherDialogueAnchor {
    pub document_id: Uuid,
    pub focus: TeacherDialogueFocus,
    pub snapshot_version: u32,
    /// Exact canonical projection, checked against the archived source by the core.
    pub snapshot: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DialogueConfirmedAudioRef {
    pub recording: Uuid,
    pub transcription: Uuid,
    pub confirmation: Uuid,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TeacherDialogueRequest {
    /// Canonical logical user message identity, frozen before the network request.
    pub request_id: Uuid,
    pub thread_id: Uuid,
    pub parent_reply_id: Option<Uuid>,
    pub config: DialogueConfig,
    pub action: DialogueAction,
    pub correction: DialogueCorrection,
    pub anchor: Option<TeacherDialogueAnchor>,
    pub ancestors: Vec<DialogueAncestor>,
    pub omitted_parent_id: Option<Uuid>,
    pub omitted_count: usize,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confirmed_audio: Option<DialogueConfirmedAudioRef>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DialogueSceneState {
    Open,
    Closed,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TeacherDialogueContent {
    pub explanation_english: Option<String>,
    pub utterance: Option<TutorExample>,
    pub next_question: Option<TutorExample>,
    pub correction: Option<TextCorrection>,
    pub scene_state: DialogueSceneState,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TeacherDialogueResponse {
    /// Bound by the server from validated input; never accepted from model output.
    pub request: TeacherDialogueRequest,
    pub content: TeacherDialogueContent,
    pub provenance: TutorProvenance,
    pub usage: TutorUsage,
}

fn bounded(text: &str, limit: usize) -> bool {
    !text.trim().is_empty() && text.len() <= limit
}
impl TeacherDialogueRequest {
    pub fn validate(&self) -> Result<(), TutorError> {
        use std::collections::BTreeSet;
        let invalid = || TutorError::InvalidRequest;
        if self.request_id.is_nil()
            || self.thread_id.is_nil()
            || !bounded(&self.config.topic, 1024)
            || [&self.config.teacher_role, &self.config.learner_role]
                .iter()
                .any(|s| s.as_ref().is_some_and(|s| !bounded(s, 512)))
            || self.text.len() > DIALOGUE_MAX_TEXT_BYTES
            || (self.text.trim().is_empty() && self.action != DialogueAction::StartScene)
            || (self.action == DialogueAction::StartScene
                && (self.config.intent != DialogueIntent::Roleplay
                    || self.parent_reply_id.is_some()
                    || self.confirmed_audio.is_some()))
            || self.ancestors.len() > DIALOGUE_MAX_ANCESTORS
            || self.ancestors.len() % 2 != 0
            || serde_json::to_vec(&self.ancestors)
                .map_err(|_| invalid())?
                .len()
                > DIALOGUE_MAX_HISTORY_BYTES
            || (self.omitted_count == 0) != self.omitted_parent_id.is_none()
            || self.omitted_count % 2 != 0
            || self.omitted_parent_id.is_some_and(|id| id.is_nil())
        {
            return Err(invalid());
        }
        match self.parent_reply_id {
            None if self.thread_id != self.request_id
                || !self.ancestors.is_empty()
                || self.omitted_count != 0 =>
            {
                return Err(invalid());
            }
            Some(parent)
                if parent.is_nil()
                    || self.request_id == self.thread_id
                    || self.ancestors.last().is_none_or(|a| a.message_id != parent)
                    || (self.omitted_count == 0
                        && self
                            .ancestors
                            .first()
                            .is_none_or(|a| a.message_id != self.thread_id))
                    || (self.omitted_count > 0
                        && self
                            .ancestors
                            .iter()
                            .any(|a| a.message_id == self.thread_id)) =>
            {
                return Err(invalid());
            }
            _ => {}
        }
        let mut ids = BTreeSet::new();
        ids.insert(self.request_id);
        if self
            .omitted_parent_id
            .is_some_and(|id| id == self.request_id)
        {
            return Err(invalid());
        }
        for (index, ancestor) in self.ancestors.iter().enumerate() {
            let role = if index % 2 == 0 {
                DialogueRole::User
            } else {
                DialogueRole::Assistant
            };
            // A StartScene user action has no fictional learner utterance; its
            // canonical ancestor text is supplied by the core, not invented here.
            if ancestor.message_id.is_nil()
                || !ids.insert(ancestor.message_id)
                || ancestor.role != role
                || !bounded(
                    &ancestor.text,
                    if role == DialogueRole::User {
                        3072
                    } else {
                        7168
                    },
                )
                || self.omitted_parent_id == Some(ancestor.message_id)
            {
                return Err(invalid());
            }
        }
        if let Some(anchor) = &self.anchor {
            if anchor.document_id.is_nil()
                || anchor.snapshot_version != DIALOGUE_ANCHOR_SNAPSHOT_VERSION
                || !bounded(&anchor.snapshot, DIALOGUE_MAX_ANCHOR_BYTES)
                || match &anchor.focus {
                    TeacherDialogueFocus::Step { step_id } => {
                        !bounded(step_id, 64)
                            || !step_id
                                .bytes()
                                .all(|c| c.is_ascii_alphanumeric() || b"-_".contains(&c))
                    }
                    TeacherDialogueFocus::Practice { index } => *index >= 4,
                    TeacherDialogueFocus::ReaderChunk { chunk_id } => {
                        !bounded(chunk_id, 128)
                            || !chunk_id
                                .bytes()
                                .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
                    }
                    _ => false,
                }
            {
                return Err(invalid());
            }
        }
        if let Some(audio) = &self.confirmed_audio {
            if [audio.recording, audio.transcription, audio.confirmation]
                .iter()
                .any(Uuid::is_nil)
                || self.text.trim().is_empty()
            {
                return Err(invalid());
            }
        }
        if serde_json::to_vec(self).map_err(|_| invalid())?.len() > DIALOGUE_MAX_REQUEST_BYTES {
            return Err(invalid());
        }
        Ok(())
    }
}
impl TeacherDialogueContent {
    pub fn validate_for(&self, request: &TeacherDialogueRequest) -> Result<(), TutorError> {
        let invalid = || TutorError::InvalidResponse;
        if self
            .explanation_english
            .as_ref()
            .is_some_and(|s| !bounded(s, 4096))
            || (self.explanation_english.is_none()
                && self.utterance.is_none()
                && self.next_question.is_none())
            || (request.config.intent == DialogueIntent::Clarification
                && self.explanation_english.is_none())
            || (request.config.intent == DialogueIntent::Roleplay
                && request.action != DialogueAction::Ask
                && self.utterance.is_none()
                && self.next_question.is_none())
            || (self.scene_state == DialogueSceneState::Closed && self.next_question.is_some())
            || serde_json::to_vec(self).map_err(|_| invalid())?.len() > DIALOGUE_MAX_CONTENT_BYTES
        {
            return Err(invalid());
        }
        // A singular field is the structural question limit. Punctuation in a
        // quoted example cannot establish how many questions the teacher asks.
        for chunk in [&self.utterance, &self.next_question].into_iter().flatten() {
            if !crate::teacher::oral_reference(&chunk.russian)
                || !bounded(&chunk.english, 1024)
                || !bounded(&chunk.note, 512)
            {
                return Err(invalid());
            }
        }
        if let Some(correction) = &self.correction {
            if request.correction != DialogueCorrection::OneTargeted
                || request.action == DialogueAction::StartScene
                || request.text.trim().is_empty()
                || correction.original.len() > 512
                || correction.replacement.len() > 512
                || !bounded(&correction.reason, 1024)
                || !request.text.contains(&correction.original)
                || (correction.original.is_empty() && correction.replacement.is_empty())
            {
                return Err(invalid());
            }
        }
        Ok(())
    }
}
impl TeacherDialogueResponse {
    pub fn validate(&self) -> Result<(), TutorError> {
        self.request.validate()?;
        self.content.validate_for(&self.request)?;
        let p = &self.provenance;
        if p.review_status != ReviewStatus::AiGeneratedTeaching
            || [&p.provider, &p.model, &p.response_id, &p.prompt_version]
                .iter()
                .any(|s| !bounded(s, 256) || s.chars().any(char::is_control))
            || !bounded(&p.scope, 2048)
        {
            return Err(TutorError::InvalidResponse);
        }
        Ok(())
    }
}
