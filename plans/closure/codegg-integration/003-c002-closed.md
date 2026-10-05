# CodeGG Integration M003 C002 — Compatibility Boundary Guard Completeness — Closed

Status: closed

Source implementation plan:

- plans/implementation/codegg-integration/003-c002-boundary-guard-completeness.md

Source roadmap:

- plans/subsystems/codegg-integration-roadmap.md

Reviewed baseline: `2ac47b0`

Implementation commit: `8c4f6e6e586b2704311bda81d7dc402203971e80`

Hosted qualification: GitHub Actions run
[37269198631](https://github.com/eggstack/eggplan/actions/runs/37269198631) at
the closure commit, conclusion `success`. See "Verification executed".

## Executive finding

C002 closed two enforcement defects in
`scripts/check-codegg-compat-boundary.sh`. The source guard now fails on
filesystem access, not only process/network/database, and every guard's failure
message now states the exact file and section it scanned. All five guards gained
deterministic synthetic self-proofs that run against fixtures in a temporary
directory.

Recorded plainly, as the plan's handoff notes require: **the production bridge
was always pure.** `crates/eggplan-codegg-compat/src/lib.rs` imports only
`std::collections` — no `std::fs`, no `File::`, no process, no network, no
database. The defect was in the enforcement, not in the crate. This is not a
bridge correctness defect and no mapping, projection, digest, or ID changed.

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
| 1. Filesystem access in `src/` fails the guard | `no_impure_source` (`scripts/check-codegg-compat-boundary.sh:64-66`) matches `std::fs`, `std::path`, `tempfile::`, the `use`-imported `fs::` call forms, and `File::open`/`create`/`create_new`/`open_options`. Self-proofs at `:215-246` assert each form fails. Demonstrated before/after against a fixture using `std::fs::read_to_string`: the old alternation produced no match, the new one matched at line 2. |
| 2. Process, network, and database access still fail | `std::process`, `Command::new`, `std::net`, `TcpStream`, `UdpSocket`, `reqwest::`, `sqlx::`, `tokio::` all retained; `async_std::` added. Self-proofs cover twelve process/network/database forms. |
| 3. The owned-identity guard still fails and does not false-positive | `no_owned_identity` (`:56-58`) unchanged in pattern. Self-proofs assert all ten identities fail as `pub struct`, `pub enum`, and `pub trait`, and that a comment mention, a private `struct`, and a `pub const` string containing `WorkOrder Goal TodoState` all pass. |
| 4. The legal `[dev-dependencies]` entry on `eggplan-repo` continues to pass | `no_production_repo_dep` (`:32-39`) is still `[dependencies]`-scoped. Self-proofs assert a manifest with `eggplan-repo` under `[dev-dependencies]` passes and one with it under `[dependencies]` fails. The real `Cargo.toml:14` entry is unchanged. |
| 5. The asymmetry is resolved deliberately and the decision recorded | Option 1 implemented; decision and rationale recorded in this record and in the script header (`:21-25`). No guard was weakened. |
| 6. All five guards have deterministic synthetic self-proofs | `prove` helper (`:120-131`) drives fixtures built by `manifest_fixture`/`source_fixture` (`:111-118`) in a `mktemp -d` root, asserting an expected fail/pass per case. Per-guard assertion counts: guard 1 ×3, guard 2 ×3, guard 3 ×10 (8 client crates + dev-`tokio` + a clean manifest), guard 4 ×32 (10 identities × 3 declaration forms, plus incidental mentions and a clean source), guard 5 ×19 (18 impure forms + one pure `std::collections` control). **Total 67**, measured by running an instrumented copy of the script that counts `prove` invocations, not by counting call sites by hand. |
| 7. Header states all five guards and their scope; every message matches what was scanned | Header at `:7-29` names each guard, its scan scope, and why only guard 1 is section-scoped. `run_guards` (`:68-95`) emits five messages, each naming the file and, for manifest guards, the section. |
| 8. `architecture/codegg-compat.md` and `architecture/deep-dive-tooling-governance.md` describe the guard accurately | `codegg-compat.md` now describes all five guards, the deliberate asymmetry and why it exists, and the self-proofs. `deep-dive-tooling-governance.md` describes each named function with its new line range, the added filesystem coverage, and the self-proofs. `deep-dive-codegg-compat.md` finding 10 is marked closed and its guard list rewritten. |
| 9. The `registry.md` citation is unambiguously a CodeGG repository path with attribution preserved | `plans/registry.md` item 7 now reads "in the CodeGG repository `dbowm91/codegg` — not a path in this repository" and drops the false `plans/implementation/` prefix. The `418fdc85656e7e1faa57f71e5e7f10f7f4859c60` attribution and hosted run `36106606574` are intact. |
| 10. No production Rust source, dependency set, schema, or public API changes | `git status --short crates/eggplan-codegg-compat/` is empty at the implementation commit; `git diff --stat crates/` is empty. Only `scripts/`, `plans/`, and `architecture/` changed. |
| 11. Native and MSRV qualification pass with all five guards green | See "Verification executed". |

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

Not run: none of the §6 list. Windows qualification is covered by the hosted
matrix; the guard itself is skipped on Windows runners, which
`architecture/deep-dive-tooling-governance.md` finding 1 already records, so
there is nothing Windows-specific to verify here.

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
| `tempfile` as a *production* dependency is not rejected by any guard | Low | Open, out of C002 scope. `tempfile` is currently a legal dev-dependency, so it cannot be added to the unscoped guard 3 without failing the tree. Adding it would require a sixth, section-scoped guard. Recorded rather than silently widened into scope. The source-side exposure is closed: `tempfile::` is matched by `no_impure_source`, so an actual use in `src/` fails even if the dependency were promoted. |
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
