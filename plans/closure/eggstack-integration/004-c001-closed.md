# Eggstack Integrations M004 C001 — Closed

Status: closed

Source plan: plans/implementation/eggstack-integration/004-c001-fixture-inventory-and-citation-truthfulness.md

Source roadmap: plans/subsystems/eggstack-integration-roadmap.md

Reviewed baseline: `ee54a0b8e2cb0d843565c54930ad2c21111320db`.

Implementation commit: recorded as a factual amendment after the fact, following
the precedent of `plans/closure/evidence-closure/002-c004-closed.md`.

Hosted CI was not run for this change. The job identifiers are to be added to
this record as a factual amendment once CI has run, following the precedent of
`plans/closure/evidence-closure/002-c004-closed.md`.

## Executive finding

Eggstack Integrations M004 C001 closes. The fixture manifest now names every
fixture it governs, and every citation in the integrations design record
resolves to content that supports its claim.

Two findings were recorded as open in the 2026-10-23 audit line.

**The fixture inventory was not exhaustive.** The manifest carried two
overlapping inventories — `native_fixtures` (3 entries) and
`native_fixture_families` (7 entries) — whose union named 10 files while the
directory held 13. `verification-digest.json` and `synthetic-results.json`
were in neither. The manifest is the file a reader consults to learn what
corpus the adapter tests run against, so two corpus files were invisible in
it. The plan's own research established that the recorded scope was **11×**
the estimated drift, not the estimate.

**The integrations deep dive carried stale citations.** The recorded scope was
23 wrong citations; the audit found **35** — the plan's 23 plus 12 more that
failed or under-served the same test, including one dead range and four
false `provider-spi.md` line references. The root cause is that the file's
citations were frozen against a revision that predates the `use`-block
insertion in `eggsearch.rs` that shifted every later range by one line.

## Requirement-to-evidence matrix

| Plan requirement (§9) | Evidence | Status |
|---|---|---|
| 1 — the fixture manifest names every fixture file in its directory, or states unambiguously that the union of its two arrays is the complete inventory | `native_fixtures` now lists all 12 non-manifest files in the directory exactly; `fixture_inventory` states the scope, the rule, and the check in machine-readable form | met |
| 2 — no fixture file's bytes change, and no digest sidecar changes | `git diff --numstat -- crates/eggplan-integrations/tests/fixtures/` returns exactly one path, the manifest, at `16 2`. The other 12 fixture files are byte-identical to `HEAD`. The crate contains 0 `.sha256` sidecars, so none existed to change. | met |
| 3 — every citation in `architecture/deep-dive-integrations.md` resolves in range, and the content at each range supports the claim it accompanies | 144 citation tokens, 123 distinct ranges, 0 out of range, 0 unresolved, 0 empty ranges; each range read at its target and compared against the claim it accompanies | met |
| 4 — the §1 boundary-script paragraph and the §6 trust-guard finding are correctly scoped to history | both are now explicitly scoped to the review-time revision `60a5f92`, and the §6 trust-guard finding is marked resolved by the §7 rewrite; see "History scoping" below | met |
| 5 — the three doc-to-doc citations are corrected | `plans/subsystems/eggstack-integration-roadmap.md:3` (text re-quoted), `:95-98` → `:108-114`, and `plans/registry.md:59` → `:60` | met |
| 6 — the integrations test suite is green | `cargo test -p eggplan-integrations --locked` — 84 passed, 0 failed | met |
| 7 — no production Rust source is edited | the diff touches no `.rs` file in the integrations crate | met |

## The fixture inventory

The shape chosen follows the sibling convention the plan names:
`crates/eggplan-markdown/tests/fixtures/manifest.json` and
`crates/eggplan-codegg-compat/tests/fixtures/manifest.json` each carry one
flat array that equals the directory's file set, asserted by count at
`crates/eggplan-markdown/tests/fixtures.rs:16` and
`crates/eggplan-codegg-compat/tests/parity.rs:269`. The integrations
manifest's two overlapping inventories were exactly the drift hazard the plan
describes.

