# Repository storage architecture

`eggplan-repo` stores canonical state under `.eggplan/`:

```text
.eggplan/
  config.toml                    # storage schema version and stable repository ID
  .lock                          # cooperative process lock
  plans/<plan-id>/
    plan.json                    # canonical versioned envelope and current Plan
    evidence/
      <observation-id>.json      # immutable finalized observations
    supersessions/
      <supersession-id>.json     # immutable correction links
    closure.pending.json         # recoverable close transaction intent
    closure.json                 # immutable finalized closure record
```

The plan envelope has storage schema version 1, a validated schema-v1 or
schema-v2 Plan, and a SHA-256 digest of that Plan's canonical bytes. Reopen validates the
envelope, domain schema, directory/object identity, and digest. Unknown or
malformed objects fail closed. Plan creation starts at revision zero; update
requires exactly the current revision and a candidate revision one greater.
Stale calls return a typed conflict. The store checks the persisted revision
while holding its short-lived lock; the revision remains the explicit CAS
token callers must reload after conflicts.

Evidence observations are append-only. Replaying the same ID and identical
canonical bytes is idempotent; the same ID with different content conflicts.
Reads validate schema and digest before returning. Ledger files are sorted by
typed ID when listed; `.tmp-*` staging remnants never enter that list.
Assessment remains pure in `eggplan-core` and does not resolve remote
providers. The store accepts at most 10,000 observations per plan, with a
1 MiB encoded-file limit per observation and a 16 MiB encoded-file limit per
Plan.

Ordinary CAS cannot transition a Plan to Closed, and cannot modify a Plan that
is already Closed. The first is `RepoError::GuardedClosureRequired`; the second
is `RepoError::ClosedPlanImmutable`, which is raised for every target status
including Closed itself, before any write. A canonical Closed Plan is therefore
immutable under ordinary CAS in both directions: only
`RepositoryStore::finalize_closure` writes one. The refusal precedes
`atomic_write`, so a rejected update leaves stored bytes unchanged; it is never
deferred into a late corruption report on the next read.

`finalize_closure` acquires
the repository lock, recaptures the Git `SubjectRevision` from the
repository's configured `GitSubjectSource` under that lock, requires the
captured subject to equal the candidate's subject, rebuilds trusted provider
policy from the candidate's bounded snapshot, recomputes assessment over the
effective supersession view, recaptures the subject a second time immediately
before writing the pending closure record, and then writes the pending
closure record before replacing the Plan and promoting the record. A
caller-owned current subject is not authoritative. A mismatch before
assessment is `RepoError::ClosureSubjectStale`; a mismatch between the two
captures is `RepoError::ClosureSubjectDrift`; a capture failure is
`RepoError::ClosureSubjectCapture`. None of those outcomes produces pending
or final closure state or a Closed Plan. Open recovers a matching pending
transaction: source revision means discard; exact closed target means
promote. Other combinations fail as corruption.

The supported external closure API is exactly
`RepositoryStore::finalize_closure`. There is no public alternate finalizer
that accepts a caller-supplied subject capture source. The internal
`SubjectCapture` seam, its `GitSubjectCapture` adapter, and the
`ScriptedSubjectCapture` test double are crate-private; they exist only to
enable deterministic stale / drift / capture-failure regressions inside
`eggplan-repo` and are not part of the supported public API. A crate-level
`compile_fail` doctest in `eggplan-repo` and the
`scripts/check-closure-authority-boundary.sh` static guard fail if a future
change re-exports those types as public or reintroduces an alternate
capture-injected finalizer.

The store validates each managed path component with `symlink_metadata`,
rejects symlinked roots/directories/plan files, uses PlanId's restricted
alphabet for directory names, and stages writes in the destination directory.
Staged bytes are flushed with `sync_all`, then `NamedTempFile::persist` performs
the platform replacement operation. On Unix the containing directory is also
synced after rename. A process crash before replacement leaves the previous
canonical file intact and may leave a `.tmp-*` file; `abandoned_staging_files`
reports such files but never loads them as state.

The power-loss guarantee is limited by OS and filesystem semantics. The file
contents are synced before replacement; Unix directory entries receive a
best-effort portable `sync_all`. Windows does not expose a portable directory
sync through this implementation, and macOS `sync_all` is not a claim of
hardware-level `F_FULLFSYNC`. Network filesystems may weaken advisory locking
or rename guarantees. Foundation M003 adds native Linux, macOS, and Windows
workflow qualification. Each closure record must cite actual hosted run IDs
and preserve any platform-specific failures.

## Git SubjectRevision

Repository config creates a stable `epr_` identity. `GitSubjectSource` uses
libgit2 to resolve the worktree HEAD; it does not spawn Git, load repository
hooks, or run repository-defined commands. A clean subject contains the
repository ID and HEAD OID. For dirty trees, a bounded sorted manifest covers
status bits, index blob IDs, regular file bytes, symlink targets, and nested
submodule HEAD/dirty manifests. Untracked files and submodules participate.
Default limits are 10,000 changed paths, 64 MiB observed worktree file bytes,
and eight submodule levels. Exceeding a limit fails instead of returning an
incomplete fingerprint. Non-Unicode paths also fail explicitly.

Dirty fingerprints are integrity identifiers, not redacted storage: the
fingerprint includes content in its hash calculation, but content is never
persisted in diagnostics or the subject object.

When created by `RepositoryStore`, the subject source explicitly excludes that
store's managed root and descendants from the worktree dirty manifest. The
exclusion is applied to normalized Git status paths, independent of `.gitignore`
and whether those paths are tracked, staged, ignored, or untracked. A sibling
whose name merely shares a string prefix remains in scope. Submodule manifests
are evaluated independently. If the configured state path is outside the
discovered worktree, no exclusion is applied. The HEAD OID remains part of the
schema-v1 subject, so committing administrative state still changes identity.

## Git subject fingerprint

`capture_git_subject_fingerprint` returns the exact revision, clean/dirty
state, and Eggplan-native dirty digest Eggplan requires for exact-subject
assessment, without any repository identity. It exists so an external host can
capture and durably persist those three values at its own execution boundary
instead of reconstructing a historical subject later. It reads the same capture
implementation as `GitSubjectSource`, so the two agree by construction, and the
digest bytes are the frozen values asserted by the golden digest matrix.

The returned `GitSubjectFingerprintV1` carries only schema version, HEAD
revision, clean/dirty state, and the optional dirty digest. It exposes no path
list, file content, index entry, symlink target, manifest byte, repository ID,
or provider/evidence/closure state, and it is not a universal Git digest
standard: CodeGG keeps its own native digest for its own execution provenance
and may use only the Eggplan-compatible value for a bound repository subject.
A host that binds plans must still prove identity itself; Eggplan offers no
repository-ID relabeling helper for a fingerprint.

The fingerprint supports the same single administrative-root exclusion,
fails closed with a typed error when that exclusion does not resolve inside the
discovered worktree, and returns the same typed `GitSubjectError` classes as
normal subject capture for bound overflow, unsafe or symlinked paths,
non-Unicode paths, Git errors, unborn HEAD, and filesystem read failures. No
partial digest is ever returned.
