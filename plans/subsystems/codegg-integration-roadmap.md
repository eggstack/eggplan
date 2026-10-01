# CodeGG Integration Roadmap

Status: closed/current; M003 dependency-ready

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

Status: ready for coordinated handoff.

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
CodeGG-local repository-binding implementation plan.

## 5. Verification

Golden parity, stale writer races, invalid evidence attribution, bounded
projection, restart adapter behavior, Goal/Todo/checkpoint regressions, and
WorkOrder scheduling invariance.

## 6. Completion

CodeGG can reuse Eggplan as a generic planning/evidence substrate without
losing runtime ownership or creating duplicate scheduler state.
