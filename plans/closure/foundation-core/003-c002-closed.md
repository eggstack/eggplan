# Foundation M003 C002 — Closed

Status: closed

Source plan: plans/implementation/foundation-core/003-c002-subject-capture-error-fidelity.md

Source roadmap: plans/subsystems/foundation-core-roadmap.md

Reviewed baseline: `37ad290ca6b971a68d2871aedb39f4882f4eb2a8`.

Implementation state: the implementation is complete in the working tree and
was not committed by the coordinator. The commit identifier is to be added to
this record as a factual amendment once the change is committed, following the
precedent of `plans/closure/evidence-closure/002-c004-closed.md`.

## Executive finding

Foundation M003 C002 closes. The Git subject capture error is now carried from
the point of failure to the point of reporting instead of being discarded at
the boundary.

On the repository inspection path, the start capture is carried as a `Result`
across the `store.rs` → `snapshot.rs` boundary
(`crates/eggplan-repo/src/snapshot.rs:309`), and the terminality logic at
`crates/eggplan-repo/src/snapshot.rs:359-373` reports four distinct
conditions. A failed start capture is reported as
`RepoError::InspectionSubject(original_error)`
(`crates/eggplan-repo/src/snapshot.rs:367`) — a capture failure, never
`SubjectDrift`. `SubjectDrift` is now reserved for the single condition it
actually describes: both captures succeeded and disagreed. The
both-captures-failed case stays soft, so the CLI's `current_subject_unavailable`
warning behavior is unchanged, and the snapshot records the original cause.

On the CLI read path, `summary_context`
(`crates/eggplan-cli/src/lib.rs:1651`) no longer discards the capture error
with `.ok()`. It maps the failure to the existing `subject_unavailable`
diagnostic code with the underlying message, matching the closure path
(`crates/eggplan-cli/src/lib.rs:1670` and `:1718`). The `current_subject_unavailable`
warning token is unchanged and now carries the specific cause
(`crates/eggplan-cli/src/lib.rs:637`).

**One user-visible behavior change is recorded explicitly.** The `show` command
on a state root whose subject cannot be captured previously exited 0 with a
subjectless summary and no diagnostic at all; it now exits 2 with
`subject_unavailable` and the underlying cause. This is the diagnostics-fidelity
change this corrective exists to make, and it is the point at which the plan's
§6 and §9.6 ("no read that previously succeeded now fails") and its §9.3
("the CLI read path reports the specific capture cause using the existing
`subject_unavailable` diagnostic code") came into conflict. The acceptance
criterion was followed, and the conflict is recorded here rather than left
implicit. See "Unresolved findings" #1.

## Plan errata

The plan's §5 "preferred shape" contained a bullet asking that the
"unavailable at the start and available at the end" case be reported as drift
"with the original capture error recorded alongside". That bullet contradicted
the plan's own §9.1 ("a start-capture failure ... is never reported as
`SubjectDrift`") and §7 (which requires the original cause to be preserved for
exactly that case), and it was not implementable: `RepoError::SubjectDrift`
carries no payload, and the plan forbids adding a `RepoError` variant.

The plan was amended during implementation to state the implemented behavior.
The amendment is recorded inline in
`plans/implementation/foundation-core/003-c002-subject-capture-error-fidelity.md`
and is marked as such in the plan text. No closed plan was rewritten; this plan
was open and unstarted when the amendment was made.

## Requirement-to-evidence matrix

