# Eggstack Integrations M003a — Eggbench Verified Bundle and Comparison Evidence

Status: ready

Repository baseline: 71904570e908a15c099f9b2804cabfbccf9ae51a

Source roadmap:

- plans/subsystems/eggstack-integration-roadmap.md
- plans/002-long-term-roadmap.md Eggstack integrations M003

Predecessors:

- plans/closure/eggstack-integration/001-closed.md
- plans/closure/eggstack-integration/002-closed.md

Reviewed Eggbench state on 2026-10-05:

- current reviewed head: `1a5e03610f5c70218501d9e1dca7a101ae1c7870`
- newest exact head found with both ordinary CI and live external-tool qualification green:
  `30a38251bccb5157beb68202ffe630f6253771e0`
- green runs for that candidate baseline:
  - CI `37143714313`
  - Live external-tool qualification `37143714261`
- current reviewed head `1a5e036...` is ten commits ahead of that green candidate and
  had both hosted workflows fail; it is research evidence, not a closure-grade pin.

Current upstream contracts reviewed:

- .eggb bundle manifest remains schema v2;
- experiment plans accept schemas v1-v10;
- standalone comparison receipts are schema v4 and retain compatibility readers
  for v1-v3;
- execution status remains distinct from comparison verdict;
- comparison receipts now distinguish performance verdict, independent
  security-correctness evidence, and final aggregate verdict.

Primary class: integration / evidence normalization / compatibility

## 1. Objective

Add an Eggbench evidence adapter to `eggplan-integrations` that converts
host-verified Eggbench bundle and comparison facts into bounded
`EvidenceObservation` values without importing Eggbench runtime ownership.

The adapter must preserve the semantic distinction among:

1. existence/integrity of a finalized immutable .eggb bundle;
2. execution outcome of the measured run;
3. comparison/qualification verdict;
4. comparison validity/incomparability;
5. subject and producer provenance.

A valid bundle is not automatically a passing benchmark. A completed comparison
with no gated aggregate verdict is not automatically a passing benchmark.

## 2. Upstream baseline gate

Before fixture freeze, re-check current `eggstack/eggbench` main and select the
newest immutable revision that satisfies all of:

- required bundle/inspect/compare DTO fields are present and understood;
- ordinary hosted CI is green on the exact SHA;
- live external-tool qualification is green on the exact SHA;
- the selected comparison receipt and bundle manifest versions are covered by
  the adapter's explicit compatibility matrix.

`30a38251...` is the current candidate because it is the newest exact head
found during planning with both workflow families green. Do not silently move
fixtures to `1a5e036...` or another newer SHA merely because it is current.

If newer Eggbench contract changes are required but no compatible exact SHA has
green ordinary + live qualification, stop and record the external gate instead
of weakening Eggplan fixtures or accepting a red dependency baseline.

## 3. Ownership and dependency boundary

Keep the M001/M002 normalization-only architecture.

`eggplan-integrations` must not:

- depend on `eggbench-core`, `eggbench-cli`, or `eggbench-runner`;
- execute `eggbench`;
- walk or verify arbitrary .eggb directories itself;
- open network connections;
- discover executables;
- mutate a ProviderRegistry;
- infer provider authority from Eggbench payload text.

The host owns acquisition and native verification. The preferred host path is:

    eggbench inspect --json / compare --json
              |
              v
      host validates exact upstream envelope
              |
              v
    bounded Eggplan adapter DTO
              |
              v
       EvidenceObservation

Update `scripts/check-integrations-boundary.sh` so the new Eggbench module is
covered by the same no-sibling-runtime, no-process, no-network, and
no-trust-enrollment checks as the existing Eggwork/Eggsearch adapters.

## 4. Provider identities and allowed evidence kinds

Use adapter-fixed provider identity. Recommended initial identity:

- `epp_eggbench`
- provider class `eggbench-verified-evidence-v1`

The adapter descriptor may allow:

- `EvidenceKind::Artifact`
- `EvidenceKind::Benchmark`

Do not add `Attestation` in M003a.

Hosts still decide whether this provider descriptor is enrolled in a
`ProviderRegistry`. Adapter-fixed identity prevents caller text from selecting
a different provider but does not itself grant trust.

