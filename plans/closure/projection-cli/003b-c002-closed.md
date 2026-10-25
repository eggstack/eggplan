# Projection/CLI M003b C002 — Closed

Status: closed

Source plan: plans/implementation/projection-cli/003b-c002-shell-completion-depth-parity.md

Source roadmap: plans/subsystems/projection-cli-roadmap.md

Reviewed baseline: `37ad290ca6b971a68d2871aedb39f4882f4eb2a8`.

Implementation commit: to be recorded once the change is committed; the
commit identifier is to be added to this record as a factual amendment after
the fact, following the precedent of
`plans/closure/evidence-closure/002-c004-closed.md`.

Hosted CI was not run for this change. The job identifiers are to be added to
this record as a factual amendment once CI has run, following the precedent of
`plans/closure/evidence-closure/002-c004-closed.md`.

## Executive finding

Projection/CLI M003b C002 closes. The four generated shell completion scripts
now reach the same declared depth, and the depth is stated in the generator, in
the user-facing documentation, and in the design record.

The defect was that the generator held one model of the command table and four
different consumers of it. Fish consumed every level the model carried; Bash
omitted subcommands; Zsh omitted subcommands, options, and option values; and
PowerShell omitted subcommands and option values. Because the fragments are
format strings, the Rust compiler could not see it: Bash assigned a `subs`
local it never read, and Zsh assigned `subcommands`, `options`, and `values`
that it never read. The omission was invisible to the build and to the test
suite, which asserted only that a name appeared somewhere in the script text —
a check the old Zsh script satisfied while offering nothing.

The generator is now one model with four renderers
(`crates/eggplan-cli/src/commands.rs:516-532` states the depth; `:601` builds the
per-command option set; `:634` builds the model once and dispatches). All four
shells offer top-level commands, subcommands, options including
subcommand-scoped options, and option values.

A defect that the plan did not anticipate was found and fixed: the **before**
Zsh script was not valid zsh at all. `zsh -n` on it reports a parse error near
`|` at line 6, because `subcommands=(… evidence:list|show|supersessions …)`
places a `|` inside an array literal. The before Bash, Fish, and PowerShell
scripts parsed cleanly. The after Zsh script parses.

## Requirement-to-evidence matrix

| Plan requirement | Evidence | Status |
|---|---|---|
| All four shells complete to the same stated depth | `crates/eggplan-cli/src/commands.rs:494` (`COMPLETION_DEPTH` and its in-file comment); `crates/eggplan-cli/tests/batch_and_completions.rs:1555` (`completion_scripts_reach_the_same_declared_depth_in_every_shell`) | met |
| No generated script assigns a variable it does not use | `crates/eggplan-cli/src/commands.rs` generators `:653`, `:737`, `:840`, `:880`; `crates/eggplan-cli/tests/batch_and_completions.rs:1625` (`completion_scripts_declare_no_unused_shell_variables`) | met |
| The completion test asserts per-shell depth, not mere mention | `crates/eggplan-cli/tests/batch_and_completions.rs:1555`; the candidate-list readers `offered_names`, `array_literal_names`, `fish_candidate_names`, `quoted_values`, `powershell_array_names` at `:1218` and following | met |
| The tautological assertion is replaced by one with a stated intent | `crates/eggplan-cli/tests/batch_and_completions.rs:1665` and the "no invented names" assertion in the same file | met |
| The intended depth is stated in the generator and in `docs/cli-reference.md` | `crates/eggplan-cli/src/commands.rs:516-532`; `docs/cli-reference.md:145` ("Completion depth") | met |
| Completion generation stays deterministic and side-effect free | `crates/eggplan-cli/tests/batch_and_completions.rs:1708` (`completion_generation_is_deterministic_and_has_no_side_effects`) unchanged and green | met |
| The `COMMANDS` metadata table, the CLI command surface, and the JSON envelope do not change | `git diff` on `crates/eggplan-cli/src/commands.rs` begins at the generator; the table, `usage()`, `options_for()`, and `command_help()` are untouched; `completions` still returns `{shell, bytes, script}` at envelope schema version 1 | met |
| Every generated script is valid in its shell | see "Shell-syntax validation" below | met |

