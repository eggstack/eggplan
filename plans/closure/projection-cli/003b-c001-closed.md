# Projection/CLI M003b C001 — Closed

Status: closed

Source plan: plans/implementation/projection-cli/003b-c001-cli-user-documentation-contract-corrections.md

Source roadmap: plans/subsystems/projection-cli-roadmap.md

Reviewed baseline: `37ad290ca6b971a68d2871aedb39f4882f4eb2a8`.

Implementation commit: `a60d36b` ("feat: complete the Projection/CLI line from
the 2026-10-23 repository audit").

This commit identifier was added to this record as a factual amendment after
the fact, following the precedent of
`plans/closure/evidence-closure/002-c004-closed.md`.

Hosted CI was not run for this change. The job identifiers are to be added to
this record as a factual amendment once CI has run, following the precedent of
`plans/closure/evidence-closure/002-c004-closed.md`.

## Executive finding

Projection/CLI M003b C001 closes. The user-facing guides now describe what the
CLI actually does.

The CLI's behavior was already correct and self-consistent. The guides were
the side that was wrong, and one of the six findings had a safety dimension:
`docs/cli-reference.md` stated that `--state-root` is "Never implicit", so an
operator reading the guide would believe the flag could not be omitted — while
the CLI defaults it to `.eggplan` **relative to the current working
directory**. An operator who omitted it in a directory other than the one they
intended would operate on a different state root than they believed, with
nothing in the guide to warn them. The guide now states the default and its
consequence, and still tells operators to pass the flag explicitly.

The other five findings are documentation truthfulness: a blanket
`--expected-revision` claim that the table below it contradicted, an
over-broad `unavailable` check-state description, an `item update` requirement
stated as "exactly one" when the implementation accepts "at least one" and
applies every supplied field, two undocumented commands, and an incomplete
value-taking flag list.

This pass also carries the user-facing documentation for the M004 `plan`
command family, so that the reference describes the final command surface
rather than an intermediate one.

## Requirement-to-evidence matrix

| Plan requirement (§9) | Evidence | Status |
|---|---|---|
| 1 — the guides no longer state that `--state-root` is never implicit, and describe the `.eggplan` default accurately | `docs/cli-reference.md:12` and `:20-23`; `docs/getting-started.md:37-39` | met |
| 2 — the revision-flag requirement is stated per command, and the guide no longer contradicts itself | `docs/cli-reference.md:161-176` — the mutating-commands table now carries an explicit `--expected-revision` column; `docs/getting-started.md:152-159` | met |
| 3 — the `unavailable` check state is described as the CLI emits it, including `not_run`/`skipped` → `incomplete` | `docs/cli-reference.md:313` and `:320-323` | met |
| 4 — the `item update` requirement is stated as at least one field, with multiple fields applied together | `docs/cli-reference.md:178-180`; `docs/getting-started.md:117-118` | met |
| 5 — `init` and `help` are documented, and the value-taking flag list is complete | `docs/cli-reference.md:16-18` and `:25-37` | met |
| 6 — every CLI command appears in the reference, and every documented flag exists in the code, verified in both directions | see "Forward and reverse coverage" below | met |
| 7 — the guide and the CLI's own usage text agree on every point this pass touches | see "Citation-by-citation re-check" below | met |
| 8 — no production behavior, schema, envelope, or diagnostic code changed | `git diff --stat` — the change touches `docs/` only; the verification suite is green and unchanged | met |
| 9 — native and MSRV qualification pass, with the projection/CLI boundary guard green | see "Verification executed" below | met |

## Citation-by-citation re-check

Every corrected statement was re-read against the current implementation
before it was written down.

| Finding | Before | After | Supporting code location |
|---|---|---|---|
| E-PC-C001-01 — `--state-root` | "Location of canonical state (conventionally `.eggplan`). Never implicit." | "Location of canonical state. Defaults to `.eggplan`, relative to the current working directory." | `crates/eggplan-cli/src/lib.rs:333-337` — `execute` resolves the state root from `--state-root` or falls back to `PathBuf::from(".eggplan")` |
| E-PC-C001-02 — `--expected-revision` | "All require `--expected-revision N`." above a table in which `new` and `markdown import` take none | A table with an explicit `--expected-revision` column, plus a paragraph explaining why a creation command has no prior revision to compare against | `crates/eggplan-cli/src/commands.rs` — the `new` and `markdown import` specs carry no `EXPECTED_REVISION`; `crates/eggplan-cli/src/lib.rs:2208-2219` — `expected_revision` reads the option with `required_option`, so it is mandatory for exactly the commands whose metadata spec declares it |
| E-PC-C001-03 — `unavailable` | "Evidence is unavailable/not-run/skipped, or the subject is unavailable" | "An observation's status is `unavailable`, or the Git subject cannot be captured", plus a paragraph stating that `not_run` and `skipped` surface as `incomplete` | `crates/eggplan-cli/src/lib.rs:2050-2056` — `assessment_state` returns `unavailable` only when an observation's status is literally `EvidenceStatus::Unavailable`; `:2062-2063` maps `EvidenceMissingOrUnavailable` to `incomplete` otherwise |
| E-PC-C001-04 — `item update` | "`item update` takes exactly one of `--status`, `--blocker`, or `--next-action`." | "`item update` requires **at least one** … and applies every field you supply" | `crates/eggplan-cli/src/lib.rs:1596-1604` — the usage error text is "provide at least one of --status, --blocker, or --next-action"; `:1632-1647` applies `--status`, `--blocker`, and `--next-action` independently |
| E-PC-C001-05 — `init` and `help` | Neither appears in any table | A "Setup and help" section documents both, with their real behavior | `crates/eggplan-cli/src/commands.rs` — the `init` and `help` entries in the command metadata table; `crates/eggplan-cli/src/lib.rs:421-430` and `:539` |
| E-PC-C001-06 — value-taking flag list | "`--state-root`, `--input`, `--expected-revision`, `--provider-policy`, `--status`, `--blocker`, `--next-action`, `--output`, `--format`" | The list now also carries `--limit`, `--after`, and `--to` | `crates/eggplan-cli/src/commands.rs` — the `list` spec's `--limit` and `--after`, and the `plan set-status` spec's `--to` |

The plan's own citations were re-read before being acted on. Two were stale
relative to the current tree and were corrected rather than followed: the
plan cites `crates/eggplan-cli/src/lib.rs:320-324` for the state-root default
and `:1305-1315`/`:1341-1356` for the `item update` behavior. Both were
re-located against the current file and the corrected locations are recorded
above.

Two statements were checked and left alone because they are already true:
`show`'s Git-subject requirement (`docs/cli-reference.md:213-216`) and the
`unavailable` check-state vocabulary itself. The plan explicitly puts both out
of scope: the first was corrected by the Foundation M003 C002 closure and must
not be reverted, and the second is a core contract rather than a
documentation error.

## Forward and reverse coverage

Both directions were checked mechanically against the live binary, by parsing
`eggplan help COMMAND --json` for every command in the metadata table.

**Forward — commands.** All 19 commands in the command metadata table appear
in `docs/cli-reference.md`. Missing: none.

**Forward — options and flags.** Every option and flag every command accepts
appears in `docs/cli-reference.md`. Missing: none. The per-command inventory
checked was:

```
init:            --state-root
new:             --state-root --input
list:            --state-root --provider-policy --status --limit --after
show:            --state-root --provider-policy
status:          --state-root --provider-policy
ready:           --state-root
graph:           --state-root
check:           --state-root --provider-policy --recover-pending
activate:        --state-root --expected-revision
plan:            --state-root            (subcommands: set-status, edit, export)
item:            --state-root --expected-revision --status --blocker --next-action
evidence:        --state-root            (subcommands: list, show, supersessions)
assess:          --state-root --provider-policy
close:           --state-root --provider-policy --expected-revision
closure:         --state-root            (subcommands: show)
registry:        --state-root --provider-policy   (subcommands: render)
markdown:        --state-root            (subcommands: render, inspect, import)
completions:     --state-root            (subcommands: bash, zsh, fish, powershell)
help:            --help
```

**Forward — subcommands.** Every subcommand of every command appears in
`docs/cli-reference.md`. Missing: none.

**Reverse — documented flags.** Every `--flag` named in `docs/cli-reference.md`
or `docs/getting-started.md` exists in the CLI's option tables. The flags
found were `--after`, `--blocker`, `--expected-revision`, `--format`,
`--help`, `--input`, `--json`, `--limit`, `--next-action`, `--output`,
`--provider-policy`, `--recover-pending`, `--state-root`, `--status`, and
`--to`.

Three of those need a note, and none is a phantom flag:

- `--format` and `--output` are **subcommand-scoped** options. They are
  declared on `markdown inspect`/`markdown import` and `markdown render`
  respectively, and the validator accepts them only there. Verified
  behaviorally: `eggplan markdown render ... --output out.md` is accepted,
  while `eggplan markdown inspect ... --output out.md` is refused with
  `usage: option --output is not valid for this command; accepted:
  --state-root, --format`.
- `--to` is the `plan set-status` option, declared in the metadata table at
  `crates/eggplan-cli/src/commands.rs` with the closed value set
  `active, blocked, cancelled`.
- `--locked` appears in `docs/cli-reference.md:345` inside a
  `cargo run -p eggplan-cli -- … --locked` example. It is a Cargo flag, not a
  CLI flag.

No flag is documented that the CLI does not accept.

## Production implementation evidence

None. This pass changed no production code.

The M004 `plan` family that this pass also documents is implemented in
`crates/eggplan-cli/src/lib.rs` — `plan_command` at `:1338`, `plan_set_status`
at `:1358`, `plan_edit` at `:1431`, `plan_export` at `:1499`,
`plan_status_name` at `:1533`, `parse_settable_plan_status` at `:1549` — and
declared in the command metadata table at
`crates/eggplan-cli/src/commands.rs:293-302`. Its own closure record is
`plans/closure/projection-cli/004-closed.md`.

The documentation added for it:

- `docs/cli-reference.md:51` — `plan export PLAN_ID [--output FILE]` in the
  read commands table.
- `docs/cli-reference.md:169-170` — `plan set-status` and `plan edit` in the
  mutating commands table.
- `docs/cli-reference.md:178-180` — the `item update` field requirement,
  corrected.
- `docs/cli-reference.md:195-212` — the plan-status transition table, the
  settable-status rule, and the `eggplan activate` shorthand note.
- `docs/cli-reference.md:214-245` — a "Plan structure editing" section: the
  export/edit round trip, the four rules, and the statement that `plan edit`
  writes no evidence, provider trust, subject revision, or closure record.
- `docs/getting-started.md:120-133` — "Moving the plan itself".
- `docs/getting-started.md:135-149` — "Changing a plan's structure".

## Verification executed

Coordinator, against the integrated tree:

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo check --workspace --all-targets --locked` | pass |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | pass, no warnings |
| `cargo test --workspace --locked` | **339 passed, 0 failed, 3 ignored** |
| `cargo test -p eggplan-cli --locked` | pass, including 12 passed, 0 failed in `tests/plan_lifecycle.rs` |
| `bash scripts/check-core-boundary.sh` | pass |
| `bash scripts/check-codegg-compat-boundary.sh` | pass |
| `bash scripts/check-integrations-boundary.sh` | pass |
| `bash scripts/check-projection-cli-boundary.sh` | pass (`projection/CLI boundary check passed`) |
| `bash scripts/check-closure-authority-boundary.sh` | pass |
| `cargo +1.89.0 check --workspace --all-targets --locked` | pass |
| `cargo +1.89.0 test --workspace --locked` | **339 passed, 0 failed, 3 ignored** |
| `git diff --check` | pass |
| forward/reverse coverage check (the script in this record) | no missing command, option, flag, or subcommand; no phantom flag |

The three ignored tests are the pre-existing characterization tests carried
from the baseline.

The coverage check was run against the built binary, not against the
metadata table read by eye, so it reflects the command surface a user
actually sees.

## Invariant review

- The guides now state only what the CLI provides. Where the CLI's behavior is
  a consequence of a closed milestone's design — the `--state-root` default —
  the guide states it and still gives the operator the same advice, rather
  than claiming a guarantee that does not exist.
- No production behavior, schema, envelope, or diagnostic code changed. The
  verification suite is green and no test was modified.
- The guide's command inventory and the CLI's command inventory are the same
  set, checked in both directions.

## Failure, recovery, and compatibility review

- The same command invocations produce the same outputs and the same exit
  codes before and after this pass. Nothing in `crates/` changed.
- The one behavior a reader of the old guide might have relied on — that
  omitting `--state-root` was impossible — was never true, so no working
  procedure breaks. The guide now warns against the mistake instead of
  denying it.

## Security, trust, and path review

- The `--state-root` correction is the one item with a security dimension, and
  it makes the guide strictly more accurate: it now tells the operator that
  the default is `.eggplan` under the current working directory, which is the
  condition under which a command could operate on an unintended state root.
- The `plan edit` documentation states that the command writes no evidence,
  provider trust, subject revision, or closure record, so a reader does not
  infer that a structural edit carries authority.
- No credential, path, or trust surface is introduced by a documentation
  change.

## Documentation and operations

- `docs/cli-reference.md` — findings E-PC-C001-01, -02, -03, -04, -05, and
  -06 corrected; the `plan` family added to the read and mutating command
  tables; a "Plan structure editing" section added; the plan-status
  transition table added.
- `docs/getting-started.md` — the `--state-root` statement corrected; the
  `item update` field requirement stated; "Moving the plan itself" and
  "Changing a plan's structure" sections added; the revision-guard paragraph
  rewritten to distinguish creation commands from revisioned mutations.
- `docs/README.md` and `docs/evidence-and-closure.md` were checked for the
  same claims. Neither repeats the `--state-root` "never implicit" claim, the
  blanket `--expected-revision` claim, the `unavailable` over-broad
  description, or the `item update` "exactly one" claim, so neither needed a
  change.
- `architecture/deep-dive-projection-cli.md` and
  `architecture/cli-control-surface.md` were updated in the same pass that
  repaired their citations and recorded the `plan` family. That repair
  corrected one claim that repeated a C001 finding: the deep dive said
  `--expected-revision` was required by "exactly the three revisioned
  mutations", which was true when written and is now false, because the M004
  `plan set-status` and `plan edit` are revisioned mutations too. It now
  names all five.

## Unresolved findings

1. **Medium — hosted CI has not run for this change.** The ubuntu, macOS, and
   Windows jobs and the MSRV job are outstanding. This record does not claim
   them as passing.
2. **Low — `eggplan help <COMMAND> <SUBCOMMAND>` does not show the
   subcommand's own options.** Found during the forward coverage check of this
   pass. The option *validator* scopes options per subcommand — `markdown
   render --output` is accepted and `markdown inspect --output` is refused —
   but the help renderer looks the command up by its parent name, so
   `eggplan help markdown render` prints only `--state-root` and omits
   `--output`, and `eggplan help plan set-status` omits `--to` and
   `--expected-revision`. This is a help-fidelity gap in the CLI, not a
   documentation error, and correcting it would be a production change, which
   this plan's §4 places out of scope. The user-facing reference documents
   each subcommand's options correctly. Recorded for a future corrective line.
3. **Informational — the `--state-root` default remains.** The guide now
   describes it accurately, but the CLI still defaults it. Making the flag
   mandatory would be a breaking change to a closed milestone and is a
   separate decision.

## Deviations from the plan

1. **The pass also documented the M004 `plan` command family.** The plan's
   §4 places the plan-lifecycle capability gap out of scope as *capability
   work*. Documenting a command that already exists is not that work: M004 was
   implemented in the same wave, and a reference that omitted a shipped
   command would have been inaccurate by the time it was read. The
   documentation added is limited to describing existing, verified behavior.
2. **The plan's citations for `crates/eggplan-cli/src/lib.rs` were stale.**
   They were re-located against the current tree rather than followed, and
   the corrected locations are recorded in the re-check table.

## Roadmap disposition

`plans/subsystems/projection-cli-roadmap.md` — the M003b corrective lines
C001, C002, and C003 are now all closed. The "### M003b corrective lines"
table gains a Closure column recording this closure record for C001. M004 is
closed as well, with its decisions recorded in
`plans/adrs/ADR-0005-plan-lifecycle-and-structure-editing-through-the-cli.md`.

## Registry updates

`plans/registry.md` — the row for
`plans/implementation/projection-cli/003b-c001-cli-user-documentation-contract-corrections.md`
is moved from `ready` to `closed` and points at this record. The "Correctives
raised by the 2026-10-23 repository audit" section records this closure in its
Projection/CLI subsection.
