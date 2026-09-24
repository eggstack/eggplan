#![forbid(unsafe_code)]
//! Repository-local plan persistence and Git subject identity.

mod git_subject;
mod store;

pub use git_subject::{GitSubjectError, GitSubjectOptions, GitSubjectSource};
pub use store::{PlanStore, RepoError, RepositoryStore, StoreOptions};

#[doc(hidden)]
pub use store::ScriptedSubjectCapture;
#[doc(hidden)]
pub use store::{GitSubjectCapture, SubjectCapture};
