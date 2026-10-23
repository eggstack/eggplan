# Tooling and Planning Governance — Deep Dive

This note reviews the repository's mechanical tooling (CI, static boundary
guards, verify order) and its planning/closure governance as an operational
control system. For the crate map and product boundary, see
[architecture overview](overview.md) (companion to the
per-subsystem architecture notes).

## 1. Boundary scripts: what each enforces and how

All five scripts live in `scripts/`, use `set -euo pipefail`, and are
text/regex guards — not type-system or Cargo-feature enforcement.

### `check-core-boundary.sh` (mechanism: `rg`)

Three guards, each stated with the exact scope it scans, so a failure message
never claims more than was checked. The header at `:30-59` states all three
with their scan scope, and every failure message names the file and the
scope it scanned.

- Tool precondition (`:17-26`, applied at `:28`): `require_tools rg mktemp`
  refuses with `required boundary-guard tool '<tool>' is not installed;
  refusing to report a pass` and exits non-zero rather than reporting an
  unscanned pass. The precondition is a function so the self-proofs can
  exercise the refusal path against a deliberately absent tool name
  (`:303-315`).
- `no_prohibited_dependency` (`:108-110`) `rg -ni` over
  `crates/eggplan-core/Cargo.toml` for the dependency families `tokio |
  async-std | reqwest | hyper | sqlx | rusqlite | diesel | subprocess |
  bollard | rmcp | openai | ollama`. The scan is not section-scoped: it
  covers `[dev-dependencies]` and `[build-dependencies]` as well.
- `no_impure_source` (`:116-118`) `rg -n` over `crates/eggplan-core/src` for
  filesystem, process, network, path, environment, database, and Git access:
  `std::fs`, the `use`-imported `fs::` call forms, `File::`, `std::process`,
  `process::Command`, `Command::new`, `std::env`, the `env::` call forms,
  `std::net`, `TcpStream`, `UdpSocket`, `std::path`, `PathBuf`, `Path::`,
  `sqlx | rusqlite | diesel | sqlite | postgres | sea_orm | tokio_postgres`,
  and `git2 | gix | git::` (`:69-86`). The manifest guard cannot express
  this boundary — `std::fs`, `std::process`, `std::net`, and `std::path` are
  in the standard library and need no dependency at all — so only a source
  scan can detect them.
- `no_reasoning_field` (`:121-123`) `rg -n` over the same tree for
  `pub ... (thinking | scratchpad | chain_of_thought | hidden_reasoning)`.
- Every guard maps ripgrep's exit status through the shared `scan` helper
  (`:94-104`): `0` is a violation, `1` is clean, and any larger status means
  the scan could not be performed. A missing or unreadable path is reported
  and exits non-zero; it is never reported as a pass.
- `run_guards` (`:146-157`) applies all three to the real tree (`:162`).
- Self-proofs (`:164-315`) run the same functions against `mktemp -d`
  fixtures with positive and negative cases for every guard: a clean
  manifest, nine prohibited dependency families, a dev-dependency, twelve
  impure source forms, core's own vocabulary that must not trip the guard
  (`EvidenceKind::Command`, a `path` variable, a `"git"` subject kind,
  `SourcePath::`, `InputFile::`), four reasoning-field forms, three
  assertions over the real tree, and the precondition refusal. They clean up
  with a targeted non-recursive `rm -f`/`rmdir` (`:171-175`) and never write
  tracked source, so the proofs are safe to run in CI.
- Intent: keep `eggplan-core` a pure deterministic domain (no async/DB/net/
  process/fs/path/Git access, no persisted model-reasoning fields).

### `check-codegg-compat-boundary.sh` (mechanism: `awk` + `rg`)

Five guards, only the first of which is section-scoped. Each is a named
function; the header at `:20-42` states all five with their scan scope, and every
failure message names the file and section it scanned. Corrective C002 added the
filesystem coverage in the fifth guard and the self-proofs; before it the
messages claimed more than was checked.

- `no_production_repo_dep` (`:45-52`) `awk` state machine tracks the
  `[dependencies]` section of `crates/eggplan-codegg-compat/Cargo.toml` and
  fails if `eggplan-repo =` appears there (dev-dependencies are out of scope for
  that test, which is what keeps the legal `Cargo.toml:14` entry legal).
- `no_codegg_dependency` (`:56-58`) `rg` fails on `codegg | codegg-core =` at
  line start in the same manifest: the bridge must not depend on CodeGG. Not
  section-scoped, so a dev- or build-dependency declaration would also fail.
