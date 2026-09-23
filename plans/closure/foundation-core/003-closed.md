# Foundation M003 Closure — Subject Scope, Strict Schema, and Platform Hardening

Status: closed

Source plan: plans/implementation/foundation-core/003-subject-scope-strict-schema-and-platform-hardening.md

Source roadmap: plans/subsystems/foundation-core-roadmap.md

Reviewed baseline: `c1fa03b697d68a53eb8d11648e67946f66fe3875`; implementation head `7656aefff9f809f963bba6f4373ac8f1603445e9`.

## Executive finding

Foundation M003 closes. Repository-managed state is excluded from the Git
dirty manifest through an explicit store-configured path exclusion. Exclusion
paths are canonicalized before comparison, covering macOS `/var` aliases and
Windows path normalization. Nested plan wire structs reject unknown fields
while the schema-v1 canonical fixture and digest remain unchanged. Native
Linux, macOS, and Windows checks and the Rust 1.89 check/test passed in hosted
CI run [35860695866](https://github.com/eggstack/eggplan/actions/runs/35860695866).

## Finding-to-evidence matrix

| Finding | Correction | Evidence |
|---|---|---|
| F-M003-01 state writes perturb source subject | `RepositoryStore::subject_source` excludes its managed root and descendants by normalized status path | `repository_managed_state_does_not_perturb_subject`; ordinary tracked source modification changes the subject |
| F-M003-02 unknown nested schema-v1 fields accepted | `deny_unknown_fields` added to Plan, PlanItem, AcceptanceCriterion, EvidenceRequirement, SubjectRevision, and ArtifactRef | `nested_unknown_plan_fields_fail_closed`; `repository_reopen_rejects_unknown_nested_plan_fields` |
| F-M003-03 Windows/macOS evidence absent | Native workflow matrix and Windows lock error classification | Hosted run 35860695866: Linux/macOS/Windows check, clippy, tests pass; Linux format/boundary and Rust 1.89 check/test pass |
| F-M003-04 README/docs drift | Removed duplicate bootstrap README; documented strict parsing, exclusion, and platform durability limits | Current README and architecture/core.md, architecture/repository.md |

## Production and verification evidence

The final hosted workflow ran against `7656aefff9f809f963bba6f4373ac8f1603445e9`.

| Command/job | Result |
|---|---|
| `cargo fmt --all -- --check` (Linux) | pass |
| `cargo check --workspace --all-targets --locked` (Linux/macOS/Windows) | pass |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` (Linux/macOS/Windows) | pass |
| `cargo test --workspace --locked` (Linux/macOS/Windows) | pass |
| `bash scripts/check-core-boundary.sh` (Linux/macOS) | pass |
| `cargo +1.89.0 check --workspace --all-targets --locked` (Linux) | pass |
| `cargo +1.89.0 test --workspace --locked` (Linux) | pass |
| `git diff --check` (local) | pass |

The Windows boundary script step is skipped by workflow configuration; its
production compile, clippy, and tests passed. Native tests exposed and led to
two corrections: canonical path comparison for macOS `/var` aliases and
recognition of Windows `ERROR_LOCK_VIOLATION` as lock contention. Earlier
workflow runs failed on those defects and on a Windows-only clippy lint; all
were corrected before the cited passing run.

## Invariant, recovery, and compatibility review

- Administrative exclusion applies to an exact worktree-relative root and its
  descendants, not shared string-prefix siblings. Submodule manifests are
  independently captured. External managed roots do not filter worktree paths.
- Canonicalizing existing roots addresses OS path aliases; if roots cannot be
  canonicalized, capture fails explicitly rather than emitting a partial
  subject.
- Ordinary source status, index, file, symlink, and submodule data remain in
  the bounded dirty manifest. Schema-v1 subject identity still includes HEAD,
  so a metadata-only commit changes identity.
- Unknown fields fail during parse/reopen. Valid schema-v1 canonical fixture
  and digest passed unchanged.
- Windows lock contention is retried until success/timeout, including raw
  `ERROR_LOCK_VIOLATION` (33). File replacement and store/reopen tests passed on
  all native runners.
- Platform durability remains bounded as documented: file data is synced
  before replacement; Unix directory sync is attempted; Windows has no
  portable directory sync in this implementation; macOS `sync_all` does not
  claim hardware-level `F_FULLFSYNC`.

## Roadmap disposition

Foundation M002's inherited platform qualification caveat is resolved by the
native supported-subset results above. Historical M002 closure remains
unchanged.

Evidence M001 C001 may proceed: managed-state subject identity, strict v1
parsing, and the supported platform contract are now qualified. Evidence M002,
Projection/CLI M001, CodeGG Integration M001, and Eggstack Provider SPI M001
remain blocked on C001 closure.

## Registry updates

- Foundation M003: closed.
- Evidence M001 C001: ready for handoff, then active as sequential work begins.
- Evidence M002, Projection/CLI M001, CodeGG Integration M001, and Eggstack
  Provider SPI M001: remain blocked on C001.
