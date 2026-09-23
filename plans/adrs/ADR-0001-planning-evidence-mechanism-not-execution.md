# ADR-0001: Planning and Evidence Mechanism, Not Execution

Status: accepted

Date: 2026-09-22

## Context

Eggplan is intended to extract/generalize repository work-state and evidence
semantics that are useful to CodeGG and other tooling. The surrounding
ecosystem already has distinct owners for scheduling, process execution,
remote execution, search/research, and performance evidence.

Without an explicit boundary, a "work-order engine" can easily grow into a
scheduler, CI runner, agent runtime, or generic workflow engine.

## Decision drivers

- Keep eggplan-core small, deterministic, and embeddable.
- Avoid duplicating CodeGG WorkOrder/scheduler behavior.
- Avoid duplicating Eggwork execution.
- Make the library usable without an LLM.
- Allow many evidence producers without letting them control planning policy.
- Keep completion based on structured state/evidence rather than model prose.

## Considered options

### A. Full workflow/scheduler engine

Rejected. It would duplicate existing Eggstack/CodeGG ownership and create
placement, retry, queue, lease, and trigger responsibilities unrelated to
Eggplan's evidence problem.

### B. AI-specific planning runtime

Rejected. A model may author or update plans through an adapter, but model
provider/session/transcript semantics do not belong in the core.

### C. Repository work specification + evidence + closure mechanism

Selected.

## Decision

Eggplan owns:

- typed plans/items/criteria/dependencies;
- readiness derivation;
- revision/CAS semantics;
- evidence requirements and normalized immutable observations;
- subject-revision applicability;
- deterministic assessment/closure;
- repository persistence and human/machine projections.

Eggplan does not own:

- scheduling, trigger/repeat policy, placement, fairness;
- process/remote execution;
- model invocation, prompts, transcripts, hidden reasoning;
- worktree allocation;
- CI service operation;
- generic issue/project management.

Executors/orchestrators integrate through explicit adapters.

## Consequences

Positive: clean library boundary, easier testing, reusable by CodeGG/CI/humans,
and no duplicated scheduler.

Negative: an end-to-end agent workflow needs another component to execute the
work and feed observations back.

## Compatibility

CodeGG project WorkOrder remains authoritative for "when a session may start."
Eggplan Plan describes work/evidence. CodeGG session WorkPlan may later adapt
Eggplan semantics, but no naming or ownership conflation is permitted.

## Verification

Static/dependency guards should prevent eggplan-core from importing async HTTP,
MCP, process-supervision, scheduler, or model SDK dependencies without a later
ADR.
