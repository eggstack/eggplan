# Foundation Core M002 — Repository Store, CAS, and Subject Identity

Status: blocked

Repository baseline: to be re-established after Foundation M001 closure.

Source roadmap:

- plans/subsystems/foundation-core-roadmap.md

Long-term requirements:

- plans/000-long-term-specification.md sections 4.6-4.8, 11, 13, 18-20
- plans/001-terminology-and-domain-model.md
- plans/002-long-term-roadmap.md Foundation M002

Applicable ADRs: ADR-0001, ADR-0002, ADR-0003.

Primary class: capability / invariant

Hard dependency: Foundation M001 closure.

## 1. Objective

Add eggplan-repo: a Git-friendly local repository backend with safe .eggplan
layout, atomic plan persistence, revision/CAS semantics, cooperative
cross-process locking, reopen/recovery validation, and a Git SubjectRevision
adapter that distinguishes clean and dirty worktrees.

## 2. Readiness gate

Do not begin until M001 closure freezes schema v1, canonical digest rules,
PlanRevision semantics, and core store-facing types.

At handoff, replace this plan's baseline with the then-current commit and
re-check Rust/fs-lock/Git implementation choices.

## 3. Invariants

- Repository paths are relative, normalized, and confined beneath the state root.
- Symlink/path traversal cannot redirect writes outside the root.
- Partial staging files are never canonical.
- Stale expected revision returns Conflict.
- A cooperative lock is not the sole correctness mechanism; CAS remains required.
- Destination replacement never silently overwrites an unexpected newer revision.
- Reopen revalidates schema and digest.
- Dirty Git state is never represented as clean HEAD.
- Repository store performs no arbitrary plan-defined command execution.

## 4. Scope

Implement:

- eggplan-repo crate;
- .eggplan/config.toml minimal configuration as needed;
- plan directory layout and current-plan JSON storage;
- safe atomic write utility;
- lock strategy with bounded acquisition behavior;
- PlanStore trait implementation and explicit errors;
- list/get/create/CAS update/cancel/close-state persistence needed by later layers;
- reopen/recovery of canonical files;
- detection/reporting of abandoned temp/staging files;
- Git subject adapter using a safe library or bounded git command adapter;
- deterministic dirty-state fingerprint algorithm and fixtures.

Do not implement evidence observation storage yet unless only generic object
storage plumbing is required and remains unused until Evidence M001.

## 5. Repository layout target

Initial target:

    .eggplan/
      config.toml
      plans/
        <plan-id>/
          plan.json
          evidence/
          closure.json   # absent until closure exists

M002 may adjust filenames through implementation evidence, but must document
the final v1 layout and keep it versioned.

## 6. Atomicity/durability

Use same-directory temporary files and atomic rename/replace where supported.
Explicitly document differences in directory-entry durability on platforms
where portable fsync is unavailable.

The implementation must not claim crash-consistent guarantees stronger than
its actual platform primitives.

## 7. Locking and contention

Choose a cross-platform lock mechanism compatible with Rust 1.89 and the
Eggstack target matrix.

Tests need at least:

- competing same-revision writers: exactly one succeeds;
- stale retry fails until caller reloads;
- lock timeout/busy is typed;
- lock disappearance/restart does not allow stale overwrite.

## 8. Git subject

Capture enough state for later exact-subject evidence policy:

- repository stable/configured identity;
- HEAD OID;
- clean/dirty;
- deterministic dirty digest when dirty;
- optional branch/display metadata.

The dirty digest must be bounded and deterministic without embedding secrets or
unbounded file contents in diagnostics.

Define behavior for untracked files and submodules explicitly.

## 9. Verification

Required categories:

- path confinement and symlink cases;
- create/load/list/CAS;
- concurrent writer races;
- interrupted staging/reopen;
- corrupt/unknown schema;
- temp collision;
- Git clean/modified/staged/untracked cases;
- equivalent dirty state stable digest;
- changed dirty state changed digest;
- non-Git repository behavior;
- Windows/macOS/Linux supported subset;
- MSRV, fmt, clippy, full tests.

## 10. Stop conditions

Stop if M001 schema/digest changes are required materially; if the chosen lock
or atomic replacement primitive cannot meet cross-platform claims; or if Git
subject capture would require executing arbitrary repository-controlled
commands/hooks.

## 11. Acceptance criteria

A caller can safely persist/reopen a Plan and update it by expected revision;
races fail explicitly; incomplete writes do not become canonical; and current
Git subject identity is truthful enough for Evidence M001 to reject stale
observations.

## 12. Closure evidence required

Record filesystem guarantees by platform, race results, path-security cases,
Git subject fixtures, exact state layout, full verification, and any limits
that Evidence M001 must preserve.
