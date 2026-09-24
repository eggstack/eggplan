# Eggstack Integrations M002 — Eggwork and Eggsearch Evidence Adapters

Status: ready for handoff

Repository baseline: 0904218554c425c30aa6501b58d7fb8bcb839414

Source roadmap:

- plans/subsystems/eggstack-integration-roadmap.md

Predecessor closure:

- plans/closure/eggstack-integration/001-closed.md
- plans/closure/evidence-closure/002-c002-closed.md

Fresh sibling baselines reviewed:

- Eggwork: eggstack/eggwork @ 128f808c62f176d414dd18a705773e45f5e2891a
- Eggsearch: eggstack/eggsearch @ 5db6e1984a1441787f6d6a54754eb4a685766ec2

Relevant current contracts:

Eggwork:
- `eggwork-core::ExecutionSnapshot`;
- `ExecutionState`;
- `ExecutionResult`;
- `ExecutionFailure`;
- `ExecutionGeneration`;
- `ArtifactRecord`;
- protocol-neutral core, Rust 1.89.

Eggsearch:
- `EvidenceBundle`;
- deterministic `bundle_id`;
- source IDs/provider IDs;
- `TrustLevel`;
- trust markers;
- evidence roles;
- fetched-item links;
- gap/warning summaries;
- bundle limits/truncation flags.

Primary class: integration / normalization / evidence provenance

## 1. Objective

Replace the M001 synthetic sibling-shaped fixtures with production-usable,
bounded normalizers for real Eggwork execution results and Eggsearch evidence
bundles.

M002 still does not make Eggplan an executor, search client, MCP client,
network stack, scheduler, or credential owner.

The host acquires native results. Eggplan normalizes them into
EvidenceObservation values through the M001 SPI.

## 2. Integration architecture

    Host / caller
       |
       +-- Eggwork client/executor ----> ExecutionSnapshot + ArtifactRecord(s)
       |
       +-- Eggsearch client/tool ------> EvidenceBundle
       |
       v
    eggplan-integrations
       |
       +-- eggwork adapter
       +-- eggsearch adapter
       |
       v
    M001 finalize_observation(...)
       |
       v
    immutable Eggplan EvidenceObservation

Provider identity remains adapter-defined and host trust remains explicit.

## 3. Dependency strategy

Do not import sibling runtimes wholesale.

### Eggwork

Although `eggwork-core` is small/protocol-neutral, M002 should avoid a
non-publishable Git runtime dependency unless package/release policy already
supports it.

Preferred implementation:

- define a versioned, strict Eggwork adapter input DTO matching the reviewed
  `ExecutionSnapshot` / `ExecutionResult` / `ArtifactRecord` wire shape;
- parse bounded serialized JSON supplied by the host;
- qualify against golden fixtures generated from the actual
  `eggwork-core` types at the reviewed SHA.

If `eggwork-core` is available from the approved package channel at
implementation time, a direct dependency is acceptable only after confirming
MSRV/dependency policy and recording the exact version/revision.

### Eggsearch

Do not add a direct production dependency on the full `eggsearch` crate in
M002. It brings Tokio, MCP, HTTP, scraping, and other runtime concerns that do
not belong in Eggplan normalization.

Use a strict bounded adapter DTO for the authoritative subset of serialized
`EvidenceBundle`.

This is not a fork of Eggsearch semantics: source fixtures and field mapping are
pinned to the reviewed sibling contract and rechecked at handoff.

## 4. Eggwork adapter descriptor

Add a fixed adapter descriptor, conceptually:

- provider id: `epp_eggwork`;
- class: `execution`;
- allowed kinds:
  - Command;
  - Test;
  - DelegatedRun;
  - Benchmark where the host explicitly requests that semantic kind.

The native payload cannot choose provider ID/class/allowed kinds.

The host's `ObservationContext.requested_kind` selects the semantic
EvidenceKind within the descriptor allowlist.

