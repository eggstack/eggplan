# Evidence and Closure M002 C004 — Closed

Status: closed

Source implementation plan:

- plans/implementation/evidence-closure/002-c004-closed-plan-cas-immutability.md

Source roadmap:

- plans/subsystems/evidence-closure-roadmap.md

Reviewed baseline: `3d726496cc2e4e76f48000310e2c96829878106a`

Implementation commit: `623dde976b83be32c737fccaeb09a239245d8325`

Hosted qualification: not run for this pass. See "Verification executed" for the
exact reason; every command in the plan's §8 list was run locally, including
both MSRV rows, and all passed.

## Executive finding

C004 closed a durable availability defect. Ordinary `compare_and_swap` accepted
a `Closed` → `Closed` rewrite, wrote it, and only then reported `Corrupt`,
leaving the entire state root permanently unopenable. The store now refuses any
CAS against a `Closed` Plan — in every direction, including `Closed` → `Closed`
itself — before any write, with the typed `RepoError::ClosedPlanImmutable`
surfacing as the stable CLI code `closed_plan_immutable`.

`plan_transition_allowed` is deliberately unchanged, and the plan's proposed
core invariant was wrong on that point; see "Plan errata" below. Authority to
write a `Closed` Plan remains exclusively `RepositoryStore::finalize_closure`,
whose behavior is unchanged.

As recorded in the plan, this was never a closure bypass. Evidence integrity and
closure authority were reviewed and are unaffected: no `ClosureRecord` is
forged, no evidence is fabricated, no provider trust is enrolled, and the guard
refuses a rewrite rather than permitting one.

## Plan errata

The plan's §3 required "a regression test pinning that no `(Closed, _)` or
`(_, Closed)` pair is legal" in `plan_transition_allowed`. That invariant is
false, and encoding it would have broken the guarded finalizer:
`plan_transition_allowed` contains the deliberate `(Active, Closed)` edge, which
is exactly the lifecycle transition `finalize_closure` performs. Authority over
*who* may close is a repository concern, enforced by the CAS guards; core states
only what is structurally possible.

The implemented test therefore pins the invariant that actually holds — `Closed`
is terminal as a *source*, so no pair leaves it, `Closed` → `Closed` included —
and separately asserts that `(Active, Closed)` remains legal, so the terminality
pin cannot later be "fixed" by deleting that edge. `plan_transition_allowed` is
byte-identical to the baseline.

## Requirement-to-evidence matrix

| Acceptance criterion | Evidence |
|---|---|
| 1. Ordinary CAS cannot modify a `Closed` Plan in any direction, including `Closed` → `Closed` | Unconditional guard at `crates/eggplan-repo/src/store.rs:930-932`, placed after the revision-conflict check and before the entry guard and before `atomic_write`. `closed_plan_is_immutable_under_ordinary_compare_and_swap` drives a `Closed` → `Closed` rewrite plus `Closed` → `{Draft, Active, Blocked, Cancelled, Closed}`, and a same-status item-only edit. |
| 2. Typed `RepoError` raised before any write; stored bytes provably unchanged | `RepoError::ClosedPlanImmutable(PlanId)`. The test snapshots `plan.json` and `closure.json` bytes before the refused mutations and asserts both are byte-identical afterward, and that the Plan still deserializes to the closed candidate. A stale expected-revision still yields `Conflict`, so the optimistic-concurrency signal is unchanged. |
| 3. No reachable path can desynchronize a `Closed` Plan from its `ClosureRecord` | The only pre-fix path to that state was the CAS accepted by 6.1; it is now refused. The test reopens the state root after all refusals and asserts `list()` and `get()` both succeed and the observation is still readable — the exact property the defect destroyed. |
| 4. `finalize_closure` remains the only writer of a `Closed` Plan, behavior unchanged | `finalize_closure_inner` writes through `atomic_write` at `store.rs:482` and never routes through `compare_and_swap`, so it is structurally unreachable from the new guard. The test drives a full `create` → activate → `InProgress` → `Completed` → evidence → `finalize_closure` cycle and asserts `record.validate(&closed)` succeeds; `guarded_closure_persists_integrity_and_reopens` is unchanged and still green. |
| 5. `plan_transition_allowed` keeps `Closed` terminal, pinned by a core test | `closed_is_terminal_across_the_whole_plan_transition_matrix` asserts no legal pair leaves `Closed` across the full 5×5 matrix, and that `(Active, Closed)` stays legal. Function body unchanged from the baseline. |
| 6. CLI maps the new rejection to a stable diagnostic code | `repo_failure` maps `ClosedPlanImmutable` → `closed_plan_immutable`. Asserted end-to-end in `close_uses_explicit_policy_and_guarded_repository_protocol`, which closes a plan through the CLI and then asserts `eggplan item update` on the `Closed` plan returns that code *and* leaves the revision at 5. |
| 7. `architecture/repository.md` and `architecture/evidence.md` state the invariant and its consequence | `repository.md` states that a canonical Closed Plan is immutable under ordinary CAS in both directions, names both errors, and records that the refusal precedes `atomic_write` so stored bytes are unchanged. `evidence.md` ties the refusal to the "Closed Plan without a matching record is corruption" rule. |
| 8. No public API or persisted schema changes | No new method, type, or persisted field. One additive variant on the existing public `RepoError` enum, which §5 of the plan explicitly contemplated ("otherwise add one precisely named variant") and which acceptance criterion 2 requires in order to be a typed error. `PlanStore`, `RepositoryStore`, `finalize_closure`, `ClosureRecord`, and every canonical digest rule are untouched. Interpreted as forbidding new API surface, not new enum variants; recorded here so the reading is explicit. |
| 9. Native and MSRV qualification pass, all five boundary guards green | See "Verification executed". All five guards pass; both `cargo +1.89.0` rows pass against the pinned 1.89.0 toolchain. |

