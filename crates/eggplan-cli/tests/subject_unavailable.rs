//! C002: an unavailable current Git subject is reported with its specific
//! cause, never as a silent success and never as a drift diagnosis.
//!
//! The state root used here is deliberately not inside a Git worktree, so the
//! subject capture fails with a deterministic `NotGit` error.

use eggplan_core::{Plan, PlanId, PlanItem, PlanItemId, PlanItemStatus};
use serde_json::Value;
use std::{fs, path::Path, process::Command};
use tempfile::{TempDir, tempdir};

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
    assert_eq!(value["ok"], true);
    value
}

fn json_err(output: &std::process::Output) -> Value {
    assert!(!output.status.success(), "expected failure");
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["ok"], false);
    value
}

fn draft_plan() -> Plan {
    Plan::new(
        PlanId::new("ep_subject_plan").unwrap(),
        "subject availability fixture",
        vec![PlanItem {
            id: PlanItemId::new("epi_subject_step").unwrap(),
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

/// A state root that is not inside a Git worktree, holding one Plan.
fn seeded_state() -> (TempDir, std::path::PathBuf) {
    let temp = tempdir().unwrap();
    let state = temp.path().join("state root");
    assert!(
        invoke(&["init", "--state-root", state.to_str().unwrap(), "--json"])
            .status
            .success()
    );
    let input = temp.path().join("plan.json");
    fs::write(&input, serde_json::to_vec(&draft_plan()).unwrap()).unwrap();
    assert!(
        invoke(&[
            "new",
            "--input",
            input.to_str().unwrap(),
            "--state-root",
            state.to_str().unwrap(),
            "--json",
        ])
        .status
        .success()
    );
    (temp, state)
}

fn policy_file(dir: &Path) -> std::path::PathBuf {
    let path = dir.join("providers.json");
    fs::write(
        &path,
        br#"{"schema_version":1,"providers":[{"provider_id":"epp_fixture","class":"test","allowed_kinds":["test"]}]}"#,
    )
    .unwrap();
    path
}

#[test]
fn show_reports_an_unavailable_subject_with_the_specific_cause() {
    let (temp, state) = seeded_state();

    // The read path must not silently present a subjectless result as a
    // complete one: it reports the capture failure with the existing
    // `subject_unavailable` diagnostic code and the underlying message.
    let shown = invoke(&[
        "--state-root",
        state.to_str().unwrap(),
        "show",
        "ep_subject_plan",
        "--json",
    ]);
    let envelope = json_err(&shown);
    assert_eq!(envelope["error"]["code"], "subject_unavailable");
    let message = envelope["error"]["message"].as_str().unwrap();
    assert!(
        message.contains("not inside a Git worktree"),
        "the specific capture cause must reach the operator: {message}"
    );

    // The human output carries the same code and the same cause.
    let human = invoke(&[
        "--state-root",
        state.to_str().unwrap(),
        "show",
        "ep_subject_plan",
    ]);
    assert_eq!(human.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&human.stderr);
    assert!(
        stderr.contains("subject_unavailable"),
        "the human path must name the diagnostic code: {stderr}"
    );
    assert!(
        stderr.contains("not inside a Git worktree"),
        "the human path must name the capture cause: {stderr}"
    );

    drop(temp);
}

#[test]
fn the_closure_path_reports_subject_unavailable_unchanged() {
    let (temp, state) = seeded_state();
    let policy = policy_file(temp.path());

    // The guarded closure path (here: the assessment it performs) already
    // reported an unavailable subject with `subject_unavailable` before the
    // read path was corrected. That behaviour must not change.
    let assessed = invoke(&[
        "assess",
        "ep_subject_plan",
        "--provider-policy",
        policy.to_str().unwrap(),
        "--state-root",
        state.to_str().unwrap(),
        "--json",
    ]);
    let envelope = json_err(&assessed);
    assert_eq!(envelope["error"]["code"], "subject_unavailable");
    assert!(
        envelope["error"]["message"]
            .as_str()
            .unwrap()
            .contains("not inside a Git worktree"),
        "the closure path keeps reporting the underlying cause: {}",
        envelope["error"]["message"]
    );
}

#[test]
fn the_current_subject_unavailable_warning_carries_the_cause_and_keeps_its_token() {
    let (temp, state) = seeded_state();

    // The snapshot's soft-unavailability path stays soft: the read succeeds and
    // warns. The warning token is unchanged and the cause is appended to the
    // warning text only — never to the reason-code list.
    let checked = json_ok(&invoke(&[
        "check",
        "--state-root",
        state.to_str().unwrap(),
        "--json",
    ]));
    let plan = &checked["data"]["plans"][0];
    assert_eq!(plan["state"], "unavailable");
    let codes: Vec<&str> = plan["reason_codes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect();
    assert_eq!(
        codes,
        vec!["current_subject_unavailable"],
        "the reason-code list must stay exactly the token"
    );
    let warnings: Vec<&str> = checked["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect();
    assert_eq!(warnings.len(), 1, "the warning count must not change");
    let warning = warnings[0];
    assert!(
        warning.starts_with("current_subject_unavailable"),
        "the warning token must stay `current_subject_unavailable`: {warning}"
    );
    assert!(
        warning.contains("not inside a Git worktree"),
        "the warning must carry the specific capture cause: {warning}"
    );

    drop(temp);
}
