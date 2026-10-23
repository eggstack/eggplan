# Foundation Core M003 C003 — Core Assessment Reason Schema Fail-Closed Parity

Status: closed (see plans/closure/foundation-core/003-c003-closed.md)

Repository baseline: 37ad290ca6b971a68d2871aedb39f4882f4eb2a8

Source roadmap:

- plans/subsystems/foundation-core-roadmap.md

Long-term requirements:

- plans/000-long-term-specification.md (unknown fields are not an extension point)
- plans/001-terminology-and-domain-model.md (assessment vocabulary)

Related ADRs: ADR-0003.

Predecessor plan/closure:

- plans/implementation/foundation-core/003-subject-scope-strict-schema-and-platform-hardening.md
- plans/closure/foundation-core/003-closed.md
- plans/implementation/evidence-closure/002-closure-records-integrity-and-recovery.md
- plans/closure/evidence-closure/002-closed.md

Primary class: invariant / schema integrity

## 1. Objective

Bring the nested `AssessmentReason` enum into the same fail-closed unknown-field
regime as the four assessment structs that contain it, so that an injected key
inside a persisted assessment cannot survive deserialization.

The four container structs already reject unknown fields. The enum they hold
does not. The assessment is persisted inside a closure record whose digest
covers known fields only, so this is exactly the case the existing regression
test was written to protect — and the test's injection paths stop one level
above the enum.

This pass changes deserialization strictness only. It changes no serialized
byte, no digest, no schema version, and no assessment outcome.

## 2. Finding

### E-FC-C003-01 — `AssessmentReason` accepts unknown fields while its containers do not

`crates/eggplan-core/src/assessment.rs:30-54`:

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(tag = "code", content = "detail", rename_all = "snake_case")]
    pub enum AssessmentReason {
        MissingObservation,
        ...
        EvidenceStatus(EvidenceObservationId, EvidenceStatus),
        ...
    }

The enum has 21 variants and no `deny_unknown_fields`. The four structs that
hold it all have it:

- `RequirementAssessment` — `crates/eggplan-core/src/assessment.rs:56-63`
- `CriterionAssessment` — `crates/eggplan-core/src/assessment.rs:65-72`
- `ItemAssessment` — `crates/eggplan-core/src/assessment.rs:74-81`
- `PlanAssessment` — `crates/eggplan-core/src/assessment.rs:83-92`

Each of the four declares a `reasons: Vec<AssessmentReason>` field, so an
unknown key injected inside a reason element is inside a struct that has
already been admitted, and is then governed only by the enum's own, laxer,
rule.

The stated invariant is in the repository's own regression test.
`crates/eggplan-core/src/closure.rs:585-620` enumerates injection paths and
asserts that each one is rejected:

    for path in [
        vec!["candidate", "assessment"],
        vec!["candidate", "assessment", "items", "0"],
        vec!["candidate", "assessment", "items", "0", "criteria", "0"],
        vec![
            "candidate",
            "assessment",
            "items",
            "0",
            "criteria",
            "0",
            "requirements",
            "0",
        ],
    ] { ... }

with the comment at `:602-604`:

    // Unknown fields are not an extension point: the assessment structs
    // are persisted inside a closure record, so an injected key must
    // fail closed even though the record digest covers known fields only.

Every enumerated path is a *struct* path. No path reaches a `reasons` array,
so the enum's behavior is not covered by the test that exists specifically to
cover it.

**Blast radius.** A closure record whose assessment carries an injected key
inside a reason element may round-trip through deserialization instead of
failing closed. The record's content digest covers known fields only, so the
injected key is outside the digest's coverage. This is a schema-integrity
gap, not a closure bypass: the digest still pins the record's known content,
the guarded finalizer still owns closure, and no evidence is fabricated or
trust enrolled. The exposure is that a persisted assessment may carry bytes
that the schema says cannot exist, and that those bytes are not digest-bound.

**Contributing decision.** Commit `37ad290` added `deny_unknown_fields` to the
four assessment structs to close an injected-field finding. The enum nested
inside them was not part of that change, and the regression test was written
against the struct paths that were changed.

**Detection gap.** As above: the injection-path list in
`crates/eggplan-core/src/closure.rs:585-596` has no `reasons` path.

### E-FC-C003-02 — the same gap exists in the compatibility bridge's serialized contract

`crates/eggplan-codegg-compat/src/lib.rs:519-542` declares two public,
serializable types without `deny_unknown_fields`:

- `CompatProjection` — `:519-531`
- `CompatItemSummary` — `:533-542`

