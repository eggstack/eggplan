# Projection and CLI Roadmap

Status: blocked

Long-term references: plans/000-long-term-specification.md sections 14-15 and 19-20.

Related ADR: ADR-0002.

## 1. Purpose

Provide the human and machine control surface over stable Eggplan core,
repository, and evidence semantics.

## 2. Ownership

Owns eggplan-cli, JSON output envelopes, derived registry/readiness views,
graph/check diagnostics, and CodeGG-style Markdown import/render.

It does not own canonical domain semantics or execution.

## 3. Invariants

- CLI is thin over libraries.
- JSON is stable/versioned where machine consumption is intended.
- Human rendering cannot hide remaining or failed work.
- Generated registry is derived from canonical state.
- Markdown import never manufactures evidence from prose.

## 4. Milestones

### M001 — CLI control surface and derived registry

Status: closed.

Plan: plans/implementation/projection-cli/001-cli-control-surface-and-derived-registry.md

Bounded projections, native CLI, guarded closure, explicit provider policy,
deep integrity checks, and derived registry output are closed. Evidence:
plans/closure/projection-cli/001-closed.md.

### M002 — Markdown import/render

Status: ready to plan (Evidence M002 C002 has closed).

M001 remains historically closed. Evidence M002 C002 has closed: the
finalization subject capture seam is now crate-internal, no downstream crate
can substitute closure subject authority, the C001 closure evidence is
reconciled with the actual hosted workflow, and `RepositoryStore::finalize_closure`
is the only supported external closure entry point.

After registering a bounded M002 implementation plan and re-checking current
CodeGG sibling interfaces, add support for Eggplan-native implementation /
closure projections and the documented subset of CodeGG's plans hierarchy.
Report lossy or unmapped fields.

### M003 — Ergonomics and performance

Roadmap-level after real repositories use M001/M002: shell completions, batch
queries, compact projections, and performance characterization.

## 5. Verification

CLI snapshots, JSON schema fixtures, invalid-state diagnostics, roundtrip
render/import where promised, stale registry detection, bounded output, and
cross-platform path/terminal behavior.

## 6. Completion

A human or agent can inspect and update planning/evidence state without direct
file surgery, while canonical authority remains explicit.
