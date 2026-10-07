use eggplan_core::{SubjectRevision, SubjectState};
use git2::{Repository, Status, StatusOptions};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct GitSubjectOptions {
    pub max_paths: usize,
    pub max_content_bytes: u64,
    pub max_submodule_depth: usize,
}
impl Default for GitSubjectOptions {
    fn default() -> Self {
        Self {
            max_paths: 10_000,
            max_content_bytes: 64 * 1024 * 1024,
            max_submodule_depth: 8,
        }
    }
}

#[derive(Debug, Error)]
pub enum GitSubjectError {
    #[error("path is not inside a Git worktree: {0}")]
    NotGit(PathBuf),
    #[error("Git operation failed: {0}")]
    Git(#[from] git2::Error),
    #[error("filesystem access failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("dirty state contains a non-Unicode path")]
    NonUnicodePath,
    #[error("dirty state exceeded configured path/content/depth bounds")]
    BoundExceeded,
    #[error("Git repository has no commit at HEAD")]
    Unborn,
    #[error("unsafe or symlinked path in Git status: {0}")]
    UnsafePath(PathBuf),
    #[error("invalid SubjectRevision: {0}")]
    InvalidSubject(String),
    #[error("excluded path does not resolve inside the discovered worktree: {0}")]
    InvalidExclusion(PathBuf),
    #[error("invalid GitSubjectFingerprintV1: {0}")]
    InvalidFingerprint(String),
}

/// Bounded, versioned, repository-ID-free capture of the exact HEAD revision,
/// clean/dirty state, and Eggplan-native dirty digest that
/// [`GitSubjectSource`] would produce for the same root, options, and
/// exclusion.
///
/// This is a compatibility fingerprint, not a universal Git digest standard:
/// the `dirty_digest` is Eggplan's own canonical dirty-manifest digest. An
/// external host may persist it so Eggplan exact-subject assessment can
/// later compare a historical capture against the repository's own capture.
///
/// The value deliberately carries no repository identity, path list, file
/// contents, index entries, symlink targets, or manifest bytes, and it grants
/// no evidence, provider, or closure authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GitSubjectFingerprintV1 {
    pub schema_version: u16,
    pub revision: String,
    pub state: SubjectState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dirty_digest: Option<String>,
}

impl GitSubjectFingerprintV1 {
    /// Version of the fingerprint contract itself.
    pub const SCHEMA_VERSION: u16 = 1;

    /// Reject a decoded or captured fingerprint that is not internally
    /// consistent: wrong schema version, blank revision, clean state with a
    /// dirty digest, or dirty state without a `sha256:<64 lowercase hex>`
    /// digest.
    pub fn validate(&self) -> Result<(), GitSubjectError> {
        if self.schema_version != Self::SCHEMA_VERSION {
            return Err(GitSubjectError::InvalidFingerprint(format!(
                "unsupported schema version: {}",
                self.schema_version
            )));
        }
        if self.revision.is_empty() {
            return Err(GitSubjectError::InvalidFingerprint(
                "revision must not be empty".into(),
            ));
        }
        match (self.state, self.dirty_digest.as_ref()) {
            (SubjectState::Clean, None) => Ok(()),
            (SubjectState::Dirty, Some(digest)) if valid_digest(digest) => Ok(()),
            _ => Err(GitSubjectError::InvalidFingerprint(
                "clean fingerprints omit the dirty digest; dirty fingerprints require sha256:<64 lowercase hex>"
                    .into(),
            )),
        }
    }
}

/// Capture the exact [`GitSubjectFingerprintV1`] Eggplan requires for
/// exact-subject dirty evidence, using the same discovery, manifest, ordering,
/// and bounds as [`GitSubjectSource::capture`].
///
/// `excluded_path` is the single administrative root (for example
/// `.eggplan`) excluded from the dirty identity and is resolved exactly as
/// `GitSubjectSource::excluding_path` resolves it. Unlike that historical
/// entry point, which silently applies no exclusion when the requested path
/// does not resolve inside the discovered worktree (or when it names the
/// worktree root itself), this function fails closed with
/// [`GitSubjectError::InvalidExclusion`]. For any exclusion that does resolve
/// inside the worktree, both entry points return byte-identical revision,
/// state, and dirty digest.
pub fn capture_git_subject_fingerprint(
    root: impl AsRef<Path>,
    options: GitSubjectOptions,
    excluded_path: Option<&Path>,
) -> Result<GitSubjectFingerprintV1, GitSubjectError> {
    let captured = capture_subject(
        root.as_ref(),
        &options,
        excluded_path,
        ExclusionMode::Strict,
    )?;
    let fingerprint = GitSubjectFingerprintV1 {
        schema_version: GitSubjectFingerprintV1::SCHEMA_VERSION,
        revision: captured.revision,
        state: captured.state,
        dirty_digest: captured.dirty_digest,
    };
    fingerprint.validate()?;
    Ok(fingerprint)
}