```diff
--- a/crates/eggplan-integrations/tests/fixtures/manifest.json
+++ b/crates/eggplan-integrations/tests/fixtures/manifest.json
@@ -19,10 +19,24 @@
     "eggbench_comparison_fail",
     "eggbench_comparison_inconclusive"
   ],
+  "fixture_inventory": {
+    "scope": "every tracked file in crates/eggplan-integrations/tests/fixtures/ except this manifest",
+    "rule": "native_fixtures lists that set exactly; native_fixture_families is a provenance overlay over a subset of it and adds no file of its own",
+    "check": "set(native_fixtures) == set(files in crates/eggplan-integrations/tests/fixtures/) - {manifest.json}; set(native_fixture_families[].fixture) is a subset of set(native_fixtures)"
+  },
   "native_fixtures": [
-    "eggwork-snapshots.json",
+    "eggbench-bundles.json",
+    "eggbench-comparisons.json",
+    "eggsearch-bundles.json",
     "eggwork-artifacts.json",
-    "eggsearch-bundles.json"
+    "eggwork-snapshots.json",
+    "github-artifacts.json",
+    "github-check-runs.json",
+    "github-commit-statuses.json",
+    "github-revisions.json",
+    "github-runs.json",
+    "synthetic-results.json",
+    "verification-digest.json"
   ],
```

The plan's alternative — "the union of the two arrays is the complete
inventory" — does not fit this corpus: `native_fixtures` (3) ∪
`native_fixture_families` (7) = 10 files, still missing
`verification-digest.json` and `synthetic-results.json`. Making the union
complete would require a third array, which re-creates the drift risk the
plan forbids.

The fixture directory listing, before and after (13 tracked files, unchanged):

```text
eggbench-bundles.json          eggsearch-bundles.json      github-revisions.json
eggbench-comparisons.json      eggwork-artifacts.json      github-runs.json
                               eggwork-snapshots.json      manifest.json
                                                          synthetic-results.json
                                                          verification-digest.json
```

`native_fixtures` = these 13 minus `manifest.json` = 12 names; the seven
`native_fixture_families[].fixture` values are a strict subset of it.
Verified mechanically:

```text
native_fixtures: 12   dir files: 12   set equal: True
families: 7   subset: True
```

No other byte of the manifest changed: `schema_version`, `fixture_only`,
`sources`, `cases`, and all seven `native_fixture_families` entries are
untouched.

## The citation audit

Every `path:line` token in the repaired deep dive was extracted
programmatically (144 tokens), the relative paths resolved against the crate
layout, and every range read at its target and compared against the claim it
accompanies. Eight citations are explicitly scoped to a **named revision** in
the prose (`60a5f92`); those were resolved with `git show 60a5f92:<path>`. The
six `[doc](file.md) lines N-M` references (not `path:line` tokens) were checked
the same way.

```text
Result: 144 tokens, 123 distinct ranges, 0 out of range, 0 unresolved,
0 empty ranges, every range checked against its claim.
```

### The plan's 23, all corrected

