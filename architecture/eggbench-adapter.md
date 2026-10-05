# Eggbench verified-bundle and comparison evidence adapter

`eggplan_integrations::eggbench` normalizes host-verified Eggbench bundle and
comparison facts into Eggplan observations. It depends on no Eggbench crate,
executes no `eggbench` binary, walks no `.eggb` directory, and opens no
transport. The host owns acquisition and native verification.

## Reviewed upstream baseline

| Fact | Value |
|---|---|
| Upstream repository | `eggstack/eggbench` |
| Reviewed and fixture-frozen revision | `30a38251bccb5157beb68202ffe630f6253771e0` |
| Ordinary hosted CI run on that exact SHA | `37143714313` (success) |
| Live external-tool qualification run on that exact SHA | `37143714261` (success) |

This is the newest exact revision found with **both** workflow families green.
Revisions above it on `main` had both workflows fail or be cancelled and are
research evidence, not a closure-grade pin. Re-checking upstream is an
implementation-agent duty; moving fixtures to a newer SHA requires the same
dual-green evidence.

`crates/eggplan-integrations/tests/fixtures/manifest.json` records this
baseline per fixture family alongside the older M001 synthetic Eggbench source.

## Host and native-verification boundary

The preferred host path is:

```
eggbench inspect --json / compare --json
          |
          v
host validates the exact upstream envelope and performs native integrity checks
          |
          v
bounded Eggplan DTO (EggbenchBundleEvidenceV1 / EggbenchComparisonEvidenceV1)
          |
          v
EvidenceObservation
```

`eggbench inspect --json` does **not** expose a manifest digest at the pinned
revision. `manifest_sha256` is the SHA-256 of the exact on-disk
`manifest.json` bytes, so the verifying host computes it after native
verification and supplies it in the DTO. Only `compare --json` exposes a bundle
digest, and only inside `result.receipt.candidate_identity`.

A bundle whose manifest is not `finalized` is rejected as input, not normalized.
A host that reports native integrity failure must not submit a DTO at all; there
is no "untrusted but recorded" path for a corrupted digest tree.

## Compatibility matrix

| Upstream artifact | Upstream versions | Adapter behavior |
|---|---|---|
| `.eggb` bundle manifest | v2 | Supported. Requires an explicit `finalized` manifest. |
| `.eggb` bundle manifest | v1 | Supported only with an explicit `legacy_status`; never reinterpreted. |
| `.eggb` bundle manifest | any other | Rejected. |
| Comparison receipt | v4 | Supported; performance, correctness, and aggregate verdicts retained separately. |
| Comparison receipt | v3 | Supported; same three axes. |
| Comparison receipt | v2, v1 | Supported; `performance_verdict`/`correctness` must be absent, because a legacy aggregate is metric-only. |
| Comparison receipt | any other | Rejected. |
| Eggplan DTO | v1 | Supported. |
| Eggplan DTO | any other | Rejected. |

Unknown fields, unknown enum variants, and unknown schema versions fail closed
on both the DTO and every upstream version. Upstream's own compatibility readers
keep v1–v3 receipts readable; Eggplan never widens past the versions above.

### Deviation from the plan's suggested provider class

The plan proposed the free-form provider class string
`eggbench-verified-evidence-v1`. The frozen SPI expresses provider class through
its closed `ProviderClass` vocabulary (`Execution`, `Research`, `Benchmark`,
`Utility`, `Other`), from which `AdapterDescriptor::provider_descriptor`
derives the core class string. `Benchmark` is used, yielding core class
`benchmark`. Choosing a bespoke class string would require widening a frozen
core/SPI contract, which is outside this milestone.

## Bounded DTOs

Neither DTO retains a native payload. Retained bundle facts are the reviewed
revision, manifest schema version, run id, exact manifest digest, finalization
flag, execution status, embedded comparison verdict, legacy v1 status, bounded
subject hints, artifact and driver counts, retained byte total, and a bounded
selection of artifact handles with digests. Retained receipt facts are the
receipt schema version, policy id, producing Eggbench version, candidate and
optional baseline identities, candidate execution status, aggregate/performance
verdicts, correctness summary, comparability axes, warning **categories**, and
the receipt digest.

Explicitly not retained: complete manifest JSON, trial stdout/stderr, arbitrary
diagnostic prose, warning `detail` strings, environment dumps, secret-bearing
route configuration, metric arrays and histograms, per-check correctness records,
argv, environment references, and filesystem paths outside the bundle.

Bounds: 256 KiB per DTO, 16 selected artifact handles, 32 driver records, 16
warning categories, 128-character run ids, 96-character labels and hints, 255
character media types, 512-character artifact paths, 64-character lowercase hex
digests. Artifact paths must be confined relative paths — no absolute path, no
backslash, no empty, `.` or `..` segment.

## Artifact integrity is not benchmark success

