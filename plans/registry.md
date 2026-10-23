# Eggplan Active Planning Registry

This file is the compact control surface for current Eggplan planning.
Detailed authority remains in specifications, ADRs, subsystem roadmaps,
implementation plans, closure records, and Git history.

## Canonical direction

- plans/000-long-term-specification.md
- plans/001-terminology-and-domain-model.md
- plans/002-long-term-roadmap.md
- plans/003-planning-process.md

## Status vocabulary

- proposed — document exists but is not dependency-ready.
- ready — hard dependencies/interfaces are satisfied; handoff is authorized.
- active — implementation is in progress.
- blocked — named dependency/evidence prevents progress.
- closing — implementation landed and closure evidence is being gathered.
- closed — closure record accepted.
- conditionally closed — implementation complete with named external evidence outstanding.
- corrective required — later defect must close before current qualification.
- superseded — replaced by another plan/decision.
- archived — retained for traceability.
- deferred — intentionally outside current implementation horizon.

## Repository planning baseline

eggstack/eggplan was an empty Git repository when planning was established on
2026-09-22/23. The repository was initialized with README.md at:

- ce2d6e7a6c1cecbe7ff554daf9d603d2d604d98c — repository initialization.

No Rust workspace or production capability existed at that baseline. Planning
documents are not evidence that implementation exists.

Planning-system bootstrap:

- 42ec41eff3e35c15acacaa0d717320c40543fbc1 — canonical specs, ADRs, subsystem roadmaps, registry, and initial implementation handoffs.

## Accepted architectural decisions

| ADR | Status | Decision |
|---|---|---|
| plans/adrs/ADR-0001-planning-evidence-mechanism-not-execution.md | accepted | Eggplan owns plan/evidence/closure mechanism, not scheduling or execution |
| plans/adrs/ADR-0002-repository-first-versioned-canonical-state.md | accepted | Versioned structured repository state is canonical; Markdown is projection/import |
| plans/adrs/ADR-0003-immutable-revision-scoped-evidence.md | accepted | Observations are immutable, provider-authoritative and subject-revision scoped |
| plans/adrs/ADR-0004-codegg-extraction-without-workorder-conflation.md | accepted | Generalize CodeGG WorkPlan semantics incrementally; WorkOrder remains CodeGG-owned |

## Active subsystem roadmaps

| Subsystem | Status | Current milestone | Authority |
|---|---|---|---|
| Foundation core/repository | closed/current | M001 closed; M002 historical caveat resolved by M003; M003 closed and cross-platform qualified. Five M003 corrective lines (C001-C005) were raised by the 2026-10-23 repository audit and are now implemented and closed; none gated another subsystem | plans/subsystems/foundation-core-roadmap.md |
| Evidence/closure | closed/current | M002 and C001/C002/C003/C004 closed; the C004 durability defect is resolved — ordinary CAS can no longer modify a Closed Plan, so no reachable path desynchronizes it from its ClosureRecord | plans/subsystems/evidence-closure-roadmap.md |
| Projection/CLI | active/current | M001/M002/M003a/M003b closed; the M003 inspection snapshot is hosted-qualified and batch/completion work builds on it. Three M003b corrective lines (C001-C003) are registered and open from the 2026-10-23 repository audit, and M004 is proposed pending its open decisions | plans/subsystems/projection-cli-roadmap.md |
| CodeGG integration | closed/current | M001-M003 and C001/C002 closed; the roadmap is terminal. C002 closed the boundary guard's filesystem gap: `check-codegg-compat-boundary.sh` now fails on filesystem access, every guard message states its actual scan scope, and all five guards have synthetic self-proofs. The bridge source was already pure; the defect was in the enforcement | plans/subsystems/codegg-integration-roadmap.md |
| Eggstack integrations | active/current | M001, M002, M003a (`674264d`), and M003b (`93d65b0`) closed; Eggsact and Eggup candidates unchanged and deferred | plans/subsystems/eggstack-integration-roadmap.md |
| Interop/distribution | deferred | waits on local core/CLI/integrations | plans/subsystems/interoperability-distribution-roadmap.md |

## Registered implementation plans