#[derive(Debug, Clone)]
pub struct GitSubjectSource {
    root: PathBuf,
    repository_id: String,
    options: GitSubjectOptions,
    excluded_root: Option<PathBuf>,
}

impl GitSubjectSource {
    pub fn new(root: impl AsRef<Path>, repository_id: impl Into<String>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            repository_id: repository_id.into(),
            options: GitSubjectOptions::default(),
            excluded_root: None,
        }
    }
    pub fn with_options(mut self, options: GitSubjectOptions) -> Self {
        self.options = options;
        self
    }

    /// Exclude one administrative directory, and all descendants, from the
    /// dirty worktree identity. The path must be lexically inside the worktree.
    pub fn excluding_path(mut self, path: impl AsRef<Path>) -> Self {
        self.excluded_root = Some(path.as_ref().to_path_buf());
        self
    }

    pub fn capture(&self) -> Result<SubjectRevision, GitSubjectError> {
        let captured = capture_subject(
            &self.root,
            &self.options,
            self.excluded_root.as_deref(),
            ExclusionMode::Lenient,
        )?;
        let subject = SubjectRevision {
            subject_kind: "git".into(),
            repository_id: self.repository_id.clone(),
            revision: captured.revision,
            state: captured.state,
            dirty_digest: captured.dirty_digest,
        };
        subject
            .validate()
            .map_err(|e| GitSubjectError::InvalidSubject(e.to_string()))?;
        Ok(subject)
    }
}

/// Repository-ID-free result of one worktree capture. Both the public
/// [`GitSubjectSource`] subject and [`capture_git_subject_fingerprint`] are
/// derived from this single value, so they cannot disagree for one capture.
#[derive(Debug, Clone, PartialEq, Eq)]
struct CapturedSubject {
    revision: String,
    state: SubjectState,
    dirty_digest: Option<String>,
}

/// How an administrative exclusion that does not resolve inside the
/// discovered worktree is treated. The mode never changes the dirty manifest
/// algorithm, row ordering, status bits, index-entry treatment, or bounds; it
/// only decides whether an unusable exclusion is ignored or rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExclusionMode {
    /// Historical `GitSubjectSource::capture` behavior: apply no exclusion.
    Lenient,
    /// Fingerprint contract behavior: fail closed.
    Strict,
}

fn capture_subject(
    root: &Path,
    options: &GitSubjectOptions,
    excluded_root: Option<&Path>,
    exclusion_mode: ExclusionMode,
) -> Result<CapturedSubject, GitSubjectError> {
    let repo =
        Repository::discover(root).map_err(|_| GitSubjectError::NotGit(root.to_path_buf()))?;
    let head = repo.head()?.target().ok_or(GitSubjectError::Unborn)?;
    let mut budget = Budget {
        paths: 0,
        bytes: 0,
        options,
    };
    let exclusion = resolve_exclusion(excluded_root, repo.workdir(), exclusion_mode)?;
    let manifest = dirty_manifest(&repo, &mut budget, 0, exclusion.as_deref())?;
    let dirty = !manifest.is_empty();
    Ok(CapturedSubject {
        revision: head.to_string(),
        state: if dirty {
            SubjectState::Dirty
        } else {
            SubjectState::Clean
        },
        dirty_digest: dirty.then(|| format!("sha256:{:x}", Sha256::digest(&manifest))),
    })
}

fn resolve_exclusion(
    excluded_root: Option<&Path>,
    workdir: Option<&Path>,
    mode: ExclusionMode,
) -> Result<Option<PathBuf>, GitSubjectError> {
    let Some(excluded_root) = excluded_root else {
        return Ok(None);
    };
    let Some(workdir) = workdir else {
        return match mode {
            ExclusionMode::Lenient => Ok(None),
            ExclusionMode::Strict => Err(GitSubjectError::InvalidExclusion(
                excluded_root.to_path_buf(),
            )),
        };
    };
    let absolute = if excluded_root.is_absolute() {
        excluded_root.to_path_buf()
    } else {
        std::env::current_dir()?.join(excluded_root)
    };
    // macOS commonly exposes /var as /private/var and Windows canonicalizes
    // path casing. Compare resolved existing roots so an administrative
    // exclusion cannot disappear due to an alias spelling.
    let absolute = fs::canonicalize(absolute)?;
    let workdir = fs::canonicalize(workdir)?;
    match absolute.strip_prefix(&workdir).ok().map(Path::to_path_buf) {
        // An exclusion that resolves to the worktree root itself would silently
        // swallow the entire dirty manifest. The fingerprint contract fails
        // closed instead of reporting a falsely clean subject.
        Some(relative) if relative.as_os_str().is_empty() => match mode {
            ExclusionMode::Lenient => Ok(None),
            ExclusionMode::Strict => Err(GitSubjectError::InvalidExclusion(absolute)),
        },
        Some(relative) => Ok(Some(relative)),
        None => match mode {
            ExclusionMode::Lenient => Ok(None),
            ExclusionMode::Strict => Err(GitSubjectError::InvalidExclusion(absolute)),
        },
    }
}

