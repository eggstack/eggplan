# CodeGG Integration M002 Closure Record

Status: closed

Source plan: plans/implementation/codegg-integration/002-staged-eggplan-assessment-adoption.md

Source roadmap: plans/subsystems/codegg-integration-roadmap.md

Repository baseline: `0d4a6af7adc6f80f975aca1bfe9bae04e2eb27d8` (the Eggplan head CodeGG pinned)

Eggplan bridge implementation commits:

- `088968bd58680ae2b3741e2f1feb0614e0ff81a0` — live assessment bridge, live/fixture path split, refreshed fixture baseline.
- `1291799a72d2ab5f2d36c75cec0d40eeb4c5d347` — M002 §13 test-matrix completion and architecture refresh.

Reviewed CodeGG baseline: `ffa1c15e654776c3ebe1022f4ce7de2582bc5d98` (`dbowm91/codegg` `main`)

Pinned Eggplan dependency revision used by CodeGG:
`0d4a6af7adc6f80f975aca1bfe9bae04e2eb27d8`

CodeGG adoption commits: `3e992291b6024fae29f2512ad2fa42a209fe7ca9`
(implementation) + `3c7438c738a142fdd0c4e79684d7d1154fe52a99` (test-hermeticity
fix found during qualification, part of the qualified tree)

CodeGG closure commit: `ffa1c15e654776c3ebe1022f4ce7de2582bc5d98`
(`dbowm91/codegg` `plans/closure/eggplan-assessment-integration/003-m002-status.md`)

## Executive finding

M002 is closed. All nine acceptance criteria are met, and no stop condition
fired.

Eggplan shipped the pure side of staged adoption in `088968b`: the live mapper
is split from the fixture wrapper, source SHA is qualification provenance rather
than runtime authority, `eggplan-repo` left the production dependency graph, and
`assess_codegg_snapshot` provides a pure deterministic assessment view with no
persistence. CodeGG shipped the consuming side in `3e992291`: an
application-layer facade that captures the exact current subject, derives
verification digests from authoritative native execution specifications through
Eggplan's own `digest_json`/`VerificationDigest` helpers, resolves host evidence,
and folds the result back into the unchanged `WorkPlanCompletionAssessment`
surface. The universal parity rule held across 28 differential cases: Eggplan
allows completion only where the legacy host-owned rule already allowed it.

Ownership did not move. `crates/codegg-core/src/work_plan/` is byte-identical
across the entire range from the fixture baseline `a3c87fc` through the
M002-adopted head, `codegg-core` carries no Eggplan dependency at all, and the
CodeGG production graph contains only `eggplan-core` and `eggplan-codegg-compat`.
No Eggplan repository state is created by CodeGG.

## Requirement-to-evidence matrix

| Plan criterion | Evidence |
|---|---|
| 1. Current CodeGG snapshots map through a live bridge independent of fixture SHA gating | `normalize_snapshot` (`src/lib.rs:430`) takes no source SHA; `normalize_fixture` (`src/lib.rs:412`) is the only SHA-gated path. Tests `live_snapshot_mapping_does_not_require_fixture_provenance_and_assesses_purely`, `old_m001_fixture_provenance_is_rejected_not_relabelled`. |
| 2. Bridge production dependency graph excludes `eggplan-repo` | `crates/eggplan-codegg-compat/Cargo.toml:8-11`; `scripts/check-codegg-compat-boundary.sh:9-17` fails the build if `[dependencies]` names `eggplan-repo`. CodeGG `Cargo.lock` shows `eggplan-codegg-compat → {eggplan-core, serde, serde_json}`. |
| 3. CodeGG execution evidence carries authoritative verification binding | CodeGG `src/work_plan_eggplan.rs` `CodeggVerificationSpecV1` + `verification_digest_for_job`, digesting with `eggplan_core::digest_json` and formatting via `VerificationDigest` (CodeGG `src/work_plan_eggplan.rs:469-470`). Eggplan side enforces the gate at `src/lib.rs:566-575`; `execution_kinds_satisfy_only_with_the_authoritative_matching_binding`. |
| 4. Eggplan-backed assessment adopted behind CodeGG's existing public surface | CodeGG `src/work_plan_eggplan.rs` facade with `assess_with_engine`; `WorkPlanCompletionAssessment` unchanged; `codegg-core` remains Eggplan-free. Production call sites migrated in `src/agent/loop.rs`, `src/tool/goal.rs`, `src/tool/work_plan.rs`. |
| 5. Differential tests show no permissive completion regression | `dbowm91/codegg` `tests/work_plan_eggplan_differential.rs` — 28 cases, `assert_no_permissive_delta` in every Git-backed case. |
| 6. WorkPlanStore/Goal/Todo/checkpoint/scheduler ownership remains CodeGG | `git diff a3c87fc..ffa1c15e -- crates/codegg-core/src/work_plan/` is empty. See the ownership invariance section below. |
| 7. No Eggplan repository state is created by CodeGG | `eggplan-repo` absent from the CodeGG production graph; the facade is pure over the snapshot and never persists. |
| 8. Current long-horizon trajectory tests remain green | `long_horizon_trajectory_qualification` (27 tests) passed in CodeGG local verification and in hosted run `36760308368`. |
| 9. Both repositories record exact revisions and hosted CI evidence | This record; `dbowm91/codegg` `plans/closure/eggplan-assessment-integration/003-m002-status.md`; hosted runs 36763541044 (Eggplan) and 36760308368 (CodeGG). |