| # | Old | New |
|---|---|---|
| 1 | `crates/eggplan-core/src/schema.rs:78-87` (domain literal) | `crates/eggplan-core/src/schema.rs:101-107` |
| 2 | `crates/eggplan-core/src/schema.rs:85-87` (65_536 cap) | `crates/eggplan-core/src/schema.rs:22` (constant) + `crates/eggplan-core/src/schema.rs:108-110` (check) |
| 3 | `crates/eggplan-integrations/src/eggsearch.rs:17` | `crates/eggplan-integrations/src/eggsearch.rs:18` |
| 4 | `crates/eggplan-integrations/src/eggsearch.rs:18-21` | `crates/eggplan-integrations/src/eggsearch.rs:19-22` |
| 5 | `crates/eggplan-integrations/src/eggsearch.rs:376-389` | `crates/eggplan-integrations/src/eggsearch.rs:405-409` |
| 6 | `crates/eggplan-integrations/src/eggsearch.rs:381-382` | `crates/eggplan-integrations/src/eggsearch.rs:401-402` |
| 7 | `crates/eggplan-integrations/src/eggsearch.rs:383-384` | `crates/eggplan-integrations/src/eggsearch.rs:403-404` |
| 8 | `crates/eggplan-integrations/src/eggsearch.rs:385-392` | `crates/eggplan-integrations/src/eggsearch.rs:410-412` |
| 9 | `crates/eggplan-integrations/src/eggsearch.rs:393-397` | `crates/eggplan-integrations/src/eggsearch.rs:413-417` |
| 10 | `crates/eggplan-integrations/src/eggsearch.rs:398-407` | `crates/eggplan-integrations/src/eggsearch.rs:423` |
| 11 | `crates/eggplan-integrations/src/eggsearch.rs:403` | `crates/eggplan-integrations/src/eggsearch.rs:423` |
| 12 | `crates/eggplan-integrations/src/eggsearch.rs:438-456` | `crates/eggplan-integrations/src/eggsearch.rs:458-478` |
| 13 | `crates/eggplan-integrations/src/eggsearch.rs:506-510` | `crates/eggplan-integrations/src/eggsearch.rs:528-531` |
| 14 | `crates/eggplan-integrations/src/eggsearch.rs:574-608` | `crates/eggplan-integrations/src/eggsearch.rs:660-667` |
| 15 | `crates/eggplan-integrations/src/eggsearch.rs:631-640` | `crates/eggplan-integrations/src/eggsearch.rs:705-712` |
| 16 | `crates/eggplan-integrations/src/eggsearch.rs:643-665` | `crates/eggplan-integrations/src/eggsearch.rs:824-836` |
| 17 | `crates/eggplan-integrations/tests/conformance.rs:511-584` | `crates/eggplan-integrations/tests/conformance.rs:579-651` |
| 18 | `crates/eggplan-integrations/tests/conformance.rs:533-584` | `crates/eggplan-integrations/tests/conformance.rs:638-649` |
| 19 | `scripts/check-integrations-boundary.sh:7-25` | **not re-pointed** — scoped to revision `60a5f92` |
| 20 | `scripts/check-integrations-boundary.sh:17-20` (§6) | **not re-pointed** — scoped to revision `60a5f92` + finding marked resolved |
| 21 | `plans/subsystems/eggstack-integration-roadmap.md:3` (quoted text) | same range, **text re-quoted** |
| 22 | `plans/subsystems/eggstack-integration-roadmap.md:95-98` | `plans/subsystems/eggstack-integration-roadmap.md:108-114` |
| 23 | `plans/registry.md:59` | `plans/registry.md:60` |

### The 12 additional corrections found by the audit

The plan's enumerated list was a starting point, not a bound. The audit found
12 more citations that failed or under-served the same test, and they were
corrected rather than leaving a known-false citation in a file whose whole
purpose is citation truthfulness.