These are two different claims and the adapter keeps them apart.

`EvidenceKind::Artifact` records that a finalized bundle exists and that its
digest tree verified natively. It normalizes to `Passed` regardless of whether
the measured run failed, was cancelled, or produced no verdict. It says nothing
about benchmark success.

`EvidenceKind::Benchmark` records a comparison verdict. It is rejected from
`normalize_comparison` and accepted only from `normalize_bundle` when the bundle
carries an upstream comparison verdict, and it always requires the host's
`VerificationDigest` through the existing core evidence schema v2 contract.

An integrity claim can never be promoted into a verdict claim: a finalized
bundle with no comparison verdict normalizes to `Inconclusive`, never `Passed`.
A comparison receipt is never an artifact-integrity claim at all.

## Status mapping

Execution status is applied first, then comparison:

| Upstream fact | Benchmark status |
|---|---|
| execution status absent | `Inconclusive` |
| execution `failed` | `Failed` |
| execution `cancelled` | `Skipped` |
| execution `invalid` | `Inconclusive` |
| critical comparability mismatch | `Inconclusive` |
| aggregate `pass` | `Passed` |
| aggregate `fail` | `Failed` |
| aggregate `inconclusive` | `Inconclusive` |
| aggregate `invalid` | `Inconclusive` |
| no aggregate verdict | `Inconclusive` |
| manifest v1 with any legacy status | `Inconclusive` |

`Invalid` never becomes `Failed` or `Passed` in either direction.

A critical comparability mismatch makes the comparison descriptive-only
upstream. Descriptive evidence does not gate, so Eggplan refuses to let it
satisfy a benchmark criterion even when the receipt reports `pass`. This is the
one place where the receipt's own aggregate is deliberately not taken at face
value, and it is documented rather than silent.

The legacy v1 `inconclusive` status is inherently ambiguous between "no
comparison" and "inconclusive comparison". Upstream normalizes it to
`execution_status: completed` while exposing the raw value separately. Eggplan
retains the normalized status as a provenance fact but never infers which of the
two meanings applied; a legacy bundle is always `Inconclusive` as a benchmark
claim.

## Subject and verification ownership

Eggbench subject fields are provenance hints. The host supplies
`ObservationContext.subject`; the adapter never constructs an Eggplan
`SubjectRevision` and never translates an Eggbench label or opaque digest into an
Eggplan repository identity.

When the DTO declares a subject revision and the Eggplan context is a Git
subject, the revisions must be identical or the observation is rejected — a
disagreement is an adapter error, not stale-but-passing evidence. When the
Eggplan subject is not Git, the hint is retained as bounded metadata and not
compared.

The `VerificationDigest` is never derived from a run id, bundle handle, manifest
digest, policy id, metric name, or prose. For `Benchmark` it is required and
comes only from the host. An artifact digest is never usable as a verification
digest.

`assert_same_bundle` rejects a comparison whose candidate identity does not
describe the bundle the host separately verified, so a receipt and a bundle
cannot drift into describing different runs.

## Artifact references

References are durable, bounded, run-scoped handles. No temporary host path is
retained as identity.

| Form | Digest |
|---|---|
| `eggbench:bundle:<run-id>` | `sha256:<manifest digest>` |
| `eggbench:bundle:<run-id>#<bundle-relative-path>` | `sha256:<artifact digest>` |
| `eggbench:comparison:<candidate-run-id>:<16-hex receipt digest prefix>` | `sha256:<receipt digest>` |

The comparison reference is emitted only when a standalone receipt digest
exists; with no digest there is nothing durable to reference.

## Explicit attestation exclusion

This adapter never emits `EvidenceKind::Attestation`. It does not treat the
existence of an attestation record as verified authenticity, does not implement
Sigstore or in-toto verification, and does not equate provenance with criterion
satisfaction. Artifact provenance captured here is the input that Interoperability
and Distribution M001 will later need; the verification itself is not done here.

The provider descriptor allows exactly `Artifact` and `Benchmark`. A regression
asserts that `Attestation` is absent from the descriptor's allowed kinds.

## Provider identity and trust

Provider identity is adapter-fixed at `epp_eggbench`. Payload text cannot select
a different provider or widen the allowed kinds. `descriptor()` returns an
`AdapterDescriptor` that a host **may** pass to its own
`ProviderRegistry::register_trusted`; the adapter never enrolls it and never
constructs a registry. `scripts/check-integrations-boundary.sh` enforces this
across the whole crate source tree, so a future module cannot escape by not
being named.

Capabilities declare `supports_in_progress: false` because every fact this
adapter accepts is terminal, `artifacts: true`, `verification_binding: true`, and
`research_trust_metadata: false` because a measured bundle carries no external
content provenance claim.

Run `scripts/check-integrations-boundary.sh` to verify there is no sibling,
transport, credential, process, filesystem, network, or trust-enrolment
capability in the crate.