| # | Acceptance criterion (plan §9) | Evidence |
|---|---|---|
| 1 | A start-capture failure is reported as a capture failure with the original error preserved, and is never reported as `SubjectDrift` | `crates/eggplan-repo/src/snapshot.rs:367`; tests `a_failed_start_capture_is_reported_as_a_capture_failure_not_drift` (`crates/eggplan-repo/src/snapshot.rs:630`) and `a_failed_start_capture_keeps_its_cause_in_the_error_source_chain` (`crates/eggplan-repo/src/snapshot.rs:670`), which asserts `!matches!(error, RepoError::SubjectDrift)` and downcasts `Error::source` to `GitSubjectError` |
| 2 | A genuine mid-scan subject change is still reported as drift | `crates/eggplan-repo/src/snapshot.rs:361-364`; test `subject_drift_during_inspection_fails_closed` (`crates/eggplan-repo/src/snapshot.rs:580`) |
| 3 | The CLI read path reports the specific capture cause under the existing `subject_unavailable` code, matching the closure path | `crates/eggplan-cli/src/lib.rs:1670` (read path) and `:1718` (closure path); test `show_reports_an_unavailable_subject_with_the_specific_cause` (`crates/eggplan-cli/tests/subject_unavailable.rs:95`) |
| 4 | The repository inspection path and the CLI read path use one diagnostic vocabulary for the same underlying failure | `crates/eggplan-cli/src/lib.rs:2049-2054` maps `RepoError::InspectionSubject(_)` to `subject_unavailable` and now includes the cause in the message; the read path and closure path use the same code |
| 5 | `InspectionSnapshot::subject_error()` is either meaningfully consumed or removed, with the choice recorded | Kept and consumed at `crates/eggplan-cli/src/lib.rs:745`, `:899`, `:1094`, `:1628`; the value is asserted by `inspection_snapshot_records_the_capture_cause_when_the_subject_is_unavailable` (`crates/eggplan-repo/tests/repository.rs:1337`) |
| 6 | No read that previously failed now succeeds; no read that previously succeeded now fails | **Partially satisfied, by design.** The repository read path still fails closed in every previously-failing case. The CLI `show` read path now fails where it previously returned a subjectless summary with no diagnostic — the plan-mandated change from §9.3. See "Unresolved findings" #1 |
| 7 | The finalizer, `SubjectCapture` containment, the fingerprint bytes, the check-state vocabulary, and the ten-state check contract are unchanged | `SubjectCapture` remains `pub(crate)` (`crates/eggplan-repo/src/store.rs:129`); `finalize_closure_with_capture` remains `pub(crate)` (`crates/eggplan-repo/src/store.rs:384`); `crates/eggplan-repo/tests/fixtures/` is byte-identical (`git diff --stat` shows no change to any fixture); the CLI's literal check-state strings are unchanged |
| 8 | The architecture record distinguishes drift from unavailability | `architecture/repository.md` — "Repository inspection read path"; `architecture/cli-control-surface.md` — "Current-subject diagnostics"; `architecture/repository-inspection-snapshot.md` — the terminality table was corrected, because it still described the removed drift conflation |
| 9 | Native qualification, MSRV qualification, and the boundary guards green | See "Verification executed". Hosted macOS/Windows native runs were **not** performed — see "Unresolved findings" #2 |

## Production implementation evidence

### Repository: the capture error is carried, not dropped

`crates/eggplan-repo/src/store.rs:1171` captures the start subject into a
`Result` and hands it to the sealer at `:1236`. The terminality logic at
`crates/eggplan-repo/src/snapshot.rs:359-373` is:

```rust
let (subject, subject_error) = match (subject_at_start, subject_at_end) {
    (Ok(start), Ok(end)) => {
        if start != end {
            return Err(RepoError::SubjectDrift);
        }
        (Some(start), None)
    }
    (Ok(_), Err(error)) => return Err(RepoError::InspectionSubject(error)),
    (Err(error), Ok(_)) => return Err(RepoError::InspectionSubject(error)),
    (Err(error), Err(_)) => {
        (None, Some(error.to_string()))
    }
};
```

The pre-existing `RepoError::InspectionSubject(#[source] GitSubjectError)`
variant is reused. No `RepoError` variant was added.

### CLI: one diagnostic vocabulary

`crates/eggplan-cli/src/lib.rs:1670` — the read path:

```rust
.map_err(|error| failure(command, "subject_unavailable", error.to_string(), json))?;
```

`crates/eggplan-cli/src/lib.rs:1718` — the unchanged closure path.

`crates/eggplan-cli/src/lib.rs:637` — `subject_unavailable_warning`, which
keeps the warning token exactly `current_subject_unavailable` and appends the
cause when the snapshot carries one. The `reason_codes` entry at
`crates/eggplan-cli/src/lib.rs:1101` is still exactly
`current_subject_unavailable`; the cause is carried in the warning text only.

### Before and after diagnostic output

Baseline extracted read-only from `37ad290` with `git archive` into a temporary
directory; the repository's own git state was not touched.

**Repository path — start capture fails, end capture succeeds.**

- BEFORE: `Err(RepoError::SubjectDrift)`, rendered as
  `git subject drifted during repository inspection`. The original
  `GitSubjectError` was discarded at the `.ok()` call and never reported.
- AFTER: `Err(RepoError::InspectionSubject(GitSubjectError::NotGit(_)))`,
  rendered as
  `git subject capture failed during repository inspection: path is not inside a Git worktree: <root>`,
  with `source` = `path is not inside a Git worktree: <root>` and
  `is_drift = false`.
- Soft path (both captures fail), read through the public API:
  `subject = None`, `subject_error = Some("path is not inside a Git worktree: <root>")`.

**CLI path — state root outside a Git worktree.**

`eggplan --state-root <non-git>/.eggplan show ep_demo --json`

- BEFORE: `"ok": true`, `"current_subject": null`, `"error": null`,
  `"warnings": []`, exit 0.