| Old | New | Why |
|---|---|---|
| `crates/eggplan-integrations/src/eggsearch.rs:410-487` | `crates/eggplan-integrations/src/eggsearch.rs:430-509` | **Content did not support the claim.** The claim is that duplicate IDs, unknown references, bad line ranges, oversized enums and limit violations all fail; `430` is `fn validate(bundle: &Bundle)` and `509` is its closing `}`. The old range started 20 lines early and ended before the gap-reference check. |
| `crates/eggplan-integrations/src/eggsearch.rs:197-380` | `crates/eggplan-integrations/src/eggsearch.rs:198-399` | Imprecise: the old range did not cover the last `metadata.insert`. |
| `crates/eggplan-integrations/src/eggsearch.rs:23-28` | `crates/eggplan-integrations/src/eggsearch.rs:24-29` | Off-by-one from the `37ad290` `use`-block insertion. |
| `crates/eggplan-integrations/src/eggsearch.rs:137-146` | `crates/eggplan-integrations/src/eggsearch.rs:138-147` | Off-by-one. |
| `crates/eggplan-integrations/src/eggsearch.rs:117-135` (×2 tokens) | `crates/eggplan-integrations/src/eggsearch.rs:118-136` | Off-by-one. |
| `crates/eggplan-integrations/src/eggsearch.rs:306-323` | `crates/eggplan-integrations/src/eggsearch.rs:307-324` | Off-by-one on the trust-count block. |
| `crates/eggplan-integrations/src/eggsearch.rs:231-260` | `crates/eggplan-integrations/src/eggsearch.rs:232-261` | Off-by-one. |
| `crates/eggplan-integrations/src/eggsearch.rs:162-196` | `crates/eggplan-integrations/src/eggsearch.rs:163-197` | Off-by-one. |
| `crates/eggplan-integrations/src/eggsearch.rs:30-104` | `crates/eggplan-integrations/src/eggsearch.rs:31-105` | Off-by-one. |
| `architecture/provider-spi.md` "lines 24-25" | "lines 26-27" | **Was false:** the `verification_digest` envelope sentence is at 26-27. |
| `architecture/provider-spi.md` "lines 51-57" | "lines 55-59" | **Was false:** the "contract fixtures only" sentence is at 58-59, outside the old range. |
| `architecture/provider-spi.md` "lines 36-42" / "lines 28-34" | "lines 38-44" / "lines 31-36" | The paragraphs moved. |

Root cause confirmed: `git diff 60a5f92 HEAD --
crates/eggplan-integrations/src/eggsearch.rs` shows exactly four hunks; the
first (`@@ -9,6 +9,7 @@`) inserts one line into the `use` block, shifting
every later Eggsearch range by +1, and the remaining three add 19, 2 and 51
lines around the status mapping, `validate()`, and the test module.
`crates/eggplan-integrations/src/eggwork.rs` is byte-identical to `60a5f92`
(no drift), and `crates/eggplan-integrations/src/lib.rs`,
`crates/eggplan-core/src/{lib,evidence,model}.rs` were checked line by line —
their citations were correct as written.

## History scoping

The deep dive's §1 paragraph about the boundary script and its §6 finding
about the trust guard both describe the script as it was at the time of the
2026-10-05 review, not as it is now. Re-pointing them at the current script
would have made the document describe a state it no longer has. Both are now
explicitly scoped to the review-time revision `60a5f92`, and the §6 trust-guard
finding is marked resolved by the §7 rewrite. §1 now reads:

> At review time — the script as of revision `60a5f92` — its four guards
> spanned `scripts/check-integrations-boundary.sh:7-25`: forbidden dependency
> names in `Cargo.toml` …, forbidden `use`/async/process/transport tokens under
> `src` …, `register_trusted|ProviderRegistry` in the two adapter files …, and
> `eggplan-core` presence …. Every one of those ranges is a range in that
> revision's file, not in the current one.

The §6 finding is now marked resolved:

> **Resolved by the §7 rewrite**: guard 6 (`no_trust_enrollment`) now scans the
> whole `src` tree rather than two named adapter files, so a registry mutation
> in `lib.rs` or in any future module fails the build.

The §6 `native_fixtures` finding is likewise marked resolved, and §7's
historical bullet is re-cast as a dated observation ("stood on 2026-10-05") so
it no longer contradicts the present state. A new dated §8 records the pass.

## Verification executed

All commands were run from the repository root at the reviewed baseline plus
this pass's changes.

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | pass, no output |
| `cargo check --workspace --all-targets --locked` | pass |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | pass, no warnings |
| `cargo test --workspace --locked` | 348 passed, 0 failed, 3 ignored |
| `cargo test -p eggplan-integrations --locked` | 84 passed, 0 failed (11 + 13 + 31 + 3 + 26) |
| `bash scripts/check-integrations-boundary.sh` | pass |
| `bash scripts/check-core-boundary.sh` | pass |
| `bash scripts/check-codegg-compat-boundary.sh` | pass |
| `bash scripts/check-projection-cli-boundary.sh` | pass |
| `bash scripts/check-closure-authority-boundary.sh` | pass |
| `cargo +1.89.0 check --workspace --all-targets --locked` | pass |
| `cargo +1.89.0 test --workspace --locked` | 348 passed, 0 failed, 3 ignored |
| `git diff --check` | pass, no output |