- `no_client_dependency` (`:62-64`) `rg` fails on `tokio | sqlx | reqwest |
  hyper | ureq | surf | isahc | async-std =` at line start in the manifest: no
  async, database, or network client may enter the bridge's dependency graph at
  all (also not section-scoped).
- `no_owned_identity` (`:69-71`) `rg` fails on `pub (struct | enum | trait |
  type) (WorkOrder | Goal | GoalVerification | TodoState | WorkPlanCheckpoint |
  ContextEpoch | AgentRunExecutor | JobExecutor | WorktreePolicy |
  SandboxPolicy)` in `src/`: the bridge owns only the WorkPlan assessment
  seam, not scheduler/runtime identity. Declarations only, so comments and
  private positions are not failures.
- `no_impure_source` (`:77-79`) `rg` fails in `src/` on process, **filesystem**,
  network, and database access: `std::process | Command::new | tokio:: |
  reqwest:: | hyper:: | sqlx:: | async_std:: | std::net | TcpStream | UdpSocket
  | std::fs | std::path | tempfile::`, the `use`-imported `fs::` call forms,
  and `File::open | create | create_new | open_options`.
- `run_guards` (`:81-109`) applies all five to the real tree. Self-proofs
  (`:114-260`) run the same functions against `mktemp -d` fixtures with positive
  and negative cases for every guard, and clean up with a targeted
  non-recursive `rm -f`/`rmdir`. No tracked file is written, so the proofs are
  safe to run in CI.

### `check-integrations-boundary.sh` (mechanism: `rg`)

Six named guards plus one contract check, each stated with the exact scope it
scans; the header at `:18-39` states all six with their scan scope, and every
failure message names the file and the scope it scanned.

- Tool precondition (`:11-16`): `command -v` for both `rg` and `awk`; a
  missing tool prints `required boundary-guard tool '<tool>' is not
  installed; refusing to report a pass` and exits 1 instead of reporting an
  unscanned pass.
- `no_sibling_runtime` (`:44-46`) `rg -n` over
  `crates/eggplan-integrations/Cargo.toml` for `eggwork | eggsearch |
  eggbench | eggsact | eggsec | codegg | eggpool` in dependency-key position,
  so Eggplan's own `eggplan-codegg-compat` is not a false positive while
  `eggbench-core` is. Not section-scoped: `[dev-dependencies]` and
  `[build-dependencies]` are covered too.
- `no_transport_dep` (`:49-51`) fails on `tokio | reqwest | hyper | ureq |
  surf | isahc | async-std | axum | octocrab | git2 | gix` at line start in
  the same manifest — no async runtime, HTTP client, or Git transport.
- `no_credential_dep` (`:55-57`) fails on `secrecy | keyring | jsonwebtoken |
  oauth2 | wiremock` at line start — an adapter is handed already-verified
  facts and must never hold or request a secret.
- `no_process_source` (`:61-63`) `rg -n` over the whole
  `crates/eggplan-integrations/src` tree for `std::process |
  process::Command | Command::new | std::env::var | exec( | spawn(`.
- `no_acquisition_source` (`:67-69`) `rg -n` over the same tree for
  `std::fs | std::path | tempfile::`, the `use`-imported `fs::` call forms,
  `File::(open | create | create_new | open_options)`, `read_dir`,
  `std::net`, `TcpStream`, `UdpSocket`, `reqwest::`, `ureq::`, `octocrab::`,
  `which::`, and `which(` — no filesystem walk, network, or executable
  discovery, because an adapter normalizes facts a host already verified and
  never acquires evidence.
- `no_trust_enrollment` (`:73-75`) `rg -n` over the same tree for
  `register_trusted | ProviderRegistry` — enrolling provider trust stays an
  explicit host policy action through `ProviderRegistry::register_trusted`.
- `requires_core_contract` (`:78-80`) fails unless `eggplan-core` appears in
  the manifest: the generic SPI must still be built on the Eggplan core
  evidence contract.
- Guards 1-3 are deliberately unscoped by manifest section, including
  `[dev-dependencies]`: an adapter must not pull in a sibling runtime, a
  transport, or a credential helper even for tests. Guards 4-6 scan the whole
  source tree rather than a hand-listed set of modules, so a newly added
  adapter cannot escape enforcement by not being named.
