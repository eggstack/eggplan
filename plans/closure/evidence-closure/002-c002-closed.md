# Evidence and Closure M002 C002 Closure — Finalization Test-Seam Containment and Closure Evidence Reconciliation

Status: closed

Source plan: plans/implementation/evidence-closure/002-c002-finalization-test-seam-containment-and-closure-evidence-reconciliation.md

Source roadmap: plans/subsystems/evidence-closure-roadmap.md

Reviewed baseline: `5dba200d142cf2e1b58c48f4f08067a2c1f77ae9`; implementation
commit recorded below.

Predecessor closures: plans/closure/evidence-closure/002-closed.md,
plans/closure/evidence-closure/002-c001-closed.md.

## Executive finding

C002 closes. The C001 deterministic `SubjectCapture` seam is no longer
reachable from a downstream crate. `RepositoryStore::finalize_closure`
remains the only supported externally callable guarded closure path; it
continues to capture both subject revisions (S1 and S2) from the repository's
configured `GitSubjectSource` under the repository lock. The
`finalize_closure_with_capture` helper that previously accepted a caller
`SubjectCapture` is `pub(crate)`; the seam types (`SubjectCapture`,
`GitSubjectCapture`, `ScriptedSubjectCapture`) are no longer re-exported
from the `eggplan-repo` crate root, are crate-private, and are reachable
only from the crate-internal `#[cfg(test)]` module that drives the
deterministic stale / drift / capture-failure regressions.

The compile/public-API boundary is mechanically enforced. A crate-level
`compile_fail` doctest in `eggplan-repo/src/lib.rs` fails compilation if any
of the seam types or the alternate finalizer become publicly importable
again. The new `scripts/check-closure-authority-boundary.sh` static guard
provides defense in depth and is wired into the hosted CI native shell
guard step on Linux and macOS. All C001 runtime regressions remain green:
stable A/A still succeeds; A/B still aborts with `ClosureSubjectDrift`; a
scripted capture failure still aborts with `ClosureSubjectCapture`; in
every failure case the Plan revision is unchanged, the Plan is not Closed,
and neither `closure.pending.json` nor `closure.json` exists.

The C001 closure evidence is factually reconciled: the
`<to-be-filled-by-CI>` placeholders are replaced with the immutable
hosted workflow run and job identifiers recorded at that time
(see plans/closure/evidence-closure/002-c001-closed.md). No C001
implementation finding, acceptance conclusion, or historical timestamp
was rewritten.

## Finding-to-evidence matrix

| Finding | Correction | Evidence |
|---|---|---|
| E-M002-C002-01 hidden documentation is not private authority | `SubjectCapture`, `GitSubjectCapture`, `ScriptedSubjectCapture` are removed from the `eggplan-repo` crate-root re-exports; the trait is `pub(crate)`; `GitSubjectCapture` is `pub(crate)`; `ScriptedSubjectCapture` lives only inside the crate-internal `#[cfg(test)] mod tests` in `store.rs`. `finalize_closure_with_capture` is `pub(crate)` and is documented as a crate-internal test hook. | `rg '^[[:space:]]*pub[[:space:]]+(use|fn[[:space:]]+new)'` against `lib.rs`/`store.rs` returns no forbidden re-export; `rg '^[[:space:]]*pub[[:space:]]+fn[[:space:]]+finalize_closure_with_capture'` returns nothing; the seam types remain constructible inside `store.rs` tests only. |
| E-M002-C002-02 C001 closure evidence contained `<to-be-filled-by-CI>` placeholders | Factual erratum in plans/closure/evidence-closure/002-c001-closed.md records the actual hosted run 35998715018 and the four successful job IDs (Linux 107629794960, macOS 107629795068, Windows 107629794971, Rust 1.89 107629794849). The C001 implementation finding, acceptance conclusion, and historical timestamps are preserved unchanged. | plans/closure/evidence-closure/002-c001-closed.md hosted workflow block; the file's status remains `closed`. |
| E-M002-C002-03 no public-API regression prevented authority re-exposure | Crate-level `compile_fail` doctests in `eggplan-repo/src/lib.rs` reference `eggplan_repo::SubjectCapture`, `eggplan_repo::ScriptedSubjectCapture`, and `RepositoryStore::finalize_closure_with_capture`; `cargo test --workspace --doc --locked` runs them as part of the standard workspace test suite, and they succeed only because the symbols are not public. The static `scripts/check-closure-authority-boundary.sh` guard catches accidental re-exports or alternate capture-injected finalizers at static-source level and is invoked by CI. | `cargo test --workspace --doc --locked` passes three compile-fail doctests; `bash scripts/check-closure-authority-boundary.sh` passes; the CI native Linux/macOS jobs run the new guard step and report success. |

