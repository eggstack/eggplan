use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

use crate::{CriterionId, EvidenceProviderId, PlanId, PlanItemId, bounds::*};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanStatus {
    Draft,
    Active,
    Blocked,
    Closed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanItemStatus {
    Pending,
    Actionable,
    InProgress,
    Blocked,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    Command,
    Test,
    StaticAnalysis,
    Revision,
    Artifact,
    DelegatedRun,
    Benchmark,
    Research,
    HumanJudgment,
    Attestation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubjectState {
    Clean,
    Dirty,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubjectRevision {
    pub subject_kind: String,
    pub repository_id: String,
    pub revision: String,
    pub state: SubjectState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dirty_digest: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactRef {
    pub reference: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubjectPolicy {
    Exact,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceCardinality {
    Any,
    All,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceRequirement {
    pub description: String,
    pub kind: EvidenceKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<EvidenceProviderId>,
    pub subject_policy: SubjectPolicy,
    pub cardinality: EvidenceCardinality,
    pub min_count: u16,
    pub allow_human_judgment: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceptanceCriterion {
    pub id: CriterionId,
    pub statement: String,
    pub human_judgment_allowed: bool,
    pub requirements: Vec<EvidenceRequirement>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanItem {
    pub id: PlanItemId,
    pub position: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<PlanItemId>,
    pub dependencies: Vec<PlanItemId>,
    pub status: PlanItemStatus,
    pub description: String,
    pub criteria: Vec<AcceptanceCriterion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocker: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_action: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plan {
    pub schema_version: u32,
    pub id: PlanId,
    pub revision: u64,
    pub objective: String,
    pub status: PlanStatus,
    pub provenance: BTreeMap<String, String>,
    pub items: Vec<PlanItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<SubjectRevision>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ValidationError {
    #[error("text field {field} is empty, contains NUL, or exceeds {max} Unicode scalar values")]
    Text { field: &'static str, max: usize },
    #[error("{field} contains {actual} entries; maximum is {max}")]
    Count {
        field: &'static str,
        actual: usize,
        max: usize,
    },
    #[error("duplicate {0}")]
    Duplicate(&'static str),
    #[error("invalid value: {0}")]
    Invalid(&'static str),
    #[error("unknown schema version {0}")]
    UnknownSchema(u32),
    #[error(transparent)]
    Graph(#[from] crate::GraphError),
}

fn text(value: &str, field: &'static str, max: usize) -> Result<(), ValidationError> {
    let len = value.chars().count();
    if value.is_empty() || value.contains('\0') || len > max {
        return Err(ValidationError::Text { field, max });
    }
    Ok(())
}
fn count(len: usize, field: &'static str, max: usize) -> Result<(), ValidationError> {
    if len > max {
        Err(ValidationError::Count {
            field,
            actual: len,
            max,
        })
    } else {
        Ok(())
    }
}

impl EvidenceRequirement {
    pub fn validate(&self) -> Result<(), ValidationError> {
        text(
            &self.description,
            "requirement.description",
            REQUIREMENT_CHARS,
        )?;
        if self.min_count == 0 {
            return Err(ValidationError::Invalid(
                "requirement min_count must be positive",
            ));
        }
        if self.kind == EvidenceKind::HumanJudgment && !self.allow_human_judgment {
            return Err(ValidationError::Invalid(
                "human judgment kind must be explicitly allowed",
            ));
        }
        Ok(())
    }
}

impl AcceptanceCriterion {
    pub fn validate(&self) -> Result<(), ValidationError> {
        text(&self.statement, "criterion.statement", CRITERION_CHARS)?;
        count(
            self.requirements.len(),
            "criterion.requirements",
            MAX_REQUIREMENTS,
        )?;
        let mut kinds = BTreeSet::new();
        for req in &self.requirements {
            req.validate()?;
            // Do not collapse semantically different constraints; exact duplicates are noise.
            let key = serde_json::to_string(req).expect("serializable requirement");
            if !kinds.insert(key) {
                return Err(ValidationError::Duplicate("evidence requirement"));
            }
        }
        Ok(())
    }
}

impl PlanItem {
    pub fn validate(&self) -> Result<(), ValidationError> {
        text(&self.description, "item.description", DESCRIPTION_CHARS)?;
        count(
            self.dependencies.len(),
            "item.dependencies",
            MAX_DEPENDENCIES,
        )?;
        count(self.criteria.len(), "item.criteria", MAX_CRITERIA)?;
        if self.dependencies.contains(&self.id) {
            return Err(ValidationError::Invalid("self dependency"));
        }
        if self.parent.as_ref() == Some(&self.id) {
            return Err(ValidationError::Invalid("self parent"));
        }
        if let Some(value) = &self.blocker {
            text(value, "item.blocker", BLOCKER_CHARS)?;
        }
        if let Some(value) = &self.next_action {
            text(value, "item.next_action", NEXT_ACTION_CHARS)?;
        }
        if self.status == PlanItemStatus::Blocked && self.blocker.is_none() {
            return Err(ValidationError::Invalid("blocked item requires blocker"));
        }
        if self.status != PlanItemStatus::Blocked && self.blocker.is_some() {
            return Err(ValidationError::Invalid("blocker requires blocked item"));
        }
        let mut ids = BTreeSet::new();
        for criterion in &self.criteria {
            criterion.validate()?;
            if !ids.insert(criterion.id.clone()) {
                return Err(ValidationError::Duplicate("criterion id"));
            }
        }
        let mut deps = BTreeSet::new();
        for dep in &self.dependencies {
            if !deps.insert(dep) {
                return Err(ValidationError::Duplicate("dependency"));
            }
        }
        Ok(())
    }
}

impl Plan {
    pub fn new(
        id: PlanId,
        objective: impl Into<String>,
        items: Vec<PlanItem>,
    ) -> Result<Self, ValidationError> {
        let plan = Self {
            schema_version: crate::SCHEMA_VERSION,
            id,
            revision: 0,
            objective: objective.into(),
            status: PlanStatus::Draft,
            provenance: BTreeMap::new(),
            items,
            subject: None,
        };
        plan.validate()?;
        Ok(plan)
    }
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.schema_version != crate::SCHEMA_VERSION {
            return Err(ValidationError::UnknownSchema(self.schema_version));
        }
        text(&self.objective, "plan.objective", OBJECTIVE_CHARS)?;
        count(self.items.len(), "plan.items", MAX_ITEMS)?;
        count(
            self.provenance.len(),
            "plan.provenance",
            MAX_EXTENSION_ENTRIES,
        )?;
        for (key, value) in &self.provenance {
            text(key, "plan.provenance.key", ID_CHARS)?;
            text(value, "plan.provenance.value", PROVENANCE_CHARS)?;
        }
        let mut ids = BTreeSet::new();
        for item in &self.items {
            item.validate()?;
            if !ids.insert(item.id.clone()) {
                return Err(ValidationError::Duplicate("item id"));
            }
        }
        let known: BTreeSet<_> = self.items.iter().map(|item| &item.id).collect();
        for item in &self.items {
            for dep in &item.dependencies {
                if !known.contains(dep) {
                    return Err(ValidationError::Invalid("missing dependency"));
                }
            }
            if let Some(parent) = &item.parent
                && !known.contains(parent)
            {
                return Err(ValidationError::Invalid("missing parent"));
            }
        }
        crate::graph::validate_graph(self)?;
        if let Some(subject) = &self.subject {
            subject.validate()?;
        }
        Ok(())
    }
}

impl SubjectRevision {
    pub fn validate(&self) -> Result<(), ValidationError> {
        text(&self.subject_kind, "subject.kind", 64)?;
        text(&self.repository_id, "subject.repository_id", 256)?;
        text(&self.revision, "subject.revision", 256)?;
        match (&self.state, self.dirty_digest.as_ref()) {
            (SubjectState::Clean, None) => Ok(()),
            (SubjectState::Dirty, Some(digest)) if valid_sha256(digest) => Ok(()),
            _ => Err(ValidationError::Invalid(
                "clean subjects omit dirty digest; dirty subjects require sha256 digest",
            )),
        }
    }
}

impl ArtifactRef {
    pub fn validate(&self) -> Result<(), ValidationError> {
        text(&self.reference, "artifact.reference", ARTIFACT_REF_CHARS)?;
        if let Some(digest) = &self.digest
            && !valid_sha256(digest)
        {
            return Err(ValidationError::Invalid(
                "artifact digest must be sha256:<64 lowercase hex>",
            ));
        }
        if let Some(media) = &self.media_type {
            text(media, "artifact.media_type", 128)?;
        }
        Ok(())
    }
}

fn valid_sha256(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64
            && hex
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

pub fn plan_transition_allowed(from: &PlanStatus, to: &PlanStatus) -> bool {
    use PlanStatus::*;
    matches!(
        (from, to),
        (Draft, Active)
            | (Draft, Cancelled)
            | (Active, Blocked)
            | (Active, Closed)
            | (Active, Cancelled)
            | (Blocked, Active)
            | (Blocked, Cancelled)
    )
}
pub fn item_transition_allowed(from: &PlanItemStatus, to: &PlanItemStatus) -> bool {
    use PlanItemStatus::*;
    matches!(
        (from, to),
        (Pending, Actionable)
            | (Pending, Blocked)
            | (Pending, Cancelled)
            | (Actionable, InProgress)
            | (Actionable, Blocked)
            | (Actionable, Cancelled)
            | (InProgress, Actionable)
            | (InProgress, Blocked)
            | (InProgress, Completed)
            | (InProgress, Cancelled)
            | (Blocked, Pending)
            | (Blocked, Actionable)
            | (Blocked, Cancelled)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn item() -> PlanItem {
        PlanItem {
            id: PlanItemId::new("epi_one").unwrap(),
            position: 0,
            parent: None,
            dependencies: vec![],
            status: PlanItemStatus::Pending,
            description: "work".into(),
            criteria: vec![],
            blocker: None,
            next_action: None,
        }
    }
    #[test]
    fn blocker_discipline_and_text_bounds() {
        let mut i = item();
        i.status = PlanItemStatus::Blocked;
        assert!(i.validate().is_err());
        i.blocker = Some("waiting".into());
        assert!(i.validate().is_ok());
        i.blocker = Some("x".repeat(BLOCKER_CHARS + 1));
        assert!(i.validate().is_err());
    }
    #[test]
    fn transition_matrices_fail_closed_for_terminal_states() {
        assert!(plan_transition_allowed(
            &PlanStatus::Draft,
            &PlanStatus::Active
        ));
        assert!(!plan_transition_allowed(
            &PlanStatus::Closed,
            &PlanStatus::Active
        ));
        assert!(item_transition_allowed(
            &PlanItemStatus::InProgress,
            &PlanItemStatus::Completed
        ));
        assert!(!item_transition_allowed(
            &PlanItemStatus::Completed,
            &PlanItemStatus::Actionable
        ));
    }

    #[test]
    fn collection_and_unicode_scalar_bounds_are_enforced() {
        let mut p = Plan::new(PlanId::new("ep_bounds").unwrap(), "é", vec![]).unwrap();
        p.objective = "é".repeat(OBJECTIVE_CHARS + 1);
        assert!(p.validate().is_err());
        p.objective = "valid".into();
        p.items = (0..=MAX_ITEMS)
            .map(|n| PlanItem {
                id: PlanItemId::new(format!("epi_{n}")).unwrap(),
                position: n as u32,
                parent: None,
                dependencies: vec![],
                status: PlanItemStatus::Pending,
                description: "step".into(),
                criteria: vec![],
                blocker: None,
                next_action: None,
            })
            .collect();
        assert!(matches!(
            p.validate(),
            Err(ValidationError::Count {
                field: "plan.items",
                ..
            })
        ));
    }

    #[test]
    fn malformed_text_and_dangling_graph_references_fail() {
        let mut p = Plan::new(PlanId::new("ep_invalid").unwrap(), "valid", vec![item()]).unwrap();
        p.items[0].description = "bad\0text".into();
        assert!(p.validate().is_err());
        p.items[0].description = "work".into();
        p.items[0].dependencies = vec![PlanItemId::new("epi_missing").unwrap()];
        assert!(p.validate().is_err());
        p.items[0].dependencies.clear();
        p.items[0].parent = Some(PlanItemId::new("epi_missing").unwrap());
        assert!(p.validate().is_err());
    }
}
