#![forbid(unsafe_code)]

//! Bounded Markdown intent interchange. Markdown never creates evidence,
//! provider authority, a live subject, or a closure record.

use eggplan_core::{
    AcceptanceCriterion, Plan, PlanId, PlanItem, PlanItemId, PlanItemStatus, PlanStatus, bounds,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const NATIVE_MARKER: &str = "<!-- eggplan-markdown:v1 -->";
pub const NATIVE_FENCE: &str = "eggplan-plan-json";
pub const MAX_INPUT_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_LINES: usize = 20_000;
pub const MAX_LINE_BYTES: usize = 64 * 1024;
pub const MAX_SECTIONS: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportFormat {
    Auto,
    Eggplan,
    Codegg,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportReport {
    pub schema_version: u32,
    pub source_format: String,
    pub source_version: Option<u32>,
    pub source_name: Option<String>,
    pub plan_id: PlanId,
    pub original_source_status: Option<String>,
    pub original_source_revision: Option<String>,
    pub generated_ids: Vec<String>,
    pub preserved_fields: Vec<String>,
    pub dropped_fields: Vec<String>,
    pub lossy_mappings: Vec<String>,
    pub warning_codes: Vec<String>,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedPlan {
    pub plan: Plan,
    pub report: ImportReport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownError(pub String);

impl std::fmt::Display for MarkdownError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for MarkdownError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NativePayload {
    format_version: u32,
    plan_schema_version: u32,
    plan_id: PlanId,
    objective: String,
    source_revision: u64,
    source_status: PlanStatus,
    items: Vec<NativeItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NativeItem {
    id: PlanItemId,
    position: u32,
    parent: Option<PlanItemId>,
    dependencies: Vec<PlanItemId>,
    description: String,
    criteria: Vec<AcceptanceCriterion>,
    blocker: Option<String>,
    next_action: Option<String>,
}

pub fn detect_format(input: &[u8]) -> Result<ImportFormat, MarkdownError> {
    check_input(input)?;
    let text =
        std::str::from_utf8(input).map_err(|_| MarkdownError("input is not UTF-8".into()))?;
    if native_marker_count(text) > 0 {
        Ok(ImportFormat::Eggplan)
    } else {
        Ok(ImportFormat::Codegg)
    }
}

pub fn import(
    input: &[u8],
    format: ImportFormat,
    source_name: Option<&str>,
) -> Result<ImportedPlan, MarkdownError> {
    check_input(input)?;
    if source_name.is_some_and(|name| name.chars().take(513).count() > 512) {
        return Err(MarkdownError("source name exceeds 512 characters".into()));
    }
    let text =
        std::str::from_utf8(input).map_err(|_| MarkdownError("input is not UTF-8".into()))?;
    let format = match format {
        ImportFormat::Auto => detect_format(input)?,
        explicit => explicit,
    };
    match format {
        ImportFormat::Auto => unreachable!(),
        ImportFormat::Eggplan => import_native(text, source_name),
        ImportFormat::Codegg => import_codegg(text, source_name),
    }
}

pub fn render(plan: &Plan, has_closure: bool) -> Result<String, MarkdownError> {
    plan.validate()
        .map_err(|e| MarkdownError(format!("invalid plan: {e}")))?;
    if estimated_payload_size(plan) > MAX_INPUT_BYTES {
        return Err(MarkdownError(
            "plan intent exceeds Markdown interchange byte bound".into(),
        ));
    }
    let payload = NativePayload {
        format_version: 1,
        plan_schema_version: plan.schema_version,
        plan_id: plan.id.clone(),
        objective: plan.objective.clone(),
        source_revision: plan.revision,
        source_status: plan.status.clone(),
        items: plan
            .items
            .iter()
            .map(|item| NativeItem {
                id: item.id.clone(),
                position: item.position,
                parent: item.parent.clone(),
                dependencies: item.dependencies.clone(),
                description: item.description.clone(),
                criteria: item.criteria.clone(),
                blocker: item.blocker.clone(),
                next_action: item.next_action.clone(),
            })
            .collect(),
    };
    let json = serde_json::to_string_pretty(&payload).map_err(|e| MarkdownError(e.to_string()))?;
    let mut output = format!("{NATIVE_MARKER}\n\n```{NATIVE_FENCE}\n{json}\n```\n\n");
    output.push_str(&format!("# {}\n\n", markdown_text(&plan.objective)));
    output.push_str(&format!(
        "Plan: `{}` · revision {} · status `{}`\n\nItems: {}\n\nClosure record present: {} (display only)\n",
        plan.id,
        plan.revision,
        plan_status(plan.status.clone()),
        plan.items.len(),
        if has_closure { "yes" } else { "no" },
    ));
    let mut items: Vec<_> = plan.items.iter().collect();
    items.sort_by_key(|item| (item.position, item.id.clone()));
    for item in items {
        output.push_str(&format!(
            "\n## {} — {}\n\nStatus: `{}`\n\n{}\n",
            markdown_text(item.id.as_str()),
            markdown_text(&item.description),
            item_status(item.status),
            markdown_text(&item.description),
        ));
        if let Some(parent) = &item.parent {
            output.push_str(&format!("\nParent: `{parent}`\n"));
        }
        if !item.dependencies.is_empty() {
            let mut deps: Vec<_> = item.dependencies.iter().map(ToString::to_string).collect();
            deps.sort();
            output.push_str(&format!("\nDependencies: {}\n", deps.join(", ")));
        }
        if let Some(blocker) = &item.blocker {
            output.push_str(&format!("\nBlocker: {}\n", markdown_text(blocker)));
        }
        if let Some(next) = &item.next_action {
            output.push_str(&format!("\nNext action: {}\n", markdown_text(next)));
        }
        for criterion in &item.criteria {
            output.push_str(&format!(
                "\n### Acceptance {}\n\n{}\n",
                markdown_text(criterion.id.as_str()),
                markdown_text(&criterion.statement),
            ));
            for requirement in &criterion.requirements {
                output.push_str(&format!(
                    "\nEvidence requirement (display only): kind={:?}, provider={}, cardinality={:?}, min_count={} — {}{}\n",
                    requirement.kind,
                    requirement.provider.as_ref().map(ToString::to_string).unwrap_or_else(|| "unspecified".into()),
                    requirement.cardinality,
                    requirement.min_count,
                    markdown_text(&requirement.description),
                    requirement.expected_verification_digest.as_ref().map(|digest| format!(", verification={}", digest.as_str())).unwrap_or_default(),
                ));
            }
        }
    }
    if output.len() > MAX_INPUT_BYTES {
        return Err(MarkdownError(
            "rendered Markdown exceeds interchange byte limit".into(),
        ));
    }
    Ok(output)
}

fn import_native(text: &str, source_name: Option<&str>) -> Result<ImportedPlan, MarkdownError> {
    let fence = format!("```{NATIVE_FENCE}");
    let starts = native_payload_starts(text);
    if starts.len() != 1 {
        return Err(MarkdownError(
            "expected exactly one native intent payload".into(),
        ));
    }
    let markers = native_markers(text);
    if markers.len() != 1 || markers[0] != NATIVE_MARKER {
        return Err(MarkdownError("native v1 marker is missing".into()));
    }
    let start = starts[0] + fence.len();
    let tail = &text[start..];
    let tail = tail
        .strip_prefix('\n')
        .or_else(|| tail.strip_prefix("\r\n"))
        .ok_or_else(|| MarkdownError("native payload fence must start on its own line".into()))?;
    let end = tail
        .find("\r\n```")
        .or_else(|| tail.find("\n```"))
        .ok_or_else(|| MarkdownError("unterminated native payload".into()))?;
    let payload_json = tail[..end].replace("\r\n", "\n");
    let payload: NativePayload = serde_json::from_str(&payload_json)
        .map_err(|e| MarkdownError(format!("invalid native intent payload: {e}")))?;
    let canonical = serde_json::to_string_pretty(&payload)
        .map_err(|e| MarkdownError(format!("invalid native intent payload: {e}")))?;
    if payload_json != canonical {
        return Err(MarkdownError(
            "native intent payload is not canonical JSON".into(),
        ));
    }
    if payload.format_version != 1 {
        return Err(MarkdownError(format!(
            "unsupported native format version {}",
            payload.format_version
        )));
    }
    if payload.items.len() > bounds::MAX_ITEMS {
        return Err(MarkdownError(
            "native item count exceeds plan bounds".into(),
        ));
    }
    let source_status = plan_status(payload.source_status.clone());
    let mut items = Vec::with_capacity(payload.items.len());
    for item in payload.items {
        let status = if item.blocker.is_some() {
            PlanItemStatus::Blocked
        } else {
            PlanItemStatus::Pending
        };
        items.push(PlanItem {
            id: item.id,
            position: item.position,
            parent: item.parent,
            dependencies: item.dependencies,
            status,
            description: item.description,
            criteria: item.criteria,
            blocker: item.blocker,
            next_action: item.next_action,
        });
    }
    let mut plan = Plan {
        schema_version: payload.plan_schema_version,
        id: payload.plan_id.clone(),
        revision: 0,
        objective: payload.objective,
        status: PlanStatus::Draft,
        provenance: Default::default(),
        items,
        subject: None,
    };
    plan.provenance
        .insert("markdown_source_status".into(), source_status.clone());
    plan.provenance.insert(
        "markdown_source_revision".into(),
        payload.source_revision.to_string(),
    );
    let mut report = report_base("eggplan", Some(1), source_name, payload.plan_id);
    report.original_source_status = Some(source_status.clone());
    report.original_source_revision = Some(payload.source_revision.to_string());
    report.preserved_fields = vec![
        "plan_id".into(),
        "objective".into(),
        "item_ids_and_order".into(),
        "parent_and_dependencies".into(),
        "descriptions".into(),
        "acceptance_criteria_and_evidence_requirements".into(),
        "blocker_and_next_action".into(),
    ];
    report
        .warning_codes
        .push("source_lifecycle_not_authoritative".into());
    report
        .warning_codes
        .push("source_revision_provenance_only".into());
    report
        .warning_codes
        .push("markdown_evidence_not_imported".into());
    report
        .warning_codes
        .push("markdown_closure_not_imported".into());
    report
        .lossy_mappings
        .push("item_status_reset_to_pending_or_blocked_from_blocker_intent".into());
    if source_status != "draft" {
        report
            .lossy_mappings
            .push("source_plan_status_reset_to_draft".into());
    }
    if report.truncated {
        report.warning_codes.push("text_truncated".into());
    }
    plan.validate()
        .map_err(|e| MarkdownError(format!("invalid native intent plan: {e}")))?;
    Ok(ImportedPlan { plan, report })
}

#[derive(Default)]
struct Section {
    title: String,
    body: Vec<String>,
}

#[derive(Debug)]
struct WorkPackage {
    title: String,
    body: Vec<String>,
    dependencies: Vec<String>,
    id: PlanItemId,
}

fn import_codegg(text: &str, source_name: Option<&str>) -> Result<ImportedPlan, MarkdownError> {
    let lines: Vec<_> = text.lines().collect();
    let title = first_h1(&lines).unwrap_or("Imported implementation plan");
    let mut status = None;
    let mut revision = None;
    let mut sections: Vec<Section> = Vec::new();
    let mut current: Option<usize> = None;
    let mut packages_raw: Vec<(String, String, Vec<String>)> = Vec::new();
    let mut package_current: Option<usize> = None;
    let mut code_fence: Option<(char, usize)> = None;
    let mut saw_code_fence = false;
    for line in lines.iter().copied() {
        if let Some((marker, length, rest)) = fence_run(line) {
            match code_fence {
                Some((active, minimum)) => {
                    if marker == active && length >= minimum && rest.trim().is_empty() {
                        code_fence = None;
                    }
                }
                None => {
                    code_fence = Some((marker, length));
                    saw_code_fence = true;
                }
            }
            continue;
        }
        if code_fence.is_some() {
            continue;
        }
        if let Some(section_title) = line.strip_prefix("## ") {
            if sections.len() >= MAX_SECTIONS {
                return Err(MarkdownError("section count exceeds limit".into()));
            }
            sections.push(Section {
                title: section_title.trim().to_string(),
                body: Vec::new(),
            });
            current = Some(sections.len() - 1);
            package_current = None;
            continue;
        }
        if current.is_none() {
            if let Some(value) = line.strip_prefix("Status:") {
                let value = value.trim().to_string();
                if status.as_ref().is_some_and(|previous| previous != &value) {
                    return Err(MarkdownError("conflicting source Status metadata".into()));
                }
                status = Some(value);
            }
            if let Some(value) = line.strip_prefix("Repository baseline:") {
                let value = value.trim().trim_matches('`').to_string();
                if revision.as_ref().is_some_and(|previous| previous != &value) {
                    return Err(MarkdownError(
                        "conflicting Repository baseline metadata".into(),
                    ));
                }
                revision = Some(value);
            }
            continue;
        }
        let section = &mut sections[current.unwrap()];
        if is_work_package_section(&section.title) && line.starts_with("### Work package ") {
            if packages_raw.len() >= bounds::MAX_ITEMS {
                return Err(MarkdownError(
                    "work-package count exceeds plan bounds".into(),
                ));
            }
            let (label, package_title) = parse_package_heading(line.trim())?;
            packages_raw.push((label, package_title, Vec::new()));
            package_current = Some(packages_raw.len() - 1);
            continue;
        }
        if let Some(index) = package_current {
            packages_raw[index].2.push(line.to_string());
        } else {
            section.body.push(line.to_string());
        }
    }
    if code_fence.is_some() {
        return Err(MarkdownError("unterminated fenced code block".into()));
    }
    let objectives: Vec<_> = sections
        .iter()
        .filter(|s| section_name(&s.title) == "objective")
        .collect();
    if objectives.len() > 1 {
        return Err(MarkdownError(
            "multiple Objective sections are ambiguous".into(),
        ));
    }
    let objective = objectives
        .first()
        .map(|s| nonempty_body(&s.body))
        .unwrap_or_else(|| title.to_string());
    let identity = source_name
        .map(str::to_string)
        .unwrap_or_else(|| sha256(text.as_bytes()));
    let plan_id = PlanId::new(format!("ep_md_{}", short_hash(&identity)))
        .map_err(|e| MarkdownError(e.to_string()))?;
    let mut generated = vec![plan_id.to_string()];
    if packages_raw.len() > bounds::MAX_ITEMS {
        return Err(MarkdownError(
            "work-package count exceeds plan bounds".into(),
        ));
    }
    let mut keys = BTreeMap::new();
    let mut packages = Vec::new();
    let mut used = BTreeSet::new();
    for (label, package_title, body) in packages_raw {
        let normalized = normalize_heading(&package_title);
        let key = label.to_ascii_lowercase();
        if keys.contains_key(&key) {
            return Err(MarkdownError("duplicate work-package label".into()));
        }
        let id = PlanItemId::new(format!(
            "epi_md_{}",
            short_hash(&format!("{identity}\0{normalized}"))
        ))
        .map_err(|e| MarkdownError(e.to_string()))?;
        if !used.insert(id.clone()) {
            return Err(MarkdownError("generated work-package ID collision".into()));
        }
        keys.insert(key.clone(), id.clone());
        generated.push(id.to_string());
        let mut dependencies = Vec::new();
        let mut content = Vec::new();
        for line in body {
            if let Some(value) = line
                .trim()
                .strip_prefix("Dependencies:")
                .or_else(|| line.trim().strip_prefix("Depends on:"))
            {
                let references: Vec<_> = value.split(',').map(str::trim).collect();
                if references.is_empty() || references.iter().any(|value| value.is_empty()) {
                    return Err(MarkdownError("malformed dependency reference list".into()));
                }
                for target in references {
                    dependencies.push(target.trim_matches('`').to_ascii_lowercase());
                }
            } else if !line.trim().is_empty() {
                content.push(line.clone());
            }
        }
        packages.push(WorkPackage {
            title: package_title,
            body: content,
            dependencies,
            id,
        });
    }
    let mut items = Vec::new();
    let mut truncated = false;
    for (index, package) in packages.iter().enumerate() {
        let mut deps = Vec::new();
        for target in &package.dependencies {
            let resolved = keys.get(target).ok_or_else(|| {
                MarkdownError(format!("malformed dependency reference `{target}`"))
            })?;
            if resolved == &package.id {
                return Err(MarkdownError("work package depends on itself".into()));
            }
            deps.push(resolved.clone());
        }
        let text = if package.body.is_empty() {
            package.title.clone()
        } else {
            package.body.join("\n").trim().to_string()
        };
        let description = truncate_chars(&text, bounds::DESCRIPTION_CHARS, &mut truncated);
        items.push(PlanItem {
            id: package.id.clone(),
            position: index as u32,
            parent: None,
            dependencies: deps,
            status: PlanItemStatus::Pending,
            description,
            criteria: Vec::new(),
            blocker: None,
            next_action: None,
        });
    }
    let acceptance_sections: Vec<_> = sections
        .iter()
        .filter(|s| is_acceptance_section(&s.title))
        .collect();
    if acceptance_sections.len() > 1 {
        return Err(MarkdownError(
            "multiple Acceptance criteria sections are ambiguous".into(),
        ));
    }
    let mut plan_level_criteria = Vec::new();
    let (statements, criteria_truncated) = acceptance_sections
        .first()
        .map(|section| acceptance_lines(&section.body))
        .unwrap_or_default();
    truncated |= criteria_truncated;
    for (index, statement) in statements.into_iter().enumerate() {
        let (target, statement) = if let Some(scoped) = statement.strip_prefix("Work package ") {
            if let Some((label, statement)) = scoped.split_once(':') {
                let target = keys
                    .get(&label.trim().to_ascii_lowercase())
                    .ok_or_else(|| {
                        MarkdownError(format!(
                            "malformed acceptance work-package reference `{label}`"
                        ))
                    })?;
                (Some(target.clone()), statement.trim().to_string())
            } else {
                (None, statement)
            }
        } else {
            (None, statement)
        };
        let bounded = truncate_chars(&statement, bounds::CRITERION_CHARS, &mut truncated);
        let id = eggplan_core::CriterionId::new(format!(
            "epc_md_{}",
            short_hash(&format!("{identity}\0acceptance\0{index}\0{bounded}"))
        ))
        .map_err(|e| MarkdownError(e.to_string()))?;
        generated.push(id.to_string());
        let criterion = AcceptanceCriterion {
            id,
            statement: bounded,
            human_judgment_allowed: false,
            requirements: Vec::new(),
        };
        if let Some(target) = target {
            let item = items
                .iter_mut()
                .find(|item| item.id == target)
                .ok_or_else(|| MarkdownError("acceptance target does not resolve".into()))?;
            item.criteria.push(criterion);
        } else {
            plan_level_criteria.push(criterion);
        }
    }
    let criteria = plan_level_criteria;
    if !criteria.is_empty() {
        if items.len() >= bounds::MAX_ITEMS {
            return Err(MarkdownError(
                "cannot add acceptance intent item at plan item limit".into(),
            ));
        }
        let id = PlanItemId::new(format!(
            "epi_md_{}",
            short_hash(&format!("{identity}\0acceptance-intent"))
        ))
        .map_err(|e| MarkdownError(e.to_string()))?;
        if !used.insert(id.clone()) {
            return Err(MarkdownError(
                "generated acceptance item ID collision".into(),
            ));
        }
        generated.push(id.to_string());
        let deps = items.iter().map(|item| item.id.clone()).collect();
        items.push(PlanItem {
            id,
            position: items.len() as u32,
            parent: None,
            dependencies: deps,
            status: PlanItemStatus::Pending,
            description: "Imported plan-level acceptance criteria (intent only)".into(),
            criteria,
            blocker: None,
            next_action: None,
        });
    }
    let mut plan = Plan::new(
        plan_id.clone(),
        truncate_chars(&objective, bounds::OBJECTIVE_CHARS, &mut truncated),
        items,
    )
    .map_err(|e| MarkdownError(format!("imported plan is invalid: {e}")))?;
    let mut report = report_base("codegg", None, source_name, plan_id);
    report.original_source_status = status.as_ref().map(|value| {
        if value.chars().count() > 512 {
            truncated = true;
        }
        value.chars().take(512).collect()
    });
    report.original_source_revision = revision.as_ref().map(|value| {
        if value.chars().count() > 512 {
            truncated = true;
        }
        value.chars().take(512).collect()
    });
    if let Some(value) = status.as_deref() {
        plan.provenance.insert(
            "markdown_source_status".into(),
            truncate_chars(value, bounds::PROVENANCE_CHARS, &mut truncated),
        );
    }
    if let Some(value) = revision.as_deref() {
        plan.provenance.insert(
            "markdown_source_revision".into(),
            truncate_chars(value, bounds::PROVENANCE_CHARS, &mut truncated),
        );
    }
    report.generated_ids = generated;
    report.preserved_fields = vec![
        "objective".into(),
        "work_package_order".into(),
        "work_package_text".into(),
    ];
    if !acceptance_sections.is_empty() {
        report.preserved_fields.push("acceptance_criteria".into());
    }
    report
        .dropped_fields
        .push("source_status_and_lifecycle_authority".into());
    if status.is_some() {
        report
            .warning_codes
            .push("source_lifecycle_not_authoritative".into());
    }
    if revision.is_some() {
        report
            .warning_codes
            .push("source_revision_provenance_only".into());
    }
    report.warning_codes.push("generated_item_id".into());
    report
        .warning_codes
        .push("markdown_evidence_not_imported".into());
    report
        .warning_codes
        .push("markdown_closure_not_imported".into());
    report
        .lossy_mappings
        .push("ordered_sections_not_dependency_edges".into());
    if packages
        .iter()
        .any(|package| !package.dependencies.is_empty())
    {
        report.preserved_fields.push("explicit_dependencies".into());
    }
    for section in &sections {
        let normalized = section_name(&section.title);
        if matches!(normalized.as_str(), "objective") {
            continue;
        }
        if is_work_package_section(&section.title) || is_acceptance_section(&section.title) {
            continue;
        }
        if section.title.chars().count() > 512 {
            truncated = true;
        }
        report
            .dropped_fields
            .push(section.title.chars().take(512).collect());
        report.warning_codes.push("unmapped_section".into());
    }
    if saw_code_fence {
        report
            .dropped_fields
            .push("fenced_code_block_contents".into());
        report.warning_codes.push("unmapped_section".into());
    }
    report.truncated |= truncated;
    if report.truncated {
        report.warning_codes.push("text_truncated".into());
    }
    report.warning_codes.sort();
    report.warning_codes.dedup();
    report.plan_id = plan.id.clone();
    plan.validate()
        .map_err(|e| MarkdownError(format!("imported plan is invalid: {e}")))?;
    Ok(ImportedPlan { plan, report })
}

fn check_input(input: &[u8]) -> Result<(), MarkdownError> {
    if input.len() > MAX_INPUT_BYTES {
        return Err(MarkdownError("Markdown input exceeds byte limit".into()));
    }
    if input.contains(&0) {
        return Err(MarkdownError("Markdown input contains NUL".into()));
    }
    let text =
        std::str::from_utf8(input).map_err(|_| MarkdownError("input is not UTF-8".into()))?;
    let mut lines = 0;
    for line in text.lines() {
        lines += 1;
        if lines > MAX_LINES {
            return Err(MarkdownError("Markdown line count exceeds limit".into()));
        }
        if line.len() > MAX_LINE_BYTES {
            return Err(MarkdownError("Markdown line exceeds byte limit".into()));
        }
    }
    Ok(())
}

fn fence_run(line: &str) -> Option<(char, usize, &str)> {
    let trimmed = line.trim_start();
    let marker = trimmed.chars().next()?;
    if marker != '`' && marker != '~' {
        return None;
    }
    let length = trimmed.chars().take_while(|ch| *ch == marker).count();
    (length >= 3).then_some((marker, length, &trimmed[marker.len_utf8() * length..]))
}

fn native_markers(text: &str) -> Vec<&str> {
    let mut code_fence: Option<(char, usize)> = None;
    let mut markers = Vec::new();
    for line in text.lines() {
        if let Some((marker, length, rest)) = fence_run(line) {
            match code_fence {
                Some((active, minimum)) => {
                    if marker == active && length >= minimum && rest.trim().is_empty() {
                        code_fence = None;
                    }
                }
                None => code_fence = Some((marker, length)),
            }
            continue;
        }
        if code_fence.is_none() && line.trim().starts_with("<!-- eggplan-markdown:") {
            markers.push(line.trim());
        }
    }
    markers
}

fn native_marker_count(text: &str) -> usize {
    native_markers(text).len()
}

fn native_payload_starts(text: &str) -> Vec<usize> {
    let expected = format!("```{NATIVE_FENCE}");
    let mut code_fence: Option<(char, usize)> = None;
    let mut starts = Vec::new();
    let mut offset = 0;
    for raw_line in text.split_inclusive('\n') {
        let line = raw_line.trim_end_matches('\n').trim_end_matches('\r');
        if let Some((marker, length, rest)) = fence_run(line) {
            if code_fence.is_none() && line.trim() == expected {
                starts.push(offset);
            }
            match code_fence {
                Some((active, minimum)) => {
                    if marker == active && length >= minimum && rest.trim().is_empty() {
                        code_fence = None;
                    }
                }
                None => code_fence = Some((marker, length)),
            }
        }
        offset += raw_line.len();
    }
    starts
}

fn first_h1<'a>(lines: &'a [&str]) -> Option<&'a str> {
    let mut code_fence: Option<(char, usize)> = None;
    for line in lines {
        if let Some((marker, length, rest)) = fence_run(line) {
            match code_fence {
                Some((active, minimum)) => {
                    if marker == active && length >= minimum && rest.trim().is_empty() {
                        code_fence = None;
                    }
                }
                None => code_fence = Some((marker, length)),
            }
        } else if code_fence.is_none()
            && let Some(title) = line.strip_prefix("# ")
            && !title.trim().is_empty()
        {
            return Some(title.trim());
        }
    }
    None
}

fn estimated_payload_size(plan: &Plan) -> usize {
    fn add_text(size: &mut usize, text: &str) {
        // JSON may escape a control scalar as six ASCII bytes. This deliberately
        // overestimates ordinary UTF-8 so the bound is checked before encoding.
        *size = size.saturating_add(text.chars().count().saturating_mul(6));
    }
    let mut size = 8 * 1024usize;
    add_text(&mut size, plan.id.as_str());
    add_text(&mut size, &plan.objective);
    for item in &plan.items {
        size = size.saturating_add(512);
        add_text(&mut size, item.id.as_str());
        add_text(&mut size, &item.description);
        if let Some(blocker) = &item.blocker {
            add_text(&mut size, blocker);
        }
        if let Some(next) = &item.next_action {
            add_text(&mut size, next);
        }
        for id in &item.dependencies {
            add_text(&mut size, id.as_str());
        }
        if let Some(parent) = &item.parent {
            add_text(&mut size, parent.as_str());
        }
        for criterion in &item.criteria {
            size = size.saturating_add(512);
            add_text(&mut size, criterion.id.as_str());
            add_text(&mut size, &criterion.statement);
            for requirement in &criterion.requirements {
                size = size.saturating_add(512);
                add_text(&mut size, &requirement.description);
                if let Some(provider) = &requirement.provider {
                    add_text(&mut size, provider.as_str());
                }
                if let Some(digest) = &requirement.expected_verification_digest {
                    add_text(&mut size, digest.as_str());
                }
            }
        }
    }
    size
}

fn report_base(
    format: &str,
    version: Option<u32>,
    source_name: Option<&str>,
    plan_id: PlanId,
) -> ImportReport {
    ImportReport {
        schema_version: 1,
        source_format: format.into(),
        source_version: version,
        source_name: source_name.map(|s| s.chars().take(512).collect()),
        plan_id,
        original_source_status: None,
        original_source_revision: None,
        generated_ids: Vec::new(),
        preserved_fields: Vec::new(),
        dropped_fields: Vec::new(),
        lossy_mappings: Vec::new(),
        warning_codes: Vec::new(),
        truncated: source_name.is_some_and(|s| s.chars().take(513).count() > 512),
    }
}

fn parse_package_heading(line: &str) -> Result<(String, String), MarkdownError> {
    let value = line
        .strip_prefix("### Work package ")
        .ok_or_else(|| MarkdownError("invalid work-package heading".into()))?;
    let (label, title) = value
        .split_once('—')
        .or_else(|| value.split_once('-'))
        .ok_or_else(|| {
            MarkdownError("work-package heading must include a label and title".into())
        })?;
    let label = label.trim().trim_matches('`').to_string();
    let title = title.trim().to_string();
    if label.is_empty() || title.is_empty() || label.len() > 64 {
        return Err(MarkdownError("invalid work-package label or title".into()));
    }
    Ok((label, title))
}

fn section_name(title: &str) -> String {
    let value = title
        .trim()
        .trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c.is_whitespace());
    value.to_ascii_lowercase()
}

fn is_work_package_section(title: &str) -> bool {
    matches!(
        section_name(title).as_str(),
        "ordered work packages" | "work packages"
    )
}

fn is_acceptance_section(title: &str) -> bool {
    let name = section_name(title);
    name == "acceptance criteria" || name.ends_with(" acceptance criteria")
}

fn nonempty_body(lines: &[String]) -> String {
    let value = lines.join("\n").trim().to_string();
    if value.is_empty() {
        "Imported plan objective".into()
    } else {
        value
    }
}

fn acceptance_lines(lines: &[String]) -> (Vec<String>, bool) {
    let mut values: Vec<_> = lines
        .iter()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .map(strip_list_marker)
        .filter(|line| !line.is_empty())
        .collect();
    if values.len() == 1 && values[0].len() > 2_000 {
        values.truncate(1);
    }
    let truncated = values.len() > bounds::MAX_CRITERIA;
    values.truncate(bounds::MAX_CRITERIA);
    (values, truncated)
}

fn strip_list_marker(line: &str) -> String {
    if let Some(rest) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
        return rest.trim().to_string();
    }
    let digits = line.bytes().take_while(u8::is_ascii_digit).count();
    if digits > 0 {
        let rest = &line[digits..];
        if let Some(rest) = rest.strip_prefix(". ").or_else(|| rest.strip_prefix(") ")) {
            return rest.trim().to_string();
        }
    }
    line.to_string()
}

fn normalize_heading(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

fn short_hash(value: &str) -> String {
    sha256(value.as_bytes())[..24].to_string()
}
fn sha256(value: &[u8]) -> String {
    format!("{:x}", Sha256::digest(value))
}

fn truncate_chars(value: &str, max: usize, truncated: &mut bool) -> String {
    let mut chars = value.chars();
    let output: String = chars.by_ref().take(max).collect();
    if chars.next().is_some() {
        *truncated = true;
    }
    if output.trim().is_empty() {
        "Imported text".into()
    } else {
        output
    }
}

fn plan_status(status: PlanStatus) -> String {
    match status {
        PlanStatus::Draft => "draft",
        PlanStatus::Active => "active",
        PlanStatus::Blocked => "blocked",
        PlanStatus::Closed => "closed",
        PlanStatus::Cancelled => "cancelled",
    }
    .into()
}
fn item_status(status: PlanItemStatus) -> &'static str {
    match status {
        PlanItemStatus::Pending => "pending",
        PlanItemStatus::Actionable => "actionable",
        PlanItemStatus::InProgress => "in_progress",
        PlanItemStatus::Blocked => "blocked",
        PlanItemStatus::Completed => "completed",
        PlanItemStatus::Cancelled => "cancelled",
    }
}
fn markdown_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\r', "")
        .replace('\n', "<br>")
}

