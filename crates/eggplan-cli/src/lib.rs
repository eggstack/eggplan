#![forbid(unsafe_code)]

//! Thin command adapter over eggplan-core, eggplan-repo, and bounded DTOs.

use eggplan_core::{
    AssessmentStatus, ClosureCandidate, ClosureId, EvidenceKind, EvidenceObservation,
    EvidenceObservationId, EvidenceProviderId, EvidenceStatus, Plan, PlanAssessment, PlanId,
    PlanItemId, PlanStatus, ProviderDescriptor, ProviderPolicyEntry, ProviderRegistry,
    SubjectRevision, assess_plan, effective_observations, parse_plan,
};
use eggplan_projection::{
    OutputEnvelope, PlanDetail, PlanSummary, graph_projection, readiness_projection, reason_code,
    registry_projection, summarize_plan,
};
use eggplan_repo::{PlanStore, RepoError, RepositoryStore};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_INPUT_BYTES: u64 = 16 * 1024 * 1024;
const MAX_POLICY_BYTES: u64 = 64 * 1024;
const MAX_EVIDENCE_ROWS: usize = 100;
const MAX_SUPERSESSION_ROWS: usize = 100;
const MAX_ASSESSMENT_ROWS: usize = 100;
const MAX_REASONS_PER_ROW: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliFailure {
    pub command: String,
    pub code: String,
    pub message: String,
    pub json: bool,
}

