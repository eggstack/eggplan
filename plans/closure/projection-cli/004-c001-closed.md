# Projection/CLI M004 C001 — Closed

Status: closed

Source plan: plans/implementation/projection-cli/004-c001-cli-help-subcommand-fidelity.md

Source roadmap: plans/subsystems/projection-cli-roadmap.md

Reviewed baseline: `ee54a0b8e2cb0d843565c54930ad2c21111320db`.

Implementation commit: recorded as a factual amendment after the fact, following
the precedent of `plans/closure/evidence-closure/002-c004-closed.md`.

Hosted CI was not run for this change. The job identifiers are to be added to
this record as a factual amendment once CI has run, following the precedent of
`plans/closure/evidence-closure/002-c004-closed.md`.

## Executive finding

Projection/CLI M004 C001 closes. The CLI's help and the CLI's own validation
now say the same thing.

The defect was confirmed by direct reproduction before any change was made.
`command_help` rendered only `spec.options` and `spec.flags` and never
`spec.sub_options`, so a subcommand's help page listed its *parent's*
options; and `help_result` resolved `args.positional.first()` as the command,
which in the `CMD SUBCOMMAND --help` form is the *subcommand*, not the
command. The two together produced two distinct manifestations:

```text
$ eggplan help markdown render
eggplan markdown - Markdown interchange <SUBCOMMAND> <ARG>
  subcommands: render, inspect, import
  --state-root <PATH>  repository state root (default .eggplan)

$ eggplan markdown render --help
eggplan [--state-root PATH] [--json] COMMAND
init - create a state root
...                                   # the whole top-level usage

$ eggplan markdown inspect x.md --output out.md --state-root .eggplan --json
option --output is not valid for this command; accepted: --state-root, --format
```

The first page omits `--output` — an option the command accepts. The second
prints the entire top-level usage instead of the requested page. The third
shows the consequence: a user reading `help markdown render` would conclude
`--output` does not exist, and the CLI would then reject it with a message
naming a *different* set of options than the help page listed. Help and
validation disagreed, and the CLI had no single owner of the truth.

The fix gives the CLI one owner: the help page for a command and subcommand is
rendered from the same option list the validator checks, so the two cannot
disagree.

## Requirement-to-evidence matrix

| Plan requirement (§9) | Evidence | Status |
|---|---|---|
| 1 — `eggplan help markdown render` lists `--output`; `eggplan help markdown inspect` does not | see "Before and after" below: `render` lists `--output <FILE>`, `inspect` lists `--format <VALUE>` and no `--output` | met |
| 2 — `eggplan help plan set-status` lists `--to` and `--expected-revision` | see "Before and after" below | met |
| 3 — `eggplan markdown render --help` prints the `markdown render` help, not the top-level usage | see "Before and after" below | met |
| 4 — `eggplan help markdown render` and `eggplan markdown render --help` produce identical text | `diff` of the two outputs is empty; asserted by the test "both_help_forms_render_the_same_page" | met |
| 5 — `eggplan help markdown nonsense` is a `usage` error naming the accepted subcommands | `help: usage: unknown subcommand "nonsense"; expected one of render, inspect, import`, exit 2 | met |
| 6 — the options listed by any command's help are exactly the options the validator accepts for that command and subcommand | the sweep test "every_help_page_lists_exactly_the_options_the_validator_accepts" over all 36 (command, subcommand) pairs; see "Agreement test" below | met |
| 7 — the command inventory, the usage text, the JSON envelope, and every machine code are unchanged | see "Nothing else moved" below | met |
| 8 — `crates/eggplan-cli/tests/fixtures/help.txt` is unchanged, because the top-level usage is unchanged | sha256 `b4348d56f800c129e67fc0e44b94fd014d33ba861c0aea50e7b97e53db1cd751` on disk and at `HEAD`; the top-level usage is byte-identical to it | met |

## Before and after

