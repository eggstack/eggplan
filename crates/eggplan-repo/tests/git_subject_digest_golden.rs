//! Frozen Git subject dirty-digest matrix (CodeGG integration C001 §5, §11).
//!
//! These digests were captured from the pre-C001 `dirty_manifest`
//! implementation at revision
//! `52a4be76b631822799e7f1e76fe5b439c94fd058` and must never change. They
//! freeze the algorithm's bytes — row ordering, status-bit encoding,
//! index-entry treatment, and the regular-file / symlink / deleted /
//! submodule row encodings — so the C001 capture refactor and the public
//! fingerprint contract cannot silently redefine Eggplan subject identity.
//!
//! Only the clean/dirty state and the dirty digest are frozen. The HEAD OID is
//! asserted separately where it is deterministic; the submodule fixture is
//! assembled by the `git` CLI, whose commit timestamps are not controlled
//! here, so no revision value is recorded for it.
//!
//! `symlink_typechange` requires Unix symlink creation, so it is asserted on
//! Linux/macOS and skipped (never silently dropped from the fixture) on
//! Windows.
//!
//! The ignored `record_*` test prints a regenerated matrix. It exists only so
//! an approved future algorithm change can restate the freeze deliberately;
//! it never rewrites the fixture itself.

use eggplan_core::SubjectState;
use eggplan_repo::GitSubjectSource;
use git2::{Repository, Signature, Time};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::{TempDir, tempdir};

const FIXTURE: &str = include_str!("fixtures/git-subject-digests-v1.json");
const FIXTURE_SHA256: &str = include_str!("fixtures/git-subject-digests-v1.sha256");

const NESTED_DIRTY_SUBMODULE: &str = "nested_dirty_submodule";

/// (case name, portable). Every case is frozen in the fixture; `portable` marks
/// the ones this platform can reproduce.
const CASES: [(&str, bool); 8] = [
    ("clean", true),
    ("unstaged_tracked_edit", true),
    ("staged_tracked_edit", true),
    ("staged_and_unstaged_same_path", true),
    ("untracked_file", true),
    ("deleted_tracked_file", true),
    ("symlink_typechange", cfg!(unix)),
    ("staged_rename", true),
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GoldenCase {
    state: SubjectState,
    dirty_digest: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GoldenMatrix {
    algorithm: String,
    frozen_from_revision: String,
    cases: BTreeMap<String, GoldenCase>,
}

fn fixed_signature(role: &str) -> Signature<'_> {
    Signature::new(
        role,
        "golden@eggplan.example.invalid",
        &Time::new(1_700_000_000, 0),
    )
    .unwrap()
}

fn write_commit(repo: &Repository, message: &str) {
    let tree_id = repo.index().unwrap().write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let author = fixed_signature("Eggplan Golden");
    repo.commit(Some("HEAD"), &author, &author, message, &tree, &[])
        .unwrap();
}

/// Deterministic base worktree: fixed file contents and fixed commit
/// identities, so every derived status bit, index blob id, and content byte is
/// reproducible across runs and platforms.
fn base_repo(root: &Path) -> Repository {
    let repo = Repository::init(root).unwrap();
    fs::write(root.join("tracked.txt"), b"base\n").unwrap();
    fs::write(root.join("other.txt"), b"other\n").unwrap();
    fs::write(root.join("rename-me.txt"), b"rename\n").unwrap();
    let mut index = repo.index().unwrap();
    for name in ["tracked.txt", "other.txt", "rename-me.txt"] {
        index.add_path(Path::new(name)).unwrap();
    }
    index.write().unwrap();
    write_commit(&repo, "base\n");
    repo
}

fn apply(case: &str, root: &Path, repo: &Repository) {
    match case {
        "clean" => {}
        "unstaged_tracked_edit" => fs::write(root.join("tracked.txt"), b"modified\n").unwrap(),
        "staged_tracked_edit" => {
            fs::write(root.join("tracked.txt"), b"staged\n").unwrap();
            repo.index()
                .unwrap()
                .add_path(Path::new("tracked.txt"))
                .unwrap();
            repo.index().unwrap().write().unwrap();
        }
        "staged_and_unstaged_same_path" => {
            fs::write(root.join("tracked.txt"), b"staged\n").unwrap();
            repo.index()
                .unwrap()
                .add_path(Path::new("tracked.txt"))
                .unwrap();
            repo.index().unwrap().write().unwrap();
            fs::write(root.join("tracked.txt"), b"staged then edited\n").unwrap();
        }
        "untracked_file" => fs::write(root.join("untracked.txt"), b"untracked\n").unwrap(),
        "deleted_tracked_file" => fs::remove_file(root.join("other.txt")).unwrap(),
        "symlink_typechange" => apply_symlink_typechange(root),
        "staged_rename" => {
            fs::rename(root.join("rename-me.txt"), root.join("renamed.txt")).unwrap();
            let mut index = repo.index().unwrap();
            index.add_path(Path::new("renamed.txt")).unwrap();
            index.remove_path(Path::new("rename-me.txt")).unwrap();
            index.write().unwrap();
        }
        other => panic!("unknown golden case {other}"),
    }
}

/// Unix-only: Windows CI runners cannot create symlinks without elevated
/// privileges, so the case is asserted on Linux/macOS and kept in the frozen
/// fixture everywhere.
#[cfg(unix)]
fn apply_symlink_typechange(root: &Path) {
    use std::os::unix::fs::symlink;
    fs::remove_file(root.join("other.txt")).unwrap();
    symlink("tracked.txt", root.join("other.txt")).unwrap();
}

#[cfg(not(unix))]
fn apply_symlink_typechange(_root: &Path) {
    unreachable!("symlink_typechange is never applied on a non-Unix platform")
}

/// Parent worktree with a committed submodule whose own tracked file is
/// dirty. The parent row carries the gitlink index id and the nested manifest.
fn submodule_repo() -> (TempDir, TempDir, Repository) {
    let parent = tempdir().unwrap();
    let subdir = tempdir().unwrap();
    let subrepo = base_repo(subdir.path());
    let parent_repo = base_repo(parent.path());
    drop(parent_repo);
    let git = |args: &[&str]| {
        let status = Command::new("git")
            .arg("-c")
            .arg("protocol.file.allow=always")
            .arg("-c")
            .arg("core.hooksPath=/dev/null")
            .arg("-C")
            .arg(parent.path())
            .args(args)
            .status()
            .unwrap();
        assert!(status.success(), "git {args:?} failed");
    };
    git(&[
        "submodule",
        "add",
        subrepo.workdir().unwrap().to_str().unwrap(),
        "module",
    ]);
    git(&[
        "-c",
        "user.name=Eggplan Golden",
        "-c",
        "user.email=golden@eggplan.example.invalid",
        "commit",
        "--no-gpg-sign",
        "-m",
        "add submodule",
    ]);
    fs::write(
        parent.path().join("module/tracked.txt"),
        b"changed inside module\n",
    )
    .unwrap();
    (parent, subdir, subrepo)
}

fn capture(case: &str) -> GoldenCase {
    let subject = if case == NESTED_DIRTY_SUBMODULE {
        let (parent, _subdir, _subrepo) = submodule_repo();
        GitSubjectSource::new(parent.path(), "epr_golden")
            .capture()
            .unwrap()
    } else {
        let dir = tempdir().unwrap();
        let repo = base_repo(dir.path());
        apply(case, dir.path(), &repo);
        GitSubjectSource::new(dir.path(), "epr_golden")
            .capture()
            .unwrap()
    };
    GoldenCase {
        state: subject.state,
        dirty_digest: subject.dirty_digest,
    }
}

fn portable_cases() -> impl Iterator<Item = &'static str> {
    CASES
        .iter()
        .filter(|(_, portable)| *portable)
        .map(|(name, _)| *name)
}

