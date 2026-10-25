# Projection/CLI M004 C001 — CLI Help Must Agree With the CLI's Own Option Validation

Status: closed

Repository baseline: ee54a0b8e2cb0d843565c54930ad2c21111320db

Source roadmap:

- plans/subsystems/projection-cli-roadmap.md

Long-term requirements:

- plans/000-long-term-specification.md (the CLI is a thin adapter over stable
  core semantics)

Related ADRs: ADR-0002, ADR-0005.

Predecessor plan/closure:

- plans/implementation/projection-cli/003b-c001-cli-user-documentation-contract-corrections.md
- plans/closure/projection-cli/003b-c001-closed.md

Primary class: correctness of a promised capability

## 1. Objective

Make `eggplan help` show the options a command actually accepts.

The command metadata table is the single source of truth for the command
surface, the help text, the option validation, and the shell completions.
Three of those four consumers scope options per subcommand. The fourth — the
help renderer — does not, so help and validation disagree, and a user reading
`eggplan help markdown render` cannot learn that `--output` is accepted.

This pass changes the CLI's help rendering and the documents that describe
it. It adds no command, no option, and no machine code.

## 2. Findings

### E-PC-C001-01 — `help COMMAND SUBCOMMAND` shows the parent's options

`crates/eggplan-cli/src/commands.rs:472` — `command_help(name)` looks up the
command's own spec and renders `spec.options` and `spec.flags`. The
subcommand-scoped options live in `spec.sub_options`
(`crates/eggplan-cli/src/commands.rs:91`) and are never rendered.

Reproduced against the current tree:

    $ eggplan help markdown render
    eggplan markdown - Markdown interchange <SUBCOMMAND> <ARG>
      subcommands: render, inspect, import
      --state-root <PATH>  repository state root (default .eggplan)