| Request | Before | After |
|---|---|---|
| `help markdown render` | `eggplan markdown - Markdown interchange <SUBCOMMAND> <ARG>` + `subcommands: render, inspect, import` + `--state-root` | `eggplan markdown render - Markdown interchange <ARG>` + `--state-root` + `--output <FILE>` |
| `markdown render --help` | the whole top-level usage (`eggplan [--state-root PATH] [--json] COMMAND`, `init - create a state root`, …) | identical to `help markdown render` above |
| `help plan set-status` | `eggplan plan - plan lifecycle and structure <SUBCOMMAND> <ARG>` + `subcommands: set-status, edit, export` + `--state-root` | `eggplan plan set-status - plan lifecycle and structure <ARG>` + `--state-root` + `--to <VALUE>` + `--expected-revision <N>` |
| `help plan export` | (parent page, no `--to`) | `eggplan plan export - plan lifecycle and structure <ARG>` + `--state-root` + `--output <FILE>` (no `--to`) |
| `help item update` | `eggplan item - update a plan item <SUBCOMMAND> <ARG> <ARG>` + `subcommands: update` + 5 options | `eggplan item update - update a plan item <ARG> <ARG>` + the same 5 options, no `subcommands:` line |

Exact after-text, from the built binary:

```text
$ eggplan help markdown render
eggplan markdown render - Markdown interchange <ARG>
  --state-root <PATH>  repository state root (default .eggplan)
  --output <FILE>  write rendered markdown to a file

$ eggplan help plan set-status
eggplan plan set-status - plan lifecycle and structure <ARG>
  --state-root <PATH>  repository state root (default .eggplan)
  --to <VALUE>  target plan status [active|blocked|cancelled]
  --expected-revision <N>  compare-and-swap expected revision

$ eggplan help item update
eggplan item update - update a plan item <ARG> <ARG>
  --state-root <PATH>  repository state root (default .eggplan)
  --expected-revision <N>  compare-and-swap expected revision
  --status <VALUE>  set the item status [pending|actionable|in_progress|blocked|completed|cancelled]
  --blocker <TEXT>  set the item blocker
  --next-action <TEXT>  set the item next action
```

Errors:

```text
$ eggplan help markdown nonsense
help: usage: unknown subcommand "nonsense"; expected one of render, inspect, import
$ echo $?
2

$ eggplan help markdown render extra
help: usage: help accepts a command and a subcommand; got 3 positional arguments
```

The unknown-subcommand message is byte-identical to the validator's message for
the same mistake, because both now come from the same function
(`crates/eggplan-cli/src/lib.rs:639-647`).

## Production implementation evidence

**`crates/eggplan-cli/src/commands.rs`**

- `command_help` (`:479-505`) takes a `subcommand: Option<&str>` and takes its
  option list from `options_for(name, subcommand)` (`:424`) — exactly the set
  the validator checks — plus the parent's flags. The `subcommands:` line is
  emitted only when no subcommand was named.
- New `positional_shape` (`:512-540`): with a subcommand named, the parent's
  `SUBCOMMAND` slot is replaced by the subcommand's name and only the
  remaining positionals are advertised.

**`crates/eggplan-cli/src/lib.rs`**

- `help_result` (`:547-557`) now resolves command *and* subcommand before
  rendering.
- New `help_target` (`:570-608`): `eggplan help COMMAND SUBCOMMAND` takes the
  command from `positional[0]` and the subcommand from `positional[1]`;
  `eggplan COMMAND SUBCOMMAND --help` takes the command from `args.command`
  and the subcommand from `positional[0]`. A third positional to `help` is a
  `usage` error.
- New `resolve_help_subcommand` (`:616-633`): a word that is not a subcommand
  of the named command is a `usage` error. An unknown *command* is not an
  error — it keeps today's fallback to the top-level usage.
- New `unknown_subcommand_message` (`:639-647`), now also used by the
  validator (`validate_command_options`, `:652-688`).

**`crates/eggplan-cli/tests/help_fidelity.rs`** (new) — 9 tests.

## Agreement test

