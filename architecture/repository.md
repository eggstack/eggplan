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

Ordinary CAS cannot transition a Plan to Closed. `finalize_closure` acquires
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