## Dependency graph, before and after

Eggplan bridge production graph, unchanged across the whole M002 span and
asserted by the boundary script:

    eggplan-codegg-compat
      -> eggplan-core
      -> serde
      -> serde_json

`eggplan-repo` and `tempfile` appear only under `[dev-dependencies]`, used by
`tests/parity.rs` for test-only snapshot persistence through the `create_snapshot`
helper that M002 §3 moved out of `src/`.

CodeGG production graph at `3c7438c7`/`ffa1c15e`, from `Cargo.lock`:

    codegg (application)  ->  eggplan-codegg-compat  ->  eggplan-core, serde, serde_json
                         ->  eggplan-core           ->  serde, serde_json, sha2, thiserror, uuid
    codegg-core           ->  (no Eggplan)

`eggplan-repo`, `eggplan-cli`, `eggplan-projection`, `eggplan-markdown`, and
`eggplan-integrations` are absent from the CodeGG production graph. CodeGG pins
both consumed crates by exact immutable `rev`, not by branch. `codegg-core`'s
`Cargo.toml` contains zero Eggplan references.

## Verification-binding derivation matrix

The bridge never mints a digest; it requires the host's binding to equal the
observation's own binding. Derivation itself is CodeGG's obligation, discharged
in `3e992291`.

| CodeGG ref | Eggplan kind | Binding required | Authoritative derivation | Unavailable → |
|---|---|---|---|---|
| `TestJob` | `Test` | yes | `codegg-verification-spec-v1` `test` variant: canonical test-runner argv, normalized cwd, target class | evidence unresolved, fail closed |
| `SchedulerJob` (managed argv) | `Command` | yes | `managed_argv` variant: canonical job payload argv + execution semantics | evidence unresolved, fail closed |
| `SchedulerJob` (shell) | `Command` | yes | `shell` variant; argv is mandatory | argv-less Shell is verification-unavailable |
| `SchedulerJob` (python path) | `Command` | yes | `python` variant; source-file content digest, not path text | hash-less inline Python is verification-unavailable |
| `SchedulerJob` (git) | `Command` | yes | `git` variant | evidence unresolved, fail closed |
| `DelegatedRun` | `DelegatedRun` | yes | `subagent_run` variant: prompt and permission-policy digests plus durable delegated identities | legacy `Subagent` is verification-unavailable |
| `AgentRun` | `DelegatedRun` | yes | same `subagent_run` spec, resolved through the durable AgentRun link | `AgentTurn` / `Research` are verification-unavailable |
| `Artifact` | `Artifact` | no | n/a | — |
| `Commit` | `Revision` | no | n/a | — |

Excluded from every spec: secrets, credentials, volatile timestamps, transport
lease/attempt IDs, mutable progress text, and display strings. Prompt and policy
content enters only as a digest, and policy reordering is proven digest-stable.
`ToolProgram` refs and any kind/payload mismatch are verification-unavailable.
CodeGG unit tests pin determinism, argv/cwd/target/timeout sensitivity, display
exclusion, policy-reorder stability, and content secrecy.

The Artifact/Commit exemption is a kind allowlist in Eggplan
(`requires_verification_binding`, `src/lib.rs:390-399`), not a host assertion,
and is covered by `artifact_and_commit_evidence_needs_no_execution_binding`.

