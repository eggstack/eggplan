# Eggstack Integrations M001 — Evidence Provider SPI

Status: closed

Eggplan repository baseline: 9f07e1528e44e8a1dd9a0df7aa03b61c2adc3e13

Implementation commit: 24682f74040aa0fd8a6c2ff6311098c964cc6358

Closure record: plans/closure/eggstack-integration/001-closed.md

Initial planning sibling baselines (historical):

- Eggwork: c990ffa2864886c502c1d5581f0d732e26fe56ee
- Eggsearch: 5db6e1984a1441787f6d6a54754eb4a685766ec2
- Eggbench: 3b93979a7fd30a08f7e367ffc94f911ae5fb8bec
- Eggsact: 576f4b0ac09238a42e5561c2da6da8ff4a47bce6

Execution-time sibling baselines, re-checked 2026-09-24:

- Eggwork: 128f808c62f176d414dd18a705773e45f5e2891a
- Eggsearch: dfa90e050c5434f3346902aeb4074901c58e90d1
- Eggbench: d7d1fd9a9b67a5b2ca6a816c841d2588a368aae9
- Eggsact: 40959b704431430668e9ca2bfe959a8ef32495d8

Source roadmap:

- plans/subsystems/eggstack-integration-roadmap.md

Long-term requirements:

- plans/000-long-term-specification.md sections 10, 16-17, 19-20
- plans/001-terminology-and-domain-model.md sections 5 and 8
- plans/002-long-term-roadmap.md Eggstack M001

Primary class: integration infrastructure / trust boundary

## 1. Objective

Freeze a narrow provider-adapter SPI that lets external systems normalize
facts into Eggplan evidence without moving their transport, execution,
scheduling, search, artifact storage, or authentication responsibilities into
Eggplan core.

M001 implements the adapter contract and synthetic conformance fixtures only.
Real Eggwork/Eggsearch adapters are M002; Eggbench/CI/forge are M003.

## 2. Current sibling interface re-check

### Eggwork

At reviewed baseline, Eggwork remains a caller-selected fixed-target executor.
Relevant stable facts include:

- ExecutionSpec and canonical request digests;
- ExecutionId + ExecutionGeneration fenced identity;
- ExecutionSnapshot and explicit states Accepted/Preparing/Running/Cancelling/
  Succeeded/Failed/Cancelled/TimedOut/Interrupted;
- result exit/failure/finalization/resource/sandbox facts;
- content-addressed ArtifactRecord metadata.

Eggplan must normalize these facts; it must not own NodeClient, leases,
cancellation, process supervision, or workspace materialization. The current
core exposes canonical `request_digest` and a workspace-aware
`request_digest_with_workspace`; an Eggwork adapter can bind those authoritative
digests through the provider-namespaced Eggplan verification helper.

### Eggsearch

Eggsearch EvidenceBundle remains deterministic and non-summarizing, carrying
source IDs, provider IDs, trust markers, quality, selected fetched spans,
provider diagnostics, and explicit EvidenceGap values.

External-untrusted source content cannot become trusted host proof merely
because an Eggsearch bundle exists.

### Eggbench

Eggbench .eggb manifest v2 separates execution_status from optional
comparison_verdict, validates immutable artifact metadata/digests, and exposes
a read-only bundle inspector.

The future adapter should reference/verify bundles rather than copy large
payloads into Eggplan.

### Eggsact

Eggsact remains an optional deterministic utility source. The SPI cannot
require MCP or an Eggsact process.

## 3. Crate and dependency boundary

Add a new crate, suggested `eggplan-integrations`.

M001 dependencies should remain limited to Eggplan libraries plus small
serialization/error utilities. Do NOT add Eggwork/Eggsearch/Eggbench network or
runtime dependencies yet.

The crate MUST NOT be a generic executor.

No async runtime is required in M001.

## 4. SPI model

Prefer a normalization SPI rather than an "execute" trait.

External hosts acquire native results using their native clients/runtimes, then
an adapter normalizes those facts.

Suggested concepts:

### AdapterDescriptor

Stable bounded descriptor:

- fixed EvidenceProviderId;
- provider class;
- allowed EvidenceKind set;
- adapter schema/version;
- capability flags such as supports_in_progress, artifacts,
  verification_binding, research_trust_metadata.

The descriptor can be converted into a core ProviderDescriptor, but merely
constructing an adapter MUST NOT auto-register it as trusted. The host still
chooses ProviderRegistry membership.

### ObservationContext

Host-supplied immutable context:

- EvidenceObservationId;
- SubjectRevision;
- requested EvidenceKind;
- observation timestamp;
- optional invocation reference;
- required VerificationDigest for execution-derived kinds;
- bounded contextual metadata needed for normalization.

Provider ID comes from AdapterDescriptor, not native payload text.

### NormalizedProviderResult

A bounded provider-neutral result containing:

- truthful EvidenceStatus;
- bounded result metadata;
- ArtifactRef list;
- optional provider-native stable handle references;
- no credentials or unbounded stdout/source content.

The adapter finalizer combines descriptor + context + normalized result into an
EvidenceObservationInput/Observation and validates kind/binding/bounds.

### Normalization errors

Separate malformed/unsupported normalization from truthful native outcomes.

A transport outage, provider unavailable state, timeout, execution failure, or
inconclusive result should normally become an explicit evidence status when
the adapter has enough authoritative facts to say so. Malformed native data or
authority mismatch is an adapter error, not an invented Passed/Failed state.

## 5. Verification-spec binding