#[cfg(test)]
mod tests {
    use super::*;
    use eggplan_core::{
        EvidenceCardinality, EvidenceKind, EvidenceRequirement, PlanItem, PlanItemStatus,
        SubjectPolicy,
    };

    fn draft() -> Plan {
        Plan::new(
            PlanId::new("ep_md_test").unwrap(),
            "Markdown intent",
            vec![PlanItem {
                id: PlanItemId::new("epi_md_one").unwrap(),
                position: 0,
                parent: None,
                dependencies: vec![],
                status: PlanItemStatus::Pending,
                description: "Write parser".into(),
                criteria: vec![],
                blocker: None,
                next_action: Some("Add bounds".into()),
            }],
        )
        .unwrap()
    }

    #[test]
    fn native_render_is_deterministic_and_intent_roundtrips() {
        let plan = draft();
        let first = render(&plan, false).unwrap();
        assert_eq!(first, render(&plan, false).unwrap());
        let imported = import(first.as_bytes(), ImportFormat::Auto, None).unwrap();
        assert_eq!(imported.plan.id, plan.id);
        assert_eq!(imported.plan.objective, plan.objective);
        assert_eq!(imported.plan.items[0].id, plan.items[0].id);
        assert_eq!(
            imported.plan.items[0].next_action,
            plan.items[0].next_action
        );
        assert_eq!(imported.plan.status, PlanStatus::Draft);
        assert_eq!(imported.plan.revision, 0);
        let crlf = first.replace('\n', "\r\n");
        assert_eq!(
            import(crlf.as_bytes(), ImportFormat::Auto, None)
                .unwrap()
                .plan
                .items,
            plan.items
        );
    }