| Subsystem | Milestone | Status | Implementation plan | Handoff |
|---|---|---|---|---|
| Foundation | M001 repository bootstrap + typed domain | closed | plans/implementation/foundation-core/001-repository-bootstrap-and-domain-contract.md | closure plans/closure/foundation-core/001-closed.md |
| Foundation | M002 repository store/CAS/Git subject | conditionally closed | plans/implementation/foundation-core/002-repository-store-cas-and-subject.md | historical closure plans/closure/foundation-core/002-conditionally-closed.md; platform caveat resolved by M003 |
| Foundation | M003 subject scope + strict schema + platform hardening | closed | plans/implementation/foundation-core/003-subject-scope-strict-schema-and-platform-hardening.md | closure plans/closure/foundation-core/003-closed.md |
| Evidence | M001 evidence ledger/assessment | closed | plans/implementation/evidence-closure/001-evidence-ledger-and-assessment.md | historical closure plans/closure/evidence-closure/001-closed.md |
| Evidence | M001 C001 verification binding + end-to-end evidence corrective | closed | plans/implementation/evidence-closure/001-c001-verification-binding-and-end-to-end-evidence-corrective.md | closure plans/closure/evidence-closure/001-c001-closed.md |
| Evidence | M002 guarded closure records + supersession + recovery | closed | plans/implementation/evidence-closure/002-closure-records-integrity-and-recovery.md | historical closure plans/closure/evidence-closure/002-closed.md |
| Evidence | M002 C001 finalization subject revalidation | closed | plans/implementation/evidence-closure/002-c001-finalization-subject-revalidation.md | historical closure plans/closure/evidence-closure/002-c001-closed.md |
| Evidence | M002 C002 finalization test-seam containment + closure evidence reconciliation | closed | plans/implementation/evidence-closure/002-c002-finalization-test-seam-containment-and-closure-evidence-reconciliation.md | closure plans/closure/evidence-closure/002-c002-closed.md; preserves M002/C001 historical closures |
| Evidence | M002 C003 closure reference + authority-guard hygiene | closed | plans/implementation/evidence-closure/002-c003-closure-reference-and-authority-guard-hygiene.md | closure plans/closure/evidence-closure/002-c003-closed.md; non-blocking maintenance |
| Evidence | M002 C004 Closed-Plan CAS immutability | closed | plans/implementation/evidence-closure/002-c004-closed-plan-cas-immutability.md | closure plans/closure/evidence-closure/002-c004-closed.md; restores design gate 10: ordinary CAS cannot modify a Closed Plan in any direction |
| Projection/CLI | M001 CLI control surface + derived registry | closed | plans/implementation/projection-cli/001-cli-control-surface-and-derived-registry.md | closure plans/closure/projection-cli/001-closed.md |
| Projection/CLI | M002 loss-aware Markdown import + deterministic render | closed | plans/implementation/projection-cli/002-loss-aware-markdown-import-and-deterministic-render.md | closure plans/closure/projection-cli/002-closed.md |
| Projection/CLI | M003a repository inspection snapshot + performance qualification | closed | plans/implementation/projection-cli/003a-repository-inspection-snapshot-and-performance-qualification.md | closure plans/closure/projection-cli/003a-closed.md; implementation `528f75f`, hosted-qualified by run `37368489637` on combined commit `2c32ffe` (all four jobs green) |
| Projection/CLI | M003b batch queries + compact projections + shell completions | closed | plans/implementation/projection-cli/003b-batch-compact-projections-and-shell-completions.md | closure plans/closure/projection-cli/003b-closed.md; implementation `2c32ffe`, hosted run `37368489637` (all four jobs green) |
| CodeGG integration | M001 golden parity + adapter seam | closed | plans/implementation/codegg-integration/001-golden-parity-and-adapter-seam.md | closure plans/closure/codegg-integration/001-closed.md |
| CodeGG integration | M002 staged Eggplan assessment adoption | closed | plans/implementation/codegg-integration/002-staged-eggplan-assessment-adoption.md | closure plans/closure/codegg-integration/002-closed.md; Eggplan bridge 088968b + 1291799, CodeGG adoption 3e992291/3c7438c7, CodeGG closure ffa1c15e |
| CodeGG integration | M003 repository Plan binding contract | closed | plans/implementation/codegg-integration/003-repository-plan-binding-contract.md | Eggplan contract implementation `3f7c603` qualified by hosted run `36868055136`; the CodeGG consumer is implemented at CodeGG `53dea47f` and hosted-qualified there by run `36938461935` (success), satisfying the former condition. Closure: plans/closure/codegg-integration/003-conditionally-closed.md |
| CodeGG integration | M003 C001 dirty-subject fingerprint + bound-evidence requalification | closed | plans/implementation/codegg-integration/003-c001-dirty-subject-fingerprint-and-bound-evidence-requalification.md | Eggplan fingerprint contract implemented at `0dd33b7` (hosted run `37063328954`, all four jobs green; earlier Windows failures `37062437251`/`37062837529` retained as non-passing evidence); CodeGG half implemented at `36ec9322`, pinned `0dd33b7` and consumed the API in `3623f65e` + `b470865a` (PR `dbowm91/codegg#90`, hosted `CI` `37084905013` green on `main`). Closure: plans/closure/codegg-integration/003-c001-closed.md. Historical M003 remains closed. |
| CodeGG integration | M003 C002 compatibility boundary-guard completeness | closed | plans/implementation/codegg-integration/003-c002-boundary-guard-completeness.md | closure plans/closure/codegg-integration/003-c002-closed.md; non-blocking tooling hardening — the bridge source was already pure |
| Eggstack integrations | M001 evidence provider SPI | closed | plans/implementation/eggstack-integration/001-evidence-provider-spi.md | closure plans/closure/eggstack-integration/001-closed.md |
| Eggstack integrations | M002 Eggwork + Eggsearch evidence adapters | closed | plans/implementation/eggstack-integration/002-eggwork-and-eggsearch-evidence-adapters.md | closure plans/closure/eggstack-integration/002-closed.md |
| Eggstack integrations | M003a Eggbench verified bundle + comparison evidence | closed | plans/implementation/eggstack-integration/003a-eggbench-verified-bundle-and-comparison-evidence.md | closure plans/closure/eggstack-integration/003a-closed.md; Eggplan `674264d`, hosted run `37350383159`; Eggbench fixture baseline `30a38251` (CI `37143714313`, live `37143714261`) |
| Eggstack integrations | M003b GitHub CI/forge + artifact evidence | closed | plans/implementation/eggstack-integration/003b-github-ci-forge-and-artifact-evidence.md | closure plans/closure/eggstack-integration/003b-closed.md; Eggplan `93d65b0`, hosted run `37351818356`; qualified against GitHub REST API `2022-11-28` |
| Foundation | M003 C001 canonical execution-kind predicate | closed | plans/implementation/foundation-core/003-c001-canonical-execution-kind-predicate.md | raised by the 2026-10-23 repository audit; consolidates one trust rule into one owner; closure: plans/closure/foundation-core/003-c001-closed.md |
| Foundation | M003 C002 subject-capture error fidelity | closed | plans/implementation/foundation-core/003-c002-subject-capture-error-fidelity.md | raised by the 2026-10-23 repository audit; diagnostics fidelity in the inspection snapshot and CLI read path; closure: plans/closure/foundation-core/003-c002-closed.md |
| Foundation | M003 C003 core assessment reason schema fail-closed parity | closed | plans/implementation/foundation-core/003-c003-core-assessment-reason-schema-fail-closed.md | raised by the 2026-10-23 repository audit; nested `AssessmentReason` unknown-field parity; closure: plans/closure/foundation-core/003-c003-closed.md |
| Foundation | M003 C004 unreachable public surface + panic-path hygiene | closed | plans/implementation/foundation-core/003-c004-unreachable-public-surface-and-panic-path-hygiene.md | raised by the 2026-10-23 repository audit; non-blocking maintenance; closure: plans/closure/foundation-core/003-c004-closed.md |
| Foundation | M003 C005 core boundary guard + tooling-governance reconciliation | closed | plans/implementation/foundation-core/003-c005-core-boundary-guard-and-tooling-governance-reconciliation.md | raised by the 2026-10-23 repository audit; tooling hardening and design-record truthfulness; closure: plans/closure/foundation-core/003-c005-closed.md |
| Projection/CLI | M003b C001 CLI user-documentation contract corrections | ready | plans/implementation/projection-cli/003b-c001-cli-user-documentation-contract-corrections.md | raised by the 2026-10-23 repository audit; non-blocking documentation correction; no closure record yet |
| Projection/CLI | M003b C002 shell completion depth parity | ready | plans/implementation/projection-cli/003b-c002-shell-completion-depth-parity.md | raised by the 2026-10-23 repository audit; completion depth and test strength; no closure record yet |
| Projection/CLI | M003b C003 Markdown interchange panic-path + bound hygiene | ready | plans/implementation/projection-cli/003b-c003-markdown-interchange-panic-path-and-bound-hygiene.md | raised by the 2026-10-23 repository audit; non-blocking maintenance; no closure record yet |
| Projection/CLI | M004 plan lifecycle transitions + plan structure editing | proposed | plans/implementation/projection-cli/004-plan-lifecycle-and-structure-editing.md | capability gap against the normative lifecycle and the roadmap completion criterion; decisions D1-D6 are open, so it is not dependency-ready |

