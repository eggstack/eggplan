# Projection/CLI M004 — Plan Lifecycle Transitions and Plan Structure Editing

Status: closed (see plans/closure/projection-cli/004-closed.md)

Repository baseline: 37ad290ca6b971a68d2871aedb39f4882f4eb2a8

Source roadmap:

- plans/subsystems/projection-cli-roadmap.md

Long-term requirements:

- plans/000-long-term-specification.md sections 14-15 and 19-20
- plans/000-long-term-specification.md lifecycle at :199-205
- plans/001-terminology-and-domain-model.md

Related ADRs: ADR-0002 and ADR-0005.

Predecessor plan/closure:

- plans/implementation/projection-cli/001-cli-control-surface-and-derived-registry.md
- plans/closure/projection-cli/001-closed.md
- plans/implementation/projection-cli/002-loss-aware-markdown-import-and-deterministic-render.md
- plans/closure/projection-cli/002-closed.md
- plans/implementation/projection-cli/003b-batch-compact-projections-and-shell-completions.md
- plans/closure/projection-cli/003b-closed.md

Primary class: capability

## 1. Objective

Close the gap between the normative plan lifecycle and the CLI's ability to
drive it, and close the gap between the Projection/CLI roadmap's completion
criterion and the CLI's ability to change a plan's structure.

Two capabilities are missing from the CLI, and both are visible to a user
who follows the specification:

- **Plan lifecycle.** `PlanStatus::Blocked` and `PlanStatus::Cancelled` are
  unreachable through the CLI. The domain model allows eight transitions; the
  CLI can perform one.
- **Plan structure.** There is no way to add or remove an item, edit
  dependencies, edit acceptance criteria, edit evidence requirements, or
  edit the objective. The only route to restructuring a plan is to hand-edit
  the canonical JSON in the state root.

This milestone was raised as **proposed**, not ready, because the two
capabilities above needed product decisions that the repository's planning
hierarchy places above this plan: the exact command surface, whether plan-structure
editing is a first-class command set or a single general-purpose editor, and whether
the `Blocked → Closed` transition the specification's lifecycle diagram implies is
in scope. Those decisions were taken and are recorded in §4 and in
`plans/adrs/ADR-0005-plan-lifecycle-and-structure-editing-through-the-cli.md`.
The plan now records the chosen surface so that implementation proceeds against
a concrete decision rather than an abstract one.

## 2. Finding

### E-PC-M004-01 — the CLI implements a two-state plan lifecycle

`plans/000-long-term-specification.md:199-205` states the normative lifecycle:

    The initial lifecycle is:

        Draft -> Active <-> Blocked -> Closed
                     \-> Cancelled

    Exact transitions may be refined by ADR, but terminal states MUST fail
    closed on ordinary mutation.

The domain implements it in `crates/eggplan-core/src/model.rs:396-409`:

    pub fn plan_transition_allowed(from: &PlanStatus, to: &PlanStatus) -> bool {
        use PlanStatus::*;
        matches!(
            (from, to),
            (Draft, Active)
                | (Draft, Cancelled)
                | (Active, Blocked)
                | (Active, Closed)
                | (Active, Cancelled)
                | (Blocked, Active)
                | (Blocked, Cancelled)
        )
    }

Eight legal transitions. The CLI reaches one of them.

| CLI path | Plan status effect | Location |
|---|---|---|
| `new --input PLAN.json` | requires `revision == 0` and `status == Draft`; the created plan is Draft | `crates/eggplan-cli/src/lib.rs:437` |
| `activate PLAN_ID` | writes `PlanStatus::Active` only | `crates/eggplan-cli/src/lib.rs:1312` |
| `item update PLAN_ID ITEM_ID` | mutates the item only; never touches `plan.status` | `crates/eggplan-cli/src/lib.rs:1582-1670` |
| `close PLAN_ID` | the guarded finalizer writes `Closed` | `crates/eggplan-cli/src/lib.rs`, closure path |
| `markdown import FILE` | creates one Draft plan | `crates/eggplan-cli/src/lib.rs:1192-1281` |