- `run_guards` (`:82-119`) applies all seven checks to the real tree
  (`:276`). Self-proofs (`:121-274`) run the same functions against
  `mktemp -d` fixtures with positive and negative cases for every guard, and
  clean up with a targeted non-recursive `rm -f`/`rmdir` (`:128-132`). No
  tracked file is written, so the proofs are safe to run in CI.

### `check-projection-cli-boundary.sh` (mechanism: `grep -RInE`)

Four `grep -RInE` guards, no tool precondition; each failure message names
the surfaces it scanned.

- Projection (`crates/eggplan-projection/Cargo.toml` +
  `crates/eggplan-projection/src`, `:10`): fails on `eggplan-cli |
  eggplan-repo | clap | crossterm | ratatui | std::process | Command::new |
  tokio:: | reqwest:: | hyper:: | mcp` — the library stays reusable.
- Markdown (`crates/eggplan-markdown/Cargo.toml` +
  `crates/eggplan-markdown/src`, `:15`): fails on `eggplan-cli |
  eggplan-repo | clap | tokio | reqwest | hyper | mcp | std::process |
  Command::new` — bounded parser library only.
- CLI (`crates/eggplan-cli/Cargo.toml` + `crates/eggplan-cli/src`, `:20`):
  fails on `tokio | reqwest | hyper | mcp | async.runtime`.
- CLI (`crates/eggplan-cli/src` only, `:25`): separately fails on
  `std::process::Command | Command::new | .output() | .spawn()` — no
  command execution to produce evidence.

### `check-closure-authority-boundary.sh` (mechanism: embedded `python3`)

- Strips `/* … */` and `//…` comments, then regex-scans only two files:
  `crates/eggplan-repo/src/lib.rs` and `crates/eggplan-repo/src/store.rs`.
- Forbids public exposure of `SubjectCapture | GitSubjectCapture |
  ScriptedSubjectCapture` via direct (`pub use store::SubjectCapture;`) or
  grouped/multiline (`pub use store::{Allowed, SubjectCapture};`) imports,
  `pub (trait | struct) <Name>`, and any
  `pub fn finalize_closure_with_*`.
- `pub(crate)` forms deliberately do not match. The script header states
  compile-fail doctests remain the primary public-API boundary; this script
  catches accidental source-level exposures.
- Includes a deterministic synthetic self-test (`prove(...)` defined at
  `:36-39`, exercised at `:44`–`:49`) for direct/grouped re-exports of all
  three hidden types, alternate finalizers, and crate-private forms before
  scanning the real files. A final case asserts that a commented-out
  `pub use store::SubjectCapture;` and a plain `pub fn finalize_closure()`
  are both accepted, so comment-stripping and the `_with_` suffix rule are
  proven, not assumed.
- The `python3` interpreter is invoked in a plain command position, not
  inside a conditional (`:6`), so a missing interpreter fails the script
  under `set -euo pipefail` rather than reporting an unscanned pass.

## 2. CI matrix (`.github/workflows/ci.yml`, 51 lines)

Two jobs, both on push and pull-request:

| Job | Runner(s) | Toolchain | Steps |
|---|---|---|---|
| `native` | `ubuntu-latest`, `macos-latest`, `windows-latest` (`fail-fast: false`) | stable + `rustfmt, clippy` | `fmt --check` (Linux only), `check --workspace --all-targets --locked`, `clippy --workspace --all-targets --locked -- -D warnings`, `test --workspace --locked`, **ripgrep install** (non-Windows), all 5 boundary scripts (non-Windows only) |
| `msrv` | `ubuntu-latest` only | pinned `1.89.0` (matches workspace `rust-version = "1.89"`) | `check --workspace --all-targets --locked`, `test --workspace --locked` only |

Consequences, verified from the file: `fmt` runs only on Linux; all boundary
guards are skipped on Windows (`if: runner.os != 'Windows'`); the MSRV job
runs no fmt, clippy, or boundary scripts. Workspace (`Cargo.toml`) is
`resolver = "2"`, `edition = "2024"`, seven members, with `--locked` used by
every check/clippy/test invocation.

**Ripgrep must be installed explicitly.** It is not in the stock runner images,
and every `rg`-based guard calls `rg` inside a conditional, so a missing `rg`
exits 127 inside the conditional and the guard reports success having scanned
nothing. This was not hypothetical: `check-core-boundary.sh` and
`check-codegg-compat-boundary.sh` were both inert in CI until corrective C002
added the install steps (`ci.yml:28-33`) and a `command -v rg` precondition to
the codegg guard. Observed directly in run `37265105852`, job `111620146353`:

