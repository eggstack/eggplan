#![forbid(unsafe_code)]
//! Repository-local plan persistence and Git subject identity.
//!
//! # Closure subject authority boundary
//!
//! `RepositoryStore::finalize_closure` is the only supported externally callable
//! guarded closure entry point. It captures both closure subject revisions
//! (S1 and S2) from the repository's configured `GitSubjectSource` under the
//! repository lock and never accepts a caller-injected subject capture source.
//!
//! The internal `SubjectCapture` seam exists only to enable deterministic
//! crate-internal regression tests of stale / drift / capture-failure
//! behavior. It is not part of the public API.
//!
//! ```compile_fail
//! use eggplan_repo::SubjectCapture;
//! fn _assert_trait_bound<T: SubjectCapture>() {}
//! ```
//!
//! ```compile_fail
//! use eggplan_repo::ScriptedSubjectCapture;
//! fn _assert_value<T>(capture: ScriptedSubjectCapture) -> T {
//!     unimplemented!()
//! }
//! ```
//!
//! ```compile_fail
//! fn _assert_no_injected_finalizer(
//!     store: &eggplan_repo::RepositoryStore,
//!     candidate: &eggplan_core::ClosureCandidate,
//!     closure_id: eggplan_core::ClosureId,
//!     finalized_at_unix_ms: u64,
//!     capture: &dyn eggplan_repo::SubjectCapture,
//! ) {
//!     let _ = store.finalize_closure_with_capture(
//!         candidate,
//!         closure_id,
//!         finalized_at_unix_ms,
//!         capture,
//!     );
//! }
//! ```
//!
//! # Git subject fingerprint boundary
//!
//! `capture_git_subject_fingerprint` exposes the exact revision, clean/dirty
//! state, and Eggplan-native dirty digest an external host must persist for
//! later Eggplan exact-subject assessment. It shares one capture
//! implementation with `GitSubjectSource::capture`, so both agree by
//! construction, and it deliberately exposes no repository identity and no
//! part of the dirty manifest.
//!
//! ```compile_fail
//! fn _assert_fingerprint_is_repository_id_free(
//!     fingerprint: eggplan_repo::GitSubjectFingerprintV1,
//! ) -> String {
//!     fingerprint.repository_id
//! }
//! ```
//!
//! ```compile_fail
//! fn _assert_fingerprint_exposes_no_paths(
//!     fingerprint: eggplan_repo::GitSubjectFingerprintV1,
//! ) -> usize {
//!     fingerprint.paths.len()
//! }
//! ```
//!
//! ```compile_fail
//! fn _assert_fingerprint_exposes_no_manifest(
//!     fingerprint: eggplan_repo::GitSubjectFingerprintV1,
//! ) -> usize {
//!     fingerprint.dirty_manifest.len()
//! }
//! ```

//! # Repository inspection snapshot boundary
//!
//! `RepositoryStore::inspection_snapshot` is the only supported multi-plan read
//! entry point. It captures the Git subject exactly twice per inspection using
//! the repository's configured `GitSubjectSource`, refuses to return a
//! snapshot whose subject drifted, and exposes no injectable subject authority.
//! Its algorithmic counters are crate-private instrumentation.
//!
//! ```compile_fail
//! use eggplan_repo::SnapshotCounters;
//! fn _assert_counters_are_private(_: SnapshotCounters) {}
//! ```
//!
//! ```compile_fail
//! fn _assert_no_injected_inspection_subject(
//!     store: &eggplan_repo::RepositoryStore,
//!     capture: &dyn eggplan_repo::SubjectCapture,
//! ) {
//!     let _ = (store, capture);
//! }
//! ```
//!
//! ```compile_fail
//! fn _assert_snapshot_is_not_serialized(
//!     snapshot: eggplan_repo::InspectionSnapshot,
//! ) -> String {
//!     serde_json::to_string(&snapshot).unwrap()
//! }
//! ```

mod git_subject;
pub(crate) mod snapshot;
mod store;
#[cfg(test)]
pub(crate) mod test_support;

pub use git_subject::{
    GitSubjectError, GitSubjectFingerprintV1, GitSubjectOptions, GitSubjectSource,
    capture_git_subject_fingerprint,
};
pub(crate) use snapshot::SnapshotCounters;
pub use snapshot::{InspectionSelection, InspectionSnapshot, LoadedPlanSnapshot};
pub use store::{PlanStore, RepoError, RepositoryStore, StoreOptions};
