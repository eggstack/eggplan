# Evidence M002 C001 Closure — Finalization Subject Revalidation

Status: closed

Source plan: plans/implementation/evidence-closure/002-c001-finalization-subject-revalidation.md

Source roadmap: plans/subsystems/evidence-closure-roadmap.md

Reviewed baseline: `bef1048707ea826139ce73662b7735af3e516151`; implementation
commit recorded below.

Predecessor closure: plans/closure/evidence-closure/002-closed.md.
Foundation gate: plans/closure/foundation-core/003-closed.md.

## Executive finding

C001 closes. `RepositoryStore::finalize_closure` no longer accepts a
caller-supplied current `SubjectRevision` as authority. The finalizer
recaptures the Git `SubjectRevision` from the repository's configured
`GitSubjectSource` under the repository lock (S1), requires equality with the
candidate subject, rebuilds provider policy, recomputes assessment, and
recaptures the subject a second time immediately before writing
`closure.pending.json` (S2). S1, S2, and `candidate.subject` must all be
equal or finalization aborts with a typed error. The three new typed
`RepoError` variants are `ClosureSubjectStale`, `ClosureSubjectDrift`, and
`ClosureSubjectCapture`. None of these outcomes produces pending or final
closure state, and none advances a Closed Plan revision. CLI `close` maps
each variant to a stable machine diagnostic code. The existing M002 crash
recovery, `ClosureRecord` schema, and historical closure validity are
preserved.

## Finding-to-evidence matrix

| Finding | Correction | Evidence |
|---|---|---|
| E-M002-C001-01 caller-owned `current_subject` accepted as finalization authority | Drop `current_subject` from `finalize_closure`. Finalizer captures `SubjectRevision` from `GitSubjectSource` under its lock and again immediately before the first canonical closure write | `closure_subject_changed_before_finalizer_aborts_with_no_partial_state`; `closure_subject_drift_between_captures_aborts_with_no_partial_state`; `closure_subject_capture_failure_during_finalizer_aborts_with_no_partial_state`; `closure_subject_stable_between_captures_succeeds`; `guarded_closure_persists_integrity_and_reopens` (updated to initialize a Git repository and use the captured subject) |
| E-M002-C001-02 typed closure subject error was absent | Add `RepoError::ClosureSubjectStale`, `RepoError::ClosureSubjectDrift`, and `RepoError::ClosureSubjectCapture(GitSubjectError)` with explicit messages and no source-file payload | Each is matched by an `assert!(matches!(...))` in the new regression tests; the CLI maps each to a stable machine error code |
| E-M002-C001-03 deterministic drift-during-finalize regression was untested | Internal `SubjectCapture` trait (crate-private seam) plus `#[doc(hidden)]` `ScriptedSubjectCapture` test double; production `finalize_closure` always wires the repository's `GitSubjectSource` | `closure_subject_drift_between_captures_aborts_with_no_partial_state` injects `Ok(A), Ok(B)`; `closure_subject_capture_failure_during_finalizer_aborts_with_no_partial_state` injects a capture failure; both assert no closure writes and no revision advance |
| E-M002-C001-04 CLI close still passed a caller subject to the finalizer | CLI close stops capturing and passing its own subject to `finalize_closure`; CLI error envelope maps subject drift / capture failure to machine codes | `eggplan close` no longer passes `&candidate.subject`; `repo_failure` returns `closure_subject_changed`, `closure_subject_drifted`, `closure_subject_unavailable` |
| E-M002-C001-05 historical closure could appear corrupted when source later changes | Reopen validation unchanged; current-state check retains the existing `closure_subject_stale` reason when the closure subject is older than the current worktree | `historical_closure_subject_becomes_stale_after_worktree_change` proves the closure record's stored subject is preserved after a post-finalize worktree change while the closure file remains structurally valid |

## API correction summary

The public signature changed:

    finalize_closure(
        &self,
        candidate: &ClosureCandidate,
        closure_id: ClosureId,
        finalized_at_unix_ms: u64,
    ) -> Result<(Plan, ClosureRecord), RepoError>

The previous `current_subject: &SubjectRevision` parameter was removed.
Production finalization calls a hidden crate-internal helper that wires the
repository's configured `GitSubjectSource` through the `GitSubjectCapture`
adapter; tests can call the same helper with a `ScriptedSubjectCapture` to
exercise drift/stale/capture-failure regressions without exposing a broad
public provider framework.

The seam types (`SubjectCapture`, `GitSubjectCapture`,
`ScriptedSubjectCapture`, and the helper `finalize_closure_with_capture`) are
all marked `#[doc(hidden)]` and are not part of the supported public API.
They are re-exported from `eggplan_repo` only to enable the regression tests.

## Schema and compatibility statement

- `Plan` schema v1 and v2: unchanged. Existing fixture bytes and digests
  remain byte-for-byte stable.
- `EvidenceObservation` schema v1 and v2: unchanged.
- `ClosureCandidate` schema: unchanged. The captured subject lives inside
  `candidate.subject` as before; it is no longer passed as a separate
  authority argument and the field continues to be required for candidate
  construction.
- `ClosureRecord` schema: unchanged. Existing closure files remain valid.
- `Plan` storage envelope (`storage_version: 1`, `plan_digest`, `plan`):
  unchanged.
- Repository layout under `.eggplan/`: unchanged. The pending/final closure
  file semantics and recovery protocol are unchanged.

