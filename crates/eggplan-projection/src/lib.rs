#![forbid(unsafe_code)]

//! Bounded, stable summaries derived from canonical Eggplan state.

use eggplan_core::{
    AssessmentReason, AssessmentStatus, Plan, PlanAssessment, PlanId, PlanItemId, PlanItemStatus,
    PlanStatus, Readiness, SubjectRevision, readiness,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const OUTPUT_SCHEMA_VERSION: u32 = 1;
pub const MAX_PROJECTED_PLANS: usize = 100;
pub const MAX_PROJECTED_ITEMS: usize = 100;
pub const MAX_TEXT_CHARS: usize = 512;
pub const MAX_WARNINGS: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutputEnvelope<T> {
    pub schema_version: u32,
    pub command: String,
    pub ok: bool,
    pub data: Option<T>,
    pub error: Option<Diagnostic>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Diagnostic {
    pub code: String,
    pub message: String,
}

impl<T> OutputEnvelope<T> {
    pub fn success(command: impl Into<String>, data: T, warnings: Vec<String>) -> Self {
        Self {
            schema_version: OUTPUT_SCHEMA_VERSION,
            command: command.into(),
            ok: true,
            data: Some(data),
            error: None,
            warnings: bound_warnings(warnings),
        }
    }

    pub fn failure(
        command: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            schema_version: OUTPUT_SCHEMA_VERSION,
            command: command.into(),
            ok: false,
            data: None,
            error: Some(Diagnostic {
                code: code.into(),
                message: truncate(&message.into(), MAX_TEXT_CHARS).0,
            }),
            warnings: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanSummary {
    pub plan_id: PlanId,
    pub revision: u64,
    pub status: PlanStatus,
    pub objective: String,
    pub objective_truncated: bool,
    pub item_count: usize,
    pub projected_item_count: usize,
    pub items_truncated: bool,
    pub item_status_counts: BTreeMap<String, usize>,
    pub current_subject: Option<SubjectRevision>,
    pub assessment_status: Option<AssessmentStatus>,
    pub assessment_reason_codes: Vec<String>,
    pub has_closure: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemSummary {
    pub item_id: PlanItemId,
    pub position: u32,
    pub status: PlanItemStatus,
    pub description: String,
    pub description_truncated: bool,
    pub dependencies: Vec<PlanItemId>,
    pub readiness: Option<String>,
    pub waiting_on: Vec<PlanItemId>,
    pub blocker: Option<String>,
    pub blocker_truncated: bool,
    pub next_action: Option<String>,
    pub next_action_truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanDetail {
    pub plan: PlanSummary,
    pub items: Vec<ItemSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphProjection {
    pub plan_id: PlanId,
    pub nodes: Vec<GraphNode>,
    pub dependency_edges: Vec<GraphEdge>,
    pub parent_edges: Vec<GraphEdge>,
    pub total_dependency_edges: usize,
    pub total_parent_edges: usize,
    pub edges_truncated: bool,
    pub truncated: bool,
    pub total_nodes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphNode {
    pub item_id: PlanItemId,
    pub position: u32,
    pub status: PlanItemStatus,
    pub description: String,
    pub description_truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphEdge {
    pub from: PlanItemId,
    pub to: PlanItemId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadinessProjection {
    pub plan_id: PlanId,
    pub items: Vec<ItemSummary>,
    pub total_items: usize,
    pub ready_count: usize,
    pub waiting_count: usize,
    pub not_actionable_count: usize,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistryProjection {
    pub repository_id: String,
    pub plan_count: usize,
    pub projected_plan_count: usize,
    pub plans_truncated: bool,
    pub plans: Vec<RegistryPlan>,
    pub blocker_count: usize,
    pub blockers_truncated: bool,
    pub blockers: Vec<RegistryBlocker>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistryPlan {
    pub plan_id: PlanId,
    pub revision: u64,
    pub status: PlanStatus,
    pub item_count: usize,
    pub closure_present: bool,
    pub readiness_count: usize,
    pub assessment_status: Option<AssessmentStatus>,
    pub assessment_reason_codes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistryBlocker {
    pub plan_id: PlanId,
    pub item_id: PlanItemId,
    pub text: String,
    pub text_truncated: bool,
}

pub fn summarize_plan(
    plan: &Plan,
    subject: Option<SubjectRevision>,
    assessment: Option<&PlanAssessment>,
    has_closure: bool,
) -> PlanSummary {
    let (objective, objective_truncated) = truncate(&plan.objective, MAX_TEXT_CHARS);
    let mut item_status_counts = BTreeMap::new();
    for item in &plan.items {
        *item_status_counts
            .entry(item_status_code(item.status).to_string())
            .or_insert(0) += 1;
    }
    let mut assessment_reason_codes = assessment
        .into_iter()
        .flat_map(|value| value.reasons.iter())
        .map(reason_code)
        .collect::<Vec<_>>();
    assessment_reason_codes.sort();
    assessment_reason_codes.dedup();
    PlanSummary {
        plan_id: plan.id.clone(),
        revision: plan.revision,
        status: plan.status.clone(),
        objective,
        objective_truncated,
        item_count: plan.items.len(),
        projected_item_count: plan.items.len().min(MAX_PROJECTED_ITEMS),
        items_truncated: plan.items.len() > MAX_PROJECTED_ITEMS,
        item_status_counts,
        current_subject: subject,
        assessment_status: assessment.map(|value| value.status),
        assessment_reason_codes,
        has_closure,
    }
}

pub fn readiness_projection(plan: &Plan) -> ReadinessProjection {
    let readiness: BTreeMap<_, _> = readiness(plan)
        .into_iter()
        .map(|item| (item.item_id, item.readiness))
        .collect();
    let mut items: Vec<_> = plan
        .items
        .iter()
        .map(|item| item_summary(item, readiness.get(&item.id)))
        .collect();
    items.sort_by_key(|item| (item.position, item.item_id.clone()));
    let ready_count = items
        .iter()
        .filter(|item| item.readiness.as_deref() == Some("ready"))
        .count();
    let waiting_count = items
        .iter()
        .filter(|item| item.readiness.as_deref() == Some("waiting_on_dependencies"))
        .count();
    let not_actionable_count = items
        .iter()
        .filter(|item| item.readiness.as_deref() == Some("not_actionable"))
        .count();
    let truncated = items.len() > MAX_PROJECTED_ITEMS;
    let total_items = items.len();
    items.truncate(MAX_PROJECTED_ITEMS);
    ReadinessProjection {
        plan_id: plan.id.clone(),
        items,
        total_items,
        ready_count,
        waiting_count,
        not_actionable_count,
        truncated,
    }
}

pub fn graph_projection(plan: &Plan) -> GraphProjection {
    let mut items: Vec<_> = plan.items.iter().collect();
    items.sort_by_key(|item| (item.position, item.id.clone()));
    let total_nodes = items.len();
    let truncated = total_nodes > MAX_PROJECTED_ITEMS;
    let selected: Vec<_> = items.into_iter().take(MAX_PROJECTED_ITEMS).collect();
    let selected_ids: BTreeSet<_> = selected.iter().map(|item| item.id.clone()).collect();
    let total_dependency_edges: usize = plan.items.iter().map(|item| item.dependencies.len()).sum();
    let total_parent_edges = plan
        .items
        .iter()
        .filter(|item| item.parent.is_some())
        .count();
    let nodes = selected
        .iter()
        .map(|item| {
            let (description, description_truncated) = truncate(&item.description, MAX_TEXT_CHARS);
            GraphNode {
                item_id: item.id.clone(),
                position: item.position,
                status: item.status,
                description,
                description_truncated,
            }
        })
        .collect();
    let mut dependency_edges = Vec::new();
    let mut parent_edges = Vec::new();
    for item in selected {
        for dependency in &item.dependencies {
            if selected_ids.contains(dependency) {
                dependency_edges.push(GraphEdge {
                    from: item.id.clone(),
                    to: dependency.clone(),
                });
            }
        }
        if let Some(parent) = &item.parent
            && selected_ids.contains(parent)
        {
            parent_edges.push(GraphEdge {
                from: item.id.clone(),
                to: parent.clone(),
            });
        }
    }
    dependency_edges.sort();
    parent_edges.sort();
    let edges_truncated =
        dependency_edges.len() < total_dependency_edges || parent_edges.len() < total_parent_edges;
    GraphProjection {
        plan_id: plan.id.clone(),
        nodes,
        dependency_edges,
        parent_edges,
        total_dependency_edges,
        total_parent_edges,
        edges_truncated,
        truncated,
        total_nodes,
    }
}

pub fn registry_projection(
    repository_id: impl Into<String>,
    plans: impl IntoIterator<Item = (Plan, bool, Option<PlanAssessment>)>,
) -> RegistryProjection {
    let mut plans: Vec<_> = plans.into_iter().collect();
    plans.sort_by(|left, right| left.0.id.cmp(&right.0.id));
    let plan_count = plans.len();
    let mut blockers = Vec::new();
    for (plan, _, _) in &plans {
        let mut items: Vec<_> = plan
            .items
            .iter()
            .filter(|item| item.blocker.is_some())
            .collect();
        items.sort_by_key(|item| (item.position, item.id.clone()));
        blockers.extend(items.into_iter().map(|item| {
            let (text, text_truncated) =
                truncate(item.blocker.as_deref().unwrap_or_default(), MAX_TEXT_CHARS);
            RegistryBlocker {
                plan_id: plan.id.clone(),
                item_id: item.id.clone(),
                text,
                text_truncated,
            }
        }));
    }
    let blocker_count = blockers.len();
    let blockers_truncated = blocker_count > MAX_PROJECTED_ITEMS;
    blockers.truncate(MAX_PROJECTED_ITEMS);
    let projected: Vec<_> = plans
        .into_iter()
        .take(MAX_PROJECTED_PLANS)
        .map(|(plan, closure_present, assessment)| {
            let readiness_count = readiness(&plan)
                .into_iter()
                .filter(|item| item.readiness == Readiness::Ready)
                .count();
            RegistryPlan {
                plan_id: plan.id,
                revision: plan.revision,
                status: plan.status,
                item_count: plan.items.len(),
                closure_present,
                readiness_count,
                assessment_status: assessment.as_ref().map(|value| value.status),
                assessment_reason_codes: assessment
                    .as_ref()
                    .map(|value| {
                        let mut codes: Vec<_> = value.reasons.iter().map(reason_code).collect();
                        codes.sort();
                        codes.dedup();
                        codes.truncate(MAX_WARNINGS);
                        codes
                    })
                    .unwrap_or_default(),
            }
        })
        .collect();
    RegistryProjection {
        repository_id: repository_id.into(),
        plan_count,
        projected_plan_count: projected.len(),
        plans_truncated: plan_count > MAX_PROJECTED_PLANS,
        plans: projected,
        blocker_count,
        blockers_truncated,
        blockers,
    }
}

fn item_summary(item: &eggplan_core::PlanItem, readiness: Option<&Readiness>) -> ItemSummary {
    let (description, description_truncated) = truncate(&item.description, MAX_TEXT_CHARS);
    let (blocker, blocker_truncated) = item
        .blocker
        .as_deref()
        .map(|value| truncate(value, MAX_TEXT_CHARS))
        .map_or((None, false), |(value, truncated)| (Some(value), truncated));
    let (next_action, next_action_truncated) = item
        .next_action
        .as_deref()
        .map(|value| truncate(value, MAX_TEXT_CHARS))
        .map_or((None, false), |(value, truncated)| (Some(value), truncated));
    let (readiness, waiting_on) = match readiness {
        Some(Readiness::Ready) => (Some("ready".into()), vec![]),
        Some(Readiness::WaitingOnDependencies(ids)) => {
            (Some("waiting_on_dependencies".into()), ids.clone())
        }
        Some(Readiness::NotActionable) => (Some("not_actionable".into()), vec![]),
        None => (None, vec![]),
    };
    ItemSummary {
        item_id: item.id.clone(),
        position: item.position,
        status: item.status,
        description,
        description_truncated,
        dependencies: item.dependencies.clone(),
        readiness,
        waiting_on,
        blocker,
        blocker_truncated,
        next_action,
        next_action_truncated,
    }
}

fn item_status_code(status: PlanItemStatus) -> &'static str {
    match status {
        PlanItemStatus::Pending => "pending",
        PlanItemStatus::Actionable => "actionable",
        PlanItemStatus::InProgress => "in_progress",
        PlanItemStatus::Blocked => "blocked",
        PlanItemStatus::Completed => "completed",
        PlanItemStatus::Cancelled => "cancelled",
    }
}

pub fn reason_code(reason: &AssessmentReason) -> String {
    match reason {
        AssessmentReason::MissingObservation => "missing_observation",
        AssessmentReason::NoAcceptanceCriteria => "no_acceptance_criteria",
        AssessmentReason::NoEvidenceRequirement => "no_evidence_requirement",
        AssessmentReason::HumanJudgmentRequired => "human_judgment_required",
        AssessmentReason::StaleSubject(_) => "stale_subject",
        AssessmentReason::UntrustedProvider(_) => "untrusted_provider",
        AssessmentReason::ProviderKindNotAllowed(_) => "provider_kind_not_allowed",
        AssessmentReason::ProviderMismatch(_) => "provider_mismatch",
        AssessmentReason::LegacyUnboundExecutionRequirement => {
            "legacy_unbound_execution_requirement"
        }
        AssessmentReason::ObservationMissingVerificationBinding(_) => {
            "observation_missing_verification_binding"
        }
        AssessmentReason::VerificationDigestMismatch(_) => "verification_digest_mismatch",
        AssessmentReason::EvidenceStatus(_, _) => "evidence_status",
        AssessmentReason::HumanJudgmentNotAllowed => "human_judgment_not_allowed",
        AssessmentReason::InvalidObservation(_) => "invalid_observation",
        AssessmentReason::DuplicateObservationId(_) => "duplicate_observation_id",
        AssessmentReason::PlanNotActive => "plan_not_active",
        AssessmentReason::InvalidPlan => "invalid_plan",
        AssessmentReason::ItemNotCompleted(_) => "item_not_completed",
        AssessmentReason::PlanBlocked => "plan_blocked",
        AssessmentReason::PlanCancelled => "plan_cancelled",
        AssessmentReason::NoPlanItems => "no_plan_items",
    }
    .into()
}

fn truncate(value: &str, max: usize) -> (String, bool) {
    let mut chars = value.chars();
    let output: String = chars.by_ref().take(max).collect();
    (output, chars.next().is_some())
}

fn bound_warnings(mut warnings: Vec<String>) -> Vec<String> {
    warnings.truncate(MAX_WARNINGS);
    warnings
        .into_iter()
        .map(|warning| truncate(&warning, MAX_TEXT_CHARS).0)
        .collect()
}
