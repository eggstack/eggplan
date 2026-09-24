use crate::{GitSubjectError, GitSubjectSource};
use eggplan_core::{
    AssessmentStatus, ClosureCandidate, ClosureId, ClosureRecord, EvidenceError,
    EvidenceObservation, EvidenceObservationId, EvidenceSupersessionRecord, Plan, PlanId,
    PlanStatus, ProviderDescriptor, ProviderRegistry, SubjectRevision, ValidationError,
    assess_plan, digest_json, effective_observations,
};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};
use tempfile::{Builder, NamedTempFile};
use thiserror::Error;
use uuid::Uuid;

const CONFIG_VERSION: u32 = 1;
const PLAN_FILE: &str = "plan.json";
const EVIDENCE_DIR: &str = "evidence";
const SUPERSESSIONS_DIR: &str = "supersessions";
const MAX_PLAN_FILE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_OBSERVATION_FILE_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone)]
pub struct StoreOptions {
    pub lock_timeout: Duration,
}
impl Default for StoreOptions {
    fn default() -> Self {
        Self {
            lock_timeout: Duration::from_secs(5),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RepositoryConfig {
    schema_version: u32,
    repository_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredPlan {
    storage_version: u32,
    plan_digest: String,
    plan: Plan,
}

#[derive(Debug, Error)]
pub enum RepoError {
    #[error("filesystem operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("plan validation or serialization failed: {0}")]
    Domain(#[from] Box<dyn std::error::Error + Send + Sync>),
    #[error("domain validation failed: {0}")]
    Validation(#[from] ValidationError),
    #[error("JSON serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("configuration is invalid: {0}")]
    Config(String),
    #[error("plan {0} already exists")]
    AlreadyExists(PlanId),
    #[error("plan {0} does not exist")]
    NotFound(PlanId),
    #[error("revision conflict for {id}: expected {expected}, current {current}")]
    Conflict {
        id: PlanId,
        expected: u64,
        current: u64,
    },
    #[error("candidate must retain plan identity and advance revision by exactly one")]
    InvalidUpdate,
    #[error("plan or item state transition is not allowed")]
    InvalidTransition,
    #[error("plans may only enter Closed through guarded closure finalization")]
    GuardedClosureRequired,
    #[error("cooperative repository lock timed out")]
    LockTimeout,
    #[error("replacement succeeded but directory durability could not be confirmed: {0}")]
    DurabilityUnknown(#[source] std::io::Error),
    #[error("symlink or non-directory found in managed state path {0}")]
    UnsafePath(PathBuf),
    #[error("unknown or corrupt canonical plan at {path}: {reason}")]
    Corrupt { path: PathBuf, reason: String },
    #[error("invalid canonical Plan at {path}: {reason}")]
    InvalidPlan { path: PathBuf, reason: String },
    #[error("evidence observation validation failed: {0}")]
    Evidence(#[from] EvidenceError),
    #[error("observation {0} already exists with different content")]
    ObservationConflict(EvidenceObservationId),
    #[error("observation {0} does not exist")]
    ObservationNotFound(EvidenceObservationId),
    #[error("observation count limit for this plan has been reached")]
    ObservationLimit,
    #[error("closure recovery is required for plan {0}")]
    RecoveryRequired(PlanId),
    #[error("closure subject changed before finalization")]
    ClosureSubjectStale,
    #[error("closure subject drifted during finalization")]
    ClosureSubjectDrift,
    #[error("closure subject capture failed: {0}")]
    ClosureSubjectCapture(#[source] GitSubjectError),
}

/// Subject capture seam used by guarded closure finalization.
///
/// Production finalization always wires this to the repository's configured
/// `GitSubjectSource`. Crate-internal tests inject a deterministic scripted
/// capture to exercise drift and stale-before-finalize regressions without
/// exposing a broad public provider framework. The seam is crate-private:
/// it is not part of the supported public API and cannot be reached by a
/// downstream crate.
pub(crate) trait SubjectCapture: Send + Sync {
    fn capture(&self) -> Result<SubjectRevision, GitSubjectError>;
}

pub(crate) struct GitSubjectCapture<'a> {
    source: &'a GitSubjectSource,
}

impl<'a> GitSubjectCapture<'a> {
    pub(crate) fn new(source: &'a GitSubjectSource) -> Self {
        Self { source }
    }
}

impl SubjectCapture for GitSubjectCapture<'_> {
    fn capture(&self) -> Result<SubjectRevision, GitSubjectError> {
        self.source.capture()
    }
}

pub trait PlanStore {
    fn create(&self, plan: &Plan) -> Result<(), RepoError>;
    fn get(&self, id: &PlanId) -> Result<Plan, RepoError>;
    fn list(&self) -> Result<Vec<PlanId>, RepoError>;
    fn compare_and_swap(
        &self,
        id: &PlanId,
        expected_revision: u64,
        next: &Plan,
    ) -> Result<Plan, RepoError>;
    fn append_observation(
        &self,
        plan_id: &PlanId,
        observation: &EvidenceObservation,
    ) -> Result<(), RepoError>;
    fn get_observation(
        &self,
        plan_id: &PlanId,
        observation_id: &EvidenceObservationId,
    ) -> Result<EvidenceObservation, RepoError>;
    fn list_observations(&self, plan_id: &PlanId) -> Result<Vec<EvidenceObservation>, RepoError>;
}

#[derive(Debug, Clone)]
pub struct RepositoryStore {
    root: PathBuf,
    options: StoreOptions,
    repository_id: String,
}

impl RepositoryStore {
    /// Open an existing repository without creating files, recovering pending
    /// closures, or otherwise mutating state. Intended for inspection/check
    /// commands that must report recovery requirements to the operator.
    pub fn open_read_only(root: impl AsRef<Path>) -> Result<Self, RepoError> {
        let root = root.as_ref().to_path_buf();
        check_dir(&root)?;
        check_dir(&root.join("plans"))?;
        let config_path = root.join("config.toml");
        let metadata = fs::symlink_metadata(&config_path).map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                RepoError::Config("repository is not initialized".into())
            } else {
                RepoError::Io(error)
            }
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(RepoError::UnsafePath(config_path));
        }
        let mut input = String::new();
        File::open(&config_path)?.read_to_string(&mut input)?;
        let config: RepositoryConfig =
            toml::from_str(&input).map_err(|error| RepoError::Config(error.to_string()))?;
        validate_repository_config(&config)?;
        Ok(Self {
            root,
            options: StoreOptions::default(),
            repository_id: config.repository_id,
        })
    }

    /// List pending closure transactions without attempting recovery.
    pub fn pending_closures(&self) -> Result<Vec<PlanId>, RepoError> {
        check_dir(&self.root)?;
        let plans_dir = self.root.join("plans");
        check_dir(&plans_dir)?;
        let mut pending = Vec::new();
        for entry in fs::read_dir(&plans_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_symlink() {
                return Err(RepoError::UnsafePath(entry.path()));
            }
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| RepoError::Config("non-Unicode plan directory name".into()))?;
            let id = PlanId::new(name).map_err(|error| RepoError::Config(error.to_string()))?;
            let path = entry.path().join("closure.pending.json");
            match fs::symlink_metadata(&path) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(RepoError::Io(error)),
                Ok(meta) if meta.file_type().is_symlink() || !meta.is_file() => {
                    return Err(RepoError::UnsafePath(path));
                }
                Ok(_) => pending.push(id),
            }
        }
        pending.sort();
        Ok(pending)
    }

    /// Return the immutable closure record after the same deep validation
    /// performed when loading a closed Plan.
    pub fn closure_record(&self, id: &PlanId) -> Result<Option<ClosureRecord>, RepoError> {
        let plan = self.get(id)?;
        if plan.status != PlanStatus::Closed {
            return Ok(None);
        }
        let path = self.plan_dir(id).join("closure.json");
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(RepoError::UnsafePath(path));
        }
        if metadata.len() > 8 * 1024 * 1024 {
            return Err(RepoError::Corrupt {
                path,
                reason: "closure record exceeds size limit".into(),
            });
        }
        let bytes = fs::read(&path)?;
        let record: ClosureRecord =
            serde_json::from_slice(&bytes).map_err(|error| RepoError::Corrupt {
                path: path.clone(),
                reason: error.to_string(),
            })?;
        record
            .validate(&plan)
            .map_err(|reason| RepoError::Corrupt { path, reason })?;
        Ok(Some(record))
    }

    fn recover_pending_closures(&self) -> Result<(), RepoError> {
        let _guard = self.lock()?;
        let root = self.root.join("plans");
        for entry in fs::read_dir(&root)? {
            let entry = entry?;
            if entry.file_type()?.is_symlink() {
                return Err(RepoError::UnsafePath(entry.path()));
            }
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let dir = entry.path();
            check_dir(&dir)?;
            let pending = dir.join("closure.pending.json");
            match fs::symlink_metadata(&pending) {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => return Err(RepoError::Io(e)),
                Ok(meta) if meta.file_type().is_symlink() || !meta.is_file() => {
                    return Err(RepoError::UnsafePath(pending));
                }
                Ok(_) => {}
            }
            let record: ClosureRecord =
                serde_json::from_slice(&fs::read(&pending)?).map_err(|e| RepoError::Corrupt {
                    path: pending.clone(),
                    reason: e.to_string(),
                })?;
            let plan_path = dir.join(PLAN_FILE);
            if fs::symlink_metadata(&plan_path)?.file_type().is_symlink() {
                return Err(RepoError::UnsafePath(plan_path));
            }
            let stored: StoredPlan =
                serde_json::from_slice(&fs::read(&plan_path)?).map_err(|e| RepoError::Corrupt {
                    path: plan_path.clone(),
                    reason: e.to_string(),
                })?;
            let plan = stored.plan;
            if digest_json(&plan)? != stored.plan_digest {
                return Err(RepoError::Corrupt {
                    path: plan_path,
                    reason: "plan content digest mismatch during closure recovery".into(),
                });
            }
            let final_path = dir.join("closure.json");
            if fs::symlink_metadata(&final_path).is_ok() {
                return Err(RepoError::Corrupt {
                    path: final_path,
                    reason: "both pending and final closure records exist".into(),
                });
            }
            if plan.status == PlanStatus::Closed
                && plan.revision == record.final_plan_revision
                && digest_json(&plan)? == record.final_plan_digest
            {
                record
                    .validate(&plan)
                    .map_err(|reason| RepoError::Corrupt {
                        path: pending.clone(),
                        reason,
                    })?;
                fs::rename(&pending, &final_path)?;
                #[cfg(unix)]
                File::open(&dir)?
                    .sync_all()
                    .map_err(RepoError::DurabilityUnknown)?;
            } else if plan.status != PlanStatus::Closed
                && plan.revision == record.candidate.source_revision
                && digest_json(&plan)? == record.candidate.source_plan_digest
            {
                fs::remove_file(&pending)?;
                #[cfg(unix)]
                File::open(&dir)?
                    .sync_all()
                    .map_err(RepoError::DurabilityUnknown)?;
            } else {
                return Err(RepoError::Corrupt {
                    path: pending,
                    reason: "pending closure does not match source or target plan".into(),
                });
            }
        }
        Ok(())
    }

    /// Finalize closure after recomputing assessment under the repository lock.
    ///
    /// Authoritative Git `SubjectRevision` recapture lives inside this
    /// boundary. Callers may not pass a caller-owned current subject. The
    /// finalizer captures S1 before assessment replay and S2 immediately
    /// before the first canonical closure write. Any drift aborts with a
    /// typed error and produces no pending/final closure state and no Closed
    /// Plan.
    ///
    /// This is the only supported externally callable guarded closure entry
    /// point. There is no public alternate finalizer that accepts an injected
    /// subject capture source.
    pub fn finalize_closure(
        &self,
        candidate: &ClosureCandidate,
        closure_id: ClosureId,
        finalized_at_unix_ms: u64,
    ) -> Result<(Plan, ClosureRecord), RepoError> {
        let source = self.subject_source();
        let capture = GitSubjectCapture::new(&source);
        self.finalize_closure_with_capture(candidate, closure_id, finalized_at_unix_ms, &capture)
    }

    /// Crate-private hook that lets crate-internal tests inject a deterministic
    /// subject capture sequence. Production `finalize_closure` always wires
    /// the repository's own `GitSubjectSource`. This method is not part of the
    /// supported public API and is not reachable from a downstream crate.
    pub(crate) fn finalize_closure_with_capture(
        &self,
        candidate: &ClosureCandidate,
        closure_id: ClosureId,
        finalized_at_unix_ms: u64,
        capture: &dyn SubjectCapture,
    ) -> Result<(Plan, ClosureRecord), RepoError> {
        self.finalize_closure_inner(candidate, closure_id, finalized_at_unix_ms, capture)
    }

    fn finalize_closure_inner(
        &self,
        candidate: &ClosureCandidate,
        closure_id: ClosureId,
        finalized_at_unix_ms: u64,
        capture: &dyn SubjectCapture,
    ) -> Result<(Plan, ClosureRecord), RepoError> {
        let _guard = self.lock()?;
        let current = self.load_unlocked(&candidate.plan_id)?;
        if current.revision != candidate.source_revision
            || digest_json(&current)? != candidate.source_plan_digest
        {
            return Err(RepoError::InvalidUpdate);
        }
        let s1 = capture
            .capture()
            .map_err(RepoError::ClosureSubjectCapture)?;
        if s1 != candidate.subject {
            return Err(RepoError::ClosureSubjectStale);
        }
        let observations = self.list_observations(&candidate.plan_id)?;
        let supersessions = self.list_supersessions_unlocked(&candidate.plan_id)?;
        let effective =
            effective_observations(&observations, &supersessions).map_err(RepoError::Config)?;
        let effective: Vec<_> = effective.into_iter().cloned().collect();
        let mut providers = ProviderRegistry::default();
        for entry in &candidate.provider_policy {
            providers.register_trusted(ProviderDescriptor::new(
                entry.provider_id.clone(),
                entry.class.clone(),
                entry.allowed_kinds.iter().copied(),
            )?)?;
        }
        let assessment = assess_plan(&current, &s1, &effective, &providers);
        if assessment.status != AssessmentStatus::Complete || assessment != candidate.assessment {
            return Err(RepoError::InvalidUpdate);
        }
        for (id, digest) in &candidate.satisfying_observations {
            if !observations
                .iter()
                .any(|o| o.id() == id && o.content_digest() == digest)
            {
                return Err(RepoError::InvalidUpdate);
            }
        }
        let mut expected_observations: Vec<_> = candidate
            .assessment
            .items
            .iter()
            .flat_map(|i| &i.criteria)
            .flat_map(|c| &c.requirements)
            .flat_map(|r| &r.satisfying_observation_ids)
            .map(|id| {
                let digest = observations
                    .iter()
                    .find(|o| o.id() == id)
                    .map(|o| o.content_digest().to_string())
                    .ok_or(RepoError::InvalidUpdate)?;
                Ok((id.clone(), digest))
            })
            .collect::<Result<_, RepoError>>()?;
        expected_observations.sort();
        if expected_observations != candidate.satisfying_observations {
            return Err(RepoError::InvalidUpdate);
        }
        if digest_json(&candidate.provider_policy)? != candidate.provider_policy_digest {
            return Err(RepoError::InvalidUpdate);
        }
        let lineage: Vec<_> = supersessions
            .iter()
            .map(|r| (r.id.clone(), r.content_digest.clone()))
            .collect();
        if lineage != candidate.supersession_digests {
            return Err(RepoError::InvalidUpdate);
        }
        let mut closed = current.clone();
        closed.revision = closed
            .revision
            .checked_add(1)
            .ok_or(RepoError::InvalidUpdate)?;
        closed.status = PlanStatus::Closed;
        let record =
            ClosureRecord::finalize(closure_id, candidate.clone(), &closed, finalized_at_unix_ms)
                .map_err(RepoError::Config)?;
        let dir = self.plan_dir(&candidate.plan_id);
        let pending = dir.join("closure.pending.json");
        let final_path = dir.join("closure.json");
        if final_path.exists() || pending.exists() {
            return Err(RepoError::InvalidUpdate);
        }
        let s2 = capture
            .capture()
            .map_err(RepoError::ClosureSubjectCapture)?;
        if s2 != s1 || s2 != candidate.subject {
            return Err(RepoError::ClosureSubjectDrift);
        }
        atomic_write(&pending, &serde_json::to_vec(&record)?)?;
        atomic_write(&self.plan_path(&candidate.plan_id), &encode_plan(&closed)?)?;
        fs::rename(&pending, &final_path)?;
        #[cfg(unix)]
        File::open(&dir)?
            .sync_all()
            .map_err(RepoError::DurabilityUnknown)?;
        Ok((closed, record))
    }

    pub fn append_supersession(
        &self,
        plan_id: &PlanId,
        record: &EvidenceSupersessionRecord,
    ) -> Result<(), RepoError> {
        record.validate().map_err(RepoError::Config)?;
        if &record.plan_id != plan_id {
            return Err(RepoError::InvalidUpdate);
        }
        let _guard = self.lock()?;
        let plan = self.load_unlocked(plan_id)?;
        if plan.status == PlanStatus::Closed {
            return Err(RepoError::InvalidUpdate);
        }
        let observations = self.list_observations(plan_id)?;
        if !observations.iter().any(|o| o.id() == &record.superseded)
            || !observations.iter().any(|o| o.id() == &record.replacement)
        {
            return Err(RepoError::InvalidUpdate);
        }
        let dir = self.supersessions_dir(plan_id);
        ensure_dir(&dir)?;
        let path = dir.join(format!("{}.json", record.id));
        if path.exists() {
            return Err(RepoError::InvalidUpdate);
        }
        let mut records = self.list_supersessions_unlocked(plan_id)?;
        records.push(record.clone());
        effective_observations(&observations, &records).map_err(RepoError::Config)?;
        atomic_write(&path, &serde_json::to_vec(record)?)?;
        Ok(())
    }

    pub fn list_supersessions(
        &self,
        plan_id: &PlanId,
    ) -> Result<Vec<EvidenceSupersessionRecord>, RepoError> {
        self.load_unlocked(plan_id)?;
        self.list_supersessions_unlocked(plan_id)
    }

    fn list_supersessions_unlocked(
        &self,
        plan_id: &PlanId,
    ) -> Result<Vec<EvidenceSupersessionRecord>, RepoError> {
        let dir = self.supersessions_dir(plan_id);
        match fs::symlink_metadata(&dir) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(RepoError::Io(e)),
            Ok(m) if m.file_type().is_symlink() || !m.is_dir() => {
                return Err(RepoError::UnsafePath(dir));
            }
            Ok(_) => {}
        }
        let mut records = Vec::new();
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            if entry.file_type()?.is_symlink() || !entry.file_type()?.is_file() {
                return Err(RepoError::UnsafePath(entry.path()));
            }
            let record: EvidenceSupersessionRecord =
                serde_json::from_slice(&fs::read(entry.path())?).map_err(|e| {
                    RepoError::Corrupt {
                        path: entry.path(),
                        reason: e.to_string(),
                    }
                })?;
            record.validate().map_err(|reason| RepoError::Corrupt {
                path: entry.path(),
                reason,
            })?;
            if record.plan_id != *plan_id
                || entry.file_name().to_string_lossy() != format!("{}.json", record.id)
            {
                return Err(RepoError::Corrupt {
                    path: entry.path(),
                    reason: "supersession identity mismatch".into(),
                });
            }
            records.push(record);
        }
        records.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(records)
    }

