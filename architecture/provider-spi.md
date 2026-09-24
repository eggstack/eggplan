# Evidence provider normalization SPI

`eggplan-integrations` defines a synchronous, provider-neutral seam for
normalizing externally acquired facts into Eggplan observations. It has no
network, async runtime, sibling-project, process, scheduler, or artifact-store
dependency. External hosts retain acquisition, authentication, and native
state authority.

## Public contract

- `AdapterDescriptor` declares a bounded provider ID, class, allowed kinds,
  adapter version, and capabilities. It can produce a core
  `ProviderDescriptor`, but only the host can explicitly add that descriptor
  to its trusted `ProviderRegistry`.
- `ObservationContext` supplies observation ID, exact subject, kind, time,
  invocation handle, optional verification binding, and bounded metadata.
- `NormalizedProviderResult` carries truthful status, bounded metadata and
  artifact references, and an explicit source trust marker where applicable.
  It has no provider ID, raw output field, or transport credentials.
- `finalize_observation` takes provider identity from the descriptor, checks
  declared kinds and capabilities, requires verification binding for
  execution-derived kinds, and delegates final v2 validation/digest creation
  to eggplan-core.
- `verification_digest` hashes canonical JSON inside a domain-separated
  envelope containing provider namespace and nonzero schema version.

## Status and trust

The SPI carries normalized statuses without collapsing native outcomes:
InProgress, Passed, Failed, NotRun, Skipped, Blocked, Unavailable, and
Inconclusive. An adapter maps native states only when it has authoritative
facts. Provider-specific mapping belongs to provider adapters, not this
generic crate. An unavailable result remains unavailable; it is never promoted
to a pass because of a message or debug string.

Research-class results must preserve an explicit source trust marker. Status
describes the normalized producer operation; source trust describes content
provenance. A research operation may be Passed when it produced a valid bundle
even when that bundle contains external-untrusted sources. This does not assert
that any source claim is true. The trust marker is included in immutable
observation metadata, and only the caller-supplied registry decides Eggplan
provider trust. A digest establishes content integrity, not authentication.

Generic metadata is bounded and rejects sensitive field names, credential-like
values, and endpoint URLs. Artifact references are bounded control records;
secret-like URLs are rejected and large payloads remain in native storage.
Adapters must safe-list any native facts they copy into metadata. Raw source
text, stdout/stderr, credentials, tokens, and endpoints are not normalization
inputs.

## Reviewed synthetic compatibility baselines

`crates/eggplan-integrations/tests/fixtures/manifest.json` records the
execution-time sibling SHAs. `synthetic-results.json` models Eggwork-like
execution states, Eggsearch-like local/external trust and gaps, and Eggbench
comparison/no-comparison verdicts. These are contract fixtures only; no sibling
crate is imported and no live integration is claimed.

Run `scripts/check-integrations-boundary.sh` to verify there is no sibling,
transport, async, process, or evidence-acquisition dependency.
