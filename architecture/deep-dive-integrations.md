# Deep dive: `eggplan-integrations` (provider SPI + Eggwork/Eggsearch adapters)

Role within the workspace: pure, synchronous normalization of host-acquired
facts into Eggplan observations. See [overview](overview.md) (forward
reference), [provider SPI](provider-spi.md), [Eggwork adapter](eggwork-adapter.md),
and [Eggsearch adapter](eggsearch-adapter.md).

## 1. Crate role: what it is and must never do

- Pure normalization SPI: no acquisition, auth, network, async, process,
  scheduler, or artifact-store ownership. Hosts retain transports, native
  clients, authentication, and registry membership
  (`crates/eggplan-integrations/src/lib.rs:1-9`).
- Dependency surface is minimal by construction
  (`crates/eggplan-integrations/Cargo.toml:8-13`): only `eggplan-core`,
  `serde`/`serde_json`, `sha2`, `thiserror`. No sibling (`eggwork`/`eggsearch`/\
  `eggbench`/`eggsact`), transport (`tokio`/`reqwest`), or runtime crates.
- Must never enroll trust: adapters produce a core `ProviderDescriptor` for the
  host to consider, but only the host can add it to its trusted
  `ProviderRegistry` (`crates/eggplan-integrations/src/lib.rs:102-113`).
  Boundary script enforces all of the above
  (`scripts/check-integrations-boundary.sh:7-25`).
- No `SubjectCapture` / closure finalization surface lives here; that authority
  stays in `eggplan-repo`. This crate ends at `EvidenceObservation`.

## 2. Public contract walkthrough (`src/lib.rs`)

- `ProviderClass` (`src/lib.rs:30-38`): `Execution`, `Research`, `Benchmark`,
  `Utility`, `Other`. `Capabilities` (`src/lib.rs:40-47`) declares
  `supports_in_progress`, `artifacts`, `verification_binding`, and
  `research_trust_metadata` — every downstream gate keys off these flags.
- `AdapterDescriptor` (`src/lib.rs:49-59`; `new` at `src/lib.rs:62-84`,
  `validate` at `src/lib.rs:86-100`): bounded provider ID, class, non-empty
  deduped `allowed_kinds`, version text; explicitly rejects
  `EvidenceKind::HumanJudgment` (`src/lib.rs:96-98`). Deserialization also
  rejects duplicate kinds (`src/lib.rs:232-242`).
- `ObservationContext` (`src/lib.rs:115-128`; `validate` at
  `src/lib.rs:131-145`): observation ID, exact subject, requested kind, time,
  optional `invocation_ref` and `verification_digest`, bounded metadata
  (`MAX_CONTEXT_METADATA = 16`, `src/lib.rs:25`). Invocation refs that look
  like secrets/endpoints are rejected (`src/lib.rs:137-141`).
- `NormalizedProviderResult` (`src/lib.rs:148-160`; `validate` at
  `src/lib.rs:170-190`): truthful status, optional `source_trust`
  (`ProviderTrusted` / `ExternalUntrusted`, `src/lib.rs:162-167`), bounded
  metadata and artifact refs. Deliberately has no provider ID — authority comes
  from the descriptor at finalization (`src/lib.rs:148-149`). Artifact refs are
  length-bounded, secret/endpoint-scanned, then core-validated
  (`src/lib.rs:172-188`).
- `finalize_observation` (`src/lib.rs:285-366`) enforces, in order: descriptor/
  context/result validation; declared-kind membership (`src/lib.rs:293-295`);
  human-judgment refusal (`src/lib.rs:296-298`); research results must carry a
  trust marker and the capability must be declared (`src/lib.rs:299-308`);
  in-progress/artifact gates (`src/lib.rs:309-315`); execution-derived kinds
  (`Command`/`Test`/`StaticAnalysis`/`DelegatedRun`/`Benchmark`) require a host
  verification binding, and a binding requires the capability
  (`src/lib.rs:316-329`); reserved `source_trust` key and duplicate context/
  result key rejection on merge (`src/lib.rs:330-352`); then delegates to
  `EvidenceObservation::finalize` for v2 validation/digest
  (`src/lib.rs:353-365`).
- `verification_digest` (`src/lib.rs:215-230`): nonzero schema version, bounded
  namespace text, recursive payload bound check (`MAX_VERIFICATION_SPEC_DEPTH =
  32`, `NODES = 4096`, strings to 4000 chars, arrays/objects to 512 entries;
  `src/lib.rs:244-283`), then delegates to
  `eggplan-core::verification_digest` for canonical-JSON domain-separated
  hashing. Matches [provider SPI](provider-spi.md) lines 24-25.
- Generic metadata hygiene (`src/lib.rs:368-412`): entry-count and length
  bounds plus a denylist — sensitive key substrings (`token`, `password`,
  `secret`, `credential`, `authorization`, `endpoint`) and credential/endpoint-
  like values (`://`, `sk-`, `bearer `, `token=`/`password=`/`secret=`/`api_key=`).
  This is a denylist at the generic layer; safe-listing happens per adapter
  (see findings).