## Contained symbols and supported finalization API inventory

Symbols removed from the public `eggplan-repo` surface in commit
`0c0484a18d72f58a1face6f5e749630dfa0c182d`:

- `pub use eggplan_repo::SubjectCapture` (was `#[doc(hidden)] pub`);
- `pub use eggplan_repo::GitSubjectCapture` (was `#[doc(hidden)] pub`);
- `pub use eggplan_repo::ScriptedSubjectCapture` (was `#[doc(hidden)] pub`);
- `RepositoryStore::finalize_closure_with_capture` is no longer `pub`; it
  is `pub(crate)` and is not part of the supported finalization API.

New private / crate-private shape:

- `pub(crate) trait SubjectCapture: Send + Sync`;
- `pub(crate) struct GitSubjectCapture<'a>` with `pub(crate) fn new`;
- `pub(crate) struct ScriptedSubjectCapture` and its `pub(crate) fn new`
  defined only inside the `#[cfg(test)] mod tests` block of
  `crates/eggplan-repo/src/store.rs`;
- `pub(crate) fn finalize_closure_with_capture` on `RepositoryStore`,
  reachable from the crate's own tests only.

Supported external closure API after C002 (unchanged signature, same
behavior, same Rust MSRV requirement):

```text
RepositoryStore::finalize_closure(
    &self,
    candidate: &ClosureCandidate,
    closure_id: ClosureId,
    finalized_at_unix_ms: u64,
) -> Result<(Plan, ClosureRecord), RepoError>
```

No new public type is added. The CLI's diagnostic mapping for the three
subject errors (`closure_subject_changed`, `closure_subject_drifted`,
`closure_subject_unavailable`) is preserved unchanged.

## Authority boundary regression evidence

Crate-level `compile_fail` doctests in `crates/eggplan-repo/src/lib.rs`:

- `use eggplan_repo::SubjectCapture;` — fails because `SubjectCapture` is
  not `pub`.
- `use eggplan_repo::ScriptedSubjectCapture;` — fails because the
  `ScriptedSubjectCapture` type and its constructor live behind
  `#[cfg(test)]` inside the crate.
- `RepositoryStore::finalize_closure_with_capture(...)` with a
  `&dyn eggplan_repo::SubjectCapture` — fails because both the method and
  the trait are crate-private.

`cargo test --workspace --doc --locked` records:

```text
test crates/eggplan-repo/src/lib.rs - (line 15) - compile fail ... ok
test crates/eggplan-repo/src/lib.rs - (line 20) - compile fail ... ok
test crates/eggplan-repo/src/lib.rs - (line 27) - compile fail ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Static boundary guard `scripts/check-closure-authority-boundary.sh` is
wired into `.github/workflows/ci.yml` under the `native` job matrix, with
the same `runner.os != 'Windows'` guard as the existing shell boundary
checks. CI run 36004813178 records success for the new step on Linux and
macOS; Windows skips the shell guards by workflow policy but the compile
failures are part of the workspace doc test that Windows does run.

## Deterministic internal seam regressions

Moved from `crates/eggplan-repo/tests/repository.rs` (where they used
public re-exports of `SubjectCapture` and `ScriptedSubjectCapture`) into
the crate-internal `#[cfg(test)] mod tests` block in
`crates/eggplan-repo/src/store.rs`. The semantic coverage is preserved:

- `closure_subject_stable_between_captures_succeeds`: scripted A/A
  through `finalize_closure_with_capture` returns `Ok((Closed plan,
  ClosureRecord))`, revision advances by exactly one, the stored record
  passes `validate(&closed)`, and `record.candidate.subject` equals the
  scripted subject.
- `closure_subject_drift_between_captures_aborts_with_no_partial_state`:
  scripted first capture returns the candidate subject, second capture
  returns a different subject; finalization fails with
  `RepoError::ClosureSubjectDrift`. The Plan revision is unchanged, the
  Plan remains `Active`, `closure.json` does not exist, and
  `closure.pending.json` does not exist.
- `closure_subject_capture_failure_during_finalizer_aborts_with_no_partial_state`:
  scripted first capture returns the candidate subject, second capture
  returns `GitSubjectError::Unborn`; finalization fails with
  `RepoError::ClosureSubjectCapture(_)` and the same no-partial-state
  assertions hold.