Add one shared helper for deriving an Eggplan VerificationDigest from a
bounded, versioned, provider-namespaced canonical verification specification.

The helper must domain-separate at least:

- adapter/provider namespace;
- verification-spec schema version;
- canonical payload bytes.

Do not compare raw human command strings.

Provider-specific M002 adapters may use stable native canonicalization where
appropriate. For example, Eggwork already exposes a canonical request digest;
the Eggwork adapter can bind that fact into the Eggplan provider-namespaced
verification specification rather than reimplementing command normalization.

Research evidence may remain optionally bound according to Evidence C001.

## 6. Trust and authority invariants

- Native result fields cannot choose the trusted Eggplan provider ID.
- AdapterDescriptor does not itself grant trust.
- Passing status from an unregistered provider remains unusable by assessment.
- Adapter metadata cannot widen allowed evidence kinds.
- Free-form source text cannot create trusted status.
- External-untrusted Eggsearch content must preserve that trust marker in
  bounded metadata/artifact references and cannot be normalized as a trusted
  execution pass.
- Digests provide integrity, not authentication.
- Secrets/tokens/endpoints are not persisted in observations by generic SPI
  machinery.

## 7. Status normalization contract

Freeze generic conformance rules without hard-coding every provider:

- in-progress native state -> InProgress;
- authoritative successful verification -> Passed only when the provider
  semantics actually establish the requirement;
- authoritative verification failure -> Failed;
- requested operation never executed -> NotRun;
- deliberate omission -> Skipped;
- dependency/policy preventing execution -> Blocked;
- result/provider cannot currently be obtained/resolved -> Unavailable;
- result exists but cannot support pass/fail -> Inconclusive.

Provider-specific mapping tables are implemented and tested in later
milestones.

Do not turn generic network/authorization errors into Passed.

## 8. Artifact handling

SPI accepts only bounded ArtifactRef control records. It never embeds large
native artifacts.

The contract must support:

- stable handle/reference;
- optional digest;
- media type;
- future role metadata through bounded adapter metadata if core ArtifactRef
  does not yet carry a role.

M002/M003 adapters will map Eggwork artifacts, Eggsearch bundles, and Eggbench
.eggb paths/manifest digests through this seam.

## 9. Synthetic conformance fixtures

Add synthetic native-result fixtures modeling the reviewed sibling contracts
without importing their crates yet:

- Eggwork-like running/succeeded/failed/timed-out/cancelled snapshot;
- Eggsearch-like trusted local and external-untrusted bundle/gap cases;
- Eggbench-like completed/no-comparison, pass, fail, inconclusive bundle
  summaries.

These fixtures test SPI expressiveness only. They must carry source baseline
metadata and must not be presented as live provider integration.

## 10. Ordered work packages

### WP1 — Adapter descriptors/context/errors

Implement strict bounded SPI types and trust-boundary docs.

### WP2 — Verification specification helper

Implement provider-namespaced canonical digest derivation with golden fixtures.

### WP3 — Normalization/finalization helper

Normalize synthetic results into EvidenceObservation while preserving explicit
status/artifacts/metadata and host-controlled provider authority.

### WP4 — Conformance corpus

Exercise Eggwork/Eggsearch/Eggbench-shaped synthetic results and negative trust
cases.

### WP5 — Qualification/documentation

Add `architecture/provider-spi.md`, native CI, current sibling baseline table,
roadmap/registry closure updates.

## 11. Required tests

At minimum:

- descriptor IDs/classes/kinds bounded and strict;
- duplicate/unsupported kind rejected;
- provider ID cannot come from native payload;
- descriptor does not auto-trust itself;
- provider-namespaced verification digests are deterministic and distinct
  across namespace/version/payload;
- execution-derived finalization requires binding;
- observation digest includes normalized semantics;
- running -> InProgress;
- unavailable -> Unavailable, not Failed/Passed;
- synthetic execution success can pass only under registered provider policy;
- Eggsearch external-untrusted fixture cannot become trusted pass by text;
- Eggbench no-comparison fixture remains distinct from comparison
  inconclusive/fail;
- large metadata/artifact lists rejected;
- secret-like native debug/test fixtures do not appear in normalized metadata
  unless explicitly safe-listed;
- core remains free of async/network dependencies;
- Rust 1.89 and native CI pass.

## 12. Required verification

Run full Eggplan workspace gates and focused integration-SPI tests. Closure
must cite the sibling SHAs actually reviewed at execution time.

## 13. Acceptance criteria

M001 closes when the SPI can faithfully represent the reviewed execution,
research, and bundle evidence shapes while keeping acquisition/transport and
trust ownership outside eggplan-core, preserving v2 verification binding, and
preventing provider/native text from self-authorizing evidence.

## 14. Stop conditions

Stop and report if:

- current sibling interfaces materially changed from the reviewed baselines;
- a generic SPI requires Tokio/HTTP/MCP in eggplan-core;
- the adapter must execute work to normalize evidence;
- preserving provider semantics requires hiding native failure state;
- trust can only be represented by auto-registering the adapter.

## 14.1 Current implementation evidence

The synchronous `eggplan-integrations` crate is implemented in
`crates/eggplan-integrations/`. Its final implementation commits and
qualification are recorded in
`plans/closure/eggstack-integration/001-closed.md`.

## 15. Closure evidence required

Record:

- execution-time sibling SHAs;
- SPI public API inventory;
- trust/authority negative tests;
- verification-digest fixtures;
- synthetic conformance matrix;
- dependency/static boundary check;
- cross-platform CI.