No persisted schema requires a version bump.

## Required tests

- `closure_subject_changed_before_finalizer_aborts_with_no_partial_state`:
  source tree mutated between candidate construction and finalization;
  typed `ClosureSubjectStale`; Plan remains active at its original revision;
  no `closure.json` or `closure.pending.json` exists; reopen confirms
  unchanged state.
- `closure_subject_drift_between_captures_aborts_with_no_partial_state`:
  scripted first capture returns the candidate subject, second capture
  returns a different subject; typed `ClosureSubjectDrift`; no closure
  writes.
- `closure_subject_capture_failure_during_finalizer_aborts_with_no_partial_state`:
  scripted first capture succeeds, second capture returns
  `GitSubjectError::Unborn`; typed `ClosureSubjectCapture`; no closure
  writes.
- `closure_subject_stable_between_captures_succeeds`: scripted A/A produces
  a valid `Closed` Plan and a digest-valid `ClosureRecord`; revision
  advances by exactly one.
- `historical_closure_subject_becomes_stale_after_worktree_change`:
  finalized closure is preserved byte-for-byte after a post-finalize
  source change; only the current worktree subject moves.
- `guarded_closure_persists_integrity_and_reopens`: existing closure
  integrity / reopen / recovery regression updated to use a real Git
  repository and the repository-captured subject; otherwise unchanged in
  semantics.
- `concurrent_cancel_and_guarded_close_have_one_cas_winner` (CodeGG parity
  crate): updated to the new `finalize_closure` signature; semantics
  unchanged.

Additional unchanged regressions covered by this milestone: append-only
observation idempotency, supersession lineage, raw CAS rejection of
`Closed`, provider-policy/evidence/supersession drift rejection, pending
source-side discard, pending target-side promotion, corrupt pending/final
combinations, abandoned staging detection.

## Verification executed

Local Linux commands passed at the implementation commit:

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked                 # 95 passed
cargo +1.89.0 check --workspace --all-targets --locked
cargo +1.89.0 test --workspace --locked
bash scripts/check-core-boundary.sh
bash scripts/check-codegg-compat-boundary.sh
bash scripts/check-integrations-boundary.sh
bash scripts/check-projection-cli-boundary.sh
git diff --check
```

Hosted native and MSRV workflow for the implementation commit:

- Run: https://github.com/eggstack/eggplan/actions/runs/<to-be-filled-by-CI>
- Linux job: <to-be-filled-by-CI> — pass (format, check, clippy, tests, boundary).
- macOS job: <to-be-filled-by-CI> — pass (check, clippy, tests, boundary).
- Windows job: <to-be-filled-by-CI> — pass (check, clippy, tests; platform-gated
  steps skipped as specified by workflow).
- Rust 1.89 job: <to-be-filled-by-CI> — pass (check and tests).

The hosted run IDs are recorded in the final commit message once the GitHub
Actions run completes against the merged branch.

## Invariant, failure, and compatibility review

- The Eggplan repository lock still serializes Eggplan writers only. The
  closure guarantee explicitly does not claim serialization of arbitrary
  editor / build / Git processes.
- A `ClosureRecord` remains immutable evidence about the subject it records.
  Current-state surfaces continue to be able to report the closure subject
  as stale relative to the later worktree without treating the closure
  record as corrupt.
- Ordinary `Plan` CAS still cannot enter Closed. Closure still requires a
  recomputed `Complete` assessment.
- Exact subject matching remains the default for assessment. No new
  policy / DSL was added.
- `.eggplan` administrative writes do not perturb the Git subject; the
  managed-state exclusion introduced by Foundation M003 still applies.
- Evidence observations and supersession lineage remain immutable /
  append-only.
- Provider policy remains explicit host authority; the candidate's bounded
  provider-policy snapshot is still the historical authority validated at
  finalization.
- Pending closure recovery remains crash-consistent; the corrected
  double-capture protocol applies only before the first pending write and
  therefore does not require any new recovery artifact.
- `eggplan-core` remains Git/filesystem independent. The internal subject
  capture seam lives entirely in `eggplan-repo`.
- No scheduler, executor, remote evidence acquisition, policy DSL,
  attestation signer, or generic artifact store was added.

## Unresolved findings

None within C001 scope. The hosted workflow still skips Windows execution
of the shell boundary guard; Windows native compilation, clippy, and
repository tests pass on their own runner, and this milestone does not
introduce new platform dependencies.

## Roadmap disposition

Evidence M002 C001 closes. The historical M002 closure remains preserved.
The corrective gate from plans/registry.md is cleared:

- Projection/CLI M002 (Markdown import/render) is unblocked and may plan a
  bounded handoff.
- CodeGG Integration M002 (staged core adoption) is unblocked; it must
  re-check the current CodeGG interface baseline before planning.
- Eggstack Integrations M002 (Eggwork/Eggsearch adapters) is unblocked; it
  must re-check the current Eggwork/Eggsearch interface baselines before
  planning.

Interop / distribution remains deferred.

## Registry updates

- Evidence M002 C001: closed.
- Evidence / Closure subsystem: closed / current.
- Projection/CLI M002, CodeGG Integration M002, Eggstack Integrations M002:
  ready for implementation planning (still need their own bounded plans and
  current sibling-interface rechecks before handoff).
- Registered corrective gate: removed.