    #[test]
    fn native_import_keeps_status_only_as_provenance_and_resets_item_state() {
        for (revision, status) in [
            (0, PlanStatus::Draft),
            (2, PlanStatus::Active),
            (4, PlanStatus::Blocked),
            (9, PlanStatus::Closed),
        ] {
            let mut plan = draft();
            plan.revision = revision;
            plan.status = status.clone();
            plan.items[0].status = PlanItemStatus::Completed;
            plan.validate().unwrap();
            let text = render(&plan, true).unwrap();
            let imported = import(text.as_bytes(), ImportFormat::Auto, None).unwrap();
            assert_eq!(imported.plan.status, PlanStatus::Draft);
            assert_eq!(imported.plan.revision, 0);
            assert_eq!(imported.plan.items[0].status, PlanItemStatus::Pending);
            assert_eq!(
                imported.plan.provenance["markdown_source_status"],
                plan_status(status)
            );
            let revision_text = revision.to_string();
            assert_eq!(
                imported.report.original_source_revision.as_deref(),
                Some(revision_text.as_str())
            );
            assert!(
                imported
                    .report
                    .warning_codes
                    .contains(&"source_lifecycle_not_authoritative".into())
            );
            assert!(
                imported
                    .report
                    .warning_codes
                    .contains(&"markdown_closure_not_imported".into())
            );
        }
    }

