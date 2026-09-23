use eggplan_core::{SubjectRevision, SubjectState};
use git2::{Repository, Status, StatusOptions};
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
        let repo = Repository::discover(&self.root)
            .map_err(|_| GitSubjectError::NotGit(self.root.clone()))?;
        let head = repo.head()?.target().ok_or(GitSubjectError::Unborn)?;
        let mut budget = Budget {
            paths: 0,
            bytes: 0,
            options: &self.options,
        };
        let exclusion = match (&self.excluded_root, repo.workdir()) {
            (Some(path), Some(workdir)) => {
                let absolute = if path.is_absolute() {
                    path.clone()
                } else {
                    std::env::current_dir()?.join(path)
                };
                absolute.strip_prefix(workdir).ok().map(Path::to_path_buf)
            }
            _ => None,
        };
        let manifest = dirty_manifest(&repo, &mut budget, 0, exclusion.as_deref())?;
        let dirty = !manifest.is_empty();
        let digest = dirty.then(|| format!("sha256:{:x}", Sha256::digest(&manifest)));
        let subject = SubjectRevision {
            subject_kind: "git".into(),
            repository_id: self.repository_id.clone(),
            revision: head.to_string(),
            state: if dirty {
                SubjectState::Dirty
            } else {
                SubjectState::Clean
            },
            dirty_digest: digest,
        };
        subject
            .validate()
            .map_err(|e| GitSubjectError::InvalidSubject(e.to_string()))?;
        Ok(subject)
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

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
