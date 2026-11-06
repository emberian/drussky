//! Application interactions, independent of Dioxus rendering.
use crate::learning::{Preferences, SessionPlan};
use crate::repository::{Repository, Result, now_ms};
use rufren_core::*;
use std::rc::Rc;
use uuid::Uuid;

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Page {
    Today,
    Lessons,
    Library,
    Practice,
    Studio,
    Progress,
    Teacher,
    Settings,
}
impl Page {
    pub(crate) const ALL: [Self; 8] = [
        Self::Today,
        Self::Lessons,
        Self::Library,
        Self::Practice,
        Self::Studio,
        Self::Progress,
        Self::Teacher,
        Self::Settings,
    ];
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Today => "Today",
            Self::Lessons => "Lessons",
            Self::Library => "Library",
            Self::Practice => "Practice",
            Self::Studio => "Speech studio",
            Self::Progress => "Progress",
            Self::Teacher => "Teacher",
            Self::Settings => "Settings",
        }
    }
    pub(crate) fn icon(self) -> &'static str {
        match self {
            Self::Today => "◌",
            Self::Lessons => "✦",
            Self::Library => "▤",
            Self::Practice => "↗",
            Self::Studio => "≋",
            Self::Progress => "◷",
            Self::Teacher => "✎",
            Self::Settings => "⚙",
        }
    }
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct Editor {
    pub(crate) draft_id: Uuid,
    pub(crate) id: Option<Uuid>,
    pub(crate) parent: Option<Uuid>,
    pub(crate) title: String,
    pub(crate) russian: String,
    pub(crate) translation: String,
    pub(crate) source: String,
    #[serde(default)]
    pub(crate) pending_save: Option<Event>,
}
impl Default for Editor {
    fn default() -> Self {
        Self {
            draft_id: Uuid::new_v4(),
            id: None,
            parent: None,
            title: String::new(),
            russian: String::new(),
            translation: String::new(),
            source: String::new(),
            pending_save: None,
        }
    }
}
#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) repo: Rc<Repository>,
    pub(crate) device: Uuid,
    pub(crate) library: Library,
    pub(crate) page: Page,
    pub(crate) session: Option<Session>,
    pub(crate) editor: Editor,
    pub(crate) notice: String,
    pub(crate) endpoint: String,
    pub(crate) token: String,
    pub(crate) syncing: bool,
    pub(crate) teacher_busy: bool,
    pub(crate) speech_busy: bool,
    pub(crate) recognition_job: Option<(Uuid, String)>,
    pub(crate) recognition_cancel_requested: bool,
    pub(crate) transfer: String,
    pub(crate) content: Rc<rufren_content::ContentPack>,
    pub(crate) preferences: Preferences,
    pub(crate) plan: Option<SessionPlan>,
    pub(crate) selected_lesson: Option<String>,
    pub(crate) studio_revision: Option<Uuid>,
}
impl PartialEq for AppState {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.repo, &other.repo)
    }
}
impl AppState {
    pub(crate) fn open() -> Result<Self> {
        let repo = Rc::new(Repository::open()?);
        #[allow(unused_mut)]
        let mut state = Self::from_repository(repo)?;
        #[cfg(not(target_arch = "wasm32"))]
        match crate::native_connection::load() {
            Ok(Some(connection)) => {
                state.endpoint = connection.endpoint;
                state.token = connection.token;
            }
            Ok(None) => {}
            Err(error) => state.notice = error,
        }
        Ok(state)
    }
    pub(crate) fn from_repository(repo: Rc<Repository>) -> Result<Self> {
        let device = repo.identity()?;
        let library = repo.library()?;
        let binding: Option<SyncBinding> = repo.get("sync")?;
        let session = repo.get::<Option<Session>>("session")?.flatten();
        let preferences = repo.get("preferences")?.unwrap_or_default();
        let plan: Option<SessionPlan> = repo.get("session_plan")?;
        let plan = plan.filter(|p| {
            session
                .as_ref()
                .is_some_and(|s: &Session| s.id == p.session)
        });
        let plan = if plan.is_some() {
            plan
        } else {
            session
                .as_ref()
                .map(|s| crate::drafts::recover_plan(&repo, s))
                .transpose()?
                .flatten()
        };
        let selected_lesson = repo.get("selected_lesson")?;
        let editor = crate::editor_drafts::load_editor(&repo)?;
        let content = Rc::new(rufren_content::load_embedded().map_err(|e| e.to_string())?);
        Ok(Self {
            repo,
            device,
            library,
            session,
            page: Page::Today,
            editor,
            notice: String::new(),
            endpoint: binding.map_or("http://127.0.0.1:8787".into(), |b| b.endpoint),
            token: String::new(),
            syncing: false,
            teacher_busy: false,
            speech_busy: false,
            recognition_job: None,
            recognition_cancel_requested: false,
            transfer: String::new(),
            content,
            preferences,
            plan,
            selected_lesson,
            studio_revision: None,
        })
    }
    pub(crate) fn navigate(&mut self, page: Page) -> Result<()> {
        if page == Page::Practice {
            self.reconcile_daily_plan(now_ms())?;
        }
        if matches!(
            page,
            Page::Library | Page::Lessons | Page::Studio | Page::Teacher
        ) {
            // A Teacher revisit has its own saved draft, separate from Session.
            // Persist support before changing pages; a failed write keeps the
            // learner here rather than silently losing the assistance record.
            self.teacher_revisit_mark_active_support(Cue::ReferenceMaterial)?;
            if let Some(mut session) = self.session.clone().filter(|s| s.phase == Phase::Retrieve) {
                if !session.cues.contains(&Cue::ReferenceMaterial) {
                    session.cues.push(Cue::ReferenceMaterial);
                    self.repo.set("session", &Some(&session))?;
                    self.session = Some(session);
                    self.notice="Reference material opened. This response will retain that support in its practice record.".into();
                }
            }
        }
        self.refresh()?;
        self.page = page;
        Ok(())
    }
    pub(crate) fn refresh(&mut self) -> Result<()> {
        self.library = self.repo.library()?;
        Ok(())
    }
    pub(crate) fn start(&mut self, revision: Uuid, skill: Skill) -> Result<()> {
        self.start_with_id(revision, skill, Uuid::new_v4())
    }
    /// A caller may durably reserve a session before exposing its practice.
    pub(crate) fn start_with_id(
        &mut self,
        revision: Uuid,
        skill: Skill,
        session_id: Uuid,
    ) -> Result<()> {
        if session_id.is_nil()
            || self
                .library
                .attempts()
                .any(|(_, a)| a.session == session_id)
        {
            return Err(
                "This practice identity is invalid or already completed. Its saved work is kept."
                    .into(),
            );
        }
        let passage = self
            .library
            .revision(revision)
            .ok_or("That passage revision is missing. Refresh or sync your library.")?;
        if passage.passage.translation.trim().is_empty() {
            return Err("Add the passage’s meaning before practicing, so you have something to compare your response with.".into());
        }
        self.archive_current()?;
        let session = Session {
            id: session_id,
            revision,
            skill,
            phase: Phase::Encounter,
            draft: String::new(),
            cues: vec![],
        };
        self.repo.set("session", &Some(&session))?;
        self.plan = None;
        self.session = Some(session);
        self.set_plan(None, None, "free-practice-v1")?;
        self.clear_active_teacher_revisit()?;
        self.page = Page::Practice;
        Ok(())
    }
    pub(crate) fn begin(&mut self) -> Result<()> {
        self.refresh()?;
        self.reconcile_daily_plan(now_ms())?;
        if self
            .active_teacher_revisit()?
            .is_some_and(|(_, draft)| draft.saved_attempt.is_none())
        {
            self.page = Page::Practice;
            return Ok(());
        }
        self.clear_active_teacher_revisit()?;
        if self
            .session
            .as_ref()
            .is_some_and(|s| s.phase != Phase::Complete)
        {
            self.page = Page::Practice;
            return Ok(());
        }
        let now = now_ms();
        let plan = self.prepare_daily_plan(now)?;
        if let Some(index) = plan.next_ready_index() {
            return self.start_daily_plan_item(index, now);
        }
        if !plan.items.is_empty() {
            self.page = Page::Today;
            return Ok(());
        }
        if !self
            .library
            .attempts()
            .any(|(event, _)| event.recorded_at_ms <= now)
        {
            let id = self
                .selected_lesson
                .clone()
                .or_else(|| self.content.lessons.first().map(|lesson| lesson.id.clone()))
                .ok_or("The lesson pack is empty")?;
            self.choose_lesson(&id)?;
            return self.navigate(Page::Teacher);
        }
        self.page = Page::Today;
        self.notice = "No returns in this session. Explore a lesson, have a conversation, or choose a passage freely.".into();
        Ok(())
    }

