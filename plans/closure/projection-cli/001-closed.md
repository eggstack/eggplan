# Projection and CLI M001 Closure Record

Status: closed

Source plan: plans/implementation/projection-cli/001-cli-control-surface-and-derived-registry.md

Source roadmap: plans/subsystems/projection-cli-roadmap.md

Implementation commit:

- `cb3a8eb4bceefe7bc5a13fb66b3d638077573cdd` — reusable bounded projections,
  native CLI, repository read-only integrity access, fixtures, docs, and CI
  boundary guard.
- `26da4449e5f54c64fa6266ce44a22b20df96f2b9` — normalize help fixture line
  endings across Windows and Unix.

## Executive finding

Eggplan now has a native human and JSON operator surface backed by
`eggplan-core`, `eggplan-repo`, and the reusable `eggplan-projection` library.
Plan creation and ordinary mutations use repository APIs and revision CAS.
Provider authority comes only from an explicit, strict per-invocation policy
file. Closure uses the guarded Evidence M002 protocol. Read commands do not
recover or mutate repository state unless `check --recover-pending` is
explicitly requested. Runtime registry output is derived from `.eggplan` and
does not edit development planning Markdown.

## Requirement-to-evidence matrix

| Requirement | Evidence |
|---|---|
| Reusable bounded summaries, deterministic readiness/graph, stable reason codes, truncation accounting, versioned JSON envelope | `eggplan-projection`; `tests/projections.rs`; golden `tests/fixtures/status-envelope.json` |
| Native command and help surface | `eggplan-cli`; frozen `tests/fixtures/help.txt`; `help_snapshot_is_stable_and_human_output_needs_no_ansi` |
| Init/new/show/status/ready/graph/check and evidence/closure inspection | `native_cli_smoke_reads_mutates_and_derives_registry`; `check_reports_recovery_required_without_touching_pending_state` |
| Revisioned lifecycle and item mutation, stale revision safety | `native_cli_smoke_reads_mutates_and_derives_registry`; repository CAS regression coverage |
| Strict explicit provider policy; unknown/duplicate/malformed input rejection; no implicit trust | `strict_provider_policy_fails_closed_and_assessment_needs_explicit_file`; `close_uses_explicit_policy_and_guarded_repository_protocol` |
| Guarded close rechecks subject and policy and creates ClosureRecord | `close_uses_explicit_policy_and_guarded_repository_protocol`; Evidence M002 closure contract |
| Check distinguishes corrupt, invalid, incomplete, unavailable, stale, and pending recovery state | CLI `check` diagnostics and per-plan classification; CLI/repository integration tests including read-only pending-state verification |
| Read-only store open performs no recovery; closure record read is deeply validated and bounded | `RepositoryStore::open_read_only`; `read_only_open_reports_pending_closure_without_recovering_or_mutating`; bounded/symlink-safe `closure_record` access |
| Derived runtime registry reports plan status, assessment reason codes, readiness, closure, and bounded blocker details without editing `plans/registry.md` | `registry render`; `native_cli_smoke_reads_mutates_and_derives_registry`; projection truncation tests |
| Cross-platform logical JSON and path behavior | JSON avoids local state-root paths; native CI runs CLI tests on Linux, macOS, and Windows |
| No executor, network provider, terminal dependency in projection layer, or process-spawn evidence path | `scripts/check-projection-cli-boundary.sh`; crate dependency manifests |

## Command and fixture inventory

The frozen help snapshot covers `init`, `new`, `show`, `status`, `ready`,
`graph`, `check`, `activate`, `item update`, evidence list/show/supersessions,
`assess`, `close`, `closure show`, and `registry render`. Representative
machine-output fixtures are `crates/eggplan-cli/tests/fixtures/help.txt` and
`crates/eggplan-projection/tests/fixtures/status-envelope.json`.

## Verification executed

Local Linux on implementation revision
`cb3a8eb4bceefe7bc5a13fb66b3d638077573cdd`:

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo check --workspace --all-targets --locked` | pass |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | pass |
| `cargo test --workspace --locked` | pass; 90 tests |
| `cargo +1.89.0 check --workspace --all-targets --locked` | pass |
| `cargo +1.89.0 test --workspace --locked` | pass; 90 tests |
| `bash scripts/check-core-boundary.sh` | pass |
| `bash scripts/check-codegg-compat-boundary.sh` | pass |
| `bash scripts/check-integrations-boundary.sh` | pass |
| `bash scripts/check-projection-cli-boundary.sh` | pass |
| `git diff --check` | pass |

Hosted workflow: [CI run 35960597675](https://github.com/eggstack/eggplan/actions/runs/35960597675)
on final qualification revision `26da4449e5f54c64fa6266ce44a22b20df96f2b9`.
Linux job `107508117769`, macOS job `107508118010`, Windows job
`107508117969`, and Rust 1.89 job `107508118006` passed. Linux and macOS ran
formatting, workspace check, clippy, workspace tests, and all four boundary
checks. Windows ran workspace check, clippy, and tests; the workflow gates
formatting and shell boundary scripts off Windows. Rust 1.89 ran workspace
check and tests.

The first hosted run, `35960300175`, found that Windows `include_str!` retained
CRLF in the help snapshot. Commit `26da444` normalizes both fixture and output
line endings; the full native/MSRV matrix above passed on that correction.

## Invariants and residual findings

- Canonical Plan, Evidence, and ClosureRecord authority remains in core and
  repository APIs; projections are bounded DTOs.
- Provider-policy input contains authority metadata only and does not persist
  as a repository-global trust store.
- `check` is read-only by default. Recovery runs only with the explicit
  `--recover-pending` option.
- Registry rendering is derived and never writes `plans/registry.md`.
- JSON output is versioned and bounded. Hash/digest validity establishes
  integrity, not authenticity.
- Markdown import/render and concrete network-backed provider adapters remain
  outside M001.

## Roadmap and registry disposition

Projection/CLI M001 is closed. M002 Markdown import/render is ready for a
planning handoff: M001 now supplies the native projections and command
contracts it depended on, and no implementation blocker remains. M003 remains
a later roadmap item after M001/M002 see real repository use. No M002
implementation plan is registered by this closure.