## 3. Eggwork adapter (`src/eggwork.rs`)

Pinned review SHA `faaa0b9…` (`src/eggwork.rs:16`); schema-version 1 only,
1 MiB snapshot/artifact byte cap, max 64 artifacts (`src/eggwork.rs:17-18`,
`src/eggwork.rs:136-155`, `src/eggwork.rs:340-352`). Descriptor is
`epp_eggwork` / `Execution` / `Command,Test,DelegatedRun,Benchmark` with
in-progress + artifacts + verification binding, no research-trust
(`src/eggwork.rs:19-24`, `src/eggwork.rs:120-134`).

| Eggwork fact (as implemented) | Eggplan outcome |
|---|---|
| `Accepted,Preparing,Running,Cancelling` (with or without result) | `InProgress` (`src/eggwork.rs:228-241`) |
| `Succeeded` + result, state agreement | `Passed` (`src/eggwork.rs:243`) |
| `Failed,TimedOut` + result | `Failed` (`src/eggwork.rs:244`) |
| `Cancelled` + result | `Skipped` (`src/eggwork.rs:245`) |
| `Interrupted` + result, or terminal state without result | `Inconclusive` (`src/eggwork.rs:242,246`) |
| Snapshot/result state mismatch | hard error, no observation (`src/eggwork.rs:222-226`) |
| `finalization_failure` present | override to `Inconclusive` (`src/eggwork.rs:321-322`) |
| `SandboxResult::Failed` or any resource `LimitExceeded` | override to `Failed` (`src/eggwork.rs:307-327`) |

Bounds as implemented: snapshot/result state agreement, positive generation,
execution-ID charset/length, matching artifact execution/generation, unique
artifact IDs, 64-hex lowercase digests, confined relative paths (no absolute,
backslash, NUL, empty/`.`/`..` segments), `kind == "File"` only, expiry sanity,
and artifact-count match (`src/eggwork.rs:162-221`). Unknown
snapshot/result/artifact fields and enum variants fail closed via
`deny_unknown_fields` + strict enums (`src/eggwork.rs:76-118`; exercised in
unit tests at `src/eggwork.rs:405-430`, `src/eggwork.rs:489-534`).
Verification binding is required before any normalization
(`src/eggwork.rs:176-178`); metadata keeps IDs, generation, terminal state,
exit/failure classes, byte counts, cleanup-warning presence, sandbox/resource
outcome labels, and digest-bound artifact refs — never stdout/stderr text,
environment, credentials, leases, timestamps, or raw reason prose
(`src/eggwork.rs:248-306`; reason strings are mapped to outcome labels at
`src/eggwork.rs:279-305`).

## 4. Eggsearch adapter (`src/eggsearch.rs`)

Pinned review SHA `dfa90e0…` (`src/eggsearch.rs:17`); 2 MiB bundle cap, max 200
sources / 100 fetched / 128 gaps (`src/eggsearch.rs:18-21`). Descriptor is
`epp_eggsearch` / `Research` / `Research`-only, no in-progress, no verification
binding, research-trust required (`src/eggsearch.rs:23-28`,
`src/eggsearch.rs:137-146`). Consumes a strict DTO subset (`Bundle` at
`src/eggsearch.rs:117-135`); goals, URLs, snippets, fetched text, and warning
prose are structurally unselectable and never copied to metadata.

| Bundle condition (as implemented) | Eggplan status |
|---|---|
| No sources | `Unavailable` (`src/eggsearch.rs:383-384`) |
| Nonempty, no gaps/truncation/unfetched | `Passed`, operation only (`src/eggsearch.rs:385-392`) |
| Any gap, any limit/fetch truncation, or any unfetched item | `Inconclusive` (`src/eggsearch.rs:376-389`) |

Trust-marker semantics: `source_trust` is always `Some(ExternalUntrusted)`
(`src/eggsearch.rs:398-407`), even when the bundle contains `LocalTrusted`
sources (counted at `src/eggsearch.rs:306-323` as `trust_local_count`).
`LocalTrusted` is content provenance only; it never enrolls provider authority
and never flips the marker (comment at `src/eggsearch.rs:381-382`; asserted in
tests at `src/eggsearch.rs:574-608`). `Passed` claims only a structurally valid
bundle production, never truth of external claims — matching
[provider SPI](provider-spi.md) lines 36-42 and [Eggsearch
adapter](eggsearch-adapter.md) lines 12-15, 24-25. Identity handling:
deterministic `sha256:` digests over sorted source/fetch/provider IDs
(`src/eggsearch.rs:231-260`, `src/eggsearch.rs:506-510`); duplicate fetch IDs,
unknown link/fetch/gap references, bad line ranges, oversized enums, and limit
violations all fail (`src/eggsearch.rs:162-196`, `src/eggsearch.rs:410-487`).
Artifact is a handle only — `eggsearch:bundle:<id>`, body stays host-native
(`src/eggsearch.rs:393-397`).

## 5. Fixture / baseline strategy

