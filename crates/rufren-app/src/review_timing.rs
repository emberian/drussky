//! Shared timing intent and immutable starts; response drafts and plans remain local.
use crate::{model::AppState, repository::Result, teacher_revisit::proven_teacher_task_origin};
use rufren_core::*;
use std::collections::BTreeSet;
use uuid::Uuid;

fn start_id(session: Uuid) -> Uuid {
    Uuid::new_v5(&session, b"review-start-v1")
}

impl AppState {
    pub(crate) fn review_cadence(&self, _now: i64) -> ReviewCadenceState {
        ReviewCadencePolicy.project(&self.library)
    }

    pub(crate) fn change_review_cadence(
        &mut self,
        cadence: ReviewCadence,
        expected_heads: BTreeSet<Uuid>,
        now: i64,
    ) -> Result<()> {
        self.refresh()?;
        if self.review_cadence(now).heads != expected_heads {
            return Err(
                "Another timing choice arrived. Review the updated choices before saving.".into(),
            );
        }
        let event = Event::new(
            self.device,
            now,
            Fact::ReviewCadenceChanged(ReviewCadenceChange {
                parents: expected_heads,
                cadence,
            }),
        )
        .map_err(|e| e.to_string())?;
        self.repo.append(&[event])?;
        self.refresh()
    }

    /// Exact target proof prevents an unrelated session from consuming a one-shot timing choice.
    fn review_attempt_target(&self, event: &Event, attempt: &Attempt) -> Option<PracticeBatchKey> {
        if self
            .library
            .events
            .get(&attempt.revision)
            .is_some_and(is_teacher_generated_reference)
        {
            let contexts = self.library.contexts(event.id);
            if contexts.len() != 1 {
                return None;
            }
            let origin = proven_teacher_task_origin(&self.library, event.id, contexts[0].0.id)?;
            Some(PracticeBatchKey::Teacher {
                document_id: origin.key.document_id,
                practice_index: origin.key.practice_index,
            })
        } else if self
            .library
            .events
            .get(&attempt.revision)
            .is_some_and(is_reserved_practice_reference)
        {
            None
        } else {
            self.library.revision(attempt.revision)?;
            Some(PracticeBatchKey::Passage {
                revision: attempt.revision,
                skill: attempt.skill,
            })
        }
    }

    pub(crate) fn review_timing(
        &self,
        target: PracticeBatchKey,
        baseline_due_at_ms: i64,
        now: i64,
    ) -> ReviewTimingState {
        let completed = self
            .library
            .attempts()
            .filter(|(e, a)| {
                e.recorded_at_ms <= now
                    && self.review_attempt_target(e, a) == Some(target)
                    && self
                        .review_start_decision(a.session, target)
                        .ok()
                        .flatten()
                        .is_some_and(|s| s.started_at_ms <= e.recorded_at_ms)
            })
            .map(|(_, a)| a.session)
            .collect();
        ReviewTimingPolicy.project(&self.library, target, baseline_due_at_ms, now, &completed)
    }

    pub(crate) fn change_review_timing(
        &mut self,
        target: PracticeBatchKey,
        action: ReviewTimingAction,
        expected_heads: BTreeSet<Uuid>,
        now: i64,
    ) -> Result<()> {
        self.refresh()?;
        // Heads are independent of baseline due date and clock time.
        if self.review_timing(target, now, now).heads != expected_heads {
            return Err(
                "Another choice for this task arrived. Review its updated timing before saving."
                    .into(),
            );
        }
        let event = Event::new(
            self.device,
            now,
            Fact::ReviewTimingChanged(ReviewTimingChange {
                target,
                parents: expected_heads,
                action,
            }),
        )
        .map_err(|e| e.to_string())?;
        self.repo.append(&[event])?;
        self.refresh()
    }