## Corrective history and current maintenance

Evidence M002 C001 and C002 are historically closed. C002 made the closure
subject authority boundary mechanically true in the public API and reconciled
C001's hosted qualification evidence.

Recorded hosted evidence remains:

- C001 implementation run: 35998715018
  - Linux 107629794960
  - macOS 107629795068
  - Windows 107629794971
  - Rust 1.89 107629794849
- C002 implementation run: 36004813178
  - Linux 107650129619
  - macOS 107650129837
  - Windows 107650129154
  - Rust 1.89 107650129645

The non-blocking maintenance correction is closed:

- Evidence M002 C003 — correct the C002 implementation-SHA citation and
  strengthen the static closure-authority guard/negative proof.

C003 does not gate Projection/CLI M002, CodeGG M002, or Eggstack M002.

Two corrective plans are registered and open as of `60a5f92`, both raised by a
code-verified architecture review rather than by a failed or skipped check. All
planned verification was run and green at that baseline; both defects are
inference- and reachability-established, not test failures.

- Evidence M002 C004 — closed at `623dde9`. Ordinary compare-and-swap accepted a
  `Closed` → `Closed` rewrite at `crates/eggplan-repo/src/store.rs:930-932`:
  the guarded-closure guard only fired when the current status was not `Closed`,
  and the transition check short-circuited on equal statuses, so
  `plan_transition_allowed` was never consulted. `atomic_write` committed at
  `revision + 1` before the re-read failed `ClosureRecord::validate`, which pins
  revision and digest, so the plan and the whole state root stopped opening with
  no recovery path. Blast radius was durable availability, not closure bypass: no
  record was forged and no authority was gained. An unconditional guard now
  refuses any CAS against a Closed Plan before `atomic_write`, with the typed
  `RepoError::ClosedPlanImmutable` (CLI code `closed_plan_immutable`).
  `plan_transition_allowed` is unchanged — `Active` → `Closed` stays legal
  because the guarded finalizer performs that transition. The Evidence/closure
  gate is lifted.