The load-bearing test is mechanical, not case-by-case: for all 36
(command, subcommand) pairs from `eggplan_cli::command_metadata()`, every
declared option is offered to the real validator
(`eggplan <command> [<sub>] <option> <value> --json`) and the ones it refuses
are compared with the options the help page lists. It runs ~430 real
invocations in ~0.4 s.

```text
running 9 tests
test an_unknown_command_still_falls_back_to_the_top_level_usage ... ok
test a_third_positional_to_help_is_a_usage_error ... ok
test a_subcommand_help_flag_does_not_fall_back_to_the_top_level_usage ... ok
test the_json_envelope_carries_the_same_page_as_the_human_output ... ok
test a_subcommand_page_lists_the_options_that_subcommand_accepts ... ok
test a_bare_help_still_prints_the_top_level_usage ... ok
test an_unknown_subcommand_is_a_usage_error_naming_the_accepted_subcommands ... ok
test both_help_forms_render_the_same_page ... ok
test every_help_page_lists_exactly_the_options_the_validator_accepts ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.47s
```

The test's power was checked by mutation: with `options_for(name, subcommand)`
temporarily replaced by `options_for(name, None)` in the renderer, the sweep
failed with `left: {"--state-root"}  right: {"--expected-revision",
"--state-root", "--to"}` for `plan set-status`. The mutation was reverted
immediately.

## Nothing else moved

- **Command inventory**: `git diff -U0 crates/eggplan-cli/src/commands.rs`
  shows hunks only at old line 472 onward. The `COMMANDS` table (`:210`), the
  option constants, `usage()` (`:448-468`), and `options_for` (`:424`) are
  untouched.
- **Top-level usage text**: unchanged. `diff <(eggplan --help)
  crates/eggplan-cli/tests/fixtures/help.txt` is empty.
- **Frozen help fixture**: byte-identical. sha256
  `b4348d56f800c129e67fc0e44b94fd014d33ba861c0aea50e7b97e53db1cd751` both on
  disk and at `HEAD`.
- **JSON envelope**: shape unchanged — `schema_version: 1`, `command`, `ok`,
  `data`, `error`, `warnings`, `warnings_truncated`; the help payload keys are
  still exactly `{"usage", "command"}`. Only the *value* of `data.command`
  now carries the subcommand page, which is the fix.
- **Machine codes**: unchanged. The only code the new paths can emit is the
  pre-existing `usage`.

## Verification executed

All commands were run from the repository root at the reviewed baseline plus
this pass's changes.

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | pass, no output |
| `cargo check --workspace --all-targets --locked` | pass |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | pass, no warnings |
| `cargo test --workspace --locked` | 348 passed, 0 failed, 3 ignored (339 at the reviewed baseline, plus the 9 new help-fidelity tests) |
| `bash scripts/check-projection-cli-boundary.sh` | pass |
| `bash scripts/check-core-boundary.sh` | pass |
| `bash scripts/check-codegg-compat-boundary.sh` | pass |
| `bash scripts/check-integrations-boundary.sh` | pass |
| `bash scripts/check-closure-authority-boundary.sh` | pass |
| `cargo +1.89.0 check --workspace --all-targets --locked` | pass |
| `cargo +1.89.0 test --workspace --locked` | 348 passed, 0 failed, 3 ignored |
| `git diff --check` | pass, no output |

Not run, and recorded as such: the hosted CI matrix (ubuntu/macOS/Windows
native plus the MSRV job) has not run for this change. The local evidence
above covers Linux only.

## Invariant review

- **One owner of the truth.** The help page and the validator now read the
  same option list, so the property is structural rather than a coincidence
  maintained by hand. The agreement test sweeps every pair.
- **The top-level usage is frozen.** The frozen fixture still matches byte for
  byte, so the change cannot have altered the command surface an adopter sees
  at the top level.
- **No schema, envelope, or machine-code change.** The envelope's keys and
  `schema_version` are unchanged; the only new failure mode is the
  pre-existing `usage` code.
- **No data migration.** Nothing on disk changed format.

## Failure, recovery, and contention review

