use eggplan_core::{
    AcceptanceCriterion, EvidenceCardinality, EvidenceKind, EvidenceObservation,
    EvidenceObservationId, EvidenceObservationInput, EvidenceProviderId, EvidenceRequirement,
    EvidenceStatus, Plan, PlanId, PlanItem, PlanItemId, PlanItemStatus, PlanStatus, SubjectPolicy,
    VerificationDigest,
};
use eggplan_repo::{PlanStore, RepositoryStore};
use git2::{Repository, Signature};
use serde_json::Value;
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

fn draft_plan() -> Plan {
    Plan::new(
        PlanId::new("ep_cli_plan").unwrap(),
        "CLI integration fixture",
        vec![PlanItem {
            id: PlanItemId::new("epi_cli_step").unwrap(),
            position: 0,
            parent: None,
            dependencies: vec![],
            status: PlanItemStatus::Pending,
            description: "CLI step".into(),
            criteria: vec![],
            blocker: None,
            next_action: None,
        }],
    )
    .unwrap()
}

#[test]
fn native_cli_smoke_reads_mutates_and_derives_registry() {
    let temp = tempdir().unwrap();
    let state = temp.path().join("state root");
    let initialized = invoke(&["--state-root", state.to_str().unwrap(), "init", "--json"]);
    let initialized = json_ok(&initialized);
    assert_eq!(initialized["command"], "init");
    let initialized_again = invoke(&["init", "--state-root", state.to_str().unwrap(), "--json"]);
    assert_eq!(
        json_ok(&initialized_again)["data"]["repository_id"],
        initialized["data"]["repository_id"]
    );

    let input = temp.path().join("plan.json");
    fs::write(&input, serde_json::to_vec(&draft_plan()).unwrap()).unwrap();
    let created = invoke(&[
        "new",
        "--input",
        input.to_str().unwrap(),
        "--state-root",
        state.to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(json_ok(&created)["data"]["revision"], 0);
    let unavailable_check = invoke(&["check", "--state-root", state.to_str().unwrap(), "--json"]);
    assert_eq!(
        json_ok(&unavailable_check)["data"]["plans"][0]["state"],
        "unavailable"
    );

    for args in [
        vec!["show", "ep_cli_plan"],
        vec!["status", "ep_cli_plan"],
        vec!["ready", "ep_cli_plan"],
        vec!["graph", "ep_cli_plan"],
        vec!["check"],
        vec!["registry", "render"],
        vec!["evidence", "list", "ep_cli_plan"],
        vec!["evidence", "supersessions", "ep_cli_plan"],
    ] {
        let mut command = vec!["--state-root", state.to_str().unwrap()];
        command.extend(args);
        command.push("--json");
        assert_eq!(json_ok(&invoke(&command))["ok"], true);
    }

    let activated = invoke(&[
        "activate",
        "ep_cli_plan",
        "--expected-revision",
        "0",
        "--state-root",
        state.to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(json_ok(&activated)["data"]["revision"], 1);
    let updated = invoke(&[
        "item",
        "update",
        "ep_cli_plan",
        "epi_cli_step",
        "--expected-revision",
        "1",
        "--status",
        "blocked",
        "--blocker",
        "waiting for input",
        "--state-root",
        state.to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(json_ok(&updated)["data"]["revision"], 2);

    let stale = invoke(&[
        "activate",
        "ep_cli_plan",
        "--expected-revision",
        "1",
        "--state-root",
        state.to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(stale.status.code(), Some(2));
    let stale_envelope: Value = serde_json::from_slice(&stale.stdout).unwrap();
    assert_eq!(stale_envelope["error"]["code"], "revision_conflict");
    let illegal = invoke(&[
        "item",
        "update",
        "ep_cli_plan",
        "epi_cli_step",
        "--expected-revision",
        "2",
        "--status",
        "completed",
        "--blocker",
        "",
        "--state-root",
        state.to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(illegal.status.code(), Some(2));
    assert_eq!(
        serde_json::from_slice::<Value>(&illegal.stdout).unwrap()["error"]["code"],
        "invalid_transition"
    );

    let repo = RepositoryStore::open_read_only(&state).unwrap();
    assert_eq!(
        repo.get(&PlanId::new("ep_cli_plan").unwrap())
            .unwrap()
            .revision,
        2
    );
    assert!(!temp.path().join("plans").join("registry.md").exists());
}

#[test]
fn help_snapshot_is_stable_and_human_output_needs_no_ansi() {
    let output = invoke(&["--help"]);
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        include_str!("fixtures/help.txt")
    );
    let temp = tempdir().unwrap();
    let human = invoke(&[
        "init",
        "--state-root",
        temp.path().join("state").to_str().unwrap(),
    ]);
    assert!(human.status.success());
    assert!(!String::from_utf8_lossy(&human.stdout).contains('\u{1b}'));
}

#[test]
fn strict_provider_policy_fails_closed_and_assessment_needs_explicit_file() {
    let temp = tempdir().unwrap();
    let state = temp.path().join(".eggplan");
    let input = temp.path().join("plan.json");
    fs::write(&input, serde_json::to_vec(&draft_plan()).unwrap()).unwrap();
    assert!(
        invoke(&["init", "--state-root", state.to_str().unwrap()])
            .status
            .success()
    );
    assert!(
        invoke(&[
            "new",
            "--input",
            input.to_str().unwrap(),
            "--state-root",
            state.to_str().unwrap()
        ])
        .status
        .success()
    );

    let missing = invoke(&[
        "assess",
        "ep_cli_plan",
        "--state-root",
        state.to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(missing.status.code(), Some(2));
    assert_eq!(
        serde_json::from_slice::<Value>(&missing.stdout).unwrap()["error"]["code"],
        "usage"
    );

    let bad_policy = temp.path().join("bad-policy.json");
    fs::write(
        &bad_policy,
        br#"{"schema_version":1,"providers":[],"trusted":true}"#,
    )
    .unwrap();
    let bad = invoke(&[
        "assess",
        "ep_cli_plan",
        "--provider-policy",
        bad_policy.to_str().unwrap(),
        "--state-root",
        state.to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(bad.status.code(), Some(2));
    assert_eq!(
        serde_json::from_slice::<Value>(&bad.stdout).unwrap()["error"]["code"],
        "invalid_policy"
    );

    let duplicate_policy = temp.path().join("duplicate-policy.json");
    fs::write(&duplicate_policy, br#"{"schema_version":1,"providers":[{"provider_id":"epp_same","class":"test","allowed_kinds":["test"]},{"provider_id":"epp_same","class":"test","allowed_kinds":["test"]}]}"#).unwrap();
    let duplicate = invoke(&[
        "assess",
        "ep_cli_plan",
        "--provider-policy",
        duplicate_policy.to_str().unwrap(),
        "--state-root",
        state.to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(
        serde_json::from_slice::<Value>(&duplicate.stdout).unwrap()["error"]["code"],
        "duplicate_provider"
    );

    let unknown_option = invoke(&["init", "--not-an-option", "x", "--json"]);
    assert_eq!(unknown_option.status.code(), Some(2));

    let plan_path = state.join("plans/ep_cli_plan/plan.json");
    fs::write(&plan_path, b"corrupt stored envelope").unwrap();
    let corrupt = invoke(&["check", "--state-root", state.to_str().unwrap(), "--json"]);
    assert_eq!(corrupt.status.code(), Some(2));
    assert_eq!(
        serde_json::from_slice::<Value>(&corrupt.stdout).unwrap()["error"]["code"],
        "corrupt_state"
    );
}

#[test]
fn check_reports_recovery_required_without_touching_pending_state() {
    let temp = tempdir().unwrap();
    let state = temp.path().join(".eggplan");
    let input = temp.path().join("plan.json");
    fs::write(&input, serde_json::to_vec(&draft_plan()).unwrap()).unwrap();
    assert!(
        invoke(&["init", "--state-root", state.to_str().unwrap()])
            .status
            .success()
    );
    assert!(
        invoke(&[
            "new",
            "--input",
            input.to_str().unwrap(),
            "--state-root",
            state.to_str().unwrap()
        ])
        .status
        .success()
    );
    let pending = state.join("plans/ep_cli_plan/closure.pending.json");
    fs::write(&pending, b"pending transaction bytes").unwrap();
    let checked = invoke(&["check", "--state-root", state.to_str().unwrap(), "--json"]);
    assert_eq!(checked.status.code(), Some(2));
    assert_eq!(
        serde_json::from_slice::<Value>(&checked.stdout).unwrap()["error"]["code"],
        "recovery_required"
    );
    assert_eq!(fs::read(&pending).unwrap(), b"pending transaction bytes");
}

#[test]
fn close_uses_explicit_policy_and_guarded_repository_protocol() {
    let temp = tempdir().unwrap();
    let git_root = temp.path().join("worktree");
    fs::create_dir(&git_root).unwrap();
    init_git(&git_root);
    let state = git_root.join(".eggplan");
    let store = RepositoryStore::open(&state).unwrap();
    let subject = store.subject_source().capture().unwrap();
    let verification = VerificationDigest::new(format!("sha256:{}", "a".repeat(64))).unwrap();
    let provider_id = EvidenceProviderId::new("epp_fixture").unwrap();
    let mut plan = draft_plan();
    plan.items[0].criteria = vec![AcceptanceCriterion {
        id: eggplan_core::CriterionId::new("epc_cli_pass").unwrap(),
        statement: "host test passed".into(),
        human_judgment_allowed: false,
        requirements: vec![EvidenceRequirement {
            description: "fixture test".into(),
            kind: EvidenceKind::Test,
            provider: Some(provider_id.clone()),
            subject_policy: SubjectPolicy::Exact,
            cardinality: EvidenceCardinality::Any,
            min_count: 1,
            allow_human_judgment: false,
            expected_verification_digest: Some(verification.clone()),
        }],
    }];
    plan.validate().unwrap();
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
        id: EvidenceObservationId::new("epe_cli_pass").unwrap(),
        provider_id: provider_id.clone(),
        kind: EvidenceKind::Test,
        status: EvidenceStatus::Passed,
        subject,
        observed_at_unix_ms: 1,
        invocation_ref: Some("test-fixture".into()),
        verification_digest: Some(verification),
        result_metadata: BTreeMap::new(),
        artifacts: vec![],
    })
    .unwrap();
    let policy = temp.path().join("providers.json");
    fs::write(&policy, br#"{"schema_version":1,"providers":[{"provider_id":"epp_fixture","class":"test","allowed_kinds":["test"]}]}"#).unwrap();
    let incomplete_check = invoke(&[
        "check",
        "ep_cli_plan",
        "--provider-policy",
        policy.to_str().unwrap(),
        "--state-root",
        state.to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(
        json_ok(&incomplete_check)["data"]["plans"][0]["state"],
        "incomplete"
    );
    store.append_observation(&plan.id, &observation).unwrap();
    let complete_check = invoke(&[
        "check",
        "ep_cli_plan",
        "--provider-policy",
        policy.to_str().unwrap(),
        "--state-root",
        state.to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(
        json_ok(&complete_check)["data"]["plans"][0]["state"],
        "complete"
    );
    let no_policy = invoke(&[
        "close",
        "ep_cli_plan",
        "--expected-revision",
        "4",
        "--state-root",
        state.to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(
        serde_json::from_slice::<Value>(&no_policy.stdout).unwrap()["error"]["code"],
        "usage"
    );

    let closed = invoke(&[
        "close",
        "ep_cli_plan",
        "--expected-revision",
        "4",
        "--provider-policy",
        policy.to_str().unwrap(),
        "--state-root",
        state.to_str().unwrap(),
        "--json",
    ]);
    let envelope = json_ok(&closed);
    assert_eq!(envelope["command"], "close");
    assert_eq!(envelope["data"]["closure"]["final_plan_revision"], 5);
    let read_only = RepositoryStore::open_read_only(&state).unwrap();
    assert_eq!(read_only.get(&plan.id).unwrap().status, PlanStatus::Closed);
    assert!(read_only.closure_record(&plan.id).unwrap().is_some());
    commit_file(&git_root, "later.txt", b"new revision");
    let stale_check = invoke(&[
        "check",
        "ep_cli_plan",
        "--provider-policy",
        policy.to_str().unwrap(),
        "--state-root",
        state.to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(json_ok(&stale_check)["data"]["plans"][0]["state"], "stale");
    let shown = invoke(&[
        "closure",
        "show",
        "ep_cli_plan",
        "--state-root",
        state.to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(json_ok(&shown)["data"]["plan_id"], "ep_cli_plan");
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

fn commit_file(path: &Path, relative: &str, contents: &[u8]) {
    let target = path.join(relative);
    fs::write(&target, contents).unwrap();
    let repository = Repository::open(path).unwrap();
    let mut index = repository.index().unwrap();
    index.add_path(Path::new(relative)).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repository.find_tree(tree_id).unwrap();
    let parent = repository.head().unwrap().peel_to_commit().unwrap();
    let signature = Signature::now("Eggplan Test", "test@example.invalid").unwrap();
    repository
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            "advance subject",
            &tree,
            &[&parent],
        )
        .unwrap();
}
