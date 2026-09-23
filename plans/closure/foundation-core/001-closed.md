# Foundation Core M001 Closure — Repository Bootstrap and Typed Domain Contract

Status: closed

Source plan: plans/implementation/foundation-core/001-repository-bootstrap-and-domain-contract.md

Source roadmap: plans/subsystems/foundation-core-roadmap.md

Reviewed baseline: 42ec41eff3e35c15acacaa0d717320c40543fbc1 (planning bootstrap)

Implementation commit: 08a9cdb (`feat(core): bootstrap typed planning domain`)

Closure commit: see Git history for this record and status transition.

## Executive finding

Foundation M001 is complete. The new Rust workspace contains the dependency-
light `eggplan-core` library. It provides schema-v1 bounded plan/domain types,
typed IDs, explicit state transition matrices, graph validation, pure readiness,
canonical JSON, and SHA-256 digests. Required local verification passed on the
current stable toolchain and Rust 1.89. No M001 acceptance criterion remains
open, and Foundation M002 can proceed.

## Requirement-to-evidence matrix

| Requirement | Evidence | Result |
|---|---|---|
| Workspace, core ownership and MSRV | `Cargo.toml`, `crates/eggplan-core/Cargo.toml`, `architecture/core.md` | Pass; Rust 1.89 declared |
| Typed, prefix-validated IDs | `identity.rs`, unit tests | Pass; parse, display, generation, bad-prefix, invalid-character, and bound cases |
| Explicit schema v1 and fail-closed unknown versions | `schema.rs`, Plan validation and roundtrip tests | Pass |
| Bounded fields and collections | `model.rs`, `lib.rs::bounds`, `architecture/core.md` | Pass; text counts Unicode scalar values and rejects empty/NUL/oversize values |
| Transition semantics and blocker discipline | `model.rs` unit tests | Pass; terminal states cannot be reopened; blocked items require blocker text |
| Parent/dependency graph validation | `graph.rs`, `model.rs` tests | Pass; missing/self/cyclic references rejected; deterministic readiness ordering tested |
| Canonical schema-v1 bytes and digest | `tests/fixtures/schema-v1-plan.json`, `.sha256`, `schema.rs` tests | Pass; compact serde JSON, declared struct field order, sorted map keys, omitted optional `None` values |
| Core responsibility boundary | `scripts/check-core-boundary.sh` | Pass; prohibited dependency families and public reasoning/transcript field names absent |
| Filesystem persistence and CAS | Explicitly excluded by M001 | Correctly deferred to M002 |

## Schema and bounds

Schema v1 defines Plan, PlanItem, AcceptanceCriterion, EvidenceRequirement,
ArtifactRef, SubjectRevision, provider-ID placeholder, and distinct typed IDs:
`ep_`, `epi_`, `epc_`, and `epp_`. IDs are limited to 96 characters.
Objectives/descriptions allow 4,000 Unicode scalar values; blockers, next
actions, provenance values, and criterion statements allow 2,000; requirement
descriptions allow 1,000; artifact references allow 2,000. Collection maxima
are 512 items, 64 dependencies per item, 128 criteria per item, 32 requirements
per criterion, and 32 provenance entries. Full bounds are documented in
`architecture/core.md` and exported as constants.

Canonical digest is lowercase `sha256:<hex>` over compact `serde_json` bytes.
Schema structs have fixed declared field order and `BTreeMap` provides sorted
keys; pretty-print output is outside the digest contract. The golden Plan
fixture freezes both exact bytes and digest.

## Exact verification executed

All commands ran from repository root after implementation, against the
implementation commit working tree. Results:

| Command | Result |
|---|---|
| `rtk cargo fmt --all -- --check` | Pass |
| `rtk cargo check --workspace --all-targets --locked` | Pass |
| `rtk cargo clippy --workspace --all-targets --locked -- -D warnings` | Pass |
| `rtk cargo test --workspace --locked` | Pass, 10 tests |
| `rtk cargo +1.89.0 check --workspace --all-targets --locked` | Pass |
| `rtk bash scripts/check-core-boundary.sh` | Pass |

The checks ran before the closure-only planning status/record changes. No
production code changed after those checks.

## Invariant and risk review

- No scheduler, executor, process, HTTP, MCP, database, or model SDK dependency
  entered `eggplan-core`.
- No hidden reasoning/transcript field is present in the persisted domain.
- Readiness is deterministic and does not grant execution authority.
- Evidence requirements remain distinct from observations; no evidence storage
  or closure assessment was added in M001.
- Plan validation rejects malformed text, oversize collections, unknown schema,
  dangling/self/cyclic graphs, and blocker-state inconsistencies.
- No migration was needed because there was no prior runtime state.
- Cross-platform filesystem, persistence, and Git behavior remain unqualified
  and are outside this milestone.

## Roadmap and registry disposition

Foundation M001 is closed. Foundation M002 is ready and its repository
baseline is `08a9cdb`. Evidence M001 remains blocked on Foundation M002 closure.
No later milestone is newly unblocked by M001 alone.

## Unresolved findings

None for M001. Cross-platform repository persistence qualification belongs to
M002 and is not claimed here.
