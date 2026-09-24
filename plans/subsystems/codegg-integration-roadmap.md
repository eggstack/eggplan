# CodeGG Integration Roadmap

Status: active / ready

Long-term references: plans/000-long-term-specification.md section 16.

Related ADR: ADR-0004.

Initial reviewed CodeGG baseline: 6e18304546f29426457eb11410957288384fdec2.
M001 execution-time baseline re-checked 2026-09-24:
28b4695661d463dd1675d045ac6299c5fbc9ea31.
M002 planning baseline re-checked 2026-09-24:
a3c87fc18ee55aaf630401a562c11bb83112fd82.

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

Status: ready for handoff.

Plan: plans/implementation/codegg-integration/002-staged-eggplan-assessment-adoption.md

M001 remains historically closed. Current CodeGG was re-checked at
`a3c87fc18ee55aaf630401a562c11bb83112fd82`; relative to the M001 fixture
baseline, the WorkPlan implementation contract remains stable and only a
WorkPlan foundation test changed in the reviewed compare range.

M002 stages Eggplan's pure plan/evidence assessment semantics behind CodeGG's
existing assessment surface. CodeGG keeps SQLite WorkPlan storage, Goal/Todo,
checkpoint/context-epoch, scheduler, worktree, and agent-loop ownership. The
production bridge must not depend on eggplan-repo and must not invent
verification identity from prose or native reference IDs.

### M003 — Repository Plan binding

Later capability: allow a CodeGG WorkOrder or session to reference an Eggplan
Plan and feed CodeGG job/run/test/artifact observations back into Eggplan.

This must not make Eggplan the CodeGG scheduler.

## 5. Verification

Golden parity, stale writer races, invalid evidence attribution, bounded
projection, restart adapter behavior, Goal/Todo/checkpoint regressions, and
WorkOrder scheduling invariance.

## 6. Completion

CodeGG can reuse Eggplan as a generic planning/evidence substrate without
losing runtime ownership or creating duplicate scheduler state.
