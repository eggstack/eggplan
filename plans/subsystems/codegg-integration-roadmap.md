# CodeGG Integration Roadmap

Status: M001-M003 closed; post-closure C001 dirty-subject corrective ready

Long-term references: plans/000-long-term-specification.md section 16.

Related ADR: ADR-0004.

Initial reviewed CodeGG baseline: 6e18304546f29426457eb11410957288384fdec2.
M001 execution-time baseline re-checked 2026-09-24:
28b4695661d463dd1675d045ac6299c5fbc9ea31.
M002 planning baseline re-checked 2026-09-24:
a3c87fc18ee55aaf630401a562c11bb83112fd82.
CodeGG production/planning baseline:
6ad127c9913ee6999f44295db2fc03f8b3e5063b.
Upstream provenance implementation/closure:
418fdc85656e7e1faa57f71e5e7f10f7f4859c60.
CodeGG-local M002 handoff registration:
ce088e9153b821d8473372c7c04786a4d90ab6ae.
CodeGG M002 implementation: 3e992291b6024fae29f2512ad2fa42a209fe7ca9
plus qualification follow-up 3c7438c738a142fdd0c4e79684d7d1154fe52a99.
CodeGG M002 closure: ffa1c15e654776c3ebe1022f4ce7de2582bc5d98.
Current reviewed CodeGG baseline: ffa1c15e654776c3ebe1022f4ce7de2582bc5d98.
Eggplan head pinned by CodeGG: 0d4a6af7adc6f80f975aca1bfe9bae04e2eb27d8.

## 1. Purpose

Extract and generalize CodeGG's reusable session WorkPlan semantics into
Eggplan without conflating them with CodeGG project WorkOrder scheduling or
forcing a flag-day migration.

## 2. Existing CodeGG seam

Reusable seed:

- codegg-core work_plan model
- evidence and assessment
- bounded projection
- store/revision semantics
- WorkPlan golden and integration tests

CodeGG-specific ownership to preserve:

- WorkOrder/occurrence/release coordinator;
- SQLite/session/project authority;
- Goal and GoalVerification;
- Todo projection;
- continuation checkpoint and context epoch;
- agent-loop continuation/final-answer policy;
- AgentRun, Job, worktree scheduling and execution.

## 3. Invariants

- No circular CodeGG/Eggplan dependency.
- CodeGG WorkOrder remains distinct.
- Model prose still cannot establish host evidence.
- Stale revisions remain explicit conflicts.
- Existing CodeGG sessions remain compatible during migration.
- Adoption does not silently change scheduling, Goal, Todo, permission,
  sandbox, worktree, or context behavior.

## 4. Milestones

### M001 — Golden parity and adapter seam

Status: closed.

Plan: plans/implementation/codegg-integration/001-golden-parity-and-adapter-seam.md

Current CodeGG interfaces were re-checked at
28b4695661d463dd1675d045ac6299c5fbc9ea31. The WorkPlan model, assessment,
projection, evidence snapshot, and CAS store remain available; later checkpoint
and context-epoch surfaces remain CodeGG-owned. M001 qualification is recorded
in plans/closure/codegg-integration/001-closed.md. No production CodeGG
ownership change was required.

### M002 — Staged core adoption

Status: closed.

Plan: plans/implementation/codegg-integration/002-staged-eggplan-assessment-adoption.md

Closure: plans/closure/codegg-integration/002-closed.md

M001 remains historically closed. Current CodeGG was re-checked at
`a3c87fc18ee55aaf630401a562c11bb83112fd82`; relative to the M001 fixture
baseline, the WorkPlan implementation contract remains stable and only a
WorkPlan foundation test changed in the reviewed compare range.

M002 staged Eggplan's pure plan/evidence assessment semantics behind CodeGG's
existing assessment surface. CodeGG kept SQLite WorkPlan storage, Goal/Todo,
checkpoint/context-epoch, scheduler, worktree, and agent-loop ownership. The
production bridge did not depend on eggplan-repo and did not invent verification
identity from prose or native reference IDs.

The staged plan closed on both sides:

- Eggplan shipped the pure bridge in `088968b` — live mapper split from the
  fixture wrapper, source SHA demoted to qualification provenance,
  `eggplan-repo` moved to dev-dependencies, and `assess_codegg_snapshot`
  providing a persistence-free deterministic assessment view. The §13 test
  matrix was completed in `1291799`.