## Production evidence

```rust
// crates/eggplan-repo/src/store.rs, inside compare_and_swap, under the store lock
if current.status == eggplan_core::PlanStatus::Closed {
    return Err(RepoError::ClosedPlanImmutable(id.clone()));
}
if next.status == eggplan_core::PlanStatus::Closed
    && current.status != eggplan_core::PlanStatus::Closed
{
    return Err(RepoError::GuardedClosureRequired);
}
```

The guard sits after the `Conflict` check, so a stale expected-revision still
reports `Conflict` rather than being masked by an immutability refusal.

The pre-fix failure mode was observed directly rather than inferred. With the
guard temporarily disabled, the new test was instrumented to print the actual
result, which produced:

```text
PROBE closed->closed returned: Err(Corrupt { path: ".../plans/ep_store/closure.json",
  reason: "closure record does not match final plan" })
PROBE reopen after rewrite: Err(Corrupt { path: ".../plans/ep_store/closure.json",
  reason: "closure record does not match final plan" })
```

This confirms the plan's §2 account exactly: a late `Corrupt` after a durable
write, and an unopenable state root. The probe was removed before commit.

### Regression that would have detected E-M002-C004-01

`closed_plan_is_immutable_under_ordinary_compare_and_swap`
(`crates/eggplan-repo/tests/repository.rs:349-509`). Verified to fail against
the pre-fix code: with the guard disabled it fails on the first `Closed` →
`Closed` assertion at `repository.rs:450`, because no typed refusal exists.

The core matrix test does **not** detect the defect — `plan_transition_allowed`
was already correct. It is a forward guard against weakening that matrix, and is
recorded as such rather than as detection evidence.

## Verification executed

All commands ran locally on Linux at the implementation commit. Every row in the
plan's §8 list was run; none is reported from the plan alone.

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo check --workspace --all-targets --locked` | pass |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | pass |
| `cargo test --workspace --locked` | pass — 24 suites, 0 failed (one pre-existing `#[ignore]` recorder in the frozen digest golden) |
| `cargo test -p eggplan-repo --locked` | pass — 23/23 in `tests/repository.rs`, plus lib, fingerprint, and doc suites |
| `cargo test --workspace --doc --locked` | pass — the six `compile_fail` doctests all pass |
| `cargo +1.89.0 check --workspace --all-targets --locked` | pass — MSRV toolchain 1.89.0 present and used |
| `cargo +1.89.0 test --workspace --locked` | pass — no failures |
| `bash scripts/check-closure-authority-boundary.sh` | pass, including its synthetic proofs |
| `bash scripts/check-core-boundary.sh` | pass |
| `bash scripts/check-codegg-compat-boundary.sh` | pass |
| `bash scripts/check-integrations-boundary.sh` | pass |
| `bash scripts/check-projection-cli-boundary.sh` | pass |
| `git diff --check` | pass — no whitespace errors |

