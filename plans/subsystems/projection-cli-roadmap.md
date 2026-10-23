# Projection and CLI Roadmap

Status: active / current; M001-M003 (M003a and M003b) closed; M003b corrective
lines C001-C003 open; M004 proposed

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

Status: closed (M003a and M003b both closed and hosted-qualified on run
`37368489637`).

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

### M003b corrective lines

Raised by the 2026-10-23 repository-wide audit against baseline
`37ad290ca6b971a68d2871aedb39f4882f4eb2a8`. All planned verification was run
and green at that baseline, and every finding is inference- and
reachability-established against the source rather than a test failure. The
M003 milestones themselves remain closed; these are corrective lines against
them. None of them gates M004 or another subsystem.

| Line | Class | Scope | Plan |
|---|---|---|---|
| C001 | non-blocking documentation correction | Correct four `docs/cli-reference.md` statements that the implementation contradicts, and document `init` and `help` | plans/implementation/projection-cli/003b-c001-cli-user-documentation-contract-corrections.md |
| C002 | correctness / completeness of a promised capability | One completion depth across all four generated shells, with per-shell depth assertions | plans/implementation/projection-cli/003b-c002-shell-completion-depth-parity.md |
| C003 | non-blocking maintenance | Remove the only production `unreachable!()` in the workspace and name the repeated Markdown bound | plans/implementation/projection-cli/003b-c003-markdown-interchange-panic-path-and-bound-hygiene.md |

### M004 — Plan lifecycle transitions and plan structure editing

Class: capability

Status: proposed.

Plan: plans/implementation/projection-cli/004-plan-lifecycle-and-structure-editing.md

The normative lifecycle at `plans/000-long-term-specification.md:199-205` is
`Draft -> Active <-> Blocked -> Closed \-> Cancelled`, and the domain
implements eight legal transitions at
`crates/eggplan-core/src/model.rs:391-403`. The CLI can perform one:
`PlanStatus::Blocked` and `PlanStatus::Cancelled` are unreachable through any
command. Separately, the CLI cannot change a plan's items, dependencies,
acceptance criteria, evidence requirements, or objective at all, so the
completion criterion at §6 below is not met.

M004 documents decisions D1-D6 and is **not dependency-ready** until they are
made. The decisions concern the command surface for lifecycle transitions,
whether `Blocked -> Closed` is legal, and the shape of plan-structure editing.
They are product decisions and belong to this roadmap and the specification,
not to an implementation plan.


## 5. Verification

CLI snapshots, JSON schema fixtures, invalid-state diagnostics, roundtrip
render/import where promised, stale registry detection, bounded output, and
cross-platform path/terminal behavior.

## 6. Completion

A human or agent can inspect and update planning/evidence state without direct
file surgery, while canonical authority remains explicit.

This criterion is **not yet met**. Inspection is met. Updating is met only
partially: item status, blocker, and next action can be changed through the
CLI, but plan lifecycle transitions other than `Draft -> Active -> Closed` and
all plan-structure changes cannot. The gap is recorded and scoped in the
proposed M004 milestone above; it is not closed by any corrective line, and
it must not be reported as satisfied until M004 closes or this criterion is
amended.