    pub fn open(root: impl AsRef<Path>) -> Result<Self, RepoError> {
        Self::open_with_options(root, StoreOptions::default())
    }

    pub fn open_with_options(
        root: impl AsRef<Path>,
        options: StoreOptions,
    ) -> Result<Self, RepoError> {
        let root = root.as_ref().to_path_buf();
        ensure_dir(&root)?;
        ensure_dir(&root.join("plans"))?;
        let _init_lock = acquire_lock(&root, options.lock_timeout)?;
        let config_path = root.join("config.toml");
        let config = if config_path.exists() {
            let meta = fs::symlink_metadata(&config_path)?;
            if meta.file_type().is_symlink() || !meta.is_file() {
                return Err(RepoError::UnsafePath(config_path));
            }
            let mut input = String::new();
            File::open(&config_path)?.read_to_string(&mut input)?;
            let config: RepositoryConfig =
                toml::from_str(&input).map_err(|e| RepoError::Config(e.to_string()))?;
            validate_repository_config(&config)?;
            config
        } else {
            let config = RepositoryConfig {
                schema_version: CONFIG_VERSION,
                repository_id: format!("epr_{}", Uuid::new_v4().simple()),
            };
            let bytes = toml::to_string(&config).map_err(|e| RepoError::Config(e.to_string()))?;
            atomic_write(&config_path, bytes.as_bytes())?;
            config
        };
        drop(_init_lock);
        let store = Self {
            root,
            options,
            repository_id: config.repository_id,
        };
        store.recover_pending_closures()?;
        store.list()?;
        Ok(store)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn repository_id(&self) -> &str {
        &self.repository_id
    }
    pub fn subject_source(&self) -> GitSubjectSource {
        GitSubjectSource::new(&self.root, &self.repository_id).excluding_path(&self.root)
    }

    pub fn abandoned_staging_files(&self) -> Result<Vec<PathBuf>, RepoError> {
        check_dir(&self.root)?;
        check_dir(&self.root.join("plans"))?;
        let mut found = Vec::new();
        for entry in fs::read_dir(self.root.join("plans"))? {
            let entry = entry?;
            let kind = entry.file_type()?;
            if kind.is_symlink() {
                return Err(RepoError::UnsafePath(entry.path()));
            }
            if !kind.is_dir() {
                continue;
            }
            for child in fs::read_dir(entry.path())? {
                let child = child?;
                if child.file_name().to_string_lossy().starts_with(".tmp-") {
                    found.push(child.path());
                }
            }
        }
        found.sort();
        Ok(found)
    }

    fn plan_dir(&self, id: &PlanId) -> PathBuf {
        self.root.join("plans").join(id.as_str())
    }
    fn plan_path(&self, id: &PlanId) -> PathBuf {
        self.plan_dir(id).join(PLAN_FILE)
    }
    fn evidence_dir(&self, id: &PlanId) -> PathBuf {
        self.plan_dir(id).join(EVIDENCE_DIR)
    }
    fn supersessions_dir(&self, id: &PlanId) -> PathBuf {
        self.plan_dir(id).join(SUPERSESSIONS_DIR)
    }
    fn observation_path(
        &self,
        plan_id: &PlanId,
        observation_id: &EvidenceObservationId,
    ) -> PathBuf {
        self.evidence_dir(plan_id)
            .join(format!("{observation_id}.json"))
    }

    fn lock(&self) -> Result<LockGuard, RepoError> {
        acquire_lock(&self.root, self.options.lock_timeout)
    }

    fn load_unlocked(&self, id: &PlanId) -> Result<Plan, RepoError> {
        check_dir(&self.root)?;
        check_dir(&self.root.join("plans"))?;
        let dir = self.plan_dir(id);
        if !dir.exists() {
            return Err(RepoError::NotFound(id.clone()));
        }
        check_dir(&dir)?;
        let path = self.plan_path(id);
        let metadata = fs::symlink_metadata(&path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                RepoError::NotFound(id.clone())
            } else {
                RepoError::Io(e)
            }
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(RepoError::UnsafePath(path));
        }
        if metadata.len() > MAX_PLAN_FILE_BYTES {
            return Err(RepoError::Corrupt {
                path,
                reason: "plan file exceeds 16 MiB storage limit".into(),
            });
        }
        let bytes = fs::read(&path)?;
        let stored: StoredPlan =
            serde_json::from_slice(&bytes).map_err(|e| RepoError::Corrupt {
                path: path.clone(),
                reason: e.to_string(),
            })?;
        if stored.storage_version != 1 {
            return Err(RepoError::Corrupt {
                path,
                reason: format!("unknown storage schema {}", stored.storage_version),
            });
        }
        let plan = stored.plan;
        plan.validate().map_err(|e| RepoError::InvalidPlan {
            path: path.clone(),
            reason: e.to_string(),
        })?;
        let digest = digest_json(&plan).map_err(|e| RepoError::Corrupt {
            path: path.clone(),
            reason: e.to_string(),
        })?;
        if digest != stored.plan_digest {
            return Err(RepoError::Corrupt {
                path,
                reason: "plan content digest mismatch".into(),
            });
        }
        if &plan.id != id {
            return Err(RepoError::Corrupt {
                path,
                reason: "stored plan ID differs from directory ID".into(),
            });
        }
        let closure_path = dir.join("closure.json");
        let pending_path = dir.join("closure.pending.json");
        if pending_path.exists() {
            return Err(RepoError::RecoveryRequired(id.clone()));
        }
        match (
            plan.status == PlanStatus::Closed,
            fs::symlink_metadata(&closure_path),
        ) {
            (true, Ok(meta)) if !meta.file_type().is_symlink() && meta.is_file() => {
                let record: ClosureRecord = serde_json::from_slice(&fs::read(&closure_path)?)
                    .map_err(|e| RepoError::Corrupt {
                        path: closure_path.clone(),
                        reason: e.to_string(),
                    })?;
                record
                    .validate(&plan)
                    .map_err(|reason| RepoError::Corrupt {
                        path: closure_path.clone(),
                        reason,
                    })?;
                let observations = self.list_observations_unlocked(id)?;
                let supersessions = self.list_supersessions_unlocked(id)?;
                effective_observations(&observations, &supersessions).map_err(|reason| {
                    RepoError::Corrupt {
                        path: closure_path.clone(),
                        reason,
                    }
                })?;
                for (observation_id, digest) in &record.candidate.satisfying_observations {
                    if !observations.iter().any(|observation| {
                        observation.id() == observation_id && observation.content_digest() == digest
                    }) {
                        return Err(RepoError::Corrupt {
                            path: closure_path.clone(),
                            reason: format!(
                                "closure observation {observation_id} is missing or changed"
                            ),
                        });
                    }
                }
                let lineage: Vec<_> = supersessions
                    .iter()
                    .map(|record| (record.id.clone(), record.content_digest.clone()))
                    .collect();
                if lineage != record.candidate.supersession_digests {
                    return Err(RepoError::Corrupt {
                        path: closure_path.clone(),
                        reason: "closure supersession lineage is missing or changed".into(),
                    });
                }
                let mut source_plan = plan.clone();
                source_plan.revision = record.candidate.source_revision;
                source_plan.status = PlanStatus::Active;
                if digest_json(&source_plan)? != record.candidate.source_plan_digest {
                    return Err(RepoError::Corrupt {
                        path: closure_path.clone(),
                        reason: "closure source Plan digest cannot be reproduced".into(),
                    });
                }
                let mut providers = ProviderRegistry::default();
                for entry in &record.candidate.provider_policy {
                    providers
                        .register_trusted(ProviderDescriptor::new(
                            entry.provider_id.clone(),
                            entry.class.clone(),
                            entry.allowed_kinds.iter().copied(),
                        )?)
                        .map_err(|error| RepoError::Corrupt {
                            path: closure_path.clone(),
                            reason: error.to_string(),
                        })?;
                }
                let effective =
                    effective_observations(&observations, &supersessions).map_err(|reason| {
                        RepoError::Corrupt {
                            path: closure_path.clone(),
                            reason,
                        }
                    })?;
                let effective: Vec<_> = effective.into_iter().cloned().collect();
                let assessment = assess_plan(
                    &source_plan,
                    &record.candidate.subject,
                    &effective,
                    &providers,
                );
                if assessment.status != AssessmentStatus::Complete
                    || assessment != record.candidate.assessment
                {
                    return Err(RepoError::Corrupt {
                        path: closure_path.clone(),
                        reason: "closure assessment cannot be reproduced".into(),
                    });
                }
            }
            (true, _) => {
                return Err(RepoError::Corrupt {
                    path: closure_path,
                    reason: "closed plan has no valid closure record".into(),
                });
            }
            (false, Ok(_)) => {
                return Err(RepoError::Corrupt {
                    path: closure_path,
                    reason: "closure record exists for non-closed plan".into(),
                });
            }
            (false, Err(error)) if error.kind() == std::io::ErrorKind::NotFound => {}
            (false, Err(error)) => return Err(RepoError::Io(error)),
        }
        Ok(plan)
    }
}