- AFTER:

  ```json
  { "schema_version": 1, "command": "show", "ok": false, "data": null,
    "error": { "code": "subject_unavailable",
               "message": "path is not inside a Git worktree: <root>" },
    "warnings": [], "warnings_truncated": false }
  ```

  exit 2.

`eggplan --state-root <non-git>/.eggplan check --json` — soft path, vocabulary
unchanged:

- BEFORE: `"state": "unavailable"`,
  `"reason_codes": ["current_subject_unavailable"]`,
  `"warnings": ["current_subject_unavailable:ep_demo"]`, exit 0.
- AFTER: `"state": "unavailable"`,
  `"reason_codes": ["current_subject_unavailable"]` (unchanged),
  `"warnings": ["current_subject_unavailable:ep_demo: path is not inside a Git worktree: <root>"]`,
  exit 0.

Same warning count, same warning token, same reason-code list.

### Tests

| Test | File | Purpose |
|---|---|---|
| `a_failed_start_capture_is_reported_as_a_capture_failure_not_drift` | `crates/eggplan-repo/src/snapshot.rs:630` | A start-capture failure is not drift |
| `a_failed_start_capture_keeps_its_cause_in_the_error_source_chain` | `crates/eggplan-repo/src/snapshot.rs:670` | The original `GitSubjectError` is recoverable through `Error::source` |
| `subject_drift_during_inspection_fails_closed` | `crates/eggplan-repo/src/snapshot.rs:580` | Genuine drift still fails closed |
| `inspection_snapshot_records_the_capture_cause_when_the_subject_is_unavailable` | `crates/eggplan-repo/tests/repository.rs:1337` | The soft path records the cause and exposes it |
| `show_reports_an_unavailable_subject_with_the_specific_cause` | `crates/eggplan-cli/tests/subject_unavailable.rs:95` | Read path: `subject_unavailable` plus the cause |
| `the_closure_path_reports_subject_unavailable_unchanged` | `crates/eggplan-cli/tests/subject_unavailable.rs:138` | Closure path unchanged |
| `the_current_subject_unavailable_warning_carries_the_cause_and_keeps_its_token` | `crates/eggplan-cli/tests/subject_unavailable.rs:167` | Warning token and reason-code list unchanged; cause carried |

## Verification executed

