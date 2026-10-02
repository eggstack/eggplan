//! Public Git subject fingerprint contract tests (CodeGG integration C001 §4,
//! §6, §7, §11).
//!
//! The fingerprint must be exactly the revision, clean/dirty state, and dirty
//! digest `GitSubjectSource::capture` produces for the same root, options, and
//! exclusion; must fail closed on every bound/exclusion/safety failure; and
//! must expose no repository identity and no part of the dirty manifest.

use eggplan_core::{
    EvidenceKind, EvidenceObservation, EvidenceObservationId, EvidenceObservationInput,
    EvidenceProviderId, EvidenceStatus, Plan, PlanId, PlanItem, PlanItemId, PlanItemStatus,
    SubjectState, VerificationDigest,
};
use eggplan_repo::{
    GitSubjectError, GitSubjectFingerprintV1, GitSubjectOptions, GitSubjectSource, PlanStore,
    RepositoryStore, capture_git_subject_fingerprint,
};
use git2::{Repository, Signature, Time};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::{TempDir, tempdir};

fn default_options() -> GitSubjectOptions {
    GitSubjectOptions::default()
}

fn fingerprint(
    root: &Path,
    options: GitSubjectOptions,
    excluded: Option<&Path>,
) -> Result<GitSubjectFingerprintV1, GitSubjectError> {
    capture_git_subject_fingerprint(root, options, excluded)
}

fn fixed_signature() -> Signature<'static> {
    Signature::new(
        "Eggplan Fingerprint Test",
        "eggplan@example.invalid",
        &Time::new(1_700_000_000, 0),
    )
    .unwrap()
}

fn commit_worktree(repo: &Repository, message: &str) {
    let mut index = repo.index().unwrap();
    index
        .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
        .unwrap();
    index.update_all(["*"].iter(), None).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let author = fixed_signature();
    repo.commit(Some("HEAD"), &author, &author, message, &tree, &[])
        .unwrap();
}

fn base_repo(root: &Path) -> Repository {
    let repo = Repository::init(root).unwrap();
    fs::write(root.join("tracked.txt"), b"base\n").unwrap();
    fs::write(root.join("other.txt"), b"other\n").unwrap();
    commit_worktree(&repo, "base\n");
    repo
}

/// Assert the §4 single-implementation-source invariant for one worktree
/// state: fingerprint and subject agree on revision, state, and dirty digest.
fn assert_fingerprint_matches_subject(root: &Path, excluded: Option<&Path>, dirty: bool) {
    let fingerprint = fingerprint(root, default_options(), excluded).unwrap();
    let mut source = GitSubjectSource::new(root, "epr_fingerprint");
    if let Some(excluded) = excluded {
        source = source.excluding_path(excluded);
    }
    let subject = source.capture().unwrap();
    assert_eq!(
        fingerprint.schema_version,
        GitSubjectFingerprintV1::SCHEMA_VERSION
    );
    assert_eq!(fingerprint.revision, subject.revision);
    assert_eq!(fingerprint.state, subject.state);
    assert_eq!(fingerprint.dirty_digest, subject.dirty_digest);
    assert_eq!(fingerprint.state == SubjectState::Dirty, dirty);
    assert_eq!(fingerprint.dirty_digest.is_some(), dirty);
}

