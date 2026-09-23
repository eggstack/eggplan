# Eggplan Planning Process

Status: normative process for this repository

## 1. Purpose

This process converts long-term architecture into bounded implementation work
and requires evidence-based closure. It mirrors CodeGG's planning conventions
while keeping Eggplan-specific evidence rules explicit.

## 2. Authority order

When documents conflict, use this order:

1. current long-term specification and terminology;
2. accepted ADRs;
3. master roadmap;
4. subsystem roadmap;
5. implementation plan;
6. closure/corrective records for what actually landed;
7. registry as compact projection.

An implementation plan cannot silently override a long-term invariant or
accepted ADR.

## 3. Plan lifecycle

1. Research the current repository baseline and relevant sibling interfaces.
2. Identify governing long-term sections and ADRs.
3. Confirm the subsystem roadmap owns the work.
4. Resolve architecture decisions before handoff.
5. Write one bounded implementation plan.
6. Register it before execution.
7. Implement and verify.
8. Write a closure record using actual evidence.
9. Update roadmap and registry.
10. Use a new corrective plan for later findings; preserve historical closure.

## 4. Required classifications

Every subsystem roadmap and implementation plan distinguishes:

- Invariant — must remain true.
- Capability — externally usable behavior.
- Infrastructure — internal machinery required by capability.
- Polish — ergonomics, performance, diagnostics, docs, cleanup.

Infrastructure is not a completed capability merely because code exists.

## 5. Implementation-plan readiness

A plan may be marked ready only when:

- its hard dependencies are closed or already present;
- current external interfaces have been re-checked;
- no unresolved ownership decision can materially change the implementation;
- scope is bounded to one coherent pass;
- failure/restart/contention/security semantics are explicit where applicable;
- required verification can actually establish the claimed boundary;
- closure evidence is named.

Repository paths may be suggested, but plans should specify ownership and
behavior rather than blindly prescribing edits against a stale tree.

## 6. Baselines

Every implementation plan records a repository baseline SHA or, for the first
empty/bootstrap state, explicitly says so.

External integration plans record reviewed sibling baselines, but those are
research baselines rather than dependency pins. Re-check them at handoff.

## 7. Evidence discipline

A closure record distinguishes:

- planned commands;
- commands actually run;
- pass;
- fail;
- timeout;
- environmental block;
- skipped;
- not run;
- unavailable external evidence.

Never convert a planned command into a passing result because it appears in the
source plan.

When possible, record immutable command/job/run/artifact identifiers and the
subject revision against which they executed.

## 8. Closure rule

A milestone is closed only when its user/caller-visible contract or explicit
infrastructure exit condition is satisfied and the required evidence exists.

Compilation or formatting alone cannot close a correctness, security,
persistence, recovery, or integration milestone.

Conditionally closed is permitted only when production implementation is
complete and the missing external/operational evidence is explicitly named and
bounded. The underlying missing evidence remains missing.

## 9. Corrective work

A later finding does not rewrite an accepted historical closure except for
factual errata.

Create a corrective plan that:

- references predecessor plan/closure;
- enumerates every unclosed finding;
- identifies current controlling semantics;
- adds regression evidence that would have detected the defect;
- updates registry/roadmap lineage.

## 10. Registry requirements

plans/registry.md is the current compact control surface. It must contain:

- canonical direction;
- status vocabulary;
- accepted ADRs;
- subsystem status;
- dependency-ready implementation plans;
- blocked/roadmap-level work;
- external integration baselines;
- current execution order.

The registry links to authority; it should not duplicate full plan content.

Eggplan's future product may generate registries from canonical runtime state.
Until that capability exists, this repository registry is maintained manually.

## 11. Implementation-plan template

Each plan contains:

- status;
- repository baseline;
- source roadmap;
- long-term requirements;
- applicable ADRs;
- primary class;
- objective;
- readiness/dependencies;
- current implementation evidence;
- invariants;
- in-scope/out-of-scope;
- required production changes;
- ordered work packages;
- failure/cancellation/restart/contention semantics;
- compatibility/migration;
- required tests;
- exact verification commands;
- documentation;
- acceptance criteria;
- stop conditions;
- closure evidence required;
- handoff notes.

## 12. Closure-record template

Each closure contains:

- status;
- source plan and roadmap;
- reviewed baseline;
- implementation commits/PRs;
- executive finding;
- requirement-to-evidence matrix;
- production evidence;
- exact verification executed and results;
- invariant review;
- failure/recovery review;
- migration/compatibility review;
- security review;
- docs/operations;
- unresolved findings with severity;
- roadmap disposition;
- registry updates.

## 13. Stop conditions

An implementation agent must stop/report rather than improvise when:

- a required architecture decision is unresolved;
- a hard dependency is absent;
- current sibling interfaces contradict the plan;
- safe migration cannot be achieved;
- scope would create a scheduler/executor/model-planner inside Eggplan;
- evidence authority would depend on trusting model/free-form prose;
- the required verification cannot be obtained honestly;
- user changes would need destructive replacement.

## 14. Planning anti-patterns

Do not:

- treat Markdown status words as sufficient machine evidence;
- duplicate execution/scheduling infrastructure to avoid an adapter;
- add transient implementation checklists to canonical long-term documents;
- write many speculative implementation plans before interfaces exist;
- silently weaken bounds to make tests pass;
- allow stale evidence to satisfy a newer subject by default;
- overwrite historical observations or closure evidence;
- equate a digest with authenticity.