impl PlanStore for RepositoryStore {
    fn create(&self, plan: &Plan) -> Result<(), RepoError> {
        plan.validate()?;
        let _guard = self.lock()?;
        check_dir(&self.root)?;
        check_dir(&self.root.join("plans"))?;
        let dir = self.plan_dir(&plan.id);
        if dir.exists() {
            return Err(RepoError::AlreadyExists(plan.id.clone()));
        }
        if plan.revision != 0 || !matches!(plan.status, eggplan_core::PlanStatus::Draft) {
            return Err(RepoError::InvalidUpdate);
        }
        ensure_dir(&dir)?;
        let bytes = encode_plan(plan)?;
        if bytes.len() as u64 > MAX_PLAN_FILE_BYTES {
            let _ = fs::remove_dir(&dir);
            return Err(RepoError::Config(
                "plan exceeds 16 MiB storage limit".into(),
            ));
        }
        if let Err(error) = atomic_write(&self.plan_path(&plan.id), &bytes) {
            if !matches!(error, RepoError::DurabilityUnknown(_)) {
                let _ = fs::remove_dir(&dir);
            }
            return Err(error);
        }
        Ok(())
    }

    fn get(&self, id: &PlanId) -> Result<Plan, RepoError> {
        self.load_unlocked(id)
    }

