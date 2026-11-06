//! Ordinary dialogue is an immutable branching graph, never a practice attempt or mastery grade.
use crate::*;
use rufren_tutor_contract::{
    DialogueAction, DialogueRole, TeacherDialogueAnchor, TeacherDialogueContent,
    TeacherDialogueFocus, TeacherDialogueRequest, TeacherDialogueResponse, TeacherRequest,
    TutorExample,
};

pub const TEACHER_DIALOGUE_SNAPSHOT_VERSION: u32 = 1;
pub const TEACHER_DIALOGUE_ANCHOR_BYTES: usize = 8192;

/// Empty StartScene is an explicit learner action, not a fictional learner utterance.
pub fn teacher_dialogue_user_body(request: &TeacherDialogueRequest) -> String {
    if request.action == DialogueAction::StartScene {
        if request.text.is_empty() {
            "[Learner action: start the scene]".into()
        } else {
            format!("[Learner action: start the scene]\n{}", request.text)
        }
    } else {
        request.text.clone()
    }
}

pub fn teacher_dialogue_reply_id(response: &TeacherDialogueResponse) -> Uuid {
    Uuid::new_v5(
        &Uuid::NAMESPACE_URL,
        format!(
            "rufren:teacher-dialogue-reply:{}:{}:{}",
            response.provenance.provider,
            response.provenance.model,
            response.provenance.response_id
        )
        .as_bytes(),
    )
}

/// Only explicit spoken chunks; never scrape or split prose into presumed utterances.
pub fn teacher_dialogue_reply_chunks(response: &TeacherDialogueResponse) -> Vec<&TutorExample> {
    response
        .content
        .utterance
        .iter()
        .chain(response.content.next_question.iter())
        .collect()
}

/// Canonical plain assistant message for a bounded ancestor suffix. No recursive request JSON.
pub fn teacher_dialogue_assistant_body(content: &TeacherDialogueContent) -> String {
    let mut parts = Vec::new();
    if let Some(example) = &content.utterance {
        parts.push(format!(
            "{}\n{}\n{}",
            example.russian, example.english, example.note
        ));
    }
    if let Some(explanation) = &content.explanation_english {
        parts.push(explanation.clone());
    }
    if let Some(c) = &content.correction {
        parts.push(format!(
            "Correction: {} → {}\n{}",
            c.original, c.replacement, c.reason
        ));
    }
    if let Some(question) = &content.next_question {
        parts.push(format!(
            "{}\n{}\n{}",
            question.russian, question.english, question.note
        ));
    }
    parts.join("\n\n")
}

