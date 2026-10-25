# Eggstack Integrations M004 C001 — Fixture Inventory and Design-Record Citation Truthfulness

Status: closed

Repository baseline: ee54a0b8e2cb0d843565c54930ad2c21111320db

Source roadmap:

- plans/subsystems/eggstack-integration-roadmap.md

Long-term requirements:

- plans/000-long-term-specification.md (the adapter SPI normalizes; it never
  acquires evidence or enrolls trust)

Related ADRs: ADR-0002, ADR-0004.

Predecessor plan/closure:

- plans/implementation/eggstack-integration/003a-eggbench-verified-bundle-and-comparison-evidence.md
- plans/closure/eggstack-integration/003a-closed.md
- plans/closure/foundation-core/003-c001-closed.md
- plans/closure/foundation-core/003-c003-closed.md

Primary class: documentation truthfulness / fixture metadata hygiene

## 1. Objective

Make the integrations fixture manifest name every fixture it governs, and make
the integrations design record point at the code it describes.

Two findings, both recorded in earlier closure records and both still open:

- `plans/closure/eggstack-integration/003a-closed.md:291-296` records that
  `native_fixtures` in the fixture manifest is not exhaustive.
- `plans/closure/foundation-core/003-c001-closed.md` (finding 3) and
  `plans/closure/foundation-core/003-c003-closed.md` (finding 4) record
  citation drift in `architecture/deep-dive-integrations.md`.

The citation drift is larger than either record states. The recorded scope was
two `crates/eggplan-core/src/schema.rs` lines; the actual scope is 23 wrong
citations across six files, and it includes the pointers that locate the
SPI's trust and verification-binding enforcement.

This pass changes one fixture manifest and one design record. No production
Rust source is edited, no digest changes, and no golden byte changes.

## 2. Findings

### E-ES-C001-01 — the fixture manifest's inventory is not exhaustive

`crates/eggplan-integrations/tests/fixtures/manifest.json:22-26` lists three
native fixtures:

    "native_fixtures": [
      "eggwork-snapshots.json",
      "eggwork-artifacts.json",
      "eggsearch-bundles.json"
    ],

The fixture directory holds thirteen tracked files. Seven more are covered by
a second, differently-shaped array, `native_fixture_families`
(`:27-134`), which carries per-family upstream provenance. Two are in neither:

- `verification-digest.json`
- `synthetic-results.json`

The recorded omission set is out of date: it names `verification-digest.json`
and "the two Eggbench files", but the two Eggbench files have been covered by
`native_fixture_families` since commit `674264d`.

Nothing reads `native_fixtures`. The only consumer of the manifest is
`crates/eggplan-integrations/tests/conformance.rs:580`, which parses it into a
`serde_json::Value` and asserts three unrelated fields. No code hashes the
manifest's bytes, and the integrations crate has no `.sha256` sidecars. The
cost is reader cost: a reviewer who treats `native_fixtures` as the inventory
under-counts the corpus.

The sibling manifests make the convention explicit.
`crates/eggplan-markdown/tests/fixtures/manifest.json` lists exactly the three
`.md` files in its directory and
`crates/eggplan-markdown/tests/fixtures.rs:16` asserts the count.
`crates/eggplan-codegg-compat/tests/fixtures/manifest.json` does the same
for its three `.json` files, asserted at
`crates/eggplan-codegg-compat/tests/parity.rs:269`. The integrations manifest
is the only one of the three that is neither exhaustive nor length-asserted.

### E-ES-C001-02 — the integrations design record's citations are stale

`architecture/deep-dive-integrations.md` carries 126 citation tokens, 114
distinct. 23 of them do not point at the content their claim describes.

The root cause is not the Foundation M003 corrective work, as the recorded
findings assume. `git log -- crates/eggplan-core/src/schema.rs` shows its last
change was `37ad290`, which predates `1e33069`, and
`git show 1e33069 --stat -- crates/eggplan-core/src/schema.rs` confirms the
M003 commit does not touch it. The citations were exactly correct at `60a5f92`,
when the deep dive was written. `37ad290` — "fix: resolve every actionable
finding in the crate-boundary audit" — performed a behaviour-changing refactor
across `crates/eggplan-core/src/schema.rs` and
`crates/eggplan-integrations/src/eggsearch.rs` and did not update the deep
dive.

The M004 wave (`a60d36b`) repaired four sibling deep dives for exactly this
reason — the registry records the standard at `plans/registry.md:375-377`:
"0 out of range, 0 unresolved, every resolved range checked against the claim
it supports" — and omitted this one.

The 23 wrong citations, by file:

`crates/eggplan-core/src/schema.rs` — 2. The domain literal and the 65,536
byte cap moved when `37ad290` extracted `VERIFICATION_MAX_BYTES`. Correct
ranges: `crates/eggplan-core/src/schema.rs:101-107` for the domain literal,
`crates/eggplan-core/src/schema.rs:22` for the
constant and `crates/eggplan-core/src/schema.rs:108-110` for the cap check.

`crates/eggplan-integrations/src/eggsearch.rs` — 14. The file's pinned review
SHA, its four cap constants, the gap/truncation status mapping, the trust
marker, the ID-charset validation, the plain SHA-256 digest helper, and three
test anchors all moved. The corrected ranges are recorded in the closure
evidence for this plan and must each be read before it is written down.

`crates/eggplan-integrations/tests/conformance.rs` — 2. The corpus test
spans `:579-651`, with the `finalize_observation` loop at `:638-649`.

