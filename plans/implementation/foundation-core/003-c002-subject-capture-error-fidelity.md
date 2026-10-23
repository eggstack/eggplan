# Foundation Core M003 C002 — Subject-Capture Error Fidelity in Inspection and CLI Read Paths

Status: closed (see plans/closure/foundation-core/003-c002-closed.md)

Repository baseline: 37ad290ca6b971a68d2871aedb39f4882f4eb2a8

Source roadmap:

- plans/subsystems/foundation-core-roadmap.md
- plans/subsystems/projection-cli-roadmap.md

Long-term requirements:

- plans/000-long-term-specification.md (subject unavailability is explicit)
- plans/001-terminology-and-domain-model.md (subject identity vocabulary)

Related ADRs: ADR-0002, ADR-0003.

Predecessor plan/closure:

- plans/implementation/foundation-core/002-repository-store-cas-and-subject.md
- plans/closure/foundation-core/002-conditionally-closed.md
- plans/implementation/projection-cli/003a-repository-inspection-snapshot-and-performance-qualification.md
- plans/closure/projection-cli/003a-closed.md
- plans/implementation/evidence-closure/002-c001-finalization-subject-revalidation.md
- plans/closure/evidence-closure/002-c001-closed.md

Primary class: correctness / diagnostics fidelity

## 1. Objective

Carry the Git subject capture error from the point of failure to the point of
reporting, on both the repository inspection path and the CLI read path, so
that a subject capture failure is reported as the failure it is rather than
being collapsed into an unrelated drift diagnosis.

Two distinct losses exist today. The repository inspection snapshot discards
the start-capture error with `.ok()` and then reports `SubjectDrift` when the
recapture succeeds. The CLI read path discards the same error with `.ok()` and
reports a generic warning. In both cases the actionable cause — for example
a transient Git lock, an unsafe path in `git status`, or a state root that is
not a Git worktree at all — is lost.

This pass changes diagnostics only. It does not change what the repository
does, what it stores, or when it fails closed.

## 2. Finding

### E-FC-C002-01 — a failed start capture is collapsed to `None`, then diagnosed as drift

`crates/eggplan-repo/src/store.rs:1154-1155`:

    let subject_at_start =
        crate::snapshot::capture_subject(&subject_source, &mut counters).ok();

The `GitSubjectError` is discarded. `subject_at_start` becomes
`Option<SubjectRevision>` and is passed into
`crate::snapshot::seal(SealedInput { .. })` at
`crates/eggplan-repo/src/store.rs:1217-1219`.

`crates/eggplan-repo/src/snapshot.rs:329-343` then matches on that
`Option`:

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

The `(None, Err(error))` arm preserves the cause. The `(None, Ok(_))` arm does
not, because the start-capture error was already discarded upstream.

The `(None, Ok(_))` comment names one cause — "the worktree came into
existence mid-scan" — but the `None` it inspects can equally mean the start
capture failed for any reason: transient Git lock contention, an unsafe path
in `git status`, a worktree that is not a Git repository, or a `GitSubjectError`
from the bound-exclusion path (`crates/eggplan-repo/src/git_subject.rs`). Every
one of those is reported to the caller as `RepoError::SubjectDrift`, which
`crates/eggplan-repo/src/store.rs:110-111` describes as "git subject drifted
during repository inspection".

**Blast radius.** Diagnostic fidelity only. The read is still refused; no
invalid state is admitted, no evidence is fabricated, and no closure authority
is affected. The cost is that an operator sees "drift" for what is a capture
failure, and cannot tell a concurrent worktree mutation apart from a Git
failure that has nothing to do with concurrency.

**Detection gap.** `crates/eggplan-repo/tests/repository.rs` covers CAS
contention, abandoned staging files, and the closed-plan immutability
regressions, but no test drives a `GitSubjectError` into
`RepositoryStore::inspection_snapshot`. The
`crates/eggplan-repo/src/git_subject.rs` error surface is therefore never
exercised through the snapshot's terminality logic.

### E-FC-C002-02 — the CLI discards the same error on the read path

`crates/eggplan-cli/src/lib.rs:1636`:

    let subject = store.subject_source().capture().ok();

The `GitSubjectError` is discarded and the read path proceeds with no subject.
The CLI then reports the generic warning `current_subject_unavailable`.

The same failure on the closure path is handled properly:
`crates/eggplan-cli/src/lib.rs:1683-1686` maps it to
`failure(command, "subject_unavailable", error.to_string(), json)`. The same
underlying error therefore produces two very different user-facing diagnostics
depending on which command was invoked.

**Blast radius.** Diagnostic fidelity only. No evidence, trust, or closure
behavior changes.

**Detection gap.** The CLI test suite exercises `subject_unavailable` on the
closure path but not the read path, so the asymmetry is not caught.

### E-FC-C002-03 — `InspectionSnapshot::subject_error()` has no consumer

