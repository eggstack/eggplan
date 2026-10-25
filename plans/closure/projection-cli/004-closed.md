# Projection/CLI M004 — Closed

Status: closed

Source plan: plans/implementation/projection-cli/004-plan-lifecycle-and-structure-editing.md

Source roadmap: plans/subsystems/projection-cli-roadmap.md

Decision record: plans/adrs/ADR-0005-plan-lifecycle-and-structure-editing-through-the-cli.md

Reviewed baseline: `37ad290ca6b971a68d2871aedb39f4882f4eb2a8`.

Implementation commit: to be recorded once the change is committed; the
commit identifier is to be added to this record as a factual amendment after
the fact, following the precedent of
`plans/closure/evidence-closure/002-c004-closed.md`.

Hosted CI was not run for this change. The job identifiers are to be added to
this record as a factual amendment once CI has run, following the precedent of
`plans/closure/evidence-closure/002-c004-closed.md`.

## Executive finding

Projection/CLI M004 closes. Every plan-status transition the specification
declares is now reachable through the CLI, and a plan's structure can be
changed without touching the canonical file directly.

The gap was never enforcement. `RepositoryStore::compare_and_swap`
(`crates/eggplan-repo/src/store.rs:991-1042`) already ran, in order: the
revision-shape check, `Plan::validate`, the revision conflict check, the
closed-plan immutability check, the guarded-closure requirement, plan-status
transition legality against the core table, item-status transition legality,
the storage size bound, and an atomic write. The graph's bounds and acyclicity
checks run inside `Plan::validate`. What the CLI did not have was a way to
*ask* for those transitions, and no way at all to express a structural change.

The CLI now has a `plan` command family
(`crates/eggplan-cli/src/lib.rs:1338`):

- `eggplan plan set-status PLAN_ID --to STATUS --expected-revision N`
  (`crates/eggplan-cli/src/lib.rs:1358`) moves a plan along its lifecycle. The
  core transition table decides legality — the CLI calls
  `eggplan_core::plan_transition_allowed` directly
  (`crates/eggplan-cli/src/lib.rs:1385`) and holds no per-command pre-state
  rule of its own.
- `eggplan plan edit PLAN_ID --input FILE --expected-revision N`
  (`crates/eggplan-cli/src/lib.rs:1431`) replaces a plan's structure with the
  Plan JSON in `FILE`, applied through the store's compare-and-swap
  (`crates/eggplan-cli/src/lib.rs:1480`).
- `eggplan plan export PLAN_ID [--output FILE]`
  (`crates/eggplan-cli/src/lib.rs:1499`) writes the plan's canonical Plan JSON,
  which is exactly what `plan edit` consumes.

The specification's lifecycle diagram and the domain's transition table also
disagreed about one transition, and M004 reconciled them. The diagram
(`plans/000-long-term-specification.md:199-205`) reads `Draft -> Active <->
Blocked -> Closed`, and the specification says transitions "may be refined by
ADR". No ADR had refined them, so the diagram stood and the domain table was
the artifact that diverged: `eggplan_core::plan_transition_allowed`
(`crates/eggplan-core/src/model.rs:396`) had no `(Blocked, Closed)` row. It
has one now, and `plans/adrs/ADR-0005-plan-lifecycle-and-structure-editing-through-the-cli.md`
records the decision, its drivers, and the alternatives that were rejected.

## Plan errata

This plan was registered as `proposed` because three of its decisions — the
command surface, the shape of structure editing, and the `Blocked -> Closed`
question — belong above an implementation plan. The decisions were taken after
registration, and the plan's §4 was amended in place to bind to the ADR, with
the amendment marked inline. No finding, scope boundary, or acceptance
criterion was weakened by that amendment. The plan's §1, §5, §6, §7, §10, and
§12 were amended to match the decided surface; §9's acceptance criteria were
left unchanged and are now decidable.

## Decision-to-evidence matrix