    pub(crate) fn save_passage(&mut self, resolve: bool) -> Result<()> {
        self.save_editor_draft(resolve)
    }
    pub(crate) fn phase(&mut self, phase: Phase) -> Result<()> {
        let mut s = self.session.clone().ok_or("No active practice")?;
        if !matches!(
            (s.phase, phase),
            (Phase::Encounter, Phase::Retrieve)
                | (Phase::Retrieve, Phase::Reflect)
                | (Phase::Reflect, Phase::Complete)
        ) {
            return Err("That practice step is not available yet.".into());
        }
        if phase == Phase::Reflect && s.draft.trim().is_empty() {
            return Err("Add a response before comparing it.".into());
        }
        if phase == Phase::Reflect {
            if s.skill == Skill::Speaking
                && self
                    .plan
                    .as_ref()
                    .is_none_or(|p| p.response_recording.is_none())
            {
                return Err("Save a spoken recording before reflecting on this attempt.".into());
            }
            if (s.skill == Skill::Listening || self.exercise().is_some_and(|e| e.requires_audio))
                && self.plan.as_ref().is_none_or(|p| p.reference_plays == 0)
            {
                return Err("Listen to the reference before reflecting on this attempt.".into());
            }
        }
        if phase == Phase::Retrieve {
            s.cues.clear();
        }
        s.phase = phase;
        self.repo.set("session", &Some(&s))?;
        self.session = Some(s);
        Ok(())
    }
    pub(crate) fn hint(&mut self) -> Result<()> {
        let mut s = self.session.clone().ok_or("No active practice")?;
        if s.phase != Phase::Retrieve {
            return Err("Hints belong to the response step.".into());
        }
        let cue = if s.skill != Skill::Reading {
            Cue::OriginalText
        } else {
            Cue::Translation
        };
        if !s.cues.contains(&cue) {
            s.cues.push(cue);
        }
        self.repo.set("session", &Some(&s))?;
        self.session = Some(s);
        Ok(())
    }
    pub(crate) fn record(&mut self, rating: SelfRating) -> Result<()> {
        self.refresh()?;
        let session = self.session.clone().ok_or("No active practice")?;
        if session.phase != Phase::Reflect {
            return Err("Finish your response before recording the attempt.".into());
        }
        let existing = self
            .library
            .attempts()
            .find(|(_, a)| a.session == session.id)
            .map(|(e, _)| e.clone());
        let recovered = existing.is_some();
        let event = if let Some(event) = existing {
            let Fact::AttemptRecorded(attempt) = &event.fact else {
                unreachable!()
            };
            if attempt.revision != session.revision || attempt.skill != session.skill {
                return Err(
                    "The saved response belongs to a different practice. Its history was kept."
                        .into(),
                );
            }
            event
        } else {
            Event::new(
                self.device,
                now_ms(),
                Fact::AttemptRecorded(Attempt {
                    session: session.id,
                    revision: session.revision,
                    skill: session.skill,
                    response: session.draft.clone(),
                    cues: session.cues.clone(),
                    self_rating: rating,
                    policy: self
                        .plan
                        .as_ref()
                        .map_or("free-practice-v1", |p| p.policy.as_str())
                        .into(),
                }),
            )
            .map_err(|e| e.to_string())?
        };
        let mut events = vec![event.clone()];
        // Browser journaling may persist a prefix before quota/interruption. Reconstruct missing
        // companions from the saved plan using the durable attempt's identity and timestamp.
        if self.library.contexts(event.id).is_empty() {
            if let Some(context) = self.attempt_context(event.id)? {
                let mut companion = Event::new(
                    event.device,
                    event.recorded_at_ms,
                    Fact::AttemptContextRecorded(context),
                )
                .map_err(|e| e.to_string())?;
                companion.id = Uuid::new_v5(&event.id, b"practice-context-v1");
                events.push(companion);
            }
        }
        if let Some(mut attachment) = self
            .plan
            .as_ref()
            .and_then(|p| p.response_recording.clone())
        {
            let target = AnnotationTarget::Attempt(event.id);
            if !self
                .library
                .recordings(target)
                .iter()
                .any(|(_, a)| a.asset_id == attachment.asset_id)
            {
                attachment.target = target;
                let mut companion = Event::new(
                    event.device,
                    event.recorded_at_ms,
                    Fact::RecordingAttached(attachment),
                )
                .map_err(|e| e.to_string())?;
                companion.id = Uuid::new_v5(&event.id, b"spoken-response-v1");
                events.push(companion);
            }
        }
        self.repo.append(&events)?;
        self.phase(Phase::Complete)?;
        self.refresh()?;
        self.notice = if recovered {
            "Your earlier saved reflection was recovered with its question and recording links."
        } else {
            "Practice saved. Give it time, then return to see what stays with you."
        }
        .into();
        Ok(())
    }
}