## Differential parity matrix

28 cases in `dbowm91/codegg` `tests/work_plan_eggplan_differential.rs`, all
passing, with a hard assertion in every Git-backed case that Eggplan never
permits completion where legacy forbids it.

Parity — same allow/deny outcome:

| Case | Legacy | Eggplan |
|---|---|---|
| No evidence | Actionable | Actionable |
| Passing `TestJob` | Complete | Complete |
| Failed `TestJob` | Actionable | Actionable |
| In-flight `TestJob` | waits | waits |
| Blocked item | Blocked | Blocked |
| Dependency gate | blocked | blocked |
| Human-judgment only | awaits | awaits |
| Delegated run | Complete | Complete |
| Linked `AgentRun` | Complete | Complete |
| Scheduler variants (managed argv / shell / python-path / git) | Complete | Complete |
| Multi-item dependencies | agree | agree |
| Non-Git workspace | legacy engine | `LegacyNonGit`, explicit |
| Unsupported `Artifact`/`Commit` evidence | legacy engine | `LegacyUnsupportedEvidence`, explicit |
| Completed without proof | Actionable | Actionable |

Intentional deltas — all Eggplan-stricter, each asserted:

| Case | Legacy | Eggplan | Why it is acceptable |
|---|---|---|---|
| Missing / dangling ref | Actionable | fail-closed error | An unverifiable ref is not a proof of absence |
| Stale or drifted subject | Complete / Actionable | fail-closed error | Registry design gate 5: stale evidence is not current proof |
| Forged serialized `Satisfied` | Complete | non-completion | `Satisfied` is a claim; the bridge records it as `SerializedDispositionIsNotEvidence` |
| Verification unavailable | Complete | fail-closed error | Prose and ref IDs are not verification identity |
| Git capture failure | legacy path | fail-closed error, never `LegacyNonGit` | A capture failure is not evidence of a non-Git workspace |
| Terminal plan | history | rejected `plan_not_active` | Lifecycle history is not re-assessed |

No case became more permissive. That is the M002 stop condition and it did not
fire.

## WorkPlan ownership invariance evidence

`git diff a3c87fc18ee55aaf630401a562c11bb83112fd82 ffa1c15e654776c3ebe1022f4ce7de2582bc5d98 -- crates/codegg-core/src/work_plan/`
is empty. The model, assessment, evidence, projection, store, Todo projection,
checkpoint, and epoch-policy modules are byte-identical across the entire M002
span. Everything M002 touched is application-layer:

    src/work_plan_eggplan.rs      (+1741)  new Eggplan facade
    src/work_plan_arbiter.rs      (+85)    legacy engine wrappers
    src/work_plan_evidence.rs     (+129)   host evidence enrichment
    src/tool/work_plan.rs         (68)     call-site migration
    src/agent/loop.rs, src/tool/goal.rs     call-site migration
    tests/…                       new differential and resolved-evidence suites

Neither of the fixture-recorded CodeGG source cases that the Eggplan corpus
depends on changed: `crates/codegg-core/tests/work_plan_foundation.rs` and
`crates/codegg-core/tests/work_plan_projection_arbiter.rs` are unchanged;
`tests/long_horizon_trajectory_qualification.rs` changed only by an upstream
import reformat plus an added `ExecutionTarget::default()` field from the M001
provenance work, leaving the recorded dependency-gating and
no-hidden-reasoning behavior intact. The fixture baseline therefore remains
valid provenance.

## Production evidence

CodeGG, `3c7438c7` (exact implementation tree):

- `cargo test -p codegg --lib -- work_plan_eggplan` — 14 passed.
- `cargo test --test work_plan_eggplan_differential` — 28 passed.
- `work_plan_projection_arbiter` 14, `work_plan_resolved_evidence` 9,
  `long_horizon_trajectory_qualification` 27, `scheduler_authority_matrix` 13,
  lib `work_plan` 19, `tool::goal`/`agent` 351 — all passed.
- `scripts/verify.sh quick` — passed. Workspace Clippy `-D warnings`,
  `cargo fmt --check`, `git diff --check` — clean.
- `bash scripts/check-core-boundary.sh` — passed.
- `python3 scripts/check_execution_ownership.py`,
  `check_scheduler_bypass.py`, `check_eggwork_target_routing.py` — passed.