The new code paths are pure. `help_target` and `resolve_help_subcommand` read
the static command table and the argument vector; they perform no I/O and hold
no state. An unknown command keeps its previous behaviour (the top-level
usage), so no previously working invocation changes meaning. An unknown
subcommand and a third positional to `help` are `usage` errors with exit
status 2, consistent with the CLI's existing usage-error contract.

## Migration and compatibility review

No migration. Two behaviours change, both from wrong to right:

- `eggplan help CMD SUBCOMMAND` now prints the subcommand's page instead of
  the parent's. The parent's page is still reachable with `eggplan help CMD`.
- `eggplan CMD SUBCOMMAND --help` now prints the subcommand's page instead of
  the top-level usage.

Both were previously printing something that did not answer the request.

## Security and trust review

No trust boundary moved. The change affects only what the CLI *prints*. It
adds no dependency, changes no credential or identity handling, and exposes
nothing new. The unknown-subcommand error names the accepted subcommands,
which are already listed by the parent page's `subcommands:` line.

## Documentation and operations

- `docs/cli-reference.md:25-53` — the `help` row is now
  `help [COMMAND [SUBCOMMAND]]`; the section states that `help COMMAND
  SUBCOMMAND` and `COMMAND SUBCOMMAND --help` are the same request, shows the
  `markdown render` / `--output` example, states that an unknown subcommand is
  a usage error naming the accepted subcommands, and records that the built-in
  usage line still spells the command `help [ARG]`.
- `architecture/cli-control-surface.md:130-165` — the "One declarative command
  inventory" section now records that the help page and the validator read the
  same list, that a subcommand page lists the parent's options plus that
  subcommand's own, that the `subcommands:` line belongs to the parent page
  alone, and that `crates/eggplan-cli/tests/help_fidelity.rs` proves the
  agreement property.
- `architecture/deep-dive-projection-cli.md` — a new help bullet in §3 and a
  new test-strategy bullet for `help_fidelity.rs`; every CLI citation
  re-verified against the new line numbers.
- `plans/closure/projection-cli/003b-c002-closed.md` cited
  `architecture/cli-control-surface.md:129` for the "### Shell completion
  depth" subsection. That anchor was already stale before this pass, and this
  pass moved it further. The closed record was **not** rewritten; it carries a
  dated factual erratum naming the current anchor
  (`architecture/cli-control-surface.md:167`), and its conclusion is
  unchanged.

## Unresolved findings

| Finding | Severity | Disposition |
|---|---|---|
| The hosted CI matrix has not run for this change | Informational | Recorded above as not run. The job identifiers are to be added to this record as a factual amendment once CI completes. |
| The built-in usage line still spells the command `help [ARG]`, because the metadata table's positional shapes are frozen by this plan's scope | Low | Recorded in `docs/cli-reference.md:53`. The usage line describes the top-level command surface, which this plan did not change. Not a defect in the help pages this pass fixed. |
| `eggplan show <id> --help` renders `show`'s help rather than erroring, because for a command with no subcommands the first positional is the command's own argument | Low | Deviation recorded in the source plan's handoff notes and implemented consistently with the validator, which skips the subcommand check for such commands. Erroring there would have made `eggplan show <id> --help` unusable. |
| `eggplan help list extra` is worded `unknown subcommand "extra"; list accepts no subcommand` rather than the `expected one of …` template | Low | The shared template has an empty list for a command with no subcommands. The validator's own wording is unchanged; the two now share one function. |

No finding is left open against a claim this pass makes.

## Roadmap and registry disposition

- Projection/CLI subsystem: unchanged at `active/current`. This corrective did
  not open or gate any milestone and did not serialize another subsystem.
- The registry row for this plan moves from `ready` to `closed` and points at
  this record.

## Registry updates

`plans/registry.md` — the row for
`plans/implementation/projection-cli/004-c001-cli-help-subcommand-fidelity.md`
is moved from `ready` to `closed` and points at this record. The subsystem row
for Projection/CLI is unchanged at `active/current`.