## 5. Eggwork input contract

Minimum bounded input:

    execution_id
    generation
    snapshot state
    optional terminal result:
      state
      exit_code
      failure
      stdout_bytes / stderr_bytes
      omitted byte counts
      cleanup warning presence/category
      finalization_failure
      artifact_count
      sandbox result
      resource results
    artifacts[]:
      artifact_id
      execution_id
      generation
      path
      kind
      digest
      size
      executable
      created/expires timestamps

Do not persist stdout/stderr content through this adapter.

Validate:

- execution ID syntax/bounds;
- generation > 0;
- snapshot/result state consistency;
- artifact execution ID/generation match;
- duplicate artifact IDs;
- digest syntax;
- bounded metadata;
- artifact count consistency where possible.

## 6. Eggwork status mapping

Define an explicit matrix.

Recommended semantic mapping:

| Native state | Eggplan status |
|---|---|
| Accepted / Preparing / Running / Cancelling | InProgress |
| Succeeded | Passed |
| Failed | Failed |
| TimedOut | Failed |
| Cancelled | Skipped |
| Interrupted | Inconclusive |

Additional rules:

- terminal snapshot without terminal result -> Inconclusive;
- contradictory snapshot/result states -> reject invalid input;
- `finalization_failure` -> Failed when the requested verification depends on
  finalization/artifact capture, otherwise Inconclusive; choose one stable
  documented rule in implementation rather than hiding the condition;
- required resource/sandbox failure remains Failed;
- cleanup warning does not turn a successful execution into Passed without
  being recorded; preserve a bounded warning code.

Do not collapse interruption/cancellation into success.

## 7. Eggwork verification binding

For Command/Test/DelegatedRun/Benchmark, the adapter MUST use the
`ObservationContext.verification_digest` supplied by the host.

The adapter must never derive verification identity from:

- execution ID;
- generation;
- exit code;
- artifact ID;
- human command text returned in a result.

The host computes the digest from the exact canonical verification/execution
specification before submission and reuses that binding when finalizing the
observation.

Missing required binding is a normalization error/fail-closed condition.

## 8. Eggwork native provenance

Persist bounded metadata only:

- execution ID;
- generation;
- terminal state;
- exit code if present;
- failure/finalization class;
- stdout/stderr byte counts and omitted counts;
- sandbox/resource outcome codes;
- artifact count.

Artifact references should preserve:

- native artifact ID;
- digest;
- bounded logical path or opaque native handle;
- size.

No credentials, endpoints, lease tokens, raw environment, raw output, or
private key material.

## 9. Eggsearch adapter descriptor

Add a fixed descriptor:

- provider id: `epp_eggsearch`;
- class: `research`;
- allowed kind: Research.

Do not let bundle/provider metadata select Eggplan provider identity.

## 10. Eggsearch input subset

Parse a bounded versioned subset of current `EvidenceBundle` sufficient to
preserve provenance/trust/gaps without persisting untrusted content:

- bundle_id;
- goal presence/digest or bounded description only if policy allows;
- created_at as provenance string if validated/bounded;
- sources:
  - source_id;
  - provider_id;
  - trust;
  - stable flag;
  - evidence_role;
  - source_kind;
- fetched_items:
  - fetch_id;
  - source_id;
  - fetched flag;
  - truncation flag;
  - line/range metadata where present;
  - trust classification;
- source_links;
- trust_summary;
- provider_summary;
- gaps:
  - kind;
  - source/provider IDs;
  - affected source IDs;
  - evidence role;
- warning codes/classes, not arbitrary large messages;
- bundle limits/truncation flags;
- research claim/conflict counts only unless a later plan explicitly needs
  claim payloads.

Do not persist source snippets, fetched text, URLs, or arbitrary research prose
in the generic observation metadata.

Native handle:

- `eggsearch:bundle:<bundle_id>`.

## 11. Eggsearch trust handling