```text
scripts/check-core-boundary.sh: line 7: rg: command not found
scripts/check-core-boundary.sh: line 12: rg: command not found
```

Both `rg`-based gaps are now closed. `check-integrations-boundary.sh` is an
`rg` guard and was exposed to the same trap; it carries its own
`command -v rg` precondition
(`scripts/check-integrations-boundary.sh:11-16`) and was hardened the same
way, so it is no longer correct to group it with the guards that never used
ripgrep.
`check-core-boundary.sh` now carries its own precondition
(`scripts/check-core-boundary.sh:17-28`) and fails closed on an unscannable
path, so it is no longer the one residual gap; see finding 6.

The two guards that never used ripgrep were never exposed to *this* trap:
`check-projection-cli-boundary.sh` uses `grep -RInE` and
`check-closure-authority-boundary.sh` uses an embedded `python3`, and both
tools ship in the stock runner images. Their exposure is narrower and
different in kind: `grep` is called inside conditionals, so a missing `grep`
would make that guard report an unscanned pass — the shape is open, but the
tool is present in every supported runner image — whereas the `python3` call
sits in a plain command position and so fails the script instead of passing
it. Neither of the two carries an explicit `command -v` precondition.

## 3. Agent verify order (`AGENTS.md`, `README.md`)

Authoritative order per `AGENTS.md`:

1. `cargo fmt --all -- --check`
2. `cargo check --workspace --all-targets --locked`
3. `cargo clippy --workspace --all-targets --locked -- -D warnings`
4. `cargo test --workspace --locked`
5. The five boundary scripts in `core → codegg-compat → integrations →
   projection-cli → closure-authority` order.

Focused variants: `cargo test -p <crate>` and `cargo test -p <crate> <filter>`. The root `README.md` dev-checks block now lists the full gate (all five guards).

## 4. Planning/closure governance as an operational tool

`plans/README.md`, `plans/003-planning-process.md`, and `plans/registry.md`
form a control surface, not documentation decoration:

- **Authority order is explicit and terminating.** `003 §2` (`:13`–`:21`) ranks
  current long-term specification and terminology → accepted ADRs → master
  roadmap → subsystem roadmap → implementation plan → closure/corrective
  records for what actually landed → registry as compact projection, and
  `plans/README.md:19`–`:39` renders the same ladder graphically. An
  implementation plan cannot silently override a long-term invariant or an
  accepted ADR (`003 §2`). This matches the chain in `AGENTS.md` and the
  document set: four canonical direction docs (`000`–`003`), four accepted
  ADRs, six subsystem roadmaps.
- **Registry as control surface.** `plans/registry.md` holds canonical
  direction, the 11-state status vocabulary (`proposed … deferred`), accepted
  ADRs, subsystem status, registered plans, external review baselines, and the
  current execution order. Per `003 §10`–`§11` it links to authority rather
  than duplicating plan content (manual until a future generated registry).