#[test]
fn fingerprint_matches_subject_across_representative_dirty_states() {
    let dir = tempdir().unwrap();
    let repo = base_repo(dir.path());
    let root = dir.path();

    assert_fingerprint_matches_subject(root, None, false);

    fs::write(root.join("tracked.txt"), b"modified\n").unwrap();
    assert_fingerprint_matches_subject(root, None, true);
    let unstaged = fingerprint(root, default_options(), None).unwrap();

    repo.index()
        .unwrap()
        .add_path(Path::new("tracked.txt"))
        .unwrap();
    repo.index().unwrap().write().unwrap();
    assert_fingerprint_matches_subject(root, None, true);
    let staged = fingerprint(root, default_options(), None).unwrap();
    assert_ne!(staged.dirty_digest, unstaged.dirty_digest);

    fs::write(root.join("tracked.txt"), b"staged then edited\n").unwrap();
    assert_fingerprint_matches_subject(root, None, true);
    let both = fingerprint(root, default_options(), None).unwrap();
    assert_ne!(both.dirty_digest, staged.dirty_digest);

    fs::write(root.join("untracked.txt"), b"untracked\n").unwrap();
    assert_fingerprint_matches_subject(root, None, true);
    let untracked = fingerprint(root, default_options(), None).unwrap();
    assert_ne!(untracked.dirty_digest, both.dirty_digest);

    fs::remove_file(root.join("other.txt")).unwrap();
    assert_fingerprint_matches_subject(root, None, true);
    let deleted = fingerprint(root, default_options(), None).unwrap();
    assert_ne!(deleted.dirty_digest, untracked.dirty_digest);

    // libgit2 does not enable rename/copy detection for this subject
    // algorithm, so a staged rename is reported as staged-add plus removal.
    fs::rename(root.join("tracked.txt"), root.join("renamed.txt")).unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("renamed.txt")).unwrap();
    index.remove_path(Path::new("tracked.txt")).unwrap();
    index.write().unwrap();
    assert_fingerprint_matches_subject(root, None, true);
    let renamed = fingerprint(root, default_options(), None).unwrap();
    assert_ne!(renamed.dirty_digest, deleted.dirty_digest);
    assert_eq!(renamed.revision, both.revision);
}

#[cfg(unix)]
#[test]
fn fingerprint_matches_subject_for_symlinked_path() {
    use std::os::unix::fs::symlink;
    let dir = tempdir().unwrap();
    let _repo = base_repo(dir.path());
    fs::remove_file(dir.path().join("other.txt")).unwrap();
    symlink("tracked.txt", dir.path().join("other.txt")).unwrap();
    assert_fingerprint_matches_subject(dir.path(), None, true);

    let dir = tempdir().unwrap();
    let _repo = base_repo(dir.path());
    symlink("../outside", dir.path().join("escape.txt")).unwrap();
    assert_fingerprint_matches_subject(dir.path(), None, true);
}

fn submodule_parent() -> (TempDir, TempDir) {
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
        "user.name=Eggplan Test",
        "-c",
        "user.email=eggplan@example.invalid",
        "commit",
        "--no-gpg-sign",
        "-m",
        "add submodule",
    ]);
    (parent, subdir)
}

#[test]
fn fingerprint_matches_subject_for_dirty_nested_submodule() {
    let (parent, _subdir) = submodule_parent();
    assert_fingerprint_matches_subject(parent.path(), None, false);
    fs::write(
        parent.path().join("module/tracked.txt"),
        b"changed inside module\n",
    )
    .unwrap();
    assert_fingerprint_matches_subject(parent.path(), None, true);
    let first = fingerprint(parent.path(), default_options(), None).unwrap();
    fs::write(
        parent.path().join("module/tracked.txt"),
        b"changed differently\n",
    )
    .unwrap();
    assert_fingerprint_matches_subject(parent.path(), None, true);
    let second = fingerprint(parent.path(), default_options(), None).unwrap();
    assert_eq!(first.revision, second.revision);
    assert_ne!(first.dirty_digest, second.dirty_digest);
}