Both have data fields, so an unknown key in either is silently accepted. The
rest of the bridge's serialized contract does use `deny_unknown_fields`
(`crates/eggplan-codegg-compat/src/lib.rs:15` deny count is 15 across the
crate's public contract types), so these two are the outlier.

This crate is consumed by the external CodeGG repository, so a forward-
compatible open contract may be deliberate there. It is recorded here because
it is the same class of gap, and because the decision must be explicit rather
than incidental. If the decision is "keep them open for forward compatibility",
that must be written down in `architecture/codegg-compat.md`; if the decision
is "match the crate's other contract types", they get the attribute.

## 3. Controlling semantics

From `plans/000-long-term-specification.md` and the canonical-JSON rules in
`architecture/core.md`, unknown fields are rejected and are not an extension
point. From `architecture/evidence.md`, a closure record's persisted assessment
must fail closed on any byte sequence the schema does not define.

Required behavior after this pass:

- an unknown key injected anywhere inside a persisted assessment — including
  inside a `reasons` element — fails closed;
- every one of the 21 `AssessmentReason` variants round-trips through
  serialize → deserialize with the stricter rule in force;
- serialized bytes, digests, and schema versions are unchanged;
- the compatibility bridge's open-versus-closed contract decision is recorded
  explicitly.

## 4. Scope

### In scope

- the serde attributes and deserialization strictness of
  `crates/eggplan-core/src/assessment.rs`;
- the injected-field regression coverage in
  `crates/eggplan-core/src/closure.rs`;
- the `deny_unknown_fields` disposition of `CompatProjection` and
  `CompatItemSummary` in `crates/eggplan-codegg-compat/src/lib.rs`;
- the corresponding documentation in `architecture/core.md`,
  `architecture/evidence.md`, `architecture/codegg-compat.md`, and the
  affected deep dives;
- registry and roadmap lineage updates.

### Out of scope

- the `AssessmentReason` variant set, its `code`/`detail` wire names, and its
  `snake_case` renaming;
- the `AssessmentStatus` enum and the assessment algorithm;
- the `ClosureRecord` schema, its digest rules, and the guarded finalizer;
- the Plan schema version, the evidence schema version, and every golden
  fixture whose bytes are frozen;
- the `SubjectRevision` schema;
- adding new assessment reasons;
- the deferred Interop/distribution subsystem;
- the CLI and Projection/CLI milestones.

## 5. Required production changes

Ownership is `eggplan-core` for the assessment schema and
`eggplan-codegg-compat` for its own contract.

Preferred shape:

- add `deny_unknown_fields` to the `AssessmentReason` enum so that its
  deserialization rejects unknown keys;
- extend the injection-path list in
  `crates/eggplan-core/src/closure.rs:585-596` to include a path through a
  `reasons` array, at the plan, item, criterion, and requirement levels as
  the existing paths are, so that the regression test covers the enum rather
  than only the structs;
- for the compatibility bridge, either add `deny_unknown_fields` to
  `CompatProjection` and `CompatItemSummary`, or record an explicit,
  documented decision that the bridge's outward contract is intentionally
  forward-compatible. The decision must be written down either way, in
  `architecture/codegg-compat.md`.

**Implementation risk that must be verified, not assumed.** `AssessmentReason`
uses serde's adjacently tagged representation
(`#[serde(tag = "code", content = "detail")]`). The pinned serde version is
1.0.229, whose derive support for `deny_unknown_fields` on adjacently tagged
enums selects a field visitor that fails on unknown keys — so the attribute is
supported in principle. Serde's adjacently tagged representation has known
subtleties around unit variants, and the interaction with
`deny_unknown_fields` must be established empirically for this enum rather
than inferred. Before landing, the implementer must confirm that **every one
of the 21 variants** round-trips through serialize → deserialize with the
attribute in force, and must record that result. If any variant fails to
round-trip, the enum's wire representation needs a decision that is out of
scope for this pass, and the stop conditions in §10 apply.

Explicitly rejected alternatives:

- changing the `AssessmentReason` wire representation to a struct-based
  encoding. That is a persisted-schema change with digest and golden-fixture
  consequences, and it needs its own decision and milestone;
- relying on the existing struct-level `deny_unknown_fields` alone. It does
  not govern the nested enum;
- deleting the regression test's existing struct paths. They remain valid
  coverage.

## 6. Failure, cancellation, restart, and contention semantics

- The pass must not change any serialized byte. Because `deny_unknown_fields`
  affects deserialization only, every existing golden fixture, plan digest,
  observation digest, and closure-record digest must remain byte-identical.
- Every existing closure record on disk must still deserialize successfully.
  A stored record containing a reason with an unknown key — which cannot be
  produced by this codebase but could exist in a hand-edited state root — must
  fail closed rather than load.
- No read, write, lock, or restart behavior changes. The pass touches no
  persistence path.
- If any of the 21 variants fails to round-trip with the stricter rule, the
  pass must not ship a partially-strict enum. Either all variants round-trip,
  or the change is not landed.

## 7. Required tests

At minimum:

- extend the injected-field regression in
  `crates/eggplan-core/src/closure.rs` with `reasons` paths at the plan, item,
  criterion, and requirement levels, asserting each is rejected;
- a core test that round-trips every one of the 21 `AssessmentReason`
  variants through serialize → deserialize, asserting equality;
- a core test asserting that an unknown key injected into a reason element is
  rejected when the reason is nested inside a persisted closure record;
- for the compatibility bridge, if the closed decision is taken, a test
  asserting `CompatProjection` and `CompatItemSummary` reject unknown keys; if
  the open decision is taken, a test asserting they accept a forward-compatible
  key so the decision is pinned rather than incidental;
- the existing `eggplan-core` and `eggplan-codegg-compat` suites, green.

## 8. Verification

Record actual results for:

    cargo fmt --all -- --check
    cargo check --workspace --all-targets --locked
    cargo clippy --workspace --all-targets --locked -- -D warnings
    cargo test --workspace --locked
    cargo test -p eggplan-core --locked
    cargo test -p eggplan-codegg-compat --locked
    cargo test --workspace --doc --locked
    cargo +1.89.0 check --workspace --all-targets --locked
    cargo +1.89.0 test --workspace --locked
    bash scripts/check-core-boundary.sh
    bash scripts/check-codegg-compat-boundary.sh
    git diff --check

The `cargo +1.89.0` rows are the MSRV gate and must be recorded truthfully,
including if the toolchain is unavailable in the implementing environment.
Hosted Linux/macOS/Windows and Rust 1.89 qualification must remain green.

## 9. Acceptance criteria

C003 closes when:

1. an unknown key injected anywhere inside a persisted assessment, including
   inside a `reasons` element, fails closed;
2. the regression coverage enumerates reason-level paths, not only
   struct-level paths;
3. all 21 `AssessmentReason` variants are proven to round-trip with the
   stricter rule in force, and that proof is recorded;
4. every serialized byte, digest, schema version, and golden fixture is
   unchanged;
5. the compatibility bridge's open-versus-closed contract decision is recorded
   explicitly in `architecture/codegg-compat.md`, and is pinned by a test;
6. the closure record schema, digest rules, and guarded finalizer are
   unchanged;
7. native and MSRV qualification pass, with the two named boundary guards
   green.

## 10. Stop conditions

Stop and report rather than improvise if:

- any `AssessmentReason` variant fails to round-trip with `deny_unknown_fields`
  in force, because that requires a wire-representation decision this pass
  does not own;
- making the assessment strict would change any serialized byte, digest,
  schema version, or golden fixture;
- the fix would require changing the `ClosureRecord` schema or its digest
  rules;
- the fix would require changing the guarded finalizer or the closure
  authority model;
- the compatibility bridge's contract decision cannot be made without
  coordinating with the external CodeGG repository, in which case the bridge
  half of the pass must be deferred rather than guessed;
- required verification cannot be obtained honestly.

## 11. Closure evidence required

- implementation commit(s) and PR reference;
- the requirement-to-evidence matrix for every acceptance criterion in §9;
- the recorded per-variant round-trip result for all 21 `AssessmentReason`
  variants;
- the recorded before/after output of the injected-field regression with the
  new reason-level paths;
- explicit confirmation that all golden fixtures and digests are unchanged;
- exact verification output, including the MSRV rows, distinguishing planned
  from actually run commands;
- hosted native/MSRV run and job identifiers where available;
- unresolved findings with severity, and roadmap disposition;
- registry updates.

Do not convert a planned command into a passing result because it appears in
§8. If a row was not run, record it as not run or unavailable.

## 12. Handoff notes

- Record truthfully that the assessment *structs* were already strict. The
  defect was the nested enum and the test's path list, not an open schema.
- Do not describe this as a closure bypass. The digest still pins known
  content, the guarded finalizer still owns closure, and no evidence is
  fabricated or trust enrolled.
- The compatibility bridge half may be deferred to a separate pass if the
  external CodeGG contract must be coordinated. Record that deferral
  explicitly rather than shipping a partial decision.
