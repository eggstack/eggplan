# Projection/CLI M004 — Plan Lifecycle Transitions and Plan Structure Editing

Status: proposed

Repository baseline: 37ad290ca6b971a68d2871aedb39f4882f4eb2a8

Source roadmap:

- plans/subsystems/projection-cli-roadmap.md

Long-term requirements:

- plans/000-long-term-specification.md sections 14-15 and 19-20
- plans/000-long-term-specification.md lifecycle at :199-205
- plans/001-terminology-and-domain-model.md

Related ADRs: ADR-0002.

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

This milestone is **proposed**, not ready. The two capabilities above need
product decisions that the repository's planning hierarchy places above this
plan: the exact command surface, whether plan-structure editing is a
first-class command set or a single general-purpose editor, and whether the
`Blocked → Closed` transition the specification's lifecycle diagram implies is
in scope. Those decisions are recorded as open in §4. This plan documents the
gap, the constraints, and the shape of a solution so that the decisions can
be made against a concrete analysis rather than in the abstract.

## 2. Finding

### E-PC-M004-01 — the CLI implements a two-state plan lifecycle

`plans/000-long-term-specification.md:199-205` states the normative lifecycle:

    The initial lifecycle is:

        Draft -> Active <-> Blocked -> Closed
                     \-> Cancelled

    Exact transitions may be refined by ADR, but terminal states MUST fail
    closed on ordinary mutation.

The domain implements it in `crates/eggplan-core/src/model.rs:391-403`:

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
| `new --input PLAN.json` | requires `revision == 0` and `status == Draft`; the created plan is Draft | `crates/eggplan-cli/src/lib.rs:423-430` |
| `activate PLAN_ID` | writes `PlanStatus::Active` only | `crates/eggplan-cli/src/lib.rs:1274` |
| `item update PLAN_ID ITEM_ID` | mutates the item only; never touches `plan.status` | `crates/eggplan-cli/src/lib.rs:1291-1379` |
| `close PLAN_ID` | the guarded finalizer writes `Closed` | `crates/eggplan-cli/src/lib.rs`, closure path |
| `markdown import FILE` | creates one Draft plan | `crates/eggplan-cli/src/lib.rs:1156-1163` |

There is no command that writes `PlanStatus::Blocked` or
`PlanStatus::Cancelled`. The consequences:

- `activate` accepts a `Blocked` plan as a valid pre-state
  (`crates/eggplan-cli/src/lib.rs:1262`), but nothing can produce one, so
  that branch of the guard is dead;
- `list --status blocked` and `list --status cancelled` are accepted filters
  (`crates/eggplan-cli/src/commands.rs:185`) that can never match a row;
- `check` can report a `blocked` check state
  (`docs/cli-reference.md:205`), but for a plan status that no CLI-created
  plan can hold;
- `docs/cli-reference.md:152-153` documents "`closed` and `cancelled` are
  terminal", describing a transition the CLI cannot perform;
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

The command inventory is `crates/eggplan-cli/src/commands.rs:187-360`. It
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

## 4. Open decisions — why this plan is proposed, not ready

These decisions are not made here, because each changes the product surface
and belongs to the roadmap and the specification, not to an implementation
plan.

**D1 — the command surface for lifecycle transitions.** Options:

1. extend `activate` into a general `plan status` command that takes the
   target status and lets the core's transition table decide legality;
2. add explicit `plan block` and `plan cancel` commands;
3. add no command and amend the specification to state that Eggplan's CLI
   exposes only `Draft → Active → Closed`.

Option 3 is a real option and must be decided explicitly, not by omission.
It would require an ADR and an amendment to the roadmap's completion
criterion.

**D2 — whether `Blocked → Closed` is legal.** The domain table does not
permit it; the specification's diagram reads as if it does. The two must be
reconciled by ADR or by amending the diagram.

**D3 — the shape of plan-structure editing.** Options:

1. a general `plan edit` command that takes a full replacement Plan JSON and
   applies it through CAS, mirroring `new`'s strict-JSON input. This is the
   smallest surface and reuses the existing validation path completely;
2. a set of fine-grained commands (`item add`, `item remove`, `item
   dependencies`, `criterion add`, …), which is a much larger surface and a
   much larger review;
3. no CLI support, with the roadmap's completion criterion amended.

Option 1 deserves serious consideration: it gives the CLI the capability the
completion criterion asks for with one new command and no new domain
semantics, because every structural rule already lives in `Plan::validate`
and the graph's bounds and acyclicity checks.

**D4 — whether plan-structure editing must reject structural changes to a
`Closed` plan.** The store already rejects ordinary CAS against a `Closed`
plan (`crates/eggplan-repo/src/store.rs`, `RepoError::ClosedPlanImmutable`),
so this likely needs no new rule, but the plan must state it rather than
inherit it silently.

**D5 — whether the `--expected-revision` requirement changes.** The existing
mutations require it. Structure editing must require it too, because the
whole point is to avoid clobbering concurrent edits. This is not really in
doubt, but it is recorded because it is a user-visible consequence.

**D6 — whether `markdown import` should remain Draft-only.** The current
rule is that import creates one Draft intent and never manufactures evidence,
provider trust, subject authority, or closure
(`architecture/markdown-interchange.md`). If M004 adds a plan-status command,
`markdown import` must continue to create Draft plans, and that must remain
explicit so the import path cannot be used to smuggle in a non-Draft plan.

## 5. Scope

### In scope

- the CLI's plan-lifecycle command surface, subject to D1 and D2;
- the CLI's plan-structure editing surface, subject to D3 and D4;
- the CLI's usage text, command metadata table, help output, and completion
  generation for any new command;
- `docs/cli-reference.md` and `docs/getting-started.md`;
- the Projection/CLI roadmap's status line and its completion criterion;
- an ADR, if D1 or D2 requires refining the specification's lifecycle;
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

- a lifecycle command, or an explicit amendment to the specification;
- a structure-editing command, or an explicit amendment to the roadmap's
  completion criterion;
- every new mutation goes through the store's compare-and-swap with an
  explicit `--expected-revision`, and reuses the core's validation and
  transition checks;
- every new command is added to `crates/eggplan-cli/src/commands.rs`'s
  metadata table, so that help, validation, and all four completion shells
  change together — the rule already stated at
  `docs/cli-reference.md:122`;
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
- a CLI test asserting the new commands appear in all four completion
  scripts and in the usage text;
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

- the decisions in §4 have not been made. This plan must not be handed to an
  implementation agent while D1, D2, and D3 are open;
- implementing the chosen surface would require changing the domain model,
  the transition table, or the store's enforcement, without an ADR that
  authorizes it;
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

- This plan is **proposed**. It is not dependency-ready and must not be
  handed to an implementation agent until the §4 decisions are made.
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
