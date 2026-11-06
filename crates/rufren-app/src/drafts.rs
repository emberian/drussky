//! Durable paused practice, including the exact question and support already used.
use crate::{
    learning::SessionPlan,
    model::{AppState, Page},
    repository::{Repository, Result, now_ms},
};
use rufren_core::{Phase, Session};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use uuid::Uuid;

const INDEX: &str = "paused_practice_ids";
fn key(id: Uuid) -> String {
    format!("paused_practice.{id}")
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PausedPractice {
    version: u32,
    pub(crate) session: Session,
    pub(crate) plan: Option<SessionPlan>,
    pub(crate) saved_at_ms: i64,
}

/// A resume can be interrupted between its two active metadata writes. Recover
/// the question by session identity without replacing a more recent response.
pub(crate) fn recover_plan(repo: &Repository, session: &Session) -> Result<Option<SessionPlan>> {
    let record = repo.get::<PausedPractice>(&key(session.id))?;
    match record {
        Some(record) if record.version == 1 && record.session.id == session.id => {
            Ok(record.plan.filter(|p| p.session == session.id))
        }
        Some(_) => Err(
            "The saved practice has an unsupported format. Update Rufren before resuming it."
                .into(),
        ),
        None => Ok(None),
    }
}

impl AppState {
    /// Complete this before replacing the active session. The record precedes its index;
    /// a failed write leaves the active session in place and prevents the replacement.
    pub(crate) fn archive_current(&self) -> Result<()> {
        let Some(session) = &self.session else {
            return Ok(());
        };
        if !matches!(session.phase, Phase::Retrieve | Phase::Reflect)
            || (session.draft.trim().is_empty()
                && session.cues.is_empty()
                && self
                    .plan
                    .as_ref()
                    .is_none_or(|p| p.reference_plays == 0 && p.response_recording.is_none()))
        {
            return Ok(());
        }
        let record = PausedPractice {
            version: 1,
            session: session.clone(),
            plan: self.plan.clone().filter(|p| p.session == session.id),
            saved_at_ms: now_ms(),
        };
        self.repo.set(&key(session.id), &record)?;
        let mut ids: BTreeSet<Uuid> = self.repo.get(INDEX)?.unwrap_or_default();
        if ids.insert(session.id) {
            self.repo.set(INDEX, &ids)?;
        }
        Ok(())
    }

    pub(crate) fn paused_practices(&self) -> Result<Vec<PausedPractice>> {
        let ids: BTreeSet<Uuid> = self.repo.get(INDEX)?.unwrap_or_default();
        let library = self.repo.library()?;
        let finished: BTreeSet<_> = library.attempts().map(|(_, a)| a.session).collect();
        let active = self.session.as_ref().map(|s| s.id);
        let mut records = Vec::new();
        for id in ids {
            if active == Some(id) || finished.contains(&id) {
                continue;
            }
            let Some(record) = self.repo.get::<PausedPractice>(&key(id))? else {
                return Err("A paused practice is missing. Your other saved work is kept.".into());
            };
            if record.version != 1
                || record.session.id != id
                || !matches!(record.session.phase, Phase::Retrieve | Phase::Reflect)
                || record.plan.as_ref().is_some_and(|p| p.session != id)
            {
                return Err("A paused practice has an unsupported format. Update Rufren before resuming it.".into());
            }
            records.push(record);
        }
        records.sort_by_key(|r| (std::cmp::Reverse(r.saved_at_ms), r.session.id));
        Ok(records)
    }

    pub(crate) fn resume_practice(&mut self, id: Uuid) -> Result<()> {
        self.flush_pending_reader_edits()?;
        let record = self
            .paused_practices()?
            .into_iter()
            .find(|r| r.session.id == id)
            .ok_or("That practice is already active, finished, or unavailable.")?;
        self.recover_pending_review_start(
            id,
            rufren_core::PracticeBatchKey::Passage {
                revision: record.session.revision,
                skill: record.session.skill,
            },
        )?;
        self.archive_current()?;
        // Keep the archived copies after resuming. If either active metadata write
        // fails, both drafts are still recoverable through the index on reopen.
        self.repo.set("session_plan", &record.plan)?;
        self.repo.set("session", &Some(&record.session))?;
        self.session = Some(record.session);
        self.plan = record.plan;
        self.clear_active_teacher_revisit()?;
        self.select_ordinary_practice_surface()?;
        self.page = Page::Practice;
        self.notice = "Practice resumed with your response and support kept.".into();
        Ok(())
    }
}