There is no command that writes `PlanStatus::Blocked` or
`PlanStatus::Cancelled`. The consequences:

- `activate` accepts a `Blocked` plan as a valid pre-state
  (`crates/eggplan-cli/src/lib.rs:1298-1311`), but nothing can produce one, so
  that branch of the guard is dead;
- `list --status blocked` and `list --status cancelled` are accepted filters
  (`crates/eggplan-cli/src/commands.rs:208`) that can never match a row;
- `check` can report a `blocked` check state
  (`docs/cli-reference.md:309`), but for a plan status that no CLI-created
  plan can hold;
- `docs/cli-reference.md:202` documents "`closed`, `cancelled` | terminal —
  no transitions", describing a transition the CLI cannot perform;
- the domain table has no `(Blocked, Closed)` row, so a blocked plan can only
  be closed by first re-activating it. The specification's diagram places
  `-> Closed` after the `Active <-> Blocked` pair, which reads as permitting
  `Blocked → Closed`. No ADR refines this; all four ADRs in `plans/adrs/`
  were checked and none addresses the lifecycle.

**Blast radius.** Capability only. Nothing is wrong with the domain model,
the store's transition enforcement, or the guarded finalizer. The CLI simply
does not expose the transitions the domain supports.

### E-PC-M004-02 — plan structure cannot be edited through the CLI

`plans/subsystems/projection-cli-roadmap.md:138-139` states the subsystem's
completion criterion:

    A human or agent can inspect and update planning/evidence state without
    direct file surgery, while canonical authority remains explicit.

The command inventory is `crates/eggplan-cli/src/commands.rs:210-396`. It
contains four mutators:

- `new` — creates a plan from strict Plan JSON
- `activate` — Draft → Active
- `item update` — one item's status, blocker, or next action
- `close` — the guarded finalizer

There is no command to:

- add or remove a plan item;
- edit an item's description, position, or parent;
- edit an item's dependencies;
- add, edit, or remove an acceptance criterion;
- add, edit, or remove an evidence requirement;
- edit the plan objective.

`new` refuses an existing Plan ID
(`crates/eggplan-repo/src/store.rs:926-928` returns `RepoError::AlreadyExists`),
so the only route to a restructured plan is to hand-edit
`.eggplan/plans/<id>/plan.json` — precisely the "direct file surgery" the
completion criterion rules out.

**Blast radius.** Capability only. The canonical store enforces revisions,
CAS, bounds, and transition legality on whatever bytes it is given, so a
hand-edited plan is still validated. The problem is that the supported path
for a routine operation — adding a task to a plan — is not the CLI.

### E-PC-M004-03 — the roadmap status line was stale

At the recorded baseline, `plans/subsystems/projection-cli-roadmap.md:3` read:

    Status: active / current; M003a and M003b planned and ready

while `:58-61` recorded M003, M003a, and M003b as closed and hosted-qualified.

**Resolution.** This finding required no implementation work and was closed
by the change that registered this plan. The roadmap's status line now reads:

    Status: active / current; M001-M003 (M003a and M003b) closed; M003b corrective
    lines C001-C003 open; M004 proposed

The roadmap's §6 completion criterion was amended in the same change to state
plainly that it is not yet met, with the reason and a pointer to M004. The
completion criterion itself is unchanged; only its honesty about the current
state changed.

## 3. Controlling semantics

From `plans/000-long-term-specification.md:199-205`: the lifecycle above is
normative, and terminal states fail closed on ordinary mutation.

From `plans/subsystems/projection-cli-roadmap.md:138-139`: a human or agent
updates planning and evidence state without direct file surgery, while
canonical authority remains explicit.

From `architecture/cli-control-surface.md`: the CLI is a thin adapter. It
holds no authority of its own; it drives the store's CAS and the core's
validation.

Required behavior after this milestone, subject to the decisions in §4:

- every plan lifecycle transition the specification declares is reachable
  through a CLI command, or the specification is amended by ADR to say which
  are not;
- a plan's items, dependencies, criteria, requirements, and objective can be
  changed through the CLI, with revision checks, CAS, bounds, and transition
  legality enforced by the existing store and core paths;
- no command edits canonical state without going through the store's
  compare-and-swap;
- the CLI remains thin: no domain semantics move into the CLI;
- the JSON envelope stays schema version 1, or a versioned change is made
  deliberately with a migration note.

## 4. Decisions — taken and recorded in ADR-0005

These decisions are not made here, because each changes the product surface
and belongs to the roadmap and the specification, not to an implementation
plan. They were taken, and `plans/adrs/ADR-0005-plan-lifecycle-and-structure-editing-through-the-cli.md`
records them with their drivers and their rejected alternatives. The summary
below binds this plan to that decision record.

**D1 — the command surface for lifecycle transitions. DECIDED: a `plan`
command family.** The CLI gains `eggplan plan set-status PLAN_ID --to STATUS
--expected-revision N`, alongside `eggplan plan edit PLAN_ID --input FILE
--expected-revision N` and `eggplan plan export PLAN_ID [--output FILE]`. The
core transition table decides legality, so no status rules live in the CLI.
`eggplan activate` is unchanged and remains the documented `Draft → Active`
shorthand. `--to closed` is refused with a message directing the caller to
`eggplan close`, because only the guarded finalizer may write `Closed`.

Option 3 (no command, amend the specification to expose only
`Draft → Active → Closed`) was considered explicitly and rejected; it would
leave two `list` filters permanently dead and narrow the specification to
accommodate a missing adapter.

**D2 — whether `Blocked → Closed` is legal. DECIDED: legal.** The
specification's diagram is authoritative, and no ADR had refined it. The
domain table gains the `(Blocked, Closed)` row so the two agree. `close` on a
`Blocked` plan stays legal, which is what the guarded finalizer already does
today because it never consults the transition table. The store's
guarded-closure requirement is unaffected: a compare-and-swap that would set
`Closed` is still refused.

**D3 — the shape of plan-structure editing. DECIDED: full-plan replacement
applied through the store's compare-and-swap.** `eggplan plan edit` takes a
full Plan JSON in the same strict form `new` accepts, parsed by
`eggplan_core::parse_plan`. The CLI requires the submitted plan's `id` to
equal the target plan's id, rejects a submitted `status` that differs from the
current plan's status with `invalid_transition`, sets the revision to
`--expected-revision + 1` itself, and applies the result through
`RepositoryStore::compare_and_swap`. `eggplan plan export` writes the plan's
canonical JSON out so the round trip never requires reading the store's own
files.

Option 2 (a set of fine-grained commands) was considered and rejected: it is a
much larger surface and needs its own partial-update semantics, which is new
domain behavior rather than reuse.

**D4 — whether plan-structure editing must reject structural changes to a
`Closed` plan. DECIDED: yes.** The store already refuses it with
`RepoError::ClosedPlanImmutable`. This plan states the rule rather than
inheriting it silently, and the CLI surfaces it as a typed failure.

**D5 — whether the `--expected-revision` requirement changes. DECIDED: it does
not.** Both mutating commands require `--expected-revision`. The whole point is
to avoid clobbering a concurrent edit.

**D6 — whether `markdown import` should remain Draft-only. DECIDED: yes.** The
current rule is that import creates one Draft intent and never manufactures
evidence, provider trust, subject authority, or closure
(`architecture/markdown-interchange.md`). `markdown import` must continue to
create Draft plans, and that must remain explicit so the import path cannot be
used to smuggle in a non-Draft plan — including now that a status command
exists.

**Amendment record.** This section was written as "Open decisions — why this
plan is proposed, not ready". The decisions were taken after the plan was
registered, and the section was replaced in place with the decided outcomes,
the ADR reference, and the rejected alternatives. No finding, scope
boundary, or acceptance criterion was weakened by the amendment; the plan's
§9 criteria are unchanged and are now decidable.