- CodeGG shipped the consuming facade in `3e992291` — application-layer
  Eggplan assessment, canonical verification-spec derivation through
  `eggplan_core::digest_json`, resolved-evidence adapter, explicit engine
  selection, a 28-case differential matrix, production call-site migration, and
  S1/S2 completion revalidation. Closed in `ffa1c15e` with hosted canonical run
  `36760308368` green on the exact implementation tree.

Ownership invariance is mechanically true, not asserted:
`crates/codegg-core/src/work_plan/` is byte-identical from the fixture baseline
through the adopted head, `codegg-core` has no Eggplan dependency, and CodeGG's
production graph contains only `eggplan-core` and `eggplan-codegg-compat`. No
Eggplan repository state is created by CodeGG.

The upstream subject-provenance handoff that unblocked M002 is
`plans/implementation/eggplan-assessment-integration/001-durable-execution-subject-provenance.md`,
implemented and closed in CodeGG at
`418fdc85656e7e1faa57f71e5e7f10f7f4859c60` with hosted CI run `36106606574`
green. The CodeGG corrective C002 that made attempt-scoped subject provenance
authoritative closed at `88d6831a`.

### M003 — Repository Plan binding

Status: closed. The CodeGG consumer is implemented at CodeGG `53dea47f`
against this exact revision, with hosted canonical CodeGG run
`36938461935` (success); CodeGG closure
`dbowm91/codegg:plans/closure/eggplan-assessment-integration/004-m003-status.md`.

Plan:
`plans/implementation/codegg-integration/003-repository-plan-binding-contract.md`

M003 adds the pure reverse compatibility contract needed for CodeGG to mirror
an existing repository Plan without making the compatibility crate a
repository client. Repository Plan structure/lifecycle/evidence/closure remain
Eggplan authority; CodeGG owns execution/session state and persists the runtime
binding/mapping.

The Eggplan side provides a bounded repository-Plan projection with exact
lifecycle mapping plus deterministic structural intent/projection digests. It
does not rewrite SubjectRevision repository identity: CodeGG must prove its
workspace subject and Eggplan repository subject describe the same Git state
before persisting an identity association.

This must not make Eggplan the CodeGG scheduler. M003 inherits the M002
verification-binding and ownership invariants and coordinates with the
CodeGG-local plan registered at
`440403e82304537f6ed9de22103987b16df6f642`:
`plans/implementation/eggplan-assessment-integration/004-repository-plan-binding-and-writeback.md`.

The Eggplan-owned contract is implemented at `3f7c603315131bb169bfdd2bb575531d228532b1`
and qualified by hosted CI run `36868055136` across Ubuntu, macOS, Windows,
and Rust 1.89. The CodeGG consumer completed the durable binding, identity
proof, evidence writeback, reconciliation, guarded-closure, and
cross-repository qualification work, so the condition that made the Eggplan
closure conditional is satisfied and both sides are closed.

### C001 — Dirty-subject fingerprint contract and bound-evidence requalification

Status: ready for coordinated handoff.

Plan:
`plans/implementation/codegg-integration/003-c001-dirty-subject-fingerprint-and-bound-evidence-requalification.md`

Post-closure review found that the CodeGG consumer's native dirty digest and
Eggplan's repository dirty digest are deliberately different canonical
encodings. Binding-time HEAD + clean/dirty equality is therefore insufficient
to prove stable dirty contents, and historical CodeGG dirty provenance cannot
be copied into an Eggplan SubjectRevision for exact-subject assessment.

C001 preserves M003 history and the existing Eggplan subject digest algorithm.
Eggplan exposes a bounded repository-ID-free fingerprint using the exact
existing digest bytes; CodeGG persists that fingerprint alongside its native
attempt provenance and uses it only for bound repository subject translation.
Legacy dirty provenance fails closed rather than being backfilled.

The corrective also requires dirty execution -> observation -> item completion
-> guarded closure qualification and reconciles stale M003 status/baseline
documentation. Clean M003 behavior remains closed/current.

## 5. Verification

Golden parity, stale writer races, invalid evidence attribution, bounded
projection, restart adapter behavior, Goal/Todo/checkpoint regressions, and
WorkOrder scheduling invariance.

## 6. Completion

CodeGG can reuse Eggplan as a generic planning/evidence substrate without
losing runtime ownership or creating duplicate scheduler state.
