# Tooling and Planning Governance — Deep Dive

This note reviews the repository's mechanical tooling (CI, static boundary
guards, verify order) and its planning/closure governance as an operational
control system. For the crate map and product boundary, see
[architecture overview](overview.md) (forward reference; companion to the
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

- `awk` state machine tracks the `[dependencies]` section of
  `crates/eggplan-codegg-compat/Cargo.toml` and fails if `eggplan-repo =`
  appears there (dev-dependencies are out of scope for that test).
- `rg` fails on `codegg | codegg-core =` at line start in the same manifest:
  the bridge must not depend on CodeGG.
- `rg` fails on `pub (struct | enum | trait | type)
  (WorkOrder | Goal | GoalVerification | TodoState | WorkPlanCheckpoint |
  ContextEpoch | AgentRunExecutor | JobExecutor | WorktreePolicy |
  SandboxPolicy)` in `src/`: the bridge owns only the WorkPlan assessment
  seam, not scheduler/runtime identity.

### `check-integrations-boundary.sh` (mechanism: `grep -nE` / `grep -RInE`)

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
- Includes a deterministic synthetic self-test (`prove(...)`) for
  direct/grouped re-exports, alternate finalizers, and crate-private forms
  before scanning the real files.

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

Focused variants: `cargo test -p <crate>` and `cargo test -p <crate> <filter>`. Note: root `README.md` lists only two of the five guards — an abbreviated quick-start, not the full gate.

## 4. Planning/closure governance as an operational tool

`plans/README.md`, `plans/003-planning-process.md`, and `plans/registry.md`
form a control surface, not documentation decoration:

- **Registry as control surface.** `plans/registry.md` holds canonical
  direction, the 11-state status vocabulary (`proposed … deferred`), accepted
  ADRs, subsystem status, registered plans, external review baselines, and the
  current execution order. Per `003 §10`–`§11` it links to authority rather
  than duplicating plan content (manual until a future generated registry).
- **Implementation-plan-before-handoff.** `plans/README.md` ("Register an
  implementation plan before handing it to an implementation agent") and
  `003 §3` step 6 require a bounded, registered plan with baseline SHA,
  readiness/dependencies, exact verification commands, and named closure
  evidence before any implementation agent starts.
- **Closure-evidence rule.** A milestone closes only on the evidence its
  source plan names (`plans/README.md` "Core planning rule", `003 §8`).
  Compilation/formatting alone never closes correctness/security/persistence/
  integration work. `conditionally closed` is allowed only with explicitly
  named, bounded missing external evidence. Evidence vocabulary
  (pass/fail/timeout/blocked/skipped/not-run/unavailable) must be recorded
  truthfully (`003 §7`, `AGENTS.md` hygiene rule).
- **Corrective-plan convention.** Later findings never silently rewrite an
  accepted closure except for factual errata; a new corrective plan references
  its predecessor, adds regression evidence, and updates registry/roadmap
  lineage (`003 §9`). Evidence M002 C001/C002/C003 is the worked example,
  including explicit non-blocking scoping.
- **Design gates and hygiene.** Twenty numbered gates in `registry.md`
  (canonical JSON freeze, provider-identity authority, append-only evidence,
  finalizer-owned subject capture, test-seam containment, Markdown-import
  limits, staged CodeGG adoption) plus hygiene rules (register before
  handoff, sync/deterministic core, no hidden model reasoning in schemas).

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
2. **`fmt` only on Linux + abbreviated `README` gate.** Formatting is
   unenforced on macOS/Windows CI, and a developer following only the
   `README.md` dev-checks block runs two of five guards — missing the
   integrations, codegg-compat, and closure-authority gates entirely.
3. **Textual guards are necessary but brittle.** `grep`/`rg` catch direct
   uses but not renamed imports, feature-unified transitive deps, or new
   paths (e.g. a new `src/` file outside the two files the closure guard
   scans). The remainder rests on the doctest and review layers, which CI
   does run cross-platform via `cargo test`.

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
