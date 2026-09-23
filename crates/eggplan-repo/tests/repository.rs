use eggplan_core::{
    EvidenceKind, EvidenceObservation, EvidenceObservationId, EvidenceObservationInput,
    EvidenceProviderId, EvidenceStatus, Plan, PlanId, PlanItem, PlanItemId, PlanItemStatus,
    PlanStatus, SubjectRevision, SubjectState,
};
use eggplan_repo::{
    GitSubjectOptions, GitSubjectSource, PlanStore, RepoError, RepositoryStore, StoreOptions,
};
use fs2::FileExt;
use git2::{Repository, Signature};
use std::{
    fs,
    io::Write,
    process::{Command, Stdio},
    sync::{Arc, Barrier},
    time::Duration,
};
use tempfile::tempdir;

fn plan() -> Plan {
    Plan::new(
        PlanId::new("ep_store").unwrap(),
        "original",
        vec![PlanItem {
            id: PlanItemId::new("epi_one").unwrap(),
            position: 0,
            parent: None,
            dependencies: vec![],
            status: PlanItemStatus::Pending,
            description: "step".into(),
            criteria: vec![],
            blocker: None,
            next_action: None,
        }],
    )
    .unwrap()
}
fn next(mut plan: Plan, objective: &str) -> Plan {
    plan.revision += 1;
    plan.objective = objective.into();
    plan
}

fn observation(id: &str, status: EvidenceStatus) -> EvidenceObservation {
    EvidenceObservation::finalize(EvidenceObservationInput {
        id: EvidenceObservationId::new(format!("epe_{id}")).unwrap(),
        provider_id: EvidenceProviderId::new("epp_test").unwrap(),
        kind: EvidenceKind::Test,
        status,
        subject: SubjectRevision {
            subject_kind: "git".into(),
            repository_id: "epr_test".into(),
            revision: "abc123".into(),
            state: SubjectState::Clean,
            dirty_digest: None,
        },
        observed_at_unix_ms: 1_700_000_000_000,
        invocation_ref: Some("cargo test".into()),
        result_metadata: Default::default(),
        artifacts: vec![],
    })
    .unwrap()
}

#[test]
fn create_load_list_cas_and_reopen() {
    let dir = tempdir().unwrap();
    let state = dir.path().join(".eggplan");
    let store = RepositoryStore::open(&state).unwrap();
    let p = plan();
    store.create(&p).unwrap();
    assert_eq!(store.get(&p.id).unwrap(), p);
    assert_eq!(store.list().unwrap(), vec![p.id.clone()]);
    let updated = store
        .compare_and_swap(&p.id, 0, &next(p.clone(), "updated"))
        .unwrap();
    assert_eq!(updated.revision, 1);
    assert!(matches!(
        store.compare_and_swap(&p.id, 0, &next(p.clone(), "stale")),
        Err(RepoError::Conflict { current: 1, .. })
    ));
    let reopened = RepositoryStore::open(&state).unwrap();
    assert_eq!(reopened.get(&p.id).unwrap(), updated);
    assert!(matches!(
        store.create(&updated),
        Err(RepoError::AlreadyExists(_))
    ));
}

#[test]
fn competing_same_revision_writers_have_exactly_one_winner() {
    let dir = tempdir().unwrap();
    let store = RepositoryStore::open(dir.path().join(".eggplan")).unwrap();
    let initial = plan();
    store.create(&initial).unwrap();
    let barrier = Arc::new(Barrier::new(3));
    let mut joins = Vec::new();
    for objective in ["writer-a", "writer-b"] {
        let store = store.clone();
        let barrier = barrier.clone();
        let initial = initial.clone();
        let objective = objective.to_owned();
        joins.push(std::thread::spawn(move || {
            barrier.wait();
            let candidate = next(initial.clone(), &objective);
            store.compare_and_swap(&initial.id, 0, &candidate)
        }));
    }
    barrier.wait();
    let results: Vec<_> = joins.into_iter().map(|j| j.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|r| matches!(r, Err(RepoError::Conflict { .. })))
            .count(),
        1
    );
    assert_eq!(
        store
            .get(&PlanId::new("ep_store").unwrap())
            .unwrap()
            .revision,
        1
    );
}

