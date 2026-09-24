# Eggstack Integrations M001 Closure Record

Status: closed

Source plan: plans/implementation/eggstack-integration/001-evidence-provider-spi.md

Source roadmap: plans/subsystems/eggstack-integration-roadmap.md

Reviewed sibling baselines (2026-09-24):

- Eggwork: `128f808c62f176d414dd18a705773e45f5e2891a`
- Eggsearch: `dfa90e050c5434f3346902aeb4074901c58e90d1`
- Eggbench: `d7d1fd9a9b67a5b2ca6a816c841d2588a368aae9`
- Eggsact: `40959b704431430668e9ca2bfe959a8ef32495d8`

Implementation commits:

- `24682f74040aa0fd8a6c2ff6311098c964cc6358` — provider normalization SPI,
  digest helper, fixtures, docs, and CI guard.
- `b9afd94f4bb4c7b44953e3895c59fc4afeb1d3b4` — portable static boundary
  guard for hosted runners without ripgrep.
- `a5fed2269518e5cf8591d9ec7b082d01f1a2c041` — exercise all synthetic source
  cases through observation finalization.

## Executive finding

`eggplan-integrations` now provides a small synchronous normalization
contract over eggplan-core. It does not acquire evidence or own transport,
authentication, execution, scheduling, artifact storage, or provider trust.
Provider identity comes from a strict descriptor; the host separately decides
whether to register its core descriptor as trusted. Execution-derived evidence
requires a verification digest. Research trust markers, truthful statuses,
bounded artifacts, and safe metadata rules survive finalization into immutable
v2 observations.

## Requirement-to-evidence matrix

| Requirement | Evidence |
|---|---|
| Strict bounded adapter descriptor, kind set, capabilities, and host-controlled trust enrollment | `AdapterDescriptor`; `descriptor_is_strict_bounded_and_does_not_self_register`; `passing_synthetic_execution_is_usable_only_after_host_trust_registration`. Duplicate and unsupported kinds, unknown fields, and empty sets are rejected. |
| Immutable subject/context and normalized result with no caller-selected provider identity | `ObservationContext`, `NormalizedProviderResult`, `finalize_observation`; finalization assertions for provider ID, kind, subject, and digest. |
| Provider-namespaced canonical verification specification | `verification_digest`; golden vector `tests/fixtures/verification-digest.json`; canonical-order, namespace, schema, payload, nesting, and bounds tests. |
| v2 execution binding and declared capabilities | `finalization_uses_descriptor_authority_and_requires_execution_binding`; `capability_and_artifact_bounds_are_enforced`. |
| Truthful and distinct status, content digest sensitivity | `status_normalization_preserves_native_outcomes_and_content_digest`; all eight EvidenceStatus values survive unchanged. |
| Eggsearch external-untrusted trust and gaps cannot pass from text | `external_untrusted_research_cannot_become_passed_by_text`; explicit trust metadata is preserved, untrusted Passed is rejected, and generic prose does not change status. |
| Eggbench no-comparison remains distinct from verdict | `benchmark_without_comparison_is_not_comparison_pass_or_fail`. |
| Eggwork/Eggsearch/Eggbench-shaped synthetic corpus | `tests/fixtures/manifest.json` records reviewed SHAs; `synthetic-results.json` contains 11 cases, each finalized through the SPI by `synthetic_sibling_contract_corpus_has_reviewed_source_metadata`. |
| Bounded artifacts/metadata and secret/endpoint exclusion | `metadata_and_artifact_records_reject_secrets_endpoints_and_oversize`; core finalizer rechecks observation bounds. |
| No acquisition/runtime/sibling dependency | `scripts/check-integrations-boundary.sh`, run in Linux/macOS CI; crate manifest has only eggplan-core, serde, serde_json, sha2, and thiserror. |
| Core has no async/network additions | `scripts/check-core-boundary.sh`; no core changes were made for M001. |

## Public API inventory

- `ProviderClass`, `Capabilities`, and `AdapterDescriptor`
- `ObservationContext`, `NormalizedProviderResult`, and `SourceTrust`
- `SpiError`
- `verification_digest`
- `finalize_observation`

The crate is synchronous. Provider-specific Eggwork, Eggsearch, Eggbench, and
Eggsact clients/mappings remain future host adapter work.

## Verification executed

Local Linux on final implementation commit `a5fed2269518e5cf8591d9ec7b082d01f1a2c041`:

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo test -p eggplan-integrations --locked` | pass; 12 conformance tests |
| `cargo check --workspace --all-targets --locked` | pass |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | pass |
| `cargo test --workspace --locked` | pass; 80 tests |
| `bash scripts/check-core-boundary.sh` | pass |
| `bash scripts/check-codegg-compat-boundary.sh` | pass |
| `bash scripts/check-integrations-boundary.sh` | pass |
| `git diff --check` | pass |
| `cargo +1.89.0 check --workspace --all-targets --locked` | pass |
| `cargo +1.89.0 test --workspace --locked` | pass; 80 tests |

Hosted workflow: [CI run 35957085143](https://github.com/eggstack/eggplan/actions/runs/35957085143)
on final implementation commit. Linux job `107497582363`, macOS job
`107497582477`, Windows job `107497582294`, and Rust 1.89 job
`107497582099` all passed. Linux and macOS ran formatting, workspace check,
clippy, workspace tests, and all three boundary checks. Windows ran workspace
check, clippy, and tests; formatting and shell boundary checks are skipped on
that runner by workflow. The Rust 1.89 job ran workspace check and tests.

The earlier workflow on `24682f7` failed only because `rg` was absent from
hosted Linux/macOS images. That run is not qualification evidence; the guard
was made portable and the successful run above verifies the correction.

## Invariant and residual review

- No transport, credential, executor, scheduler, async runtime, artifact store,
  or live sibling dependency was introduced.
- Descriptor construction does not grant trust. The host's explicit
  `ProviderRegistry` remains the authority.
- Evidence digests provide integrity, not authentication.
- Generic metadata filters common credential/endpoint forms; provider adapters
  must select fields deliberately and safe-list native data before normalization.
- No unresolved M001 finding remains. No real provider adapter is claimed.

## Compatibility, recovery, and documentation

M001 introduces no stored schema or migration. It finalizes observations
through the existing core v2 contract, which revalidates subject, binding,
metadata, and artifacts. There is no new mutable state requiring recovery.
`architecture/provider-spi.md` documents trust, status, bounds, and fixture
provenance. README/operator integration guidance is deferred until concrete
provider adapters exist.

## Roadmap and registry disposition

Eggstack M001 is closed. M002 (Eggwork and Eggsearch adapters) has its hard
dependency satisfied and is ready for planning/handoff; sibling interfaces
must be rechecked when that work starts. M003 (Eggbench plus CI/forge) remains
blocked on M002. Projection/CLI M001's Evidence M002 dependency is already
closed; it remains ready and can start after its repository baseline is
refreshed to this closure commit.
