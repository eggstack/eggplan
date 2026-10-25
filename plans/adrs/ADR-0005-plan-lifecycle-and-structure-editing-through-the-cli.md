# ADR-0005: Plan Lifecycle and Structure Editing Through the CLI

Status: accepted

Date: 2026-10-25

## Context

`plans/000-long-term-specification.md:199-205` states the normative plan
lifecycle:

    Draft -> Active <-> Blocked -> Closed
                     \-> Cancelled

    Exact transitions may be refined by ADR, but terminal states MUST fail
    closed on ordinary mutation.

Two gaps exist between that normative statement and what the workspace
implements.

**The domain and the specification disagree about one transition.**
`eggplan_core::plan_transition_allowed`
(`crates/eggplan-core/src/model.rs:396-409`) permits seven transitions and has
no `(Blocked, Closed)` row. The specification's diagram reads as permitting
it: the `-> Closed` edge follows the `Active <-> Blocked` pair. No ADR has
refined the lifecycle, so the domain currently diverges from the specification
without the refinement the specification itself names as the mechanism.

**The CLI exposes almost none of the lifecycle, and none of the structure.**
`eggplan activate` (`crates/eggplan-cli/src/lib.rs:1283`) performs only
`Draft -> Active`. `Blocked` and `Cancelled` are unreachable from any command,
so `list --status blocked` and `list --status cancelled` are filters that can
never match a row, and `check`'s `blocked` check state describes a plan status
no CLI-created plan can hold. Separately, there is no command to add or remove
an item, edit dependencies, edit an acceptance criterion, edit an evidence
requirement, or edit the objective. `new` refuses an existing Plan ID
(`crates/eggplan-repo/src/store.rs:934`), so the only supported route to a
restructured plan is hand-editing the canonical file inside the state root.

The enforcement machinery is not the problem. `RepositoryStore::compare_and_swap`
(`crates/eggplan-repo/src/store.rs:991-1042`) already performs, in order: the
revision-shape check, `Plan::validate`, the revision conflict check, the
closed-plan immutability check, the guarded-closure requirement, plan-status
transition legality against the core table, item-status transition legality,
the storage size bound, and an atomic write. The graph's bounds and acyclicity
checks run inside `Plan::validate`. The gap is exposure, not enforcement.

## Decision drivers

- Expose the domain's own rules instead of re-implementing them in the CLI.
- Keep the CLI a thin adapter; move no domain semantics into it.
- Keep every mutation behind the store's compare-and-swap with an explicit
  revision check.
- Prefer the smallest reviewable surface; introduce no patch or merge
  semantics.
- Keep the specification, the domain model, and the CLI consistent with each
  other.
- Never manufacture evidence, provider trust, subject authority, or closure
  from a structural edit.

## Considered options

### Lifecycle command surface

#### A. A `plan` command family — Selected

One command drives every transition, and the core transition table decides
legality. The CLI holds no per-command pre-state rules, so the legal-transition
logic stays in one place.

#### B. Flat `block` and `cancel` commands — Rejected

This matches the existing flat vocabulary of `activate` and `close`, but each
command would carry its own pre-state rule, so the legal-transition logic
would be spread across the CLI and would duplicate the core table it is
supposed to defer to.

#### C. No lifecycle command; narrow the specification — Rejected

This is a real option and must be decided explicitly rather than by omission.
It would leave `Blocked` and `Cancelled` permanently unreachable, keep two
`list` filters dead, and require amending the Projection/CLI roadmap's
completion criterion. It would also be a narrowing of the specification driven
by a missing adapter rather than by a product need.

### Whether `Blocked -> Closed` is legal

#### A. The specification's diagram is authoritative; the domain table gains the row — Selected

The specification says transitions "may be refined by ADR". No ADR refined
them, so the diagram stands as written and the domain table is the artifact
that diverges. `close` on a `Blocked` plan stays legal, which is what the
guarded finalizer already does today because it never consults the transition
table.

#### B. The domain table is authoritative; amend the diagram and guard `close` — Rejected

This keeps the core untouched, but it narrows the specification, and it
requires adding a guard so that `close` refuses a plan that is not `Active`.
That changes the behavior of an already-shipped command and narrows the
normative lifecycle to accommodate a missing row in an implementation table.

#### C. Leave both as they are — Rejected

M004 would close with the specification and the domain still contradicting
each other, and the contradiction would be invisible to every user because
`Blocked` is unreachable.

### Shape of structure editing

#### A. Full-plan replacement applied through the store's compare-and-swap — Selected

Every structural rule already lives in `Plan::validate` and the graph checks.
This reuses the entire validation path, adds one command, and introduces no
new domain semantics.

#### B. Fine-grained commands (`item add`, `item remove`, `item dependencies`, `criterion add`, and so on) — Rejected

More ergonomic, but a much larger command surface, a much larger review, and
each command would need its own partial-update semantics, which is new domain
behavior rather than reuse.

#### C. No CLI support; amend the roadmap's completion criterion — Rejected

It leaves routine operations — adding a task to a plan — outside the CLI,
which is exactly what the roadmap's completion criterion rules out.

## Decision

### D1 — the lifecycle command surface

The CLI gains a `plan` command family, added to the declarative command table
in `crates/eggplan-cli/src/commands.rs` so that help, validation, and all four
completion shells change together:

- `eggplan plan set-status PLAN_ID --to STATUS --expected-revision N`
- `eggplan plan edit PLAN_ID --input FILE --expected-revision N`
- `eggplan plan export PLAN_ID [--output FILE]`

`eggplan activate` is unchanged and remains the documented `Draft -> Active`
shorthand. Two commands that overlap is accepted deliberately: removing
`activate` would be a breaking change to a closed milestone's command surface,
and the overlap is documented rather than hidden.