#[test]
fn independent_processes_cannot_both_commit_the_same_revision() {
    if std::env::var_os("EGGPLAN_STORE_CHILD").is_some() {
        let root = std::env::var_os("EGGPLAN_STORE_ROOT").unwrap();
        let objective = std::env::var("EGGPLAN_STORE_OBJECTIVE").unwrap();
        let store = RepositoryStore::open(root).unwrap();
        let id = PlanId::new("ep_store").unwrap();
        let mut candidate = store.get(&id).unwrap();
        candidate.revision = 1;
        candidate.objective = objective;
        let result = store.compare_and_swap(&id, 0, &candidate);
        std::process::exit(if result.is_ok() { 0 } else { 20 });
    }

    let dir = tempdir().unwrap();
    let root = dir.path().join(".eggplan");
    let store = RepositoryStore::open(&root).unwrap();
    store.create(&plan()).unwrap();
    let executable = std::env::current_exe().unwrap();
    let launch = |objective: &str| {
        Command::new(&executable)
            .arg("--exact")
            .arg("independent_processes_cannot_both_commit_the_same_revision")
            .arg("--nocapture")
            .env("EGGPLAN_STORE_CHILD", "1")
            .env("EGGPLAN_STORE_ROOT", &root)
            .env("EGGPLAN_STORE_OBJECTIVE", objective)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap()
    };
    let mut first = launch("process-a");
    let mut second = launch("process-b");
    let mut codes = [first.wait().unwrap().code(), second.wait().unwrap().code()];
    codes.sort();
    assert_eq!(codes, [Some(0), Some(20)]);
    assert_eq!(
        store
            .get(&PlanId::new("ep_store").unwrap())
            .unwrap()
            .revision,
        1
    );
}

#[test]
fn lock_timeout_and_abandoned_staging_are_reported() {
    let dir = tempdir().unwrap();
    let state = dir.path().join(".eggplan");
    let store = RepositoryStore::open_with_options(
        &state,
        StoreOptions {
            lock_timeout: Duration::from_millis(30),
        },
    )
    .unwrap();
    let initial = plan();
    store.create(&initial).unwrap();
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(state.join(".lock"))
        .unwrap();
    file.lock_exclusive().unwrap();
    assert!(matches!(
        store.compare_and_swap(&initial.id, 0, &next(initial.clone(), "locked")),
        Err(RepoError::LockTimeout)
    ));
    file.unlock().unwrap();
    let staging = state.join("plans/ep_store/.tmp-interrupted");
    fs::File::create(&staging)
        .unwrap()
        .write_all(b"partial")
        .unwrap();
    assert_eq!(store.abandoned_staging_files().unwrap(), vec![staging]);
    assert_eq!(store.get(&initial.id).unwrap().revision, 0);
}

#[test]
fn stale_revision_is_rejected_after_lock_file_disappears_and_store_reopens() {
    let dir = tempdir().unwrap();
    let state = dir.path().join(".eggplan");
    let store = RepositoryStore::open(&state).unwrap();
    let initial = plan();
    store.create(&initial).unwrap();
    store
        .compare_and_swap(&initial.id, 0, &next(initial.clone(), "committed"))
        .unwrap();
    fs::remove_file(state.join(".lock")).unwrap();
    let reopened = RepositoryStore::open(&state).unwrap();
    assert!(matches!(
        reopened.compare_and_swap(&initial.id, 0, &next(initial.clone(), "stale")),
        Err(RepoError::Conflict { current: 1, .. })
    ));
}

#[cfg(unix)]
#[test]
fn symlinked_plan_directory_is_rejected() {
    use std::os::unix::fs::symlink;
    let dir = tempdir().unwrap();
    let outside = tempdir().unwrap();
    let state = dir.path().join(".eggplan");
    let store = RepositoryStore::open(&state).unwrap();
    symlink(outside.path(), state.join("plans/ep_store")).unwrap();
    assert!(matches!(
        store.get(&PlanId::new("ep_store").unwrap()),
        Err(RepoError::UnsafePath(_))
    ));
    assert!(fs::read_dir(outside.path()).unwrap().next().is_none());
}