struct Budget<'a> {
    paths: usize,
    bytes: u64,
    options: &'a GitSubjectOptions,
}
impl Budget<'_> {
    fn add_path(&mut self) -> Result<(), GitSubjectError> {
        self.paths = self.paths.saturating_add(1);
        if self.paths > self.options.max_paths {
            Err(GitSubjectError::BoundExceeded)
        } else {
            Ok(())
        }
    }
    fn add_bytes(&mut self, amount: usize) -> Result<(), GitSubjectError> {
        self.bytes = self.bytes.saturating_add(amount as u64);
        if self.bytes > self.options.max_content_bytes {
            Err(GitSubjectError::BoundExceeded)
        } else {
            Ok(())
        }
    }
    /// Non-mutating counterpart of `add_bytes`, used to reject a known length
    /// before the read that would allocate it. The accounted total is still
    /// advanced by `add_bytes` against the bytes actually read, so a file that
    /// changes between the stat and the read is charged its real size.
    fn check_bytes(&self, amount: u64) -> Result<(), GitSubjectError> {
        if self.bytes.saturating_add(amount) > self.options.max_content_bytes {
            Err(GitSubjectError::BoundExceeded)
        } else {
            Ok(())
        }
    }
}

fn dirty_manifest(
    repo: &Repository,
    budget: &mut Budget<'_>,
    depth: usize,
    excluded_root: Option<&Path>,
) -> Result<Vec<u8>, GitSubjectError> {
    if depth > budget.options.max_submodule_depth {
        return Err(GitSubjectError::BoundExceeded);
    }
    let mut options = StatusOptions::new();
    options
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(false)
        .exclude_submodules(false);
    let statuses = repo.statuses(Some(&mut options))?;
    let mut rows = Vec::with_capacity(statuses.len());
    for entry in statuses.iter() {
        let status = entry.status();
        if status == Status::CURRENT {
            continue;
        }
        let path = entry.path().ok_or(GitSubjectError::NonUnicodePath)?;
        let relative = Path::new(path);
        if excluded_root
            .is_some_and(|excluded| relative == excluded || relative.starts_with(excluded))
        {
            continue;
        }
        budget.add_path()?;
        let workdir = repo
            .workdir()
            .ok_or_else(|| GitSubjectError::NotGit(repo.path().to_path_buf()))?;
        let work_path = safe_worktree_path(workdir, Path::new(path))?;
        let mut row = Vec::new();
        field(&mut row, path.as_bytes());
        row.extend_from_slice(&status.bits().to_le_bytes());

        if let Some(index_entry) = repo.index()?.get_path(Path::new(path), 0) {
            let oid = index_entry.id.to_string();
            field(&mut row, oid.as_bytes());
        } else {
            field(&mut row, b"no-index-entry");
        }

        match fs::symlink_metadata(&work_path) {
            Ok(meta) if meta.file_type().is_symlink() => {
                let target = fs::read_link(&work_path)?;
                let target = target.to_str().ok_or(GitSubjectError::NonUnicodePath)?;
                budget.add_bytes(target.len())?;
                field(&mut row, b"symlink");
                field(&mut row, target.as_bytes());
            }
            Ok(meta) if meta.is_file() => {
                // Reject an oversized worktree file from the length already
                // stat'd, before the read that would allocate it.
                budget.check_bytes(meta.len())?;
                let content = fs::read(&work_path)?;
                budget.add_bytes(content.len())?;
                field(&mut row, b"file");
                field(&mut row, &content);
                row.extend_from_slice(&meta.len().to_le_bytes());
            }
            Ok(meta) if meta.is_dir() => {
                field(&mut row, b"directory-or-submodule");
                if let Ok(subrepo) = Repository::open(&work_path) {
                    let subhead = subrepo
                        .head()?
                        .target()
                        .ok_or(GitSubjectError::Unborn)?
                        .to_string();
                    field(&mut row, subhead.as_bytes());
                    let nested = dirty_manifest(&subrepo, budget, depth + 1, None)?;
                    field(&mut row, &nested);
                }
            }
            Ok(_) => field(&mut row, b"other-file-type"),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                field(&mut row, b"deleted")
            }
            Err(error) => return Err(GitSubjectError::Io(error)),
        }
        rows.push((path.to_owned(), row));
    }
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    let mut manifest = Vec::new();
    for (_, row) in rows {
        field(&mut manifest, &row);
    }
    Ok(manifest)
}