## Production implementation evidence

### One model, four renderers

`crates/eggplan-cli/src/commands.rs:516-532` declares the intended depth in an
in-file comment and in a constant the generators read. `CompletionOption`,
`CommandCompletion`, and `CompletionModel` at `:535`, `:544`, and `:557` are the
model. `completion_options()` at `:601` derives each command's option set as
`spec.options` plus `spec.sub_options` plus `spec.flags`, deduplicated by name
with value sets unioned — the same notion of "the options of a command" that
`options_for()`, the validator's own function, uses.
`completion_script()` at `:634` builds the model once and delegates to `bash()`
at `:653`, `zsh()` at `:737`, `fish()` at `:840`, and `powershell()` at `:880`.

PowerShell's script block now declares `param($wordToComplete, $commandAst)`.
`$cursorPosition` was assigned and never read, so it was dropped; the completer
was re-driven to confirm it still works.

### Depth before and after

| Shell | bytes before | bytes after | depth before | depth after |
|---|---|---|---|---|
| Bash | 1013 | 2848 | commands, options | commands, subcommands, options, values |
| Zsh | 1115 | 2927 | commands only | commands, subcommands, options, values |
| Fish | 7572 | 7994 | commands, subcommands, options, values | same, plus subcommand-scoped options |
| PowerShell | 1179 | 3262 | commands, options | commands, subcommands, options, values |

Before, the Bash fragment was:

```sh
local cur commands="init new …" subs="item:update evidence:list|show|supersessions …" opts="--state-root --state-root …"
COMPREPLY=( $(compgen -W "$commands $opts" -- "$cur") )
```

`subs` was assigned and never read. Before, the Zsh fragment was:

```zsh
local -a commands subcommands options values
commands=(init new …)   subcommands=(item:update …)   options=(--state-root …)   values=(--status=draft,active,…)
_describe 'command' commands
```

Three arrays were assigned and never read, and the `subcommands` literal was
not valid zsh.

After, the Bash fragment dispatches on the command and the previous word:

```sh
case "${command}:${previous}" in
  list:--status) candidates=(draft active blocked closed cancelled) ;;
  item:--status) candidates=(pending actionable in_progress blocked completed cancelled) ;;
  markdown:--format) candidates=(eggplan codegg auto) ;;
esac
case "${command}" in
  evidence) candidates=(list show supersessions) ;;
esac
case "${command}" in
  list) candidates+=(--state-root --provider-policy --status --limit --after) ;;
esac
```

### The new assertions

`crates/eggplan-cli/tests/batch_and_completions.rs:1555` —
`completion_scripts_reach_the_same_declared_depth_in_every_shell` — loops over
`["bash", "zsh", "fish", "powershell"]` and, for each shell, asserts:

```rust
// Depth 1: every top-level command name.
for spec in &metadata { assert!(offered.contains(&spec.name), …); }
// Depth 2: every subcommand of every command that has subcommands.
for spec in &metadata { for subcommand in &spec.subcommands { … } }
// Depth 3: every option and flag of every command that has options.
for spec in &metadata { for option in spec.options.iter().chain(spec.flags.iter()) { … } }
// Depth 4: every fixed value of every option that declares values.
for (_, _, values) in EXPECTED_OPTION_VALUES { for value in *values { … } }
```

The assertion is uniform across the four shells, not Zsh-only. `offered` is
read from each script's *candidate lists*, not from raw text, so a script that
merely mentions a name does not pass.

`crates/eggplan-cli/tests/batch_and_completions.rs:1625` —
`completion_scripts_declare_no_unused_shell_variables` — asserts that no
generated script declares an unread variable.

