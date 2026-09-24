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

mod git_subject;
mod store;

pub use git_subject::{GitSubjectError, GitSubjectOptions, GitSubjectSource};
pub use store::{PlanStore, RepoError, RepositoryStore, StoreOptions};