- `tests/fixtures/manifest.json:1-27` records sibling SHAs as fixture/review
  provenance (`fixture_only: true`) and lists native fixtures
  (`eggwork-snapshots.json`, `eggwork-artifacts.json`, `eggsearch-bundles.json`).
  Unit tests consume the native fixtures directly (`src/eggwork.rs:464-486`,
  `src/eggsearch.rs:643-665`); `tests/conformance.rs:64-80` consumes the
  synthetic corpus. No sibling crate is imported — these are contract fixtures,
  not live integration, per [provider SPI](provider-spi.md) lines 51-57.
- `tests/fixtures/synthetic-results.json:1-17` models Eggwork states, Eggsearch
  trust/gap shapes, and Eggbench verdicts as a stable status-mapping table for
  conformance tests. It is a documentation-adjacent contract aid, not evidence
  of sibling behavior.

## 6. Review findings

Strengths: fail-closed DTOs with byte/count/charset/path/digest bounds on both
adapters; safe-listed metadata (explicit `insert` lists at
`src/eggwork.rs:248-306` and `src/eggsearch.rs:197-380`) with content fields
structurally excluded; reason/detail prose reduced to outcome labels
(`src/eggwork.rs:279-305`); verification binding enforced for all
execution-derived kinds including `StaticAnalysis` (`src/lib.rs:316-323` plus
`src/eggwork.rs:176-178`); trust-marker discipline on the research path with
deterministic digests preserving auditability without copying content.

Gaps and risks:

- `synthetic-results.json:8` maps Eggwork `timed_out` to `unavailable`, but the
  implementation maps `TimedOut` to `Failed` (`src/eggwork.rs:244`) and
  [Eggwork adapter](eggwork-adapter.md) lines 16-18 agrees with the code. The
  fixture row is stale relative to both; confirm whether the conformance test
  actually asserts that row or skips Eggwork-detail cases.
- `synthetic-results.json:10` models an Eggsearch local bundle with
  `source_trust: provider_trusted`, but `normalize` always emits
  `external_untrusted` (`src/eggsearch.rs:398-407`) and [Eggsearch
  adapter](eggsearch-adapter.md) lines 12-15 mandates that. Either the fixture
  models a hypothetical the adapter intentionally refuses, or the row predates
  the trust-marker decision — worth a fixture comment or correction.
- `manifest.json:7` pins Eggbench `d7d1fd9…`, while the subsystem roadmap M003
  section cites a recheck at `d870512a…` with manifest v2 separating execution
  status from comparison verdict. The manifest is therefore stale for the
  stated M003 readiness baseline, and no Eggbench adapter exists yet
  (`src/lib.rs:8-9` exports only `eggwork`/`eggsearch`) despite Eggbench rows
  in both fixture files. Scope the next fixture refresh to M003 planning.
- Generic-layer metadata protection is a denylist (`src/lib.rs:384-412`):
  substring key matching plus `://`/`sk-`/credential-prefix value heuristics.
  Adequate as defense-in-depth behind per-adapter safe-listing, but it misses
  secret shapes without those markers (bare high-entropy tokens, PEM blocks)
  and over-matches benign values containing `://`. Do not rely on it as the
  primary control; keep adapters safe-listing (they do today).
- `Bundle` (`src/eggsearch.rs:117-135`) has no `deny_unknown_fields`, unlike
  every Eggwork DTO (`src/eggwork.rs:76-118`). Unknown bundle fields are
  silently ignored, which matches "ignores unselected fields" in [Eggsearch
  adapter](eggsearch-adapter.md) line 5 but weakens unknown-variant
  fail-closure relative to Eggwork. Confirm this asymmetry is intentional and,
  if so, note which unknown inputs must still fail (today: trust/gap/link
  enums via strict deserialization, exercised at `src/eggsearch.rs:631-640`).
- Roadmap status text is self-inconsistent: header says "M002 closed"
  (`plans/subsystems/eggstack-integration-roadmap.md:3`) while the M002
  section says "ready for handoff" (`plans/subsystems/eggstack-integration-roadmap.md:96-98`).
  The roadmap narrative (section 3, Eggwork) also suggests mapping "bounded
  stdout/stderr" into observations, while the implementation and [Eggwork
  adapter](eggwork-adapter.md) lines 25-30 deliberately retain only byte
  counts. Clarify the roadmap wording so a future reader does not regress the
  content-omission boundary.
- Status-space coverage is intentionally narrow: the SPI carries eight statuses
  ([provider SPI](provider-spi.md) lines 28-34) but Eggwork only emits five
  and Eggsearch only three. `NotRun`/`Blocked`/`Unavailable` (Eggwork) can
  never appear by construction today. That preserves the no-collapse rule but
  means future native "not run / blocked" states need explicit mapping work
  rather than falling out of the current tables.

## Verification pointers

- `cargo test -p eggplan-integrations` (unit + `tests/conformance.rs`
  fixture-driven status/trust mapping).
- `bash scripts/check-integrations-boundary.sh` (no sibling/transport/async/
  process deps; no `register_trusted`/`ProviderRegistry` in adapters).
- `cargo clippy -p eggplan-integrations --all-targets --locked -- -D warnings`
  for the crate-focused lint gate.