A fourth internal test,
`finalize_closure_does_not_expose_capture_injection_to_external_callers`,
documents the visibility contract from inside the crate: it constructs a
`ScriptedSubjectCapture`, takes a `&dyn SubjectCapture` reference through
the `GitSubjectCapture` adapter, and binds the
`RepositoryStore::finalize_closure_with_capture` function pointer to prove
the seam remains usable from crate-internal tests but cannot be imported
by a downstream crate.

The pre-existing `closure_subject_changed_before_finalizer_aborts_with_no_partial_state`
and `historical_closure_subject_becomes_stale_after_worktree_change`
integration tests in `crates/eggplan-repo/tests/repository.rs` use only
the public `RepositoryStore::finalize_closure` and the `GitSubjectSource`
captured from `RepositoryStore::subject_source()`, so they remain valid
public-API regressions.

## Required C001 regressions preserved

- `closure_subject_changed_before_finalizer_aborts_with_no_partial_state`
  (integration): real worktree mutation between candidate construction
  and `finalize_closure`; typed `ClosureSubjectStale`; Plan stays `Active`
  with original revision; no `closure.json`, no `closure.pending.json`;
  reopen confirms the unchanged state.
- `historical_closure_subject_becomes_stale_after_worktree_change`
  (integration): finalization succeeds using the public
  `finalize_closure`; a later source-file change leaves the closure
  record byte-for-byte unchanged while the current worktree subject
  moves; reopen reports the closure subject as historical evidence.
- `guarded_closure_persists_integrity_and_reopens`: end-to-end guarded
  close + pending recovery + reopen + tamper-detection.
- `concurrent_cancel_and_guarded_close_have_one_cas_winner`
  (CodeGG parity crate): still uses only the public `finalize_closure`
  signature.

All other pre-existing regressions in the workspace remain green: CAS
contention, lock timeout / abandoned staging detection, symlink rejection,
append-only observation ledger, idempotent observation replay, conflict
detection on observation content change, schema-v1 strict rejection of
unknown nested plan or artifact fields, evidence provider-policy and
supersession drift rejection, pending source-side discard, pending
target-side promotion, corrupt pending/final combinations, Git clean /
staged / untracked / submodule dirty fingerprints, managed-state subject
exclusion, CLI subject diagnostic codes, and CodeGG parity adapter golden
cases.

## Schema and compatibility statement

- `Plan` schema v1 and v2: unchanged. Existing fixture bytes and digests
  remain byte-for-byte stable.
- `EvidenceObservation` schema v1 and v2: unchanged.
- `ClosureCandidate` schema: unchanged. The captured subject remains a
  field of the candidate and is no longer passed as a separate authority
  argument.
- `ClosureRecord` schema: unchanged. Existing closure files remain valid
  and are reopened byte-for-byte.
- `Plan` storage envelope (`storage_version: 1`, `plan_digest`, `plan`):
  unchanged.
- Repository layout under `.eggplan/`: unchanged. The pending/final
  closure file semantics and recovery protocol are unchanged.
- `RepositoryStore::finalize_closure` signature: unchanged from C001.
  Only the visibility of an alternate capture-injected helper is reduced.
- `RepoError::ClosureSubjectStale`, `RepoError::ClosureSubjectDrift`,
  `RepoError::ClosureSubjectCapture(GitSubjectError)`: unchanged in
  shape, message, and CLI diagnostic mapping.

No persisted schema requires a version bump. The C002 pass is a
containment pass; it removes an accidental, undocumented public test seam
that Eggplan never declared as a stable contract.

## Verification executed

