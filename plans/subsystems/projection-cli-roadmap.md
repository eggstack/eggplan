# Projection and CLI Roadmap

Status: active / current; M003a and M003b planned and ready

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

Status: closed.

Plan: plans/implementation/projection-cli/002-loss-aware-markdown-import-and-deterministic-render.md

M001 remains historically closed. Evidence M002 C002 has closed and the current
CodeGG planning/document shape was re-checked at
`5f4532659dbf0df2cd9f2b3bdb024217d2ea7868`.

M002 adds deterministic Eggplan-native Markdown render/import plus a strict,
loss-aware subset of CodeGG-style implementation-plan Markdown. Markdown
remains projection/import data only: source lifecycle is provenance and cannot
manufacture evidence, provider trust, SubjectRevision authority, or closure.
Implementation and local/hosted verification passed. See
plans/closure/projection-cli/002-closed.md.

### M003 — Ergonomics and performance

Status: planned / ready.

Real repository use exposed two separable work lines.

#### M003a — Repository inspection snapshot and performance qualification

Plan:
plans/implementation/projection-cli/003a-repository-inspection-snapshot-and-performance-qualification.md

Replace repeated per-Plan Git subject capture and repeated canonical-state reads
in repository-wide status/registry/check paths with one bounded ephemeral
repository inspection snapshot. Use cooperative read locking plus subject S1/S2
revalidation, preserve deep integrity checks, and qualify algorithmic complexity
with non-gating wall-time characterization.

#### M003b — Batch queries, compact projections, and shell completions

Plan:
plans/implementation/projection-cli/003b-batch-compact-projections-and-shell-completions.md

Add bounded multi-Plan queries, a new explicit compact projection,
provider-policy-aware read assessment, and deterministic Bash/Zsh/Fish/
PowerShell completion generation from one command metadata source. Existing
machine response shapes remain compatible.

M003b command metadata/compact DTO work may proceed in parallel with M003a.
Repository-wide batch execution must consume the M003a inspection snapshot
rather than restoring repeated per-Plan reads.

## 5. Verification

CLI snapshots, JSON schema fixtures, invalid-state diagnostics, roundtrip
render/import where promised, stale registry detection, bounded output, and
cross-platform path/terminal behavior.

## 6. Completion

A human or agent can inspect and update planning/evidence state without direct
file surgery, while canonical authority remains explicit.
