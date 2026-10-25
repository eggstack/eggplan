# Projection and CLI Roadmap

Status: active / current; M001-M003 (M003a and M003b) closed; M003b corrective
lines C001-C003 closed; M004 closed

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

| Line | Class | Scope | Plan | Closure |
|---|---|---|---|---|
| C001 | non-blocking documentation correction | Correct four `docs/cli-reference.md` statements that the implementation contradicts, and document `init` and `help` | plans/implementation/projection-cli/003b-c001-cli-user-documentation-contract-corrections.md | closed — plans/closure/projection-cli/003b-c001-closed.md |
| C002 | correctness / completeness of a promised capability | One completion depth across all four generated shells, with per-shell depth assertions | plans/implementation/projection-cli/003b-c002-shell-completion-depth-parity.md | closed — plans/closure/projection-cli/003b-c002-closed.md |
| C003 | non-blocking maintenance | Remove the only production `unreachable!()` in the workspace and name the repeated Markdown bound | plans/implementation/projection-cli/003b-c003-markdown-interchange-panic-path-and-bound-hygiene.md | closed — plans/closure/projection-cli/003b-c003-closed.md |

C002 and C003 were implemented in parallel against disjoint files, and C001 was
implemented last so that its documentation pass ran against the final command
surface rather than an intermediate one. No corrective line serialized against
another, and none gated M004.

### M004 — Plan lifecycle transitions and plan structure editing

Class: capability

Status: closed.

Plan: plans/implementation/projection-cli/004-plan-lifecycle-and-structure-editing.md

Decision record: plans/adrs/ADR-0005-plan-lifecycle-and-structure-editing-through-the-cli.md

Closure: plans/closure/projection-cli/004-closed.md

The normative lifecycle at `plans/000-long-term-specification.md:199-205` is
`Draft -> Active <-> Blocked -> Closed \-> Cancelled`. Before M004 the CLI
could perform one transition, `Draft -> Active`; `PlanStatus::Blocked` and
`PlanStatus::Cancelled` were unreachable through any command, and the CLI
could not change a plan's items, dependencies, acceptance criteria, evidence
requirements, or objective at all.

The decisions that gated M004 were taken and are recorded in ADR-0005:

- **D1** — the CLI gains a `plan` command family: `plan set-status`, `plan
  edit`, and `plan export`. The core transition table decides legality, so the
  CLI holds no per-command pre-state rule. `eggplan activate` is unchanged.
- **D2** — the specification's lifecycle diagram is authoritative over the
  domain table, and `eggplan_core::plan_transition_allowed` gained the
  `(Blocked, Closed)` row. The two now agree.
- **D3** — structure editing is a full-plan replacement applied through the
  repository's compare-and-swap, with `plan export` producing the canonical
  Plan JSON that `plan edit` consumes.

D4, D5, and D6 were also decided: a `Closed` plan refuses structural edits,
both mutating commands require `--expected-revision`, and `markdown import`
remains Draft-only.

M004 was registered **proposed** because these decisions belong to this
roadmap and the specification, not to an implementation plan. They are now
made, so the plan is closed against the ADR above.


## 5. Verification

CLI snapshots, JSON schema fixtures, invalid-state diagnostics, roundtrip
render/import where promised, stale registry detection, bounded output, and
cross-platform path/terminal behavior.

## 6. Completion

A human or agent can inspect and update planning/evidence state without direct
file surgery, while canonical authority remains explicit.

This criterion is **met** as of M004. Inspection was met earlier. Updating
is now met in full: item status, blocker, and next action could always be
changed through the CLI, and M004 added the plan's lifecycle transitions and
its structure — objective, items, dependencies, acceptance criteria, and
evidence requirements — through `eggplan plan set-status` and
`eggplan plan edit`. Both write through the repository's compare-and-swap, so
canonical authority stays explicit and the plan's own validation still
applies; neither command creates evidence, provider trust, a subject revision,
or a closure record.

`eggplan plan edit` requires the caller to author a full Plan JSON, and the
round trip is `eggplan plan export`, edit the file, `eggplan plan edit
--input`. That ergonomic cost is recorded in ADR-0005 as the price of reusing
the whole validation path instead of inventing partial-update semantics.