- **Implementation-plan-before-handoff.** `plans/README.md:67` ("Register an
  implementation plan before handing it to an implementation agent") and
  `003 §3` step 6 (`:33`, "Register it before execution") require a bounded,
  registered plan before any implementation agent starts. The content of such
  a plan is fixed by `003 §11` (`:138`–`:161`: status, repository baseline,
  source roadmap, applicable ADRs, readiness/dependencies, exact verification
  commands, closure evidence required) and its `ready` status is gated by
  `003 §5` (`:52`–`:60`), which requires named closure evidence and required
  verification that "can actually establish the claimed boundary".
- **Closure-evidence rule.** A milestone closes only on the evidence its
  source plan names (`plans/README.md:51`–`:54` "Core planning rule",
  `003 §8` `:95`–`:99`). Compilation/formatting alone never closes
  correctness/security/persistence/**recovery**/integration work.
  `conditionally closed` is allowed only with production implementation
  complete and explicitly named, bounded missing external evidence — and
  "the underlying missing evidence remains missing" (`003 §8` `:101`–`:103`).
  `003 §7` (`:75`–`:91`) requires a closure record to distinguish *planned*
  commands from *commands actually run*, and never convert a planned command
  into a passing result because it appears in the source plan. The full
  evidence vocabulary is pass/fail/timeout/environmental block/skipped/not
  run/unavailable external evidence, recorded truthfully (`003 §7`,
  `AGENTS.md` hygiene rule, `registry.md:429`–`:436`).
- **Corrective-plan convention.** Later findings never silently rewrite an
  accepted closure except for factual errata; a new corrective plan references
  its predecessor, enumerates every unclosed finding, identifies controlling
  semantics, adds regression evidence that would have detected the defect, and
  updates registry/roadmap lineage (`003 §9` `:107`–`:116`). Evidence M002
  C001/C002/C003 is the worked example, including explicit non-blocking
  scoping (`registry.md:118`–`:123`, `:296`).
- **Design gates and hygiene.** Twenty-three numbered gates in `registry.md:359`–
  `:425` (canonical JSON freeze, provider-identity authority, append-only
  evidence, finalizer-owned subject capture, test-seam containment,
  Markdown-import limits, staged CodeGG adoption, and the frozen
  `capture_git_subject_fingerprint` digest contract at gate 23) plus the
  hygiene rules at `registry.md:429`–`:436` (register before handoff,
  preserve historical closure/use corrective plans, record exact evidence and
  unrun/blocked checks, sync/deterministic core, no hidden model reasoning in
  persisted schemas, non-blocking hygiene must not serialize independent
  handoffs).

Layout and hygiene, verified by enumeration rather than asserted:

- Naming is mechanical (`plans/README.md:60`–`:65`): ADR
  `adrs/ADR-NNNN-short-title.md`, subsystem roadmap
  `subsystems/<subsystem>-roadmap.md`, implementation plan
  `implementation/<subsystem>/NNN-short-title.md`, closure record
  `closure/<subsystem>/NNN-status.md`. Corrective work extends both to
  `NNN-cNNN-short-title.md` / `NNN-cNNN-closed.md`.
- `implementation/` and `closure/` use the same five subsystem directories
  (`codegg-integration`, `eggstack-integration`, `evidence-closure`,
  `foundation-core`, `projection-cli`). At the recorded baseline (`37ad290`)
  the two trees hold 23 implementation plans against 23 closure records; they
  held 19 against 18 at the C002 closure (`8c4f6e6`). The trees are no longer
  equal by construction: the corrective lines registered after the 2026-10-23
  audit are open and have no closure record yet. A gap in that direction is
  legitimate and expected, because a plan with status `ready` has no closure
  record yet: `plans/003-planning-process.md` §7 requires a closure record to
  distinguish planned from actually-run commands and must never be written
  from a plan. Every `closed` row in the registry resolves to an existing
  closure file, and no closure record is orphaned. Re-derive both counts by
  enumeration rather than trusting this line — they change whenever a plan is
  registered or closed.
- The "historical" qualifications are real and bounded, not discrepancies:
  Foundation M002 is `conditionally closed` (registry `:67`, platform caveat
  resolved by M003) and CodeGG M003's `003-conditionally-closed.md` has since
  been satisfied by hosted qualification while the registry row still reads
  `closed` (registry `:82`). Projection/CLI is no longer unstarted work: M003
  was split into M003a and M003b, both registered and closed (registry
  `:78`–`:79`), with three M003b corrective lines open (registry `:94`–`:96`)
  and M004 proposed (registry `:97`); the subsystem row reads
  `active/current` (registry `:57`).
- `plans/` contains no non-Markdown files, no empty directories, and no
  scratch artifacts: every file under it is Markdown (67 tracked files at the
  recorded baseline `37ad290`).
- `plans/archive/` is currently README-only. Its policy (`archive/README.md`)
  is to retain completed/superseded *interim* planning, preserve relative
  structure, never archive canonical specs or accepted ADRs just because their
  first implementation completed, and keep historical closure discoverable
  under `plans/closure/`. The `archived` status in the vocabulary is therefore
  defined but unused.

## 5. Review findings

**Strengths.** Layered defense: compiler + clippy `-D warnings` + five
fast textual guards + compile-fail doctests + evidence-gated planning.
Guards encode real ADR decisions (provider trust, subject authority, CodeGG
non-ownership) in CI rather than prose. `--locked`, a frozen and byte-stable
schema v1 alongside v2 as the current write version, and
compact-canonical-JSON golden fixtures keep builds and digests reproducible.

**Gaps/risks.**

1. **Windows-skipped guards are the largest hole.** All five boundary
   scripts are skipped on `windows-latest`, so a Windows-only contributor
   (or a Windows-passing PR) can land a boundary violation that only fails
   on Linux/macOS runners. The scripts use `rg`/`grep`/`awk`/`python3`,
   which explains the skip, but the authority boundary (`SubjectCapture`,
   `finalize_closure_with_*`) is exactly what should hold on every OS.
2. **`fmt` only on Linux.** Formatting is enforced only by the
   `ubuntu-latest` leg of the `native` matrix, so a formatting violation can
   reach `macos-latest`/`windows-latest` green. (`AGENTS.md`, `README.md`, and
   `ci.yml` agree on the verify order and on the full five-guard gate, so a
   contributor following either document runs the same gate CI does; an
   earlier draft of this note wrongly claimed the `README.md` dev-checks block
   listed only two guards — `README.md:110`–`:120` lists all five.)
3. **Textual guards are necessary but brittle.** `grep`/`rg` catch direct
   uses but not renamed imports, feature-unified transitive deps, or new
   paths (e.g. a new `src/` file outside the two files the closure guard
   scans). The remainder rests on the doctest and review layers, which CI
   does run cross-platform via `cargo test` — nine `compile_fail` doctests in
   `crates/eggplan-repo/src/lib.rs` are the one authority check that runs on
   every OS.
4. **`registry.md` cited a plan path that does not exist — closed by C002.**
   `registry.md:284`–`:287` names
   `plans/implementation/eggplan-assessment-integration/001-durable-execution-subject-provenance.md`
   as the "upstream provenance predecessor", but `plans/implementation/`
   contains only the five subsystem directories; no such file or directory was
   present. The underlying work closed in the CodeGG repository, so this was a
   cross-repository pointer rendered as an in-repo path, and a reader could not
   tell "never lived here" from "lost". C002 §7 qualified the path visibly as a
   CodeGG repository path and kept the `418fdc85…` attribution, so it no longer
   reads as a local path. Every implementation-plan citation in the registry now
   either resolves or is explicitly external.
5. **`check-codegg-compat-boundary.sh` asymmetry — resolved by C002.** Only the
   `eggplan-repo` guard was `[dependencies]`-scoped, so a dev-dependency on
   `eggplan-repo` was deliberately allowed. The CodeGG and async/db/network
   guards were plain line-start matches over the whole manifest, so a dev- or
   build-dependency on `codegg` or `tokio` failed the guard. That strictness was
   kept deliberately — Option 1 of the C002 decision — and resolved by making
   each message state its actual scan scope in the file header
   (`scripts/check-codegg-compat-boundary.sh:20-42`) and in the failure text,
   so the file no longer leaves a reader guessing which the intent was. One
   correction to the earlier reading of this
   finding: it claimed *all three* manifest guards said "production
   dependencies". Only guard 3 did. Guard 1's message matched its
   `[dependencies]` scope, and guard 2 said "compatibility crate", not
   "production". All five messages now name the file and section scanned, which
   makes the distinction structural rather than a matter of reading closely.
6. **`check-core-boundary.sh` had no tool precondition and never scanned core's
   source for impure access — resolved by C005.** C002 discovered that ripgrep
   is absent from the stock runner images and that this script called `rg`
   inside two `if` statements, so both its guards reported success having
   scanned nothing until `ci.yml` began installing ripgrep. The workflow fix
   made the guard active, but the script still had no `command -v rg` guard
   and no synthetic self-proof, so dropping the install step would silently
   re-inert it; C002 §4 excluded modifying the other four guards, so this was
   reported rather than changed. C005 closed both gaps. The script now
   requires `rg` and `mktemp` up front
   (`scripts/check-core-boundary.sh:17-28`), and a third guard,
   `no_impure_source` (`:116-118`), scans `crates/eggplan-core/src` for
   filesystem, process, network, path, environment, database, and Git access —
   a boundary the manifest guard cannot express, because those APIs are in the
   standard library and need no dependency. Every guard maps ripgrep's exit
   status explicitly, so an unscannable path fails closed instead of
   reporting a pass, and deterministic synthetic self-proofs (`:164-315`) now
   cover each guard. The core crate's source was always pure; the defect was
   in the enforcement, not in the crate.

## Verification pointers

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
bash scripts/check-core-boundary.sh
bash scripts/check-codegg-compat-boundary.sh
bash scripts/check-integrations-boundary.sh
bash scripts/check-projection-cli-boundary.sh
bash scripts/check-closure-authority-boundary.sh
```