    fn list(&self) -> Result<Vec<PlanId>, RepoError> {
        check_dir(&self.root)?;
        check_dir(&self.root.join("plans"))?;
        let mut ids = Vec::new();
        for entry in fs::read_dir(self.root.join("plans"))? {
            let entry = entry?;
            let kind = entry.file_type()?;
            if kind.is_symlink() {
                return Err(RepoError::UnsafePath(entry.path()));
            }
            if !kind.is_dir() {
                continue;
            }
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| RepoError::Config("non-Unicode plan directory name".into()))?;
            let id = PlanId::new(name).map_err(|e| RepoError::Config(e.to_string()))?;
            self.load_unlocked(&id)?;
            ids.push(id);
        }
        ids.sort();
        Ok(ids)
    }

    fn compare_and_swap(
        &self,
        id: &PlanId,
        expected_revision: u64,
        next: &Plan,
    ) -> Result<Plan, RepoError> {
        if &next.id != id || expected_revision.checked_add(1) != Some(next.revision) {
            return Err(RepoError::InvalidUpdate);
        }
        next.validate()?;
        let _guard = self.lock()?;
        let current = self.load_unlocked(id)?;
        if current.revision != expected_revision {
            return Err(RepoError::Conflict {
                id: id.clone(),
                expected: expected_revision,
                current: current.revision,
            });
        }
        if next.status == eggplan_core::PlanStatus::Closed
            && current.status != eggplan_core::PlanStatus::Closed
        {
            return Err(RepoError::GuardedClosureRequired);
        }
        if next.status != current.status
            && !eggplan_core::plan_transition_allowed(&current.status, &next.status)
        {
            return Err(RepoError::InvalidTransition);
        }
        for old_item in &current.items {
            if let Some(new_item) = next
                .items
                .iter()
                .find(|candidate| candidate.id == old_item.id)
                && new_item.status != old_item.status
                && !eggplan_core::item_transition_allowed(&old_item.status, &new_item.status)
            {
                return Err(RepoError::InvalidTransition);
            }
        }
        let bytes = encode_plan(next)?;
        if bytes.len() as u64 > MAX_PLAN_FILE_BYTES {
            return Err(RepoError::Config(
                "plan exceeds 16 MiB storage limit".into(),
            ));
        }
        atomic_write(&self.plan_path(id), &bytes)?;
        self.load_unlocked(id)
    }

    fn append_observation(
        &self,
        plan_id: &PlanId,
        observation: &EvidenceObservation,
    ) -> Result<(), RepoError> {
        observation.validate()?;
        let _guard = self.lock()?;
        self.load_unlocked(plan_id)?;
        let dir = self.evidence_dir(plan_id);
        ensure_dir(&dir)?;
        if self.list_observations(plan_id)?.len() >= eggplan_core::bounds::MAX_OBSERVATIONS_PER_PLAN
        {
            return Err(RepoError::ObservationLimit);
        }
        let path = self.observation_path(plan_id, observation.id());
        match fs::symlink_metadata(&path) {
            Ok(meta) if meta.file_type().is_symlink() || !meta.is_file() => {
                return Err(RepoError::UnsafePath(path));
            }
            Ok(_) => {
                let existing = load_observation(&path)?;
                let incoming_bytes = observation.canonical_json()?;
                let existing_bytes = existing.canonical_json()?;
                return if incoming_bytes == existing_bytes {
                    Ok(())
                } else {
                    Err(RepoError::ObservationConflict(observation.id().clone()))
                };
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(RepoError::Io(error)),
        }
        let bytes = observation.canonical_json()?;
        atomic_write(&path, &bytes)?;
        let stored = load_observation(&path)?;
        if stored != *observation {
            return Err(RepoError::ObservationConflict(observation.id().clone()));
        }
        Ok(())
    }

    fn get_observation(
        &self,
        plan_id: &PlanId,
        observation_id: &EvidenceObservationId,
    ) -> Result<EvidenceObservation, RepoError> {
        self.load_unlocked(plan_id)?;
        let ledger = self.evidence_dir(plan_id);
        match fs::symlink_metadata(&ledger) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(RepoError::ObservationNotFound(observation_id.clone()));
            }
            Err(error) => return Err(RepoError::Io(error)),
            Ok(meta) if meta.file_type().is_symlink() || !meta.is_dir() => {
                return Err(RepoError::UnsafePath(ledger));
            }
            Ok(_) => {}
        }
        let path = self.observation_path(plan_id, observation_id);
        let meta = fs::symlink_metadata(&path).map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                RepoError::ObservationNotFound(observation_id.clone())
            } else {
                RepoError::Io(error)
            }
        })?;
        if meta.file_type().is_symlink() || !meta.is_file() {
            return Err(RepoError::UnsafePath(path));
        }
        load_observation(&path)
    }

    fn list_observations(&self, plan_id: &PlanId) -> Result<Vec<EvidenceObservation>, RepoError> {
        self.load_unlocked(plan_id)?;
        self.list_observations_unlocked(plan_id)
    }
}

