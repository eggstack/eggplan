# Projection/CLI M003b C002 — Shell Completion Depth Parity and Completion-Test Strength

Status: closed (see plans/closure/projection-cli/003b-c002-closed.md)

Repository baseline: 37ad290ca6b971a68d2871aedb39f4882f4eb2a8

Source roadmap:

- plans/subsystems/projection-cli-roadmap.md

Long-term requirements:

- plans/000-long-term-specification.md sections 14-15 and 19-20

Related ADRs: ADR-0002.

Predecessor plan/closure:

- plans/implementation/projection-cli/003b-batch-compact-projections-and-shell-completions.md
- plans/closure/projection-cli/003b-closed.md

Primary class: correctness / completeness of a promised capability

## 1. Objective

Bring the four generated completion scripts to a single, stated depth, and
strengthen the completion test so that it can detect a script that has lost
its subcommand or option coverage.

M003b promised "deterministic Bash/Zsh/Fish/PowerShell completion generation
from one command metadata source". The generation is deterministic and does
come from one metadata source, but the four shells do not consume that source
to the same depth: Fish uses all of it, Bash uses most of it, and Zsh uses
only the top-level command names. The three unused variables in the Bash and
Zsh generators are the visible symptom.

The test that was written with M003b asserts only that each script *mentions*
certain names, so it cannot detect the gap.

## 2. Finding

### E-PC-C002-01 — the four shells consume the command metadata to different depths

The generator is `crates/eggplan-cli/src/commands.rs:507-608`. It builds four
values from the one `COMMANDS` metadata table — `commands`,
`subcommands`, `options`, `values` — and then each shell consumes a different
subset.

**Bash** — `crates/eggplan-cli/src/commands.rs:508-518`:

     local cur commands="{commands}" subs="{subcommands}" opts="{options}"
     ...
     COMPREPLY=( $(compgen -W "$commands $opts" -- "$cur") )

`subs` is assigned at `:511` and never used. Bash offers top-level commands
and options, but not subcommands. `eggplan evidence <TAB>` does not complete
`list`, `show`, or `supersessions`.

**Zsh** — `crates/eggplan-cli/src/commands.rs:519-531`:

     local -a commands subcommands options values
     commands=({commands})
     subcommands=({subcommands})
     options=({options})
     values=({values})
     _describe 'command' commands

`subcommands` (`:525`), `options` (`:526`), and `values` (`:527`) are all
assigned and then never used. Zsh offers top-level command names and nothing
else: no subcommands, no options, no option values.

**Fish** — `crates/eggplan-cli/src/commands.rs:532-571` — uses all of it:
commands (`:534-538`), subcommands (`:539-544`), options (`:545-551`), option
values (`:552-559`), and flags (`:561-568`).

**PowerShell** — `crates/eggplan-cli/src/commands.rs:572-605` — offers
commands (`:577-585`) and options (`:586-596`), but no subcommands and no
option values.

Resulting parity:

| Shell | Top-level commands | Subcommands | Options | Option values |
|---|---|---|---|---|
| Fish | yes | yes | yes | yes |
| Bash | yes | **no** | yes | no |
| Zsh | yes | **no** | **no** | **no** |
| PowerShell | yes | **no** | yes | no |

**Blast radius.** Usability only. No command behavior changes, no output
changes, no evidence or trust effect. A user on Zsh gets materially less
from the completion feature than a user on Fish, from the same metadata.

**Contributing decision.** The M003b implementation generated one metadata
source and then emitted four hand-written shell fragments, each consuming a
different subset. Nothing recorded the intended depth per shell, so the
fragments drifted without any check noticing.

### E-PC-C002-02 — the completion test cannot detect the gap

`crates/eggplan-cli/tests/batch_and_completions.rs:1212-1241`:

    for shell in ["bash", "zsh", "fish", "powershell"] {
        ...
        let script = data["data"]["script"].as_str().unwrap();
        assert!(!script.is_empty());
        assert_eq!(data["data"]["bytes"], script.len());
        for command in ["status", "list", "check", "completions", "registry"] {
            assert!(
                script.contains(command),
                "{shell} completion must mention {command}"
            );
        }
        assert!(!script.contains("definitely-not-a-command"));
        // fish spells long options without the leading dashes.
        assert!(
            script.contains("--provider-policy") || script.contains("provider-policy"),
            "{shell} completion must list options"
        );
    }

