# CodeGG Integration Roadmap

Status: blocked

Long-term references: plans/000-long-term-specification.md section 16.

Related ADR: ADR-0004.

Reviewed CodeGG baseline:
6e18304546f29426457eb11410957288384fdec2.

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

Status: blocked on Evidence M002 closure.

Plan: plans/implementation/codegg-integration/001-golden-parity-and-adapter-seam.md

Current CodeGG interfaces were re-checked at
6e18304546f29426457eb11410957288384fdec2. After Evidence M002 closes,
port representative WorkPlan fixtures through a non-circular compatibility
crate and define explicit identity/status/evidence mapping. No production
CodeGG ownership change is required.

### M002 — Staged core adoption

Blocked on positive M001.

Make CodeGG consume Eggplan generic domain/assessment where it reduces
duplication. Keep CodeGG storage and runtime policy adapters.

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
