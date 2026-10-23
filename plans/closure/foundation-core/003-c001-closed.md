# Foundation M003 C001 — Closed

Status: closed

Source plan: plans/implementation/foundation-core/003-c001-canonical-execution-kind-predicate.md

Source roadmap: plans/subsystems/foundation-core-roadmap.md

Reviewed baseline: `37ad290ca6b971a68d2871aedb39f4882f4eb2a8`.

Implementation state: the implementation is complete in the working tree and
was not committed by the coordinator. The commit identifier is to be added to
this record as a factual amendment once the change is committed, following the
precedent of `plans/closure/evidence-closure/002-c004-closed.md`.

## Executive finding

Foundation M003 C001 closes. The execution-derived evidence-kind
classification now has exactly one owner. `eggplan-core` exposes it as the
public function `is_execution_evidence`
(`crates/eggplan-core/src/lib.rs:47`), whose body is an exhaustive `match` over
`EvidenceKind` with no wildcard arm. `eggplan-integrations`
(`crates/eggplan-integrations/src/lib.rs:322`) and `eggplan-codegg-compat`
(`crates/eggplan-codegg-compat/src/lib.rs:618`) consume that function and no
longer carry a local copy of the kind set. The obligation to classify a new
variant is now enforced by the compiler: adding an `EvidenceKind` variant is a
compile error until the new variant is classified in one place.

The classification itself did not change. `Command`, `Test`, `StaticAnalysis`,
`DelegatedRun`, and `Benchmark` remain execution-derived; `Revision`,
`Artifact`, `Research`, `HumanJudgment`, and `Attestation` remain not. No
schema version, digest, wire format, or persisted byte changed.

## Requirement-to-evidence matrix

| # | Acceptance criterion (plan §9) | Evidence |
|---|---|---|
| 1 | Exactly one canonical classification, owned by `eggplan-core` | `crates/eggplan-core/src/lib.rs:47` — `pub fn is_execution_evidence(kind: EvidenceKind) -> bool` |
| 2 | Neither downstream crate contains a local copy of the kind set | `crates/eggplan-integrations/src/lib.rs:322` calls the core predicate; `crates/eggplan-codegg-compat/src/lib.rs:618` calls the core predicate. The five-kind `matches!` lists are deleted from both crates |
| 3 | Adding an `EvidenceKind` variant fails to compile until it is classified | Exhaustive `match` at `crates/eggplan-core/src/lib.rs:48-59`. Compile-failure probe recorded below |
| 4 | The classification of all ten current variants is unchanged | `crates/eggplan-core/src/lib.rs:70` — `execution_evidence_classification_pins_every_kind` pins all ten by name. `crates/eggplan-integrations/tests/execution_binding_gate.rs` and `crates/eggplan-codegg-compat/tests/execution_binding_gate.rs` compare each downstream gate against the core predicate for every kind |
| 5 | SPI rejection and bridge binding requirement are behaviorally identical | The SPI still raises `SpiError::MissingVerificationBinding` and the bridge still fails closed with `execution evidence lacks the authoritative matching verification binding`. Pinned by the two new binding-gate test files plus the unchanged `tests/conformance.rs::execution_evidence_requires_host_verification_binding` and the 20-test `tests/parity.rs` suite |
| 6 | No schema, digest, wire-format, or public-API change beyond the single predicate | Only `crates/eggplan-core/src/lib.rs`, `crates/eggplan-integrations/src/lib.rs`, and `crates/eggplan-codegg-compat/src/lib.rs` changed in production code. No `Cargo.toml` changed. No serde attribute changed |
| 7 | The architecture record states the single owner | `architecture/core.md`, `architecture/provider-spi.md`, `architecture/codegg-compat.md`, `architecture/deep-dive-core-domain.md`, `architecture/deep-dive-codegg-compat.md`, `architecture/deep-dive-integrations.md` |
| 8 | Native qualification, MSRV qualification, and the three affected boundary guards green | See "Verification executed". Hosted macOS/Windows native runs were **not** performed — see "Unresolved findings" |

## Production implementation evidence

### Core: the canonical predicate

`crates/eggplan-core/src/lib.rs:47-59` replaces the former `pub(crate)`
`matches!` helper with a public, documented function whose body is a
deliberately non-exhaustive-pattern `match`:

```rust
pub fn is_execution_evidence(kind: EvidenceKind) -> bool {
    match kind {
        EvidenceKind::Command
        | EvidenceKind::Test
        | EvidenceKind::StaticAnalysis
        | EvidenceKind::DelegatedRun
        | EvidenceKind::Benchmark => true,
        EvidenceKind::Revision
        | EvidenceKind::Artifact
        | EvidenceKind::Research
        | EvidenceKind::HumanJudgment
        | EvidenceKind::Attestation => false,
    }
}
```

The three internal core call sites
(`crates/eggplan-core/src/assessment.rs:262`,
`crates/eggplan-core/src/model.rs:316`,
`crates/eggplan-core/src/evidence.rs:263`) already referenced the helper
through `crate::is_execution_evidence` and are unchanged in behavior.

The plan's preferred shape — a public function returning `bool` whose body is
an exhaustive `match` directly over `EvidenceKind` — was adopted. The rejected
alternative, a const slice plus a completeness test, was not used, because a
test cannot fail a build that adds a variant; the exhaustive match can.

### Downstream: consumption instead of duplication

`crates/eggplan-integrations/src/lib.rs:322` now reads:

```rust
if eggplan_core::is_execution_evidence(context.requested_kind)
```

`crates/eggplan-codegg-compat/src/lib.rs:613-619` keeps the private
`requires_verification_binding` helper for its single caller, but its body
delegates to the core predicate.

### Compile-failure probe

A temporary variant `ProbeNewKind` was added to `EvidenceKind` and
`cargo check -p eggplan-core --locked` was run. The build failed, as required:

```
error[E0004]: non-exhaustive patterns: `model::EvidenceKind::ProbeNewKind` not covered
  --> crates/eggplan-core/src/lib.rs:48:11
   |           ^^^^ pattern `model::EvidenceKind::ProbeNewKind` not covered
  --> crates/eggplan-core/src/model.rs:30:10
   |     ------------ not covered
error: could not compile `eggplan-core` (lib) due to 1 previous error
```

The probe variant was removed immediately afterwards; the probe was not left in
the tree. `git diff` confirms `crates/eggplan-core/src/model.rs` contains no
`ProbeNewKind`.

### Tests

| Test | File | Purpose |
|---|---|---|
| `execution_evidence_classification_pins_every_kind` | `crates/eggplan-core/src/lib.rs:70` | Pins all ten variants by name |
| `missing_binding_gate_matches_the_canonical_core_predicate_for_every_kind` | `crates/eggplan-integrations/tests/execution_binding_gate.rs` | The SPI gate equals the core predicate for every kind |
| `bridge_binding_requirement_matches_the_canonical_core_predicate` | `crates/eggplan-codegg-compat/tests/execution_binding_gate.rs` | The bridge gate equals the core predicate for every mapped CodeGG ref kind |

## Verification executed

