//! Durable ordinary dialogue. Immutable messages are shared; composer and delivery state are local.
use crate::{
    model::AppState,
    repository::{Result, now_ms},
};
use rufren_core::*;
use rufren_tutor::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

const LOCAL_KEY: &str = "teacher_dialogue_state";
const MAX_DRAFTS: usize = 16;
const MAX_OUTBOX: usize = 8;
const MAX_LOCAL_BYTES: usize = 512 * 1024;
thread_local! { static RUNTIME_INSTANCE:Uuid=Uuid::new_v4(); }
fn runtime_instance() -> Uuid {
    RUNTIME_INSTANCE.with(|id| *id)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DialogueDraft {
    pub id: Uuid,
    pub thread_id: Uuid,
    pub parent_reply_id: Option<Uuid>,
    pub config: DialogueConfig,
    pub action: DialogueAction,
    pub correction: DialogueCorrection,
    pub anchor: Option<TeacherDialogueAnchor>,
    pub text: String,
    pub confirmed_audio: Option<DialogueConfirmedAudioRef>,
    pub revision: u64,
    pub committed_request_id: Option<Uuid>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DialogueDeliveryStatus {
    PendingCommit,
    Ready,
    Sending,
    OutcomeUnknown,
}
#[derive(Clone, Debug)]
pub(crate) struct DialogueOutboxSummary {
    pub request_id: Uuid,
    pub status: DialogueDeliveryStatus,
    pub problem: Option<String>,
}
#[derive(Clone, Debug)]
pub(crate) struct DialogueDispatch {
    pub request: TeacherDialogueRequest,
    pub dispatch_id: Uuid,
    draft_id: Option<Uuid>,
    draft_revision: Option<u64>,
    selection_epoch: u64,
}
#[derive(Clone, Default, Debug)]
pub(crate) struct DialogueRecoveryReport {
    pub committed: usize,
    pub interrupted: usize,
    pub diagnostics: Vec<String>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OutboxEntry {
    event: Event,
    origin_revision: u64,
    status: DialogueDeliveryStatus,
    dispatch_id: Option<Uuid>,
    runtime_instance: Option<Uuid>,
    problem: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LocalDialogue {
    version: u32,
    active: Option<Uuid>,
    selection_epoch: u64,
    drafts: BTreeMap<Uuid, DialogueDraft>,
    outbox: BTreeMap<Uuid, OutboxEntry>,
}
impl Default for LocalDialogue {
    fn default() -> Self {
        Self {
            version: 1,
            active: None,
            selection_epoch: 0,
            drafts: BTreeMap::new(),
            outbox: BTreeMap::new(),
        }
    }
}
fn invalid_local() -> String {
    "Saved dialogue drafts or delivery state need recovery review. Their original data was kept."
        .into()
}
fn bump(value: u64) -> Result<u64> {
    value.checked_add(1).ok_or_else(invalid_local)
}
fn request_of(entry: &OutboxEntry) -> Result<&TeacherDialogueRequest> {
    match &entry.event.fact {
        Fact::TeacherDialogueUserRecorded(request) => Ok(request),
        _ => Err(invalid_local()),
    }
}
fn validate_draft(draft: &DialogueDraft) -> Result<()> {
    if draft.id.is_nil()
        || draft.thread_id.is_nil()
        || draft.revision == 0
        || draft.text.len() > DIALOGUE_MAX_TEXT_BYTES
        || draft.committed_request_id.is_some_and(|id| id != draft.id)
        || match draft.parent_reply_id {
            None => draft.thread_id != draft.id,
            Some(parent) => parent.is_nil() || parent == draft.id || draft.thread_id == draft.id,
        }
        || draft.config.topic.len() > 1024
        || draft
            .config
            .teacher_role
            .as_ref()
            .is_some_and(|s| s.len() > 512)
        || draft
            .config
            .learner_role
            .as_ref()
            .is_some_and(|s| s.len() > 512)
    {
        return Err(invalid_local());
    }
    Ok(())
}
impl AppState {
    /// The source link must also work on a device without the origin's local cursor.
    pub(crate) fn return_to_dialogue_source(
        &mut self,
        anchor: &TeacherDialogueAnchor,
    ) -> Result<()> {
        let expected =
            teacher_dialogue_anchor(&self.library, anchor.document_id, anchor.focus.clone())
                .map_err(|error| error.to_string())?;
        if expected != *anchor {
            return Err("This conversation's saved source does not match the available text. Its original question is kept.".into());
        }
        if matches!(
            &anchor.focus,
            TeacherDialogueFocus::ReaderWhole | TeacherDialogueFocus::ReaderChunk { .. }
        ) {
            self.note_reference_access()?;
            let chunk = match &anchor.focus {
                TeacherDialogueFocus::ReaderChunk { chunk_id } => Some(chunk_id.as_str()),
                _ => None,
            };
            self.open_reader_document(anchor.document_id, chunk)?;
            return self.navigate(crate::model::Page::Reader);
        }
        let document = self
            .library
            .teacher_document(anchor.document_id)
            .ok_or("Sync this conversation's saved source first.")?;
        if matches!(document.response.request, TeacherRequest::TeachLesson(_)) {
            let flow = document.response.content.lesson_flow.as_ref();
            let step = match &anchor.focus {
                TeacherDialogueFocus::Step { step_id } => flow
                    .and_then(|flow| flow.steps.iter().position(|step| step.id == *step_id))
                    .ok_or("The source step is unavailable in this saved lesson.")?,
                TeacherDialogueFocus::Practice { index } => match flow {
                    Some(flow) => flow
                        .steps
                        .iter()
                        .position(|step| step.practice_index == Some(*index))
                        .ok_or("This practice is not part of the saved lesson's guided steps.")?,
                    None => index + 1,
                },
                TeacherDialogueFocus::Explanation => 0,
                TeacherDialogueFocus::Feedback => {
                    return Err("This source is a lesson rather than feedback.".into());
                }
                TeacherDialogueFocus::ReaderWhole | TeacherDialogueFocus::ReaderChunk { .. } => {
                    return Err("This passage source is unavailable. Sync it and try again.".into());
                }
            };
            self.set_teacher_lesson_cursor(anchor.document_id, step)?;
        }
        self.select_teacher_document(anchor.document_id)?;
        self.repo.set("teacher_surface", &"lessons")
    }

    fn load_dialogue_local(&self) -> Result<LocalDialogue> {
        let local: LocalDialogue = self.repo.get(LOCAL_KEY)?.unwrap_or_default();
        if local.version != 1
            || local.drafts.len() > MAX_DRAFTS
            || local.outbox.len() > MAX_OUTBOX
            || local
                .active
                .is_some_and(|id| !local.drafts.contains_key(&id))
        {
            return Err(invalid_local());
        }
        for (id, draft) in &local.drafts {
            if *id != draft.id {
                return Err(invalid_local());
            }
            validate_draft(draft)?;
        }
        for (id, entry) in &local.outbox {
            let request = request_of(entry)?;
            if request.request_id != *id
                || entry.origin_revision == 0
                || entry.event.validate().is_err()
                || entry.problem.as_ref().is_some_and(|p| p.len() > 1024)
                || (entry.status == DialogueDeliveryStatus::Sending
                    && (entry.dispatch_id.is_none() || entry.runtime_instance.is_none()))
            {
                return Err(invalid_local());
            }
        }
        Ok(local)
    }
    fn save_dialogue_local(&self, local: &LocalDialogue) -> Result<()> {
        if local.drafts.len() > MAX_DRAFTS
            || local.outbox.len() > MAX_OUTBOX
            || serde_json::to_vec(local).map_err(|e| e.to_string())?.len() > MAX_LOCAL_BYTES
        {
            return Err("There are too many unfinished dialogue drafts or pending messages on this device. Finish an existing message before starting another; all earlier work is kept.".into());
        }
        self.repo.set(LOCAL_KEY, local)
    }
    fn compact_dialogue_local(&self, local: &mut LocalDialogue) {
        // Only redundant local cache for messages already durable in the shared graph.
        // Unsent text, unfinished commits and live dispatches are never discarded.
        local.outbox.retain(|id, entry| {
            entry.status == DialogueDeliveryStatus::Sending
                || self.library.dialogue_replies(*id).is_empty()
        });
        local.drafts.retain(|id, draft| {
            Some(*id) == local.active
                || draft.committed_request_id.is_none()
                || self.library.dialogue_user(*id).is_none()
                || self.library.dialogue_replies(*id).is_empty()
        });
    }
    fn insert_dialogue_draft(
        &self,
        local: &mut LocalDialogue,
        draft: DialogueDraft,
    ) -> Result<DialogueDraft> {
        self.compact_dialogue_local(local);
        validate_draft(&draft)?;
        if !local.drafts.contains_key(&draft.id) && local.drafts.len() >= MAX_DRAFTS {
            return Err("You have sixteen unfinished dialogue drafts here. Continue an existing draft before starting another; your text is kept.".into());
        }
        local.active = Some(draft.id);
        local.selection_epoch = bump(local.selection_epoch)?;
        local.drafts.insert(draft.id, draft.clone());
        self.save_dialogue_local(local)?;
        Ok(draft)
    }
    pub(crate) fn new_dialogue(
        &self,
        config: DialogueConfig,
        action: DialogueAction,
        anchor: Option<(Uuid, TeacherDialogueFocus)>,
    ) -> Result<DialogueDraft> {
        let mut local = self.load_dialogue_local()?;
        let id = Uuid::new_v4();
        let anchor = anchor
            .map(|(document, focus)| {
                teacher_dialogue_anchor(&self.library, document, focus).map_err(|e| e.to_string())
            })
            .transpose()?;
        let draft = DialogueDraft {
            id,
            thread_id: id,
            parent_reply_id: None,
            config,
            action,
            correction: DialogueCorrection::Off,
            anchor,
            text: String::new(),
            confirmed_audio: None,
            revision: 1,
            committed_request_id: None,
        };
        self.insert_dialogue_draft(&mut local, draft)
    }
    pub(crate) fn new_anchored_dialogue(
        &self,
        document: Uuid,
        focus: TeacherDialogueFocus,
        config: DialogueConfig,
        action: DialogueAction,
    ) -> Result<DialogueDraft> {
        self.new_dialogue(config, action, Some((document, focus)))
    }
    /// Save a complete editable source-bound composer in one local transaction.
    pub(crate) fn new_anchored_dialogue_with_text(
        &self,
        document: Uuid,
        focus: TeacherDialogueFocus,
        config: DialogueConfig,
        action: DialogueAction,
        text: String,
    ) -> Result<DialogueDraft> {
        if text.len() > DIALOGUE_MAX_TEXT_BYTES {
            return Err(
                "Keep one dialogue message under 2 KiB. Your earlier drafts are kept.".into(),
            );
        }
        let anchor =
            teacher_dialogue_anchor(&self.library, document, focus).map_err(|e| e.to_string())?;
        let mut local = self.load_dialogue_local()?;
        let id = Uuid::new_v4();
        self.insert_dialogue_draft(
            &mut local,
            DialogueDraft {
                id,
                thread_id: id,
                parent_reply_id: None,
                config,
                action,
                correction: DialogueCorrection::Off,
                anchor: Some(anchor),
                text,
                confirmed_audio: None,
                revision: 1,
                committed_request_id: None,
            },
        )
    }
    pub(crate) fn dialogue_drafts(&self) -> Result<Vec<DialogueDraft>> {
        Ok(self.load_dialogue_local()?.drafts.into_values().collect())
    }
    pub(crate) fn active_dialogue_draft(&self) -> Result<Option<DialogueDraft>> {
        let local = self.load_dialogue_local()?;
        Ok(local.active.and_then(|id| local.drafts.get(&id).cloned()))
    }
    /// Explicitly discard only unsent local text. Source audio and immutable messages remain.
    pub(crate) fn discard_dialogue_draft(&self, id: Uuid) -> Result<()> {
        let mut local = self.load_dialogue_local()?;
        let draft = local
            .drafts
            .get(&id)
            .ok_or("That local dialogue draft is unavailable.")?;
        if draft.committed_request_id.is_some()
            || local.outbox.contains_key(&id)
            || self.repo.library()?.dialogue_user(id).is_some()
        {
            return Err("This message is already saved or queued. Its immutable conversation is kept; only an unsent local draft can be discarded.".into());
        }
        local.drafts.remove(&id);
        if local.active == Some(id) {
            local.active = None;
        }
        local.selection_epoch = bump(local.selection_epoch)?;
        self.save_dialogue_local(&local)
    }
    pub(crate) fn dialogue_selected_leaf(&self) -> Result<Option<Uuid>> {
        Ok(self
            .active_dialogue_draft()?
            .and_then(|draft| draft.committed_request_id.or(draft.parent_reply_id)))
    }
    pub(crate) fn select_dialogue_draft(&self, id: Uuid) -> Result<DialogueDraft> {
        let mut local = self.load_dialogue_local()?;
        let draft = local
            .drafts
            .get(&id)
            .cloned()
            .ok_or("That local dialogue draft is unavailable.")?;
        local.active = Some(id);
        local.selection_epoch = bump(local.selection_epoch)?;
        self.save_dialogue_local(&local)?;
        Ok(draft)
    }
    pub(crate) fn open_dialogue_reply(&self, reply_id: Uuid) -> Result<DialogueDraft> {
        let reply = self
            .library
            .dialogue_reply(reply_id)
            .ok_or("Sync that exact teacher reply before continuing.")?;
        let path = self
            .library
            .dialogue_path(reply_id)
            .map_err(|e| e.to_string())?;
        if !path.is_complete() {
            return Err("Sync the missing dialogue history, source or confirmed words before continuing this branch.".into());
        }
        let mut local = self.load_dialogue_local()?;
        if let Some(draft) = local
            .drafts
            .values()
            .find(|draft| {
                draft.parent_reply_id == Some(reply_id) && draft.committed_request_id.is_none()
            })
            .cloned()
        {
            local.active = Some(draft.id);
            local.selection_epoch = bump(local.selection_epoch)?;
            self.save_dialogue_local(&local)?;
            return Ok(draft);
        }
        self.insert_dialogue_draft(
            &mut local,
            DialogueDraft {
                id: Uuid::new_v4(),
                thread_id: reply.request.thread_id,
                parent_reply_id: Some(reply_id),
                config: reply.request.config.clone(),
                action: if reply.request.config.intent == DialogueIntent::Roleplay {
                    DialogueAction::Respond
                } else {
                    DialogueAction::Ask
                },
                correction: reply.request.correction,
                anchor: reply.request.anchor.clone(),
                text: String::new(),
                confirmed_audio: None,
                revision: 1,
                committed_request_id: None,
            },
        )
    }
    pub(crate) fn open_dialogue_user(&self, request_id: Uuid) -> Result<DialogueDraft> {
        let request = self
            .library
            .dialogue_user(request_id)
            .ok_or("Sync that saved learner message first.")?;
        let mut local = self.load_dialogue_local()?;
        if let Some(draft) = local.drafts.get(&request_id).cloned() {
            local.active = Some(request_id);
            local.selection_epoch = bump(local.selection_epoch)?;
            self.save_dialogue_local(&local)?;
            return Ok(draft);
        }
        self.insert_dialogue_draft(
            &mut local,
            DialogueDraft {
                id: request_id,
                thread_id: request.thread_id,
                parent_reply_id: request.parent_reply_id,
                config: request.config.clone(),
                action: request.action,
                correction: request.correction,
                anchor: request.anchor.clone(),
                text: request.text.clone(),
                confirmed_audio: request.confirmed_audio.clone(),
                revision: 1,
                committed_request_id: Some(request_id),
            },
        )
    }
    fn edit_dialogue_draft(
        &self,
        draft: &mut DialogueDraft,
        edit: impl FnOnce(&mut DialogueDraft) -> Result<()>,
    ) -> Result<()> {
        let mut local = self.load_dialogue_local()?;
        let mut current = local
            .drafts
            .get(&draft.id)
            .cloned()
            .ok_or("This dialogue draft is unavailable; your other drafts are kept.")?;
        if current.revision != draft.revision {
            return Err("A newer version of this dialogue draft is already saved. Reopen it before editing.".into());
        }
        if current.committed_request_id.is_some() || local.outbox.contains_key(&draft.id) {
            return Err("This message is already saved for sending. Continue from a reply or start a new question; its original words are immutable.".into());
        }
        edit(&mut current)?;
        current.revision = bump(current.revision)?;
        validate_draft(&current)?;
        local.drafts.insert(current.id, current.clone());
        self.save_dialogue_local(&local)?;
        *draft = current;
        Ok(())
    }
    pub(crate) fn save_dialogue_draft(
        &self,
        draft: &mut DialogueDraft,
        text: String,
    ) -> Result<()> {
        if text.len() > DIALOGUE_MAX_TEXT_BYTES {
            return Err(
                "Keep one dialogue message under 2 KiB. Your earlier draft is kept.".into(),
            );
        }
        self.edit_dialogue_draft(draft, |current| {
            if current.text != text {
                current.confirmed_audio = None;
            }
            current.text = text;
            Ok(())
        })
    }
    pub(crate) fn set_dialogue_options(
        &self,
        draft: &mut DialogueDraft,
        action: DialogueAction,
        correction: DialogueCorrection,
    ) -> Result<()> {
        self.edit_dialogue_draft(draft, |current| {
            current.action = action;
            current.correction = correction;
            Ok(())
        })
    }
    pub(crate) fn set_dialogue_config(
        &self,
        draft: &mut DialogueDraft,
        config: DialogueConfig,
    ) -> Result<()> {
        self.edit_dialogue_draft(draft, |current| {
            if current.parent_reply_id.is_some() {
                return Err(
                    "A conversation's scene stays fixed. Start a new scene to change it.".into(),
                );
            }
            current.config = config;
            Ok(())
        })
    }
    pub(crate) fn clear_dialogue_audio(&self, draft: &mut DialogueDraft) -> Result<()> {
        self.edit_dialogue_draft(draft, |current| {
            current.confirmed_audio = None;
            Ok(())
        })
    }
    pub(crate) fn adopt_dialogue_audio(
        &mut self,
        draft: &mut DialogueDraft,
        text: String,
        source: DialogueConfirmedAudioRef,
    ) -> Result<()> {
        self.refresh()?;
        let words = self
            .library
            .dialogue_confirmed_input(draft.thread_id, draft.id, &source)
            .map_err(|e| e.to_string())?;
        if words != text {
            return Err(
                "Those words do not match this message's exact confirmed recording.".into(),
            );
        }
        self.edit_dialogue_draft(draft, |current| {
            current.text = text;
            current.confirmed_audio = Some(source);
            Ok(())
        })
    }
    fn dialogue_request_from_draft(&self, draft: &DialogueDraft) -> Result<TeacherDialogueRequest> {
        validate_draft(draft)?;
        if let Some(anchor) = &draft.anchor {
            let canonical =
                teacher_dialogue_anchor(&self.library, anchor.document_id, anchor.focus.clone())
                    .map_err(|e| e.to_string())?;
            if canonical != *anchor {
                return Err(
                    "This dialogue's source snapshot no longer matches the exact saved teaching."
                        .into(),
                );
            }
        }
        if let Some(source) = &draft.confirmed_audio {
            if self
                .library
                .dialogue_confirmed_input(draft.thread_id, draft.id, source)
                .map_err(|e| e.to_string())?
                != draft.text
            {
                return Err(
                    "Confirm this message's exact recorded words again before sending.".into(),
                );
            }
        }
        let path = draft
            .parent_reply_id
            .map(|parent| {
                self.library
                    .dialogue_path(parent)
                    .map_err(|e| e.to_string())
            })
            .transpose()?;
        if path
            .as_ref()
            .is_some_and(|p| !p.is_complete() || p.thread_id != draft.thread_id)
        {
            return Err(
                "This branch is missing history or source evidence. Sync it before sending.".into(),
            );
        }
        let messages = path.as_ref().map_or(&[][..], |p| p.messages.as_slice());
        if messages.len() % 2 != 0 {
            return Err("The selected parent is not a complete teacher reply.".into());
        }
        let mut start = messages.len().saturating_sub(DIALOGUE_MAX_ANCESTORS);
        if start % 2 != 0 {
            start += 1;
        }
        loop {
            let ancestors = messages[start..]
                .iter()
                .map(|message| DialogueAncestor {
                    message_id: message.id(),
                    role: message.role(),
                    text: message.body(),
                })
                .collect::<Vec<_>>();
            let request = TeacherDialogueRequest {
                request_id: draft.id,
                thread_id: draft.thread_id,
                parent_reply_id: draft.parent_reply_id,
                config: draft.config.clone(),
                action: draft.action,
                correction: draft.correction,
                anchor: draft.anchor.clone(),
                ancestors,
                omitted_parent_id: if start == 0 {
                    None
                } else {
                    Some(messages[start - 1].id())
                },
                omitted_count: start,
                text: draft.text.clone(),
                confirmed_audio: draft.confirmed_audio.clone(),
            };
            let history_size = serde_json::to_vec(&request.ancestors)
                .map_err(|e| e.to_string())?
                .len();
            let size = serde_json::to_vec(&request)
                .map_err(|e| e.to_string())?
                .len();
            if history_size <= DIALOGUE_MAX_HISTORY_BYTES && size <= DIALOGUE_MAX_REQUEST_BYTES {
                request.validate().map_err(|e| e.to_string())?;
                let event = Event::new(
                    self.device,
                    now_ms(),
                    Fact::TeacherDialogueUserRecorded(request.clone()),
                )
                .map_err(|e| e.to_string())?;
                let mut checked = self.library.clone();
                checked.merge(&[event]).map_err(|e| e.to_string())?;
                return Ok(request);
            }
            if messages.len().saturating_sub(start) <= 2 {
                return Err("This exact message and its required context exceed the dialogue limit. Shorten the question or start a new anchored conversation; your draft is kept.".into());
            }
            start += 2;
        }
    }
    pub(crate) fn prepare_dialogue_turn(&mut self, draft: &mut DialogueDraft) -> Result<Uuid> {
        self.refresh()?;
        let mut local = self.load_dialogue_local()?;
        let current = local
            .drafts
            .get(&draft.id)
            .cloned()
            .ok_or("This dialogue draft is unavailable.")?;
        if current.revision != draft.revision {
            return Err("A newer dialogue draft is saved. Reopen it before sending.".into());
        }
        if let Some(id) = current.committed_request_id {
            if self.library.dialogue_user(id).is_none() {
                return Err("The saved message is missing from this library. Sync or recover its pending commit first.".into());
            }
            *draft = current;
            return Ok(id);
        }
        let event = if let Some(entry) = local.outbox.get(&current.id) {
            let frozen = request_of(entry)?;
            if entry.origin_revision != current.revision
                || frozen.text != current.text
                || frozen.config != current.config
                || frozen.action != current.action
                || frozen.correction != current.correction
                || frozen.anchor != current.anchor
                || frozen.confirmed_audio != current.confirmed_audio
            {
                return Err("The pending commit and local draft differ. Both were kept for recovery; neither was sent.".into());
            }
            entry.event.clone()
        } else {
            self.compact_dialogue_local(&mut local);
            if local.outbox.len() >= MAX_OUTBOX {
                return Err("Eight dialogue messages are already pending here. Finish or retry one before sending another; this draft is kept.".into());
            }
            let request = self.dialogue_request_from_draft(&current)?;
            let mut event = Event::new(
                self.device,
                now_ms(),
                Fact::TeacherDialogueUserRecorded(request),
            )
            .map_err(|e| e.to_string())?;
            event.id = Uuid::new_v5(&current.id, b"teacher-dialogue-user-event-v1");
            local.outbox.insert(
                current.id,
                OutboxEntry {
                    event: event.clone(),
                    origin_revision: current.revision,
                    status: DialogueDeliveryStatus::PendingCommit,
                    dispatch_id: None,
                    runtime_instance: None,
                    problem: None,
                },
            );
            self.save_dialogue_local(&local)?; // Reservation survives a later fact/journal failure.
            event
        };
        self.repo.append(&[event])?;
        self.refresh()?;
        let mut committed = current;
        committed.committed_request_id = Some(committed.id);
        committed.revision = bump(committed.revision)?;
        local.drafts.insert(committed.id, committed.clone());
        local
            .outbox
            .get_mut(&committed.id)
            .ok_or_else(invalid_local)?
            .status = DialogueDeliveryStatus::Ready;
        local.selection_epoch = bump(local.selection_epoch)?;
        self.save_dialogue_local(&local)?;
        *draft = committed;
        Ok(draft.id)
    }
    pub(crate) fn dialogue_outbox(&self) -> Result<Vec<DialogueOutboxSummary>> {
        Ok(self
            .load_dialogue_local()?
            .outbox
            .into_iter()
            .map(|(request_id, entry)| DialogueOutboxSummary {
                request_id,
                status: entry.status,
                problem: entry.problem,
            })
            .collect())
    }
    pub(crate) fn begin_dialogue_dispatch(&mut self, request_id: Uuid) -> Result<DialogueDispatch> {
        self.refresh()?;
        let request = self
            .library
            .dialogue_user(request_id)
            .cloned()
            .ok_or("Save the learner message before sending it.")?;
        request.validate().map_err(|e| e.to_string())?;
        if !self
            .library
            .dialogue_path(request_id)
            .map_err(|e| e.to_string())?
            .is_complete()
        {
            return Err("Sync the exact conversation source, ancestors and confirmed words before sending this saved message.".into());
        }
        let mut local = self.load_dialogue_local()?;
        if local.outbox.get(&request_id).is_some_and(|e| {
            e.status == DialogueDeliveryStatus::Sending
                && e.runtime_instance == Some(runtime_instance())
        }) {
            return Err("This saved message is already waiting for a reply.".into());
        }
        if !local.outbox.contains_key(&request_id) {
            self.compact_dialogue_local(&mut local);
            if local.outbox.len() >= MAX_OUTBOX {
                return Err("Finish an existing pending message before retrying another.".into());
            }
            let event=self.library.events.values().find(|e|matches!(&e.fact,Fact::TeacherDialogueUserRecorded(r)if r.request_id==request_id)).cloned().ok_or("The exact saved user event is unavailable.")?;
            let revision = local
                .drafts
                .get(&request_id)
                .map_or(1, |draft| draft.revision);
            local.outbox.insert(
                request_id,
                OutboxEntry {
                    event,
                    origin_revision: revision,
                    status: DialogueDeliveryStatus::Ready,
                    dispatch_id: None,
                    runtime_instance: None,
                    problem: None,
                },
            );
        }
        if request_of(local.outbox.get(&request_id).unwrap())? != &request {
            return Err("The queued request differs from its immutable saved message. Both were kept for recovery.".into());
        }
        let draft = local.active.and_then(|id| local.drafts.get(&id));
        let dispatch = DialogueDispatch {
            request,
            dispatch_id: Uuid::new_v4(),
            draft_id: draft.map(|d| d.id),
            draft_revision: draft.map(|d| d.revision),
            selection_epoch: local.selection_epoch,
        };
        let entry = local.outbox.get_mut(&request_id).unwrap();
        entry.status = DialogueDeliveryStatus::Sending;
        entry.dispatch_id = Some(dispatch.dispatch_id);
        entry.runtime_instance = Some(runtime_instance());
        entry.problem = None;
        self.save_dialogue_local(&local)?;
        Ok(dispatch)
    }
    pub(crate) fn keep_dialogue_reply(
        &mut self,
        dispatch: &DialogueDispatch,
        response: &TeacherDialogueResponse,
    ) -> Result<Uuid> {
        self.refresh()?;
        if response.request != dispatch.request
            || self.library.dialogue_user(dispatch.request.request_id) != Some(&dispatch.request)
        {
            return Err("This reply belongs to another saved message. Your conversation and drafts are kept.".into());
        }
        response.validate().map_err(|e| e.to_string())?;
        let reply_id = teacher_dialogue_reply_id(response);
        if let Some(old) = self.library.dialogue_reply(reply_id) {
            if old != response {
                return Err(
                    "A teacher reply identity changed its contents. The earlier reply is kept."
                        .into(),
                );
            }
        } else {
            let event = Event::new(
                self.device,
                now_ms(),
                Fact::TeacherDialogueReplyRecorded(response.clone()),
            )
            .map_err(|e| e.to_string())?;
            self.repo.append(&[event])?;
            self.refresh()?;
        }
        let mut local = self.load_dialogue_local()?;
        let matching = local
            .outbox
            .get(&dispatch.request.request_id)
            .is_some_and(|entry| entry.dispatch_id == Some(dispatch.dispatch_id));
        let advance = matching
            && local.active == dispatch.draft_id
            && local.selection_epoch == dispatch.selection_epoch
            && local
                .active
                .and_then(|id| local.drafts.get(&id))
                .is_some_and(|draft| {
                    Some(draft.revision) == dispatch.draft_revision
                        && draft.committed_request_id == Some(dispatch.request.request_id)
                });
        if matching {
            local.outbox.remove(&dispatch.request.request_id);
        }
        self.save_dialogue_local(&local)?;
        if advance {
            self.open_dialogue_reply(reply_id)?;
        }
        Ok(reply_id)
    }
    pub(crate) fn fail_dialogue_dispatch(
        &self,
        dispatch: &DialogueDispatch,
        problem: String,
    ) -> Result<()> {
        let mut local = self.load_dialogue_local()?;
        if let Some(entry) = local.outbox.get_mut(&dispatch.request.request_id) {
            if entry.dispatch_id == Some(dispatch.dispatch_id) {
                entry.status = DialogueDeliveryStatus::OutcomeUnknown;
                entry.problem = Some(problem.chars().take(512).collect());
                self.save_dialogue_local(&local)?;
            }
        }
        Ok(())
    }
    pub(crate) fn reconcile_dialogue(&mut self) -> Result<DialogueRecoveryReport> {
        self.refresh()?;
        let mut local = self.load_dialogue_local()?;
        let mut report = DialogueRecoveryReport::default();
        for (id, entry) in &mut local.outbox {
            if entry.status == DialogueDeliveryStatus::PendingCommit {
                match self.repo.append(&[entry.event.clone()]) {
                    Ok(()) => {
                        entry.status = DialogueDeliveryStatus::Ready;
                        report.committed += 1;
                        if let Some(draft) = local.drafts.get_mut(id) {
                            if draft.revision == entry.origin_revision
                                && draft.committed_request_id.is_none()
                            {
                                draft.committed_request_id = Some(*id);
                                draft.revision = bump(draft.revision)?;
                            } else if draft.committed_request_id != Some(*id)
                                && report.diagnostics.len() < 8
                            {
                                report.diagnostics.push("A pending message was recovered; a changed local draft was preserved separately.".into());
                            }
                        }
                    }
                    Err(problem) => {
                        if report.diagnostics.len() < 8 {
                            report.diagnostics.push(format!("A saved dialogue message is still awaiting local recovery: {problem}"));
                        }
                    }
                }
            }
            if entry.status == DialogueDeliveryStatus::Sending
                && entry.runtime_instance != Some(runtime_instance())
            {
                entry.status = DialogueDeliveryStatus::OutcomeUnknown;
                entry.problem=Some("The previous call was interrupted. Its outcome is unknown; retry only when you choose to send the same saved request again.".into());
                report.interrupted += 1;
            }
        }
        self.refresh()?;
        self.compact_dialogue_local(&mut local);
        self.save_dialogue_local(&local)?;
        Ok(report)
    }
}