/// A deterministic source projection, not a generated summary. Truncation is explicit, UTF-8
/// safe, and byte-bounded; archive identity and focus remain separate typed fields.
pub fn teacher_dialogue_anchor(
    library: &Library,
    document_id: Uuid,
    focus: TeacherDialogueFocus,
) -> Result<TeacherDialogueAnchor, Error> {
    teacher_dialogue_anchor_in(&library.events, document_id, focus)
}
fn teacher_dialogue_anchor_in(
    events: &BTreeMap<Uuid, Event>,
    document_id: Uuid,
    focus: TeacherDialogueFocus,
) -> Result<TeacherDialogueAnchor, Error> {
    let invalid = || Error::Invalid("that dialogue source or focus is unavailable".into());
    let document = events
        .values()
        .find_map(|e| match &e.fact {
            Fact::TeacherDocumentRecorded(d) if d.document_id == document_id => Some(d),
            _ => None,
        })
        .ok_or_else(invalid)?;
    let answer = &document.response;
    let mut snapshot = String::from("Rufren saved teaching source v1\n");
    match &answer.request {
        TeacherRequest::TeachLesson(r) => snapshot.push_str(&format!(
            "Lesson: {}\nObjective: {}\n",
            r.title, r.objective
        )),
        TeacherRequest::AnswerFeedback(r) => {
            snapshot.push_str(&format!("Feedback task: {}\n", r.task))
        }
    }
    match &focus {
        TeacherDialogueFocus::Explanation => {
            snapshot.push_str(&format!("Explanation:\n{}", answer.content.explanation))
        }
        TeacherDialogueFocus::Practice { index } => {
            let e = answer.content.practice.get(*index).ok_or_else(invalid)?;
            snapshot.push_str(&format!("Practice {}\nPrompt: {}\n", index + 1, e.prompt));
            if let Some(skill) = &e.skill {
                snapshot.push_str(&format!(
                    "Skill: {}\n",
                    serde_json::to_string(skill).map_err(|_| invalid())?
                ));
            }
            if let Some(r) = &e.reference_russian {
                snapshot.push_str(&format!("Reference Russian: {r}\n"));
            }
            if let Some(e) = &e.reference_english {
                snapshot.push_str(&format!("Reference meaning: {e}\n"));
            }
            if let Some(r) = &e.expected_response {
                snapshot.push_str(&format!(
                    "Example response (alternatives may be valid): {r}\n"
                ));
            }
            snapshot.push_str(&format!("Teaching note: {}", e.teaching_note));
        }
        TeacherDialogueFocus::Step { step_id } => {
            let step = answer
                .content
                .lesson_flow
                .as_ref()
                .and_then(|f| f.steps.iter().find(|s| s.id == *step_id))
                .ok_or_else(invalid)?;
            snapshot.push_str(&format!(
                "Step: {}\nGoal: {}\nOrientation: {}\n",
                step.id, step.goal, step.orientation
            ));
            if let Some(detail) = &step.detail {
                snapshot.push_str(&format!("Detail: {detail}\n"));
            }
            for e in &step.examples {
                let e = answer
                    .content
                    .examples
                    .get(e.example_index)
                    .ok_or_else(invalid)?;
                snapshot.push_str(&format!(
                    "Example: {}\nMeaning: {}\nNote: {}\n",
                    e.russian, e.english, e.note
                ));
            }
            if let Some(index) = step.practice_index {
                let practice = answer.content.practice.get(index).ok_or_else(invalid)?;
                snapshot.push_str(&format!("Practice {}: {}\n", index + 1, practice.prompt));
            }
        }
        TeacherDialogueFocus::Feedback => {
            let TeacherRequest::AnswerFeedback(r) = &answer.request else {
                return Err(invalid());
            };
            snapshot.push_str(&format!(
                "Question: {}\nLearner response: {}\n",
                r.prompt, r.response
            ));
            if let Some(reference) = &r.reference {
                snapshot.push_str(&format!("Reference: {reference}\n"));
            }
            snapshot.push_str(&format!(
                "Teacher explanation:\n{}\n",
                answer.content.explanation
            ));
            if let Some(f) = &answer.content.feedback {
                if let Some(s) = &f.suggested_response {
                    snapshot.push_str(&format!("Suggested response: {s}\n"));
                }
                for c in &f.corrections {
                    snapshot.push_str(&format!(
                        "Correction: {} → {}\n{}\n",
                        c.original, c.replacement, c.reason
                    ));
                }
            }
        }
    }
    if snapshot.len() > TEACHER_DIALOGUE_ANCHOR_BYTES {
        const MARKER: &str = "\n[Source truncated at this UTF-8 boundary]";
        let mut boundary = TEACHER_DIALOGUE_ANCHOR_BYTES - MARKER.len();
        while !snapshot.is_char_boundary(boundary) {
            boundary -= 1;
        }
        snapshot.truncate(boundary);
        snapshot.push_str(MARKER);
    }
    Ok(TeacherDialogueAnchor {
        document_id,
        focus,
        snapshot_version: TEACHER_DIALOGUE_SNAPSHOT_VERSION,
        snapshot,
    })
}