- CodeGG M003 C002 — closed at `8c4f6e6`. The source guard matched only
  process, network, and database access, so a `std::fs`, `File::open`, or
  `read_to_string` in the bridge source would have passed CI. `no_impure_source`
  now matches `std::fs`, `std::path`, `tempfile::`, the `use`-imported `fs::`
  call forms, and the `File::` constructors, covering both the
  fully-qualified and imported forms. The manifest guards stay unscoped by
  deliberate decision and their messages now state the section actually scanned.
  The bridge source was always pure — `src/lib.rs` imports only
  `std::collections` — so the defect was in the enforcement, not the crate.
  No crate file changed.

C004 and C002 are both closed. Neither gated the terminal CodeGG roadmap or
any other subsystem.

### Correctives raised by the 2026-10-23 repository audit

A repository-wide read-only audit was performed against baseline
`37ad290ca6b971a68d2871aedb39f4882f4eb2a8`. All planned verification was run
and green at that baseline — `cargo fmt --all -- --check`,
`cargo clippy --workspace --all-targets --locked -- -D warnings`,
`cargo test --workspace --locked`, and all five boundary guards. Every
finding below is inference- and reachability-established against the source,
not a test failure, and each carries a recorded reproduction where one was
possible. No production code, schema, or document was changed by the audit
itself.

Eight plans are registered from that audit. Seven are corrective lines against
closed milestones; one is a proposed capability milestone. None of the
correctives is blocking, and none gates another subsystem.

**Status:** the five Foundation corrective lines (M003 C001-C005) have been
implemented and are closed with their own closure records. The three
Projection/CLI corrective lines (M003b C001-C003) and the proposed Projection/CLI
M004 milestone remain open. The findings below are kept as the audit's record
of what was found; each closed line names its closure record.

**Foundation core/repository** — five corrective lines against M003, all closed:

