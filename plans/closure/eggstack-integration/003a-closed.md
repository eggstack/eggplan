# Eggstack Integrations M003a — Eggbench Verified Bundle and Comparison Evidence — Closed

Status: closed

Source implementation plan:

- plans/implementation/eggstack-integration/003a-eggbench-verified-bundle-and-comparison-evidence.md

Source roadmap:

- plans/subsystems/eggstack-integration-roadmap.md

Reviewed Eggplan baseline: `71904570e908a15c099f9b2804cabfbccf9ae51a`

Reviewed upstream interface:

- `eggstack/eggbench` @ `30a38251bccb5157beb68202ffe630f6253771e0`
  - ordinary hosted CI `37143714313` (success)
  - live external-tool qualification `37143714261` (success)

Implementation commit:

- `674264db7dda3f063cbbedd93f6f4938c1959a11` — bounded Eggbench verified-bundle
  and comparison-receipt adapter, compatibility fixtures, architecture
  documentation, and the rewritten integrations boundary guard.

Hosted qualification: GitHub Actions run
[37350383159](https://github.com/eggstack/eggplan/actions/runs/37350383159)
passed on the exact implementation commit.

- Linux job `111899458987`: format, check, clippy, tests, and all five boundary
  guards.
- macOS job `111899459403`: check, clippy, tests, and all five boundary guards;
  formatting is skipped by workflow policy on macOS.
- Windows job `111899459218`: check, clippy, tests, and all five boundary guards;
  formatting is skipped by workflow policy on Windows.
- Rust 1.89 job `111899459318`: workspace check and tests.

## Executive finding

Eggplan now normalizes host-verified Eggbench `.eggb` bundle identity and
standalone comparison receipts into Eggplan observations through two bounded,
Eggplan-owned DTOs. The adapter links no Eggbench crate, executes no `eggbench`
binary, walks no bundle directory, computes no manifest digest, and opens no
transport.

The five claims the plan required to stay distinct do stay distinct: artifact
integrity, execution outcome, comparison verdict, comparison
validity/incomparability, and subject/producer provenance. A finalized natively
verified bundle is an integrity claim that stays `Passed` even when the measured
run failed or was cancelled, and it can never become a passing benchmark claim.
A receipt reporting `pass` under a critical comparability mismatch normalizes to
`Inconclusive`, because upstream treats that comparison as descriptive-only.

## Upstream baseline gate (WP1)

The plan required re-checking `eggstack/eggbench` `main` and freezing the newest
exact revision with green ordinary CI **and** green live external-tool
qualification before any fixture was written.

Re-checked on 2026-10-05 against the live hosted API. `30a38251…` remains the
newest exact SHA with both workflow families green. Every revision above it on
`main` has both workflows failing or cancelled:

| Revision | CI | Live qualification |
|---|---|---|
| `1a5e0361…` (current head) | failure `37331059081` | failure `37331058745` |
| `e750931e…` | failure `37313248651` | failure `37313248643` |
| `f61df141…` | failure `37306547852` | failure `37306547799` |
| `751aaea…` | failure `37274523510` | failure `37274523492` |
| `944f9828…` | failure `37273303313` | failure `37273303267` |
| `13eb4434…` | failure `37268180613` | failure `37268180445` |
| `30a38251…` | **success `37143714313`** | **success `37143714261`** |

The gate is satisfied as planned. The plan's instruction not to silently move
fixtures to the current head was followed: `1a5e0361…` is recorded as research
evidence only.

The upstream contract was read from that exact SHA, never from the Eggbench
working tree, which was dirty and ten commits ahead. Two upstream facts shaped
the design:

- `eggbench inspect --json` exposes **no** manifest digest at the pinned
  revision. `manifest_sha256` is the SHA-256 of the exact on-disk
  `manifest.json` bytes and is only reachable through
  `compare --json`'s `receipt.candidate_identity`. The plan anticipated this and
  permits the host to compute it after native verification; that is the
  contract.
- `compare --json` reports `comparability_match` as the **inverse** of the
  receipt's `comparability.critical_mismatch`. The DTO carries the upstream
  field and the inversion is documented as the host's responsibility.

## Requirement-to-evidence matrix

| Plan requirement | Evidence |
|---|---|
| §1 preserve five distinct semantic classes | `artifact_integrity_never_becomes_benchmark_success` fixes the Artifact/Benchmark split across success, failure, cancellation, invalid execution, and legacy v1; `a_comparison_receipt_is_not_an_artifact_integrity_claim` rejects `Artifact` from `normalize_comparison`; `descriptive_comparison_cannot_become_passing_benchmark_evidence` covers incomparable evidence; subject/producer provenance is asserted in `an_eggbench_hint_is_never_translated_into_a_eggplan_subject` and `a_custom_artifact_role_label_is_bounded_not_dropped`. |
| §2 upstream baseline gate | Section "Upstream baseline gate" above; `fixtures_are_pinned_to_an_exact_dual_green_upstream_revision` asserts the SHA and both run IDs against every fixture row. |
| §3 ownership and dependency boundary | `eggplan-integrations/Cargo.toml` gains no dependency; the crate still depends only on `eggplan-core`, `serde`, `serde_json`, `sha2`, `thiserror`. `the_adapter_cannot_enroll_trust_or_acquire_evidence` greps the adapter source for process, filesystem, transport, registry, and URL tokens. Guard coverage is in the boundary section below. |
| §4 provider identity and allowed kinds | `descriptor()` is adapter-fixed at `epp_eggbench`. `provider_identity_is_fixed_and_trust_is_host_controlled` asserts core class `benchmark`, exactly `Artifact` + `Benchmark`, no `Attestation`, and that a fresh `ProviderRegistry` does not contain the provider. `unsupported_kinds_are_rejected_in_both_directions` rejects Test/Command/Research/Attestation/HumanJudgment/Revision. |
| §5 bounded verified-bundle DTO | `EggbenchBundleEvidenceV1` retains only the reviewed revision, manifest schema version, run id, manifest digest, finalization flag, execution status, embedded verdict, legacy status, bounded subject hints, counts, retained bytes, selected artifact handles, and driver provenance. `unknown_dto_fields_and_oversize_payloads_are_rejected` proves `deny_unknown_fields` and the 256 KiB cap. `a_non_finalized_bundle_is_not_admissible_evidence` rejects a non-finalized manifest. |
| §6 separate comparison DTO | `EggbenchComparisonEvidenceV1` is a distinct type, not an overload of the bundle DTO. `legacy_receipts_cannot_carry_performance_or_correctness_sections` and `unknown_dto_and_upstream_versions_are_rejected` enforce the v1–v4 matrix and fail closed outside it. |
| §7 status normalization | `comparison_verdicts_are_normalized_without_collapsing_axes` walks all ten receipt fixtures; `performance_and_correctness_axes_are_retained_separately` keeps a failing performance gate with a clean correctness gate readable as two axes and one combined `Failed`; `invalid_and_inconclusive_never_become_failed_or_passed`; `legacy_inconclusive_status_is_never_disambiguated_by_inference` retains `legacy_status_ambiguous` alongside the normalized `completed`; `a_comparison_receipt_is_not_an_artifact_integrity_claim`; `descriptive_comparison_cannot_become_passing_benchmark_evidence`. |
| §8 subject and verification binding | `subject_revision_disagreement_is_rejected_for_git_subjects` rejects a mismatched Git revision on both DTOs; `an_eggbench_hint_is_never_translated_into_a_eggplan_subject` proves a non-Git subject is not compared and the hint stays metadata; `benchmark_evidence_requires_host_verification_binding` proves the digest is required, is the host's, and is never re-derived. |
| §9 artifact references | `artifact_refs_are_run_scoped_handles_with_exact_digests` and `comparison_refs_use_the_bounded_receipt_digest_prefix` fix the three handle forms, exact digests, the 16-hex comparison prefix, and the no-digest no-reference case; both assert no `://` and no `/tmp` survives as identity. |
| §10 compatibility fixtures | `tests/fixtures/eggbench-bundles.json` (7 cases) and `tests/fixtures/eggbench-comparisons.json` (10 cases) cover all ten plan categories: finalized no-verdict, failed execution, cancelled, invalid, legacy v1, embedded verdict, custom role, pass, fail, inconclusive, invalid, descriptive-only, no-verdict, v4 with performance + correctness, v3, v2, and v1. `tests/fixtures/manifest.json` records per-family upstream SHA, both run IDs, and supported schema ranges. Fixtures are minimal serialized DTOs; no native runtime source and no bundle payload is copied. |
| §11 negative/security tests | Every plan-listed negative test is present: unknown DTO schema, unknown manifest/receipt version, malformed SHA-256, candidate/baseline identity mismatch, duplicate artifact handles, subject revision disagreement, missing Benchmark verification binding, artifact integrity cannot become Benchmark Passed, descriptive cannot become Benchmark Passed, Invalid cannot become Failed or Passed, legacy inconclusive not disambiguated, prose absent from serialized observations, and adapter cannot enroll its own provider. Additionally: escaping artifact paths, duplicate drivers, duplicate warning categories, non-snake_case warning categories, selection/count bounds, non-finalized bundles, oversize payloads, and a source-level no-enrollment/no-acquisition proof. |
| §12 documentation | `architecture/eggbench-adapter.md` documents the exact reviewed and fixture baselines, the host/native-verification boundary, DTO schemas and bounds, the manifest/receipt compatibility matrix, Artifact vs Benchmark semantics, the status mapping, subject/verification ownership, and the explicit attestation exclusion. |
| §14 required verification | Section below; all commands run locally and hosted. |
| §15 acceptance criteria | All ten criteria are covered by the rows above; criterion 10 is the hosted run. |

## Compatibility matrix recorded

| Upstream artifact | Versions | Behavior |
|---|---|---|
| `.eggb` bundle manifest | v2 | Supported; `finalized` must be true. |
| `.eggb` bundle manifest | v1 | Supported only with an explicit `legacy_status`; never reinterpreted. |
| `.eggb` bundle manifest | other | Rejected. |
| Comparison receipt | v4, v3 | Supported; performance, correctness, aggregate retained separately. |
| Comparison receipt | v2, v1 | Supported; performance/correctness must be absent. |
| Comparison receipt | other | Rejected. |
| Eggplan DTO | v1 | Supported. |
| Eggplan DTO | other | Rejected. |

Unknown fields and unknown enum variants fail closed at every level.

## Status-mapping matrix recorded

| Input | Benchmark status |
|---|---|
| execution status absent | `Inconclusive` |
| execution `failed` | `Failed` |
| execution `cancelled` | `Skipped` |
| execution `invalid` | `Inconclusive` |
| critical comparability mismatch | `Inconclusive` |
| aggregate `pass` | `Passed` |
| aggregate `fail` | `Failed` |
| aggregate `inconclusive` / `invalid` | `Inconclusive` |
| no aggregate verdict | `Inconclusive` |
| manifest v1 (any legacy status) | `Inconclusive` |
| any finalized verified bundle as `Artifact` | `Passed` (integrity only) |

## Boundary guard evidence

`scripts/check-integrations-boundary.sh` was rewritten from five ad-hoc `grep`
invocations into six scope-labelled guards that scan the whole
`crates/eggplan-integrations/src` tree rather than a hand-listed set of adapter
files. This closes a finding recorded in `architecture/deep-dive-integrations.md`
section 6: the previous trust guard scanned only `src/eggwork.rs` and
`src/eggsearch.rs`, so a registry mutation added to `src/lib.rs` or to any
future module would have passed. Because M003a is the first adapter added after
that finding was written, the gap had become immediately reachable.

Coverage added beyond the previous guard: filesystem access (`std::fs`,
`std::path`, `tempfile::`, the `use`-imported `fs::` call forms, `File::`
constructors), env-var reads, executable discovery, and credential/secret-store
crates. The guard now also requires `rg` and `awk` to be installed and fails
loudly otherwise, so it cannot silently no-op as `rg`-based guards have in this
repository before. Every failure message states the exact scope it scanned.

Verification of the guard itself:

- 40 synthetic self-proofs run the same guard functions against fixtures in a
  temporary directory and never read or write tracked source.
- A real-tree negative proof injected `fn probe(r: &mut ProviderRegistry)` into
  `src/eggbench.rs`; the guard failed with exit 1 and the expected message, and
  passed again after restore. This is a one-off manual proof, not a checked-in
  test; the standing coverage is the self-proof plus the adapter source test
  `the_adapter_cannot_enroll_trust_or_acquire_evidence`.

## Exact local verification

All commands ran on Linux at implementation commit
`674264db7dda3f063cbbedd93f6f4938c1959a11` and passed:

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked                      # 187 passed
cargo +1.89.0 check --workspace --all-targets --locked
cargo +1.89.0 test --workspace --locked              # 187 passed
bash scripts/check-core-boundary.sh
bash scripts/check-codegg-compat-boundary.sh
bash scripts/check-integrations-boundary.sh
bash scripts/check-projection-cli-boundary.sh
bash scripts/check-closure-authority-boundary.sh
git diff --check
```

The Eggbench adapter contributes 31 conformance tests in
`crates/eggplan-integrations/tests/eggbench_conformance.rs`.

## Invariant review

- Eggplan remains a planning/evidence mechanism. Eggbench execution, bundle
  writing, and integrity verification remain host-owned; the adapter normalizes
  facts only.
- Artifact integrity and benchmark success are separate observations with
  separate kinds. Neither can be promoted into the other.
- A comparison receipt cannot satisfy an artifact-integrity criterion at all.
- Invalid and incomparable evidence never becomes `Passed` or `Failed`.
- The legacy v1 overloaded `inconclusive` status stays ambiguous forever; Eggplan
  never guesses which meaning applied.
- Execution status is distinct from comparison verdict upstream, and stays
  distinct here.
- Benchmark observations remain verification-bound through the existing core
  evidence schema v2 contract; no adapter derives a verification digest.
- Eggbench subject hints are provenance. They are never translated into an
  Eggplan repository identity, and a disagreement with a Git subject is a hard
  rejection rather than stale-but-passing evidence.
- Provider identity is adapter-fixed; trust enrollment remains an explicit host
  registry action that no adapter can perform.
- Attestation verification remains deferred to Interoperability M001. The
  adapter emits no `Attestation` kind.
- Large native evidence stays outside Eggplan control records: references are
  bounded opaque handles with exact digests, and no artifact body, manifest JSON,
  stdout/stderr, warning prose, per-check correctness record, argv, or
  environment reference is retained.

## Migration and compatibility review

No schema migration was needed. Plan `SCHEMA_VERSION` and
`EVIDENCE_SCHEMA_VERSION` remain 2; no envelope changed. This milestone adds one
provider module to an existing crate and no new dependency.

Existing M001/M002 fixture provenance is unchanged. `manifest.json` keeps the
M001 synthetic Eggbench source `d7d1fd9…` and gains a separate
`native_fixture_families` array, so the stale-pin finding recorded in the
deep-dive remains a factual historical observation rather than a silent edit,
while the M003a families carry the closure-grade dual-green baseline.

## Security, trust, and path review

- No credential, token, endpoint, or secret-bearing value can be persisted: the
  generic SPI denylist applies, and the adapter additionally rejects `://`,
  control characters, and NUL in every retained label and hint.
- Artifact paths must be confined bundle-relative paths. Absolute paths,
  backslashes, and empty/`.`/`..` segments are rejected, so no path traversal
  reaches an artifact reference.
- Durable identity is a run-scoped handle, never a temporary host path.
- Warning `detail` prose and per-check correctness records are structurally
  excluded: the DTOs have `deny_unknown_fields` and do not carry those fields.
- The upstream `.eggb` root and artifact paths are never opened by Eggplan.
- A host that reports native integrity failure must not submit a DTO; a
  non-finalized manifest is rejected as input rather than normalized.

## Documentation and operations

- `architecture/eggbench-adapter.md` — new, the complete adapter contract.
- `architecture/deep-dive-integrations.md` — section 7 appended with the M003a
  delta; sections 1–6 left intact so the earlier review remains evidence of
  what was accepted then.
- `architecture/provider-spi.md` — fixture-family provenance and the widened
  guard description.
- `architecture/overview.md` — crate role row now lists the Eggbench adapter.

## Roadmap disposition

Eggstack M003a is closed. The Eggstack integration roadmap's M003 line is now
half-complete: M003b remains planned and ready and was independent of the
Eggbench fixture gate, so nothing in M003a blocked or serialized it.

## Registry updates

- M003a row set to `closed` with this closure record.
- Subsystem status line notes M003a closed and M003b ready.
- External interface research baseline for Eggbench now records the frozen
  dual-green SHA as the implemented baseline, alongside the researched current
  head.

## Unresolved findings

1. **Low — the plan's suggested provider class string is not implementable.**
   §4 recommended the free-form class `eggbench-verified-evidence-v1`. The
   frozen SPI derives the core class string from its closed `ProviderClass`
   vocabulary, so that exact string cannot be produced without widening a core
   contract. `Benchmark` was used, yielding core class `benchmark`. Recorded as
   a documented deviation rather than a silent substitution; widening
   `ProviderClass` would be a separate core decision.
2. **Low — `eggbench inspect --json` still exposes no manifest digest.** The
   host must compute `manifest_sha256` from the exact finalized manifest bytes
   after native verification. This is documented and is not a hard dependency,
   but it means the adapter trusts the host's digest computation rather than
   re-deriving it. Eggplan cannot verify it without walking the bundle, which
   the ownership boundary forbids.
3. **Informational — `check-integrations-boundary.sh` self-proofs are shell, not
   Rust.** The 40 self-proofs run inside the script on every CI run, so they are
   gated, but they are not unit tests and a failure surfaces as a script error
   rather than a test name.
4. **Informational — the deep-dive finding that `native_fixtures` in
   `manifest.json` is not exhaustive still stands.** It names three M001/M002
   files and still omits `verification-digest.json` and the two Eggbench files.
   The Eggbench provenance is authoritative through `native_fixture_families`,
   so nothing in M003a depends on the incomplete list. Left for a future
   maintenance pass rather than fixed here, to keep this milestone's diff scoped.

## Residual findings for M003b and Interoperability M001

- M003b (GitHub forge) is unaffected by this milestone and remains ready.
- Interoperability M001 will need the bounded artifact provenance preserved
  here — run-scoped handles with exact SHA-256 digests and producer identity —
  as input to future attestation verification. M003a deliberately performs no
  Sigstore, in-toto, or SLSA work.
- The upstream Eggbench baseline will need rechecking whenever fixtures move. A
  newer SHA requires the same dual-green ordinary and live qualification
  evidence before it can become a fixture pin.