fn safe_worktree_path(root: &Path, relative: &Path) -> Result<PathBuf, GitSubjectError> {
    use std::path::Component;
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(GitSubjectError::UnsafePath(relative.to_path_buf()));
    }
    let components: Vec<_> = relative.components().collect();
    let mut path = root.to_path_buf();
    for (index, component) in components.iter().enumerate() {
        path.push(component.as_os_str());
        if index + 1 < components.len() {
            match fs::symlink_metadata(&path) {
                Ok(meta) if meta.file_type().is_symlink() || !meta.is_dir() => {
                    return Err(GitSubjectError::UnsafePath(path));
                }
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
                Err(error) => return Err(GitSubjectError::Io(error)),
            }
        }
    }
    Ok(path)
}

fn field(output: &mut Vec<u8>, bytes: &[u8]) {
    output.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    output.extend_from_slice(bytes);
}

fn valid_digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64
            && hex
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn strict_exclusion_fails_closed_and_lenient_exclusion_is_unchanged() {
        let worktree = tempdir().unwrap();
        fs::create_dir(worktree.path().join(".eggplan")).unwrap();
        let administrative = worktree.path().join(".eggplan");
        let outside = tempdir().unwrap();
        let workdir = Some(worktree.path());

        for mode in [ExclusionMode::Strict, ExclusionMode::Lenient] {
            assert_eq!(
                resolve_exclusion(Some(administrative.as_path()), workdir, mode).unwrap(),
                Some(PathBuf::from(".eggplan")),
                "a resolvable administrative root is excluded identically"
            );
        }
        assert_eq!(
            resolve_exclusion(None, workdir, ExclusionMode::Strict).unwrap(),
            None
        );
        assert!(matches!(
            resolve_exclusion(Some(worktree.path()), workdir, ExclusionMode::Strict),
            Err(GitSubjectError::InvalidExclusion(_))
        ));
        assert!(matches!(
            resolve_exclusion(Some(outside.path()), workdir, ExclusionMode::Strict),
            Err(GitSubjectError::InvalidExclusion(_))
        ));
        assert_eq!(
            resolve_exclusion(Some(outside.path()), workdir, ExclusionMode::Lenient).unwrap(),
            None
        );
        assert!(matches!(
            resolve_exclusion(Some(administrative.as_path()), None, ExclusionMode::Strict),
            Err(GitSubjectError::InvalidExclusion(_))
        ));
        assert_eq!(
            resolve_exclusion(Some(administrative.as_path()), None, ExclusionMode::Lenient)
                .unwrap(),
            None
        );
    }

    #[test]
    fn fingerprint_validation_rejects_inconsistent_values() {
        let dirty = GitSubjectFingerprintV1 {
            schema_version: GitSubjectFingerprintV1::SCHEMA_VERSION,
            revision: "a".repeat(40),
            state: SubjectState::Dirty,
            dirty_digest: Some(format!("sha256:{}", "0".repeat(64))),
        };
        dirty.validate().unwrap();
        for invalid in [
            GitSubjectFingerprintV1 {
                schema_version: 0,
                ..dirty.clone()
            },
            GitSubjectFingerprintV1 {
                revision: String::new(),
                ..dirty.clone()
            },
            GitSubjectFingerprintV1 {
                state: SubjectState::Clean,
                ..dirty.clone()
            },
            GitSubjectFingerprintV1 {
                dirty_digest: None,
                ..dirty.clone()
            },
            GitSubjectFingerprintV1 {
                dirty_digest: Some(format!("sha256:{}", "0".repeat(63))),
                ..dirty.clone()
            },
            GitSubjectFingerprintV1 {
                dirty_digest: Some(format!("sha256:{}", "A".repeat(64))),
                ..dirty.clone()
            },
        ] {
            assert!(matches!(
                invalid.validate(),
                Err(GitSubjectError::InvalidFingerprint(_))
            ));
        }
    }

    #[test]
    fn git_status_paths_must_be_relative_and_normalized() {
        let root = tempdir().unwrap();
        assert!(matches!(
            safe_worktree_path(root.path(), Path::new("../outside")),
            Err(GitSubjectError::UnsafePath(_))
        ));
        assert!(matches!(
            safe_worktree_path(root.path(), Path::new("/outside")),
            Err(GitSubjectError::UnsafePath(_))
        ));
    }

    #[cfg(unix)]
    #[test]
    fn git_status_paths_cannot_descend_through_symlinks() {
        use std::os::unix::fs::symlink;
        let root = tempdir().unwrap();
        let outside = tempdir().unwrap();
        symlink(outside.path(), root.path().join("link")).unwrap();
        assert!(matches!(
            safe_worktree_path(root.path(), Path::new("link/file")),
            Err(GitSubjectError::UnsafePath(_))
        ));
    }
}