- **M003 C001** — the execution-evidence kind set is implemented in three
  places (`crates/eggplan-core/src/lib.rs:33-42`,
  `crates/eggplan-codegg-compat/src/lib.rs:598-607`,
  `crates/eggplan-integrations/src/lib.rs:318-325`) with no exhaustive owner,
  so a new `EvidenceKind` variant would not fail compilation and could reach
  the evidence ledger without a verification binding. The classification
  itself is correct and unchanged. **Closed** —
  `plans/closure/foundation-core/003-c001-closed.md`. The classification now has
  one owner, `eggplan-core::is_execution_evidence`, whose exhaustive `match`
  makes a new `EvidenceKind` variant a compile error until it is classified.
- **M003 C002** — the Git subject capture error is discarded at
  `crates/eggplan-repo/src/store.rs:1154-1155` and at
  `crates/eggplan-cli/src/lib.rs:1636`. A failed start capture followed by a
  successful recapture is reported as `SubjectDrift`
  (`crates/eggplan-repo/src/snapshot.rs:329-343`), and the CLI read path
  reports a generic warning where the closure path reports the specific
  cause. Diagnostics fidelity only; no read that failed before now succeeds.
  **Closed** — `plans/closure/foundation-core/003-c002-closed.md`. A start-capture
  failure is now reported as the capture failure it is and never as
  `SubjectDrift`; the CLI read path reports the specific cause under the
  existing `subject_unavailable` code. One user-visible change is recorded in
  the closure evidence: `show` on an unavailable subject now fails with
  `subject_unavailable` instead of exiting 0 with a subjectless summary.
- **M003 C003** — `AssessmentReason`
  (`crates/eggplan-core/src/assessment.rs:30-54`) does not deny unknown
  fields while all four assessment structs that contain it do, and the
  injected-field regression at `crates/eggplan-core/src/closure.rs:585-596`
  enumerates only struct-level paths. Schema-integrity gap, not a closure
  bypass. **Closed** — `plans/closure/foundation-core/003-c003-closed.md`. The
  nested enum now denies unknown fields, with all 21 variants proven to
  round-trip under the strict rule; the bridge's outward projection contract
  was decided explicitly (kept open) and pinned by a test.
- **M003 C004** — unreachable public surface and panic-path hygiene:
  `InspectionSelection::is_repository_wide` has no consumer,
  `RepoError::Domain` is never constructed, and `Criterion::validate`
  contains an `expect` on a path reachable from parsed input. Non-blocking
  maintenance; no observable behavior changes. **Closed** —
  `plans/closure/foundation-core/003-c004-closed.md`. The unreachable accessor
  and the unconstructible error variant are removed, the production `expect` is
  now a typed error, and the residual lock race is recorded as an explicit
  accepted risk rather than left implicit.
- **M003 C005** — `scripts/check-core-boundary.sh` is the only guard without a
  tool precondition and the only one that does not scan core's production
  source for impure access. Both were reproduced: without ripgrep the guard
  reports a pass, and a synthetic core source containing `std::fs` passes the
  guard. `architecture/deep-dive-tooling-governance.md` also misdescribes the
  integrations guard's mechanism, guard count, scan scope, and precondition.
  The core crate itself is pure; the defect was in the enforcement.
  **Closed** — `plans/closure/foundation-core/003-c005-closed.md`. The core
  guard now refuses to report a pass without its tool, scans core's production
  source for impure access, and carries deterministic self-proofs; the
  tooling-governance deep dive now describes the integrations guard correctly.

**Projection/CLI** — three corrective lines against M003b, plus one proposed
milestone, all still open:

- **M003b C001** — four statements in `docs/cli-reference.md` contradict the
  implementation, including the safety-relevant claim that `--state-root` is
  "Never implicit" while `crates/eggplan-cli/src/lib.rs:320-324` defaults it
  to `.eggplan`. `init` and `help` are undocumented, and the value-taking
  flag list is incomplete. Non-blocking documentation correction; the CLI's
  behavior is correct and self-consistent.
- **M003b C002** — the four generated completion scripts consume the one
  command metadata source to different depths. Fish reaches commands,
  subcommands, options, and values; Bash omits subcommands; Zsh omits
  subcommands, options, and values; PowerShell omits subcommands and values.
  The test at `crates/eggplan-cli/tests/batch_and_completions.rs:1212-1241`
  asserts only that a script mentions a name, so it cannot detect the gap.