#[cfg(unix)]
#[test]
fn symlinked_plan_file_is_rejected_without_touching_target() {
    use std::os::unix::fs::symlink;
    let dir = tempdir().unwrap();
    let outside = tempdir().unwrap();
    let state = dir.path().join(".eggplan");
    let store = RepositoryStore::open(&state).unwrap();
    let p = plan();
    store.create(&p).unwrap();
    let plan_path = state.join("plans/ep_store/plan.json");
    fs::remove_file(&plan_path).unwrap();
    let outside_file = outside.path().join("victim.json");
    fs::write(&outside_file, b"unchanged").unwrap();
    symlink(&outside_file, &plan_path).unwrap();
    assert!(matches!(store.get(&p.id), Err(RepoError::UnsafePath(_))));
    assert_eq!(fs::read(outside_file).unwrap(), b"unchanged");
}

#[test]
fn illegal_lifecycle_updates_are_rejected() {
    let dir = tempdir().unwrap();
    let store = RepositoryStore::open(dir.path().join(".eggplan")).unwrap();
    let p = plan();
    store.create(&p).unwrap();
    let mut closed = p.clone();
    closed.revision = 1;
    closed.status = PlanStatus::Closed;
    assert!(matches!(
        store.compare_and_swap(&p.id, 0, &closed),
        Err(RepoError::InvalidTransition)
    ));
    let mut completed = p.clone();
    completed.revision = 1;
    completed.items[0].status = PlanItemStatus::Completed;
    assert!(matches!(
        store.compare_and_swap(&p.id, 0, &completed),
        Err(RepoError::InvalidTransition)
    ));
}

#[test]
fn unknown_or_corrupt_plan_is_rejected_on_reopen() {
    let dir = tempdir().unwrap();
    let state = dir.path().join(".eggplan");
    let store = RepositoryStore::open(&state).unwrap();
    let p = plan();
    store.create(&p).unwrap();
    let stored_path = state.join("plans/ep_store/plan.json");
    let original = fs::read_to_string(&stored_path).unwrap();
    fs::write(&stored_path, original.replace("original", "tampered")).unwrap();
    assert!(matches!(store.get(&p.id), Err(RepoError::Corrupt { .. })));
    fs::write(&stored_path, b"{\"storage_version\":99}").unwrap();
    assert!(matches!(store.get(&p.id), Err(RepoError::Corrupt { .. })));
}

#[test]
fn observation_ledger_is_append_only_idempotent_and_reopens() {
    let dir = tempdir().unwrap();
    let state = dir.path().join(".eggplan");
    let store = RepositoryStore::open(&state).unwrap();
    let p = plan();
    store.create(&p).unwrap();
    let passed = observation("one", EvidenceStatus::Passed);
    store.append_observation(&p.id, &passed).unwrap();
    store.append_observation(&p.id, &passed).unwrap();
    assert_eq!(store.get_observation(&p.id, passed.id()).unwrap(), passed);
    assert_eq!(
        store.list_observations(&p.id).unwrap(),
        vec![passed.clone()]
    );
    assert!(matches!(
        store.append_observation(&p.id, &observation("one", EvidenceStatus::Failed)),
        Err(RepoError::ObservationConflict(_))
    ));

    let reopened = RepositoryStore::open(&state).unwrap();
    assert_eq!(
        reopened.list_observations(&p.id).unwrap(),
        vec![passed.clone()]
    );
    let path = state.join("plans/ep_store/evidence/epe_one.json");
    let json = fs::read_to_string(&path).unwrap();
    fs::write(&path, json.replace("cargo test", "cargo build")).unwrap();
    assert!(matches!(
        reopened.get_observation(&p.id, passed.id()),
        Err(RepoError::Corrupt { .. })
    ));
}

#[cfg(unix)]
#[test]
fn symlinked_observation_ledger_is_rejected() {
    use std::os::unix::fs::symlink;
    let dir = tempdir().unwrap();
    let outside = tempdir().unwrap();
    let state = dir.path().join(".eggplan");
    let store = RepositoryStore::open(&state).unwrap();
    let p = plan();
    store.create(&p).unwrap();
    symlink(outside.path(), state.join("plans/ep_store/evidence")).unwrap();
    assert!(matches!(
        store.append_observation(&p.id, &observation("one", EvidenceStatus::Passed)),
        Err(RepoError::UnsafePath(_))
    ));
    assert!(fs::read_dir(outside.path()).unwrap().next().is_none());
}

fn init_git_repo(root: &std::path::Path) -> Repository {
    let repo = Repository::init(root).unwrap();
    fs::write(root.join("tracked.txt"), b"base\n").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new("tracked.txt")).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = Signature::now("Eggplan Test", "eggplan@example.invalid").unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
        .unwrap();
    drop(tree);
    repo
}

