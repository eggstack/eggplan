use crate::GitSubjectSource;
use eggplan_core::{Plan, PlanId, ValidationError, digest_json};
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
    #[error("cooperative repository lock timed out")]
    LockTimeout,
    #[error("replacement succeeded but directory durability could not be confirmed: {0}")]
    DurabilityUnknown(#[source] std::io::Error),
    #[error("symlink or non-directory found in managed state path {0}")]
    UnsafePath(PathBuf),
    #[error("unknown or corrupt canonical plan at {path}: {reason}")]
    Corrupt { path: PathBuf, reason: String },
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
}

#[derive(Debug, Clone)]
pub struct RepositoryStore {
    root: PathBuf,
    options: StoreOptions,
    repository_id: String,
}

impl RepositoryStore {
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
            if config.schema_version != CONFIG_VERSION
                || !config.repository_id.starts_with("epr_")
                || config.repository_id.len() == 4
                || config.repository_id.len() > eggplan_core::bounds::ID_CHARS
                || !config.repository_id[4..]
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
            {
                return Err(RepoError::Config(
                    "unknown schema version or empty repository_id".into(),
                ));
            }
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
        Ok(Self {
            root,
            options,
            repository_id: config.repository_id,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn repository_id(&self) -> &str {
        &self.repository_id
    }
    pub fn subject_source(&self) -> GitSubjectSource {
        GitSubjectSource::new(&self.root, &self.repository_id)
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
        plan.validate().map_err(|e| RepoError::Corrupt {
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
        atomic_write(&self.plan_path(id), &bytes)?;
        self.load_unlocked(id)
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
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                if start.elapsed() >= timeout {
                    return Err(RepoError::LockTimeout);
                }
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) => return Err(RepoError::Io(error)),
        }
    }
}

fn encode_plan(plan: &Plan) -> Result<Vec<u8>, RepoError> {
    let stored = StoredPlan {
        storage_version: 1,
        plan_digest: digest_json(plan)?,
        plan: plan.clone(),
    };
    Ok(serde_json::to_vec(&stored)?)
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