- **M003b C003** — the Markdown import path contains the workspace's only
  production `unreachable!()`
  (`crates/eggplan-markdown/src/lib.rs:114`), and the 512-character bound is
  a repeated bare literal at six sites rather than a named constant.
  Non-blocking maintenance; imported plans and rendered documents are
  unchanged.
- **M004** — capability, not a corrective. `PlanStatus::Blocked` and
  `PlanStatus::Cancelled` are unreachable through the CLI although the domain
  permits them (`crates/eggplan-core/src/model.rs:391-403`), and the CLI
  cannot change a plan's items, dependencies, criteria, requirements, or
  objective at all, which the roadmap's completion criterion
  (`plans/subsystems/projection-cli-roadmap.md:96-99`) rules out. The plan
  documents decisions D1-D6 and is registered **proposed**, not ready.

Two items from the audit are explicitly *not* findings and are recorded here
so they are not reopened: the deferred Interop/distribution subsystem
(`plans/subsystems/interoperability-distribution-roadmap.md`), which is out
of scope by design; and the fact that `eggplan-integrations` and
`eggplan-codegg-compat` have no in-repo consumer, which follows from the
host-owned acquisition boundary and from CodeGG being an external consumer.


## Current execution order

1. Foundation M001/M002/M003 — closed/current foundation.
2. Evidence M001 + C001 — closed; v2 verification binding is current.
3. Evidence M002 + C001 + C002 + C003 + C004 — closed guarded closure, subject
   authority, guard hygiene, and Closed-Plan CAS immutability. The C004
   durability defect is resolved, so the Evidence/closure gate is lifted and
   Projection/CLI M002 and Eggstack M002 are no longer blocked behind it. No
   Evidence/closure corrective is currently open.
4. CodeGG Integration M001 — historically closed.
5. Eggstack Provider SPI M001 — historically closed.
6. Projection/CLI M001 — historically closed.
7. CodeGG M002 is closed on both sides. The upstream provenance predecessor is
   `eggplan-assessment-integration/001-durable-execution-subject-provenance.md`
   **in the CodeGG repository `dbowm91/codegg` — not a path in this
   repository.** It closed there at `418fdc85656e7e1faa57f71e5e7f10f7f4859c60`
   (hosted CI run `36106606574` green) and made subject provenance
   attempt-scoped. Eggplan then closed its pure bridge (088968b, test matrix
   1291799), and CodeGG closed staged adoption in `3e992291`/ffa1c15e with
   hosted canonical run `36760308368`. Differential adoption and
   verification-digest derivation are no longer outstanding.
8. Projection/CLI M002 and Eggstack M002 are closed, independent of the CodeGG
   provenance handoff. Eggstack M003 is ready for planning against the
   rechecked Eggbench contract at `d870512a5a1af16276ff05286ff0b6e2366b7f8f`.
   Evidence M002 C003 closed as non-blocking hygiene without gating either
   capability plan.
9. After positive M002 closures:
   - Eggplan's M003 pure contract is closed: hosted qualification at
     `3f7c603` and the CodeGG consuming implementation landed at CodeGG
     `53dea47f`, hosted-qualified there by run `36938461935` (success). Both
     sides of the cross-repository contract are closed; the CodeGG
     integration roadmap is terminal for M003.
   - CodeGG M003 C001 dirty-subject corrective is closed on both sides
     (Eggplan contract `352a0f7`, qualified `0dd33b7`; CodeGG `36ec9322` plus
     the pin bump in `3623f65e`/`b470865a`). The CodeGG integration roadmap is
     terminal; Projection/CLI M003, Eggstack M003, and other capability work
     never had to serialize on it;
   - CodeGG M003 C002 boundary-guard completeness is closed at `8c4f6e6`. It
     hardened the static ownership guard only; the bridge was already pure, the
     roadmap stayed terminal, and nothing serialized on it;
   - Projection/CLI M003 is now split into ready M003a inspection/performance and M003b batch/compact/completion plans; completion/projection work may proceed in parallel while repository-wide batch execution consumes the M003a read model;
   - Eggstack M003a is closed at Eggplan `674264d`, hosted-qualified by run `37350383159`. The dual-green gate was rechecked on 2026-10-05 and `30a38251...` remains the newest exact Eggbench SHA with both ordinary CI and live qualification green; every revision above it on main has both workflows failing or cancelled, so the current head stayed research evidence. M003b was never serialized behind that gate, landed independently, and is closed at Eggplan `93d65b0` hosted-qualified by run `37351818356`. It was qualified against GitHub REST API version `2022-11-28` using both live authenticated responses and GitHub's published OpenAPI description. That cross-check corrected an assumption in the plan: GitHub does supply `artifact.digest` as `sha256:<hex>`, but only for `upload-artifact` v4 or newer and null otherwise, so the adapter records it when present and never infers one.