pub struct ExecutionResult {
    pub command: String,
    pub data: Value,
    pub warnings: Vec<String>,
    pub human: String,
    pub json: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProviderPolicyFile {
    schema_version: u32,
    providers: Vec<ProviderPolicyInput>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProviderPolicyInput {
    provider_id: EvidenceProviderId,
    class: String,
    allowed_kinds: Vec<EvidenceKind>,
}

struct LoadedPolicy {
    registry: ProviderRegistry,
    entries: Vec<ProviderPolicyEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
struct StatusData {
    plans: Vec<PlanSummary>,
    total_plans: usize,
    truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
struct EvidenceListData {
    observations: Vec<EvidenceBrief>,
    total_count: usize,
    truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
struct EvidenceBrief {
    observation_id: EvidenceObservationId,
    provider_id: EvidenceProviderId,
    kind: EvidenceKind,
    status: EvidenceStatus,
    observed_at_unix_ms: u64,
    content_digest: String,
    artifact_count: usize,
    verification_digest: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
struct AssessmentData {
    plan_id: PlanId,
    plan_revision: u64,
    status: AssessmentStatus,
    reason_codes: Vec<String>,
    items: Vec<AssessmentItem>,
    total_items: usize,
    items_truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
struct AssessmentItem {
    item_id: PlanItemId,
    status: AssessmentStatus,
    reason_codes: Vec<String>,
    total_reasons: usize,
    reasons_truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
struct ClosureSummary {
    closure_id: ClosureId,
    plan_id: PlanId,
    source_revision: u64,
    final_plan_revision: u64,
    satisfying_observation_count: usize,
    provider_count: usize,
    subject: SubjectRevision,
    content_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
struct CheckData {
    repository_id: String,
    plans_checked: usize,
    observations_checked: usize,
    supersessions_checked: usize,
    closures_checked: usize,
    abandoned_staging_count: usize,
    plans: Vec<CheckedPlan>,
    plans_truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
struct CheckedPlan {
    plan_id: PlanId,
    integrity: String,
    state: String,
    assessment_status: Option<AssessmentStatus>,
    reason_codes: Vec<String>,
    current_subject_available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
struct InitData {
    repository_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
struct MutationData {
    plan_id: PlanId,
    revision: u64,
    status: PlanStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
struct NewData {
    plan_id: PlanId,
    revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
struct ClosureCreatedData {
    closure: ClosureSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
struct SupersessionListData {
    records: Vec<eggplan_core::EvidenceSupersessionRecord>,
    total_count: usize,
    truncated: bool,
}

#[derive(Debug)]
struct ParsedArgs {
    command: String,
    positional: Vec<String>,
    options: BTreeMap<String, String>,
    flags: BTreeSet<String>,
    json: bool,
}

pub fn run(args: impl IntoIterator<Item = String>) -> i32 {
    let raw: Vec<_> = args.into_iter().collect();
    let wants_json = raw.iter().any(|arg| arg == "--json");
    match execute(&raw) {
        Ok(result) => {
            if result.json {
                let envelope =
                    OutputEnvelope::success(result.command, result.data, result.warnings);
                println!(
                    "{}",
                    serde_json::to_string_pretty(&envelope).unwrap_or_else(|_| "{}".into())
                );
            } else {
                println!("{}", result.human);
            }
            0
        }
        Err(error) => {
            if wants_json || error.json {
                let envelope: OutputEnvelope<Value> =
                    OutputEnvelope::failure(error.command, error.code, error.message);
                println!(
                    "{}",
                    serde_json::to_string_pretty(&envelope).unwrap_or_else(|_| "{}".into())
                );
            } else {
                eprintln!("{}: {}: {}", error.command, error.code, error.message);
            }
            2
        }
    }
}

pub fn execute(args: &[String]) -> Result<ExecutionResult, CliFailure> {
    let parsed = parse_args(args)?;
    let state_root = parsed
        .options
        .get("--state-root")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".eggplan"));
    dispatch(parsed, state_root)
}

fn parse_args(args: &[String]) -> Result<ParsedArgs, CliFailure> {
    if args.is_empty() {
        return Err(failure("help", "usage", usage(), false));
    }
    let mut command = None;
    let mut positional = Vec::new();
    let mut options = BTreeMap::new();
    let mut flags = BTreeSet::new();
    let mut json = false;
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if arg == "--json" {
            json = true;
        } else if arg == "--recover-pending" {
            flags.insert(arg.clone());
        } else if arg == "--help" || arg == "-h" {
            flags.insert("--help".into());
        } else if is_value_option(arg) {
            i += 1;
            let value = args.get(i).ok_or_else(|| {
                failure(
                    command.as_deref().unwrap_or("usage"),
                    "usage",
                    format!("missing value for {arg}"),
                    json,
                )
            })?;
            if options.insert(arg.clone(), value.clone()).is_some() {
                return Err(failure(
                    command.as_deref().unwrap_or("usage"),
                    "usage",
                    format!("duplicate option {arg}"),
                    json,
                ));
            }
        } else if arg.starts_with('-') {
            return Err(failure(
                command.as_deref().unwrap_or("usage"),
                "usage",
                format!("unknown option {arg}"),
                json,
            ));
        } else if command.is_none() {
            command = Some(arg.clone());
        } else {
            positional.push(arg.clone());
        }
        i += 1;
    }
    let command = command
        .or_else(|| flags.contains("--help").then(|| "help".into()))
        .ok_or_else(|| failure("usage", "usage", usage(), json))?;
    Ok(ParsedArgs {
        command,
        positional,
        options,
        flags,
        json,
    })
}

fn is_value_option(value: &str) -> bool {
    matches!(
        value,
        "--state-root"
            | "--input"
            | "--expected-revision"
            | "--provider-policy"
            | "--status"
            | "--blocker"
            | "--next-action"
    )
}

fn dispatch(args: ParsedArgs, state_root: PathBuf) -> Result<ExecutionResult, CliFailure> {
    let command = args.command.clone();
    validate_command_options(&args)?;
    match command.as_str() {
        "init" => {
            require_positions(&args, 0, 0)?;
            let store = RepositoryStore::open(&state_root)
                .map_err(|error| repo_failure(&command, error, args.json))?;
            let data = InitData {
                repository_id: store.repository_id().into(),
            };
            let human = format!("Initialized {}", state_root.display());
            success(&args, data, Vec::new(), human)
        }
        "new" => {
            require_positions(&args, 0, 0)?;
            let input = required_option(&args, "--input")?;
            let bytes = read_bounded(Path::new(input), MAX_INPUT_BYTES, &command, args.json)?;
            let plan = parse_plan(&bytes)
                .map_err(|error| failure(&command, "invalid_plan", error.to_string(), args.json))?;
            if plan.revision != 0 || plan.status != PlanStatus::Draft {
                return Err(failure(
                    &command,
                    "invalid_plan",
                    "new Plan must be revision zero and Draft",
                    args.json,
                ));
            }
            let store = RepositoryStore::open(&state_root)
                .map_err(|error| repo_failure(&command, error, args.json))?;
            store
                .create(&plan)
                .map_err(|error| repo_failure(&command, error, args.json))?;
            let data = NewData {
                plan_id: plan.id.clone(),
                revision: plan.revision,
            };
            success(
                &args,
                data,
                Vec::new(),
                format!("Created {} at revision 0", plan.id),
            )
        }
        "show" => {
            require_positions(&args, 1, 1)?;
            let id = plan_id(&args.positional[0], &command, args.json)?;
            let store = read_store(&state_root, &command, args.json)?;
            let plan = store
                .get(&id)
                .map_err(|error| repo_failure(&command, error, args.json))?;
            let (subject, assessment, closure) =
                summary_context(&store, &plan, None, &command, args.json)?;
            let summary = summarize_plan(&plan, subject, assessment.as_ref(), closure);
            let ready = readiness_projection(&plan);
            let items = ready.items;
            let data = PlanDetail {
                plan: summary,
                items,
            };
            success(
                &args,
                data,
                Vec::new(),
                format!("{} {:?} revision {}", plan.id, plan.status, plan.revision),
            )
        }
        "status" => {
            require_positions(&args, 0, 1)?;
            let store = read_store(&state_root, &command, args.json)?;
            let ids = if let Some(raw) = args.positional.first() {
                vec![plan_id(raw, &command, args.json)?]
            } else {
                store
                    .list()
                    .map_err(|error| repo_failure(&command, error, args.json))?
            };
            let total_plans = ids.len();
            let mut warnings = Vec::new();
            let plans = ids
                .into_iter()
                .take(100)
                .map(|id| {
                    let plan = store
                        .get(&id)
                        .map_err(|error| repo_failure(&command, error, args.json))?;
                    let (subject, assessment, closure) =
                        summary_context(&store, &plan, None, &command, args.json)?;
                    if subject.is_none() {
                        warnings.push("current_subject_unavailable".into());
                    }
                    Ok(summarize_plan(&plan, subject, assessment.as_ref(), closure))
                })
                .collect::<Result<Vec<_>, CliFailure>>()?;
            let data = StatusData {
                plans,
                total_plans,
                truncated: total_plans > 100,
            };
            success(&args, data, warnings, format!("{total_plans} plan(s)"))
        }
        "ready" => {
            require_positions(&args, 1, 1)?;
            let id = plan_id(&args.positional[0], &command, args.json)?;
            let store = read_store(&state_root, &command, args.json)?;
            let plan = store
                .get(&id)
                .map_err(|error| repo_failure(&command, error, args.json))?;
            let projection = readiness_projection(&plan);
            let human = format!(
                "{} ready, {} waiting, {} not actionable",
                projection.ready_count, projection.waiting_count, projection.not_actionable_count
            );
            success(&args, projection, Vec::new(), human)
        }
        "graph" => {
            require_positions(&args, 1, 1)?;
            let id = plan_id(&args.positional[0], &command, args.json)?;
            let store = read_store(&state_root, &command, args.json)?;
            let plan = store
                .get(&id)
                .map_err(|error| repo_failure(&command, error, args.json))?;
            let graph = graph_projection(&plan);
            let human = graph
                .nodes
                .iter()
                .map(|node| node.item_id.as_str().to_string())
                .collect::<Vec<_>>()
                .join("\n");
            success(&args, graph, Vec::new(), human)
        }
        "check" => check_command(&args, &state_root),
        "activate" => mutate_activate(&args, &state_root),
        "item" => mutate_item(&args, &state_root),
        "evidence" => evidence_command(&args, &state_root),
        "assess" => assess_command(&args, &state_root),
        "close" => close_command(&args, &state_root),
        "closure" => closure_command(&args, &state_root),
        "registry" => registry_command(&args, &state_root),
        "help" | "--help" | "-h" => success(&args, json!({"usage": usage()}), Vec::new(), usage()),
        _ => Err(failure(&command, "usage", usage(), args.json)),
    }
}

fn validate_command_options(args: &ParsedArgs) -> Result<(), CliFailure> {
    let mut allowed = BTreeSet::from(["--state-root"]);
    let mut allowed_flags = BTreeSet::new();
    match args.command.as_str() {
        "init" => {}
        "new" => {
            allowed.insert("--input");
        }
        "show" | "status" | "ready" | "graph" | "registry" | "closure" | "evidence" => {}
        "check" => {
            allowed_flags.insert("--recover-pending");
            allowed.insert("--provider-policy");
        }
        "activate" => {
            allowed.insert("--expected-revision");
        }
        "item" => {
            allowed.extend([
                "--expected-revision",
                "--status",
                "--blocker",
                "--next-action",
            ]);
        }
        "assess" => {
            allowed.insert("--provider-policy");
        }
        "close" => {
            allowed.extend(["--expected-revision", "--provider-policy"]);
        }
        "help" => {
            allowed_flags.insert("--help");
        }
        _ => return Ok(()),
    }
    if let Some(option) = args
        .options
        .keys()
        .find(|option| !allowed.contains(option.as_str()))
    {
        return Err(failure(
            &args.command,
            "usage",
            format!("option {option} is not valid for this command"),
            args.json,
        ));
    }
    if let Some(flag) = args
        .flags
        .iter()
        .find(|flag| !allowed_flags.contains(flag.as_str()))
    {
        return Err(failure(
            &args.command,
            "usage",
            format!("flag {flag} is not valid for this command"),
            args.json,
        ));
    }
    Ok(())
}

fn check_command(args: &ParsedArgs, root: &Path) -> Result<ExecutionResult, CliFailure> {
    let command = "check";
    require_positions(args, 0, 1)?;
    let mut store = read_store(root, command, args.json)?;
    let pending = store
        .pending_closures()
        .map_err(|error| repo_failure(command, error, args.json))?;
    let mut warnings = Vec::new();
    if !pending.is_empty() {
        if args.flags.contains("--recover-pending") {
            RepositoryStore::open(root).map_err(|error| repo_failure(command, error, args.json))?;
            warnings.push(format!("recovered_pending_closures:{}", pending.len()));
            store = read_store(root, command, args.json)?;
        } else {
            return Err(failure(
                command,
                "recovery_required",
                format!(
                    "pending closures require explicit --recover-pending: {}",
                    pending
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(",")
                ),
                args.json,
            ));
        }
    } else if args.flags.contains("--recover-pending") {
        // Explicit recovery request is safe and idempotent even when no work is pending.
        RepositoryStore::open(root).map_err(|error| repo_failure(command, error, args.json))?;
        store = read_store(root, command, args.json)?;
    }
    let ids = if let Some(raw) = args.positional.first() {
        vec![plan_id(raw, command, args.json)?]
    } else {
        store
            .list()
            .map_err(|error| repo_failure(command, error, args.json))?
    };
    let policy = args
        .options
        .get("--provider-policy")
        .map(|path| load_policy(path, args.json))
        .transpose()?;
    let empty_registry = ProviderRegistry::default();
    let registry = policy
        .as_ref()
        .map_or(&empty_registry, |policy| &policy.registry);
    let mut observation_count = 0;
    let mut supersession_count = 0;
    let mut closure_count = 0;
    let mut checked_plans = Vec::new();
    for id in &ids {
        let plan = store
            .get(id)
            .map_err(|error| repo_failure(command, error, args.json))?;
        let observations = store
            .list_observations(id)
            .map_err(|error| repo_failure(command, error, args.json))?;
        let supersessions = store
            .list_supersessions(id)
            .map_err(|error| repo_failure(command, error, args.json))?;
        observation_count += observations.len();
        supersession_count += supersessions.len();
        let closure = store
            .closure_record(id)
            .map_err(|error| repo_failure(command, error, args.json))?;
        closure_count += usize::from(closure.is_some());
        let current_subject = store.subject_source().capture().ok();
        let (state, assessment_status, reason_codes) = if let Some(subject) = &current_subject {
            let effective = effective_observations(&observations, &supersessions)
                .map_err(|message| failure(command, "corrupt_evidence", message, args.json))?;
            let effective: Vec<_> = effective.into_iter().cloned().collect();
            let assessment = assess_plan(&plan, subject, &effective, registry);
            let mut codes: Vec<_> = assessment.reasons.iter().map(reason_code).collect();
            codes.extend(
                assessment
                    .items
                    .iter()
                    .flat_map(|item| &item.reasons)
                    .map(reason_code),
            );
            let stale_closure = closure
                .as_ref()
                .is_some_and(|record| record.candidate.subject != *subject);
            if stale_closure {
                codes.push("closure_subject_stale".into());
            }
            codes.sort();
            codes.dedup();
            let has_stale_subject =
                stale_closure || codes.iter().any(|code| code == "stale_subject");
            let state = if has_stale_subject {
                "stale"
            } else {
                assessment_state(assessment.status, &observations)
            };
            (state.to_string(), Some(assessment.status), codes)
        } else {
            warnings.push(format!("current_subject_unavailable:{id}"));
            (
                "unavailable".into(),
                None,
                vec!["current_subject_unavailable".into()],
            )
        };
        checked_plans.push(CheckedPlan {
            plan_id: id.clone(),
            integrity: "valid".into(),
            state,
            assessment_status,
            reason_codes,
            current_subject_available: current_subject.is_some(),
        });
    }
    let abandoned = store
        .abandoned_staging_files()
        .map_err(|error| repo_failure(command, error, args.json))?
        .len();
    let plans_truncated = checked_plans.len() > 100;
    checked_plans.truncate(100);
    let data = CheckData {
        repository_id: store.repository_id().into(),
        plans_checked: ids.len(),
        observations_checked: observation_count,
        supersessions_checked: supersession_count,
        closures_checked: closure_count,
        abandoned_staging_count: abandoned,
        plans: checked_plans,
        plans_truncated,
    };
    if abandoned > 0 {
        warnings.push(format!("abandoned_staging_files:{abandoned}"));
    }
    success(
        args,
        data,
        warnings,
        format!(
            "checked {} plan(s), {observation_count} observation(s), {closure_count} closure(s)",
            ids.len()
        ),
    )
}

fn mutate_activate(args: &ParsedArgs, root: &Path) -> Result<ExecutionResult, CliFailure> {
    require_positions(args, 1, 1)?;
    let expected = expected_revision(args)?;
    let id = plan_id(&args.positional[0], "activate", args.json)?;
    let store =
        RepositoryStore::open(root).map_err(|error| repo_failure("activate", error, args.json))?;
    let mut plan = store
        .get(&id)
        .map_err(|error| repo_failure("activate", error, args.json))?;
    if plan.revision != expected {
        return Err(failure(
            "activate",
            "revision_conflict",
            format!("expected revision {expected}, current {}", plan.revision),
            args.json,
        ));
    }
    if !matches!(plan.status, PlanStatus::Draft | PlanStatus::Blocked) {
        return Err(failure(
            "activate",
            "invalid_transition",
            "only Draft or Blocked Plans can be activated",
            args.json,
        ));
    }
    plan.revision = plan
        .revision
        .checked_add(1)
        .ok_or_else(|| failure("activate", "invalid_update", "revision overflow", args.json))?;
    plan.status = PlanStatus::Active;
    let updated = store
        .compare_and_swap(&id, expected, &plan)
        .map_err(|error| repo_failure("activate", error, args.json))?;
    let data = MutationData {
        plan_id: updated.id.clone(),
        revision: updated.revision,
        status: updated.status.clone(),
    };
    success(
        args,
        data,
        Vec::new(),
        format!("{} active at revision {}", updated.id, updated.revision),
    )
}

fn mutate_item(args: &ParsedArgs, root: &Path) -> Result<ExecutionResult, CliFailure> {
    if args.positional.first().map(String::as_str) != Some("update") {
        return Err(failure(
            "item",
            "usage",
            "expected: item update PLAN_ID ITEM_ID",
            args.json,
        ));
    }
    require_positions(args, 3, 3)?;
    let id = plan_id(&args.positional[1], "item update", args.json)?;
    let item_id = PlanItemId::new(args.positional[2].clone())
        .map_err(|error| failure("item update", "invalid_id", error.to_string(), args.json))?;
    let expected = expected_revision(args)?;
    if !args.options.contains_key("--status")
        && !args.options.contains_key("--blocker")
        && !args.options.contains_key("--next-action")
    {
        return Err(failure(
            "item update",
            "usage",
            "provide at least one of --status, --blocker, or --next-action",
            args.json,
        ));
    }
    let store = RepositoryStore::open(root)
        .map_err(|error| repo_failure("item update", error, args.json))?;
    let mut plan = store
        .get(&id)
        .map_err(|error| repo_failure("item update", error, args.json))?;
    if plan.revision != expected {
        return Err(failure(
            "item update",
            "revision_conflict",
            format!("expected revision {expected}, current {}", plan.revision),
            args.json,
        ));
    }
    let item = plan
        .items
        .iter_mut()
        .find(|item| item.id == item_id)
        .ok_or_else(|| {
            failure(
                "item update",
                "item_not_found",
                item_id.to_string(),
                args.json,
            )
        })?;
    if let Some(raw) = args.options.get("--status") {
        item.status = serde_json::from_value(json!(raw)).map_err(|_| {
            failure(
                "item update",
                "invalid_status",
                "unknown item status",
                args.json,
            )
        })?;
    }
    if let Some(value) = args.options.get("--blocker") {
        item.blocker = (!value.is_empty()).then(|| value.clone());
    }
    if let Some(value) = args.options.get("--next-action") {
        item.next_action = (!value.is_empty()).then(|| value.clone());
    }
    plan.revision = plan.revision.checked_add(1).ok_or_else(|| {
        failure(
            "item update",
            "invalid_update",
            "revision overflow",
            args.json,
        )
    })?;
    let updated = store
        .compare_and_swap(&id, expected, &plan)
        .map_err(|error| repo_failure("item update", error, args.json))?;
    let data = MutationData {
        plan_id: updated.id.clone(),
        revision: updated.revision,
        status: updated.status.clone(),
    };
    success(
        args,
        data,
        Vec::new(),
        format!("{} updated at revision {}", item_id, updated.revision),
    )
}

fn evidence_command(args: &ParsedArgs, root: &Path) -> Result<ExecutionResult, CliFailure> {
    if args.positional.is_empty() {
        return Err(failure(
            "evidence",
            "usage",
            "expected evidence list, show, or supersessions",
            args.json,
        ));
    }
    let action = args.positional[0].as_str();
    match action {
        "list" => {
            require_positions(args, 2, 2)?;
            let id = plan_id(&args.positional[1], "evidence list", args.json)?;
            let store = read_store(root, "evidence list", args.json)?;
            let rows = store
                .list_observations(&id)
                .map_err(|error| repo_failure("evidence list", error, args.json))?;
            let total_count = rows.len();
            let observations: Vec<_> = rows
                .into_iter()
                .take(MAX_EVIDENCE_ROWS)
                .map(evidence_brief)
                .collect();
            let data = EvidenceListData {
                observations,
                total_count,
                truncated: total_count > MAX_EVIDENCE_ROWS,
            };
            success(
                args,
                data,
                Vec::new(),
                format!("{total_count} observation(s)"),
            )
        }
        "show" => {
            require_positions(args, 3, 3)?;
            let id = plan_id(&args.positional[1], "evidence show", args.json)?;
            let observation_id =
                EvidenceObservationId::new(args.positional[2].clone()).map_err(|error| {
                    failure("evidence show", "invalid_id", error.to_string(), args.json)
                })?;
            let store = read_store(root, "evidence show", args.json)?;
            let observation = store
                .get_observation(&id, &observation_id)
                .map_err(|error| repo_failure("evidence show", error, args.json))?;
            success(
                args,
                observation.clone(),
                Vec::new(),
                format!(
                    "{} {:?} {:?}",
                    observation_id,
                    observation.kind(),
                    observation.status()
                ),
            )
        }
        "supersessions" => {
            require_positions(args, 2, 2)?;
            let id = plan_id(&args.positional[1], "evidence supersessions", args.json)?;
            let store = read_store(root, "evidence supersessions", args.json)?;
            let rows = store
                .list_supersessions(&id)
                .map_err(|error| repo_failure("evidence supersessions", error, args.json))?;
            let total_count = rows.len();
            let records = rows.into_iter().take(MAX_SUPERSESSION_ROWS).collect();
            let data = SupersessionListData {
                records,
                total_count,
                truncated: total_count > MAX_SUPERSESSION_ROWS,
            };
            success(
                args,
                data,
                Vec::new(),
                format!("{total_count} supersession(s)"),
            )
        }
        _ => Err(failure(
            "evidence",
            "usage",
            "expected evidence list, show, or supersessions",
            args.json,
        )),
    }
}

fn assess_command(args: &ParsedArgs, root: &Path) -> Result<ExecutionResult, CliFailure> {
    require_positions(args, 1, 1)?;
    let id = plan_id(&args.positional[0], "assess", args.json)?;
    let policy = load_policy(required_option(args, "--provider-policy")?, args.json)?;
    let store = read_store(root, "assess", args.json)?;
    let plan = store
        .get(&id)
        .map_err(|error| repo_failure("assess", error, args.json))?;
    let (assessment, _) = compute_assessment(&store, &plan, &policy, "assess", args.json)?;
    let data = assessment_data(&assessment);
    success(args, data, Vec::new(), format!("{:?}", assessment.status))
}

fn close_command(args: &ParsedArgs, root: &Path) -> Result<ExecutionResult, CliFailure> {
    require_positions(args, 1, 1)?;
    let expected = expected_revision(args)?;
    let id = plan_id(&args.positional[0], "close", args.json)?;
    let policy = load_policy(required_option(args, "--provider-policy")?, args.json)?;
    let store =
        RepositoryStore::open(root).map_err(|error| repo_failure("close", error, args.json))?;
    let plan = store
        .get(&id)
        .map_err(|error| repo_failure("close", error, args.json))?;
    if plan.revision != expected {
        return Err(failure(
            "close",
            "revision_conflict",
            format!("expected revision {expected}, current {}", plan.revision),
            args.json,
        ));
    }
    let (assessment, subject) = compute_assessment(&store, &plan, &policy, "close", args.json)?;
    let observations = store
        .list_observations(&id)
        .map_err(|error| repo_failure("close", error, args.json))?;
    let supersessions = store
        .list_supersessions(&id)
        .map_err(|error| repo_failure("close", error, args.json))?;
    let candidate = ClosureCandidate::build(
        &plan,
        subject,
        assessment,
        &observations,
        &supersessions,
        policy.entries,
        now_ms().map_err(|message| failure("close", "clock_error", message, args.json))?,
    )
    .map_err(|message| failure("close", "closure_not_ready", message, args.json))?;
    let (_, record) = store
        .finalize_closure(
            &candidate,
            &candidate.subject,
            ClosureId::generate(),
            now_ms().map_err(|message| failure("close", "clock_error", message, args.json))?,
        )
        .map_err(|error| repo_failure("close", error, args.json))?;
    let summary = closure_summary(&record);
    let data = ClosureCreatedData {
        closure: summary.clone(),
    };
    success(
        args,
        data,
        Vec::new(),
        format!(
            "closed {} at revision {}",
            summary.plan_id, summary.final_plan_revision
        ),
    )
}

fn closure_command(args: &ParsedArgs, root: &Path) -> Result<ExecutionResult, CliFailure> {
    if args.positional.first().map(String::as_str) != Some("show") {
        return Err(failure(
            "closure",
            "usage",
            "expected: closure show PLAN_ID",
            args.json,
        ));
    }
    require_positions(args, 2, 2)?;
    let id = plan_id(&args.positional[1], "closure show", args.json)?;
    let store = read_store(root, "closure show", args.json)?;
    let record = store
        .closure_record(&id)
        .map_err(|error| repo_failure("closure show", error, args.json))?
        .ok_or_else(|| {
            failure(
                "closure show",
                "closure_not_found",
                id.to_string(),
                args.json,
            )
        })?;
    let summary = closure_summary(&record);
    success(
        args,
        summary,
        Vec::new(),
        format!("closure {} for {}", record.id, record.candidate.plan_id),
    )
}

fn registry_command(args: &ParsedArgs, root: &Path) -> Result<ExecutionResult, CliFailure> {
    if args.positional.first().map(String::as_str) != Some("render") {
        return Err(failure(
            "registry",
            "usage",
            "expected: registry render",
            args.json,
        ));
    }
    require_positions(args, 1, 1)?;
    let store = read_store(root, "registry render", args.json)?;
    let ids = store
        .list()
        .map_err(|error| repo_failure("registry render", error, args.json))?;
    let mut plans = Vec::with_capacity(ids.len());
    let mut warnings = Vec::new();
    let empty_registry = ProviderRegistry::default();
    let has_plans = !ids.is_empty();
    for id in ids {
        let plan = store
            .get(&id)
            .map_err(|error| repo_failure("registry render", error, args.json))?;
        let closure = store
            .closure_record(&id)
            .map_err(|error| repo_failure("registry render", error, args.json))?
            .is_some();
        let assessment = if let Ok(subject) = store.subject_source().capture() {
            let observations = store
                .list_observations(&id)
                .map_err(|error| repo_failure("registry render", error, args.json))?;
            let supersessions = store
                .list_supersessions(&id)
                .map_err(|error| repo_failure("registry render", error, args.json))?;
            let effective =
                effective_observations(&observations, &supersessions).map_err(|message| {
                    failure("registry render", "corrupt_evidence", message, args.json)
                })?;
            let effective: Vec<_> = effective.into_iter().cloned().collect();
            Some(assess_plan(&plan, &subject, &effective, &empty_registry))
        } else {
            warnings.push(format!("current_subject_unavailable:{id}"));
            None
        };
        plans.push((plan, closure, assessment));
    }
    let projection = registry_projection(store.repository_id(), plans);
    let human = format!(
        "{} plan(s) in {}",
        projection.plan_count, projection.repository_id
    );
    if has_plans {
        warnings
            .push("assessment_uses_empty_provider_registry:no provider policy was supplied".into());
    }
    success(args, projection, warnings, human)
}

fn summary_context(
    store: &RepositoryStore,
    plan: &Plan,
    policy: Option<&LoadedPolicy>,
    command: &str,
    json: bool,
) -> Result<(Option<SubjectRevision>, Option<PlanAssessment>, bool), CliFailure> {
    let closure = store
        .closure_record(&plan.id)
        .map_err(|error| repo_failure(command, error, json))?
        .is_some();
    let current_subject = store.subject_source().capture().ok();
    let assessment = if let Some(policy) = policy.filter(|_| current_subject.is_some()) {
        Some(compute_assessment(store, plan, policy, command, json)?.0)
    } else if let Some(subject) = &current_subject {
        let observations = store
            .list_observations(&plan.id)
            .map_err(|error| repo_failure(command, error, json))?;
        let supersessions = store
            .list_supersessions(&plan.id)
            .map_err(|error| repo_failure(command, error, json))?;
        let effective = effective_observations(&observations, &supersessions)
            .map_err(|message| failure(command, "corrupt_evidence", message, json))?;
        Some(assess_plan(
            plan,
            subject,
            &effective.into_iter().cloned().collect::<Vec<_>>(),
            &ProviderRegistry::default(),
        ))
    } else {
        None
    };
    Ok((current_subject, assessment, closure))
}

fn compute_assessment(
    store: &RepositoryStore,
    plan: &Plan,
    policy: &LoadedPolicy,
    command: &str,
    json: bool,
) -> Result<(PlanAssessment, SubjectRevision), CliFailure> {
    let subject = store
        .subject_source()
        .capture()
        .map_err(|error| failure(command, "subject_unavailable", error.to_string(), json))?;
    let observations = store
        .list_observations(&plan.id)
        .map_err(|error| repo_failure(command, error, json))?;
    let supersessions = store
        .list_supersessions(&plan.id)
        .map_err(|error| repo_failure(command, error, json))?;
    let effective = effective_observations(&observations, &supersessions)
        .map_err(|message| failure(command, "corrupt_evidence", message, json))?;
    Ok((
        assess_plan(
            plan,
            &subject,
            &effective.into_iter().cloned().collect::<Vec<_>>(),
            &policy.registry,
        ),
        subject,
    ))
}

fn assessment_data(assessment: &PlanAssessment) -> AssessmentData {
    let mut reason_codes: Vec<_> = assessment.reasons.iter().map(reason_code).collect();
    reason_codes.sort();
    reason_codes.dedup();
    let total_items = assessment.items.len();
    let items = assessment
        .items
        .iter()
        .take(MAX_ASSESSMENT_ROWS)
        .map(|item| {
            let mut reasons = item.reasons.iter().map(reason_code).collect::<Vec<_>>();
            reasons.sort();
            reasons.dedup();
            let total_reasons = reasons.len();
            reasons.truncate(MAX_REASONS_PER_ROW);
            AssessmentItem {
                item_id: item.item_id.clone(),
                status: item.status,
                reason_codes: reasons.clone(),
                total_reasons,
                reasons_truncated: total_reasons > reasons.len(),
            }
        })
        .collect();
    AssessmentData {
        plan_id: assessment.plan_id.clone(),
        plan_revision: assessment.plan_revision,
        status: assessment.status,
        reason_codes,
        items,
        total_items,
        items_truncated: total_items > MAX_ASSESSMENT_ROWS,
    }
}

fn assessment_state(
    status: AssessmentStatus,
    observations: &[EvidenceObservation],
) -> &'static str {
    match status {
        AssessmentStatus::Complete => "complete",
        AssessmentStatus::InvalidOrStale => "invalid_or_stale",
        AssessmentStatus::EvidenceMissingOrUnavailable
            if observations
                .iter()
                .any(|observation| observation.status() == EvidenceStatus::Unavailable) =>
        {
            "unavailable"
        }
        AssessmentStatus::EvidenceFailed => "failed",
        AssessmentStatus::Blocked => "blocked",
        AssessmentStatus::InFlight => "in_flight",
        AssessmentStatus::AwaitingHumanJudgment => "awaiting_human_judgment",
        AssessmentStatus::Inconclusive => "inconclusive",
        AssessmentStatus::EvidenceMissingOrUnavailable
        | AssessmentStatus::ActionableWorkRemaining => "incomplete",
    }
}

fn closure_summary(record: &eggplan_core::ClosureRecord) -> ClosureSummary {
    ClosureSummary {
        closure_id: record.id.clone(),
        plan_id: record.candidate.plan_id.clone(),
        source_revision: record.candidate.source_revision,
        final_plan_revision: record.final_plan_revision,
        satisfying_observation_count: record.candidate.satisfying_observations.len(),
        provider_count: record.candidate.provider_policy.len(),
        subject: record.candidate.subject.clone(),
        content_digest: record.content_digest.clone(),
    }
}

fn evidence_brief(observation: EvidenceObservation) -> EvidenceBrief {
    EvidenceBrief {
        observation_id: observation.id().clone(),
        provider_id: observation.provider_id().clone(),
        kind: observation.kind(),
        status: observation.status(),
        observed_at_unix_ms: observation.observed_at_unix_ms(),
        content_digest: observation.content_digest().into(),
        artifact_count: observation.artifacts().len(),
        verification_digest: observation
            .verification_digest()
            .map(|value| value.as_str().into()),
    }
}

fn load_policy(path: &str, json: bool) -> Result<LoadedPolicy, CliFailure> {
    let bytes = read_bounded(Path::new(path), MAX_POLICY_BYTES, "provider-policy", json)?;
    let parsed: ProviderPolicyFile = serde_json::from_slice(&bytes)
        .map_err(|error| failure("provider-policy", "invalid_policy", error.to_string(), json))?;
    if parsed.schema_version != 1 {
        return Err(failure(
            "provider-policy",
            "unknown_schema",
            "provider policy schema_version must be 1",
            json,
        ));
    }
    if parsed.providers.len() > 128 {
        return Err(failure(
            "provider-policy",
            "policy_bound",
            "provider count exceeds 128",
            json,
        ));
    }
    let mut registry = ProviderRegistry::default();
    let mut entries = Vec::new();
    let mut ids = BTreeSet::new();
    for provider in parsed.providers {
        if !ids.insert(provider.provider_id.clone()) {
            return Err(failure(
                "provider-policy",
                "duplicate_provider",
                provider.provider_id.to_string(),
                json,
            ));
        }
        if provider.allowed_kinds.is_empty() || provider.allowed_kinds.len() > 10 {
            return Err(failure(
                "provider-policy",
                "invalid_policy",
                "allowed_kinds must contain 1 to 10 values",
                json,
            ));
        }
        let unique: BTreeSet<_> = provider.allowed_kinds.iter().copied().collect();
        if unique.len() != provider.allowed_kinds.len() {
            return Err(failure(
                "provider-policy",
                "duplicate_kind",
                provider.provider_id.to_string(),
                json,
            ));
        }
        let descriptor = ProviderDescriptor::new(
            provider.provider_id.clone(),
            provider.class.clone(),
            unique.iter().copied(),
        )
        .map_err(|error| failure("provider-policy", "invalid_policy", error.to_string(), json))?;
        registry.register_trusted(descriptor).map_err(|error| {
            failure("provider-policy", "invalid_policy", error.to_string(), json)
        })?;
        entries.push(ProviderPolicyEntry {
            provider_id: provider.provider_id,
            class: provider.class,
            allowed_kinds: unique,
        });
    }
    entries.sort_by(|a, b| a.provider_id.cmp(&b.provider_id));
    Ok(LoadedPolicy { registry, entries })
}

fn read_store(root: &Path, command: &str, json: bool) -> Result<RepositoryStore, CliFailure> {
    RepositoryStore::open_read_only(root).map_err(|error| repo_failure(command, error, json))
}

fn read_bounded(path: &Path, max: u64, command: &str, json: bool) -> Result<Vec<u8>, CliFailure> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| failure(command, "input_unavailable", error.to_string(), json))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(failure(
            command,
            "unsafe_input",
            "input must be a regular non-symlink file",
            json,
        ));
    }
    if metadata.len() > max {
        return Err(failure(
            command,
            "input_too_large",
            format!("input exceeds {max} bytes"),
            json,
        ));
    }
    let file = fs::File::open(path)
        .map_err(|error| failure(command, "input_unavailable", error.to_string(), json))?;
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(max.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| failure(command, "input_unavailable", error.to_string(), json))?;
    if bytes.len() as u64 > max {
        return Err(failure(
            command,
            "input_too_large",
            format!("input exceeds {max} bytes"),
            json,
        ));
    }
    Ok(bytes)
}

fn plan_id(raw: &str, command: &str, json: bool) -> Result<PlanId, CliFailure> {
    PlanId::new(raw.to_string())
        .map_err(|error| failure(command, "invalid_id", error.to_string(), json))
}

fn expected_revision(args: &ParsedArgs) -> Result<u64, CliFailure> {
    required_option(args, "--expected-revision")?
        .parse()
        .map_err(|_| {
            failure(
                &args.command,
                "invalid_revision",
                "expected revision must be a nonnegative integer",
                args.json,
            )
        })
}

fn required_option<'a>(args: &'a ParsedArgs, key: &str) -> Result<&'a str, CliFailure> {
    args.options.get(key).map(String::as_str).ok_or_else(|| {
        failure(
            &args.command,
            "usage",
            format!("required option {key} is missing"),
            args.json,
        )
    })
}

fn require_positions(args: &ParsedArgs, min: usize, max: usize) -> Result<(), CliFailure> {
    if args.positional.len() < min || args.positional.len() > max {
        return Err(failure(&args.command, "usage", usage(), args.json));
    }
    Ok(())
}

fn success<T: Serialize>(
    args: &ParsedArgs,
    value: T,
    warnings: Vec<String>,
    human: String,
) -> Result<ExecutionResult, CliFailure> {
    let data = serde_json::to_value(value)
        .map_err(|error| failure(&args.command, "serialization", error.to_string(), args.json))?;
    Ok(ExecutionResult {
        command: args.command.clone(),
        data,
        warnings,
        human,
        json: args.json,
    })
}

fn failure(command: &str, code: &str, message: impl Into<String>, json: bool) -> CliFailure {
    CliFailure {
        command: command.into(),
        code: code.into(),
        message: message.into(),
        json,
    }
}

fn repo_failure(command: &str, error: RepoError, json: bool) -> CliFailure {
    let (code, message) = match error {
        RepoError::NotFound(id) => ("plan_not_found", id.to_string()),
        RepoError::AlreadyExists(id) => ("plan_exists", id.to_string()),
        RepoError::Conflict {
            expected, current, ..
        } => (
            "revision_conflict",
            format!("expected revision {expected}, current {current}"),
        ),
        RepoError::InvalidTransition => (
            "invalid_transition",
            "plan or item transition is not allowed".into(),
        ),
        RepoError::GuardedClosureRequired => (
            "guarded_closure_required",
            "Plans may only close through guarded closure".into(),
        ),
        RepoError::InvalidUpdate => (
            "invalid_update",
            "candidate update violates revision or lifecycle constraints".into(),
        ),
        RepoError::Validation(error) => ("invalid_plan", error.to_string()),
        RepoError::RecoveryRequired(id) => (
            "recovery_required",
            format!("pending closure recovery required for {id}"),
        ),
        RepoError::Corrupt { reason, .. } => ("corrupt_state", reason),
        RepoError::InvalidPlan { reason, .. } => ("invalid_plan", reason),
        RepoError::UnsafePath(_) => ("unsafe_path", "repository contains an unsafe path".into()),
        RepoError::LockTimeout => ("lock_timeout", "repository lock timed out".into()),
        other => ("repository_error", other.to_string()),
    };
    failure(command, code, message, json)
}

fn now_ms() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().try_into().unwrap_or(u64::MAX))
        .map_err(|error| error.to_string())
}

fn usage() -> String {
    "eggplan [--state-root PATH] [--json] COMMAND\nCommands: init, new --input PLAN.json, show PLAN_ID, status [PLAN_ID], ready PLAN_ID, graph PLAN_ID, check [PLAN_ID] [--recover-pending], activate PLAN_ID --expected-revision N, item update PLAN_ID ITEM_ID --expected-revision N (--status STATUS | --blocker TEXT | --next-action TEXT), evidence list|show|supersessions, assess PLAN_ID --provider-policy FILE, close PLAN_ID --expected-revision N --provider-policy FILE, closure show PLAN_ID, registry render".into()
}