Local Linux commands passed at the implementation commit
`0c0484a18d72f58a1face6f5e749630dfa0c182d`:

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked                 # 99 passed (18 suites)
cargo test --workspace --doc --locked           # 3 passed (6 suites, includes 3 compile_fail)
cargo +1.89.0 check --workspace --all-targets --locked
cargo +1.89.0 test --workspace --locked         # 99 passed (18 suites)
bash scripts/check-core-boundary.sh
bash scripts/check-codegg-compat-boundary.sh
bash scripts/check-integrations-boundary.sh
bash scripts/check-projection-cli-boundary.sh
bash scripts/check-closure-authority-boundary.sh
git diff --check
```

Hosted native and MSRV workflow for the implementation commit:

- Run: https://github.com/eggstack/eggplan/actions/runs/36004813178
- Linux (ubuntu-latest) job: 107650129619 — pass (format, check, clippy,
  tests, all five boundary guards including the new
  `check-closure-authority-boundary.sh`).
- macOS (macos-latest) job: 107650129837 — pass (check, clippy, tests,
  all five boundary guards; `cargo fmt` is skipped by workflow policy on
  macOS).
- Windows (windows-latest) job: 107650129154 — pass (check, clippy,
  tests; `cargo fmt` and the shell boundary guards are skipped by
  workflow policy on Windows; the compile-fail doctest still runs as
  part of `cargo test --workspace --doc --locked`).
- Rust 1.89 (msrv) job: 107650129645 — pass (check and tests on
  `ubuntu-latest`).

## Invariant, failure, and compatibility review

- The supported external closure path is exactly
  `RepositoryStore::finalize_closure`. There is no public alternate
  finalizer that accepts an injected subject authority.
- The `SubjectCapture` seam types live entirely inside `eggplan-repo` and
  are not part of any external API. A downstream crate cannot implement a
  closure subject capture authority, cannot provide a `SubjectCapture`
  instance to a finalizer, and cannot call an alternate finalizer that
  accepts injected subject state.
- The `Git`/filesystem dependency remains inside `eggplan-repo`.
  `eggplan-core` does not grow a Git capture dependency; the
  capture-injected abstract `SubjectCapture` trait is implemented only
  inside `eggplan-repo` for crate-internal test use.
- The corrected double-capture protocol (S1 before assessment replay, S2
  immediately before the pending write) is unchanged and remains a typed
  failure with no partial closure state.
- The supported public `finalize_closure` signature is unchanged; only
  the visibility of an alternate capture-injected helper is reduced.
- The CLI's stable machine diagnostics for subject errors are unchanged.
- `.eggplan` administrative writes still do not perturb the Git subject;
  the managed-state exclusion introduced by Foundation M003 still applies.
- Evidence evidence and supersession lineage remain immutable / append-only.
- Provider policy remains explicit host authority; the candidate's bounded
  provider-policy snapshot is still the historical authority validated at
  finalization.
- Pending closure recovery remains crash-consistent; the corrected
  double-capture protocol applies only before the first pending write.
- No scheduler, executor, remote evidence acquisition, policy DSL,
  attestation signer, or generic artifact store was added.

## Unresolved findings

None within C002 scope. The hosted Windows native runner continues to
execute the compile-fail authority regression through the workspace doc
test step; Windows shell boundary guards remain out of scope by the same
workflow policy that already governs the other boundary guards.

## Roadmap disposition

Evidence M002 C002 closes. The historical M002 and M002 C001 closures
remain preserved.

- Evidence / Closure subsystem: closed / current.
- Registered corrective gate from `plans/registry.md`: removed.
- Projection/CLI M002 — Markdown import/render: unblocked. It still
  needs a fresh bounded implementation plan and a CodeGG sibling
  interface recheck before handoff.
- CodeGG Integration M002 — staged core adoption: unblocked. It still
  needs a fresh CodeGG interface recheck before planning.
- Eggstack Integrations M002 — Eggwork/Eggsearch adapters: unblocked.
  It still needs fresh Eggwork and Eggsearch interface rechecks before
  planning.

Interop / distribution remains deferred.

## Registry updates

- Evidence M002 C002: closed.
- Evidence / Closure subsystem: closed / current.
- Projection/CLI M002, CodeGG Integration M002, Eggstack Integrations
  M002: ready to plan. Each still requires a registered bounded
  implementation plan and a fresh sibling-interface recheck before
  handoff.
- Registered corrective gate: removed.

### Factual erratum (2026-09-24)

The original C002 implementation SHA cited above was copied incorrectly. The
correct implementation commit is
`0c0484a18d72f58a1face6f5e749630dfa0c182d`, matching hosted run
36004813178. This repairs the citation only; the original closure status,
findings, hosted job evidence, and acceptance conclusions are unchanged.

## Plan follow-through summary

Every required work package in the source plan landed:

- WP1 — Contain the capture seam: done. The seam types are no longer
  re-exported from the crate root; `SubjectCapture` and
  `GitSubjectCapture` are `pub(crate)`; `ScriptedSubjectCapture` is
  defined only inside the crate-internal `#[cfg(test)]` module;
  `finalize_closure_with_capture` is `pub(crate)`.
- WP2 — Preserve deterministic race coverage: done. The scripted S1/S2
  stable / drift / capture-failure regressions live inside the crate
  without semantic loss.
- WP3 — Add public authority-boundary regression: done. Crate-level
  `compile_fail` doctests plus `check-closure-authority-boundary.sh`,
  wired into CI.
- WP4 — Reconcile C001 closure evidence: done. Placeholders replaced
  with run 35998715018 and the four successful job IDs; an erratum note
  documents the update under C002.
- WP5 — Qualify C002 and close honestly: done. Local verification
  recorded, hosted run 36004813178 completed successfully on all four
  lanes, and this closure record cites those identifiers without
  invented placeholder evidence.