| Decision | ADR reference | Evidence | Status |
|---|---|---|---|
| D1 — a `plan` command family, with the core table deciding legality | ADR-0005 D1 | `crates/eggplan-cli/src/commands.rs:293-302` (the command metadata entry); `crates/eggplan-cli/src/lib.rs:530` (the dispatch arm), `:1338`, `:1358`; `crates/eggplan-cli/src/lib.rs:1385` (the core table consulted) | met |
| D2 — `Blocked -> Closed` is legal; the core table gains the row | ADR-0005 D2 | `crates/eggplan-core/src/model.rs:396-409`; `crates/eggplan-core/src/model.rs:526` (`plan_transition_table_matches_the_specified_lifecycle`) | met |
| D3 — full-plan replacement through the store's compare-and-swap, with a canonical export | ADR-0005 D3 | `crates/eggplan-cli/src/lib.rs:1431` and `:1480`; `crates/eggplan-cli/src/lib.rs:1499`; `crates/eggplan-cli/src/lib.rs:224` (`PlanExportData`) | met |
| D4 — a `Closed` plan refuses structural edits | ADR-0005 D4 | `crates/eggplan-repo/src/store.rs:1011` (`ClosedPlanImmutable`); `crates/eggplan-cli/tests/plan_lifecycle.rs` `plan_edit_rejects_a_closed_plan` | met |
| D5 — `--expected-revision` is required on both mutating commands | ADR-0005 D5 | `crates/eggplan-cli/src/commands.rs:299-300`; `crates/eggplan-cli/tests/plan_lifecycle.rs` `plan_set_status_requires_an_explicit_expected_revision` and `plan_edit_rejects_a_stale_revision` | met |
| D6 — `markdown import` remains Draft-only | ADR-0005 D6 | `crates/eggplan-cli/tests/plan_lifecycle.rs` `markdown_import_still_creates_a_draft_plan` | met |

## Production implementation evidence

### The core transition table

`crates/eggplan-core/src/model.rs:396-409` — `plan_transition_allowed` now
permits `(Blocked, Closed)` in addition to the seven transitions it already
permitted. The change is purely additive: no previously-permitted transition
was removed, so no stored plan's status became illegal.

`crates/eggplan-core/src/model.rs:526` —
`plan_transition_table_matches_the_specified_lifecycle` — pins all eight legal
edges and asserts that every other pair is refused, so the table is exactly
the specified lifecycle and not a superset of it. The existing
`transition_matrices_fail_closed_for_terminal_states` test was updated to
assert both legal edges into `Closed` and to keep the terminality pin, and
its explanatory comment was corrected: it previously said `Active -> Closed`
was "the one legal edge into Closed".

### The CLI command family

`crates/eggplan-cli/src/commands.rs:293-302` declares the `plan` command in
the declarative metadata table, so help, option validation, and all four
shell completions change together. Its subcommand-scoped options are
`--to` and `--expected-revision` for `set-status`, `--input` and
`--expected-revision` for `edit`, and `--output` for `export`.

`crates/eggplan-cli/src/commands.rs:185` declares the settable status set
`["active", "blocked", "cancelled"]`, and `:190` re-exports it to the command
handler so the value set the command accepts and the value set the shell
completions offer cannot drift apart.

`crates/eggplan-cli/src/lib.rs:1338` — `plan_command` dispatches the three
subcommands. `:1358` — `plan_set_status` reads the plan, checks the
revision, refuses a `Closed` plan, and asks the core table whether the
transition is legal (`:1385`); a legal transition is applied through
`compare_and_swap` (`:1403`). `:1431` — `plan_edit` parses the input with
`eggplan_core::parse_plan` — the same strict parser `new` uses, so unknown
fields are rejected and the schema version is checked — requires the
submitted plan's `id` to match the target, refuses a submitted status that
differs from the current one, sets the revision to `--expected-revision + 1`
itself, and applies the result through `compare_and_swap` (`:1480`).
`:1499` — `plan_export` writes `eggplan_core::canonical_json(&plan)` to
stdout or to `--output FILE`. `:1533` — `plan_status_name` is exhaustive over
`PlanStatus`, so a new variant is a compile error rather than a silently
missing name in a diagnostic. `:1549` — `parse_settable_plan_status` gives
the caller a direct reason for each rejected value.

