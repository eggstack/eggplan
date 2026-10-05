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

- Manifest scan: `rg -ni` over `crates/eggplan-core/Cargo.toml` for the
  dependency families `tokio | async-std | reqwest | hyper | sqlx | rusqlite |
  diesel | subprocess | bollard | rmcp | openai | ollama`.
- Source scan: `rg -n` over `crates/eggplan-core/src` for
  `pub ... (thinking | scratchpad | chain_of_thought | hidden_reasoning)`.
- Intent: keep `eggplan-core` a pure deterministic domain (no async/DB/net/
  process/LLM deps, no persisted model-reasoning fields).

### `check-codegg-compat-boundary.sh` (mechanism: `awk` + `rg`)

Five guards, only the first of which is section-scoped. Each is a named
function; the header at `:7-29` states all five with their scan scope, and every
failure message names the file and section it scanned. Corrective C002 added the
filesystem coverage in the fifth guard and the self-proofs; before it the
messages claimed more than was checked.

- `no_production_repo_dep` (`:32-39`) `awk` state machine tracks the
  `[dependencies]` section of `crates/eggplan-codegg-compat/Cargo.toml` and
  fails if `eggplan-repo =` appears there (dev-dependencies are out of scope for
  that test, which is what keeps the legal `Cargo.toml:14` entry legal).
- `no_codegg_dependency` (`:43-45`) `rg` fails on `codegg | codegg-core =` at
  line start in the same manifest: the bridge must not depend on CodeGG. Not
  section-scoped, so a dev- or build-dependency declaration would also fail.
- `no_client_dependency` (`:49-51`) `rg` fails on `tokio | sqlx | reqwest |
  hyper | ureq | surf | isahc | async-std =` at line start in the manifest: no
  async, database, or network client may enter the bridge's dependency graph at
  all (also not section-scoped).
- `no_owned_identity` (`:56-58`) `rg` fails on `pub (struct | enum | trait |
  type) (WorkOrder | Goal | GoalVerification | TodoState | WorkPlanCheckpoint |
  ContextEpoch | AgentRunExecutor | JobExecutor | WorktreePolicy |
  SandboxPolicy)` in `src/`: the bridge owns only the WorkPlan assessment
  seam, not scheduler/runtime identity. Declarations only, so comments and
  private positions are not failures.
- `no_impure_source` (`:64-66`) `rg` fails in `src/` on process, **filesystem**,
  network, and database access: `std::process | Command::new | tokio:: |
  reqwest:: | hyper:: | sqlx:: | async_std:: | std::net | TcpStream | UdpSocket
  | std::fs | std::path | tempfile::`, the `use`-imported `fs::` call forms,
  and `File::open | create | create_new | open_options`.
- `run_guards` (`:68-95`) applies all five to the real tree. Self-proofs
  (`:101-247`) run the same functions against `mktemp -d` fixtures with positive
  and negative cases for every guard, and clean up with a targeted
  non-recursive `rm -f`/`rmdir`. No tracked file is written, so the proofs are
  safe to run in CI.

### `check-integrations-boundary.sh` (mechanism: `grep -nE` / `grep -RInE` / `grep -q`)

- Manifest: fails on `eggwork | eggsearch | eggbench | eggsact | tokio |
  reqwest | hyper | mcp | async.runtime` in
  `crates/eggplan-integrations/Cargo.toml` — no sibling runtimes, transports,
  or acquisition deps.
- Source: fails on `^use (eggwork | eggsearch | eggbench | eggsact)::`,
  `async fn`, `tokio::`, `reqwest::`, `std::process`, `Command::new` under
  `src/` — the SPI normalizes facts, never acquires or executes evidence.
- Trust: fails on `register_trusted | ProviderRegistry` in
  `src/eggwork.rs` and `src/eggsearch.rs` — adapters cannot enroll trust.
- Positive check: fails unless `eggplan-core` appears in `Cargo.toml`.

### `check-projection-cli-boundary.sh` (mechanism: `grep -RInE`)

- Projection (`Cargo.toml` + `src/`): fails on `eggplan-cli | eggplan-repo |
  clap | crossterm | ratatui | std::process | Command::new | tokio:: |
  reqwest:: | hyper:: | mcp` — the library stays reusable.
- Markdown (`Cargo.toml` + `src/`): fails on `eggplan-cli | eggplan-repo |
  clap | tokio | reqwest | hyper | mcp | std::process | Command::new` —
  bounded parser library only.
