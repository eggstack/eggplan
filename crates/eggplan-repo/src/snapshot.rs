//! Bounded command-local repository inspection read model.
//!
//! Repository-wide CLI reads previously captured the Git subject once per Plan
//! and reopened the same canonical files repeatedly. Dirty subject capture can
//! enumerate 10,000 paths and hash 64 MiB, so multiplying that work by Plan
//! count was the dominant cost of a repository-wide read.
//!
//! [`InspectionSnapshot`] collapses that into one cooperatively locked,
//! revision-checked pass:
//!
//! 1. acquire the same lock root writers use, in shared mode;
//! 2. capture the Git subject as `S1`;
//! 3. enumerate and select Plan IDs deterministically;
//! 4. load and deep-validate each selected Plan once;
//! 5. recapture the Git subject as `S2` immediately before releasing the lock;
//! 6. fail with [`RepoError::SubjectDrift`] if `S1 != S2`.
//!
//! The snapshot is ephemeral process state. It is never serialized under
//! `.eggplan`, it is not a cache, and it is not canonical state.

use crate::{GitSubjectError, GitSubjectSource, RepoError};
use eggplan_core::{
    ClosureRecord, EvidenceObservation, EvidenceSupersessionRecord, Plan, PlanId, SubjectRevision,
};
use std::{
    fs::{self, File, OpenOptions},
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};

/// What a bounded inspection should load.
///
/// The two bounded forms exist because the read commands that consume a
/// snapshot do not have the same integrity obligations, and M003a must not
/// change either one's observable behavior:
///
/// - [`InspectionSelection::Bounded`] touches only the first `limit` plans.
///   This is exactly what repository-wide `status` and `registry render` did
///   before, so a corrupt plan beyond the bound stays invisible to them.
/// - [`InspectionSelection::AllValidated`] deep-validates every plan — so
///   `check` keeps full integrity coverage — while retaining record data only
///   for the first `limit` plans.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InspectionSelection {
    Bounded { limit: usize },
    AllValidated { limit: usize },
    One(PlanId),
}

impl InspectionSelection {
    pub(crate) fn retain_limit(&self) -> Option<usize> {
        match self {
            Self::Bounded { limit } | Self::AllValidated { limit } => Some(*limit),
            Self::One(_) => None,
        }
    }
}

/// One Plan and everything a read command needs from it, deep-validated once.
#[derive(Debug, Clone)]
pub struct LoadedPlanSnapshot {
    pub plan: Plan,
    pub observations: Vec<EvidenceObservation>,
    pub supersessions: Vec<EvidenceSupersessionRecord>,
    /// `effective_observations` over the two above, computed once.
    pub effective: Vec<EvidenceObservation>,
    pub closure: Option<ClosureRecord>,
}

impl LoadedPlanSnapshot {
    pub fn plan(&self) -> &Plan {
        &self.plan
    }

    /// Effective observations after supersession, ready for assessment.
    pub fn effective(&self) -> &[EvidenceObservation] {
        &self.effective
    }

    /// Evidence actually attached to the Plan as opposed to superseded.
    pub fn active_observations(&self) -> &[EvidenceObservation] {
        &self.observations
    }

    pub fn supersessions(&self) -> &[EvidenceSupersessionRecord] {
        &self.supersessions
    }

    pub fn closure(&self) -> Option<&ClosureRecord> {
        self.closure.as_ref()
    }
}

/// Algorithmic counters for one snapshot.
///
/// Crate-private by design. These exist so the complexity properties can be
/// asserted deterministically in CI; they are not a public seam and never
/// influence an observation, status, or projection.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SnapshotCounters {
    pub(crate) subject_captures: u64,
    pub(crate) plan_files_decoded: u64,
    pub(crate) observation_files_decoded: u64,
    pub(crate) supersession_files_decoded: u64,
    pub(crate) closure_files_decoded: u64,
    /// Plans deep-validated, including those beyond the retention bound.
    pub(crate) plans_validated: u64,
    /// Plans whose record data was retained in memory.
    pub(crate) plans_retained: u64,
}

/// Repository-level read model for one bounded inspection.
///
/// Every value here is already validated. Projection and assessment consume
/// these facts and perform no further I/O.
#[derive(Debug, Clone)]
pub struct InspectionSnapshot {
    pub(crate) repository_id: String,
    /// Captured twice; guaranteed equal because the constructor rejects drift.
    pub(crate) subject: Option<SubjectRevision>,
    /// Present when Git subject capture failed. Read commands degrade to
    /// `current_subject_unavailable` rather than inventing a subject.
    pub(crate) subject_error: Option<String>,
    /// Deep-validated plans retained for projection, in deterministic order.
    pub(crate) plans: Vec<LoadedPlanSnapshot>,
    /// Total plans selected for inspection, which may exceed `plans.len()`.
    pub(crate) total_plans: usize,
    /// Record counts across every selected plan, not just the retained ones.
    pub(crate) observations_counted: usize,
    pub(crate) supersessions_counted: usize,
    pub(crate) closures_counted: usize,
    pub(crate) pending_closures: Vec<eggplan_core::PlanId>,
    pub(crate) abandoned_staging_files: Vec<PathBuf>,
    pub(crate) counters: SnapshotCounters,
}

