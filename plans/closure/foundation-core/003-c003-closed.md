# Foundation M003 C003 — Closed

Status: closed

Source plan: plans/implementation/foundation-core/003-c003-core-assessment-reason-schema-fail-closed.md

Source roadmap: plans/subsystems/foundation-core-roadmap.md

Reviewed baseline: `37ad290ca6b971a68d2871aedb39f4882f4eb2a8`.

Implementation state: the implementation is complete in the working tree and
was not committed by the coordinator. The commit identifier is to be added to
this record as a factual amendment once the change is committed, following the
precedent of `plans/closure/evidence-closure/002-c004-closed.md`.

## Executive finding

Foundation M003 C003 closes. The nested `AssessmentReason` enum now denies
unknown fields, exactly like the four assessment structs that contain it
(`crates/eggplan-core/src/assessment.rs:41`). An injected key inside a reason
element now fails closed instead of riding along outside the coverage of a
closure record's content digest.

The support for `deny_unknown_fields` on an adjacently tagged enum was not
assumed. It was verified against the pinned serde source and then proven
empirically: all 21 `AssessmentReason` variants were round-tripped
serialize → deserialize with the strict rule in force, and each was shown to
reject an injected unknown key
(`crates/eggplan-core/src/assessment.rs:1163`). The regression test now also
enumerates reason-level injection paths, not only struct-level paths
(`crates/eggplan-core/src/closure.rs:629`).

The compatibility bridge's outward projection contract was decided explicitly
rather than left incidental, and the decision is recorded and pinned. See
"Unresolved findings" #3.

## Requirement-to-evidence matrix

| # | Acceptance criterion (plan §9) | Evidence |
|---|---|---|
| 1 | An unknown key inside any persisted assessment, including inside a `reasons` element, fails closed | `crates/eggplan-core/src/assessment.rs:41` — `deny_unknown_fields` on the enum. `crates/eggplan-core/src/closure.rs:629` — `closure_shapes_reject_unknown_fields_inside_assessment_reasons` injects into the reason element at four levels (plan, item, criterion, requirement) and asserts both `ClosureRecord` and `PlanAssessment` reject the result |
| 2 | The regression coverage enumerates reason-level paths, not only struct-level paths | `crates/eggplan-core/src/closure.rs:629`; the injection paths exercised are `candidate.assessment.reasons[0]`, `...items[0].reasons[0]`, `...criteria[0].reasons[0]`, and `...requirements[0].reasons[0]` |
| 3 | Every `AssessmentReason` variant is proven to round-trip with the strict rule in force | `crates/eggplan-core/src/assessment.rs:1163` — `every_assessment_reason_variant_round_trips_strictly` enumerates all 21 variants, asserts the list length is 21, round-trips each, and asserts each rejects an injected unknown key |
| 4 | Every serialized byte, digest, schema version, and golden fixture is unchanged | No serde attribute on any *struct* changed; no schema version changed; `crates/eggplan-core/tests/fixtures/` is byte-identical (`git diff --stat` reports no change to any fixture). The `codegg-compat` 20-test parity suite and the `integrations` suite are green |
| 5 | The compatibility bridge's open/closed decision is recorded explicitly and pinned by a test | Recorded in `architecture/codegg-compat.md` ("Outward projection contract: open by decision"). Pinned by `crates/eggplan-codegg-compat/tests/contract_strictness.rs`: `compat_projection_accepts_a_forward_compatible_unknown_key`, `compat_item_summary_accepts_a_forward_compatible_unknown_key`, `compat_projection_accepts_a_forward_compatible_unknown_key_inside_an_item`, and `the_open_decision_is_scoped_to_the_outward_projection_types` (which proves the crate's internal contract types still reject unknown keys) |
| 6 | The closure record schema, the digest rules, the guarded finalizer, and the frozen Git subject identity are unchanged | No change to `crates/eggplan-core/src/closure.rs` production code; the change there is a new test. No change to `crates/eggplan-repo/src/store.rs`'s finalizer path. `crates/eggplan-repo/tests/` fixtures unchanged |
| 7 | Native qualification, MSRV qualification, and the boundary guards green | See "Verification executed". Hosted macOS/Windows native runs were **not** performed — see "Unresolved findings" #4 |

## Production implementation evidence

### The enum

