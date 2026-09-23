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

Blocked until Foundation M003 and Evidence M001 C001 close. The CLI and its machine-readable JSON/projection contracts must not freeze the known self-staleness, loose-v1 parsing, or broad execution-evidence matching behavior.

After the corrective gate, commands should cover init, new, show, status, ready, graph, check, evidence,
and assess with JSON output and bounded diagnostics.

### M002 — Markdown import/render

Blocked on M001.

Support Eggplan-native implementation/closure projections and the documented
subset of CodeGG's plans hierarchy. Report lossy or unmapped fields.

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
