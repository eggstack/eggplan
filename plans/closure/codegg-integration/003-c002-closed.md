# CodeGG Integration M003 C002 — Compatibility Boundary Guard Completeness — Closed

Status: closed

Source implementation plan:

- plans/implementation/codegg-integration/003-c002-boundary-guard-completeness.md

Source roadmap:

- plans/subsystems/codegg-integration-roadmap.md

Reviewed baseline: `2ac47b0`

Implementation commit: `8c4f6e6e586b2704311bda81d7dc402203971e80`

Hosted qualification: run
[37265105852](https://github.com/eggstack/eggplan/actions/runs/37265105852) for
`4344ab7` concluded **failure** — `native (ubuntu-latest)` job
[111620146353](https://github.com/eggstack/eggplan/actions/runs/37265105852/job/111620146353)
and `native (macos-latest)` job `111620146410` failed because ripgrep is absent
from the runner images, while `msrv` `111620146275` and `native (windows-latest)`
`111620146356` passed. That failure is the evidence for finding
C-CODEGG-C002-04 and is retained here rather than superseded.

Dated amendment, after the workflow fix and tool precondition: run
[37265557533](https://github.com/eggstack/eggplan/actions/runs/37265557533) for
`73df3f1` concluded **success** — all four jobs green, with
`check-core-boundary.sh` and `check-codegg-compat-boundary.sh` running for real
for the first time. Confirmed again on run
[37265723272](https://github.com/eggstack/eggplan/actions/runs/37265723272) for
`fa0da35`: `msrv` [111622025366](https://github.com/eggstack/eggplan/actions/runs/37265723272/job/111622025366),
`native (macos-latest)` `111622025557`, `native (ubuntu-latest)`
`111622025574`, `native (windows-latest)` `111622025625` — all `success`.

## Executive finding

C002 closed two planned enforcement defects in
`scripts/check-codegg-compat-boundary.sh` and, in the process, found and fixed a
third and more serious one that was not in the plan. The source guard now fails on
filesystem access, not only process/network/database; every guard's failure
message now states the exact file and section it scanned; and all five guards
gained deterministic synthetic self-proofs that run against fixtures in a
temporary directory.

Those self-proofs are what surfaced the unplanned finding: ripgrep is absent from
the GitHub runner images, so **both** `rg`-based guards had been silently
passing in CI without scanning anything — including the pre-existing
`check-core-boundary.sh`. That is the reason this pass also changed
`.github/workflows/ci.yml`. It is written up below as
C-CODEGG-C002-04 and is the single most consequential result of this corrective.

Recorded plainly, as the plan's handoff notes require: **the production bridge
was always pure.** `crates/eggplan-codegg-compat/src/lib.rs` imports only
`std::collections` — no `std::fs`, no `File::`, no process, no network, no
database. The defect was in the enforcement, not in the crate. This is not a
bridge correctness defect and no mapping, projection, digest, or ID changed.

## New finding C-CODEGG-C002-04 — ripgrep is absent from the runners, so every `rg` guard was inert in CI

This was not in the plan. It was found by C002's own self-proofs, and it is more
serious than the filesystem hole C002 was opened to fix.

`.github/workflows/ci.yml` never installed ripgrep, and it is not part of the
stock `ubuntu-latest` or `macos-latest` GitHub runner images. Every `rg`
invocation in the boundary guards sits inside an `if` statement, so with `rg`
absent each call exits 127 *inside a conditional*; the `if` takes the else
branch and the guard reports success. Direct evidence from the CI run for the
implementation commit, `native (ubuntu-latest)` job
[111620146353](https://github.com/eggstack/eggplan/actions/runs/37265105852):

```text
scripts/check-core-boundary.sh: line 7: rg: command not found
scripts/check-core-boundary.sh: line 12: rg: command not found
codegg-compat guard self-proof failed: guard2 rejects dev-dependency codegg
  (expected fail=1, got fail=0)
```

Consequences:

- `scripts/check-codegg-compat-boundary.sh` and `scripts/check-core-boundary.sh`
  had **never enforced anything in CI**. Both printed their success line while
  scanning nothing. The other three guards use `grep` and `python3`, which are
  present, and did run for real.
- The C002 filesystem coverage would itself have been cosmetic had the workflow
  not been fixed: the guard would have "passed" in CI without ever invoking
  `rg`.

C002 could not meet its own objective — "the guard actually enforces the
boundary it documents" — without fixing the environment, so the fix is recorded
here as a plan amendment rather than treated as scope creep. Two changes:

1. `.github/workflows/ci.yml` installs ripgrep explicitly on Linux and macOS,
   before the guard steps, with a comment explaining why. This makes both
   previously-inert guards start working.
2. `scripts/check-codegg-compat-boundary.sh` now refuses to report a pass when
   `rg` or `awk` is unavailable. Verified by running the script with ripgrep
   removed from `PATH`: it exits 1 with
   `required boundary-guard tool 'rg' is not installed; refusing to report a pass`.

`six compile_fail doctests` and the closure-authority guard were never affected,
because they use `python3` and `rustdoc` rather than `rg`.

The residual hazard — `scripts/check-core-boundary.sh` still has the same
silent-no-op exposure and no tool precondition — is out of C002's declared
scope, which excludes modifying the other four guards. It is recorded below as
an unresolved finding and needs its own corrective.

## Recorded decision: C-CODEGG-C002-02

The plan's §5 offered two resolutions and preferred **Option 1**: keep the `rg`
guards unscoped as deliberate strictness, and correct the messages to say what
is actually scanned. Option 1 is implemented. Guards 2 and 3 still match any
section, so a dev- or build-dependency on `codegg`, `tokio`, or an equivalent
client still fails. No check was loosened to accommodate anything, and no
compensating guard was needed because nothing was removed.

The reason the asymmetry is correct rather than accidental: the crate
legitimately needs `eggplan-repo` and `tempfile` as dev-dependencies so the
test suite can exercise the real store (`tests/parity.rs`,
`tests/repository_projection.rs`). Guard 1 is `[dependencies]`-scoped for
exactly that reason. For CodeGG itself and for async/database/network clients
there is no such need, so the stricter unscoped reading is kept. The header
now states this rationale in-file, which is what the original file lacked.

## Plan refinement: the message defect was narrower than stated

The plan (§2, C-CODEGG-C002-02) and the pre-existing deep-dive finding both
asserted that the `rg` guards at `:19`, `:24`, and `:34` had messages claiming
"production" while scanning unscoped, and the deep dive went further, saying
"all three messages say 'production dependencies'". On reading the original
script, **only guard 3's message was actually wrong.** Each original message,
classified:

| Guard | Original message | Original scope | Verdict |
|---|---|---|---|
| 1 (`:15`) | "compatibility **production** dependencies must not include eggplan-repo" | `[dependencies]` only | Accurate — matched its scope |
| 2 (`:20`) | "compatibility crate must not depend on CodeGG" | whole manifest | Accurate — says "crate", not "production" |
| 3 (`:25`) | "compatibility **production** dependencies must not include async, database, or network clients" | whole manifest | **Inaccurate** — claimed production, matched every section |
| 4 (`:30`) | "compatibility crate declares ownership outside the WorkPlan seam" | source tree | Accurate |
| 5 (`:35`) | "compatibility **production** source must remain free of process, network, and database access" | source tree | Accurate but incomplete — the source tree *is* production source; it simply did not cover filesystem |

So the real defect count is one inaccurate message, not three. All five messages
were rewritten to name the file and section scanned, which is what C002 §3
requires and which makes the distinction structural rather than a matter of
reading closely. The `architecture/deep-dive-tooling-governance.md` finding that
carried the "all three" claim now records the correction inline.

## Requirement-to-evidence matrix

| Acceptance criterion | Evidence |
|---|---|
| 1. Filesystem access in `src/` fails the guard | `no_impure_source` (`scripts/check-codegg-compat-boundary.sh:77-79`) matches `std::fs`, `std::path`, `tempfile::`, the `use`-imported `fs::` call forms, and `File::open`/`create`/`create_new`/`open_options`. Self-proofs at `:228-259` assert each form fails. Demonstrated before/after against a fixture using `std::fs::read_to_string`: the old alternation produced no match, the new one matched at line 2. |
| 2. Process, network, and database access still fail | `std::process`, `Command::new`, `std::net`, `TcpStream`, `UdpSocket`, `reqwest::`, `sqlx::`, `tokio::` all retained; `async_std::` added. Self-proofs cover twelve process/network/database forms. |
| 3. The owned-identity guard still fails and does not false-positive | `no_owned_identity` (`:69-71`) unchanged in pattern. Self-proofs assert all ten identities fail as `pub struct`, `pub enum`, and `pub trait`, and that a comment mention, a private `struct`, and a `pub const` string containing `WorkOrder Goal TodoState` all pass. |
| 4. The legal `[dev-dependencies]` entry on `eggplan-repo` continues to pass | `no_production_repo_dep` (`:45-52`) is still `[dependencies]`-scoped. Self-proofs assert a manifest with `eggplan-repo` under `[dev-dependencies]` passes and one with it under `[dependencies]` fails. The real `Cargo.toml:14` entry is unchanged. |
| 5. The asymmetry is resolved deliberately and the decision recorded | Option 1 implemented; decision and rationale recorded in this record and in the script header (`:21-25`). No guard was weakened. |
| 6. All five guards have deterministic synthetic self-proofs | `prove` helper (`:133-144`) drives fixtures built by `manifest_fixture`/`source_fixture` (`:124-131`) in a `mktemp -d` root, asserting an expected fail/pass per case. Per-guard assertion counts: guard 1 ×3, guard 2 ×3, guard 3 ×10 (8 client crates + dev-`tokio` + a clean manifest), guard 4 ×32 (10 identities × 3 declaration forms, plus incidental mentions and a clean source), guard 5 ×19 (18 impure forms + one pure `std::collections` control). **Total 67**, measured by running an instrumented copy of the script that counts `prove` invocations, not by counting call sites by hand. |
| 7. Header states all five guards and their scope; every message matches what was scanned | Header at `:20-42` names each guard, its scan scope, and why only guard 1 is section-scoped. `run_guards` (`:81-109`) emits five messages, each naming the file and, for manifest guards, the section. |
| 8. `architecture/codegg-compat.md` and `architecture/deep-dive-tooling-governance.md` describe the guard accurately | `codegg-compat.md` now describes all five guards, the deliberate asymmetry and why it exists, and the self-proofs. `deep-dive-tooling-governance.md` describes each named function with its new line range, the added filesystem coverage, and the self-proofs. `deep-dive-codegg-compat.md` finding 10 is marked closed and its guard list rewritten. |
| 9. The `registry.md` citation is unambiguously a CodeGG repository path with attribution preserved | `plans/registry.md` item 7 now reads "in the CodeGG repository `dbowm91/codegg` — not a path in this repository" and drops the false `plans/implementation/` prefix. The `418fdc85656e7e1faa57f71e5e7f10f7f4859c60` attribution and hosted run `36106606574` are intact. |
| 10. No production Rust source, dependency set, schema, or public API changes | `git status --short crates/eggplan-codegg-compat/` is empty at the implementation commit; `git diff --stat crates/` is empty. Only `scripts/`, `plans/`, `architecture/`, and `.github/workflows/ci.yml` changed. |
| 11. Native and MSRV qualification pass with all five guards green | Locally, every §6 row passes. Hosted: the first run (`37265105852`) **failed** because the runners have no ripgrep, which is what surfaced finding C-CODEGG-C002-04. The workflow fix and the tool precondition were then applied, and runs `37265557533` and `37265723272` are green on all four jobs with both `rg`-based guards executing for real. |

## Production evidence

The guard is `scripts/check-codegg-compat-boundary.sh`. The fifth guard, which
is the substantive fix:

```bash
no_impure_source() {
    rg -n 'std::process|Command::new|tokio::|reqwest::|hyper::|sqlx::|async_std::|std::net|TcpStream|UdpSocket|std::fs|std::path|tempfile::|fs::(read|read_to_string|write|create_dir|remove_file|rename|copy|File|metadata|canonicalize)|File::(open|create|create_new|open_options)' "$1"
}
```

Both call forms are matched deliberately. `std::fs::read_to_string` is caught by
`std::fs`, but a bare `fs::read_to_string` behind `use std::fs;` carries no
`std::` prefix at the call site, so the `fs::` and `File::` alternatives are
required for the guard to hold in the form real code takes.

### Before/after on a filesystem-using fixture

A synthetic bridge source containing
`std::fs::read_to_string("plans/registry.md").unwrap()` was matched against the
pre-fix alternation and the new one:

```text
=== OLD pattern (git HEAD version) vs fs-using source ===
OLD: NO MATCH -> guard passes, filesystem use undetected
=== NEW pattern vs same file ===
2:    std::fs::read_to_string("plans/registry.md").unwrap()
NEW: MATCHED -> guard fails as required
```

## Verification executed

All commands ran locally on Linux. Every row in the plan's §6 list was run; none
is reported from the plan alone.

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo check --workspace --all-targets --locked` | pass |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | pass |
| `cargo test --workspace --locked` | pass — 24 suites, 0 failed |
| `cargo +1.89.0 check --workspace --all-targets --locked` | pass — MSRV 1.89.0 |
| `cargo +1.89.0 test --workspace --locked` | pass — 24 suites, 0 failed |
| `bash scripts/check-codegg-compat-boundary.sh` | pass, including 67 self-proof assertions |
| `bash scripts/check-core-boundary.sh` | pass |
| `bash scripts/check-integrations-boundary.sh` | pass |
| `bash scripts/check-projection-cli-boundary.sh` | pass |
| `bash scripts/check-closure-authority-boundary.sh` | pass, including its own synthetic proofs |
| `git diff --check` | pass — no whitespace errors |
| `bash -n scripts/check-codegg-compat-boundary.sh` | pass — shell syntax valid |
| `git status --short crates/eggplan-codegg-compat/` | empty — bridge crate untouched |
| `git diff --stat crates/` | empty — no crate changed |

Not run at the time of writing: nothing in the §6 list, all of which passed
locally as tabulated above. Hosted qualification is covered by the runs recorded
in "Hosted qualification".

Erratum: an earlier revision of this record named run `37269198631`, an
identifier that does not exist in this repository. It was written before the
push and was never observed. It has been replaced with the real run id
`37265105852`.

The script's self-proof cleanup uses a targeted `rm -f` of the two fixture files
plus `rmdir` of the two directories, not a recursive force removal, so it
leaves no temporary state behind and does not risk an over-broad match on a
variable path.

## Invariant review

- **Bridge purity.** Now mechanically enforced, not merely asserted. Before this
  pass the crate was pure and the guard did not prove it; now both are true and
  the proof is re-checked on every CI run.
- **CodeGG ownership.** `no_owned_identity` still forbids a public declaration
  of any of the ten owned identities, and still ignores incidental mentions.
  Nothing about WorkOrder, scheduler, runtime, or storage ownership moved.
- **Persistence authority.** `no_production_repo_dep` still scopes
  `eggplan-repo` out of production dependencies. The finalizer, evidence ledger,
  and subject authority are untouched.
- **Test-suite capability.** Preserved: the dev-dependencies the parity and
  repository-projection tests rely on remain legal, verified both by self-proof
  and by the real manifest passing.

## Failure, recovery, and compatibility review

- Self-proof failures exit non-zero with the offending label, so a guard that
  stops detecting fails CI loudly rather than silently permitting the boundary
  violation it exists to prevent.
- A self-proof never writes tracked source. Fixtures live in a `mktemp -d`
  directory for the duration of the script only.
- No persisted state, no schema, no flag, and no default behavior changed for any
  consumer. The only observable difference is that a boundary violation in the
  bridge — which does not currently exist — would now fail CI instead of passing.

## Security review

No trust or authority surface changed; the change strictly adds enforcement. No
credential, network, or subprocess surface was introduced. The fixture content
is static text written to a private temp directory, and nothing in the fixtures
is derived from repository or user data. The new `std::path` and `tempfile::`
patterns cannot false-positive on the current bridge source, which imports only
`std::collections`.

## Unresolved findings

| Finding | Severity | Disposition |
|---|---|---|
| `scripts/check-core-boundary.sh` still has no tool precondition: with `rg` absent, both its guards silently report success | **Medium** | Open, needs its own corrective. C002 §4 explicitly excludes modifying the other four guards, so this was not changed here. The workflow now installs ripgrep, so the guard is finally active in CI, but the silent-no-op hazard remains if the install is ever dropped. Recommended fix: the same `command -v rg` precondition, plus a synthetic self-proof in the style C002 introduced. |
| `tempfile` as a *production* dependency is not rejected by any guard | Low | Open, out of C002 scope. `tempfile` is currently a legal dev-dependency, so it cannot be added to the unscoped guard 3 without failing the tree. Adding it would require a sixth, section-scoped guard. The source-side exposure is closed: `tempfile::` is matched by `no_impure_source`, so an actual use in `src/` fails even if the dependency were promoted. |
| `scripts/check-integrations-boundary.sh` greps `register_trusted|ProviderRegistry` only in `eggwork.rs`/`eggsearch.rs`, never `lib.rs` | Low | Open, unrelated subsystem. Noted during the architecture review; owned by the integrations surface, not by this pass. |
| Registry row for CodeGG M003 reads `closed` while its closure file is `003-conditionally-closed.md` | Low | Open, explicitly out of C002 scope (§4). Traceability nit previously flagged to the user; still open. |

No unresolved finding affects the bridge boundary, evidence integrity, or
closure authority.

## Roadmap disposition

The CodeGG integration roadmap remains terminal for M001–M003 and C001. C002 is
now closed, so no registered corrective remains open in any subsystem. C002 did
not gate Eggplan-side or CodeGG-side capability plans, and closing it reopens
nothing: M003 and C001 are untouched, and their closure evidence is
unchanged. The bridge remains pure and the boundary is now actually enforced.

## Registry updates

- `plans/registry.md`: CodeGG integration subsystem note records C002 as
  closed; the C002 registered-plan row moves `ready` → `closed` with its
  closure path; the execution-order note changes from "the only registered open
  corrective" to recording that no corrective is open. Registry item 7's
  cross-repository citation is corrected per C002 §7.
- `plans/subsystems/codegg-integration-roadmap.md`: the `### C002` section
  status moves `ready` → `closed` with the closure path recorded.