impl RepositoryStore {
    fn list_observations_unlocked(
        &self,
        plan_id: &PlanId,
    ) -> Result<Vec<EvidenceObservation>, RepoError> {
        let dir = self.evidence_dir(plan_id);
        match fs::symlink_metadata(&dir) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(RepoError::Io(error)),
            Ok(meta) if meta.file_type().is_symlink() || !meta.is_dir() => {
                return Err(RepoError::UnsafePath(dir));
            }
            Ok(_) => {}
        }
        let mut observations = Vec::new();
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| RepoError::Config("non-Unicode observation filename".into()))?;
            if name.starts_with(".tmp-") {
                continue;
            }
            if !name.ends_with(".json") {
                return Err(RepoError::Config(format!(
                    "unexpected evidence ledger entry {name}"
                )));
            }
            let file_type = entry.file_type()?;
            if file_type.is_symlink() || !file_type.is_file() {
                return Err(RepoError::UnsafePath(entry.path()));
            }
            let observation = load_observation(&entry.path())?;
            let expected = format!("{}.json", observation.id());
            if name != expected {
                return Err(RepoError::Corrupt {
                    path: entry.path(),
                    reason: "observation ID differs from ledger filename".into(),
                });
            }
            if observations.len() >= eggplan_core::bounds::MAX_OBSERVATIONS_PER_PLAN {
                return Err(RepoError::ObservationLimit);
            }
            observations.push(observation);
        }
        observations.sort_by(|a, b| a.id().cmp(b.id()));
        Ok(observations)
    }
}