Run by the coordinator against the integrated working tree:

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo check --workspace --all-targets --locked` | pass |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | pass, no warnings |
| `cargo test --workspace --locked` | pass — 312 passed, 0 failed, 3 ignored (pre-existing characterization tests) |
| `bash scripts/check-core-boundary.sh` | pass |
| `bash scripts/check-codegg-compat-boundary.sh` | pass |
| `bash scripts/check-integrations-boundary.sh` | pass |
| `bash scripts/check-projection-cli-boundary.sh` | pass |
| `bash scripts/check-closure-authority-boundary.sh` | pass |
| `cargo +1.89.0 check --workspace --all-targets --locked` | pass |
| `cargo +1.89.0 test --workspace --locked` | pass — 312 passed, 0 failed, 3 ignored |

Run by the implementing agent, scoped to the crates it owned:

| Command | Result |
|---|---|
| `cargo check -p eggplan-repo --all-targets --locked` | pass |
| `cargo test -p eggplan-repo --locked` | pass — 85 passed, 0 failed, 3 ignored (pre-existing) |
| `cargo clippy -p eggplan-repo --all-targets --locked -- -D warnings` | pass |
| `cargo check -p eggplan-cli --all-targets --locked` | pass |
| `cargo test -p eggplan-cli --locked` | pass — 42 passed, 0 failed |
| `cargo clippy -p eggplan-cli --all-targets --locked -- -D warnings` | pass |
| `cargo fmt -p eggplan-repo -p eggplan-cli -- --check` | pass |

**Not run:** the hosted CI matrix on ubuntu, macOS, and Windows native
runners. The local evidence is Linux-native plus the Rust 1.89.0 MSRV job. CI
job identifiers must be added to this record as an amendment once the change is
pushed.

## Invariant review

- **The read still fails closed.** Every case that failed before still fails.
  No failure was converted into a success.
- **The subject remains unverifiable without a capture.** A snapshot without a
  subject still reports `subject: None`; the soft path is unchanged.
- **The closure finalizer is untouched.** No change to `finalize_closure`,
  `finalize_closure_with_capture`, `SubjectCapture`, or the guarded finalizer's
  containment rules.
- **The Git subject identity contract is untouched.** The fingerprint bytes,
  row ordering, status bits, index-entry treatment, default bounds, and the
  pinned golden fixtures under `crates/eggplan-repo/tests/` are byte-identical.
  `git diff --stat` reports no change to any fixture file.
- **The check-state vocabulary is untouched.** Ten states, plus the
  `closure_subject_stale` reason code and the check-time `stale` state. The
  change touches only warning *text* and the `subject_unavailable` message.

## Failure, recovery, and compatibility review

- **Recovery:** the repository's recovery protocol is untouched. A pending
  closure still requires explicit `--recover-pending`.
- **Contention:** the lock protocol, the exclusive/shared lock split, the lock
  timeout, and abandoned staging are untouched.
- **Compatibility:** the only observable compatibility change is the `show`
  diagnostic, recorded above and in "Unresolved findings" #1. The JSON envelope
  schema version is unchanged; no field was added, removed, or renamed.

## Security, trust, and path review

- **Trust:** a read can no longer present a subjectless result as a complete
  one. Previously, a failed subject capture on the `show` path produced a
  successful response whose `current_subject` was `null`; the reader had no
  signal that the subject was unavailable. Now the read fails with the
  diagnostic code `subject_unavailable` and the underlying cause. This closes a
  gap against `architecture/cli-control-surface.md`, which requires that a read
  report that the subject is unavailable.
- **Provider trust:** unchanged.
- **Paths:** no path handling was added or changed. The captured cause may name
  a path, which was already the case for the closure path's
  `subject_unavailable` message.

## Documentation and operations

- `architecture/repository.md` — new sections distinguishing drift from
  unavailability and describing the inspection read path.
- `architecture/cli-control-surface.md` — new section "Current-subject
  diagnostics".
- `architecture/repository-inspection-snapshot.md` — the terminality table was
  corrected. It previously stated that an "unavailable → available" pair
  yields `SubjectDrift`, which the implementation no longer does; leaving it
  would have contradicted the code.
- `architecture/deep-dive-repository.md` — the inspection snapshot's
  subject-error handling described, and six stale citations re-verified.
- `docs/getting-started.md` — the statement that "`init`, `new`, `show`, and
  `status` work without one" was corrected. `show` now requires a resolvable
  Git subject and exits nonzero with `subject_unavailable` when the capture
  fails. The corrected statement was verified empirically on an unborn branch:
  `show` exits 2 with `subject_unavailable`, while `status` still exits 0 with
  the `current_subject_unavailable` warning.
- `docs/cli-reference.md` — the `assess`/`close` subject requirement now states
  that `show` requires a resolvable subject too, and names the commands that
  do not require one.

## Unresolved findings

| # | Severity | Finding | Disposition |
|---|---|---|---|
| 1 | medium | The `show` read path now fails where it previously succeeded, with `subject_unavailable` and exit 2. This satisfies §9.3 and the CLI rule against presenting a subjectless result as a complete one, and it contradicts §6/§9.6 of the same plan. | Accepted as the intended diagnostics change and recorded here, as required by the plan. Consumers that relied on `show` exiting 0 with a null `current_subject` will see a different outcome. A follow-up corrective is **not** proposed: this is the behavior the plan required. |
| 2 | medium | The hosted CI matrix (ubuntu, macOS, Windows native) was not run. | Must be run by CI after the change is pushed; record the job identifiers here as an amendment. |
| 3 | low | The implementation is in the working tree and is not yet committed. | The commit identifier is to be appended after the commit. |
| 4 | low | `architecture/deep-dive-repository.md` retains further pre-existing stale citations outside the paragraphs this pass touched (for example the crate's public-surface range and several `store.rs` ranges). | Pre-existing debt, out of this corrective's scope. Not repaired here so that the scope of this record stays exactly what the plan covered. |
| 5 | informational | In the both-captures-failed soft case, the snapshot records the *start* capture error and discards the end capture error. | Intentional: the start capture is the original point of failure. In the realistic both-fail scenario both are the same `NotGit` error, so the observable warning text is unchanged. Recorded so the choice is not mistaken for an oversight. |

## Roadmap disposition

Foundation-core roadmap M003 corrective line C002 is closed. The
"M003 corrective lines" table in `plans/subsystems/foundation-core-roadmap.md`
is updated to record this closure and its closure record.

The finding that opened this corrective — a swallowed `GitSubjectError`
reported as an unrelated drift diagnosis, and a `.ok()` in the CLI read path
reported as a generic warning — is closed on both paths.

## Registry updates

- `plans/registry.md`: the Foundation M003 C002 row in *Registered
  implementation plans* is updated to `closed`, with a pointer to
  `plans/closure/foundation-core/003-c002-closed.md`.
- `plans/registry.md`: the *Open correctives raised by the 2026-10-23
  repository audit* subsection loses its C002 entry, which moves to the closed
  list.
- `plans/subsystems/foundation-core-roadmap.md`: M003 corrective line C002 is
  marked closed.
- No historical closure record was rewritten. The only plan text amended was
  the open, unstarted C002 plan, and the amendment is marked inline.