Every assertion is satisfied by a script that merely contains the names as
text. The Zsh script, which contains no subcommand and no option at all,
passes this test because it mentions `status`, `list`, `check`, `completions`,
and `registry` — all top-level command names — and because the string
`provider-policy` does not appear in a Zsh script that does not offer options
but does not contain that substring either… except that the assertion is an
`||`, so it passes if either spelling is present, and the current Zsh script
does not contain `provider-policy` at all, which means this assertion is
currently only satisfied for Zsh by the top-level `commands` list happening to
include no such substring — see the closure note below.

The test therefore cannot fail for the condition this finding describes. It
also contains a tautological assertion
(`assert!(!script.contains("definitely-not-a-command"))`) that can never fail
and documents no intent.

**Detection gap.** There is no assertion anywhere in the CLI test suite that
a Zsh script contains a subcommand, because no such assertion was written.

## 3. Controlling semantics

From `plans/subsystems/projection-cli-roadmap.md` §3: "JSON is
stable/versioned where machine consumption is intended." From the M003b plan's
own description, completion generation is one capability across four shells,
not four capabilities. `docs/cli-reference.md:105-118` describes the
completions command to adopters and implies the same parity.

Required behavior after this pass:

- every one of the four shells completes at the same depth, and that depth is
  stated explicitly in the generator and in the user-facing guide;
- no generated script contains a variable that is assigned and never used;
- the completion test asserts the depth, not mere mention;
- completion generation stays deterministic and stays derived from the single
  `COMMANDS` metadata source;
- no command, flag, or output behavior changes.

## 4. Scope

### In scope

- the four shell generators in `crates/eggplan-cli/src/commands.rs`;
- the completion test in
  `crates/eggplan-cli/tests/batch_and_completions.rs`;
- the completions section of `docs/cli-reference.md`;
- the completions statements in `architecture/deep-dive-projection-cli.md` and
  `architecture/cli-control-surface.md`, if they state a depth that the
  generators do not implement;
- registry and roadmap lineage updates.

### Out of scope

- the `COMMANDS` metadata table's contents — the set of commands, flags, and
  option values is not changed by this pass;
- any CLI command, flag, or output contract;
- the JSON envelope and its schema version;
- the compact projection and batch-query work from M003b;
- the user-documentation corrections owned by
  plans/implementation/projection-cli/003b-c001-cli-user-documentation-contract-corrections.md;
- the plan lifecycle and structure-editing capability gap;
- the deferred Interop/distribution subsystem.

## 5. Required production changes

Preferred shape:

- decide the intended completion depth for all four shells and implement it
  for all four. The recommended depth is the depth Fish already reaches —
  top-level commands, subcommands, options, and option values — because it
  is the only depth already implemented and tested, and because the metadata
  source already carries everything needed;
- remove every assigned-but-unused variable from the Bash and Zsh
  generators, so that a future depth regression is visible in the generated
  script rather than hidden in an unused local;
- extend the completion test so that it asserts the depth for each shell
  rather than the presence of a name: for every command with subcommands,
  assert that the script contains each subcommand; for every command with
  options, assert that the script contains each option; and for every option
  with a fixed value set, assert that the script contains those values. The
  assertion must be per-shell, so a Zsh script that omits subcommands fails;
- replace the tautological `definitely-not-a-command` assertion with one that
  has a stated intent, or remove it;
- state the intended depth in the generator's in-file comment and in
  `docs/cli-reference.md`.

**Implementation note.** The shell fragments are format strings, so an unused
shell local is invisible to the Rust compiler. The parity must be enforced by
the test, not by the type system. This is why §7 requires per-shell depth
assertions rather than a shared "the script mentions X" assertion.

Explicitly rejected alternatives:

- documenting the differing depths instead of fixing them, because the
  capability was promised as one generation path across four shells;
- dropping Zsh's completion feature rather than deepening it. A shallower
  but honest script is better than an unused variable set, but removing the
  shell from the supported set is a product decision that belongs to a
  milestone, not to this corrective;
- keeping the shared "mentions" assertion and adding one Zsh-only assertion.
  The depth must be uniform, so the assertion must be uniform too.

