//! Application interactions, independent of Dioxus rendering.
use crate::repository::{Repository, Result, now_ms};
use rufren_core::*;
use std::{collections::BTreeSet, rc::Rc};
use uuid::Uuid;

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Page {
    Today,
    Library,
    Practice,
    Studio,
    Progress,
    Settings,
}
impl Page {
    pub(crate) const ALL: [Self; 6] = [
        Self::Today,
        Self::Library,
        Self::Practice,
        Self::Studio,
        Self::Progress,
        Self::Settings,
    ];
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Today => "Today",
            Self::Library => "Library",
            Self::Practice => "Practice",
            Self::Studio => "Speech studio",
            Self::Progress => "Progress",
            Self::Settings => "Settings",
        }
    }
    pub(crate) fn icon(self) -> &'static str {
        match self {
            Self::Today => "◌",
            Self::Library => "▤",
            Self::Practice => "↗",
            Self::Studio => "≋",
            Self::Progress => "◷",
            Self::Settings => "⚙",
        }
    }
}
#[derive(Clone, Default)]
pub(crate) struct Editor {
    pub(crate) id: Option<Uuid>,
    pub(crate) parent: Option<Uuid>,
    pub(crate) title: String,
    pub(crate) russian: String,
    pub(crate) translation: String,
    pub(crate) source: String,
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
    pub(crate) transfer: String,
}
impl PartialEq for AppState {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.repo, &other.repo)
    }
}
impl AppState {
    pub(crate) fn open() -> Result<Self> {
        let repo = Rc::new(Repository::open()?);
        let device = repo.identity()?;
        let library = repo.library()?;
        let binding: Option<SyncBinding> = repo.get("sync")?;
        let session = repo.get::<Option<Session>>("session")?.flatten();
        Ok(Self {
            repo,
            device,
            library,
            session,
            page: Page::Today,
            editor: Editor::default(),
            notice: String::new(),
            endpoint: binding.map_or("http://127.0.0.1:8787".into(), |b| b.endpoint),
            token: String::new(),
            syncing: false,
            transfer: String::new(),
        })
    }
    pub(crate) fn refresh(&mut self) -> Result<()> {
        self.library = self.repo.library()?;
        Ok(())
    }
    pub(crate) fn save_session(&self) -> Result<()> {
        self.repo.set("session", &self.session)
    }
    pub(crate) fn start(&mut self, revision: Uuid, skill: Skill) -> Result<()> {
        let passage = self
            .library
            .revision(revision)
            .ok_or("That passage revision is missing. Refresh or sync your library.")?;
        if passage.passage.translation.trim().is_empty() {
            return Err("Add the passage’s meaning before practicing, so you have something to compare your response with.".into());
        }
        self.session = Some(Session {
            id: Uuid::new_v4(),
            revision,
            skill,
            phase: Phase::Encounter,
            draft: String::new(),
            cues: vec![],
        });
        self.save_session()?;
        self.page = Page::Practice;
        Ok(())
    }
    pub(crate) fn begin(&mut self) -> Result<()> {
        self.refresh()?;
        if self
            .session
            .as_ref()
            .is_some_and(|s| s.phase != Phase::Complete)
        {
            self.page = Page::Practice;
            return Ok(());
        }
        let revision = if let Some(id) = LeastPracticed.choose(&self.library) {
            id
        } else if self.library.heads().is_empty() {
            let event = starter(self.device, now_ms());
            let id = event.id;
            self.repo.append(&[event])?;
            self.refresh()?;
            id
        } else {
            return Err(
                "Resolve the competing passage revisions in your library before practicing them."
                    .into(),
            );
        };
        self.start(revision, Skill::Reading)
    }
    pub(crate) fn edit(&mut self, id: Uuid) {
        if let Some(r) = self.library.revision(id) {
            let p = &r.passage;
            self.editor = Editor {
                id: Some(p.id),
                parent: Some(id),
                title: p.title.clone(),
                russian: p.russian.clone(),
                translation: p.translation.clone(),
                source: p.source.clone(),
            };
            self.page = Page::Library;
        }
    }
    pub(crate) fn save_passage(&mut self, resolve: bool) -> Result<()> {
        self.refresh()?;
        let d = &self.editor;
        let id = d.id.unwrap_or_else(Uuid::new_v4);
        let parents = if resolve {
            self.library
                .heads()
                .get(&id)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .collect()
        } else {
            d.parent.into_iter().collect::<BTreeSet<_>>()
        };
        let event = Event::new(
            self.device,
            now_ms(),
            Fact::PassageRevised(Revision {
                passage: Passage {
                    id,
                    title: d.title.trim().into(),
                    russian: d.russian.trim().into(),
                    translation: d.translation.trim().into(),
                    source: d.source.trim().into(),
                    coach_reviewed: false,
                },
                parents,
            }),
        )
        .map_err(|e| e.to_string())?;
        let revision = event.id;
        self.repo.append(&[event])?;
        self.refresh()?;
        self.edit(revision);
        self.notice = "Passage saved on this device.".into();
        Ok(())
    }
    pub(crate) fn phase(&mut self, phase: Phase) -> Result<()> {
        let s = self.session.as_mut().ok_or("No active practice")?;
        if phase == Phase::Retrieve {
            s.cues.clear();
        }
        s.phase = phase;
        self.save_session()
    }
    pub(crate) fn hint(&mut self) -> Result<()> {
        let s = self.session.as_mut().ok_or("No active practice")?;
        let cue = if s.skill == Skill::Writing {
            Cue::OriginalText
        } else {
            Cue::Translation
        };
        if !s.cues.contains(&cue) {
            s.cues.push(cue);
        }
        self.save_session()
    }
    pub(crate) fn record(&mut self, rating: SelfRating) -> Result<()> {
        self.refresh()?;
        let s = self.session.clone().ok_or("No active practice")?;
        if s.phase != Phase::Reflect {
            return Err("Finish your response before recording the attempt.".into());
        }
        if !self.library.attempts().any(|(_, a)| a.session == s.id) {
            let event = Event::new(
                self.device,
                now_ms(),
                Fact::AttemptRecorded(Attempt {
                    session: s.id,
                    revision: s.revision,
                    skill: s.skill,
                    response: s.draft,
                    cues: s.cues,
                    self_rating: rating,
                    policy: LeastPracticed.id().into(),
                }),
            )
            .map_err(|e| e.to_string())?;
            self.repo.append(&[event])?;
        }
        self.phase(Phase::Complete)?;
        self.refresh()?;
        self.notice = "Practice saved. You can return to this passage whenever you like.".into();
        Ok(())
    }
}
