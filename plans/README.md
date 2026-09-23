# Eggplan Planning System

This directory separates durable architectural direction from temporary
execution planning. It intentionally mirrors the planning hierarchy used by
CodeGG and newer Eggstack repositories.

## Canonical long-term documents

- 000-long-term-specification.md — normative end-state product and architecture.
- 001-terminology-and-domain-model.md — normative language, identities, and state.
- 002-long-term-roadmap.md — dependency-ordered capability roadmap.
- 003-planning-process.md — rules for deriving, handing off, and closing work.
- registry.md — compact current control surface.

Long-term documents describe what Eggplan is becoming and what must remain
true. Implementation plans describe the next bounded change against a concrete
repository baseline.

## Planning hierarchy

    Long-term specification and terminology
            |
            v
    Architecture decision records
            |
            v
    Master roadmap
            |
            v
    Subsystem roadmaps
            |
            v
    Milestone implementation plans
            |
            v
    Implementation and verification
            |
            v
    Closure records and archive

## Directory roles

- adrs/ — durable architecture decisions. Accepted decisions are superseded,
  not silently rewritten.
- subsystems/ — subsystem ownership and dependency roadmaps.
- implementation/ — bounded implementation-agent handoffs.
- closure/ — requirement-to-evidence records and residual risk.
- archive/ — completed or superseded interim planning retained for traceability.
- registry.md — status, dependencies, external interface baselines, and next handoff.

## Core planning rule

A milestone is not complete because code exists, compiles, or an agent says it
is done. Closure requires the evidence named by the source plan.

Every roadmap and implementation plan must distinguish invariants,
capabilities, infrastructure, and polish. Infrastructure must not be presented
as user-visible capability until its acceptance criteria are actually met.

## Naming conventions

- ADR: adrs/ADR-NNNN-short-title.md
- Subsystem roadmap: subsystems/<subsystem>-roadmap.md
- Implementation plan: implementation/<subsystem>/NNN-short-title.md
- Closure record: closure/<subsystem>/NNN-status.md

Register an implementation plan before handing it to an implementation agent.