## 6. Failure, cancellation, restart, and contention semantics

- Completion generation must remain deterministic: the same command metadata
  must produce byte-identical scripts. The existing determinism test at
  `crates/eggplan-cli/tests/batch_and_completions.rs:1243-1258` must remain
  green.
- Completion generation must remain side-effect free: generating a script
  must not scan the repository or touch the state root. The existing
  no-side-effect assertion must remain green.
- The generated scripts must remain valid shell for all four shells. If the
  implementation introduces shell syntax beyond the current fragments, the
  implementer must verify the scripts parse in the corresponding shell where
  one is available, and record that result.
- No command behavior changes: the same invocations produce the same outputs
  and the same exit codes.

## 7. Required tests

At minimum, the completion test must assert, for each of the four shells:

- every top-level command name is present;
- every subcommand of every command that has subcommands is present;
- every option of every command that has options is present;
- every fixed value of every option that declares values is present;
- the script contains no shell variable that is assigned and never used,
  asserted for the generated Bash and Zsh fragments specifically, since that
  is the mechanism by which the depth regression hid.

The determinism and no-side-effect assertions must remain, unchanged in
substance.

## 8. Verification

Record actual results for:

    cargo fmt --all -- --check
    cargo check --workspace --all-targets --locked
    cargo clippy --workspace --all-targets --locked -- -D warnings
    cargo test --workspace --locked
    cargo test -p eggplan-cli --locked
    cargo test -p eggplan-cli completion --locked
    cargo +1.89.0 check --workspace --all-targets --locked
    cargo +1.89.0 test --workspace --locked
    bash scripts/check-projection-cli-boundary.sh
    git diff --check

The `cargo +1.89.0` rows are the MSRV gate and must be recorded truthfully,
including if the toolchain is unavailable in the implementing environment.
Hosted Linux/macOS/Windows and Rust 1.89 qualification must remain green. If
the implementation changes the generated shell fragments, shell-syntax
validation results for bash, zsh, and fish must be recorded where those
shells are available, and "not available" must be recorded honestly where
they are not.

## 9. Acceptance criteria

C002 closes when:

1. all four shells complete to the same, explicitly stated depth;
2. no generated script assigns a variable it does not use;
3. the completion test asserts per-shell depth — subcommands, options, and
   option values — and fails if any shell regresses;
4. the tautological assertion is replaced by one with a stated intent, or
   removed;
5. completion generation remains deterministic and side-effect free, with the
   existing assertions green;
6. the intended depth is stated in the generator and in
   `docs/cli-reference.md`;
7. the `COMMANDS` metadata table, the CLI command surface, and the JSON
   envelope are unchanged;
8. native and MSRV qualification pass, with the projection/CLI boundary guard
   green.

## 10. Stop conditions

Stop and report rather than improvise if:

- reaching the intended depth requires adding a completion feature that the
  `COMMANDS` metadata table cannot express, which would mean the metadata
  source must change and that is a separate decision;
- implementing the depth would break the determinism guarantee or introduce a
  repository scan into script generation;
- the generated scripts cannot be made valid shell across the four targets
  without dropping a shell from the supported set, which is a product
  decision;
- required verification cannot be obtained honestly.

## 11. Closure evidence required

- implementation commit(s) and PR reference;
- the requirement-to-evidence matrix for every acceptance criterion in §9;
- the generated script for each shell before and after, and the per-shell depth
  assertions that were added;
- explicit confirmation that determinism and no-side-effect behavior are
  unchanged;
- exact verification output, including the MSRV rows, distinguishing planned
  from actually run commands;
- hosted native/MSRV run and job identifiers where available;
- unresolved findings with severity, and roadmap disposition;
- registry updates.

Do not convert a planned command into a passing result because it appears in
§8. If a row was not run, record it as not run or unavailable.

## 12. Handoff notes

- This is a completion-depth correctness pass, not a new capability. The
  M003b commitment was one generation path across four shells; this pass makes
  the four shells agree with that commitment.
- Record truthfully that the generation was deterministic and derived from one
  metadata source throughout. The defect was uneven consumption of that
  source and a test that could not see it.
- Do not absorb the user-documentation corrections or the plan-lifecycle
  capability gap into this pass.