#[test]
fn fingerprint_excludes_the_single_administrative_root() {
    let dir = tempdir().unwrap();
    let _repo = base_repo(dir.path());
    let state = dir.path().join(".eggplan");
    fs::create_dir_all(state.join("plans/ep_fingerprint")).unwrap();
    fs::write(state.join("plans/ep_fingerprint/plan.json"), b"{}\n").unwrap();
    // The administrative root is untracked; without the exclusion it perturbs
    // the subject, with the exclusion it contributes nothing.
    assert_fingerprint_matches_subject(dir.path(), Some(&state), false);
    let excluded = fingerprint(dir.path(), default_options(), Some(&state)).unwrap();
    assert_eq!(excluded.dirty_digest, None);
    let included = fingerprint(dir.path(), default_options(), None).unwrap();
    assert!(included.dirty_digest.is_some());

    fs::write(
        state.join("plans/ep_fingerprint/plan.json"),
        b"{\"changed\":true}\n",
    )
    .unwrap();
    fs::write(dir.path().join("tracked.txt"), b"modified\n").unwrap();
    assert_fingerprint_matches_subject(dir.path(), Some(&state), true);
    let with_source = fingerprint(dir.path(), default_options(), Some(&state)).unwrap();
    fs::write(dir.path().join("tracked.txt"), b"modified again\n").unwrap();
    let changed_source = fingerprint(dir.path(), default_options(), Some(&state)).unwrap();
    assert_ne!(with_source.dirty_digest, changed_source.dirty_digest);

    // A sibling that merely shares the excluded name as a string prefix stays
    // in scope.
    fs::create_dir_all(dir.path().join(".eggplan-admin")).unwrap();
    fs::write(dir.path().join(".eggplan-admin/plan.json"), b"{}\n").unwrap();
    let sibling = fingerprint(dir.path(), default_options(), Some(&state)).unwrap();
    assert_ne!(sibling.dirty_digest, changed_source.dirty_digest);
}

#[test]
fn fingerprint_rejects_invalid_or_outside_exclusions() {
    let dir = tempdir().unwrap();
    let _repo = base_repo(dir.path());
    let outside = tempdir().unwrap();
    let outside_state = outside.path().join(".eggplan");
    fs::create_dir_all(&outside_state).unwrap();

    assert!(matches!(
        fingerprint(dir.path(), default_options(), Some(&outside_state)),
        Err(GitSubjectError::InvalidExclusion(_))
    ));
    // Excluding the worktree root itself would swallow the whole manifest and
    // report a falsely clean subject; the fingerprint contract rejects it.
    assert!(matches!(
        fingerprint(dir.path(), default_options(), Some(dir.path())),
        Err(GitSubjectError::InvalidExclusion(_))
    ));
    // A missing administrative root fails closed instead of silently
    // capturing an unexcluded manifest.
    let missing_state = dir.path().join(".eggplan");
    assert!(matches!(
        fingerprint(dir.path(), default_options(), Some(&missing_state)),
        Err(GitSubjectError::Io(_))
    ));
    // A bare repository is rejected as a Git failure before any exclusion is
    // evaluated, identically to the subject source.
    let bare = tempdir().unwrap();
    Repository::init_bare(bare.path()).unwrap();
    let bare_exclusion = dir.path().join(".eggplan");
    let bare_error =
        fingerprint(bare.path(), default_options(), Some(&bare_exclusion)).unwrap_err();
    assert_eq!(
        bare_error.to_string(),
        GitSubjectSource::new(bare.path(), "epr_fingerprint")
            .excluding_path(&bare_exclusion)
            .capture()
            .unwrap_err()
            .to_string()
    );
    // an exclusion outside the worktree: no exclusion is applied.
    let lenient = GitSubjectSource::new(dir.path(), "epr_fingerprint")
        .excluding_path(&outside_state)
        .capture()
        .unwrap();
    assert_eq!(lenient.dirty_digest, None);
    fs::write(dir.path().join("tracked.txt"), b"dirty\n").unwrap();
    assert!(
        GitSubjectSource::new(dir.path(), "epr_fingerprint")
            .excluding_path(&outside_state)
            .capture()
            .unwrap()
            .dirty_digest
            .is_some()
    );
}