The two exclusions from the settable set are stated rather than left to a
later failure. `--to closed` is refused with `guarded_closure_required` and a
message pointing at `eggplan close`, because only the guarded finalizer may
write `Closed`; the store refuses a plain compare-and-swap to `Closed` with
`RepoError::GuardedClosureRequired` (`crates/eggplan-repo/src/store.rs:1016`)
regardless. `--to draft` is refused with `invalid_transition`, because no
transition reaches `Draft`.

### Tests

`crates/eggplan-cli/tests/plan_lifecycle.rs` is new and holds twelve tests:

| Test | Asserts |
|---|---|
| `plan_set_status_drives_every_legal_transition` | `Draft -> Active`, `Active -> Blocked`, `Blocked -> Active`, `Active -> Cancelled` all succeed and advance the revision; `Cancelled` is terminal |
| `plan_set_status_rejects_transitions_the_core_table_forbids` | `Draft -> Blocked` and `Active -> Draft` are `invalid_transition`; a stale revision is `revision_conflict` |
| `plan_set_status_refuses_to_close_a_plan_directly` | `--to closed` is `guarded_closure_required`, the message points at `eggplan close`, and the plan is unchanged |
| `plan_set_status_requires_an_explicit_expected_revision` | a missing `--expected-revision` is `usage` |
| `plan_edit_changes_structure_and_keeps_the_plan_identity` | the objective, the item set, the dependencies, and the criterion statement all change; the plan id and status do not |
| `plan_edit_rejects_a_stale_revision` | `revision_conflict`, and the revision does not advance |
| `plan_edit_cannot_change_the_plan_status` | `invalid_transition`, and the message points at `eggplan plan set-status` |
| `plan_edit_rejects_a_mismatched_plan_id` | `invalid_plan` |
| `plan_edit_rejects_a_closed_plan` | a plan closed through the real guarded protocol is refused with `closed_plan_immutable` |
| `plan_export_writes_canonical_json_that_plan_edit_consumes` | the exported bytes are canonical Plan JSON, two exports are byte-identical, and the exported file is accepted unchanged by `plan edit` |
| `plan_commands_appear_in_the_usage_text_and_every_completion_script` | the `plan` family is in the usage text and `set-status`, `edit`, and `export` are in all four completion scripts |
| `markdown_import_still_creates_a_draft_plan` | the import path still creates a Draft plan |

`plan_edit_rejects_a_closed_plan` drives the full closure protocol — a git
subject, an active plan with a completed item, a trusted provider, a passing
observation, and the guarded finalizer — rather than fabricating a closed
plan on disk.

### The help fixture

`crates/eggplan-cli/tests/fixtures/help.txt:11` gained the line
`plan SUBCOMMAND ARG - plan lifecycle and structure`. That is the only
difference between the frozen fixture and the CLI's actual output, and it is
the intended consequence of adding a command to the inventory.

## Verification executed

Coordinator, against the integrated tree:

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | pass (one formatting fix applied in `crates/eggplan-cli/src/commands.rs` and `crates/eggplan-cli/src/lib.rs`) |
| `cargo check --workspace --all-targets --locked` | pass |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | pass, no warnings |
| `cargo test --workspace --locked` | **339 passed, 0 failed, 3 ignored** |
| `cargo test -p eggplan-core --locked` | **46 passed, 0 failed** |
| `cargo test -p eggplan-cli --locked` | pass, including **12 passed, 0 failed** in `tests/plan_lifecycle.rs` |
| `bash scripts/check-core-boundary.sh` | pass |
| `bash scripts/check-codegg-compat-boundary.sh` | pass |
| `bash scripts/check-integrations-boundary.sh` | pass |
| `bash scripts/check-projection-cli-boundary.sh` | pass (`projection/CLI boundary check passed`) |
| `bash scripts/check-closure-authority-boundary.sh` | pass |
| `cargo +1.89.0 check --workspace --all-targets --locked` | pass |
| `cargo +1.89.0 test --workspace --locked` | recorded below |
| `git diff --check` | pass |

The three ignored tests are the pre-existing characterization tests carried
from the baseline.