`scripts/check-integrations-boundary.sh` — 2. These cite the *review-time*
shape of the script, when it scanned two adapter files. The script was
rewritten into six scope-labelled guards that scan the whole `src` tree, and
the deep dive's own §7 records that rewrite. The §1 paragraph must therefore
be scoped explicitly to the review-time state, exactly as the deep dive
already scopes its `crates/eggplan-integrations/src/lib.rs:8-9` citation, and the §6 finding about the
adapter-file-scoped trust guard must be marked as resolved by §7.

Doc-to-doc citations — 3. The Eggstack roadmap's status line and its M002
section, and the registry's Eggstack row, all point at the wrong ranges.

## 3. Controlling semantics

- A design record's `path:line` citation is a claim that the content at that
  range supports the sentence it accompanies. A stale citation is a false
  statement in the design record, not a cosmetic defect.
- A live design record is corrected in place. This differs from a historical
  closure record, whose conclusions are preserved and whose later
  corrections are appended as dated errata
  (`.skills/eggplan-milestone/SKILL.md:63-66`).
- A fixture manifest that names a subset of its fixtures invites a reader to
  treat the subset as the whole. Either it names every fixture, or it says
  what its scope is.
- The two arrays must not be able to drift again. Whichever shape is chosen,
  the manifest must make the union explicit.

## 4. Scope

In scope:

- `crates/eggplan-integrations/tests/fixtures/manifest.json`
- `architecture/deep-dive-integrations.md`
- `architecture/deep-dive-tooling-governance.md`, only for the citations the
  integration deep dive's repair moves and only if the repair moves them

Out of scope:

- any production Rust source;
- any fixture file's contents — no fixture may change, because the fixture
  corpus is contract evidence;
- `crates/eggplan-integrations/tests/conformance.rs` — the manifest's shape
  may change only in a way that keeps its three existing assertions true;
- `plans/`.

## 5. Required production changes

1. `native_fixtures` must name every fixture file in
   `crates/eggplan-integrations/tests/fixtures/` that is not itself the
   manifest, or the manifest must state unambiguously that
   `native_fixture_families` carries the remaining files and that the union
   of the two arrays is the complete inventory.
2. The manifest must make the union checkable rather than asserted.
3. Every one of the 23 stale citations in
   `architecture/deep-dive-integrations.md` must be corrected to the range
   that supports its claim, each verified by reading the file.
4. The §1 boundary-script paragraph must be scoped to the review-time state
   of the script, and the §6 trust-guard finding must be marked as resolved
   by the §7 rewrite.
5. The three stale doc-to-doc citations must be corrected.

## 6. Failure, cancellation, restart, and contention semantics

Both files are read-only artifacts. Nothing in this pass writes state,
acquires a lock, or touches the repository.

## 7. Required tests

- The integrations conformance suite must stay green, which proves the
  manifest's three existing assertions still hold after the change.
- A check that the fixture directory's file set and the manifest's union
  agree, so the inventory cannot drift again. It may be a test or a
  documented rule; the closure evidence must state which.

## 8. Verification

    cargo fmt --all -- --check
    cargo check --workspace --all-targets --locked
    cargo clippy --workspace --all-targets --locked -- -D warnings
    cargo test --workspace --locked
    cargo test -p eggplan-integrations --locked
    bash scripts/check-core-boundary.sh
    bash scripts/check-codegg-compat-boundary.sh
    bash scripts/check-integrations-boundary.sh
    bash scripts/check-projection-cli-boundary.sh
    bash scripts/check-closure-authority-boundary.sh
    cargo +1.89.0 check --workspace --all-targets --locked
    cargo +1.89.0 test --workspace --locked
    git diff --check

The citation audit must additionally be run and reported: every citation in
the repaired file, its resolved range, and whether the content at that range
supports the claim.

## 9. Acceptance criteria

1. The fixture manifest names every fixture file in its directory, or states
   unambiguously that the union of its two arrays is the complete inventory.
2. No fixture file's bytes change, and no digest sidecar changes.
3. Every citation in `architecture/deep-dive-integrations.md` resolves in
   range, and the content at each range supports the claim it accompanies.
4. The §1 boundary-script paragraph and the §6 trust-guard finding are
   correctly scoped to history.
5. The three doc-to-doc citations are corrected.
6. The integrations test suite is green.
7. No production Rust source is edited.

## 10. Stop conditions

- A citation cannot be corrected without changing the sentence it supports,
  which would mean the design record's claim — not just its pointer — is
  wrong.
- The manifest's shape cannot be changed without altering a fixture file's
  bytes or a digest.
- The repair would require editing a closed closure record's conclusions
  rather than appending an erratum.

## 11. Closure evidence required

- The full citation audit for the repaired deep dive: every citation, its
  resolved range, and the content check.
- The before/after fixture directory listing and the manifest diff.
- The integrations and workspace test results.
- The coordinator's verification table, with "not run" rows recorded honestly.
- An explicit statement that no fixture byte, digest, schema version, or
  persisted byte changed.

## 12. Handoff notes

- This plan is **ready**. Both findings are reproduced, the citation drift's
  true scope and root cause are identified, and the recorded root-cause
  attribution is corrected here.
- The recorded omission set for the fixture manifest is out of date; the plan
  states the current one.
- `plans/closure/eggstack-integration/003a-closed.md:291-296` and
  `plans/closure/foundation-core/003-c001-closed.md` (finding 3) and
  `plans/closure/foundation-core/003-c003-closed.md` (finding 4) record these
  findings. Their conclusions are not changed by this pass; the findings
  become closed by the closure record this plan produces.
