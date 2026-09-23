# ADR-0004: CodeGG Extraction Without WorkOrder Conflation

Status: accepted

Date: 2026-09-22

## Context

CodeGG currently has two distinct planning-related domains.

Project-level WorkOrder controls release/scheduling/materialization of future
sessions. Session-local WorkPlan records detailed work state, dependencies,
acceptance/evidence, and completion arbitration.

The reusable seed for Eggplan lives primarily in codegg-core::work_plan and in
the repository plans/ lifecycle, not in CodeGG's WorkOrder scheduler.

A direct extraction risks circular repository dependencies or a flag-day
migration.

## Decision drivers

- Preserve CodeGG's established ownership.
- Avoid circular dependency between CodeGG and Eggplan.
- Reuse proven WorkPlan semantics and tests.
- Improve the generic evidence model without breaking CodeGG immediately.
- Allow Eggplan to be useful independently.

## Decision

Eggplan uses Plan/PlanItem terminology. It does not reuse WorkOrder as its core
type name.

The first implementation independently establishes eggplan-core based on the
documented/golden semantics rather than importing CodeGG crates.

A later CodeGG integration milestone ports representative golden tests and
builds adapters.

Reusable candidates include:

- typed plan/item IDs;
- bounded graph validation/actionability;
- CAS revision concepts;
- acceptance/evidence separation;
- pure completion assessment;
- bounded projections.

CodeGG-owned behavior remains in CodeGG:

- project WorkOrder/occurrence/release gates;
- Session/Turn/AgentRun/Job ownership;
- Goal binding and verification policy;
- Todo projection/feedback policy;
- checkpoint/context-epoch integration;
- model tools and terminal-answer arbiter behavior;
- SQLite schema/migrations unless adapted behind Eggplan traits.

CodeGG adoption is staged only after golden semantic parity. There is no
requirement to replace every codegg-core::work_plan type in one change.

## Consequences

Positive: avoids circular dependencies and preserves CodeGG scheduler/session
boundaries.

Negative: temporary duplication exists during the parity phase and must be
tracked until one generic owner is actually adopted.

## Verification

The integration roadmap must run golden cases covering bounds, graph
actionability/cycles, revision conflicts, evidence statuses, completion
assessment, bounded projections, restart semantics where adapter-relevant, and
proof that WorkOrder scheduling is unaffected.
