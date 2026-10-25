//! M004 — plan lifecycle transitions and plan structure editing through the CLI.
//!
//! The commands under test are `eggplan plan set-status`, `eggplan plan edit`,
//! and `eggplan plan export`, added under ADR-0005. The properties asserted
//! here are the ones the ADR makes promises about: the core transition table
//! decides legality, every mutation goes through the store's compare-and-swap
//! with an explicit revision, structure editing cannot smuggle a status change
//! or a closure, and the import path still creates only Draft plans.

use eggplan_core::{
    AcceptanceCriterion, EvidenceCardinality, EvidenceKind, EvidenceObservation,
    EvidenceObservationId, EvidenceObservationInput, EvidenceProviderId, EvidenceRequirement, Plan,
    PlanId, PlanItem, PlanItemId, PlanItemStatus, PlanStatus, SubjectPolicy, VerificationDigest,
};
use eggplan_repo::{PlanStore, RepositoryStore};
use git2::{Repository, Signature};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs, path::Path, process::Command};
use tempfile::tempdir;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_eggplan")
}

fn invoke(args: &[&str]) -> std::process::Output {
    Command::new(bin()).args(args).output().unwrap()
}

fn json_ok(output: &std::process::Output) -> Value {
    assert!(
        output.status.success(),
        "stderr: {}\nstdout: {}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["ok"], true);
    value
}

/// The machine code of a failed command, asserted against the JSON envelope.
fn error_code(output: &std::process::Output) -> String {
    assert_eq!(
        output.status.code(),
        Some(2),
        "stderr: {}\nstdout: {}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["ok"], false);
    value["error"]["code"].as_str().unwrap().to_string()
}

fn init_git(path: &Path) {
    let repository = Repository::init(path).unwrap();
    let mut index = repository.index().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repository.find_tree(tree_id).unwrap();
    let signature = Signature::now("Eggplan Test", "test@example.invalid").unwrap();
    repository
        .commit(Some("HEAD"), &signature, &signature, "fixture", &tree, &[])
        .unwrap();
}

fn provider_id() -> EvidenceProviderId {
    EvidenceProviderId::new("epp_m004").unwrap()
}

fn verification() -> VerificationDigest {
    VerificationDigest::new(format!("sha256:{}", "a".repeat(64))).unwrap()
}

/// A Draft plan with one item, one criterion, and one evidence requirement.
fn fixture_plan() -> Plan {
    let plan = Plan::new(
        PlanId::new("ep_m004_plan").unwrap(),
        "M004 fixture objective",
        vec![PlanItem {
            id: PlanItemId::new("epi_m004_step").unwrap(),
            position: 0,
            parent: None,
            dependencies: vec![],
            status: PlanItemStatus::Pending,
            description: "M004 step".into(),
            criteria: vec![AcceptanceCriterion {
                id: eggplan_core::CriterionId::new("epc_m004_pass").unwrap(),
                statement: "the fixture holds".into(),
                human_judgment_allowed: false,
                requirements: vec![EvidenceRequirement {
                    description: "fixture test".into(),
                    kind: EvidenceKind::Test,
                    provider: Some(provider_id()),
                    subject_policy: SubjectPolicy::Exact,
                    cardinality: EvidenceCardinality::Any,
                    min_count: 1,
                    allow_human_judgment: false,
                    expected_verification_digest: Some(verification()),
                }],
            }],
            blocker: None,
            next_action: None,
        }],
    )
    .unwrap();
    plan.validate().unwrap();
    plan
}

/// A state root holding one Draft plan.
struct Seed {
    temp: tempfile::TempDir,
    state: std::path::PathBuf,
}

impl Seed {
    fn new() -> Seed {
        let temp = tempdir().unwrap();
        let state = temp.path().join("state root");
        json_ok(&invoke(&[
            "--state-root",
            state.to_str().unwrap(),
            "init",
            "--json",
        ]));
        let input = temp.path().join("plan.json");
        fs::write(&input, serde_json::to_vec(&fixture_plan()).unwrap()).unwrap();
        json_ok(&invoke(&[
            "new",
            "--input",
            input.to_str().unwrap(),
            "--state-root",
            state.to_str().unwrap(),
            "--json",
        ]));
        Seed { temp, state }
    }

    fn root(&self) -> &str {
        self.state.to_str().unwrap()
    }

    fn path(&self, name: &str) -> std::path::PathBuf {
        self.temp.path().join(name)
    }

    /// The plan's canonical JSON, as `plan export` produces it.
    fn exported(&self) -> Value {
        let output = invoke(&[
            "plan",
            "export",
            "ep_m004_plan",
            "--state-root",
            self.root(),
            "--json",
        ]);
        json_ok(&output)["data"]["plan"].clone()
    }
}

/// A `plan set-status` invocation against the seeded plan.
fn set_status(seed: &Seed, revision: &str, status: &str) -> std::process::Output {
    invoke(&[
        "plan",
        "set-status",
        "ep_m004_plan",
        "--to",
        status,
        "--expected-revision",
        revision,
        "--state-root",
        seed.root(),
        "--json",
    ])
}

#[test]
fn plan_set_status_drives_every_legal_transition() {
    let seed = Seed::new();
    // Draft -> Active.
    let activated = json_ok(&set_status(&seed, "0", "active"));
    assert_eq!(activated["data"]["status"], "active");
    assert_eq!(activated["data"]["revision"], 1);
    // Active -> Blocked.
    let blocked = json_ok(&set_status(&seed, "1", "blocked"));
    assert_eq!(blocked["data"]["status"], "blocked");
    assert_eq!(blocked["data"]["revision"], 2);
    // Blocked -> Active.
    let reactivated = json_ok(&set_status(&seed, "2", "active"));
    assert_eq!(reactivated["data"]["status"], "active");
    assert_eq!(reactivated["data"]["revision"], 3);
    // Active -> Cancelled. `Cancelled` is terminal, so it is the last move.
    let cancelled = json_ok(&set_status(&seed, "3", "cancelled"));
    assert_eq!(cancelled["data"]["status"], "cancelled");
    assert_eq!(cancelled["data"]["revision"], 4);
    // Cancelled is terminal: nothing leaves it.
    assert_eq!(
        error_code(&set_status(&seed, "4", "active")),
        "invalid_transition"
    );
    assert_eq!(
        error_code(&set_status(&seed, "4", "cancelled")),
        "invalid_transition"
    );
}

#[test]
fn plan_set_status_rejects_transitions_the_core_table_forbids() {
    let seed = Seed::new();
    // Draft -> Blocked is not in the core transition table.
    assert_eq!(
        error_code(&set_status(&seed, "0", "blocked")),
        "invalid_transition"
    );
    // Draft -> Active is legal, so the plan can reach Active...
    json_ok(&set_status(&seed, "0", "active"));
    // ...but Active -> Draft is not.
    assert_eq!(
        error_code(&set_status(&seed, "1", "draft")),
        "invalid_transition"
    );
    // And a stale revision is a conflict, not a transition failure.
    assert_eq!(
        error_code(&set_status(&seed, "0", "blocked")),
        "revision_conflict"
    );
}

#[test]
fn plan_set_status_refuses_to_close_a_plan_directly() {
    let seed = Seed::new();
    let refused = set_status(&seed, "0", "closed");
    assert_eq!(error_code(&refused), "guarded_closure_required");
    let envelope: Value = serde_json::from_slice(&refused.stdout).unwrap();
    assert!(
        envelope["error"]["message"]
            .as_str()
            .unwrap()
            .contains("eggplan close"),
        "the refusal must point at the guarded closure command"
    );
    // The refused command must not have advanced anything.
    let read_only = RepositoryStore::open_read_only(&seed.state).unwrap();
    let plan = read_only
        .get(&PlanId::new("ep_m004_plan").unwrap())
        .unwrap();
    assert_eq!(plan.status, PlanStatus::Draft);
    assert_eq!(plan.revision, 0);
}

#[test]
fn plan_set_status_requires_an_explicit_expected_revision() {
    let seed = Seed::new();
    let missing = invoke(&[
        "plan",
        "set-status",
        "ep_m004_plan",
        "--to",
        "active",
        "--state-root",
        seed.root(),
        "--json",
    ]);
    assert_eq!(error_code(&missing), "usage");
}

#[test]
fn plan_edit_changes_structure_and_keeps_the_plan_identity() {
    let seed = Seed::new();
    json_ok(&set_status(&seed, "0", "active"));
    let mut edited = seed.exported();
    // A new objective, a second item, a dependency, and a changed criterion.
    edited["objective"] = json!("Edited objective");
    edited["items"].as_array_mut().unwrap().push(json!({
        "id": "epi_m004_two",
        "position": 1,
        "dependencies": ["epi_m004_step"],
        "status": "pending",
        "description": "Second item",
        "criteria": [],
    }));
    edited["items"][0]["dependencies"] = json!([]);
    edited["items"][0]["criteria"][0]["statement"] = json!("the edited criterion holds");
    let edited_path = seed.path("edited.json");
    fs::write(&edited_path, serde_json::to_vec(&edited).unwrap()).unwrap();

    let result = json_ok(&invoke(&[
        "plan",
        "edit",
        "ep_m004_plan",
        "--input",
        edited_path.to_str().unwrap(),
        "--expected-revision",
        "1",
        "--state-root",
        seed.root(),
        "--json",
    ]));
    assert_eq!(result["data"]["revision"], 2);
    assert_eq!(result["data"]["status"], "active");

    let read_only = RepositoryStore::open_read_only(&seed.state).unwrap();
    let stored = read_only
        .get(&PlanId::new("ep_m004_plan").unwrap())
        .unwrap();
    assert_eq!(stored.objective, "Edited objective");
    assert_eq!(stored.items.len(), 2);
    assert_eq!(stored.items[1].dependencies.len(), 1);
    assert_eq!(
        stored.items[0].criteria[0].statement,
        "the edited criterion holds"
    );
    // The plan identity and the status are untouched by a structural edit.
    assert_eq!(stored.id.as_str(), "ep_m004_plan");
    assert_eq!(stored.status, PlanStatus::Active);
}

#[test]
fn plan_edit_rejects_a_stale_revision() {
    let seed = Seed::new();
    let edited = seed.exported();
    let edited_path = seed.path("edited.json");
    fs::write(&edited_path, serde_json::to_vec(&edited).unwrap()).unwrap();
    let stale = invoke(&[
        "plan",
        "edit",
        "ep_m004_plan",
        "--input",
        edited_path.to_str().unwrap(),
        "--expected-revision",
        "7",
        "--state-root",
        seed.root(),
        "--json",
    ]);
    assert_eq!(error_code(&stale), "revision_conflict");
    let read_only = RepositoryStore::open_read_only(&seed.state).unwrap();
    assert_eq!(
        read_only
            .get(&PlanId::new("ep_m004_plan").unwrap())
            .unwrap()
            .revision,
        0,
        "the refused edit must not have advanced the revision"
    );
}

#[test]
fn plan_edit_cannot_change_the_plan_status() {
    let seed = Seed::new();
    let mut edited = seed.exported();
    edited["status"] = json!("active");
    let edited_path = seed.path("edited.json");
    fs::write(&edited_path, serde_json::to_vec(&edited).unwrap()).unwrap();
    let refused = invoke(&[
        "plan",
        "edit",
        "ep_m004_plan",
        "--input",
        edited_path.to_str().unwrap(),
        "--expected-revision",
        "0",
        "--state-root",
        seed.root(),
        "--json",
    ]);
    assert_eq!(error_code(&refused), "invalid_transition");
    let envelope: Value = serde_json::from_slice(&refused.stdout).unwrap();
    assert!(
        envelope["error"]["message"]
            .as_str()
            .unwrap()
            .contains("plan set-status"),
        "the refusal must point at the lifecycle command"
    );
}

#[test]
fn plan_edit_rejects_a_mismatched_plan_id() {
    let seed = Seed::new();
    let mut edited = seed.exported();
    edited["id"] = json!("ep_some_other_plan");
    let edited_path = seed.path("edited.json");
    fs::write(&edited_path, serde_json::to_vec(&edited).unwrap()).unwrap();
    let refused = invoke(&[
        "plan",
        "edit",
        "ep_m004_plan",
        "--input",
        edited_path.to_str().unwrap(),
        "--expected-revision",
        "0",
        "--state-root",
        seed.root(),
        "--json",
    ]);
    assert_eq!(error_code(&refused), "invalid_plan");
}

#[test]
fn plan_edit_rejects_a_closed_plan() {
    // A Closed plan is produced by the guarded finalizer, so this test drives
    // the full closure protocol: a git subject, an active plan with a
    // completed item, a trusted provider, and a passing observation.
    let temp = tempdir().unwrap();
    let git_root = temp.path().join("worktree");
    fs::create_dir(&git_root).unwrap();
    init_git(&git_root);
    let state = git_root.join(".eggplan");
    let store = RepositoryStore::open(&state).unwrap();
    let subject = store.subject_source().capture().unwrap();
    let plan = fixture_plan();
    store.create(&plan).unwrap();
    let mut current = store.get(&plan.id).unwrap();
    current.revision = 1;
    current.status = PlanStatus::Active;
    current = store.compare_and_swap(&plan.id, 0, &current).unwrap();
    for (revision, status) in [
        (2, PlanItemStatus::Actionable),
        (3, PlanItemStatus::InProgress),
        (4, PlanItemStatus::Completed),
    ] {
        let mut next = current.clone();
        next.revision = revision;
        next.items[0].status = status;
        current = store
            .compare_and_swap(&plan.id, current.revision, &next)
            .unwrap();
    }
    let observation = EvidenceObservation::finalize(EvidenceObservationInput {
        id: EvidenceObservationId::new("epe_m004_pass").unwrap(),
        provider_id: provider_id(),
        kind: EvidenceKind::Test,
        status: eggplan_core::EvidenceStatus::Passed,
        subject,
        observed_at_unix_ms: 1,
        invocation_ref: Some("test-fixture".into()),
        verification_digest: Some(verification()),
        result_metadata: BTreeMap::new(),
        artifacts: vec![],
    })
    .unwrap();
    store.append_observation(&plan.id, &observation).unwrap();
    let policy = temp.path().join("providers.json");
    fs::write(
        &policy,
        br#"{"schema_version":1,"providers":[{"provider_id":"epp_m004","class":"test","allowed_kinds":["test"]}]}"#,
    )
    .unwrap();
    json_ok(&invoke(&[
        "close",
        "ep_m004_plan",
        "--expected-revision",
        "4",
        "--provider-policy",
        policy.to_str().unwrap(),
        "--state-root",
        state.to_str().unwrap(),
        "--json",
    ]));

    // The closed plan's canonical JSON is the input for a structural edit.
    let exported = json_ok(&invoke(&[
        "plan",
        "export",
        "ep_m004_plan",
        "--state-root",
        state.to_str().unwrap(),
        "--json",
    ]));
    let edited_path = temp.path().join("edited.json");
    let mut edited = exported["data"]["plan"].clone();
    edited["objective"] = json!("rewritten after closure");
    fs::write(&edited_path, serde_json::to_vec(&edited).unwrap()).unwrap();
    let refused = invoke(&[
        "plan",
        "edit",
        "ep_m004_plan",
        "--input",
        edited_path.to_str().unwrap(),
        "--expected-revision",
        "5",
        "--state-root",
        state.to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(error_code(&refused), "closed_plan_immutable");
}

#[test]
fn plan_export_writes_canonical_json_that_plan_edit_consumes() {
    let seed = Seed::new();
    let exported_path = seed.path("exported.json");
    let output = invoke(&[
        "plan",
        "export",
        "ep_m004_plan",
        "--output",
        exported_path.to_str().unwrap(),
        "--state-root",
        seed.root(),
        "--json",
    ]);
    let envelope = json_ok(&output);
    assert_eq!(
        envelope["data"]["output_file"],
        exported_path.to_str().unwrap()
    );
    let bytes = fs::read(&exported_path).unwrap();
    // The exported bytes are the plan's canonical JSON.
    let parsed: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(parsed["id"], "ep_m004_plan");
    assert_eq!(parsed["objective"], "M004 fixture objective");
    assert_eq!(parsed["items"][0]["id"], "epi_m004_step");
    // Exporting twice produces byte-identical output.
    let second_path = seed.path("exported-again.json");
    json_ok(&invoke(&[
        "plan",
        "export",
        "ep_m004_plan",
        "--output",
        second_path.to_str().unwrap(),
        "--state-root",
        seed.root(),
        "--json",
    ]));
    assert_eq!(bytes, fs::read(&second_path).unwrap());
    // Round trip: submit the exported file unchanged.
    let result = json_ok(&invoke(&[
        "plan",
        "edit",
        "ep_m004_plan",
        "--input",
        exported_path.to_str().unwrap(),
        "--expected-revision",
        "0",
        "--state-root",
        seed.root(),
        "--json",
    ]));
    assert_eq!(result["data"]["revision"], 1);
    // And the round trip preserved the structure it was given.
    let read_only = RepositoryStore::open_read_only(&seed.state).unwrap();
    let stored = read_only
        .get(&PlanId::new("ep_m004_plan").unwrap())
        .unwrap();
    assert_eq!(stored.objective, "M004 fixture objective");
    assert_eq!(stored.items.len(), 1);
    assert_eq!(stored.items[0].criteria.len(), 1);
}

#[test]
fn plan_commands_appear_in_the_usage_text_and_every_completion_script() {
    let seed = Seed::new();
    let help = invoke(&["help", "--json"]);
    let envelope = json_ok(&help);
    let usage = envelope["data"]["usage"].as_str().unwrap();
    assert!(usage.contains("plan"), "usage must name the plan family");

    for shell in ["bash", "zsh", "fish", "powershell"] {
        let output = invoke(&["completions", shell, "--state-root", seed.root(), "--json"]);
        let script = json_ok(&output)["data"]["script"]
            .as_str()
            .unwrap()
            .to_string();
        assert!(
            script.contains("set-status"),
            "{shell} completion must offer the set-status subcommand"
        );
        assert!(
            script.contains("edit"),
            "{shell} completion must offer the edit subcommand"
        );
        assert!(
            script.contains("export"),
            "{shell} completion must offer the export subcommand"
        );
    }
}

#[test]
fn markdown_import_still_creates_a_draft_plan() {
    // D6: the import path must not be usable to smuggle in a non-Draft plan,
    // now that a status command exists.
    let temp = tempdir().unwrap();
    let state = temp.path().join("state");
    json_ok(&invoke(&[
        "--state-root",
        state.to_str().unwrap(),
        "init",
        "--json",
    ]));
    let markdown = temp.path().join("intent.md");
    fs::write(
        &markdown,
        b"# eggplan\n\n## objective\n\nShip the thing\n\n## items\n\n- do the work\n",
    )
    .unwrap();
    let imported = json_ok(&invoke(&[
        "markdown",
        "import",
        markdown.to_str().unwrap(),
        "--state-root",
        state.to_str().unwrap(),
        "--json",
    ]));
    assert_eq!(imported["data"]["status"], "draft");
    assert_eq!(imported["data"]["revision"], 0);
    let plan_id = imported["data"]["plan_id"].as_str().unwrap();
    // The imported plan cannot be moved out of Draft by the import itself.
    let read_only = RepositoryStore::open_read_only(&state).unwrap();
    let stored = read_only
        .get(&PlanId::new(plan_id.to_string()).unwrap())
        .unwrap();
    assert_eq!(stored.status, PlanStatus::Draft);
}