#[test]
fn git_subject_clean_staged_untracked_and_dirty_fingerprints() {
    let dir = tempdir().unwrap();
    let repo = init_git_repo(dir.path());
    let source = GitSubjectSource::new(repo.workdir().unwrap(), "epr_fixture");
    let clean = source.capture().unwrap();
    assert_eq!(clean.state, eggplan_core::SubjectState::Clean);
    fs::write(dir.path().join("tracked.txt"), b"modified\n").unwrap();
    let dirty1 = source.capture().unwrap();
    let dirty2 = source.capture().unwrap();
    assert_eq!(dirty1, dirty2);
    assert_eq!(dirty1.state, eggplan_core::SubjectState::Dirty);
    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new("tracked.txt")).unwrap();
    index.write().unwrap();
    let staged = source.capture().unwrap();
    assert_ne!(staged.dirty_digest, dirty1.dirty_digest);
    fs::write(dir.path().join("untracked.txt"), b"untracked").unwrap();
    let untracked = source.capture().unwrap();
    assert_ne!(untracked.dirty_digest, staged.dirty_digest);
    assert_eq!(
        source
            .clone()
            .with_options(GitSubjectOptions {
                max_paths: 1,
                ..GitSubjectOptions::default()
            })
            .capture()
            .unwrap_err()
            .to_string(),
        "dirty state exceeded configured path/content/depth bounds"
    );
}

#[test]
fn repository_managed_state_does_not_perturb_subject() {
    let dir = tempdir().unwrap();
    let repo = init_git_repo(dir.path());
    let state = dir.path().join(".eggplan");
    let store = RepositoryStore::open(&state).unwrap();
    let a = store.subject_source().capture().unwrap();
    store.create(&plan()).unwrap();
    store
        .append_observation(
            &PlanId::new("ep_store").unwrap(),
            &observation("bound", EvidenceStatus::Passed),
        )
        .unwrap();
    let b = store.subject_source().capture().unwrap();
    assert_eq!(a, b);

    fs::write(dir.path().join("tracked.txt"), b"changed source\n").unwrap();
    let c = store.subject_source().capture().unwrap();
    assert_ne!(b, c);
    drop(repo);
}

#[test]
fn repository_reopen_rejects_unknown_nested_plan_fields() {
    let dir = tempdir().unwrap();
    let store = RepositoryStore::open(dir.path().join(".eggplan")).unwrap();
    let p = plan();
    store.create(&p).unwrap();
    let path = store.root().join("plans/ep_store/plan.json");
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    value["plan"]["items"][0]["future"] = serde_json::json!("unsupported");
    fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(matches!(store.get(&p.id), Err(RepoError::Corrupt { .. })));
}

#[test]
fn git_subject_non_git_path_is_explicit() {
    let dir = tempdir().unwrap();
    assert!(
        GitSubjectSource::new(dir.path(), "epr_test")
            .capture()
            .is_err()
    );
}

#[test]
fn git_subject_includes_submodule_head_and_nested_dirty_content() {
    let parent = tempdir().unwrap();
    let subdir = tempdir().unwrap();
    let subrepo = init_git_repo(subdir.path());
    let parent_repo = init_git_repo(parent.path());
    drop(parent_repo);
    let git = |args: &[&str]| {
        let result = std::process::Command::new("git")
            .arg("-c")
            .arg("protocol.file.allow=always")
            .arg("-c")
            .arg("core.hooksPath=/dev/null")
            .arg("-C")
            .arg(parent.path())
            .args(args)
            .status()
            .unwrap();
        assert!(result.success(), "git {args:?} failed");
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
    let source = GitSubjectSource::new(parent.path(), "epr_fixture");
    assert_eq!(
        source.capture().unwrap().state,
        eggplan_core::SubjectState::Clean
    );
    fs::write(
        parent.path().join("module/tracked.txt"),
        b"changed inside module\n",
    )
    .unwrap();
    let nested_dirty = source.capture().unwrap();
    fs::write(
        parent.path().join("module/tracked.txt"),
        b"changed differently\n",
    )
    .unwrap();
    assert_ne!(
        nested_dirty.dirty_digest,
        source.capture().unwrap().dirty_digest
    );
}
