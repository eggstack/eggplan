# CodeGG Integration M001 Closure Record

Status: closed

Source plan: plans/implementation/codegg-integration/001-golden-parity-and-adapter-seam.md

Source roadmap: plans/subsystems/codegg-integration-roadmap.md

Reviewed CodeGG baseline: `28b4695661d463dd1675d045ac6299c5fbc9ea31`

Fixture manifest: `crates/eggplan-codegg-compat/tests/fixtures/manifest.json`

Implementation commit: `068b748c4bd5028a27319b9deb201bf2372b795e`

## Executive finding

The `eggplan-codegg-compat` crate defines a strict, bounded, one-way mapping
from versioned CodeGG WorkPlan snapshots into Eggplan domain state. It has no
CodeGG dependency, does not modify CodeGG, and leaves WorkOrder, Goal, Todo,
checkpoint/context epoch, runtime, executor, and worktree ownership in CodeGG.
Serialized completion and acceptance claims cannot create trusted evidence;
host resolution supplies the provider identity, exact subject, and required
verification binding. Eggplan's complete assessment remains available behind
the lossy five-family compatibility projection.

## Requirement-to-evidence matrix

| Requirement | Evidence |
|---|---|
| Reviewed baseline and representative versioned fixtures | `tests/fixtures/manifest.json` records CodeGG SHA `28b469...` and foundation, projection/arbiter, and trajectory source cases. |
| Deterministic bounded identity mapping and strict schema | `src/lib.rs`; `strict_versioned_fixture_corpus_and_stable_identity_mapping`; `dependency_mapping_and_input_bounds_are_explicit`. |
| Explicit exact, normalized, lossy, unsupported mapping | `architecture/codegg-compat.md`; `Loss` diagnostics and snapshot status mapping. |
| Evidence authority resolver; claims and owners cannot self-authorize | `EvidenceResolver`; tests `serialized_satisfied_owner_and_completed_labels_do_not_authorize_evidence`, `completed_acceptance_without_any_host_reference_remains_incomplete`, `stale_and_unbound_or_mismatched_execution_evidence_fail_closed`. |
| Status/actionability, dependencies, judgment, cancellation and empty completion | `tests/parity.rs`, covering all fixture item/plan states, dependency gating, judgment, unavailable Artifact, empty acceptance, and five completion families. |
| CAS, restart, and guarded closure contention | `snapshot_creation_uses_cas_and_restart_preserves_mapped_state`; `concurrent_cancel_and_guarded_close_have_one_cas_winner`. |
| Bounded current/actionable projection and reason code | `current_projection_is_bounded_stable_and_excludes_completed_history`. |
| No CodeGG-owned behavior and no production dependency | `scripts/check-codegg-compat-boundary.sh`; CI runs this guard on non-Windows hosts. Cargo dependency boundary reviewed. |

## Compatibility disposition

- Exact: bounded item status snapshots, acceptance descriptions, and supported
  graph references map without replaying source transitions.
- Normalized: typed identifiers receive deterministic namespaced Eggplan IDs;
  current source state is persisted through legal Eggplan revisions; evidence
  kinds and statuses pass through the explicit host resolver.
- Lossy: acceptance note and free-form evidence detail are omitted and
  diagnosed; item-scoped evidence cannot be assigned precisely to acceptance
  rows; completion families fold several Eggplan assessment outcomes into the
  existing actionable view.
- Unsupported as authority: `Completed` is not guarded Closed, `Satisfied` is
  not proof, and owner run/job IDs do not establish provider trust.

## Verification executed

Local Linux, on implementation commit `068b748c4bd5028a27319b9deb201bf2372b795e`:

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo test -p eggplan-codegg-compat --locked` | pass; 14 tests |
| `cargo clippy -p eggplan-codegg-compat --all-targets --offline -- -D warnings` | pass |
| `cargo check --workspace --all-targets --locked` | pass |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | pass |
| `cargo test --workspace --locked` | pass; 68 tests |
| `bash scripts/check-core-boundary.sh` | pass |
| `bash scripts/check-codegg-compat-boundary.sh` | pass |
| `git diff --check` | pass |
| `cargo +1.89.0 check --workspace --all-targets --locked` | pass |
| `cargo +1.89.0 test --workspace --locked` | pass |

Hosted workflow: [CI run 35955529814](https://github.com/eggstack/eggplan/actions/runs/35955529814)
on the implementation commit. Linux job `107492936422`, macOS job
`107492936496`, Rust 1.89 job `107492936464`, and Windows job
`107492936309` all passed. Linux ran formatting, check, clippy, tests, and both
boundary checks. macOS ran check, clippy, tests, and both boundary checks
(formatting is platform-skipped by workflow). Windows ran check, clippy, tests,
and both boundary checks (formatting is platform-skipped by workflow). Rust
1.89 ran check and tests.

## Invariant and residual review

- Eggplan core/repository have no CodeGG dependency; no CodeGG production
  change or runtime ownership moved.
- Evidence remains host-authoritative and exact-subject, with v2 verification
  binding for execution-derived requirements.
- Bounded fixture parsing, IDs, and projections are deterministic. Hash-derived
  IDs provide stable mapping, not identity authority.
- No unresolved implementation finding remains. The hosted workflow completed
  successfully on all four jobs.

## Roadmap and registry disposition

CodeGG Integration M001 is closed. Eggstack Provider SPI M001 is the next
registered plan and has been re-checked against the current sibling baselines;
it is advanced to active against repository baseline
`068b748c4bd5028a27319b9deb201bf2372b795e`. Projection/CLI M001 remains ready
after its baseline is refreshed when it becomes the next registered step.