#[test]
fn fingerprint_reports_the_same_typed_failures_as_the_subject_source() {
    let not_git = tempdir().unwrap();
    assert!(matches!(
        fingerprint(not_git.path(), default_options(), None),
        Err(GitSubjectError::NotGit(_))
    ));
    assert!(matches!(
        GitSubjectSource::new(not_git.path(), "epr_fingerprint").capture(),
        Err(GitSubjectError::NotGit(_))
    ));

    let unborn = tempdir().unwrap();
    Repository::init(unborn.path()).unwrap();
    let fingerprint_error = fingerprint(unborn.path(), default_options(), None).unwrap_err();
    let source_error = GitSubjectSource::new(unborn.path(), "epr_fingerprint")
        .capture()
        .unwrap_err();
    assert_eq!(fingerprint_error.to_string(), source_error.to_string());
    assert!(matches!(
        fingerprint_error,
        GitSubjectError::Unborn | GitSubjectError::Git(_)
    ));
}

#[test]
fn fingerprint_fails_closed_on_configured_bounds() {
    let dir = tempdir().unwrap();
    let _repo = base_repo(dir.path());
    fs::write(dir.path().join("one.txt"), b"one\n").unwrap();
    fs::write(dir.path().join("two.txt"), b"two\n").unwrap();

    let path_bound = GitSubjectOptions {
        max_paths: 1,
        ..GitSubjectOptions::default()
    };
    assert!(matches!(
        fingerprint(dir.path(), path_bound.clone(), None),
        Err(GitSubjectError::BoundExceeded)
    ));
    assert!(matches!(
        GitSubjectSource::new(dir.path(), "epr_fingerprint")
            .with_options(path_bound)
            .capture(),
        Err(GitSubjectError::BoundExceeded)
    ));

    let byte_bound = GitSubjectOptions {
        max_content_bytes: 4,
        ..GitSubjectOptions::default()
    };
    assert!(matches!(
        fingerprint(dir.path(), byte_bound.clone(), None),
        Err(GitSubjectError::BoundExceeded)
    ));
    assert!(matches!(
        GitSubjectSource::new(dir.path(), "epr_fingerprint")
            .with_options(byte_bound)
            .capture(),
        Err(GitSubjectError::BoundExceeded)
    ));

    let (parent, _subdir) = submodule_parent();
    fs::write(
        parent.path().join("module/tracked.txt"),
        b"changed inside module\n",
    )
    .unwrap();
    let depth_bound = GitSubjectOptions {
        max_submodule_depth: 0,
        ..GitSubjectOptions::default()
    };
    assert!(matches!(
        fingerprint(parent.path(), depth_bound.clone(), None),
        Err(GitSubjectError::BoundExceeded)
    ));
    assert!(matches!(
        GitSubjectSource::new(parent.path(), "epr_fingerprint")
            .with_options(depth_bound)
            .capture(),
        Err(GitSubjectError::BoundExceeded)
    ));
}

#[test]
fn fingerprint_reports_typed_discovery_failures() {
    let not_git = tempdir().unwrap();
    assert!(matches!(
        fingerprint(not_git.path(), default_options(), None),
        Err(GitSubjectError::NotGit(_))
    ));
    assert!(matches!(
        GitSubjectSource::new(not_git.path(), "epr_fingerprint").capture(),
        Err(GitSubjectError::NotGit(_))
    ));

    let unborn = tempdir().unwrap();
    Repository::init(unborn.path()).unwrap();
    let fingerprint_error = fingerprint(unborn.path(), default_options(), None).unwrap_err();
    let source_error = GitSubjectSource::new(unborn.path(), "epr_fingerprint")
        .capture()
        .unwrap_err();
    assert_eq!(fingerprint_error.to_string(), source_error.to_string());
    assert!(matches!(
        fingerprint_error,
        GitSubjectError::Unborn | GitSubjectError::Git(_)
    ));
}