`--output` is accepted by `markdown render` — the validator proves it:

    $ eggplan markdown inspect x.md --output out.md --state-root .eggplan --json
    {
      "schema_version": 1,
      "command": "markdown",
      "ok": false,
      "data": null,
      "error": {
        "code": "usage",
        "message": "option --output is not valid for this command; accepted: --state-root, --format"

The help text does not list it.

The same holds for the `plan` family added by M004: `eggplan help plan
set-status` omits `--to` and `--expected-revision`.

### E-PC-C001-02 — `COMMAND SUBCOMMAND --help` shows the whole usage instead of the command's help

`crates/eggplan-cli/src/lib.rs:546` — `help_result` resolves the requested
command as `args.positional.first()`. For the `eggplan markdown render
--help` form, `args.command` is `markdown` and `args.positional` is
`["render"]`, so `positional.first()` is `"render"` — a subcommand name, not
a command name. `command_help("render")` returns `None`, and the fallback is
the entire usage text.

Reproduced against the current tree:

    $ eggplan markdown render --help
    eggplan [--state-root PATH] [--json] COMMAND
    init - create a state root
    new - create a plan from a definition
    ... 19 lines of top-level usage ...

`eggplan plan set-status --help` and `eggplan item update --help` behave the
same way. The user asked for the command's help and received the program's
usage.

Both manifestations share one root cause: the help path never resolves the
subcommand word, and `command_help` cannot render a subcommand.

### E-PC-C001-03 — the shell completions already do the right thing

`crates/eggplan-cli/src/commands.rs:601` — `completion_options` derives a
command's option set as `spec.options` plus `spec.sub_options` plus
`spec.flags`, and `options_for` at `:424` does the same for the validator.
The completions therefore already offer `--output` for `markdown render`.

This is the standard the help renderer must meet: the three consumers must
agree, or one of them is wrong. Here the help renderer is the wrong one.

## 3. Controlling semantics

- The command metadata table is the only command inventory. Help, validation,
  and completions must be three renderings of it, never three independent
  lists.
- The options a help page lists are exactly the options the validator
  accepts for that command and subcommand. A help page that lists fewer is
  wrong in the same way a validator that rejects more is wrong.
- `eggplan help COMMAND SUBCOMMAND` and `eggplan COMMAND SUBCOMMAND --help`
  are the same request and must produce the same answer.
- A word that is not a subcommand of the named command is a usage error, not
  a silently ignored word. The validator already refuses
  `eggplan markdown nonsense` with `usage: unknown subcommand ...`; the help
  path must agree.
- The help path must not become a way to bypass the command inventory. A
  subcommand is only a subcommand if the metadata table says so.

## 4. Scope

In scope:

- `crates/eggplan-cli/src/commands.rs`
- `crates/eggplan-cli/src/lib.rs`
- `crates/eggplan-cli/tests/**`
- `docs/cli-reference.md`
- `architecture/cli-control-surface.md`
- `architecture/deep-dive-projection-cli.md`

Out of scope:

- the set of commands, flags, options, and value sets — the metadata table's
  contents must not change;
- the JSON envelope, its schema version, and every machine code;
- the completion scripts' content, beyond any change that follows from the
  metadata table staying identical;
- `plans/`.

## 5. Required production changes

1. `command_help` must accept the subcommand and render:
   - the header `eggplan <command> <subcommand>`, with the positional shape
     the subcommand actually consumes rather than the parent's
     `SUBCOMMAND` slot;
   - the union of the parent's options and the subcommand's own options —
     the same set `options_for` returns for that command and subcommand;
   - the parent's flags;
   - the subcommand list only when no subcommand was named.
2. `help_result` must resolve the command and the subcommand from the parsed
   arguments for both request forms:
   - `eggplan help COMMAND SUBCOMMAND` — the command is the first positional
     and the subcommand the second;
   - `eggplan COMMAND SUBCOMMAND --help` — the command is `args.command` and
     the subcommand is the first positional.
3. A positional that is not a subcommand of the named command must be a
   `usage` error naming the accepted subcommands, matching the validator's
   wording.
4. A third positional to `help` must be a `usage` error.
5. The usage text, the command inventory, and every machine code must be
   unchanged.

## 6. Failure, cancellation, restart, and contention semantics

The help path is read-only and holds no lock. It must not touch the
repository: `eggplan help` works without a state root today and must continue
to.

## 7. Required tests

- A test that `eggplan help markdown render` lists `--output` and that
  `eggplan help markdown inspect` does not.
- A test that `eggplan help plan set-status` lists `--to` and
  `--expected-revision`, and that `eggplan help plan export` does not.
- A test that `eggplan markdown render --help` and `eggplan help markdown
  render` produce the same help text.
- A test that `eggplan markdown render --help` no longer falls back to the
  top-level usage.
- A test that `eggplan help markdown nonsense` is a `usage` error naming the
  accepted subcommands.
- A test that `eggplan help` with no argument still prints the top-level
  usage, and that `eggplan help list` still prints the `list` command's
  help.
- A test that the options a help page lists are exactly the options the
  validator accepts for that command and subcommand — the agreement property,
  asserted mechanically rather than case by case.

## 8. Verification

    cargo fmt --all -- --check
    cargo check --workspace --all-targets --locked
    cargo clippy --workspace --all-targets --locked -- -D warnings
    cargo test --workspace --locked
    bash scripts/check-core-boundary.sh
    bash scripts/check-codegg-compat-boundary.sh
    bash scripts/check-integrations-boundary.sh
    bash scripts/check-projection-cli-boundary.sh
    bash scripts/check-closure-authority-boundary.sh
    cargo +1.89.0 check --workspace --all-targets --locked
    cargo +1.89.0 test --workspace --locked
    git diff --check

The help output must additionally be captured before and after, and the
before/after diff shown in the closure evidence.

## 9. Acceptance criteria

1. `eggplan help markdown render` lists `--output`; `eggplan help markdown
   inspect` does not.
2. `eggplan help plan set-status` lists `--to` and `--expected-revision`.
3. `eggplan markdown render --help` prints the `markdown render` help, not
   the top-level usage.
4. `eggplan help markdown render` and `eggplan markdown render --help`
   produce identical text.
5. `eggplan help markdown nonsense` is a `usage` error naming the accepted
   subcommands.
6. The options listed by any command's help are exactly the options the
   validator accepts for that command and subcommand.
7. The command inventory, the usage text, the JSON envelope, and every machine
   code are unchanged.
8. `crates/eggplan-cli/tests/fixtures/help.txt` is unchanged, because the
   top-level usage is unchanged.

## 10. Stop conditions

- The fix would require changing the metadata table's contents.
- The fix would require a new machine code or a change to the envelope.
- The agreement property cannot be asserted mechanically, which would mean the
  help renderer keeps its own list.

## 11. Closure evidence required

- The before/after help output for each affected command and subcommand.
- The agreement test's output.
- The coordinator's verification table, with "not run" rows recorded honestly.
- An explicit statement that the command inventory, the usage text, the
  envelope, and every machine code are unchanged.

## 12. Handoff notes

- This plan is **ready**. The defect is reproduced in two forms, the root
  cause is identified in two functions, and the required changes are
  specified concretely.
- The defect was found during the 2026-10-25 audit and recorded as an
  unresolved finding in
  `plans/closure/projection-cli/003b-c001-closed.md`, where it was explicitly
  deferred because correcting it is a production change.
- `architecture/deep-dive-projection-cli.md` and
  `architecture/cli-control-surface.md` carry citations into the two files
  this plan edits. Those citations must be re-verified after the change.
