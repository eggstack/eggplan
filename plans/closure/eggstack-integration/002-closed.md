# Eggstack Integrations M002 — Closed

Status: closed

Source implementation plan:

- plans/implementation/eggstack-integration/002-eggwork-and-eggsearch-evidence-adapters.md

Source roadmap:

- plans/subsystems/eggstack-integration-roadmap.md

Reviewed Eggplan baseline: `cc65a3e3faf9012e9903b68ee9834f937e7cb126`

Reviewed sibling interfaces:

- Eggwork `eggstack/eggwork` @ `faaa0b905fa6bc43e46825fdd98530b5533a970f`
- Eggsearch `eggstack/eggsearch` @ `dfa90e050c5434f3346902aeb4074901c58e90d1`

Implementation commits:

- `3a836ddf614e75fbe1f49d7876e08c9f3da4ce57` — bounded Eggwork and
  Eggsearch adapters, fixtures, docs, and boundary checks.
- `bf3db0f189db4cde6f219f9ae878d41a6004c990` — Eggwork sandbox/resource,
  duplicate artifact, and generation-mismatch regression coverage.

Hosted qualification: GitHub Actions run
[36039972370](https://github.com/eggstack/eggplan/actions/runs/36039972370)
passed on the final implementation commit.

## Executive finding

Eggplan now normalizes host-acquired Eggwork execution snapshots/artifacts and
Eggsearch evidence bundles through the M001 SPI. Both adapters use bounded
DTOs; neither imports a sibling runtime nor performs execution, search, network,
MCP, or credential acquisition. Provider identities are adapter-fixed and
provider trust remains an explicit host registry decision.

## Requirement-to-evidence matrix

| Requirement | Evidence |
|---|---|
| Eggwork native execution states | `eggwork::tests::states_map_and_failures_remain_distinct` covers Accepted/Preparing/Running/Cancelling, Succeeded, Failed, TimedOut, Cancelled, Interrupted, and terminal-without-result. Snapshot/result disagreement and unknown states reject. |
| Eggwork verification authority | Execution kinds require `ObservationContext.verification_digest`; it is copied from the host and never derived from execution IDs, exit codes, or returned command text. Missing binding is tested as a typed SPI error. |
| Eggwork artifacts and bounded provenance | Fixture artifact IDs must match execution/generation; duplicate IDs, bad digests, path escape, count mismatch, and mismatched generations reject. References preserve opaque artifact ID and SHA-256 digest. |
| Eggwork sensitive-data omission | Output bytes/omitted counts are retained without output content. Sandbox/resource messages, cleanup warning prose, environment, credentials, lease tokens, and arbitrary diagnostics are not persisted. Failure outcome classes remain visible. |
| Eggsearch bundle normalization | Six bounded bundle fixtures cover external web, LocalTrusted label, degraded provider, all-external gap, truncation, and empty/unavailable. Source/fetch/provider identities are represented by deterministic digests; link and line-range counts, trust counts, bundle handle, and stable gap codes are retained. |
| Eggsearch trust non-escalation | The fixed descriptor uses `epp_eggsearch`; source `LocalTrusted` remains only a content-provenance count and normalized `source_trust` stays `external_untrusted`. No adapter mutates or enrolls a ProviderRegistry. |
| Eggsearch content omission | URL, title, snippet, goal, fetched text, warning prose, and research claim/conflict bodies are excluded. Negative fixture tests assert private fixture values are absent from the serialized observation. |
| Status semantics | Eggsearch `Passed` describes only successful production of a structurally valid bundle under the adapter profile; it does not assert source truth. Gaps, fetch failures, or truncation are Inconclusive; no-source is Unavailable. Eggwork finalization failure is Inconclusive, sandbox/resource failure is Failed. |
| Sibling contract and dependency boundary | Bounded serialized fixtures are pinned to the reviewed sibling SHAs. `eggplan-integrations` adds no sibling/runtime dependency. Boundary guard rejects sibling imports, async/network/process APIs, and adapter trust enrollment. |

## Exact local verification

All commands ran on Linux at final implementation commit
`bf3db0f189db4cde6f219f9ae878d41a6004c990` and passed:

```text
rtk cargo fmt --all -- --check
rtk cargo check --workspace --all-targets --locked
rtk cargo clippy --workspace --all-targets --locked -- -D warnings
rtk cargo test --workspace --locked                 # 126 passed
rtk cargo test --workspace --doc --locked           # 3 compile-fail doctests passed
rtk cargo +1.89.0 check --workspace --all-targets --locked
rtk cargo +1.89.0 test --workspace --locked         # 126 passed
rtk bash scripts/check-core-boundary.sh
rtk bash scripts/check-codegg-compat-boundary.sh
rtk bash scripts/check-integrations-boundary.sh
rtk bash scripts/check-projection-cli-boundary.sh
rtk bash scripts/check-closure-authority-boundary.sh
rtk git diff --check
```

Hosted run `36039972370` passed:

- Linux job `107769411542`: format, check, clippy, tests, and all five
  boundary guards.
- macOS job `107769411593`: check, clippy, tests, and all five boundary
  guards; formatting is skipped by workflow policy on macOS.
- Windows job `107769411549`: check, clippy, tests, and all five boundary
  guards; formatting is skipped by workflow policy on Windows.
- Rust 1.89 job `107769411216`: workspace check and tests.

## Invariant and compatibility review

- Eggplan remains a planning/evidence mechanism. Eggwork execution and
  Eggsearch acquisition remain host-owned.
- Eggwork verification binding is host-supplied; no execution identity or
  result prose can forge it.
- Eggsearch content trust is separate from operation status. A Passed Research
  observation records successful bundle production and preserves
  `external_untrusted`; it is not a claim that external content is true.
- The SPI's source-trust marker is provenance metadata. Host provider
  enrollment remains explicit and independent.
- Artifact/bundle handles are bounded references; large native payloads remain
  outside Eggplan observation metadata.
- No schema migration or sibling runtime dependency was needed.

## Roadmap and dependency disposition

Eggstack M002 is closed. Eggbench was rechecked at
`d870512a5a1af16276ff05286ff0b6e2366b7f8f`; current manifest v2 separates
execution status from comparison verdict. Eggstack M003 is ready for planning
against that interface.

Projection/CLI M003 remains roadmap-level until real repositories use its
Markdown surface. CodeGG M002 remains blocked only by the separately registered
upstream attempt-scoped execution-subject provenance handoff. Interop and
distribution remain deferred while that local capability work is outstanding.

## Unresolved findings

None.
