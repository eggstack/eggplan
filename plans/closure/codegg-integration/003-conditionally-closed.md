# CodeGG Integration M003 Closure Record

Status: conditionally closed

Source implementation plan:
`plans/implementation/codegg-integration/003-repository-plan-binding-contract.md`

Source roadmap: `plans/subsystems/codegg-integration-roadmap.md`

Implementation revision: `3f7c603315131bb169bfdd2bb575531d228532b1`

Hosted qualification: Eggplan CI run
[`36868055136`](https://github.com/eggstack/eggplan/actions/runs/36868055136)
on the exact implementation revision; Ubuntu, macOS, Windows, and Rust 1.89
jobs all succeeded.

Coordinated CodeGG plan:
`dbowm91/codegg:plans/implementation/eggplan-assessment-integration/004-repository-plan-binding-and-writeback.md`
registered at CodeGG `440403e82304537f6ed9de22103987b16df6f642`, status `ready`.

## Executive finding

Eggplan's pure, bounded reverse-projection contract is implemented and
qualified. The cross-repository consumer has not yet been implemented, so
acceptance criteria 8 and 9 remain outstanding and this closure is conditional.
CodeGG M003 is dependency-ready against the exact qualified Eggplan revision;
its binding persistence, identity proof, writeback, reconciliation, and
guarded-close consumption remain future work in CodeGG's registered plan.

## Requirement-to-evidence matrix

| Requirement | Evidence and disposition |
|---|---|
| 1. Bindable repository Plan has deterministic bounded projection | `project_repository_plan` returns strict schema-v1 `RepositoryPlanProjectionV1` after core validation; focused projection tests and workspace suite pass. |
| 2. Intent and full projection digests preserve required semantics | `RepositoryIntent` covers objective, ordered item IDs/positions/relationships/descriptions, criteria and complete requirements. Tests prove lifecycle/blocker/next-action changes leave intent stable while projection digest changes; objective, criterion, and dependency changes alter intent. |
| 3. Projection fabricates no evidence, trust, or closure authority | DTO contains no observations, provider registry, candidate, or closure record. Projection test exercises structured requirements and source identities; projection code accepts only the Plan. |
| 4. Lifecycle mapping is exact | Plan Active/Blocked are preserved; Draft/Closed/Cancelled reject. Item status is copied exactly. Covered by `only_active_and_blocked_plans_are_bindable` and projection preservation tests. |
| 5. Identity and purity boundaries hold | DTO retains Eggplan IDs; no repository-ID rewrite helper exists. Production dependency graph is `eggplan-core`, `serde`, and `serde_json`; compatibility boundary script passes. |
| 6. Reverse-projection schema and manifest are versioned | `REPOSITORY_PROJECTION_SCHEMA_VERSION == 1`; strict `deny_unknown_fields` DTOs and `RepositoryPlanBindingManifestV1` are public. |
| 7. M001/M002 APIs and fixtures remain compatible | Existing 19 parity tests pass unchanged alongside 4 new projection tests. |
| 8. CodeGG consumes one exact immutable Eggplan revision | Outstanding. CodeGG has registered its M003 plan against the qualified contract; implementation has not started. |
| 9. Cross-repository binding/writeback/closure tests | Outstanding and owned by the CodeGG M003 plan. No cross-store atomicity is claimed. |
| 10. Native/MSRV hosted qualification | Satisfied for the Eggplan contract: hosted run `36868055136`, exact SHA `3f7c603`; all four jobs succeeded. CodeGG qualification belongs to its M003 closure. |

## Production implementation evidence

- `crates/eggplan-codegg-compat/src/lib.rs`: schema-v1 projection DTOs,
  binding manifest, bindable lifecycle check, structural intent digest,
  complete projection digest, and pure `project_repository_plan` function.
- `crates/eggplan-codegg-compat/tests/repository_projection.rs`: 4 tests
  covering rich requirement preservation, typed source identities, lifecycle
  rejection, determinism/strict serialization, digest stability/sensitivity.
- `scripts/check-codegg-compat-boundary.sh`: rejects persistence, async,
  network-client, process, and database access in the compatibility crate.
- `architecture/codegg-compat.md` and
  `architecture/deep-dive-codegg-compat.md`: document M002 snapshot assessment
  and M003 reverse projection as separate directions.

## Verification

All commands were run from Eggplan at the implementation revision:

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo check --workspace --all-targets --locked` | pass |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | pass |
| `cargo test --workspace --locked` | pass; 134 tests |
| `cargo +1.89.0 check --workspace --all-targets --locked` | pass |
| `cargo +1.89.0 test --workspace --locked` | pass |
| `bash scripts/check-codegg-compat-boundary.sh` | pass |
| `bash scripts/check-core-boundary.sh` | pass |
| `bash scripts/check-closure-authority-boundary.sh` | pass |
| `git diff --check` | pass |
| Hosted CI `36868055136` | pass: Ubuntu, macOS, Windows, Rust 1.89 |

The hosted workflow intentionally skips boundary scripts on Windows and skips
formatting on non-Ubuntu platforms; Ubuntu ran formatting and all five boundary
scripts. The requested CodeGG consuming tests were not run because CodeGG M003
production implementation remains unstarted.

## Invariant, recovery, and compatibility review

The compatibility crate remains pure and does not acquire or persist evidence.
Projection never upgrades an item to Completed, rewrites repository identity,
selects a provider, or closes a Plan. CodeGG must independently prove subject
equivalence and use repository CAS, append-only evidence, assessment, and
guarded closure APIs under its own plan. Cross-store atomicity is explicitly
outside this contract.

## Unresolved condition

| Condition | Severity | Owner/disposition |
|---|---|---|
| CodeGG must consume this exact immutable revision and complete binding, writeback, reconciliation, and guarded-close integration tests plus hosted qualification | required downstream capability | CodeGG M003 is registered `ready` at `plans/implementation/eggplan-assessment-integration/004-repository-plan-binding-and-writeback.md`; keep the CodeGG integration roadmap active until its closure. |
| Commit/Revision evidence authority from CodeGG artifacts | deferred | Explicitly deferred by the source plan; Artifact evidence still requires durable digest, producing run, and exact subject in the CodeGG consumer. |

## Roadmap and registry disposition

- Eggplan implementation plan M003: `conditionally closed` with the named
  CodeGG consumer/qualification condition above.
- CodeGG integration subsystem roadmap remains `active` while CodeGG M003 is
  ready for implementation.
- No unrelated plan is blocked by the outstanding consumer work. CodeGG M003
  can start against this exact pushed and hosted-qualified Eggplan contract.