#[test]
fn fingerprint_detects_dirty_content_change_between_two_captures() {
    let dir = tempdir().unwrap();
    let _repo = base_repo(dir.path());
    fs::write(dir.path().join("tracked.txt"), b"dirty a\n").unwrap();
    let e1 = fingerprint(dir.path(), default_options(), None).unwrap();
    fs::write(dir.path().join("tracked.txt"), b"dirty b\n").unwrap();
    let e2 = fingerprint(dir.path(), default_options(), None).unwrap();
    assert_ne!(e1, e2, "same HEAD and dirty class must not hide a change");
    assert_eq!(e1.revision, e2.revision);
    assert_eq!(e1.state, e2.state);
    assert_ne!(e1.dirty_digest, e2.dirty_digest);
    assert_eq!(
        fingerprint(dir.path(), default_options(), None).unwrap(),
        e2,
        "a stable worktree must be byte-stable across captures"
    );
}

#[test]
fn fingerprint_matches_the_repository_store_subject_it_will_be_bound_to() {
    let dir = tempdir().unwrap();
    let _repo = base_repo(dir.path());
    let store = RepositoryStore::open(dir.path().join(".eggplan")).unwrap();
    let state_root = store.root().to_path_buf();
    store
        .create(&simple_plan())
        .expect("plan creation must succeed");
    assert_eq!(
        fingerprint(dir.path(), default_options(), Some(&state_root))
            .unwrap()
            .dirty_digest,
        None
    );

    fs::write(dir.path().join("tracked.txt"), b"dirty worktree\n").unwrap();
    let store_subject = store.subject_source().capture().unwrap();
    let captured = fingerprint(dir.path(), default_options(), Some(&state_root)).unwrap();
    assert_eq!(captured.revision, store_subject.revision);
    assert_eq!(captured.state, store_subject.state);
    assert_eq!(captured.dirty_digest, store_subject.dirty_digest);
    assert!(captured.dirty_digest.is_some());

    // Administrative writes after binding must not perturb the proven subject.
    store
        .append_observation(
            &PlanId::new("ep_fingerprint").unwrap(),
            &simple_observation(),
        )
        .unwrap();
    assert_eq!(
        fingerprint(dir.path(), default_options(), Some(&state_root)).unwrap(),
        captured
    );
}

#[test]
fn fingerprint_schema_is_versioned_strict_and_self_consistent() {
    let dir = tempdir().unwrap();
    let _repo = base_repo(dir.path());
    fs::write(dir.path().join("tracked.txt"), b"dirty\n").unwrap();
    let captured = fingerprint(dir.path(), default_options(), None).unwrap();

    let encoded = serde_json::to_vec(&captured).unwrap();
    assert_eq!(
        String::from_utf8(encoded.clone()).unwrap(),
        format!(
            "{{\"schema_version\":1,\"revision\":\"{}\",\"state\":\"dirty\",\"dirty_digest\":\"{}\"}}",
            captured.revision,
            captured.dirty_digest.clone().unwrap()
        )
    );
    assert_eq!(
        serde_json::from_slice::<GitSubjectFingerprintV1>(&encoded).unwrap(),
        captured
    );
    assert!(serde_json::from_slice::<GitSubjectFingerprintV1>(
        br#"{"schema_version":1,"revision":"abc","state":"clean","dirty_digest":null,"paths":[]}"#
    )
    .is_err());

    let clean = fingerprint(
        &{
            let dir = tempdir().unwrap();
            let _repo = base_repo(dir.path());
            dir.keep()
        },
        default_options(),
        None,
    )
    .unwrap();
    assert_eq!(
        String::from_utf8(serde_json::to_vec(&clean).unwrap()).unwrap(),
        format!(
            "{{\"schema_version\":1,\"revision\":\"{}\",\"state\":\"clean\"}}",
            clean.revision
        )
    );

    captured.validate().unwrap();
    assert!(captured.clone().validate().is_ok());
    for invalid in [
        GitSubjectFingerprintV1 {
            schema_version: 2,
            ..captured.clone()
        },
        GitSubjectFingerprintV1 {
            revision: String::new(),
            ..captured.clone()
        },
        GitSubjectFingerprintV1 {
            state: SubjectState::Clean,
            ..captured.clone()
        },
        GitSubjectFingerprintV1 {
            dirty_digest: None,
            ..captured.clone()
        },
        GitSubjectFingerprintV1 {
            dirty_digest: Some("sha256:NOTHEX".into()),
            ..captured.clone()
        },
    ] {
        assert!(
            matches!(
                invalid.validate(),
                Err(GitSubjectError::InvalidFingerprint(_))
            ),
            "invalid fingerprint accepted"
        );
    }
}

