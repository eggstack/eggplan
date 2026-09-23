# Foundation Core M002 Closure — Repository Store, CAS, and Git Subject

Status: conditionally closed

Source plan: plans/implementation/foundation-core/002-repository-store-cas-and-subject.md

Source roadmap: plans/subsystems/foundation-core-roadmap.md

Reviewed baseline: e459c1b (Foundation M001 closure and M002 baseline)

Implementation commit: d154234 (`feat(repo): add CAS store and Git subject capture`)

Closure record and status transition: recorded in Git history.

## Executive finding

M002 production implementation is complete and Linux qualification passed. The
`eggplan-repo` crate provides versioned `.eggplan` state, safe plan paths,
digest-checked reopen, atomic replacement, cross-process cooperative locking,
CAS revisions, and bounded Git SubjectRevision capture including dirty
worktrees and submodules. M002 is conditionally closed because Windows and
macOS runtime and cross-target evidence could not be obtained in this Linux
environment. The missing evidence and platform durability limits are bounded
and documented in `architecture/repository.md`. Evidence M001 can proceed using
the stable repository and subject contracts while preserving those limits.

## Requirement-to-evidence matrix

| Requirement | Evidence | Result |
|---|---|---|
| `eggplan-repo` and `.eggplan` layout | `crates/eggplan-repo`, `architecture/repository.md` | Pass; config, plan envelope, lock, reserved evidence/closure paths documented |
| Repository identity and config version | `.eggplan/config.toml` initialization and reopen | Pass; stable generated `epr_` identity and config schema v1 |
| Safe IDs, path checks, symlink rejection | `store.rs`, repository tests | Pass on Linux; tests reject symlink plan directory and plan file without touching targets |
| Create/get/list/reopen | `PlanStore`, repository integration tests | Pass |
| CAS revision semantics | concurrent thread and independent-process tests | Pass; exactly one revision-0 writer succeeds, loser cannot overwrite; stale post-reopen write conflicts |
| Lock behavior | bounded `fs2` lock and lock-timeout/disappearance tests | Pass on Linux; timeout typed; stale revision remains rejected after lock file removal/reopen |
| Atomic staging and recovery | same-directory `.tmp-*`, `sync_all`, replacement and staging inspection tests | Pass on Linux; abandoned staging is reported and never loaded as canonical |
| Schema and digest reopen validation | stored envelope schema v1, content digest, corrupt/unknown tests | Pass; tampering and unknown storage schema fail closed |
| Git clean/modified/staged/untracked behavior | libgit2 fixture tests | Pass; equivalent dirty capture is stable and content changes alter the digest |
| Submodules | nested checked-out submodule fixture | Pass on Linux; gitlink, nested HEAD, and nested dirty content are included |
| Non-Git and dirty fingerprint bounds | non-Git and max-path fixture tests | Pass; unavailable identity and exceeded bounds return explicit errors |
| MSRV and ordinary verification | commands below | Pass on Rust 1.89 and stable Linux |
| Windows/macOS platform qualification | cross-target attempts below | Environmental block; no runtime claim made |

## Exact verification executed

All successful commands ran from repository root after implementation:

| Command | Result |
|---|---|
| `rtk cargo fmt --all -- --check` | Pass |
| `rtk cargo check --workspace --all-targets --locked` | Pass |
| `rtk cargo clippy --workspace --all-targets --locked -- -D warnings` | Pass |
| `rtk cargo test --workspace --locked` | Pass, 24 tests across 5 suites |
| `rtk cargo +1.89.0 check --workspace --all-targets --locked` | Pass |
| `rtk bash scripts/check-core-boundary.sh` | Pass |
| `rtk git diff --check` | Pass |
| `rtk cargo check --workspace --all-targets --locked --target x86_64-pc-windows-msvc` | Failed/environmental block: `libz-sys` could not find MSVC/Vcpkg; host GNU compiler is unsupported for the MSVC target |
| `rtk cargo check --workspace --all-targets --locked --target x86_64-apple-darwin` | Failed/environmental block: host C compiler rejected Apple `-arch`/deployment-target flags; Apple SDK/cross compiler unavailable |

Windows/macOS runtime tests were not run. Cross-target failures above are
toolchain/environment failures, not passing verification. The checks ran after
the last production source change; later edits only recorded planning closure
and status.

## State layout, concurrency, and failure review

Canonical Plan envelopes contain storage schema version 1, the validated Plan,
and its SHA-256 digest. Reopen checks envelope version, Plan schema and graph,
directory/object identity, and digest. CAS requires an exact expected revision
and next revision and validates lifecycle transitions under the repository
lock. Lock state is not a revision token; stale revisions still conflict after
reopen or lock-file replacement.

Writes use a same-directory named staging file, sync its contents, then
atomically persist over the canonical path. Unix also syncs the containing
directory. If that final directory sync fails after replacement, the store
reports `DurabilityUnknown` because the new file may already be visible. A
crash before rename leaves the prior canonical file intact; staged files are
reported separately. Windows directory-entry durability and macOS hardware
flush guarantees are not overclaimed. Network filesystem lock/rename behavior
remains outside qualification.

Git capture uses libgit2 only; Eggplan does not execute Git or repository
hooks. Dirty manifests are sorted and bounded to 10,000 paths, 64 MiB of
worktree file bytes, and eight submodule levels. The manifest incorporates
status bits, index object IDs, regular-file bytes, symlink targets, and nested
submodule HEAD/dirty manifests. Ignored files are excluded according to Git
status semantics. Unsafe/non-Unicode paths and exceeded bounds fail explicitly.

## Invariant, compatibility, and security review

- The store executes no plan-defined commands and adds no scheduler or remote
  service behavior.
- IDs are restricted before they become path components. Managed roots,
  directories, config, and plan files are checked against symlinks.
- Dirty content contributes only to a digest; contents are not persisted in
  diagnostics or SubjectRevision.
- Plan revision state is canonical and stale writes fail explicitly.
- Evidence observation persistence remains reserved for Evidence M001.
- No prior runtime data existed, so no migration was necessary.
- Static check, clean Git subject, all local dirty cases, submodule fixture,
  and Linux path cases pass.

## Roadmap and registry disposition

Foundation M002 is conditionally closed. The repository and subject contracts
required by Evidence M001 are available and Linux-qualified; Evidence M001 may
proceed with the Windows/macOS qualification caveat preserved. Foundation M003
remains roadmap-level. The dependent plan's baseline/status handoff is recorded
in the subsequent registry update commit.

## Unresolved findings

No production defect blocks local repository/evidence integration. Outstanding
operational evidence: Windows runtime path/replace/lock tests and macOS runtime
path/replace/lock/durability tests. Do not claim those platforms qualified
until those checks run successfully.