Not run: hosted Linux/macOS/Windows qualification and the hosted MSRV job. No
push-triggered run identifier is recorded here because this record was authored
before the branch was pushed. Per `plans/003-planning-process.md` §7 this is
recorded as *not run* rather than inferred as passing; the C004 defect was
reproducible with a local integration test, and the local MSRV rows cover the
toolchain gate that the hosted job exists to enforce.

## Invariant review

- **Canonical bytes.** Unchanged. The fix adds no encoding, digest, or ordering
  rule, and the test asserts the refused path leaves `plan.json` and
  `closure.json` byte-identical.
- **Subject equality.** Untouched. The S1/S2 double-capture algorithm, the
  `ClosureSubjectStale` / `ClosureSubjectDrift` / `ClosureSubjectCapture`
  outcomes, and `GitSubjectSource` are not modified, and no C002 containment was
  regressed: `SubjectCapture` and `finalize_closure_with_capture` remain
  crate-private, which the closure-authority guard and the six compile-fail
  doctests both re-prove.
- **Host-conferred trust.** Untouched. No adapter, registry, or policy path was
  modified; the integrations boundary guard is green.
- **Closed-Plan immutability (newly enforced).** A canonical `Closed` Plan is
  now unreachable by ordinary CAS from any state, so no ordinary operation can
  produce the desynchronization `architecture/evidence.md` calls corruption.

## Failure, recovery, and compatibility review

- The guard runs before any write, so a rejected update leaves repository state
  byte-identical. No partial state, no pending closure, no revision advance.
- `Corrupt` is no longer the first signal of an attempted illegal transition; the
  refusal is typed and precedes persistence.
- No new locking. The check is a precondition inside the already-locked section
  that `compare_and_swap` already held; the pending-closure protocol is
  unchanged.
- **No migration is provided for already-poisoned repositories.** That is the
  plan's explicit scope decision, not an oversight. Recovery or migration would
  need its own plan and its own decision about what a desynchronized record
  should become. A repository poisoned before this commit still fails to open,
  and now fails to open *without* the ability to be caused again. No such
  repository was found in this workspace.
- No compatibility or migration surface: no persisted schema, no canonical
  digest, no CLI flag, and no default behavior changed for any plan that is not
  `Closed`.

## Security review

No trust or authority surface changed. The change strictly *removes* a reachable
mutation of a `Closed` Plan. It adds no credential, network, or subprocess
surface, and no error message leaks repository content — the new variant carries
only the `PlanId` the caller already supplied. Denial of service by a caller who
already holds write access to a state root they control is the pre-existing
blast radius, and it is now unreachable through the public API.

## Unresolved findings

| Finding | Severity | Disposition |
|---|---|---|
| Already-poisoned state roots have no repair path | Medium | Open by design. Out of C004 scope (§4). Needs a separate plan; recorded so the next reader does not assume a poisoned repository can be recovered. |
| `RepoError` gained an additive variant on a non-`#[non_exhaustive]` public enum | Low | Accepted. Additive and source-breaking only for an exhaustive downstream `match`; the sole in-tree consumer (`repo_failure`) has a catch-all. Recorded under criterion 8 so the interpretation is explicit rather than silent. |
| Registry row for CodeGG M003 reads `closed` while its closure file is `003-conditionally-closed.md` | Low | Not C004. Traceability nit previously flagged to the user; still open, untouched here. |

No unresolved finding affects evidence integrity or closure authority.

## Roadmap disposition

Evidence/closure M002 returns to **closed with condition**: C001, C002, C003 and
now C004 are all closed, and the C004 durability defect is resolved. No
milestone was reopened, no historical closure record was rewritten, and no
finding in C001–C003 changed. C004 gated further Evidence/closure work; that
gate is now lifted, so Projection/CLI M002 and Eggstack M002 are no longer
blocked behind it. CodeGG M003 C002 remains independently open and
non-blocking.

## Registry updates

- `plans/registry.md`: Evidence/closure subsystem status `corrective required` →
  `closed with condition`; the C004 registered-plan row `ready` → `closed` with
  its closure path; the maintenance and execution-order notes updated to record
  C004 as closed and lift the Evidence/closure gate.
- `plans/subsystems/evidence-closure-roadmap.md`: `### M002 C004` status
  `ready` → `closed`, with the closure path recorded and the finding restated as
  closed. The pre-implementation description of the defect is retained as the
  record of what was found.