`crates/eggplan-core/src/assessment.rs:29-42`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "code",
    content = "detail",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum AssessmentReason {
```

The four assessment structs that hold this enum already carried
`deny_unknown_fields` (`crates/eggplan-core/src/assessment.rs:68`, `:77`,
`:86`, `:95`), so the enum is now consistent with its containers.

### Why the strict rule is supportable here — verified, not assumed

serde is pinned at 1.0.229 (`Cargo.lock`). The derive implementation for
adjacently tagged enums selects a field visitor that rejects unknown keys
(`serde_derive-1.0.229/src/de/enum_adjacently.rs`, the `TagOrContentFieldVisitor`
path). The plan required an empirical proof rather than a source reading, and
the proof is the test at `crates/eggplan-core/src/assessment.rs:1163`: for each
of the 21 variants it serializes, deserializes, compares for equality, and
then deserializes a copy carrying an extra `future` key and asserts failure.

The adjacently tagged representation is the subtle part, and it is why the
proof is per-variant rather than a single sample: unit variants serialize
without a `detail` key, newtype variants serialize a single value, and tuple
variants serialize an array. All three shapes round-trip.

### The regression test

`crates/eggplan-core/src/closure.rs:629` extends the closure-record injection
regression to reason elements. The test builds a closure candidate whose
assessment carries a reason at each of the four levels, finalizes a
`ClosureRecord` from it, and then injects an unknown field into each reason
element in turn, asserting that both `ClosureRecord` and `PlanAssessment`
reject the tampered value.

### The bridge decision

`CompatProjection` and `CompatItemSummary` remain **open** — they accept an
unknown key — and that is now a recorded decision with a rationale and a
pinning test rather than an accident. The rationale, as written in
`architecture/codegg-compat.md`:

- These two types are the bridge's outward contract, consumed by the external
  `dbowm91/codegg` repository, which cannot be coordinated from this
  workspace. The crate's internal contract types remain strict.
- The bridge is the producer of this contract, so it emits only the keys it
  defines; strictness on the read side would primarily constrain a
  downstream consumer that may legitimately round-trip or extend the payload
  across versions.
- Unknown fields are not an extension point inside Eggplan's own persisted
  schemas, which remain strict. This is an explicitly recorded exception scoped
  to the bridge's outward projection contract.
- The contract is versioned by `CompatProjection.schema_version`, so a
  consumer that needs a new field can negotiate the version rather than rely
  on lenient parsing.

### Tests

| Test | File | Purpose |
|---|---|---|
| `every_assessment_reason_variant_round_trips_strictly` | `crates/eggplan-core/src/assessment.rs:1163` | All 21 variants round-trip and reject an injected key |
| `closure_shapes_reject_unknown_fields_inside_assessment_reasons` | `crates/eggplan-core/src/closure.rs:629` | Reason-level injection fails closed at all four levels |
| `compat_projection_accepts_a_forward_compatible_unknown_key` | `crates/eggplan-codegg-compat/tests/contract_strictness.rs` | Pins the open decision |
| `compat_item_summary_accepts_a_forward_compatible_unknown_key` | `crates/eggplan-codegg-compat/tests/contract_strictness.rs` | Pins the open decision |
| `compat_projection_accepts_a_forward_compatible_unknown_key_inside_an_item` | `crates/eggplan-codegg-compat/tests/contract_strictness.rs` | Pins the open decision at the nested level |
| `the_open_decision_is_scoped_to_the_outward_projection_types` | `crates/eggplan-codegg-compat/tests/contract_strictness.rs` | The crate's internal types still reject unknown keys |

## Verification executed

Run by the coordinator against the integrated working tree:

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo check --workspace --all-targets --locked` | pass |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | pass, no warnings |
| `cargo test --workspace --locked` | pass — 312 passed, 0 failed, 3 ignored (pre-existing characterization tests) |
| `bash scripts/check-core-boundary.sh` | pass |
| `bash scripts/check-codegg-compat-boundary.sh` | pass |
| `bash scripts/check-integrations-boundary.sh` | pass |
| `bash scripts/check-projection-cli-boundary.sh` | pass |
| `bash scripts/check-closure-authority-boundary.sh` | pass |
| `cargo +1.89.0 check --workspace --all-targets --locked` | pass |
| `cargo +1.89.0 test --workspace --locked` | pass — 312 passed, 0 failed, 3 ignored |

Run by the implementing agents, scoped to the crates they owned:

| Command | Result |
|---|---|
| `cargo test -p eggplan-core --locked` | pass — 45 passed, 0 failed |
| `cargo test -p eggplan-integrations --locked` | pass — 84 passed, 0 failed |
| `cargo test -p eggplan-codegg-compat --locked` | pass — 31 passed, 0 failed |
| `cargo clippy -p eggplan-integrations --all-targets --locked -- -D warnings` | pass |
| `cargo clippy -p eggplan-codegg-compat --all-targets --locked -- -D warnings` | pass |

**Not run:** the hosted CI matrix on ubuntu, macOS, and Windows native
runners. CI job identifiers must be added to this record as an amendment once
the change is pushed.

## Invariant review

- **Canonical bytes are unchanged.** No struct's serde representation changed.
  The enum's serialization is untouched; only its deserialization became
  strict. The schema-v1 and schema-v2 golden fixtures and their `.sha256`
  sidecars are byte-identical.
- **Closure record digests are unchanged.** A closure record's content digest
  covers known fields only. The strict rule means a record carrying an unknown
  key can no longer be parsed at all, so it cannot be admitted with a digest
  that does not cover it. This is the specific gap the corrective targeted.
- **The guarded finalizer is untouched.** No change to the finalizer, to
  `SubjectCapture` containment, or to the closure authority boundary.
- **v1 compatibility is preserved.** v1 remains frozen and readable
  byte-for-byte, both schema versions still parse, and unknown fields are
  rejected rather than treated as an extension point.

## Failure, recovery, and compatibility review

- **Recovery:** unchanged. The pending-closure recovery protocol still
  requires an explicit `--recover-pending`.
- **Compatibility:** the strictness change is a *rejection* of previously
  accepted input. A closure record or assessment that carried an unknown key
  inside a reason element would previously have parsed and would now fail. No
  record written by this codebase carries such a key, because the writer never
  emits one.
- **External consumers:** the bridge's outward types remain open, so the
  external CodeGG repository's read path is unaffected.

## Security, trust, and path review

- **Integrity:** the narrow gap this closes is that an unknown key inside a
  reason element sat outside the closure record's content-digest coverage.
  It could not alter a digest, but it could sit in a persisted record without
  being covered by it. Unknown fields are now rejected at every level.
- **Trust:** no trust rule changed. The `AssessmentReason` variants and their
  meanings are unchanged.
- **Paths:** no path handling was added or changed.

## Documentation and operations

- `architecture/evidence.md` — states that the nested `AssessmentReason` enum
  denies unknown fields, like the four structs that hold it, and that an
  injected key inside a reason element now fails closed.
- `architecture/codegg-compat.md` — the explicit open/closed decision for the
  outward projection types, with the rationale and the pinning test.
- `architecture/deep-dive-core-domain.md` — the review finding that described
  the nested enum as accepting unknown fields was rewritten; the gotcha entry
  now explains the adjacently tagged representation and the serde support for
  `deny_unknown_fields` on such an enum.
- `architecture/deep-dive-codegg-compat.md` — the open outward contract is
  recorded in the projection walkthrough.

## Unresolved findings

| # | Severity | Finding | Disposition |
|---|---|---|---|
| 1 | medium | The hosted CI matrix (ubuntu, macOS, Windows native) was not run. | Must be run by CI after the change is pushed; record the job identifiers here as an amendment. |
| 2 | low | The implementation is in the working tree and is not yet committed. | The commit identifier is to be appended after the commit. |
| 3 | medium | The bridge's outward projection types remain lenient by decision, and the external `dbowm91/codegg` repository has not been consulted. | Recorded in `architecture/codegg-compat.md` as an open coordination item. A CodeGG consumer must not treat an unknown key as an error on these two types, and must not rely on strict rejection as a signal. If the external repository prefers strictness on the outward contract, the decision can be revisited with a versioned schema bump. |
| 4 | low | The plan's claim that the `eggplan-codegg-compat` crate has 15 `deny_unknown_fields` uses "across its public contract types" was imprecise: all 15 are on structs, and the public adjacently tagged `Loss` enum does not carry the attribute, so it is also open. | Recorded in `architecture/deep-dive-codegg-compat.md`. The claim was not repeated as written in the design record. |
| 5 | low | `architecture/deep-dive-integrations.md` carries pre-existing citation drift for `crates/eggplan-core/src/schema.rs` that this pass did not introduce and did not repair. | Left as-is and recorded here rather than silently rewriting an unrelated claim. |

## Roadmap disposition

Foundation-core roadmap M003 corrective line C003 is closed. The
"M003 corrective lines" table in `plans/subsystems/foundation-core-roadmap.md`
is updated to record this closure and its closure record.

The finding that opened this corrective — the nested `AssessmentReason` enum
accepting unknown fields inside a digest-covered closure record — is closed on
the core side. The bridge-side half of the same finding is closed by an
explicitly recorded and pinned decision rather than by a strictness change,
because the bridge's outward contract is consumed outside this workspace.

## Registry updates

- `plans/registry.md`: the Foundation M003 C003 row in *Registered
  implementation plans* is updated to `closed`, with a pointer to
  `plans/closure/foundation-core/003-c003-closed.md`.
- `plans/registry.md`: the *Open correctives raised by the 2026-10-23
  repository audit* subsection loses its C003 entry, which moves to the closed
  list.
- `plans/subsystems/foundation-core-roadmap.md`: M003 corrective line C003 is
  marked closed.
- No historical closure record was rewritten.