fn simple_plan() -> Plan {
    Plan::new(
        PlanId::new("ep_fingerprint").unwrap(),
        "fingerprint binding subject",
        vec![PlanItem {
            id: PlanItemId::new("epi_fingerprint").unwrap(),
            position: 0,
            parent: None,
            dependencies: vec![],
            status: PlanItemStatus::Pending,
            description: "bind a fingerprint".into(),
            criteria: vec![],
            blocker: None,
            next_action: None,
        }],
    )
    .unwrap()
}

fn simple_observation() -> EvidenceObservation {
    EvidenceObservation::finalize(EvidenceObservationInput {
        id: EvidenceObservationId::new("epe_fingerprint").unwrap(),
        provider_id: EvidenceProviderId::new("epp_test").unwrap(),
        kind: EvidenceKind::Test,
        status: EvidenceStatus::Passed,
        subject: eggplan_core::SubjectRevision {
            subject_kind: "git".into(),
            repository_id: "epr_fingerprint".into(),
            revision: "0".repeat(40),
            state: SubjectState::Dirty,
            dirty_digest: Some(format!("sha256:{}", "a".repeat(64))),
        },
        observed_at_unix_ms: 1_700_000_000_000,
        invocation_ref: None,
        verification_digest: Some(
            VerificationDigest::new(format!("sha256:{}", "b".repeat(64))).unwrap(),
        ),
        result_metadata: Default::default(),
        artifacts: vec![],
    })
    .unwrap()
}

#[test]
fn fingerprint_and_source_agree_on_the_default_options_path_only() {
    // A non-default bound applies to both entry points identically; the
    // fingerprint never widens or narrows the documented defaults.
    let dir = tempdir().unwrap();
    let _repo = base_repo(dir.path());
    fs::write(dir.path().join("tracked.txt"), b"dirty\n").unwrap();
    let options = GitSubjectOptions {
        max_paths: 16,
        max_content_bytes: 1024,
        max_submodule_depth: 2,
    };
    let source = GitSubjectSource::new(dir.path(), "epr_fingerprint")
        .with_options(options.clone())
        .capture()
        .unwrap();
    let captured = fingerprint(dir.path(), options, None).unwrap();
    assert_eq!(captured.revision, source.revision);
    assert_eq!(captured.state, source.state);
    assert_eq!(captured.dirty_digest, source.dirty_digest);
    assert_eq!(
        GitSubjectOptions::default().max_paths,
        default_options().max_paths
    );
    assert_eq!(10_000, GitSubjectOptions::default().max_paths);
    assert_eq!(
        64 * 1024 * 1024,
        GitSubjectOptions::default().max_content_bytes
    );
    assert_eq!(8, GitSubjectOptions::default().max_submodule_depth);
}

#[test]
fn fingerprint_is_reachable_from_an_external_root_argument() {
    // The signature takes any `AsRef<Path>` root, including a path inside the
    // worktree, and discovery is unchanged.
    let dir = tempdir().unwrap();
    let _repo = base_repo(dir.path());
    let nested: PathBuf = dir.path().join("other.txt");
    assert_eq!(
        fingerprint(&nested, default_options(), None).unwrap(),
        fingerprint(dir.path(), default_options(), None).unwrap()
    );
}