Eggsearch's `TrustLevel` is source/content provenance, not Eggplan provider
authority.

M002 must never auto-enroll `LocalTrusted` as a trusted Eggplan provider.

Rules:

- adapter provider identity remains fixed `epp_eggsearch`;
- ProviderRegistry trust remains host-controlled;
- normalized `SourceTrust` should default to `ExternalUntrusted` for
  Eggsearch bundles in M002;
- preserve counts of external/local/unknown trust as bounded metadata;
- a future explicit host-local policy may promote a local-only bundle, but that
  is not M002.

This conservative rule prevents content-origin labels from becoming authority.

## 12. Eggsearch status mapping

The observation describes the research/retrieval operation, not the truth of
every claim in the bundle.

Recommended mapping:

- structurally valid bundle with at least one usable source and no fatal
  acquisition failure -> Passed;
- empty/no-source bundle -> Unavailable;
- provider failure/fetch failure that prevents requested evidence -> Unavailable
  or Inconclusive according to deterministic gap classification;
- bundle truncated or usable-with-gaps -> Inconclusive when the requested
  research requirement cannot be proven complete;
- interrupted/deadline-limited retrieval -> Inconclusive;
- malformed/inconsistent bundle -> normalization error, no observation.

Define a small `EggsearchGapSeverity` table over current gap kinds rather than
matching arbitrary warning prose.

A Passed Research observation only means the requested research operation
produced a valid bundle under host policy. It does not assert that external
claims are true.

## 13. Gap classification

At minimum classify current gap families:

### fatal/unavailable

- NoPrimarySourceFound when primary source is required by adapter profile;
- FetchFailed when all required fetched evidence failed;
- ProviderDegraded when no usable fallback exists;
- NativeAdvisoryUnavailable for an advisory-required profile.

### inconclusive

- AllResultsExternalUntrusted;
- FreshnessNotEnforced;
- NoCounterpointFound;
- NoBenchmarksFound;
- MissingTests/Examples/Manifest/Changelog/SecurityPolicy;
- local dirty/remote mismatch;
- partial/truncated acquisition.

### informational/non-fatal

- gaps unrelated to the requested evidence role/profile.

Keep this profile bounded and explicit. Do not build a general policy DSL.

## 14. Contract versioning and sibling drift

Add constants recording reviewed baselines:

- Eggwork SHA `128f808c62f176d414dd18a705773e45f5e2891a`;
- Eggsearch SHA `5db6e1984a1441787f6d6a54754eb4a685766ec2`.

These are fixture provenance, not runtime trust.

At handoff, implementation MUST recheck sibling heads. If authoritative fields
changed, update the DTO/fixtures and record the new reviewed SHA before coding.

Unknown required enum variants/contract versions fail closed.
Additive irrelevant fields may be tolerated only if they cannot change
authority/status semantics.

## 15. Real sibling fixture qualification

Check in bounded fixtures generated from the reviewed siblings:

Eggwork:

- running snapshot;
- successful command;
- nonzero/failed execution;
- timeout;
- cancellation;
- interruption;
- artifact success;
- artifact/finalization failure.

Eggsearch:

- normal external web bundle;
- local-trusted-labeled bundle (prove no host trust escalation);
- provider degraded bundle;
- all-external-untrusted gap;
- truncated bundle;
- empty/unavailable bundle;
- malformed/unknown gap/status negative fixture.

Fixtures must not contain secrets or large fetched text.

## 16. M001 SPI composition

Reuse:

- AdapterDescriptor;
- ObservationContext;
- NormalizedProviderResult;
- `verification_digest`;
- `finalize_observation`.

Do not duplicate the generic finalization logic.

Provider-specific modules should only:

1. validate native input;
2. map native state/provenance to NormalizedProviderResult;
3. call the existing finalizer.

## 17. Public API