`crates/eggplan-cli/tests/batch_and_completions.rs:1665` —
`completion_option_value_sets_match_the_metadata` — cross-checks the closed
value sets against the CLI's own help output and usage message.

### Proof that the new test detects the old scripts

The same helper code was compiled standalone and run against the captured
before-state and after-state scripts:

```
AFTER  bash: offered=58 missing=[]           unused_vars=[]
       zsh: offered=96 missing=[]           unused_vars=[]
       fish: offered=86 missing=[]          unused_vars=[]
       powershell: offered=96 missing=[]    unused_vars=[]
BEFORE bash: offered=0  missing=[evidence, supersessions, …, auto, codegg]  unused_vars=["subs"]
       zsh: offered=62 missing=[supersessions, …, --format, --output, …]    unused_vars=["subcommands","options","values"]
       fish: offered=80 missing=[format, output, auto, codegg]              unused_vars=[]
       powershell: offered=54 missing=[supersessions, …]                    unused_vars=[]
```

All four "before" shells fail the new assertions. All four "after" shells
pass.

### Shell-syntax validation

| Shell | Version | Result |
|---|---|---|
| bash | 5.2.21 | `bash -n` **OK**; functional drive of `_eggplan` OK |
| zsh | 5.9 | `zsh -n` **OK**; functional drive (stubbed `_describe`) OK |
| fish | 3.7.0 | `fish --no-execute` **OK**; `complete -C` OK |
| pwsh | 7.6.8 | AST parse **no errors**; `TabExpansion2` OK |

zsh and fish were not present on the machine when the work started. They were
installed so that this validation could be real rather than recorded as "not
available". That installation is a machine-level change outside the repository;
no repository file was affected.

## Verification executed