- CLI (`Cargo.toml` + `src/`): fails on `tokio | reqwest | hyper | mcp |
  async.runtime`; separately fails on `std::process::Command |
  Command::new | .output() | .spawn()` — no runtime/network client and no
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
- Includes a deterministic synthetic self-test (`prove(...)`, `:44`–`:49`)
  for direct/grouped re-exports of all three hidden types, alternate
  finalizers, and crate-private forms before scanning the real files. A final
  case asserts that a commented-out `pub use store::SubjectCapture;` and a
  plain `pub fn finalize_closure()` are both accepted, so comment-stripping
  and the `_with_` suffix rule are proven, not assumed.

## 2. CI matrix (` .github/workflows/ci.yml`, 41 lines)

Two jobs, both on push and pull-request:

| Job | Runner(s) | Toolchain | Steps |
|---|---|---|---|
| `native` | `ubuntu-latest`, `macos-latest`, `windows-latest` (`fail-fast: false`) | stable + `rustfmt, clippy` | `fmt --check` (Linux only), `check --workspace --all-targets --locked`, `clippy --workspace --all-targets --locked -- -D warnings`, `test --workspace --locked`, all 5 boundary scripts (non-Windows only) |
| `msrv` | `ubuntu-latest` only | pinned `1.89.0` (matches workspace `rust-version = "1.89"`) | `check --workspace --all-targets --locked`, `test --workspace --locked` only |

Consequences, verified from the file: `fmt` runs only on Linux; all boundary
guards are skipped on Windows (`if: runner.os != 'Windows'`); the MSRV job
runs no fmt, clippy, or boundary scripts. Workspace (`Cargo.toml`) is
`resolver = "2"`, `edition = "2024"`, seven members, with `--locked` used by
every check/clippy/test invocation.

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
  `AGENTS.md` hygiene rule, `registry.md:279`).
- **Corrective-plan convention.** Later findings never silently rewrite an
  accepted closure except for factual errata; a new corrective plan references
  its predecessor, enumerates every unclosed finding, identifies controlling
  semantics, adds regression evidence that would have detected the defect, and
  updates registry/roadmap lineage (`003 §9` `:107`–`:116`). Evidence M002
  C001/C002/C003 is the worked example, including explicit non-blocking
  scoping (`registry.md:105`–`:110`, `:287`–`:288`).
- **Design gates and hygiene.** Twenty-three numbered gates in `registry.md:209`–
  `:277` (canonical JSON freeze, provider-identity authority, append-only
  evidence, finalizer-owned subject capture, test-seam containment,
  Markdown-import limits, staged CodeGG adoption, and the frozen
  `capture_git_subject_fingerprint` digest contract at gate 23) plus the
  hygiene rules at `registry.md:279`–`:288` (register before handoff,
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
  `foundation-core`, `projection-cli`), 19 implementation plans against 19
  closure records as of the C002 closure (`8c4f6e6`). The counts are currently
  equal because every registered plan is now closed; there is no registered open
  corrective. A gap would be legitimate and expected in the other direction: a
  plan with status `ready` has no closure record yet, because
  `plans/003-planning-process.md` §7 requires a closure record to distinguish
  planned from actually-run commands and must never be written from a plan.
  Every `closed` row in the registry resolves to an existing closure file, and no
  closure record is orphaned. Re-derive both counts by enumeration rather than
  trusting this line — they change whenever a plan is registered or closed.
- The two "historical" qualifications are real and bounded, not
  discrepancies: Foundation M002 is `conditionally closed` (registry
  `:67`, platform caveat resolved by M003) and CodeGG M003's
  `003-conditionally-closed.md` has since been satisfied by hosted
  qualification while the registry row still reads `closed` (`:79`).
  Projection/CLI M003 is not a registered plan at all — it is unstarted work
  awaiting real repository use (`:57`, `:142`).
- `plans/` contains no non-Markdown files, no empty directories, and no
  scratch artifacts across 55 files.
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
non-ownership) in CI rather than prose. `--locked`, frozen schema v1, and
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
   listed only two guards — `README.md:47`–`:55` lists all five.)
3. **Textual guards are necessary but brittle.** `grep`/`rg` catch direct
   uses but not renamed imports, feature-unified transitive deps, or new
   paths (e.g. a new `src/` file outside the two files the closure guard
   scans). The remainder rests on the doctest and review layers, which CI
   does run cross-platform via `cargo test` — six `compile_fail` doctests in
   `crates/eggplan-repo/src/lib.rs` are the one authority check that runs on
   every OS.
4. **`registry.md` cited a plan path that does not exist — closed by C002.**
   `registry.md:158` named
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
   (`:7-29`) and in the failure text, so the file no longer leaves a reader
   guessing which the intent was. One correction to the earlier reading of this
   finding: it claimed *all three* manifest guards said "production
   dependencies". Only guard 3 did. Guard 1's message matched its
   `[dependencies]` scope, and guard 2 said "compatibility crate", not
   "production". All five messages now name the file and section scanned, which
   makes the distinction structural rather than a matter of reading closely.

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