    pub(crate) fn review_start_decision(
        &self,
        session: Uuid,
        target: PracticeBatchKey,
    ) -> Result<Option<ReviewStartDecision>> {
        if let Some(event) = self.library.events.get(&start_id(session)) {
            if !matches!(&event.fact,Fact::ReviewStarted(d) if d.session == session && d.target == target)
            {
                return Err("This response has a conflicting saved timing decision. Its original history was kept.".into());
            }
        }
        for event in self.library.events.values() {
            if let Fact::ReviewStarted(decision) = &event.fact {
                if decision.session == session {
                    return if decision.target == target {
                        Ok(Some(decision.clone()))
                    } else {
                        Err("This response has a saved decision for another exact task.".into())
                    };
                }
            }
        }
        Ok(None)
    }

    /// Complete a durably prepared start without assigning a new policy to a legacy response.
    pub(crate) fn recover_pending_review_start(
        &mut self,
        session: Uuid,
        target: PracticeBatchKey,
    ) -> Result<()> {
        if let Some(pending) = self
            .repo
            .get::<ReviewStartDecision>(&format!("review_start_pending:{session}"))?
        {
            if pending.session != session || pending.target != target {
                return Err(
                    "This pending timing decision belongs to another response. Its draft was kept."
                        .into(),
                );
            }
            self.record_review_start(&pending)?;
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn prepare_review_start(
        &mut self,
        session: Uuid,
        target: PracticeBatchKey,
        baseline_due_at_ms: i64,
        started_at_ms: i64,
        as_of_ms: i64,
        policy: &str,
        reason: &str,
        evidence_ids: BTreeSet<Uuid>,
    ) -> Result<ReviewStartDecision> {
        self.refresh()?;
        if let Some(existing) = self.review_start_decision(session, target)? {
            return Ok(existing);
        }
        if let Some(pending) = self
            .repo
            .get::<ReviewStartDecision>(&format!("review_start_pending:{session}"))?
        {
            if pending.session != session || pending.target != target {
                return Err("This pending start belongs to another exact task.".into());
            }
            return Ok(pending);
        }
        let cadence = self.review_cadence(as_of_ms);
        let timing = self.review_timing(target, baseline_due_at_ms, as_of_ms);
        let evidence_total_count = evidence_ids.len();
        let evidence_ids_sha256 = review_evidence_ids_sha256(&evidence_ids);
        let evidence_ids = evidence_ids
            .into_iter()
            .take(MAX_REVIEW_EVIDENCE_IDS)
            .collect();
        let decision = ReviewStartDecision {
            target,
            session,
            as_of_ms,
            started_at_ms,
            cadence: cadence.cadence,
            cadence_heads: cadence.heads,
            cadence_fallback_reason: cadence.fallback_reason,
            baseline_due_at_ms,
            effective_due_at_ms: timing.effective_due_at_ms,
            timing_paused: timing.paused,
            timing_pending: timing.pending,
            timing_heads: timing.heads,
            policy: policy.into(),
            reason: reason.into(),
            evidence_ids,
            evidence_total_count,
            evidence_ids_sha256,
        };
        Event::new(self.device, as_of_ms, Fact::ReviewStarted(decision.clone()))
            .map_err(|e| e.to_string())?;
        self.repo
            .set(&format!("review_start_pending:{session}"), &decision)?;
        Ok(decision)
    }

    /// A retry reads the issued snapshot, never reassigns a historical start to newer preferences.
    pub(crate) fn record_review_start(&mut self, decision: &ReviewStartDecision) -> Result<()> {
        self.refresh()?;
        if self
            .review_start_decision(decision.session, decision.target)?
            .is_some()
        {
            return Ok(());
        }
        let mut event = Event::new(
            self.device,
            decision.as_of_ms,
            Fact::ReviewStarted(decision.clone()),
        )
        .map_err(|e| e.to_string())?;
        event.id = start_id(decision.session);
        self.repo.append(&[event])?;
        self.refresh()
    }

    pub(crate) fn review_cadence_for_attempt(
        &self,
        event: &Event,
        attempt: &Attempt,
    ) -> ReviewCadence {
        self.review_attempt_target(event, attempt)
            .and_then(|target| {
                self.review_start_decision(attempt.session, target)
                    .ok()
                    .flatten()
            })
            .filter(|d| d.started_at_ms <= event.recorded_at_ms)
            .map_or(ReviewCadence::Standard, |d| d.cadence)
    }
}