impl InspectionSnapshot {
    pub fn repository_id(&self) -> &str {
        &self.repository_id
    }

    /// The single subject captured for this inspection.
    ///
    /// `None` means Git subject capture failed; see
    /// [`InspectionSnapshot::subject_error`].
    pub fn subject(&self) -> Option<&SubjectRevision> {
        self.subject.as_ref()
    }

    pub fn subject_error(&self) -> Option<&str> {
        self.subject_error.as_deref()
    }

    /// Retained, deep-validated plans in deterministic selection order.
    pub fn plans(&self) -> &[LoadedPlanSnapshot] {
        &self.plans
    }

    /// Number of plans selected for inspection. Greater than `plans().len()`
    /// when the selection bound retained only a prefix.
    pub fn total_plans(&self) -> usize {
        self.total_plans
    }

    /// True when selection was bounded and further plans exist.
    pub fn truncated(&self) -> bool {
        self.total_plans > self.plans.len()
    }

    pub fn observations_counted(&self) -> usize {
        self.observations_counted
    }

    pub fn supersessions_counted(&self) -> usize {
        self.supersessions_counted
    }

    pub fn closures_counted(&self) -> usize {
        self.closures_counted
    }

    pub fn pending_closures(&self) -> &[eggplan_core::PlanId] {
        &self.pending_closures
    }

    pub fn abandoned_staging_files(&self) -> &[PathBuf] {
        &self.abandoned_staging_files
    }

    /// Crate-private algorithmic counters. Not part of the public API: this is
    /// instrumentation for the in-crate complexity regressions, not a seam a
    /// caller can drive.
    pub(crate) fn counters(&self) -> &SnapshotCounters {
        &self.counters
    }
}

pub(crate) fn acquire_shared_lock(
    root: &std::path::Path,
    timeout: Duration,
) -> Result<SharedLockGuard, RepoError> {
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
        match file.try_lock_shared() {
            Ok(()) => return Ok(SharedLockGuard(file)),
            Err(error) if crate::store::is_shared_lock_contention(&error) => {
                if start.elapsed() >= timeout {
                    return Err(RepoError::LockTimeout);
                }
                thread::sleep(Duration::from_millis(10));
            }
            Err(std::fs::TryLockError::Error(error)) => return Err(RepoError::Io(error)),
            Err(std::fs::TryLockError::WouldBlock) => {
                unreachable!("contention handled by the guard arm above")
            }
        }
    }
}

pub(crate) struct SharedLockGuard(File);
impl Drop for SharedLockGuard {
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}

pub(crate) fn capture_subject(
    source: &GitSubjectSource,
    counters: &mut SnapshotCounters,
) -> Result<SubjectRevision, GitSubjectError> {
    counters.subject_captures += 1;
    source.capture()
}

/// Build a snapshot from already-loaded facts.
///
/// The caller performs steps 1-2 and 4; this function performs the terminality
/// contract: recapture `S2`, compare, and only then hand back a snapshot that
/// claims a stable subject. Keeping the comparison here means no code path can
/// return a snapshot without it having been made.
pub(crate) fn seal(
    repository_id: String,
    subject_at_start: Option<SubjectRevision>,
    plans: Vec<LoadedPlanSnapshot>,
    total_plans: usize,
    observations_counted: usize,
    supersessions_counted: usize,
    closures_counted: usize,
    pending_closures: Vec<eggplan_core::PlanId>,
    abandoned_staging_files: Vec<PathBuf>,
    subject_source: &GitSubjectSource,
    mut counters: SnapshotCounters,
) -> Result<InspectionSnapshot, RepoError> {
    let subject_at_end = capture_subject(subject_source, &mut counters);

    // A capture that fails on recapture while S1 succeeded is drift, not an
    // ordinary unavailability: the worktree changed underneath the read.
    let (subject, subject_error) = match (subject_at_start, subject_at_end) {
        (Some(start), Ok(end)) => {
            if start != end {
                return Err(RepoError::SubjectDrift);
            }
            (Some(start), None)
        }
        (Some(_), Err(error)) => return Err(RepoError::InspectionSubject(error)),
        (None, Err(error)) => (None, Some(error.to_string())),
        (None, Ok(_)) => {
            // Unavailable at start and available at end: the worktree came into
            // existence mid-scan. Treat as drift rather than trusting either.
            return Err(RepoError::SubjectDrift);
        }
    };

    Ok(InspectionSnapshot {
        repository_id,
        subject,
        subject_error,
        plans,
        total_plans,
        observations_counted,
        supersessions_counted,
        closures_counted,
        pending_closures,
        abandoned_staging_files,
        counters,
    })
}

/// Selection ordering is deterministic: `RepositoryStore::list` sorts, and the
/// snapshot preserves that order. This helper exists so the property is
/// asserted where it is relied upon rather than assumed from a caller.
pub(crate) fn selection_is_deterministic(ids: &[eggplan_core::PlanId]) -> bool {
    ids.windows(2).all(|pair| pair[0] <= pair[1])
}