`crates/eggplan-repo/src/snapshot.rs:185` exposes a public accessor for the
snapshot's subject error. The field it reads is written at
`crates/eggplan-repo/src/snapshot.rs:337` and read by no production code in
the workspace. The workspace-wide identifier search returns only the accessor
definition and this plan.

If this pass changes the snapshot's subject-error representation — see §5 —
the accessor's shape changes too. Its disposition must be decided explicitly
in this pass rather than left as an unused public accessor.

## 3. Controlling semantics

From `plans/000-long-term-specification.md` and `architecture/cli-control-surface.md:72-74`,
current Git subject unavailability is explicit: a read must report that the
subject is unavailable and must not silently present a subjectless result as a
complete one. From `architecture/deep-dive-repository.md`, the inspection
snapshot distinguishes *drift* (the subject changed under the read) from
*unavailability* (the subject could not be captured), and those are different
conditions with different remedies.

Required behavior after this pass:

- a start-capture failure is reported as a capture failure, with its original
  error preserved;
- a genuine mid-scan subject change is still reported as drift;
- the CLI read path reports the specific capture cause, using the same
  `subject_unavailable` diagnostic code the closure path already uses;
- the repository inspection snapshot and the CLI read path agree on the
  diagnostic vocabulary for the same underlying failure;
- no read succeeds that failed before, and no read that succeeded before fails
  for a new reason.

## 4. Scope

### In scope

- `crates/eggplan-repo/src/store.rs` inspection-snapshot subject capture;
- `crates/eggplan-repo/src/snapshot.rs` terminality logic and the
  subject-error representation;
- `crates/eggplan-cli/src/lib.rs` read-path subject capture;
- the `InspectionSnapshot::subject_error()` accessor's disposition;
- regression tests in `crates/eggplan-repo/tests/` and
  `crates/eggplan-cli/tests/`;
- documentation updates in `architecture/repository.md`,
  `architecture/cli-control-surface.md`, and the affected deep dives;
- registry and roadmap lineage updates.

### Out of scope

- the guarded closure finalizer's S1/S2 double-capture algorithm, which is
  owned by Evidence M002 C001 and is correct;
- `GitSubjectSource::capture` semantics, the frozen fingerprint contract, the
  dirty-digest golden bytes, and the bound-exclusion rules;
- `RepositoryStore::finalize_closure` and `SubjectCapture` containment, which
  are owned by Evidence M002 C002;
- the CLI's check-state vocabulary, the `stale` and `unavailable` check
  states, and the ten-state check contract;
- adding any new CLI command or diagnostic code beyond reusing the existing
  `subject_unavailable` code;
- repository locking, the pending-closure protocol, and atomic write;
- the deferred Interop/distribution subsystem.

## 5. Required production changes

Ownership is `eggplan-repo` for the snapshot contract and `eggplan-cli` for
its consumption.

Preferred shape:

- carry the start capture as a `Result` rather than an `Option`, so the
  failure is preserved across the `store.rs` → `snapshot.rs` boundary; the
  existing `RepoError::InspectionSubject(#[source] GitSubjectError)` variant
  at `crates/eggplan-repo/src/store.rs:112-113` already carries exactly this
  payload and should be reused rather than a new variant added;
- distinguish the four conditions in `snapshot.rs:329-343`: equal subjects
  (success), differing subjects (drift), and a failed capture at either end
  (unavailability with the original error);
- report the mid-scan case in which the subject is unavailable at the start and
  available at the end as the capture failure it is — the read still fails
  closed, but it fails with `RepoError::InspectionSubject` carrying the original
  capture error rather than with a drift diagnosis the read cannot support.
  `SubjectDrift` is reserved for the one case it describes: both captures
  succeeded and disagreed. *(Amended during implementation: the earlier
  wording in this bullet, which asked for the case to be reported as drift
  "with the original capture error recorded alongside", contradicted this plan's
  own acceptance criteria and the required tests, which forbid reporting a
  start-capture failure as `SubjectDrift`. The acceptance criteria govern.)*;
- in the CLI read path, replace `.ok()` with an explicit match that maps a
  capture failure to the existing `subject_unavailable` diagnostic code and
  preserves the underlying message, matching the closure path's behavior at
  `crates/eggplan-cli/src/lib.rs:1683-1686`;
- decide the `InspectionSnapshot::subject_error()` accessor's disposition
  explicitly: either it carries a meaningful, consumed value under the new
  representation, or it is removed. Do not leave a public accessor that
  returns a value no caller reads.

Explicitly rejected alternatives:

- adding a new `RepoError` variant for this condition, because
  `InspectionSubject` already carries the exact payload and a second variant
  would split one condition across two diagnostics;
- changing `SubjectDrift` to absorb capture failures, because that would
  preserve the current conflation rather than remove it;
- changing the guarded finalizer's capture path, which is a separate,
  already-correct contract.

## 6. Failure, cancellation, restart, and contention semantics

- The pass must not turn any previously-failing read into a succeeding read,
  and must not turn any previously-succeeding read into a failing one. A
  read that previously returned `SubjectDrift` because the start capture
  failed must still fail; it must fail with the more accurate cause.