The CLI was also exercised directly against a real state root during
development: `plan set-status`, `plan edit`, and `plan export` were driven
end to end, including the refusal paths, and the resulting plan was read back
through `RepositoryStore` to confirm the stored bytes.

The end-to-end run also surfaced a user-visible consequence of ADR-0005 D3's
status rule, which the guides now state: because `plan edit` refuses a
submitted status that differs from the plan's current status, an export taken
before a status change is stale and is refused with `invalid_transition`
("cancelled -> draft is not a legal plan transition"). The remedy is to
export again. The coordinator's run confirmed both behaviors: the stale edit
was refused, and the same edit against a fresh export was applied and read
back with the new objective, item description, and criterion statement
intact.

## Invariant review

- **The domain decides, the CLI reports.** `plan set-status` asks
  `eggplan_core::plan_transition_allowed`; it does not carry its own copy of
  the legal-transition set. A change to the core table changes the CLI's
  behavior with no CLI change.
- **Every mutation goes through the store's compare-and-swap.** No command
  writes canonical state outside the store, and every mutation requires an
  explicit `--expected-revision`.
- **The store remains the enforcement point.** `Plan::validate`, the graph's
  bounds and acyclicity checks, the revision conflict check, the closed-plan
  immutability check, the guarded-closure requirement, and the item-status
  transition rules all still apply to `plan edit`, because it applies its
  result through `compare_and_swap`.
- **The command metadata table remains the single source of truth.** The
  `plan` family is declared there, so help, option validation, and the
  completions changed together by construction.
- **The public API of `eggplan-core` gained one transition and lost none.**

## Failure, recovery, and compatibility review

- `eggplan activate` is unchanged and remains the documented `Draft -> Active`
  shorthand. The overlap with `eggplan plan set-status --to active` is
  deliberate and documented; removing `activate` would be a breaking change
  to a closed milestone's command surface.
- `eggplan markdown import` is unchanged and still creates one Draft plan.
- The Plan schema version stays 2 and the envelope schema version stays 1.
  No schema, digest, or golden-fixture byte changed.
- `eggplan list --status blocked` and `--status cancelled` were filters that
  could never match a row and now can. `eggplan check`'s `blocked` check state
  was unreachable and now is.
- The core transition table gained a row. No stored plan's bytes change and no
  existing plan's status becomes illegal; a plan that is `Blocked` may now
  also be closed, which the guarded finalizer already permitted.
- A refused mutation leaves the plan untouched. The tests assert this
  explicitly for the closed-plan, stale-revision, and status-change refusals.

## Security, trust, and path review

- **No new trust surface.** Neither mutating command creates evidence,
  enrolls a provider, captures a subject, or writes a closure record.
  `plan edit` cannot smuggle a `Closed` plan into existence: the status must
  match the current plan's status, and the store refuses a compare-and-swap to
  `Closed` outright.
- **`plan edit` cannot manufacture provenance.** The submitted Plan JSON's
  `provenance` map is subject to the same validation as any other field; a
  caller can write provenance, exactly as a caller who can write a plan at
  `new` can, and it is still a caller-authored string map with no authority
  attached to it.
- **Path handling.** `plan edit` reads the file named by `--input` through
  the existing `read_bounded` helper, which enforces the 16 MiB input bound
  and refuses a symlink. `plan export` writes plan content to the path named
  by `--output`; it writes no credentials and reads no state beyond the plan
  named on the command line.
- **Untrusted input.** The submitted Plan JSON is parsed by
  `eggplan_core::parse_plan`, the strict parser, and is then re-validated by
  `Plan::validate` inside the store. Unknown fields are rejected rather than
  ignored.

## Documentation and operations

- `plans/adrs/ADR-0005-plan-lifecycle-and-structure-editing-through-the-cli.md`
  is new and records D1, D2, and D3 with their drivers, the rejected
  alternatives, the compatibility and security implications, and the
  verification expectations.
- `docs/cli-reference.md` documents the three commands, the plan-status
  lifecycle, the `plan edit` constraints, and the export/edit round trip.