struct LockGuard(File);
impl Drop for LockGuard {
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}

fn acquire_lock(root: &Path, timeout: Duration) -> Result<LockGuard, RepoError> {
    let path = root.join(".lock");
    if let Ok(meta) = fs::symlink_metadata(&path)
        && (meta.file_type().is_symlink() || !meta.is_file())
    {
        return Err(RepoError::UnsafePath(path));
    }
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)?;
    let start = Instant::now();
    loop {
        match file.try_lock_exclusive() {
            Ok(()) => return Ok(LockGuard(file)),
            Err(error) if is_lock_contention(&error) => {
                if start.elapsed() >= timeout {
                    return Err(RepoError::LockTimeout);
                }
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) => return Err(RepoError::Io(error)),
        }
    }
}

fn validate_repository_config(config: &RepositoryConfig) -> Result<(), RepoError> {
    if config.schema_version != CONFIG_VERSION
        || !config.repository_id.starts_with("epr_")
        || config.repository_id.len() == 4
        || config.repository_id.len() > eggplan_core::bounds::ID_CHARS
        || !config.repository_id[4..]
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
    {
        return Err(RepoError::Config(
            "unknown schema version or empty repository_id".into(),
        ));
    }
    Ok(())
}

fn is_lock_contention(error: &std::io::Error) -> bool {
    if error.kind() == std::io::ErrorKind::WouldBlock {
        return true;
    }
    #[cfg(windows)]
    {
        // LockFileEx reports ERROR_LOCK_VIOLATION (33) for a conflicting
        // exclusive range lock; std does not consistently map it to
        // WouldBlock across Rust/Windows versions.
        error.raw_os_error() == Some(33)
    }
    #[cfg(not(windows))]
    false
}