## 5. Verified bundle DTO

Define a strict bounded DTO owned by Eggplan, for example
`EggbenchBundleEvidenceV1`, with an explicit schema version and no unbounded
native payload.

Retain only facts needed to identify and interpret verified evidence:

- reviewed Eggbench revision/version provenance;
- bundle manifest schema version;
- stable run ID;
- exact manifest SHA-256 supplied by the verifying host;
- execution status, when present;
- comparison verdict embedded in the bundle, when present;
- legacy v1 status, when present;
- bounded subject revision/digest hints from Eggbench;
- artifact count and total retained bytes;
- selected bounded artifact handles/digests;
- bounded driver/producer provenance needed for interpretation.

Do not retain complete manifest JSON, trial stdout/stderr, arbitrary diagnostic
prose, environment dumps, secret-bearing route configuration, or large metric
arrays/histograms.

The host may compute the exact manifest digest after native verification if
Eggbench's inspect projection does not expose it directly. An upstream
`manifest_sha256` inspect field is desirable but is not a hard dependency.

## 6. Comparison DTO

Define a separate strict DTO, for example `EggbenchComparisonEvidenceV1`,
rather than overloading the bundle DTO.

Retain:

- comparison receipt schema version;
- policy ID;
- candidate run ID + candidate manifest digest;
- optional baseline run ID + baseline manifest digest;
- aggregate verdict;
- performance verdict when the receipt version exposes it;
- correctness aggregate when present;
- whether comparison-critical dimensions matched;
- bounded warning categories/count;
- exact SHA-256 of the standalone comparison receipt when one exists.

Accept only explicitly supported receipt schemas. Planning baseline requires
v1-v4 compatibility or an explicit rejection matrix documented in
`architecture/eggbench-adapter.md`.

Unknown interpretation-changing receipt versions fail closed.

## 7. Status normalization

Normalize bundle execution and comparison separately.

### Bundle/artifact observation

A natively verified finalized bundle with a valid manifest/artifact digest tree
may produce `Artifact::Passed` even when the measured run failed. That status
means the artifact exists and verified structurally; it does not assert
benchmark success.

If the host reports native integrity verification failure, reject the adapter
input rather than producing a trusted observation from corrupted evidence.

### Benchmark observation

Map current Eggbench semantics conservatively:

- aggregate `pass` -> `Passed`
- aggregate `fail` -> `Failed`
- aggregate `inconclusive` -> `Inconclusive`
- aggregate `invalid` -> `Inconclusive`
- descriptive/no aggregate verdict -> `Inconclusive` unless the host is
  explicitly recording only an Artifact observation
- failed execution -> `Failed`
- cancelled execution -> `Skipped`

A legacy v1 overloaded inconclusive status remains `Inconclusive`. Never
invent whether it meant missing comparison versus inconclusive comparison.

Unknown enum values fail closed.

## 8. Subject and verification binding

Eggbench subject fields are provenance hints, not Eggplan
`SubjectRevision` authority.

The host supplies `ObservationContext.subject` and, for Benchmark evidence,
the authoritative `VerificationDigest`.

For Git subjects, if the bounded native Eggbench DTO declares an exact revision
and the Eggplan observation context is also Git, require compatible revision
identity or reject the observation. Do not translate an Eggbench label or
opaque digest into an Eggplan repository identity.

Never derive the verification digest from run ID, bundle path, manifest digest
alone, policy ID, metric names, or human prose.

## 9. Artifact references

Represent large native evidence by bounded opaque references.

Recommended forms:

- `eggbench:bundle:<run-id>`
- `eggbench:comparison:<candidate-run-id>:<receipt-digest-prefix>`

Attach exact `sha256:<64 lowercase hex>` digests when available. Do not retain
temporary filesystem paths as durable identity.

## 10. Compatibility fixtures

Freeze fixtures from the selected exact Eggbench baseline for at least:

1. successful finalized bundle with no comparison verdict;
2. failed execution with a structurally valid bundle;
3. cancelled execution;
4. comparison pass;
5. comparison fail;
6. comparison inconclusive;
7. comparison invalid/incomparable;
8. descriptive/no-verdict comparison;
9. current v4 receipt with performance + correctness fields;
10. legacy receipt/bundle cases required by the compatibility matrix.

