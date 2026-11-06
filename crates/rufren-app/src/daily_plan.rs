//! Workspace-local bounded activity plans. Only actual saved responses complete an item.
use crate::{
    model::{AppState, Page},
    repository::Result,
    teacher_revisit::proven_teacher_task_origin,
};
use rufren_core::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

const ACTIVE: &str = "daily_plan_active";
const SETTINGS: &str = "daily_plan_settings";
fn batch_policy() -> String {
    PRACTICE_BATCH_POLICY_ID.into()
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct DailyPlanSettings {
    pub response_budget: usize,
    pub quiet: bool,
}
impl Default for DailyPlanSettings {
    fn default() -> Self {
        Self {
            response_budget: 3,
            quiet: false,
        }
    }
}
impl DailyPlanSettings {
    fn validate(&self) -> Result<()> {
        if [1, 3, 5].contains(&self.response_budget) {
            Ok(())
        } else {
            Err("Choose 1, 3 or 5 responses for a short session.".into())
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DailyPlanItemState {
    Ready,
    Completed { attempt_id: Uuid },
    Skipped,
    Unavailable { reason: String },
}
#[derive(Debug, Clone)]
pub(crate) struct DailyPlanItem {
    pub index: usize,
    pub key: PracticeBatchKey,
    pub skill: Skill,
    pub title: String,
    pub reason: String,
    pub state: DailyPlanItemState,
}
#[derive(Debug, Clone)]
pub(crate) struct DailyPlan {
    pub id: Uuid,
    pub created_at_ms: i64,
    pub settings: DailyPlanSettings,
    pub items: Vec<DailyPlanItem>,
}
impl DailyPlan {
    pub(crate) fn is_finished(&self) -> bool {
        self.items.iter().all(|i| {
            matches!(
                i.state,
                DailyPlanItemState::Completed { .. } | DailyPlanItemState::Skipped
            )
        })
    }
    pub(crate) fn next_ready_index(&self) -> Option<usize> {
        self.items
            .iter()
            .position(|i| i.state == DailyPlanItemState::Ready)
    }
    pub(crate) fn completed_count(&self) -> usize {
        self.items
            .iter()
            .filter(|i| matches!(i.state, DailyPlanItemState::Completed { .. }))
            .count()
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredItem {
    key: PracticeBatchKey,
    skill: Skill,
    title: String,
    reason: String,
    baseline_attempt_ids: BTreeSet<Uuid>,
    /// Fresh IDs durably reserved before starting; these are intents, never completion evidence.
    started_sessions: BTreeMap<Uuid, i64>,
    #[serde(default)]
    start_pending: bool,
    skipped: bool,
    #[serde(default)]
    skip_cleanup_pending: bool,
    #[serde(default)]
    skip_session: Option<Uuid>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredPlan {
    version: u32,
    #[serde(default = "batch_policy")]
    policy: String,
    id: Uuid,
    created_at_ms: i64,
    settings: DailyPlanSettings,
    items: Vec<StoredItem>,
}
impl StoredPlan {
    fn validate(&self) -> Result<()> {
        self.settings.validate()?;
        let mut keys = BTreeSet::new();
        if self.policy != PRACTICE_BATCH_POLICY_ID
            || self.version != 1
            || self.id.is_nil()
            || self.created_at_ms < 0
            || self.items.len() > self.settings.response_budget
        {
            return Err(
                "This saved short session needs a compatible app; its work was kept.".into(),
            );
        }
        for item in &self.items {
            item.key.validate().map_err(|e| e.to_string())?;
            if item.started_sessions.len() > 1
                || item.start_pending && item.started_sessions.is_empty()
                || item.skip_cleanup_pending && !item.skipped
                || item.skip_session.is_some_and(|id| id.is_nil())
            {
                return Err("This short session has inconsistent recovery intents; saved responses were kept.".into());
            }
            if !keys.insert(item.key)
                || item.title.len() > 32000
                || item.reason.len() > 32000
                || item.baseline_attempt_ids.iter().any(Uuid::is_nil)
                || item
                    .started_sessions
                    .iter()
                    .any(|(id, t)| id.is_nil() || *t < self.created_at_ms)
                || matches!(item.key,PracticeBatchKey::Passage {skill,..} if skill!=item.skill)
            {
                return Err("This short session has inconsistent task bindings; its saved responses were kept.".into());
            }
        }
        Ok(())
    }
}
fn target_attempts(library: &Library, key: PracticeBatchKey) -> Vec<(&Event, &Attempt)> {
    library
        .attempts()
        .filter(|(event, a)| match key {
            PracticeBatchKey::Passage { revision, skill } => {
                a.revision == revision && a.skill == skill
            }
            PracticeBatchKey::Teacher {
                document_id,
                practice_index,
            } => {
                let contexts = library.contexts(event.id);
                contexts.len() == 1
                    && proven_teacher_task_origin(library, event.id, contexts[0].0.id).is_some_and(
                        |o| {
                            o.key.document_id == document_id
                                && o.key.practice_index == practice_index
                        },
                    )
            }
        })
        .collect()
}
fn completion(library: &Library, plan: &StoredPlan, item: &StoredItem, now: i64) -> Option<Uuid> {
    target_attempts(library, item.key)
        .into_iter()
        .filter(|(event, attempt)| {
            if item.baseline_attempt_ids.contains(&event.id)
                || event.recorded_at_ms > now
                || event.recorded_at_ms < plan.created_at_ms
            {
                return false;
            }
            // Neither a previously unknown imported attempt nor a response opened before this batch
            // counts merely because its completion timestamp happens to be later.
            item.started_sessions
                .get(&attempt.session)
                .is_some_and(|start| *start >= plan.created_at_ms && *start <= event.recorded_at_ms)
        })
        .min_by_key(|(e, _)| (e.recorded_at_ms, e.id))
        .map(|(e, _)| e.id)
}
fn active_teacher_key(state: &AppState) -> Result<Option<PracticeBatchKey>> {
    let Some(value) = state
        .repo
        .get::<serde_json::Value>("teacher_revisit_active")?
    else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let document = value
        .get("document_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok());
    let index = value
        .get("practice_index")
        .and_then(|v| v.as_u64())
        .and_then(|i| usize::try_from(i).ok());
    Ok(document
        .zip(index)
        .map(|(document_id, practice_index)| PracticeBatchKey::Teacher {
            document_id,
            practice_index,
        }))
}
impl AppState {
    pub(crate) fn daily_plan_settings(&self) -> Result<DailyPlanSettings> {
        let settings = self
            .repo
            .get::<DailyPlanSettings>(SETTINGS)?
            .unwrap_or_default();
        settings.validate()?;
        Ok(settings)
    }
    pub(crate) fn save_daily_plan_settings(&self, settings: DailyPlanSettings) -> Result<()> {
        settings.validate()?;
        self.repo.set(SETTINGS, &settings)
    }
    fn stored_daily_plan(&self) -> Result<Option<StoredPlan>> {
        let stored = self.repo.get::<StoredPlan>(ACTIVE)?;
        if let Some(p) = &stored {
            p.validate()?;
        }
        Ok(stored)
    }
    fn plan_candidates(&self, now: i64) -> Vec<(PracticeBatchCandidate, String)> {
        let mut last_skill = BTreeMap::<Skill, i64>::new();
        let mut last_source = BTreeMap::<PracticeBatchSource, i64>::new();
        for (event, a) in self
            .library
            .attempts()
            .filter(|(e, _)| e.recorded_at_ms <= now)
        {
            last_skill
                .entry(a.skill)
                .and_modify(|t| *t = (*t).max(event.recorded_at_ms))
                .or_insert(event.recorded_at_ms);
            let source = if self
                .library
                .events
                .get(&a.revision)
                .is_some_and(is_teacher_generated_reference)
            {
                let c = self.library.contexts(event.id);
                if c.len() == 1
                    && proven_teacher_task_origin(&self.library, event.id, c[0].0.id).is_some()
                {
                    Some(PracticeBatchSource::Teacher)
                } else {
                    None
                }
            } else if self
                .library
                .events
                .get(&a.revision)
                .is_some_and(is_reserved_practice_reference)
            {
                None
            } else if self.library.revision(a.revision).is_some() {
                Some(PracticeBatchSource::Passage)
            } else {
                None
            };
            if let Some(source) = source {
                last_source
                    .entry(source)
                    .and_modify(|t| *t = (*t).max(event.recorded_at_ms))
                    .or_insert(event.recorded_at_ms);
            }
        }
        let mut candidates = Vec::new();
        for e in self
            .teacher_revisits(now)
            .entries
            .into_iter()
            .filter(|e| e.due)
        {
            candidates.push((
                PracticeBatchCandidate {
                    key: PracticeBatchKey::Teacher {
                        document_id: e.key.document_id,
                        practice_index: e.key.practice_index,
                    },
                    skill: e.skill,
                    due_at_ms: e.state.due_at_ms,
                    last_skill_practice_ms: last_skill.get(&e.skill).copied(),
                    last_source_practice_ms: last_source
                        .get(&PracticeBatchSource::Teacher)
                        .copied(),
                },
                e.title,
            ));
        }
        for e in SpacedReview
            .queue(&self.library, now, &Skill::ALL)
            .into_iter()
            .filter(|e| e.due && e.state.attempts > 0)
        {
            let Some(revision) = self.library.revision(e.state.key.revision) else {
                continue;
            };
            if revision.passage.translation.trim().is_empty() {
                continue;
            }
            candidates.push((
                PracticeBatchCandidate {
                    key: PracticeBatchKey::Passage {
                        revision: e.state.key.revision,
                        skill: e.state.key.skill,
                    },
                    skill: e.state.key.skill,
                    due_at_ms: e.state.due_at_ms,
                    last_skill_practice_ms: last_skill.get(&e.state.key.skill).copied(),
                    last_source_practice_ms: last_source
                        .get(&PracticeBatchSource::Passage)
                        .copied(),
                },
                revision.passage.title.clone(),
            ));
        }
        candidates
    }
    fn daily_plan_title(&self, key: PracticeBatchKey, index: usize) -> String {
        match key {
            PracticeBatchKey::Passage { .. } => format!("Saved passage · item {}", index + 1),
            PracticeBatchKey::Teacher {
                document_id,
                practice_index,
            } => {
                let lesson = self.library.teacher_document(document_id).and_then(|d| {
                    match &d.response.request {
                        rufren_tutor::TeacherRequest::TeachLesson(r) => {
                            self.content.lesson(&r.lesson_id)
                        }
                        _ => None,
                    }
                });
                format!(
                    "{} · exercise {}",
                    lesson
                        .map(|l| l.title.as_str())
                        .unwrap_or("Saved Teacher practice"),
                    practice_index + 1
                )
            }
        }
    }
    pub(crate) fn daily_plan(&self, now: i64) -> Result<Option<DailyPlan>> {
        let Some(plan) = self.stored_daily_plan()? else {
            return Ok(None);
        };
        let available: BTreeSet<_> = self
            .plan_candidates(now)
            .into_iter()
            .map(|(c, _)| c.key)
            .collect();
        let mut items = Vec::new();
        for (index, item) in plan.items.iter().enumerate() {
            let older_draft = if let PracticeBatchKey::Teacher {
                document_id,
                practice_index,
            } = item.key
            {
                self.repo
                    .get::<crate::teacher_practice::TeacherPracticeDraft>(&format!(
                        "teacher_revisit:{document_id}:{practice_index}"
                    ))?
                    .is_some_and(|d| {
                        d.saved_attempt.is_none() && d.started_at_ms < plan.created_at_ms
                    })
            } else {
                false
            };
            let state = if let Some(attempt_id) = completion(&self.library, &plan, item, now) {
                DailyPlanItemState::Completed { attempt_id }
            } else if item.skipped {
                DailyPlanItemState::Skipped
            } else if older_draft {
                DailyPlanItemState::Unavailable {reason:"An earlier response for this task is unfinished. Its draft is kept; resume it separately or leave this item for later.".into()}
            } else if !available.contains(&item.key) {
                DailyPlanItemState::Unavailable {reason:"This exact return is no longer available or due after the latest saved history. Its planned place and earlier work are kept; skip it to finish this session.".into()}
            } else {
                DailyPlanItemState::Ready
            };
            items.push(DailyPlanItem {
                index,
                key: item.key,
                skill: item.skill,
                title: self.daily_plan_title(item.key, index),
                reason: item.reason.clone(),
                state,
            });
        }
        Ok(Some(DailyPlan {
            id: plan.id,
            created_at_ms: plan.created_at_ms,
            settings: plan.settings.clone(),
            items,
        }))
    }
    pub(crate) fn prepare_daily_plan(&mut self, now: i64) -> Result<DailyPlan> {
        self.refresh()?;
        if let Some(plan) = self.daily_plan(now)? {
            // An empty selection is not a completed workload. Reconsider it on
            // an explicit start so material becoming due later can appear.
            if !plan.items.is_empty() {
                return Ok(plan);
            }
        }
        self.create_daily_plan(now)
    }
    fn create_daily_plan(&mut self, now: i64) -> Result<DailyPlan> {
        if now < 0 {
            return Err("The device clock cannot start a short session yet.".into());
        }
        let settings = self.daily_plan_settings()?;
        let candidates = self.plan_candidates(now);
        let raw: Vec<_> = candidates.iter().map(|(c, _)| c.clone()).collect();
        let selected = BoundedPracticeBatch
            .select(&raw, settings.response_budget, settings.quiet, now)
            .map_err(|e| e.to_string())?;
        let items=selected.into_iter().map(|c| {
            let title=candidates.iter().find(|(a,_)|a.key==c.key).map(|(_,t)|t.clone()).unwrap_or_default();
            let kind=match c.key {PracticeBatchKey::Teacher {..}=>"Teacher question",PracticeBatchKey::Passage {..}=>"passage"};
            let recent = c.last_skill_practice_ms.map(|time| {
                let days = now.saturating_sub(time) / DAY_MS;
                if days == 0 { "less than a day earlier".to_owned() }
                else if days == 1 { "a day earlier".to_owned() }
                else { format!("{days} days earlier") }
            }).unwrap_or_else(|| "not yet recorded".into());
            StoredItem {key:c.key,skill:c.skill,title,reason:format!("This {kind} was ready when you chose the session. Your latest saved {} response was {recent}. The selection makes room for different skills and materials.",c.skill.label().to_lowercase()),baseline_attempt_ids:target_attempts(&self.library,c.key).into_iter().map(|(e,_)|e.id).collect(),started_sessions:BTreeMap::new(),start_pending:false,skipped:false,skip_cleanup_pending:false,skip_session:None}
        }).collect();
        let stored = StoredPlan {
            version: 1,
            policy: batch_policy(),
            id: Uuid::new_v4(),
            created_at_ms: now,
            settings,
            items,
        };
        stored.validate()?;
        self.repo.set(ACTIVE, &stored)?;
        self.daily_plan(now)?
            .ok_or("The short session could not be reopened.".into())
    }
    pub(crate) fn new_daily_plan(&mut self, now: i64) -> Result<DailyPlan> {
        self.refresh()?;
        self.reconcile_daily_plan(now)?;
        if self.daily_plan(now)?.is_some_and(|p| !p.is_finished()) {
            return Err("Finish or skip the remaining items before starting a new short session. Pausing keeps this one.".into());
        }
        if self
            .session
            .as_ref()
            .is_some_and(|s| s.phase != Phase::Complete)
        {
            // Choosing a new batch is an explicit pause. Keep the response before
            // detaching its active pointer; failed storage never discards the draft.
            self.archive_current()?;
            self.repo.set("session", &None::<Session>)?;
            self.repo
                .set("session_plan", &None::<crate::learning::SessionPlan>)?;
            self.session = None;
            self.plan = None;
        }
        // Detach even if its source has not synced yet. Validating a dangling
        // active pointer must not block this explicit pause; its draft is separate.
        self.clear_active_teacher_revisit()?;
        self.select_ordinary_practice_surface()?;
        self.create_daily_plan(now)
    }
    pub(crate) fn skip_daily_plan_item(&mut self, index: usize) -> Result<()> {
        self.refresh()?;
        let projected = self
            .daily_plan(crate::repository::now_ms())?
            .ok_or("No short session is active.")?;
        if projected
            .items
            .get(index)
            .is_some_and(|i| matches!(i.state, DailyPlanItemState::Completed { .. }))
        {
            return Err("That response is already saved; it remains completed.".into());
        }
        let mut stored = self
            .stored_daily_plan()?
            .ok_or("No short session is active.")?;
        let item = stored
            .items
            .get_mut(index)
            .ok_or("That short-session item is unavailable.")?;
        if !item.skip_cleanup_pending {
            let active = match item.key {
                PracticeBatchKey::Passage { revision, skill } => self
                    .session
                    .as_ref()
                    .filter(|s| {
                        s.revision == revision && s.skill == skill && s.phase != Phase::Complete
                    })
                    .map(|s| s.id),
                PracticeBatchKey::Teacher { .. } => {
                    if active_teacher_key(self)? == Some(item.key) {
                        self.repo
                            .get::<serde_json::Value>("teacher_revisit_active")?
                            .and_then(|v| {
                                v.get("draft_id")
                                    .and_then(|id| id.as_str())
                                    .and_then(|id| Uuid::parse_str(id).ok())
                            })
                    } else {
                        None
                    }
                }
            };
            item.skipped = true;
            item.start_pending = false;
            item.skip_cleanup_pending = true;
            item.skip_session = active;
            self.repo.set(ACTIVE, &stored)?; // Save intent before detaching any recoverable draft.
        }
        self.page = Page::Today;
        self.reconcile_daily_plan(crate::repository::now_ms())
    }
    /// Finish interrupted start/skip effects before begin() resumes global active work.
    pub(crate) fn reconcile_daily_plan(&mut self, now: i64) -> Result<()> {
        let Some(mut stored) = self.stored_daily_plan()? else {
            return Ok(());
        };
        for index in 0..stored.items.len() {
            if stored.items[index].skip_cleanup_pending {
                self.page = Page::Today;
                let item = &stored.items[index];
                if let Some(id) = item.skip_session {
                    match item.key {
                        PracticeBatchKey::Passage { .. }
                            if self
                                .session
                                .as_ref()
                                .is_some_and(|s| s.id == id && s.phase != Phase::Complete) =>
                        {
                            self.archive_current()?;
                            self.repo.set("session", &None::<Session>)?;
                            self.repo
                                .set("session_plan", &None::<crate::learning::SessionPlan>)?;
                            self.session = None;
                            self.plan = None;
                        }
                        PracticeBatchKey::Teacher { .. } => {
                            let active = self
                                .repo
                                .get::<serde_json::Value>("teacher_revisit_active")?;
                            let same = active
                                .as_ref()
                                .and_then(|v| v.get("draft_id"))
                                .and_then(|v| v.as_str())
                                .and_then(|v| Uuid::parse_str(v).ok())
                                == Some(id);
                            if same {
                                self.clear_active_teacher_revisit()?;
                            }
                        }
                        _ => {}
                    }
                }
                stored.items[index].skip_cleanup_pending = false;
                self.repo.set(ACTIVE, &stored)?;
            }
        }
        if let Some(index) = stored
            .items
            .iter()
            .position(|i| i.start_pending && !i.skipped)
        {
            if completion(&self.library, &stored, &stored.items[index], now).is_none() {
                let restored = self
                    .execute_daily_start(&stored, index, now)
                    .and_then(|_| self.refresh());
                if let Err(error) = restored {
                    self.page = Page::Today;
                    return Err(error);
                }
            }
            stored.items[index].start_pending = false;
            if let Err(e) = self.repo.set(ACTIVE, &stored) {
                self.page = Page::Today;
                return Err(e);
            }
        }
        Ok(())
    }
    fn execute_daily_start(&mut self, stored: &StoredPlan, index: usize, now: i64) -> Result<()> {
        let item = &stored.items[index];
        let (&session, &requested_at) = item
            .started_sessions
            .iter()
            .next()
            .ok_or("This practice has no reserved start identity.")?;
        let existing = match item.key {
            PracticeBatchKey::Passage { .. } => {
                self.session.as_ref().is_some_and(|s| s.id == session)
                    || self
                        .paused_practices()?
                        .iter()
                        .any(|p| p.session.id == session)
            }
            PracticeBatchKey::Teacher {
                document_id,
                practice_index,
            } => self
                .repo
                .get::<crate::teacher_practice::TeacherPracticeDraft>(&format!(
                    "teacher_revisit:{document_id}:{practice_index}"
                ))?
                .is_some_and(|d| d.id == session && d.saved_attempt.is_none()),
        };
        if !existing
            && !self
                .plan_candidates(now)
                .iter()
                .any(|(c, _)| c.key == item.key)
        {
            return Err("The planned return changed before it could start. Its place is kept; skip it or inspect the updated source.".into());
        }
        match item.key {
            PracticeBatchKey::Teacher {
                document_id,
                practice_index,
            } => {
                self.archive_current()?;
                let draft = self.start_teacher_revisit_with_id(
                    TeacherRevisitKey {
                        document_id,
                        practice_index,
                    },
                    now,
                    session,
                )?;
                if draft.started_at_ms < stored.created_at_ms {
                    self.page = Page::Today;
                    return Err(
                        "The original response began before this short session; its draft is kept."
                            .into(),
                    );
                }
            }
            PracticeBatchKey::Passage { revision, skill } => {
                let active = self.session.as_ref().is_some_and(|s| s.id == session);
                if !active {
                    if self
                        .paused_practices()?
                        .iter()
                        .any(|p| p.session.id == session)
                    {
                        self.resume_practice(session)?;
                    } else {
                        self.start_with_id(revision, skill, session)?;
                    }
                }
                let actual = self
                    .session
                    .as_ref()
                    .ok_or("The reserved passage response is unavailable.")?;
                if actual.revision != revision || actual.skill != skill || actual.id != session {
                    return Err("The reserved response belongs to another exact task.".into());
                }
                if self
                    .plan
                    .as_ref()
                    .is_none_or(|p| p.session != session || p.policy != SpacedReview.id())
                {
                    self.set_plan(None, None, SpacedReview.id())?;
                }
                if self
                    .session
                    .as_ref()
                    .is_some_and(|s| s.phase == Phase::Encounter)
                {
                    self.phase(Phase::Retrieve)?;
                }
                self.clear_active_teacher_revisit()?;
                let key = ReviewKey { revision, skill };
                let decision=Fact::TeachingDecisionRecorded(TeachingDecision {session,policy:SpacedReview.id().into(),selected:key,action:TeachingAction::Retrieve,candidates:vec![PolicyCandidate {key,action:TeachingAction::Retrieve,score:0,reason:item.reason.clone()}],assumptions:vec!["Existing fixed passage cadence; a bounded session balances sources and skills. Selection is not practice or memory evidence.".into()],predictions:vec![],override_reason:None});
                let id = Uuid::new_v5(&session, b"daily-plan-decision-v1");
                if let Some(old) = self.library.events.get(&id) {
                    if old.fact != decision {
                        return Err("The reserved policy decision has different contents; its original was kept.".into());
                    }
                } else {
                    self.repo.append(&[Event {
                        version: EVENT_VERSION,
                        id,
                        device: self.device,
                        recorded_at_ms: requested_at,
                        fact: decision,
                    }])?;
                }
            }
        }
        Ok(())
    }
    pub(crate) fn start_daily_plan_item(&mut self, index: usize, now: i64) -> Result<()> {
        self.refresh()?;
        self.reconcile_daily_plan(now)?;
        let projected = self.daily_plan(now)?.ok_or("No short session is active.")?;
        let current = projected
            .items
            .get(index)
            .ok_or("That short-session item is unavailable.")?;
        if current.state != DailyPlanItemState::Ready {
            return Err(
                "This item is already completed, skipped, or unavailable; its work was kept."
                    .into(),
            );
        }
        let mut stored = self
            .stored_daily_plan()?
            .ok_or("The short session is unavailable.")?;
        if stored.id != projected.id {
            return Err("The short session changed. Choose the intended item again.".into());
        }
        if stored.items[index].started_sessions.is_empty() {
            stored.items[index]
                .started_sessions
                .insert(Uuid::new_v4(), now);
        }
        stored.items[index].start_pending = true;
        self.repo.set(ACTIVE, &stored)?; // Exact identity is durable before any task input appears.
        if let Err(e) = self.reconcile_daily_plan(now) {
            self.page = Page::Today;
            return Err(e);
        }
        self.refresh()?;
        self.page = Page::Practice;
        Ok(())
    }
}