Run by the coordinator against the integrated working tree:

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo check --workspace --all-targets --locked` | pass |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | pass, no warnings |
| `cargo test --workspace --locked` | pass — 312 passed, 0 failed, 3 ignored (the 3 ignored are the pre-existing characterization tests, unchanged) |
| `bash scripts/check-core-boundary.sh` | pass |
| `bash scripts/check-codegg-compat-boundary.sh` | pass |
| `bash scripts/check-integrations-boundary.sh` | pass |
| `bash scripts/check-projection-cli-boundary.sh` | pass |
| `bash scripts/check-closure-authority-boundary.sh` | pass |
| `cargo +1.89.0 check --workspace --all-targets --locked` | pass |
| `cargo +1.89.0 test --workspace --locked` | pass — 312 passed, 0 failed, 3 ignored |

Run by the implementing agent, scoped to the crates it owned:

| Command | Result |
|---|---|
| `cargo check -p eggplan-integrations --all-targets --locked` | pass |
| `cargo test -p eggplan-integrations --locked` | pass — 84 passed, 0 failed |
| `cargo clippy -p eggplan-integrations --all-targets --locked -- -D warnings` | pass |
| `cargo check -p eggplan-codegg-compat --all-targets --locked` | pass |
| `cargo test -p eggplan-codegg-compat --locked` | pass — 31 passed, 0 failed |
| `cargo clippy -p eggplan-codegg-compat --all-targets --locked -- -D warnings` | pass |
| `cargo fmt --package eggplan-integrations --package eggplan-codegg-compat -- --check` | pass |

**Not run:** the hosted CI matrix on ubuntu, macOS, and Windows native
runners. The local runs above are Linux-native only. CI job identifiers do not
exist for this change yet and must be added to this record as a factual
amendment once the change is pushed and CI has run.

## Invariant review

- **The core crate remains pure.** `is_execution_evidence` is a pure
  classification over a `Copy` enum. It opens no file, starts no process, and
  performs no I/O. `scripts/check-core-boundary.sh` passes against the changed
  core source.
- **The adapter boundary is unchanged.** `eggplan-integrations` still performs
  no acquisition and enrolls no provider trust; the change is a call to a
  pure predicate in place of an inline `matches!`.
- **The bridge boundary is unchanged.** `eggplan-codegg-compat` still depends
  only on `eggplan-core` and `serde`, still performs no repository I/O, and
  still never mints a verification binding.
- **Trust classification did not move.** The five execution-derived kinds are
  the same five kinds as before the pass. No observation, plan, closure record,
  or digest changes meaning.

## Failure, recovery, and compatibility review

- **Compatibility:** the change is source-compatible for `eggplan-integrations`
  and `eggplan-codegg-compat` callers. `is_execution_evidence` changes
  visibility from `pub(crate)` to `pub` in `eggplan-core`, which is additive.
- **Failure behavior:** unchanged. The SPI still raises the same error variant
  for the same five kinds; the bridge still fails the whole mapping closed for
  the same five kinds.
- **Recovery:** not applicable — the change is a compile-time classification
  refactor with no runtime state.

## Security, trust, and path review

- **Trust:** the classification that decides whether an evidence kind requires
  an authoritative verification binding now lives in one auditable place
  rather than three. This *narrows* the surface where a trust rule could be
  re-derived inconsistently; it does not change which kinds are trusted.
- **Provider trust enrollment:** unchanged. `eggplan-integrations` still cannot
  enroll a trusted provider.
- **Paths:** no path handling was added or changed.

## Documentation and operations

- `architecture/core.md` — states that `eggplan-core` is the single owner of
  the execution-derived classification, that it is compiler-enforced by an
  exhaustive `match`, and that both downstream crates consume it.
- `architecture/provider-spi.md` — states that the SPI verification-binding
  gate is driven by the canonical core predicate.
- `architecture/codegg-compat.md` — states that the bridge binding requirement
  calls the canonical core predicate.
- `architecture/deep-dive-core-domain.md`, `architecture/deep-dive-codegg-compat.md`,
  `architecture/deep-dive-integrations.md` — predicate location and shape
  updated, and citations re-verified against the files as they now stand.

## Unresolved findings

| # | Severity | Finding | Disposition |
|---|---|---|---|
| 1 | medium | The hosted CI matrix (ubuntu, macOS, Windows native) was not run for this change. The local evidence is Linux-native only, plus the Rust 1.89.0 MSRV job. | Must be run by CI after the change is pushed; record the job identifiers in this record as an amendment. No known platform-specific risk was introduced — the change is a `match` over a `Copy` enum. |
| 2 | low | The implementation is in the working tree and is not yet committed. | The commit identifier is to be appended to this record after the commit. The precedent `plans/closure/evidence-closure/002-c004-closed.md` shows this amendment being made as a factual addition after the fact. |
| 3 | informational | The plan claimed the `eggplan-codegg-compat` crate has 15 `deny_unknown_fields` uses across its public contract types. Verified: the count is 15, but all 15 are on *structs*. The public `Loss` enum is adjacently tagged and does not carry the attribute, so it is also open. | Recorded in `architecture/deep-dive-codegg-compat.md`. The claim was not repeated as written anywhere in the design record. |
| 4 | informational | `architecture/deep-dive-integrations.md` carries pre-existing citation drift for `crates/eggplan-core/src/schema.rs` that this pass did not introduce and did not repair. | Left as-is and recorded here rather than silently rewriting an unrelated claim. |

## Roadmap disposition

Foundation-core roadmap M003 corrective line C001 is closed. The
"M003 corrective lines" table in `plans/subsystems/foundation-core-roadmap.md`
is updated to record this closure, its closure record, and its implementation
commit placeholder.

The finding that opened this corrective — three independent copies of the
execution-derived kind set — is closed for the three crates that carried
copies. The `eggplan-repo` crate never carried a copy and needed no change.

## Registry updates

- `plans/registry.md`: the Foundation M003 C001 row in *Registered
  implementation plans* is updated to `closed`, with a pointer to
  `plans/closure/foundation-core/003-c001-closed.md`.
- `plans/registry.md`: the *Open correctives raised by the 2026-10-23
  repository audit* subsection loses its C001 entry, which moves to the closed
  list.
- `plans/subsystems/foundation-core-roadmap.md`: M003 corrective line C001 is
  marked closed.
- No historical closure record was rewritten. No closed plan's conclusion was
  altered.