    #[test]
    fn native_roundtrip_preserves_parent_dependencies_acceptance_and_requirements() {
        let source = "# Example\n## 1. Objective\n\nDo bounded work.\n## 2. Ordered work packages\n### Work package A — First\nDo first.\n### Work package B — Second\nDo second.\n## 3. Acceptance criteria\n- Finish both steps.\n";
        let mut source_plan = import(source.as_bytes(), ImportFormat::Codegg, Some("native.md"))
            .unwrap()
            .plan;
        source_plan.items[1].parent = Some(source_plan.items[0].id.clone());
        source_plan.schema_version = 1;
        source_plan.items.last_mut().unwrap().criteria[0]
            .requirements
            .push(EvidenceRequirement {
                description: "review report descriptor".into(),
                kind: EvidenceKind::Test,
                provider: None,
                subject_policy: SubjectPolicy::Exact,
                cardinality: EvidenceCardinality::Any,
                min_count: 1,
                allow_human_judgment: false,
                expected_verification_digest: None,
            });
        source_plan.validate().unwrap();
        let rendered = render(&source_plan, false).unwrap();
        let imported = import(rendered.as_bytes(), ImportFormat::Eggplan, None).unwrap();
        assert_eq!(imported.plan.items, source_plan.items);
    }

    #[test]
    fn native_rejects_duplicate_payload_and_unknown_fields() {
        let rendered = render(&draft(), false).unwrap();
        let duplicated = format!("{rendered}\n```eggplan-plan-json\n{{}}\n```\n");
        assert!(import(duplicated.as_bytes(), ImportFormat::Eggplan, None).is_err());
        let bad = format!(
            "{NATIVE_MARKER}\n\n```{NATIVE_FENCE}\n{{\"format_version\":1,\"future\":true}}\n```\n"
        );
        assert!(import(bad.as_bytes(), ImportFormat::Eggplan, None).is_err());
        let unsupported = render(&draft(), false)
            .unwrap()
            .replace("\"format_version\": 1", "\"format_version\": 9");
        assert!(import(unsupported.as_bytes(), ImportFormat::Auto, None).is_err());
    }