fn encode_plan(plan: &Plan) -> Result<Vec<u8>, RepoError> {
    let stored = StoredPlan {
        storage_version: 1,
        plan_digest: digest_json(plan)?,
        plan: plan.clone(),
    };
    Ok(serde_json::to_vec(&stored)?)
}

fn load_observation(path: &Path) -> Result<EvidenceObservation, RepoError> {
    let meta = fs::symlink_metadata(path)?;
    if meta.file_type().is_symlink() || !meta.is_file() {
        return Err(RepoError::UnsafePath(path.to_path_buf()));
    }
    if meta.len() > MAX_OBSERVATION_FILE_BYTES {
        return Err(RepoError::Corrupt {
            path: path.to_path_buf(),
            reason: "observation file exceeds 1 MiB storage limit".into(),
        });
    }
    let bytes = fs::read(path)?;
    EvidenceObservation::parse(&bytes).map_err(|error| RepoError::Corrupt {
        path: path.to_path_buf(),
        reason: error.to_string(),
    })
}

fn ensure_dir(path: &Path) -> Result<(), RepoError> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() || !meta.is_dir() => {
            Err(RepoError::UnsafePath(path.to_path_buf()))
        }
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => match fs::create_dir(path) {
            Ok(()) => check_dir(path),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => check_dir(path),
            Err(error) => Err(RepoError::Io(error)),
        },
        Err(e) => Err(RepoError::Io(e)),
    }
}
fn check_dir(path: &Path) -> Result<(), RepoError> {
    let meta = fs::symlink_metadata(path)?;
    if meta.file_type().is_symlink() || !meta.is_dir() {
        Err(RepoError::UnsafePath(path.to_path_buf()))
    } else {
        Ok(())
    }
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), RepoError> {
    let parent = path
        .parent()
        .ok_or_else(|| RepoError::UnsafePath(path.to_path_buf()))?;
    check_dir(parent)?;
    let mut temp: NamedTempFile = Builder::new().prefix(".tmp-").tempfile_in(parent)?;
    temp.write_all(bytes)?;
    temp.as_file().sync_all()?;
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() || !meta.is_file() => {
            return Err(RepoError::UnsafePath(path.to_path_buf()));
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(RepoError::Io(error)),
    }
    temp.persist(path).map_err(|e| RepoError::Io(e.error))?;
    #[cfg(unix)]
    File::open(parent)?
        .sync_all()
        .map_err(RepoError::DurabilityUnknown)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    //! Crate-internal seam tests for `finalize_closure` S1/S2 recapture.
    //!
    //! The `SubjectCapture` seam and the `finalize_closure_with_capture` hook
    //! are intentionally not part of the public API. These tests live inside
    //! the crate so they can use deterministic scripted capture sequences to
    //! prove stale / drift / capture-failure behaviour while remaining invisible
    /// to downstream crates.
    use super::*;
    use crate::GitSubjectError;
    use eggplan_core::{
        AcceptanceCriterion, AssessmentStatus, ClosureCandidate, ClosureId, CriterionId,
        EvidenceCardinality, EvidenceKind, EvidenceObservation, EvidenceObservationId,
        EvidenceObservationInput, EvidenceProviderId, EvidenceRequirement, EvidenceStatus, Plan,
        PlanId, PlanItem, PlanItemId, PlanItemStatus, PlanStatus, ProviderDescriptor,
        ProviderPolicyEntry, ProviderRegistry, SubjectRevision, SubjectState, VerificationDigest,
        assess_plan, effective_observations,
    };
    use git2::{Repository, Signature};
    use std::collections::VecDeque;
    use std::sync::Mutex;
    use tempfile::tempdir;

    /// Deterministic test double for the `SubjectCapture` seam. Each call to
    /// `capture` returns the next scripted result, panicking when the
    /// sequence is exhausted. Constructed only from crate-internal tests.
    pub(crate) struct ScriptedSubjectCapture {
        results: Mutex<VecDeque<Result<SubjectRevision, GitSubjectError>>>,
    }

    impl ScriptedSubjectCapture {
        pub(crate) fn new(results: Vec<Result<SubjectRevision, GitSubjectError>>) -> Self {
            Self {
                results: Mutex::new(results.into_iter().collect()),
            }
        }
    }

    impl SubjectCapture for ScriptedSubjectCapture {
        fn capture(&self) -> Result<SubjectRevision, GitSubjectError> {
            self.results
                .lock()
                .expect("scripted capture poisoned")
                .pop_front()
                .expect("scripted capture sequence exhausted")
        }
    }

    fn init_git_repo(root: &Path) -> Repository {
        let repo = Repository::init(root).unwrap();
        fs::write(root.join("tracked.txt"), b"base\n").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("tracked.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = Signature::now("Eggplan Test", "eggplan@example.invalid").unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
            .unwrap();
        drop(tree);
        repo
    }

    fn close_ready_store(root: &Path) -> (RepositoryStore, Plan, SubjectRevision) {
        let store = RepositoryStore::open(root).unwrap();
        let subject = store.subject_source().capture().unwrap();
        let binding = VerificationDigest::new(format!("sha256:{}", "c".repeat(64))).unwrap();
        let mut plan = Plan::new(
            PlanId::new("ep_revalidate").unwrap(),
            "revalidate closure subject",
            vec![PlanItem {
                id: PlanItemId::new("epi_revalidate").unwrap(),
                position: 0,
                parent: None,
                dependencies: vec![],
                status: PlanItemStatus::Completed,
                description: "revalidate closure subject".into(),
                criteria: vec![AcceptanceCriterion {
                    id: CriterionId::new("epc_revalidate").unwrap(),
                    statement: "designated test passed".into(),
                    human_judgment_allowed: false,
                    requirements: vec![EvidenceRequirement {
                        description: "designated test invocation".into(),
                        kind: EvidenceKind::Test,
                        provider: None,
                        subject_policy: eggplan_core::SubjectPolicy::Exact,
                        cardinality: EvidenceCardinality::Any,
                        min_count: 1,
                        allow_human_judgment: false,
                        expected_verification_digest: Some(binding.clone()),
                    }],
                }],
                blocker: None,
                next_action: None,
            }],
        )
        .unwrap();
        plan.subject = Some(subject.clone());
        store.create(&plan).unwrap();
        plan.revision = 1;
        plan.status = PlanStatus::Active;
        let plan = store.compare_and_swap(&plan.id, 0, &plan).unwrap();
        let observation = EvidenceObservation::finalize(EvidenceObservationInput {
            id: EvidenceObservationId::new("epe_revalidate_pass").unwrap(),
            provider_id: EvidenceProviderId::new("epp_test").unwrap(),
            kind: EvidenceKind::Test,
            status: EvidenceStatus::Passed,
            subject: subject.clone(),
            observed_at_unix_ms: 11,
            invocation_ref: Some("cargo test".into()),
            verification_digest: Some(binding),
            result_metadata: Default::default(),
            artifacts: vec![],
        })
        .unwrap();
        store.append_observation(&plan.id, &observation).unwrap();
        (store, plan, subject)
    }

    fn build_complete_candidate(
        store: &RepositoryStore,
        plan: &Plan,
        subject: &SubjectRevision,
    ) -> ClosureCandidate {
        let policy = vec![ProviderPolicyEntry {
            provider_id: EvidenceProviderId::new("epp_test").unwrap(),
            class: "host".into(),
            allowed_kinds: [EvidenceKind::Test].into_iter().collect(),
        }];
        let observations = store.list_observations(&plan.id).unwrap();
        let supersessions = store.list_supersessions(&plan.id).unwrap();
        let effective: Vec<_> = effective_observations(&observations, &supersessions)
            .unwrap()
            .into_iter()
            .cloned()
            .collect();
        let mut providers = ProviderRegistry::default();
        providers
            .register_trusted(
                ProviderDescriptor::new(
                    EvidenceProviderId::new("epp_test").unwrap(),
                    String::from("host"),
                    [EvidenceKind::Test],
                )
                .unwrap(),
            )
            .unwrap();
        let assessment = assess_plan(plan, subject, &effective, &providers);
        assert_eq!(assessment.status, AssessmentStatus::Complete);
        ClosureCandidate::build(
            plan,
            subject.clone(),
            assessment,
            &observations,
            &supersessions,
            policy,
            12,
        )
        .unwrap()
    }

    fn assert_no_partial_closure_state(store: &RepositoryStore, plan_id: &PlanId) {
        assert_eq!(store.get(plan_id).unwrap().status, PlanStatus::Active);
        let plan_dir = store.root().join("plans").join(plan_id.as_str());
        assert!(!plan_dir.join("closure.json").exists());
        assert!(!plan_dir.join("closure.pending.json").exists());
    }

    #[test]
    fn closure_subject_stable_between_captures_succeeds() {
        let dir = tempdir().unwrap();
        let _repo = init_git_repo(dir.path());
        let (store, plan, subject) = close_ready_store(&dir.path().join(".eggplan"));
        let candidate = build_complete_candidate(&store, &plan, &subject);
        let capture: Box<dyn SubjectCapture> = Box::new(ScriptedSubjectCapture::new(vec![
            Ok(subject.clone()),
            Ok(subject.clone()),
        ]));
        let (closed, record) = store
            .finalize_closure_with_capture(&candidate, ClosureId::generate(), 13, capture.as_ref())
            .unwrap();
        assert_eq!(closed.status, PlanStatus::Closed);
        assert_eq!(closed.revision, plan.revision + 1);
        record.validate(&closed).unwrap();
        assert_eq!(record.candidate.subject, subject);
    }

    #[test]
    fn closure_subject_drift_between_captures_aborts_with_no_partial_state() {
        let dir = tempdir().unwrap();
        let _repo = init_git_repo(dir.path());
        let (store, plan, subject) = close_ready_store(&dir.path().join(".eggplan"));
        let candidate = build_complete_candidate(&store, &plan, &subject);
        let drifted = SubjectRevision {
            subject_kind: subject.subject_kind.clone(),
            repository_id: subject.repository_id.clone(),
            revision: subject.revision.clone(),
            state: SubjectState::Dirty,
            dirty_digest: Some(format!("sha256:{}", "d".repeat(64))),
        };
        let capture: Box<dyn SubjectCapture> = Box::new(ScriptedSubjectCapture::new(vec![
            Ok(subject.clone()),
            Ok(drifted),
        ]));
        let result = store.finalize_closure_with_capture(
            &candidate,
            ClosureId::generate(),
            13,
            capture.as_ref(),
        );
        assert!(matches!(result, Err(RepoError::ClosureSubjectDrift)));
        assert_eq!(store.get(&plan.id).unwrap().revision, plan.revision);
        assert_no_partial_closure_state(&store, &plan.id);
    }

    #[test]
    fn closure_subject_capture_failure_during_finalizer_aborts_with_no_partial_state() {
        let dir = tempdir().unwrap();
        let _repo = init_git_repo(dir.path());
        let (store, plan, subject) = close_ready_store(&dir.path().join(".eggplan"));
        let candidate = build_complete_candidate(&store, &plan, &subject);
        let capture: Box<dyn SubjectCapture> = Box::new(ScriptedSubjectCapture::new(vec![
            Ok(subject.clone()),
            Err(GitSubjectError::Unborn),
        ]));
        let result = store.finalize_closure_with_capture(
            &candidate,
            ClosureId::generate(),
            13,
            capture.as_ref(),
        );
        assert!(matches!(result, Err(RepoError::ClosureSubjectCapture(_))));
        assert_eq!(store.get(&plan.id).unwrap().revision, plan.revision);
        assert_no_partial_closure_state(&store, &plan.id);
    }

    #[test]
    fn finalize_closure_does_not_expose_capture_injection_to_external_callers() {
        let dir = tempdir().unwrap();
        let _repo = init_git_repo(dir.path());
        let store = RepositoryStore::open(dir.path().join(".eggplan")).unwrap();
        // The crate-internal hook must remain reachable from the crate's own
        // tests so the deterministic regressions can run. The compiler enforces
        // the boundary by giving the hook `pub(crate)` visibility only.
        let _ = ScriptedSubjectCapture::new(Vec::new());
        let _capture: &dyn SubjectCapture = &GitSubjectCapture::new(&store.subject_source());
        let _: fn(&RepositoryStore, &ClosureCandidate, ClosureId, u64, &dyn SubjectCapture) -> _ =
            RepositoryStore::finalize_closure_with_capture;
    }
}