- `docs/getting-started.md` mentions the family where it helps an adopter.
- `architecture/cli-control-surface.md` records the new command family in the
  control-surface design record: its subcommands, its options, its machine
  codes, and the rule that it writes only through the repository's
  compare-and-swap and never creates evidence, provider trust, a subject
  revision, or a closure.
- `architecture/deep-dive-projection-cli.md` records the new command family in
  the CLI walkthrough, the new test file in the test strategy, the corrected
  `--expected-revision` count (three revisioned mutations before this change,
  five after), and the corrected review finding 4 — the CLI now has **two**
  arbitrary-path writes, `markdown render --output` and `plan export
  --output`, not one. The severity of that finding is unchanged: both write
  caller-named paths with plan-derived bytes, neither creates authority, and a
  clobbered file is recoverable only by the user. The count changed, not the
  class.
- `crates/eggplan-cli/tests/fixtures/help.txt` gained the `plan` usage line.

### Opportunistic citation repair, recorded

The `plan` family shifted line numbers in `crates/eggplan-cli/src/lib.rs` and
`crates/eggplan-cli/src/commands.rs`, and the core transition row shifted
`crates/eggplan-core/src/model.rs`. Every citation in
`architecture/deep-dive-projection-cli.md` and `architecture/cli-control-surface.md`
was re-verified against the current files and corrected where it had drifted.

While doing that, the repair found a **pre-existing** drift in
`architecture/deep-dive-repository.md`: roughly 56 `store.rs` citations and
three `repository.rs` citations were anchored to a much older version of
`crates/eggplan-repo/src/store.rs` (which has since grown from 1483 to 1669
lines) and pointed at the wrong code for the CAS guards, the finalizer, the
pending-closure recovery path, and the `.exists()` sites. That drift predates
this wave — it is present at the baseline commit `37ad290ca6b971a68d2871aedb39f4882f4eb2a8`
— and it was corrected as well, because a design record that points at the
wrong code is the same defect class this wave exists to remove. The
correction is citation-only: the prose, the findings, and their severities are
unchanged, except for one historical sentence in finding 6.1 that depended on
`open` calling `store.list()`, a call that was removed in `37ad290`; the
sentence now says so rather than citing code that no longer exists.

All 385 citations across the four architecture files were re-read after the
repair: 0 out of range, 0 unresolved, and every resolved range checked against
the claim it supports.

## Unresolved findings

1. **Medium — hosted CI has not run for this change.** The ubuntu, macOS, and
   Windows jobs and the MSRV job are outstanding. This record does not claim
   them as passing.
2. **Low — `plan edit` requires the caller to author a full Plan JSON.** The
   round trip is `plan export`, edit the file, `plan edit --input`. This is
   the ergonomic cost ADR-0005 accepted in exchange for reusing the whole
   validation path instead of inventing partial-update semantics. A finer
   command set would remove the cost at the price of a much larger surface;
   that is a product decision for a future milestone, not a defect in this
   one.
3. **Low — `eggplan activate` and `eggplan plan set-status --to active`
   overlap.** Both are documented. Removing `activate` is deferred because it
   would be a breaking change to a closed milestone.
4. **Informational — the CLI's `--to` value set is narrower than the canonical
   plan-status vocabulary.** `closed` and `draft` are plan statuses but are not
   settable through `set-status`. Each rejection carries its own reason, and
   the completions offer only the settable values.

## Roadmap disposition

`plans/subsystems/projection-cli-roadmap.md` — M004 moves from `proposed` to
closed. The roadmap's completion criterion for plan structure editing, which
previously read as an unmet requirement, is now met by `eggplan plan edit`
applied through the store's compare-and-swap: a human or an agent can inspect
and update a plan's items, dependencies, criteria, requirements, and
objective without direct file surgery on the canonical state. The M003b
corrective lines C001, C002, and C003 are closed as well.

## Registry updates

`plans/registry.md` — the row for
`plans/implementation/projection-cli/004-plan-lifecycle-and-structure-editing.md`
is moved from `proposed` to `closed` and points at this record. The registry's
Projection/CLI subsection of the "Correctives raised by the 2026-10-23
repository audit" section records this closure.