pub(crate) fn validate_dialogue_fact(event: &Event) -> Result<(), Error> {
    let invalid = || Error::Invalid("invalid dialogue request or response".into());
    match &event.fact {
        Fact::TeacherDialogueUserRecorded(r) => r.validate().map_err(|_| invalid())?,
        Fact::TeacherDialogueReplyRecorded(r) => {
            r.validate().map_err(|_| invalid())?;
            if serde_json::to_vec(r).map_err(|_| invalid())?.len() > 131072 {
                return Err(invalid());
            }
        }
        _ => {}
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TeacherDialogueMessage<'a> {
    User(&'a TeacherDialogueRequest),
    Reply(&'a TeacherDialogueResponse),
}
impl<'a> TeacherDialogueMessage<'a> {
    pub fn id(self) -> Uuid {
        match self {
            Self::User(r) => r.request_id,
            Self::Reply(r) => teacher_dialogue_reply_id(r),
        }
    }
    pub fn thread_id(self) -> Uuid {
        self.request().thread_id
    }
    pub fn request(self) -> &'a TeacherDialogueRequest {
        match self {
            Self::User(r) => r,
            Self::Reply(r) => &r.request,
        }
    }
    pub fn parent(self) -> Option<Uuid> {
        match self {
            Self::User(r) => r.parent_reply_id,
            Self::Reply(r) => Some(r.request.request_id),
        }
    }
    pub fn body(self) -> String {
        match self {
            Self::User(r) => teacher_dialogue_user_body(r),
            Self::Reply(r) => teacher_dialogue_assistant_body(&r.content),
        }
    }
    pub fn role(self) -> DialogueRole {
        match self {
            Self::User(_) => DialogueRole::User,
            Self::Reply(_) => DialogueRole::Assistant,
        }
    }
}
fn index(
    events: &BTreeMap<Uuid, Event>,
) -> Result<BTreeMap<Uuid, TeacherDialogueMessage<'_>>, Error> {
    let mut messages = BTreeMap::new();
    for event in events.values() {
        let message = match &event.fact {
            Fact::TeacherDialogueUserRecorded(r) => TeacherDialogueMessage::User(r),
            Fact::TeacherDialogueReplyRecorded(r) => TeacherDialogueMessage::Reply(r),
            _ => continue,
        };
        if messages
            .insert(message.id(), message)
            .is_some_and(|old| old != message)
        {
            return Err(Error::Invalid(
                "dialogue logical message ID reused with different content".into(),
            ));
        }
    }
    Ok(messages)
}
pub(crate) fn dialogue_user_in(
    events: &BTreeMap<Uuid, Event>,
    id: Uuid,
) -> Option<&TeacherDialogueRequest> {
    events.values().find_map(|e| match &e.fact {
        Fact::TeacherDialogueUserRecorded(r) if r.request_id == id => Some(r),
        _ => None,
    })
}
pub(crate) fn dialogue_reply_in(
    events: &BTreeMap<Uuid, Event>,
    id: Uuid,
) -> Option<&TeacherDialogueResponse> {
    events.values().find_map(|e| match &e.fact {
        Fact::TeacherDialogueReplyRecorded(r) if teacher_dialogue_reply_id(r) == id => Some(r),
        _ => None,
    })
}

pub(crate) fn validate_dialogue_graph(events: &BTreeMap<Uuid, Event>) -> Result<(), Error> {
    let messages = index(events)?;
    if messages.is_empty() {
        return Ok(());
    }
    let invalid = || {
        Error::Invalid(
            "dialogue branch, source, or frozen context does not match its known history".into(),
        )
    };
    for message in messages.values().copied() {
        let r = message.request();
        if r.request_id == r.thread_id {
            if r.parent_reply_id.is_some()
                || !r.ancestors.is_empty()
                || r.omitted_parent_id.is_some()
                || r.omitted_count != 0
            {
                return Err(invalid());
            }
        } else if r.parent_reply_id.is_none() {
            return Err(invalid());
        }
        if let Some(root) = messages.get(&r.thread_id) {
            if !matches!(root, TeacherDialogueMessage::User(_))
                || root.request().request_id != root.request().thread_id
                || root.request().config != r.config
                || root.request().anchor != r.anchor
            {
                return Err(invalid());
            }
        }
        match message {
            TeacherDialogueMessage::User(_) => {
                if let Some(parent) = r.parent_reply_id.and_then(|id| messages.get(&id)) {
                    if !matches!(parent, TeacherDialogueMessage::Reply(_))
                        || parent.thread_id() != r.thread_id
                        || parent.request().config != r.config
                        || parent.request().anchor != r.anchor
                    {
                        return Err(invalid());
                    }
                }
            }
            TeacherDialogueMessage::Reply(reply) => {
                if let Some(user) = messages.get(&r.request_id) {
                    if !matches!(user, TeacherDialogueMessage::User(_))
                        || user.request() != &reply.request
                    {
                        return Err(invalid());
                    }
                }
            }
        }
        if let Some(anchor) = &r.anchor {
            let available = events.values().any(|e|matches!(&e.fact,Fact::TeacherDocumentRecorded(d) if d.document_id==anchor.document_id));
            if available
                && teacher_dialogue_anchor_in(events, anchor.document_id, anchor.focus.clone())?
                    != *anchor
            {
                return Err(invalid());
            }
        }
        validate_dialogue_input_source(r, events)?;
        if let Some(parent) = r.parent_reply_id {
            if r.ancestors
                .last()
                .is_none_or(|a| a.message_id != parent || a.role != DialogueRole::Assistant)
            {
                return Err(invalid());
            }
        }
        let mut predecessor = r.omitted_parent_id;
        let mut seen = BTreeSet::new();
        for ancestor in &r.ancestors {
            if !seen.insert(ancestor.message_id) || ancestor.message_id == r.request_id {
                return Err(invalid());
            }
            if let Some(known) = messages.get(&ancestor.message_id) {
                if known.thread_id() != r.thread_id
                    || known.role() != ancestor.role
                    || known.body() != ancestor.text
                    || known.parent() != predecessor
                {
                    return Err(invalid());
                }
            }
            predecessor = Some(ancestor.message_id);
        }
        if r.omitted_parent_id.is_none() != (r.omitted_count == 0) {
            return Err(invalid());
        }
        let mut omitted = r.omitted_parent_id;
        let mut count = 0;
        let mut visited = BTreeSet::new();
        while let Some(id) = omitted {
            if !visited.insert(id) {
                return Err(invalid());
            }
            let Some(node) = messages.get(&id) else {
                if count >= r.omitted_count {
                    return Err(invalid());
                }
                break;
            };
            count += 1;
            if count > r.omitted_count || node.thread_id() != r.thread_id {
                return Err(invalid());
            }
            omitted = node.parent();
        }
        if omitted.is_none() && count != r.omitted_count {
            return Err(invalid());
        }
    }
    // Iterative logical-node cycle detection; event-envelope IDs are independent receipt IDs.
    let mut done = BTreeSet::new();
    for root in messages.keys() {
        let mut active = BTreeSet::new();
        let mut cursor = Some(*root);
        while let Some(id) = cursor {
            if done.contains(&id) {
                break;
            }
            if !active.insert(id) {
                return Err(Error::Invalid("cyclic dialogue history".into()));
            }
            cursor = messages.get(&id).and_then(|m| m.parent());
        }
        done.extend(active);
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct TeacherDialoguePath<'a> {
    pub thread_id: Uuid,
    pub messages: Vec<TeacherDialogueMessage<'a>>,
    pub missing_parent: Option<Uuid>,
    pub missing_documents: Vec<Uuid>,
    pub incomplete_audio: Vec<Uuid>,
}
impl TeacherDialoguePath<'_> {
    pub fn is_complete(&self) -> bool {
        self.missing_parent.is_none()
            && self.missing_documents.is_empty()
            && self.incomplete_audio.is_empty()
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeacherDialogueThread {
    pub thread_id: Uuid,
    pub leaf_ids: Vec<Uuid>,
    pub user_messages: usize,
    pub teacher_replies: usize,
    pub incomplete: bool,
}
impl Library {
    pub fn dialogue_user(&self, id: Uuid) -> Option<&TeacherDialogueRequest> {
        dialogue_user_in(&self.events, id)
    }
    pub fn dialogue_reply(&self, id: Uuid) -> Option<&TeacherDialogueResponse> {
        dialogue_reply_in(&self.events, id)
    }
    pub fn dialogue_replies(&self, user: Uuid) -> Vec<(Uuid, &TeacherDialogueResponse)> {
        let mut replies = BTreeMap::new();
        for event in self.events.values() {
            if let Fact::TeacherDialogueReplyRecorded(r) = &event.fact {
                if r.request.request_id == user {
                    replies.insert(teacher_dialogue_reply_id(r), r);
                }
            }
        }
        replies.into_iter().collect()
    }
    pub fn dialogue_children(&self, reply: Uuid) -> Vec<&TeacherDialogueRequest> {
        let mut users = BTreeMap::new();
        for event in self.events.values() {
            if let Fact::TeacherDialogueUserRecorded(r) = &event.fact {
                if r.parent_reply_id == Some(reply) {
                    users.insert(r.request_id, r);
                }
            }
        }
        users.into_values().collect()
    }
    pub fn dialogue_path(&self, leaf: Uuid) -> Result<TeacherDialoguePath<'_>, Error> {
        let messages = index(&self.events)?;
        let first = messages
            .get(&leaf)
            .ok_or_else(|| Error::Invalid("dialogue message is unavailable".into()))?;
        let mut path = TeacherDialoguePath {
            thread_id: first.thread_id(),
            messages: Vec::new(),
            missing_parent: None,
            missing_documents: Vec::new(),
            incomplete_audio: Vec::new(),
        };
        let mut cursor = Some(leaf);
        let mut seen = BTreeSet::new();
        while let Some(id) = cursor {
            if !seen.insert(id) {
                return Err(Error::Invalid("cyclic dialogue history".into()));
            }
            let Some(message) = messages.get(&id).copied() else {
                path.missing_parent = Some(id);
                break;
            };
            let r = message.request();
            if let Some(a) = &r.anchor {
                if self.teacher_document(a.document_id).is_none() {
                    path.missing_documents.push(a.document_id);
                }
            }
            if let Some(audio) = &r.confirmed_audio {
                // Historical sent text stays bound to its exact adopted words even if a later
                // confirmation supersedes them. Active-head uniqueness is a new-Send guard.
                if !self
                    .events
                    .get(&audio.recording)
                    .is_some_and(|e| matches!(e.fact, Fact::DialogueAudioAttached(_)))
                    || !self
                        .events
                        .get(&audio.transcription)
                        .is_some_and(|e| matches!(e.fact, Fact::DialogueSpeechTranscribed(_)))
                    || !self
                        .events
                        .get(&audio.confirmation)
                        .is_some_and(|e| matches!(e.fact, Fact::DialogueTranscriptConfirmed(_)))
                {
                    path.incomplete_audio.push(r.request_id);
                }
            }
            path.messages.push(message);
            cursor = message.parent();
        }
        path.messages.reverse();
        path.missing_documents.sort();
        path.missing_documents.dedup();
        path.incomplete_audio.sort();
        path.incomplete_audio.dedup();
        Ok(path)
    }
    pub fn dialogue_threads(&self) -> Vec<TeacherDialogueThread> {
        let Ok(messages) = index(&self.events) else {
            return Vec::new();
        };
        let used: BTreeSet<_> = messages.values().filter_map(|m| m.parent()).collect();
        let mut threads = BTreeMap::<Uuid, TeacherDialogueThread>::new();
        for (id, message) in messages {
            let t = threads
                .entry(message.thread_id())
                .or_insert_with(|| TeacherDialogueThread {
                    thread_id: message.thread_id(),
                    leaf_ids: Vec::new(),
                    user_messages: 0,
                    teacher_replies: 0,
                    incomplete: false,
                });
            match message {
                TeacherDialogueMessage::User(_) => t.user_messages += 1,
                TeacherDialogueMessage::Reply(_) => t.teacher_replies += 1,
            }
            if !used.contains(&id) {
                t.leaf_ids.push(id);
                t.incomplete |= self.dialogue_path(id).map_or(true, |p| !p.is_complete());
            }
        }
        threads.into_values().collect()
    }
}