- A genuine mid-scan subject change must still produce `SubjectDrift`, with
  no error payload, so callers that distinguish drift from unavailability by
  error type continue to work.
- The CLI must continue to emit its `current_subject_unavailable` warning on
  the read path. The change is that the warning and any accompanying
  diagnostic carry the specific cause. Warning counts and the set of commands
  that warn are unchanged.
- No locking, persistence, or atomic-write behavior changes. The snapshot
  remains a read-only, bounded, ephemeral structure.
- Restart and recovery semantics are unchanged: no repository state is
  migrated, and no stored bytes change.

## 7. Required tests

At minimum:

- a repository test that drives a `GitSubjectError` at the start capture and
  asserts the returned error is the capture failure, not `SubjectDrift`, and
  that the original error is recoverable through the error source chain;
- a repository test that asserts a genuine mid-scan subject change still
  yields `SubjectDrift`;
- a repository test that asserts the case where the subject is unavailable at
  the start and available at the end still fails closed, and that the original
  capture cause is preserved;
- a CLI test that drives an unavailable subject on a read command and asserts
  the `subject_unavailable` diagnostic code and that the underlying message is
  present;
- a CLI test that asserts the closure path's `subject_unavailable` behavior is
  unchanged;
- the existing `eggplan-repo` and `eggplan-cli` suites, green and unchanged
  apart from the new regressions.

## 8. Verification

Record actual results for:

    cargo fmt --all -- --check
    cargo check --workspace --all-targets --locked
    cargo clippy --workspace --all-targets --locked -- -D warnings
    cargo test --workspace --locked
    cargo test -p eggplan-repo --locked
    cargo test -p eggplan-cli --locked
    cargo +1.89.0 check --workspace --all-targets --locked
    cargo +1.89.0 test --workspace --locked
    bash scripts/check-core-boundary.sh
    bash scripts/check-closure-authority-boundary.sh
    bash scripts/check-projection-cli-boundary.sh
    git diff --check

The `cargo +1.89.0` rows are the MSRV gate and must be recorded truthfully,
including if the toolchain is unavailable in the implementing environment.
Hosted Linux/macOS/Windows and Rust 1.89 qualification must remain green.

## 9. Acceptance criteria

C002 closes when:

1. a start-capture failure is reported as a capture failure with the original
   error preserved, and is never reported as `SubjectDrift`;
2. a genuine mid-scan subject change still reports `SubjectDrift`;
3. the CLI read path reports the specific capture cause using the existing
   `subject_unavailable` diagnostic code, matching the closure path;
4. the repository inspection snapshot and the CLI read path use one diagnostic
   vocabulary for the same underlying failure;
5. `InspectionSnapshot::subject_error()` is either meaningfully consumed or
   removed, with the choice recorded;
6. no read that previously failed now succeeds, and no read that previously
   succeeded now fails;
7. the guarded closure finalizer, `SubjectCapture` containment, the frozen
   Git fingerprint bytes, and the check-state vocabulary are unchanged;
8. `architecture/repository.md` and `architecture/cli-control-surface.md`
   distinguish drift from unavailability correctly;
9. native and MSRV qualification pass, with the three named boundary guards
   green.

## 10. Stop conditions

Stop and report rather than improvise if:

- preserving the start-capture error requires changing the guarded closure
  finalizer's capture path or its S1/S2 algorithm;
- preserving the start-capture error requires exposing a public subject-capture
  seam, which Evidence M002 C002 containment forbids;
- the fix would change the frozen Git subject fingerprint bytes, the
  dirty-digest golden fixtures, or the bound-exclusion rules;
- the fix would change the check-state vocabulary or the ten-state check
  contract, which is out of scope here;
- any previously-succeeding read is found to fail for a new reason;
- required verification cannot be obtained honestly.

## 11. Closure evidence required

- implementation commit(s) and PR reference;
- the requirement-to-evidence matrix for every acceptance criterion in §9;
- the recorded before/after diagnostic output for a capture failure on both the
  repository and CLI paths;
- explicit confirmation that the guarded closure finalizer, the fingerprint
  contract, and the check-state vocabulary are unchanged;
- exact verification output, including the MSRV rows, distinguishing planned
  from actually run commands;
- hosted native/MSRV run and job identifiers where available;
- unresolved findings with severity, and roadmap disposition;
- registry updates.

Do not convert a planned command into a passing result because it appears in
§8. If a row was not run, record it as not run or unavailable.

## 12. Handoff notes

- This is a diagnostics-fidelity pass. Do not describe it as a correctness or
  availability fix: no read that previously failed now succeeds, and no
  evidence, trust, or closure behavior changes.
- Record truthfully that the repository inspection snapshot and the CLI read
  path were already fail-closed on this condition. The defect was the loss of
  the cause, not a failure to refuse.
- The `SubjectCapture` containment rule from Evidence M002 C002 binds this
  pass: the fix must not add a public seam through which a subject source can
  be injected.