fn matrix() -> BTreeMap<String, GoldenCase> {
    portable_cases()
        .map(|case| (case.to_string(), capture(case)))
        .chain(std::iter::once((
            NESTED_DIRTY_SUBMODULE.to_string(),
            capture(NESTED_DIRTY_SUBMODULE),
        )))
        .collect()
}

fn frozen() -> GoldenMatrix {
    serde_json::from_str(FIXTURE.trim()).unwrap()
}

#[test]
fn golden_dirty_manifest_digests_are_frozen() {
    assert_eq!(
        format!("sha256:{:x}", Sha256::digest(FIXTURE.trim().as_bytes())),
        FIXTURE_SHA256.trim(),
        "fixture bytes changed; the digest freeze must be restated deliberately"
    );
    let frozen = frozen();
    let observed = matrix();
    let mut expected_names: Vec<&str> = CASES
        .iter()
        .map(|(name, _)| *name)
        .chain(std::iter::once(NESTED_DIRTY_SUBMODULE))
        .collect();
    expected_names.sort_unstable();
    assert_eq!(frozen.cases.len(), expected_names.len());
    let skipped: Vec<&str> = CASES
        .iter()
        .filter(|(_, portable)| !*portable)
        .map(|(name, _)| *name)
        .collect();
    assert_eq!(
        frozen.cases.keys().map(String::as_str).collect::<Vec<_>>(),
        expected_names
    );
    for name in skipped {
        assert!(
            frozen.cases.contains_key(name),
            "platform-skipped case {name} must stay frozen in the fixture"
        );
    }
    for (name, expected) in &observed {
        assert_eq!(
            frozen.cases.get(name),
            Some(expected),
            "golden digest drift for {name}"
        );
    }
}

#[test]
fn golden_cases_are_state_consistent() {
    let frozen = frozen();
    let clean = frozen.cases.get("clean").unwrap();
    assert_eq!(clean.state, SubjectState::Clean);
    assert_eq!(clean.dirty_digest, None);
    for (case, entry) in &frozen.cases {
        assert_eq!(
            entry.state == SubjectState::Dirty,
            entry.dirty_digest.is_some(),
            "case {case} is not self-consistent"
        );
        if let Some(digest) = entry.dirty_digest.as_deref() {
            assert!(
                digest.starts_with("sha256:") && digest.len() == 71,
                "case {case} has a malformed frozen digest"
            );
        }
    }
}

#[test]
#[ignore = "prints a regenerated golden matrix; never rewrites the fixture"]
fn record_golden_dirty_manifest_digests() {
    let regenerated = GoldenMatrix {
        algorithm: frozen().algorithm,
        frozen_from_revision: frozen().frozen_from_revision,
        cases: matrix(),
    };
    println!("{}\n", serde_json::to_string_pretty(&regenerated).unwrap());
}