10. Interoperability/distribution remains deferred until the local capability
    wave is qualified.
11. The 2026-10-23 repository audit raised five Foundation corrective lines
    (M003 C001-C005) and three Projection/CLI corrective lines (M003b
    C001-C003), plus proposed Projection/CLI M004. Every one of them is
    non-blocking: the C001-C005 lines do not gate CodeGG, Eggstack, or
    Projection/CLI capability work, and the M003b lines do not gate M004.
    The Foundation lines touch `eggplan-core`, `eggplan-repo`,
    `eggplan-integrations`, and `eggplan-codegg-compat`; the Projection/CLI
    lines touch `eggplan-cli` and `eggplan-markdown`. They are independent of
    each other except that M003 C002 and M003b C001 both correct
    subject-capture diagnostics, and M003b C002's completion-depth
    assertions will need to cover any command added by M004. M004 remains
    **proposed** until decisions D1-D6 in its plan are made.
    **The five Foundation lines are now implemented and closed**, each with its
    own closure record under `plans/closure/foundation-core/`. They did not
    serialize against each other or against any capability work: M003 C001,
    C003, and C004 closed together in the core and repository crates, M003
    C002 closed with the CLI read-path diagnostics change recorded in its
    closure evidence, and M003 C005 closed with the hardened core boundary
    guard. The three Projection/CLI lines and M004 remain open and are
    unaffected by the Foundation closures, except that M003b C001's
    documentation corrections should now reflect the C002 read-path change.

## External interface research baselines

Reviewed during planning; these are not dependency pins.

| Project/standard | Reviewed baseline | Relevant boundary |
|---|---|---|
| CodeGG | 3623f65e + b470865a (C001 pin bump and fingerprint consumption, on `main`); 36ec9322 (C001 provenance v2); b5b3b14e (C001 conditional closure, later closed); 53dea47f414641c3f9756c3f8f181f6208be8115 (M003 implementation); aa21cfe1763d7ea00d11e582ed4108233e4088a9 (M003 closure) | M001-M003 and C001 closed on both sides. Post-closure review found dirty-digest representation mismatch for bound exact-subject evidence; Eggplan's bounded repository-ID-free fingerprint contract landed at `0dd33b7` and CodeGG consumes it at both capture sites on the same pin. CodeGG integration is terminal. |
| Eggwork | faaa0b905fa6bc43e46825fdd98530b5533a970f | protocol-neutral execution snapshots/results/generation/artifact records; executor/scheduler remain outside Eggplan |
| Eggsearch | dfa90e050c5434f3346902aeb4074901c58e90d1 | deterministic EvidenceBundle source/provider/trust/gap metadata; full runtime must not become an Eggplan dependency |
| Eggbench | current reviewed head 1a5e03610f5c70218501d9e1dca7a101ae1c7870 (both workflows failing); implemented fixture baseline 30a38251bccb5157beb68202ffe630f6253771e0 (CI 37143714313, live 37143714261) | manifest v2 remains current; comparison receipt v4; execution, performance/correctness, and aggregate verdict semantics remain distinct. M003a rechecked on 2026-10-05 and froze the newest compatible dual-green exact SHA as its fixture baseline; `inspect --json` exposes no manifest digest, so the verifying host supplies it |
| Eggsact | 40959b704431430668e9ca2bfe959a8ef32495d8 | deterministic in-process/preflight utilities |
| Eggup | cf5b3d3819c168eb2dbf841daa8332f3eb28c915 | future verified distribution/update consumer |
| SLSA | v1.2 approved provenance docs reviewed 2026-09-22 | subject + provenance separation |
| in-toto | attestation framework stable v1.0 reviewed 2026-09-22 | statement/predicate and signed supply-chain evidence model |
| GitHub attestations | docs reviewed 2026-09-22 | Sigstore-backed artifact provenance; validity does not imply semantic safety |

Implementation agents MUST re-check current sibling interfaces when the
integration milestone is actually handed off.