Fixtures should be minimal serialized compatibility fixtures, not copied native
runtime source or entire benchmark bundles. Record the upstream source SHA for
every fixture family.

## 11. Negative/security tests

At minimum:

- unknown DTO schema rejected;
- unknown manifest/receipt version rejected unless explicitly supported;
- malformed SHA-256 rejected;
- candidate/baseline identity mismatch rejected;
- duplicate artifact handles rejected;
- subject revision disagreement rejected;
- missing Benchmark verification binding rejected by the existing core contract;
- artifact integrity success cannot become Benchmark Passed;
- descriptive comparison cannot become Benchmark Passed;
- Invalid cannot become Failed or Passed;
- legacy inconclusive cannot be disambiguated by inference;
- arbitrary native prose is absent from serialized observations;
- adapter cannot enroll its own provider.

## 12. Documentation

Add `architecture/eggbench-adapter.md` documenting the exact reviewed and
fixture baselines, host/native-verification boundary, DTO schemas/bounds,
manifest/receipt compatibility matrix, Artifact vs Benchmark semantics, status
mapping, subject/verification ownership, and explicit attestation exclusion.

## 13. Ordered work packages

### WP1 — Rebaseline and freeze compatibility contract

Select the exact dual-green Eggbench SHA and freeze minimal native fixture
provenance.

### WP2 — Bundle evidence normalizer

Implement bounded verified-bundle DTO parsing, artifact references, provider
descriptor, and structural status mapping.

### WP3 — Comparison evidence normalizer

Implement receipt compatibility, aggregate/performance/correctness distinction,
and conservative Benchmark status mapping.

### WP4 — Authority and compatibility hardening

Add subject agreement, verification-binding regressions, unknown-version
failure, and boundary guard updates.

### WP5 — Documentation and hosted qualification

Complete architecture docs, registry/roadmap closure data, native
Linux/macOS/Windows + Rust 1.89 qualification.

## 14. Required verification

    cargo fmt --all -- --check
    cargo check --workspace --all-targets --locked
    cargo clippy --workspace --all-targets --locked -- -D warnings
    cargo test --workspace --locked
    cargo +1.89.0 check --workspace --all-targets --locked
    cargo +1.89.0 test --workspace --locked
    bash scripts/check-integrations-boundary.sh
    bash scripts/check-core-boundary.sh
    git diff --check

Hosted native CI must pass on the exact Eggplan implementation SHA. The
Eggbench fixture baseline must cite exact successful ordinary and live
qualification workflow IDs.

## 15. Acceptance criteria

M003a closes when:

1. Eggplan normalizes natively verified .eggb identity without importing
   Eggbench runtime dependencies.
2. Artifact integrity and benchmark success remain separate claims.
3. supported comparison receipts preserve Pass/Fail/Inconclusive/Invalid
   semantics without collapsing correctness and performance.
4. descriptive/no-verdict evidence cannot become passing Benchmark evidence.
5. Benchmark observations remain exact-subject and verification-bound.
6. large native evidence remains outside Eggplan control records.
7. unknown versions/enum values fail closed.
8. fixture provenance points to an exact dual-green Eggbench SHA.
9. boundary guards prove no sibling runtime/process/network/trust authority
   entered the integration crate.
10. native/MSRV Eggplan qualification is green.

## 16. Stop conditions

Stop and record a dependency/corrective if current Eggbench semantics cannot be
represented without importing `eggbench-core`; native verification cannot be
separated from normalization; a newer required contract has no green exact
hosted baseline; compatibility requires guessing meaning from prose; Artifact
and Benchmark authority would need to be conflated; or implementation would
verify GitHub/Sigstore attestations here instead of Interoperability M001.

## 17. Closure evidence

Record the selected Eggbench SHA; exact Eggbench ordinary/live green run IDs;
fixture manifest; DTO/version compatibility matrix; status-mapping matrix;
negative authority tests; integration-boundary guard proof; Eggplan
implementation SHA; Eggplan native/MSRV hosted run IDs; and residual
compatibility findings/M003b disposition.