Recommended narrow surface:

    eggwork::descriptor()
    eggwork::parse_snapshot(bytes)
    eggwork::normalize(snapshot, artifacts, context)

    eggsearch::descriptor()
    eggsearch::parse_bundle(bytes)
    eggsearch::normalize(bundle, context, profile)

Exact names may vary.

No function should:

- execute Eggwork work;
- connect to eggworkd;
- query Eggsearch;
- call MCP;
- fetch URLs;
- read credentials.

## 18. Boundary guards

Extend `check-integrations-boundary.sh` to reject:

- tokio/reqwest/axum/rmcp/network dependencies in eggplan-integrations;
- process spawning;
- direct credential/environment acquisition;
- scheduler/executor ownership;
- accidental use of Eggsearch trust labels as provider registration.

If a direct `eggwork-core` dependency is approved, explicitly allow only that
protocol-neutral package and document why.

## 19. Required tests

### Eggwork

- every state mapping;
- terminal/result consistency;
- generation/artifact identity;
- verification digest required for execution kinds;
- no output/environment/lease secret persistence;
- artifact digest/native handle mapping;
- deterministic repeated normalization;
- unknown enum/version fails closed.

### Eggsearch

- valid research bundle;
- empty/unavailable;
- gap classification;
- truncation/inconclusive;
- source trust preserved as metadata;
- LocalTrusted does not enroll host trust;
- URLs/snippets/fetched text omitted from observation metadata;
- bundle ID/native handle;
- deterministic repeated normalization;
- malformed/unknown semantic variant fails closed.

### Cross-SPI

- fixed provider identity cannot be overridden by payload;
- unsupported EvidenceKind rejected;
- metadata bounds;
- sensitive key/value rejection remains active;
- observation content digest stable.

## 20. Documentation

Update:

- `architecture/provider-spi.md`;
- add `architecture/eggwork-adapter.md`;
- add `architecture/eggsearch-adapter.md`;
- subsystem roadmap baseline table.

Document exact status/trust/gap matrices.

## 21. Verification

At minimum:

    cargo fmt --all -- --check
    cargo check --workspace --all-targets --locked
    cargo clippy --workspace --all-targets --locked -- -D warnings
    cargo test --workspace --locked
    cargo +1.89.0 check --workspace --all-targets --locked
    cargo +1.89.0 test --workspace --locked
    bash scripts/check-integrations-boundary.sh
    bash scripts/check-core-boundary.sh
    git diff --check

Hosted Linux/macOS/Windows and Rust 1.89 lanes must pass.

## 22. Acceptance criteria

M002 closes when:

1. real reviewed Eggwork serialized results normalize through the production
   adapter;
2. real reviewed Eggsearch bundles normalize through the production adapter;
3. Eggwork execution kinds require host-supplied verification binding;
4. provider identity cannot be selected by native payload;
5. Eggsearch content trust never auto-promotes host provider trust;
6. raw output/fetched text/URLs/credentials are not persisted in generic
   observation metadata;
7. native failures/gaps remain distinguishable;
8. no network/process/scheduler/search acquisition code enters
   eggplan-integrations;
9. current sibling fixtures and native/MSRV CI pass.

## 23. Stop conditions

Stop and report if:

- Eggwork normalization requires importing executor/client ownership;
- Eggsearch normalization requires a production dependency on its full runtime
  solely to access DTOs;
- verification binding can only be reconstructed from execution IDs/prose;
- external Eggsearch content must be auto-trusted to satisfy tests;
- sibling contract drift makes fixture-only mapping ambiguous;
- a general policy engine is required to classify research gaps.

## 24. Closure evidence

Record:

- exact reviewed sibling SHAs;
- dependency decision (DTO vs approved eggwork-core package);
- Eggwork state/status matrix;
- verification-binding tests;
- artifact provenance mapping;
- Eggsearch gap/status matrix;
- trust non-escalation evidence;
- sensitive/untrusted payload omission tests;
- fixture provenance;
- implementation SHA;
- hosted native/MSRV workflow IDs;
- M003 disposition.