    #[test]
    fn codegg_ids_and_loss_report_are_deterministic_without_order_edges() {
        let source = "# Example\nStatus: implemented\n\n## 1. Objective\n\nDo bounded work.\n\n## 2. Ordered work packages\n\n### Work package A — First\n\nDo first.\n\n### Work package B — Second\n\nDo second.\n\n## 3. Acceptance criteria\n\n- Both imported.\n\n## 4. Scope\n\nOutside the subset.\n";
        let a = import(source.as_bytes(), ImportFormat::Auto, Some("fixture.md")).unwrap();
        let b = import(source.as_bytes(), ImportFormat::Auto, Some("fixture.md")).unwrap();
        assert_eq!(a.plan, b.plan);
        assert_eq!(a.report, b.report);
        assert_eq!(a.plan.status, PlanStatus::Draft);
        assert_eq!(a.plan.items[0].dependencies.len(), 0);
        assert_eq!(
            a.report.original_source_status.as_deref(),
            Some("implemented")
        );
        assert!(
            a.report
                .lossy_mappings
                .contains(&"ordered_sections_not_dependency_edges".into())
        );
        assert!(a.report.warning_codes.contains(&"unmapped_section".into()));
    }

    #[test]
    fn codegg_explicit_dependency_maps_and_bad_reference_fails() {
        let source = "# Example\n## 1. Objective\n\nDo it.\n## 2. Ordered work packages\n### Work package A — First\nDo first.\n### Work package B — Second\nDependencies: A\nDo second.\n";
        let imported = import(source.as_bytes(), ImportFormat::Codegg, Some("x.md")).unwrap();
        assert_eq!(
            imported.plan.items[1].dependencies,
            vec![imported.plan.items[0].id.clone()]
        );
        let bad = source.replace("Dependencies: A", "Dependencies: missing");
        assert!(import(bad.as_bytes(), ImportFormat::Codegg, Some("x.md")).is_err());
        let malformed = source.replace("Dependencies: A", "Dependencies: A,");
        assert!(import(malformed.as_bytes(), ImportFormat::Codegg, Some("x.md")).is_err());
        let ambiguous = source.replace("## 1. Objective", "## 1. Objective\n\n## 2. Objective");
        assert!(import(ambiguous.as_bytes(), ImportFormat::Codegg, Some("x.md")).is_err());
    }

    #[test]
    fn explicitly_scoped_acceptance_maps_to_named_work_package() {
        let source = "# Example\n## 1. Objective\n\nDo it.\n## 2. Ordered work packages\n### Work package A — First\nDo first.\n### Work package B — Second\nDo second.\n## 3. Acceptance criteria\n- Work package A: First package is valid.\n- Overall plan is ready for review.\n";
        let imported = import(source.as_bytes(), ImportFormat::Codegg, Some("x.md")).unwrap();
        assert_eq!(imported.plan.items[0].criteria.len(), 1);
        assert_eq!(
            imported.plan.items[0].criteria[0].statement,
            "First package is valid."
        );
        assert_eq!(imported.plan.items[2].criteria.len(), 1);
    }

    #[test]
    fn bounds_and_nul_are_rejected() {
        assert!(import(b"# x\0", ImportFormat::Auto, None).is_err());
        assert!(import(&vec![b'x'; MAX_INPUT_BYTES + 1], ImportFormat::Auto, None).is_err());
    }
}