## 5. Scope

### In scope

- the CLI's `plan` command family — `plan set-status`, `plan edit`, and
  `plan export` (ADR-0005 D1 and D3);
- `eggplan-core`'s plan transition table, which gains the `(Blocked, Closed)`
  row the specification's diagram implies (ADR-0005 D2);
- the CLI's usage text, command metadata table, help output, and completion
  generation for any new command;
- `docs/cli-reference.md` and `docs/getting-started.md`;
- the Projection/CLI roadmap's status line and its completion criterion;
- the ADR that records the decisions;
- registry and roadmap lineage updates.

### Out of scope

- any change to `eggplan-core`'s domain model, transition table, validation,
  bounds, or graph rules, unless D2 requires a transition-table change that
  the ADR explicitly authorizes;
- any change to `eggplan-repo`'s CAS, locking, atomic write, closure
  finalizer, or evidence ledger;
- evidence acquisition, provider trust, or the provider SPI;
- the CodeGG bridge and the adapters;
- the deferred Interop/distribution subsystem;
- the user-documentation corrections and the completion-depth work owned by
  the M003b corrective lines;
- changing the CLI's JSON envelope schema version, unless a decision in §4
  requires it and a migration note accompanies it.

## 6. Required production changes

Ownership is `eggplan-cli`. The domain and the store already enforce
everything that must be enforced; the CLI's job is to expose them.

Required shape, subject to §4:

- a `plan set-status` lifecycle command whose legality is decided by the core
  transition table, plus the core transition table's missing `(Blocked,
  Closed)` row;
- a `plan edit` structure-editing command and a `plan export` canonical-JSON
  export, together closing the roadmap's completion criterion rather than
  amending it;
- every new mutation goes through the store's compare-and-swap with an
  explicit `--expected-revision`, and reuses the core's validation and
  transition checks;
- `plan edit` rejects a submitted `status` that differs from the current
  plan's status, so structure editing and lifecycle transitions stay separate
  surfaces and `plan edit` is not a back door to `Closed`;
- every new command is added to `crates/eggplan-cli/src/commands.rs`'s
  metadata table, so that help, validation, and all four completion shells
  change together — the rule already stated at
  `docs/cli-reference.md:141`;
- every new command's JSON envelope is schema version 1 and uses the existing
  envelope shape;
- new commands are read-then-write with the plan's current revision read
  under the store's lock, so no read-modify-write race is introduced.

Explicitly rejected alternatives:

- implementing plan-structure editing as direct file writes to the state root.
  It would bypass CAS, bounds, and transition enforcement, and would violate
  the completion criterion it is meant to satisfy;
- adding a CLI command that mutates canonical state without a revision check;
- relaxing the store's `ClosedPlanImmutable` rule to make structure editing
  easier;
- having `markdown import` create a non-Draft plan.

## 7. Required tests

At minimum:

- a CLI test that drives every legal plan lifecycle transition declared by
  the specification, asserting the transition succeeds and that an illegal
  transition is rejected with `invalid_transition`;
- a CLI test asserting that a plan's items, dependencies, criteria,
  requirements, and objective can each be changed through the CLI, and that
  the resulting plan passes `Plan::validate` and the graph's bounds and
  acyclicity checks;
- a CLI test asserting that a structure edit against a stale revision is
  rejected with `conflict`;
- a CLI test asserting that a structure edit against a `Closed` plan is
  rejected;
- a CLI test asserting that `markdown import` still creates a Draft plan;
- a CLI test asserting that `plan export` writes the plan's canonical JSON and
  that a `plan export` → edit → `plan edit` round trip preserves the plan's
  structure;
- a CLI test asserting the new commands appear in all four completion
  scripts and in the usage text;
- a core test pinning `(Blocked, Closed)` as a legal transition and the
  terminal states as fail-closed;
- the existing `eggplan-cli` suite, green and unchanged apart from the new
  regressions.

## 8. Verification

Record actual results for:

    cargo fmt --all -- --check
    cargo check --workspace --all-targets --locked
    cargo clippy --workspace --all-targets --locked -- -D warnings
    cargo test --workspace --locked
    cargo test -p eggplan-cli --locked
    cargo test -p eggplan-core --locked
    cargo test -p eggplan-repo --locked
    cargo +1.89.0 check --workspace --all-targets --locked
    cargo +1.89.0 test --workspace --locked
    bash scripts/check-projection-cli-boundary.sh
    bash scripts/check-core-boundary.sh
    git diff --check

The `cargo +1.89.0` rows are the MSRV gate and must be recorded truthfully,
including if the toolchain is unavailable in the implementing environment.
Hosted Linux/macOS/Windows and Rust 1.89 qualification must remain green.

## 9. Acceptance criteria

M004 closes when:

1. every plan lifecycle transition the specification declares is reachable
   through the CLI, or the specification has been amended by ADR to state
   which are not, and the amendment is recorded;
2. a plan's items, dependencies, criteria, requirements, and objective can be
   changed through the CLI, or the roadmap's completion criterion has been
   amended and the amendment is recorded;
3. every new mutation uses compare-and-swap with an explicit
   `--expected-revision`, and the store's and core's existing checks are the
   only enforcement path;
4. no command writes canonical state directly to the state root;
5. `markdown import` still creates one Draft plan and manufactures no
   evidence, provider trust, subject authority, or closure;
6. every new command appears in the command metadata table, the usage text,
   all four completion scripts, and `docs/cli-reference.md`;
7. the JSON envelope remains schema version 1, or a deliberate versioned
   change ships with a migration note;
8. the domain model, the store's CAS, locking, atomic write, and the closure
   finalizer are unchanged unless an ADR explicitly authorized a change;
9. native and MSRV qualification pass, with the named boundary guards green.

## 10. Stop conditions

Stop and report rather than improvise if:

- a decision in §4 turns out to be contradicted by the implementation, in
  which case the ADR is amended and the deviation recorded, rather than the
  implementation improvising a different surface;
- implementing the chosen surface would require changing the domain model,
  the transition table, or the store's enforcement beyond the single
  `(Blocked, Closed)` row that ADR-0005 authorizes;
- the JSON envelope would have to change version without a migration note;
- a new command would need to read and write canonical state without the
  store's lock and CAS;
- `markdown import` would have to create a non-Draft plan;
- required verification cannot be obtained honestly.

## 11. Closure evidence required

- implementation commit(s) and PR reference;
- the decisions taken for D1 through D6, with the ADR reference where one was
  required;
- the requirement-to-evidence matrix for every acceptance criterion in §9;
- the recorded CLI transcripts for each new command and each rejected
  transition;
- exact verification output, including the MSRV rows, distinguishing planned
  from actually run commands;
- hosted native/MSRV run and job identifiers where available;
- unresolved findings with severity, and roadmap disposition;
- registry updates.

Do not convert a planned command into a passing result because it appears in
§8. If a row was not run, record it as not run or unavailable.

## 12. Handoff notes

- This plan is **ready**. The §4 decisions were taken and are recorded in
  `plans/adrs/ADR-0005-plan-lifecycle-and-structure-editing-through-the-cli.md`;
  the plan's §4 was amended in place to bind to that decision record, with the
  amendment marked inline.
- Record truthfully that the domain, the store, and the CLI's existing
  commands are correct. The gap is that the CLI does not expose transitions
  the domain supports and cannot express structure changes at all.
- This is capability work, not a corrective. The M003b corrective lines and
  the M004 milestone are separate lines and neither gates the other.
- The roadmap status line at
  `plans/subsystems/projection-cli-roadmap.md:3` was corrected to match the
  milestone records in the change that registered this plan, and §6 was
  amended to state that the completion criterion is not yet met. Neither
  change depends on whether M004 proceeds.