- In-module static guards — exact-rev manifest+lock assertion, production-graph
  boundary assertion, and production call-site assertion all green.

Eggplan, `1291799a72d2ab5f2d36c75cec0d40eeb4c5d347` (see verification table).

## Verification executed

Eggplan local Linux:

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo check --workspace --all-targets --locked` | pass |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | pass |
| `cargo test --workspace --locked` | pass; 130 tests across 21 suites |
| `cargo test -p eggplan-codegg-compat --locked` | pass; 19 tests |
| `bash scripts/check-core-boundary.sh` | pass |
| `bash scripts/check-codegg-compat-boundary.sh` | pass |
| `bash scripts/check-integrations-boundary.sh` | pass |
| `bash scripts/check-projection-cli-boundary.sh` | pass |
| `bash scripts/check-closure-authority-boundary.sh` | pass |
| `cargo +1.89.0 check --workspace --all-targets --locked` | pass |
| `cargo +1.89.0 test --workspace --locked` | pass |
| `git diff --check` | pass |

Hosted Eggplan workflow: [CI run 36763541044](https://github.com/eggstack/eggplan/actions/runs/36763541044)
on `1291799a72d2ab5f2d36c75cec0d40eeb4c5d347`, the M002 implementation
commit. Conclusion: success.

| Job | ID | Result |
|---|---|---|
| native (ubuntu-latest) | 110051873828 | success — format, check, clippy, tests, all five boundary scripts |
| native (macos-latest) | 110051873976 | success — check, clippy, tests, boundary scripts (format is platform-skipped by workflow) |
| native (windows-latest) | 110051873961 | success — check, clippy, tests, boundary scripts (format is platform-skipped by workflow) |
| msrv (Rust 1.89.0) | 110051873538 | success — check and tests only |

The closure/roadmap/registry commit `0985aee` is itself green on all four
jobs: [CI run 36764064936](https://github.com/eggstack/eggplan/actions/runs/36764064936).
Per the M001 convention, the authoritative run cited above is the
implementation commit's run; later documentation-only commits are recorded but
not treated as the qualification subject.

CodeGG verification is recorded in CodeGG's own closure and was not re-executed
from this repository; the commands and results above are quoted from
`dbowm91/codegg` `plans/closure/eggplan-assessment-integration/003-m002-status.md`, with the
hosted run independently re-verified from this repository
(`gh run view 36760308368` → `conclusion: success`,
`headSha: 3c7438c738a142fdd0c4e79684d7d1154fe52a99`, workflow `CI`, event
`push`).

CodeGG-side commands from plan §17 were **not run from this repository**. They
were run in `dbowm91/codegg` against `3c7438c7` and recorded there, with
substitutions for names that differ (`./scripts/verify.sh quick` in place of the
per-suite invocations, `cargo test --test work_plan_eggplan_differential` in
place of a differently named matrix suite, and no separate
`--test work_plan_projection_arbiter` invocation because that suite lives in
`codegg-core`). That substitution is CodeGG's record to make, and it is named
here rather than silently treated as satisfied. Every plan §17 Eggplan command
was run.

## Invariant review

- No circular dependency. CodeGG consumes Eggplan; Eggplan has no CodeGG
  dependency and the bridge cannot see CodeGG types.
- CodeGG `WorkOrder` remains distinct and untouched; the facade is a
  `WorkPlan` seam, and `codegg-core` gained no Eggplan reference.
- Model prose still cannot establish host evidence. Serialized
  `AcceptanceDisposition::Satisfied` maps to a claim and is recorded as
  `SerializedDispositionIsNotEvidence`; it produced non-completion in the
  differential suite where legacy wrongly completed.
- Stale revisions remain explicit conflicts. Stale and drifted subjects are
  fail-closed errors; the S2 revalidation race
  (`completion_revalidation_refuses_changed_subject`) leaves the plan Active and
  evidence untouched.
- Source `revision` remains provenance. `SourceRevisionIsProvenanceOnly` is
  always recorded, and source lifecycle is never replayed.
- CodeGG `Completed` did not become an Eggplan `ClosureRecord`. The bridge maps
  it to `PlanStatus::Active` plus `CompletedPlanNeedsGuardedClose`; only Eggplan
  Evidence M002 guarded finalization can close a plan.
- Owner run/job IDs stay provenance. They appear only in
  `MappingManifest::owner_provenance` and never in provider authority.
- Scheduling, Goal, Todo, permission, sandbox, worktree, and context-epoch
  behavior was not changed to make parity pass. Those call sites were migrated
  to engine selection only.
- No new Eggplan persisted schema or repository-state behavior was introduced.
  `eggplan-repo` remains dev/test-only in this repository too.

## Failure and recovery review

- Verification binding missing or mismatched fails the whole assessment closed
  rather than degrading to partial satisfaction. Deliberate: a single
  unverifiable ref must not silently drop the others.
- A Git subject capture failure in CodeGG is an error, never reclassified as
  non-Git. Only a positively non-Git workspace selects the legacy engine.
- S1/S2 completion revalidation: if the subject changes between assessment and
  the CAS, CodeGG returns `subject_changed_before_completion` and preserves both
  the plan row and its evidence. Covered by an explicit test.
- Bridge input bounds are explicit and rejected rather than truncated into
  partial meaning: `MAX_FIXTURE_BYTES`, `MAX_ITEMS`,
  `MAX_ACCEPTANCES_PER_ITEM`, `MAX_EVIDENCE_REFS_PER_ITEM`, `MAX_TEXT_CHARS`,
  `MAX_SOURCE_ID_CHARS`, `MAX_DETAIL_CHARS`.
- Retries are the host's responsibility. The bridge is pure and has no state to
  corrupt; CodeGG's engine selection is deterministic for identical inputs.
- Cancellation stays a CodeGG CAS transition. `Cancelled` maps to cancelled
  intent only and creates no closure.

## Migration and compatibility review

- M001 sessions are unaffected. `WorkPlanStore`, the schema layout version, and
  the WorkPlan DTOs are unchanged; the facade is a new application-layer module.
- The M001 fixture corpus was not relabelled. `SOURCE_CODEGG_SHA` moved to
  `a3c87fc` and any other SHA, including the previous M001 baseline
  `28b4695661d463dd1675d045ac6299c5fbc9ea31`, is rejected by
  `normalize_fixture` with an explicit test.
- `WorkPlanCompletionAssessment` remains the CodeGG-facing DTO, so Todo and
  arbiter callers need no knowledge of Eggplan types.
- Schema v1 remains frozen; unknown fixture fields are still rejected, not
  treated as an extension point.
- `eggplan-repo` never entered the CodeGG production graph, so no consumer
  outside CodeGG needs a package-format decision. Registry-resolvable
  publishing remains out of scope per plan §15.

## Security review

- No secret, credential, token, or absolute host path enters a fixture, a
  mapping manifest, or a verification specification. Prompt and policy content
  enters specifications only as a digest.
- Volatile identifiers — timestamps, lease/attempt IDs, transport addresses,
  mutable progress text — are excluded from verification specs, so digests stay
  reproducible rather than secret-bearing.
- A digest is never treated as authenticity. Registry design gate 14 stands:
  a matching digest proves the observation covered the same verification
  specification, not that the observation is true. Provider identity and trust
  remain host-supplied and registry-governed.
- The bridge has no ambient authority: no `register_trusted` call, no
  `SubjectCapture`, no `finalize_closure*` in its `src/`, verified by source
  grep and by the boundary script.
- Inputs are untrusted: `deny_unknown_fields` on every DTO, explicit byte and
  collection bounds, and NUL/length rejection on all text fields.
- Adversarial resolver output fails closed. Stale subject, mismatched kind,
  reused observation ID, or a non-matching verification digest all abort the
  mapping.

## Documentation

Eggplan:

- `architecture/codegg-compat.md` — live bridge versus fixture provenance, an
  explicit verification-binding ownership table, Artifact/Commit exemption
  rationale, and the no-relabel policy for superseded fixture SHAs.
- `architecture/deep-dive-codegg-compat.md` — refreshed test inventory, the
  corrected baseline recheck, and two new findings (whole-assessment
  fail-closed; resolver-supplied provider identity).

CodeGG (its repository): `architecture/work_plan.md` documents the staged
boundary, engine selection, verification-spec v1, evidence adapter, S1/S2
revalidation, and retained WorkPlanStore/arbiter/Goal/Todo/checkpoint ownership.

## Unresolved findings

| Finding | Severity | Disposition |
|---|---|---|
| Resolver and `ProviderRegistry` correctness is trusted host code; the bridge only fails closed on subject/kind/ID/binding violations | accepted, bounded | Recorded in the deep dive. Misconfigured trust can only withhold completion, never manufacture it. |
| Fail-closed is whole-assessment, so one bad ref suppresses otherwise-valid evidence in that snapshot | accepted, by design | CodeGG surfaces the error rather than retrying with a filtered snapshot. M003 must not relax this. |
| `MappingManifest::validate` does not re-derive IDs from sources; determinism rests on the parity test | low | Acceptable for a pure bridge. Stated explicitly in the deep dive. |
| The CodeGG CI timing-flake corrective C002 was a prerequisite for trustworthy hosted qualification | closed | Implementation `696ec282c283984c248478bc481d4f48c6bdb693` with run `36748429660` green, closed at `d9dd024900dca609a34fb81222902c8b6425aa30`. Canonical M002 run `36760308368` postdates it. |
| A pre-existing CodeGG plugin-dir test-isolation defect surfaced during qualification | closed, unrelated | Fixed in `3c7438c7`; no M002 code involved. Proven green by the canonical run. |
| Plan §17 CodeGG verification commands were not executed verbatim from this repository | informational | CodeGG records its actual equivalents in its own closure; not treated as satisfied here. |
| crates.io publishing / vendoring decision for future releases | deferred | Explicitly out of M002 scope per plan §15. Not needed while the CodeGG pin is a git rev. |

No unresolved implementation finding remains on the Eggplan side.

## Roadmap disposition

CodeGG Integration M002 is closed. Subsystem status moves from active to
closed/current, with M003 becoming the next planned capability.

M003 — CodeGG repository Plan binding is now **dependency-ready**: M001 closed
the golden-parity seam, M002 closed staged assessment adoption, and CodeGG's own
closure records that M002 "lifts the Eggplan gate". M003 may be planned from
`ffa1c15e654776c3ebe1022f4ce7de2582bc5d98`, subject to the M003 boundary: a
CodeGG WorkOrder or session referencing an Eggplan Plan must not make Eggplan
CodeGG's WorkOrder scheduler.

No other lane is unblocked by this closure. Projection/CLI M003 remains
gated on real repository use, Eggstack M003 remains ready for planning against
the rechecked Eggbench baseline, and Interoperability/distribution stays
deferred.

## Registry updates

- `plans/registry.md` CodeGG integration subsystem status: active → closed/current,
  current milestone M003.
- `plans/registry.md` registered plan table: M002 row `ready` → `closed` with
  this closure record.
- `plans/registry.md` external interface baseline: CodeGG row updated from the
  superseded branch-local SHAs to the landed `3e992291` / `3c7438c7` /
  `ffa1c15e` revisions.
- `plans/registry.md` execution order: M002 moves from "ready as a coordinated
  two-repository handoff" to closed; the CodeGG M003 gate is lifted.
- `plans/subsystems/codegg-integration-roadmap.md`: M002 status closed, M003
  recorded as next and dependency-ready, baselines refreshed.

### Factual errata

The M002 plan header and the pre-closure registry cited CodeGG branch-local
SHAs `85058541`, `81a914df`, `79bae034`, closure `e4528ab9`, and hosted run
`36336450431`. Those objects are not present in `dbowm91/codegg` after the work
landed on `main`; the landed revisions are `3e992291` (implementation),
`3c7438c7` (qualification follow-up), and `ffa1c15e` (closure), with hosted
canonical run `36760308368`. This is a factual correction of superseded
revision identifiers only. No plan finding, criterion, or historical closure
conclusion changes.

A later maintenance refactor of `crates/eggplan-codegg-compat/src/lib.rs`
(the M003 repository-Plan binding work) moved four cited definitions, so the
`file:line` references in this record no longer point at the same lines. The
original citations are preserved as accepted; the current locations are
`normalize_snapshot` at `:638` (cited `:430`), `normalize_fixture` at `:620`
(cited `:412`), `requires_verification_binding` at `:598` (cited `:390-399`),
and the execution-binding gate at `:775-784` (cited `:566-575`). Every
substantive claim above still holds: both mappers exist, `normalize_fixture`
remains the only SHA-gated path, and the gate is unchanged. This is citation
drift only; no acceptance criterion or closure conclusion changes.