## Key design gates

1. Exact canonical JSON normalization/digest rules must be frozen in Foundation
   M001 before evidence digests depend on them.
2. Dirty Git subject identity must be deterministic and bounded before evidence
   can claim exact-revision validity.
3. Repository persistence must prove atomicity/recovery/CAS before multiple
   agents use it concurrently.
4. Evidence provider identity is host/adapter authority, never caller prose.
5. A passing observation for stale source remains historical evidence, not
   current closure proof.
6. CodeGG migration must preserve Goal/Todo/checkpoint/context-epoch ownership
   and must not turn Eggplan into CodeGG's WorkOrder scheduler.
7. Eggbench bundle reuse should be preferred over inventing another large
   immutable bundle format.
8. Eggplan's own managed state must not perturb the source SubjectRevision used
   to judge evidence applicability.
9. Execution-derived evidence must bind to the verification specification it
   actually observed; kind/provider/subject matching alone is insufficient for
   specific verification authority.
10. A canonical Closed Plan must have a valid guarded ClosureRecord; ordinary
    CAS must never be a bypass around closure assessment.
11. Evidence correction is append-only lineage. Historical observations are
    never rewritten to make a later assessment pass.
12. Guarded closure finalization owns its current SubjectRevision capture;
    callers may propose a candidate subject but cannot assert current subject
    authority at commit time. `RepositoryStore::finalize_closure` no longer
    accepts a caller-supplied current subject.
13. The finalizer revalidates subject stability immediately before its first
    canonical closure write; arbitrary external worktree writers remain
    outside Eggplan's lock and later changes make closure historical/stale, not
    corrupt. Subject mismatch, drift, and capture failure are typed
    `RepoError` variants distinct from invalid-update / lifecycle / corrupt
    errors.
14. Test seams are not authority seams: no downstream/public API may inject the
    SubjectRevision source used by guarded closure finalization. `#[doc(hidden)]`
    is not an access-control boundary.
15. Closure records may cite only observed completed verification; placeholder
    workflow/job IDs are not passing evidence.
16. Markdown import is plan-intent ingestion only: source lifecycle, prose,
    evidence claims, provider trust, SubjectRevision, and closure are not
    authority.
17. CodeGG staged adoption must use a pure Eggplan assessment bridge in
    production, keep CodeGG storage/runtime ownership, and derive execution
    VerificationDigest values from authoritative native execution specs rather
    than prose/reference IDs.
18. Eggsearch trust labels are content provenance, never Eggplan provider
    enrollment; Eggwork execution verification binding is host-supplied.
19. Sibling commit SHAs recorded by compatibility/adapters are fixture/review
    provenance, not runtime trust or branch dependencies.
20. Historical CodeGG execution subjects must come from durable attempt-time
    provenance. Eggplan integration must never reconstruct an older job/run
    subject from CodeGG's current worktree.
21. Repository Plan binding must prove CodeGG workspace identity and Eggplan
    `epr_*` identity refer to the same Git subject before translation; blind
    SubjectRevision repository-id relabeling is forbidden.
22. For a bound plan, Eggplan repository state is canonical for plan/item
    lifecycle, evidence, and closure. CodeGG may keep a durable runtime mirror
    only with explicit reconciliation semantics; cross-store atomicity must
    never be claimed.
23. Bound dirty CodeGG evidence must carry the exact Eggplan-compatible dirty
    digest captured at execution time. CodeGG-native dirty digest bytes are
    not interchangeable with Eggplan SubjectRevision dirty digests, and
    current-worktree backfill for historical attempts is forbidden. The
    Eggplan-compatible value is obtained from
    `eggplan_repo::capture_git_subject_fingerprint` at the exact Eggplan
    revision the host pins; those digest bytes are frozen by
    `crates/eggplan-repo/tests/git_subject_digest_golden.rs`, so a host that
    derives a different digest fails loudly rather than silently.

## Planning hygiene

- Register before handoff.
- Preserve historical closure; use corrective plans.
- Record exact evidence and unrun/blocked checks.
- Do not copy Eggwork/Eggsearch/Eggbench responsibilities into eggplan-core.
- Keep core synchronous/deterministic where practical; async/network belongs in adapters.
- No hidden model reasoning in persisted schemas.
- Mark maintenance gates explicitly: non-blocking hygiene must not serialize
  otherwise independent capability handoffs.