Coordinator, against the integrated tree:

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo check --workspace --all-targets --locked` | pass |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | pass, no warnings |
| `cargo test --workspace --locked` | **339 passed, 0 failed, 3 ignored** |
| `bash scripts/check-core-boundary.sh` | pass |
| `bash scripts/check-codegg-compat-boundary.sh` | pass |
| `bash scripts/check-integrations-boundary.sh` | pass |
| `bash scripts/check-projection-cli-boundary.sh` | pass (`projection/CLI boundary check passed`) |
| `bash scripts/check-closure-authority-boundary.sh` | pass |
| `cargo +1.89.0 check --workspace --all-targets --locked` | pass |
| `cargo +1.89.0 test --workspace --locked` | recorded below |
| `git diff --check` | pass |

Implementation agent, scoped to the crate it owns:

| Command | Result |
|---|---|
| `cargo fmt --package eggplan-cli -- --check` | pass |
| `cargo clippy -p eggplan-cli --all-targets --locked -- -D warnings` | pass (two `needless_borrow` findings fixed) |
| `cargo test -p eggplan-cli --locked` | 42 passed, 0 failed in the owned binaries (`batch_and_completions` 30, `commands` 7, `inspection_snapshot_golden` 5) |
| `cargo test -p eggplan-cli completion --locked` | 8 passed, 0 failed in the owned file |
| `bash scripts/check-projection-cli-boundary.sh` | pass |

The three ignored tests are the pre-existing characterization tests carried
from the baseline; they were ignored at the baseline too.

## Invariant review

- The command metadata table is the single source of truth for the command
  surface, the help text, the option validation, and the completions. This
  change reinforces that: the generators read the table rather than restating
  it.
- Completion generation remains a pure function of the metadata. It reads no
  repository, no state root, and no environment, and it is byte-deterministic
  for the same table.
- No command, flag, help string, machine code, exit code, or envelope field
  changed. The `completions` command still returns `{shell, bytes, script}` at
  envelope schema version 1.

## Failure, recovery, and compatibility review

- The change is confined to the text of the generated scripts. A user's shell
  configuration that sourced a completion script keeps working; the candidates
  it offers are a superset of what it offered before.
- The generated scripts remain static text and remain safe to source.
- No persisted state, digest, schema version, or golden fixture changed.

## Security, trust, and path review

- The generated scripts are built from a compile-time constant table. No
  repository content, plan content, provider identity, or credential can reach
  them.
- The generator performs no I/O of any kind. The existing
  `completion_generation_uses_no_process_filesystem_or_network_api` assertion
  still passes.
- The newly offered option values are the same closed sets the CLI already
  accepted and already validated.

## Documentation and operations

- `docs/cli-reference.md:145` — a new "### Completion depth" section states
  the depth for adopters.
- `architecture/cli-control-surface.md:129` — a new "### Shell completion
  depth" subsection records the design decision.
- `architecture/deep-dive-projection-cli.md` — an as-implemented bullet for the
  generators and a test-strategy bullet. Its citations into
  `crates/eggplan-cli/src/commands.rs` and `crates/eggplan-cli/src/lib.rs`
  were re-verified against the current files after the M004 `plan` family
  shifted them, and corrected where they had drifted.

Every citation written into those three documents was verified by reading the
file at that exact line. The pre-existing `lib.rs` citations in
`architecture/deep-dive-projection-cli.md` were re-verified after the
concurrent `plan` command work changed that file.

## Unresolved findings

1. **Medium — hosted CI has not run for this change.** The ubuntu, macOS, and
   Windows jobs and the MSRV job are outstanding. This record does not claim
   them as passing.
2. **Low — the option value sets are mirrored in the test.** `command_metadata()`
   exposes option names but not value sets, so
   `EXPECTED_OPTION_VALUES` in `crates/eggplan-cli/tests/batch_and_completions.rs:1218`
   mirrors the table's closed sets. The mirror is cross-checked against the
   CLI's own help output and usage message by
   `completion_option_value_sets_match_the_metadata`. A removed value fails the
   presence assertion; an added value fails the "no invented names" assertion.
   The residual risk is that a value set is changed in both the table and the
   mirror in the same commit without the cross-check noticing; that requires
   editing the help text too.
3. **Low — the unused-variable check is textual.** It is not vacuous: it flags
   the old Bash `subs`, the old Zsh `subcommands`/`options`/`values`, and a
   synthetic unused `$unused` in PowerShell.
4. **Informational — the machine gained `zsh` and `fish`.** Both were installed
   so the shell-syntax validation could be real. No repository file changed.

## Deviations from the plan

1. **Subcommand-scoped options are offered in all four shells.** The plan
   referenced "the depth Fish already reaches". The implementation offers
   `spec.sub_options` as well — `markdown render --output`, `markdown
   inspect/import --format` — in every shell, because `options_for()`
   validates against that set and `command_metadata()` exposes it. This is a
   superset of the plan's reference depth, so it satisfies both readings. It is
   the only place where the emitted option set grew.
2. **The depth test is structural rather than substring-based.** The plan's §7
   wording says a name must be "present". A substring check would pass the old
   Zsh script, which mentioned every subcommand and option inside unread
   arrays. The test reads each shell's candidate lists instead, which is
   stronger than the literal wording and is what makes "a Zsh script that
   omits subcommands must fail the test" true.

## Roadmap disposition

`plans/subsystems/projection-cli-roadmap.md` — the M003b corrective lines
C001, C002, and C003 are now all closed. The "### M003b corrective lines"
table gains a Closure column recording this closure record for C002. M004 is
no longer `proposed`; its decisions are recorded in
`plans/adrs/ADR-0005-plan-lifecycle-and-structure-editing-through-the-cli.md`
and its closure is recorded separately.

## Registry updates

`plans/registry.md` — the row for
`plans/implementation/projection-cli/003b-c002-shell-completion-depth-parity.md`
is moved from `ready` to `closed` and points at this record. The "Correctives
raised by the 2026-10-23 repository audit" section records this closure in its
Projection/CLI subsection.
