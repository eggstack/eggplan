# Foundation Core M003 C001 — Canonical Execution-Evidence Kind Predicate

Status: closed (see plans/closure/foundation-core/003-c001-closed.md)

Repository baseline: 37ad290ca6b971a68d2871aedb39f4882f4eb2a8

Source roadmap:

- plans/subsystems/foundation-core-roadmap.md

Long-term requirements:

- plans/000-long-term-specification.md (evidence kinds, verification binding)
- plans/001-terminology-and-domain-model.md (evidence kind vocabulary)

Related ADRs: ADR-0003.

Predecessor plan/closure:

- plans/implementation/foundation-core/003-subject-scope-strict-schema-and-platform-hardening.md
- plans/closure/foundation-core/003-closed.md
- plans/implementation/evidence-closure/001-c001-verification-binding-and-end-to-end-evidence-corrective.md
- plans/closure/evidence-closure/001-c001-closed.md

Primary class: invariant / trust-rule consolidation

## 1. Objective

Collapse the three independent implementations of the "this evidence kind is
execution-derived and therefore requires a verification binding" rule into one
canonical predicate owned by `eggplan-core`, and make the rule structurally
incapable of drifting when `EvidenceKind` gains a variant.

The rule is a trust rule. `EvidenceKind` has ten variants
(`crates/eggplan-core/src/model.rs:30-41`); five of them are execution-derived.
A sixth variant added without a decision would currently leave all three
implementations silently unchanged, and an execution-derived kind would reach
the evidence ledger without a verification binding.

This pass does not change which kinds are execution-derived, does not change
the trust model, and does not reopen Evidence M001 C001. It changes where the
rule lives and makes the compiler enforce it.

## 2. Finding

### E-FC-C001-01 — the execution-kind rule exists in three copies, none exhaustive

The same five-kind set is matched in three independent places:

| Site | Symbol | Location |
|---|---|---|
| `eggplan-core` | `is_execution_evidence` | `crates/eggplan-core/src/lib.rs:33-42` |
| `eggplan-codegg-compat` | `requires_verification_binding` | `crates/eggplan-codegg-compat/src/lib.rs:598-607` |
| `eggplan-integrations` | inline `execution_derived` | `crates/eggplan-integrations/src/lib.rs:318-325` |

All three match `Command | Test | StaticAnalysis | DelegatedRun | Benchmark`.
`eggplan-core`'s copy is `pub(crate)`, so the other two crates cannot call it
and re-derived the rule instead.

**Why it is a defect and not layering.** Each site uses a `matches!`
predicate, which is not exhaustive over `EvidenceKind` and therefore produces
no compiler error when a variant is added. The rule is duplicated *within the
same repository*, in the same crate graph, with the same semantics, and with
no mechanism keeping them equal. The three sites have already diverged in
form: core exposes a named predicate, codegg-compat exposes a differently
named private predicate, and integrations inlines the test into an `if`
condition at `lib.rs:318-326`.

**Blast radius.** The three sites are the three enforcement points of one
trust rule:

- `crates/eggplan-integrations/src/lib.rs:326-328` rejects a provider result
  whose requested kind is execution-derived and whose verification digest is
  absent, with `SpiError::MissingVerificationBinding`.
- `crates/eggplan-codegg-compat/src/lib.rs:598-607` decides whether a mapped
  CodeGG evidence kind requires a verification binding during bridge mapping.
- `crates/eggplan-core/src/lib.rs:33-42` is the core-side predicate used to
  decide whether an observation's kind demands a verification binding.

If a new execution-derived kind were added to `EvidenceKind` and only one of
the three sites were updated, the other two would accept execution evidence
without a verification binding. That is an evidence-integrity divergence, not
a cosmetic one.

**Contributing decision.** The rule was made `pub(crate)` in core when only
core needed it. The later crates needed it too, and `eggplan-codegg-compat`
is forbidden from depending on `eggplan-repo` but not from depending on
`eggplan-core`, so a public core predicate was always available.

**Detection gap.** No test asserts that the three sites agree, and no test
enumerates `EvidenceKind` to force a decision when a variant is added. The
`eggplan-integrations` conformance suite
(`crates/eggplan-integrations/tests/conformance.rs`) exercises the missing
binding rejection for the five currently-classified kinds but cannot detect a
newly added sixth.

## 3. Controlling semantics