`--to` accepts the statuses `plan set-status` can actually apply: `active`,
`blocked`, and `cancelled`. The two exclusions are stated to the caller
directly rather than left to a later failure: `Closed` is written only by the
guarded finalizer, so `--to closed` is rejected with `guarded_closure_required`
and a message pointing at `eggplan close`; and no transition reaches `Draft`,
so `--to draft` is rejected with `invalid_transition`. The store refuses a
plain compare-and-swap to `Closed` with `RepoError::GuardedClosureRequired`
(`crates/eggplan-repo/src/store.rs:1016`) regardless. The command's value set
is the same set the shell completions offer, so completion never suggests a
value that cannot be applied.

### D2 — `Blocked -> Closed` is legal

`eggplan_core::plan_transition_allowed` gains the `(Blocked, Closed)` row, so
the domain table matches the specification's diagram. `close` on a `Blocked`
plan remains legal.

The store's guarded-closure requirement is unaffected: a compare-and-swap that
would set `Closed` is still refused, so `Closed` remains reachable only
through `eggplan close`.

### D3 — shape of structure editing

`eggplan plan edit` takes a full Plan JSON in the same strict form `new`
accepts, parsed by `eggplan_core::parse_plan`. The CLI:

- requires the submitted plan's `id` to equal the target plan's id;
- rejects a submitted `status` that differs from the current plan's status,
  with `invalid_transition`, so structure editing and lifecycle transitions
  stay separate surfaces and `plan edit` is not a back door to `Closed`;
- sets the submitted plan's revision to `--expected-revision + 1` itself, so
  the revision the caller states is the revision the store checks;
- applies the result through `RepositoryStore::compare_and_swap`, so
  `Plan::validate`, the graph bounds and acyclicity checks, the revision
  conflict check, closed-plan immutability, the guarded-closure requirement,
  and item-status transition legality all still apply, and no command writes
  canonical state outside the store.

`eggplan plan export` writes the plan's canonical JSON
(`eggplan_core::canonical_json`) to stdout, or to `--output FILE`. It exists so
that the edit round trip — export, edit, submit — never requires reading the
store's own files.

### D4 — structural edits against a `Closed` plan are refused

`plan edit` against a `Closed` plan is refused. The store already refuses it
with `RepoError::ClosedPlanImmutable`; this ADR states the rule so it is not
inherited silently.

### D5 — `--expected-revision` is required

Both mutating commands require `--expected-revision`. The point of structure
editing is to avoid clobbering a concurrent edit.

### D6 — `markdown import` remains Draft-only

Unchanged. `markdown import` continues to create exactly one Draft plan and
manufactures no evidence, provider trust, subject authority, or closure. This
is stated explicitly so that the new status command cannot be used to smuggle
a non-Draft plan into existence through the import path.

## Consequences

Positive: every plan-status transition the specification declares is reachable
through the CLI; the dead `list` filters and the unreachable `blocked` check
state become meaningful; a plan's structure can be changed without touching
the canonical file directly; the CLI holds no lifecycle rules of its own.

Negative: `plan edit` requires the caller to author a full Plan JSON. The
round trip is `plan export`, edit the file, `plan edit --input`. That is a
real ergonomic cost, accepted in exchange for reusing the whole validation path
instead of inventing partial-update semantics.

Negative: `eggplan activate` and `eggplan plan set-status --to active` overlap.
Both are documented.

Negative: `plan_transition_allowed` is a public core API and its table changed.
The change is additive: no previously-allowed transition was removed.

## Compatibility and migration

- The Plan schema version stays 2 and the envelope schema version stays 1.
  No schema, digest, or golden-fixture byte changes.
- `eggplan activate` is unchanged.
- The new commands are purely additive to the command inventory.
- The core transition table gains one row. No stored plan's bytes change, and
  no existing plan's status becomes illegal.
- `list --status blocked` and `list --status cancelled` become reachable
  filters, and `check`'s `blocked` check state becomes reachable.

## Security and reliability implications

- No new trust surface. Neither command creates evidence, enrolls a provider,
  captures a subject, or writes a closure record.
- Every write goes through the store's compare-and-swap under its lock, with
  the revision conflict check and the atomic write it already performs.
- `plan export` writes plan content to a path the caller names. It writes no
  credentials and reads no state beyond the plan named on the command line.
- `plan edit` cannot smuggle a `Closed` plan into existence: the status must
  match the current plan's status, and the store refuses a compare-and-swap to
  `Closed` outright.

## Verification expectations

- A CLI test drives every legal plan-status transition and asserts that an
  illegal one is refused with `invalid_transition`.
- A CLI test asserts that items, dependencies, criteria, requirements, and the
  objective can each be changed through `plan edit`, and that the result
  satisfies `Plan::validate` and the graph's bounds and acyclicity checks.
- A CLI test asserts that `plan edit` against a stale revision is refused with
  `conflict`.
- A CLI test asserts that `plan edit` against a `Closed` plan is refused.
- A CLI test asserts that `plan edit` cannot change a plan's status.
- A CLI test asserts that `markdown import` still creates a Draft plan.
- A CLI test asserts that the new commands appear in the command metadata table,
  the usage text, and all four completion scripts.
- The core transition table gains a test that pins `(Blocked, Closed)` as legal
  and the terminal states as fail-closed.

## Supersession

None. This ADR refines the lifecycle that
`plans/000-long-term-specification.md:199-205` declares, using the refinement
mechanism that section names. It does not supersede ADR-0001 (planning is an
evidence mechanism, not an executor), ADR-0002 (repository-first versioned
canonical state), ADR-0003 (immutable revision-scoped evidence), or ADR-0004
(CodeGG extraction without WorkOrder conflation).
