//! A bounded, inspectable allocation of ready activities. Planning is not practice evidence.
use crate::*;

pub const PRACTICE_BATCH_POLICY_ID: &str = "bounded-balanced-practice-v1";
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PracticeBatchKey {
    Teacher {
        document_id: Uuid,
        practice_index: usize,
    },
    Passage {
        revision: Uuid,
        skill: Skill,
    },
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum PracticeBatchSource {
    Teacher,
    Passage,
}
impl PracticeBatchKey {
    pub fn source(self) -> PracticeBatchSource {
        match self {
            Self::Teacher { .. } => PracticeBatchSource::Teacher,
            Self::Passage { .. } => PracticeBatchSource::Passage,
        }
    }
    pub fn validate(self) -> Result<(), Error> {
        let valid = match self {
            Self::Teacher {
                document_id,
                practice_index,
            } => !document_id.is_nil() && practice_index < 4,
            Self::Passage { revision, .. } => !revision.is_nil(),
        };
        if valid {
            Ok(())
        } else {
            Err(Error::Invalid("invalid planned practice identity".into()))
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PracticeBatchCandidate {
    pub key: PracticeBatchKey,
    pub skill: Skill,
    pub due_at_ms: i64,
    pub last_skill_practice_ms: Option<i64>,
    pub last_source_practice_ms: Option<i64>,
}
#[derive(Debug, Clone, Copy, Default)]
pub struct BoundedPracticeBatch;
impl BoundedPracticeBatch {
    /// Alternate available sources, then favor less recently practiced skills within that source.
    /// Temporary selection counts allocate this batch; they never become learner observations.
    /// Equal histories use oldest due date and stable identity, not an inferred ability score.
    pub fn select(
        &self,
        candidates: &[PracticeBatchCandidate],
        response_budget: usize,
        quiet: bool,
        now_ms: i64,
    ) -> Result<Vec<PracticeBatchCandidate>, Error> {
        if ![1, 3, 5].contains(&response_budget) {
            return Err(Error::Invalid(
                "choose a 1, 3 or 5 response practice budget".into(),
            ));
        }
        let mut unique = BTreeMap::new();
        for c in candidates {
            c.key.validate()?;
            if matches!(c.key,PracticeBatchKey::Passage {skill,..} if skill!=c.skill)
                || [c.last_skill_practice_ms, c.last_source_practice_ms]
                    .into_iter()
                    .flatten()
                    .any(|t| t > now_ms)
            {
                return Err(Error::Invalid(
                    "inconsistent practice candidate metadata".into(),
                ));
            }
            if unique.insert(c.key, c).is_some_and(|old| old != c) {
                return Err(Error::Invalid(
                    "practice candidate identity reused with different metadata".into(),
                ));
            }
        }
        let mut remaining: Vec<_> = unique
            .into_values()
            .filter(|c| {
                c.due_at_ms <= now_ms
                    && (!quiet || matches!(c.skill, Skill::Reading | Skill::Writing))
            })
            .cloned()
            .collect();
        let mut selected = Vec::new();
        let mut sources = BTreeMap::<PracticeBatchSource, usize>::new();
        let mut skills = BTreeMap::<Skill, usize>::new();
        while selected.len() < response_budget && !remaining.is_empty() {
            let i = (0..remaining.len())
                .min_by_key(|&i| {
                    let c = &remaining[i];
                    (
                        sources.get(&c.key.source()).copied().unwrap_or(0),
                        c.last_source_practice_ms,
                        skills.get(&c.skill).copied().unwrap_or(0),
                        c.last_skill_practice_ms,
                        c.due_at_ms,
                        c.key,
                    )
                })
                .unwrap();
            let chosen = remaining.remove(i);
            *sources.entry(chosen.key.source()).or_default() += 1;
            *skills.entry(chosen.skill).or_default() += 1;
            selected.push(chosen);
        }
        Ok(selected)
    }
}