From `plans/registry.md` design gates and ADR-0003, execution evidence is
trust-bearing and must be bound to a verification digest before it can be
promoted to proof. The classification of which kinds are execution-derived is
a domain fact, so it must have exactly one owner.

Required behavior after this pass:

- one canonical, publicly callable predicate exists in `eggplan-core`;
- `eggplan-codegg-compat` and `eggplan-integrations` call it and contain no
  local copy of the kind set;
- adding a variant to `EvidenceKind` produces a **compile error** until the
  classification is decided, rather than a silent default;
- the classification of the ten existing variants is byte-for-byte the
  behavior-preserving choice;
- no trust, schema, digest, or wire-format change occurs.

## 4. Scope

### In scope

- the canonical predicate and its classification surface in `eggplan-core`;
- replacing the private copy in `crates/eggplan-codegg-compat/src/lib.rs`;
- replacing the inline copy in `crates/eggplan-integrations/src/lib.rs`;
- core regression coverage for the classification;
- a guard in `scripts/check-core-boundary.sh` is **not** in scope here — see
  plans/implementation/foundation-core/003-c005-core-boundary-guard-and-tooling-governance-reconciliation.md,
  which owns the guard scripts;
- documentation updates in `architecture/core.md`,
  `architecture/provider-spi.md`, `architecture/codegg-compat.md`, and the
  matching `architecture/deep-dive-*.md` files;
- registry and roadmap lineage updates.

### Out of scope

- changing which kinds are execution-derived;
- changing `EvidenceKind`'s variant set, its `snake_case` wire names, or the
  plan schema version;
- changing the provider SPI's verification-binding requirements, the
  `SpiError` taxonomy, or adapter behavior;
- changing CodeGG WorkPlan mapping, digests, or IDs;
- changing the `Eggplan` trust model, provider enrollment, or closure
  authority;
- any change to `plans/000-long-term-specification.md`'s evidence vocabulary;
- adding a new public API beyond the single canonical predicate;
- the CLI and Projection/CLI milestones.

## 5. Required production changes

Ownership is `eggplan-core`. The rule is a domain fact, so the domain owns it,
and the adapters and the bridge consume it.

Preferred shape — make the compiler enforce the classification:

- add one exhaustive classification in `eggplan-core` over `EvidenceKind`,
  expressed as a `match` with no wildcard arm, so that adding a variant to
  `EvidenceKind` fails to compile until every variant is classified;
- expose the existing predicate as a public, documented function in
  `eggplan-core`, implemented in terms of that exhaustive classification;
- keep the existing name `is_execution_evidence` so the core-side call sites
  and tests read unchanged;
- replace `crates/eggplan-codegg-compat/src/lib.rs:598-607` with a call to
  the core predicate, and delete the local five-kind list;
- replace the inline `matches!` at `crates/eggplan-integrations/src/lib.rs:318-325`
  with a call to the core predicate, and delete the local five-kind list.

Explicitly rejected alternative: a shared `const` slice of execution kinds plus
a test that asserts its length equals the number of `EvidenceKind` variants.
A test can be skipped, filtered, or miscounted, and a `const` list cannot
express "this new variant is deliberately non-execution" without a separate
bookkeeping convention. The exhaustive `match` makes the decision a compile-time
obligation in the same place the variant is declared.

If `eggplan-core` cannot express the classification as an exhaustive `match`
over a `Copy` enum without exposing a new public type, prefer a public
function returning a `bool` whose body is an exhaustive `match` directly over
`EvidenceKind`, rather than introducing a public classification enum.

## 6. Failure, cancellation, restart, and contention semantics

- This pass adds no I/O, no locking, no persistence, and no process execution.
- It changes no serialized bytes: `EvidenceKind`'s `snake_case` names, its
  variant set, and every plan/observation digest are untouched.
- A provider adapter that today rejects a missing verification binding for an
  execution-derived kind must continue to reject it with the identical
  `SpiError::MissingVerificationBinding`. Any observable difference in the
  error payload is a regression, not an improvement.
- The CodeGG bridge's verification-binding requirement for mapped kinds must
  be identical before and after; the parity suite in
  `crates/eggplan-codegg-compat/tests/parity.rs` must remain green unchanged.
- Concurrency, restart, and recovery semantics are unchanged: the pass touches
  no repository state.

## 7. Required tests

At minimum:

- a core test that pins the classification of all ten current `EvidenceKind`
  variants, asserting the five execution-derived and five non-execution
  kinds by name, so a reclassification is an explicit, reviewable edit;
- a core test that the public predicate is usable by a downstream crate
  (exercised through the existing `eggplan-integrations` and
  `eggplan-codegg-compat` suites rather than a new cross-crate test);
- the existing `eggplan-integrations` conformance suite, including the
  missing-verification-binding rejection, unchanged and green;
- the existing `eggplan-codegg-compat` parity and repository-projection suites,
  unchanged and green;
- the existing `eggplan-core` suite, unchanged and green.

A test that merely asserts the predicate returns `true` for the five
execution kinds is not sufficient on its own: the pinned classification table
must enumerate every variant, because that is what makes a future variant's
absence loud.

## 8. Verification

Record actual results for:

    cargo fmt --all -- --check
    cargo check --workspace --all-targets --locked
    cargo clippy --workspace --all-targets --locked -- -D warnings
    cargo test --workspace --locked
    cargo test -p eggplan-core --locked
    cargo test -p eggplan-integrations --locked
    cargo test -p eggplan-codegg-compat --locked
    cargo +1.89.0 check --workspace --all-targets --locked
    cargo +1.89.0 test --workspace --locked
    bash scripts/check-core-boundary.sh
    bash scripts/check-integrations-boundary.sh
    bash scripts/check-codegg-compat-boundary.sh
    git diff --check

The `cargo +1.89.0` rows are the MSRV gate and must be recorded truthfully,
including if the toolchain is unavailable in the implementing environment.
Hosted Linux/macOS/Windows and Rust 1.89 qualification must remain green.

## 9. Acceptance criteria

C001 closes when:

1. exactly one canonical execution-kind classification exists, in
   `eggplan-core`;
2. `eggplan-codegg-compat` and `eggplan-integrations` contain no local copy of
   the kind set and call the canonical predicate;
3. adding a variant to `EvidenceKind` fails to compile until the variant is
   classified, demonstrated by a recorded compile-failure probe;
4. the classification of all ten current variants is unchanged from the
   behavior before this pass;
5. the provider SPI's missing-verification-binding rejection and the CodeGG
   bridge's binding requirement are byte-identical in behavior;
6. no persisted schema, digest, wire format, or public API beyond the single
   canonical predicate changes;
7. `architecture/core.md`, `architecture/provider-spi.md`,
   `architecture/codegg-compat.md`, and the affected deep dives state the
   single owner of the rule;
8. native and MSRV qualification pass, with the three named boundary guards
   green.

## 10. Stop conditions

Stop and report rather than improvise if:

- making the classification exhaustive would require changing the
  `EvidenceKind` variant set or its wire names;
- the codegg-compat boundary would have to be loosened to permit the core
  dependency, or the integrations boundary to permit a new dependency;
- collapsing the copies would require changing the provider SPI's trust
  semantics, the `SpiError` taxonomy, or the closure authority model;
- any observable behavior change in the missing-verification-binding path is
  discovered, because that is a trust-model change and needs its own decision;
- required verification cannot be obtained honestly.

## 11. Closure evidence required

Per `plans/003-planning-process.md` §7 and `plans/closure/README.md`, a closure
record must distinguish planned commands from commands actually run, and must
cover:

- implementation commit(s) and PR reference;
- the requirement-to-evidence matrix for every acceptance criterion in §9;
- the recorded compile-failure probe that shows a new `EvidenceKind` variant
  is not silently accepted;
- exact verification output, including the MSRV rows;
- hosted native/MSRV run and job identifiers where available;
- an explicit statement that evidence integrity and trust semantics were
  reviewed and that the classification did not change;
- unresolved findings with severity, and roadmap disposition;
- registry updates.

Do not convert a planned command into a passing result because it appears in
§8. If a row was not run, record it as not run or unavailable.

## 12. Handoff notes

- This is a consolidation pass over an existing rule. Do not describe it as a
  trust-model change, a new capability, or a closure change.
- Record truthfully that the rule was previously correct but duplicated. The
  defect was the absence of a single owner and the absence of a compile-time
  obligation, not a wrong classification.
- Do not absorb Projection/CLI, CodeGG, or Eggstack milestone work into this
  pass. The CodeGG and Eggstack adapters are touched only to consume the
  canonical predicate.