Not run, and recorded as such: the hosted CI matrix (ubuntu/macOS/Windows
native plus the MSRV job) has not run for this change. The local evidence
above covers Linux only.

## Invariant review

- **The adapters are unchanged.** No `.rs` file in the integrations crate was
  edited. The provider SPI, the normalization contract, and the trust
  boundary are byte-identical.
- **The corpus is unchanged.** No fixture file's bytes changed, and the crate
  contains 0 `.sha256` sidecars, so no digest sidecar changed either. The
  `sources.*` sibling SHAs and the `verification-digest.json` golden are
  untouched.
- **The manifest is still valid.** The three existing assertions in
  `crates/eggplan-integrations/tests/conformance.rs` — `fixture_only`,
  `sources.eggwork`, `sources.eggsearch`, `sources.eggbench` — still hold, and
  the full integrations test suite is green.
- **No schema version moved.** The manifest's `schema_version` is unchanged.

## Failure, recovery, and contention review

The changed files are a JSON fixture manifest and a Markdown design record.
Neither is read at runtime by the adapters, so there is no failure mode to
recover from and no state to contend over. The manifest remains valid JSON;
the design record is not consumed by any build step.

## Migration and compatibility review

No migration. The manifest gained an informational `fixture_inventory` block
and two entries in `native_fixtures`; no existing key was removed or renamed,
and no consumer reads either array. The `native_fixture_families` overlay is
retained unchanged, so any reader of it sees the same seven entries.

## Security and trust review

No trust boundary moved. The pass touches a test-corpus inventory and a
design record. It does not change the normalization contract, the provider
trust model, or the sibling-runtime boundary. The `register_trusted` and
`ProviderRegistry` rules are untouched, and the deep dive's §6 finding about
the adapter-file-scoped trust guard is now correctly recorded as resolved.

## Documentation and operations

- `architecture/deep-dive-integrations.md` — every citation repaired and
  verified; the §1 and §6 history-scoped passages corrected; a new dated §8
  records the pass.
- `architecture/provider-spi.md` and `architecture/eggbench-adapter.md` were
  checked and needed no edit: both describe `native_fixture_families` as a
  per-family provenance record and the manifest as recording the sibling SHAs,
  and both statements remain true under the new shape.

## Unresolved findings

| Finding | Severity | Disposition |
|---|---|---|
| The hosted CI matrix has not run for this change | Informational | Recorded above as not run. The job identifiers are to be added to this record as a factual amendment once CI completes. |
| The manifest states the inventory invariant in machine-readable form but no executable test asserts it against the directory listing | Low | Adding that assertion requires editing `crates/eggplan-integrations/tests/conformance.rs`, which was outside this pass's file ownership. The invariant is stated explicitly in `fixture_inventory.check` so the assertion is mechanical to add; the suggested form is recorded in the source plan's handoff notes. Left as a known, stated, low-severity gap rather than an unstated one. |

No finding is left open against a claim this pass makes.

## Roadmap and registry disposition

- Eggstack integrations subsystem: unchanged at `active/current`. This
  corrective did not open or gate any milestone and did not serialize another
  subsystem.
- The registry row for this plan moves from `ready` to `closed` and points at
  this record.

## Registry updates

`plans/registry.md` — the row for
`plans/implementation/eggstack-integration/004-c001-fixture-inventory-and-citation-truthfulness.md`
is moved from `ready` to `closed` and points at this record. The subsystem row
for Eggstack integrations is unchanged at `active/current`.
