# Evidence and Closure M002 C003 — Closure Reference and Authority-Guard Hygiene

Status: closing

Repository baseline: 0904218554c425c30aa6501b58d7fb8bcb839414

Source roadmap:

- plans/subsystems/evidence-closure-roadmap.md

Predecessor closure:

- plans/closure/evidence-closure/002-c002-closed.md

Primary class: non-blocking maintenance / evidence hygiene / defense in depth

## 1. Objective

Correct two post-C002 documentation/guard-quality defects without reopening the
C002 authority correction or blocking the next capability wave.

The C002 implementation is functionally qualified and the public closure
authority boundary is protected by compile-fail tests. This pass repairs:

1. an incorrect implementation SHA copied into the C002 plan/closure record;
2. a static shell guard whose public re-export pattern is narrower than its
   comments and closure claim.

Also remove stale registry wording that calls a fully closed history section a
"Registered corrective gate".

This pass is independent of Projection/CLI M002, CodeGG M002, and Eggstack
M002. Those plans may proceed in parallel.

## 2. Findings

### E-M002-C003-01 — incorrect C002 implementation SHA in planning evidence

The actual C002 implementation commit is:

- `0c0484a18d72f58a1face6f5e749630dfa0c182d`

The C002 closure record and implementation-plan follow-through text instead
contain:

- `0c0484afe5be83ed92e6e4a4fdbcf6af6dbf6f3c`

The hosted run `36004813178` correctly identifies the real implementation
commit, so this is a citation/reference defect rather than missing
qualification.

Required correction:

- replace only the incorrect SHA references;
- preserve historical status, findings, run IDs, job IDs, dates, and
  acceptance conclusions;
- add a concise erratum note if useful for auditability rather than silently
  implying the original file was always correct.

### E-M002-C003-02 — static authority guard misses ordinary Rust re-export shapes

Current `scripts/check-closure-authority-boundary.sh` is defense in depth
behind compile-fail doctests, but its first pattern expects a forbidden symbol
too close to `pub use`. Ordinary Rust forms such as:

    pub use store::SubjectCapture;
    pub use store::{SubjectCapture, GitSubjectCapture};

can evade that specific source regex.

The compile-fail doctests remain the stronger boundary and already prevent the
actual public exposure from passing the test suite. C003 must make the static
guard match its documented scope as well.

## 3. Required guard behavior

The guard must fail on at least:

- direct public re-export of `SubjectCapture`;
- grouped public re-export containing `SubjectCapture`;
- direct/grouped re-export of `GitSubjectCapture`;
- direct/grouped re-export of `ScriptedSubjectCapture`;
- `pub trait SubjectCapture`;
- `pub struct GitSubjectCapture`;
- `pub struct ScriptedSubjectCapture`;
- `pub fn finalize_closure_with_capture`;
- any public `finalize_closure_with_*` alternate finalizer intended to
  inject a subject authority source.

It must not false-positive on:

- `pub(crate)` declarations;
- comments/documentation mentioning the names;
- compile-fail examples;
- private/crate-private constructors;
- the supported public `finalize_closure`.

Prefer a small readable shell/Python guard over a complex fragile regex. If
shell is retained, explicitly test the guard against synthetic positive and
negative snippets.

## 4. Guard self-test

Add a deterministic negative-proof mode or fixture test analogous to other
Eggstack ownership guards.

At minimum demonstrate:

- known-safe current source passes;
- synthetic `pub use store::SubjectCapture;` fails;
- synthetic grouped re-export fails;
- synthetic `pub fn finalize_closure_with_capture` fails;
- `pub(crate)` equivalents pass.

The self-test must not modify tracked source.

Wire it into CI on the same Linux/macOS lanes as the existing guard, or make
the guard script run its own synthetic proof every invocation.

## 5. Registry/planning wording cleanup

The registry currently retains the heading `## Registered corrective gate`
while stating C001/C002 are closed and no corrective plan is registered.

Change that section to a truthful historical/current label such as:

- `## Corrective history`; or
- `## Current corrective status`.

After C003 registration, it should clearly state that C003 is a non-blocking
maintenance handoff and does not gate the three M002 capability plans.

Do not rewrite prior closure history.

## 6. Scope

### In scope

- correct the erroneous C002 implementation SHA references;
- strengthen `check-closure-authority-boundary.sh`;
- deterministic negative-proof coverage for the static guard;
- registry wording cleanup;
- focused documentation updates if the static guard semantics are described;
- normal native/MSRV qualification.

### Out of scope

- changing C001/C002 production closure behavior;
- changing the S1/S2 algorithm;
- changing public APIs;
- persisted schema changes;
- Projection/CLI M002;
- CodeGG M002;
- Eggstack M002.

## 7. Required tests

At minimum:

- current authority-boundary guard passes;
- synthetic direct re-export fails;
- synthetic grouped re-export fails;
- synthetic alternate public finalizer fails;
- crate-private seam shapes pass;
- the three C002 `compile_fail` doctests still pass;
- all existing closure subject regressions remain green;
- no incorrect C002 implementation SHA remains in active C002 plan/closure
  records.

## 8. Verification

Record actual results for:

    cargo fmt --all -- --check
    cargo check --workspace --all-targets --locked
    cargo clippy --workspace --all-targets --locked -- -D warnings
    cargo test --workspace --locked
    cargo test --workspace --doc --locked
    cargo +1.89.0 check --workspace --all-targets --locked
    cargo +1.89.0 test --workspace --locked
    bash scripts/check-closure-authority-boundary.sh
    git diff --check

Hosted Linux/macOS/Windows and Rust 1.89 qualification should remain green.

## 9. Acceptance criteria

C003 closes when:

1. every active C002 reference uses
   `0c0484a18d72f58a1face6f5e749630dfa0c182d`;
2. the authority guard catches direct and grouped forbidden re-exports;
3. the guard has deterministic negative-proof coverage;
4. C002 compile-fail tests remain the primary public-API boundary evidence;
5. registry wording no longer claims a closed gate is currently registered;
6. no production API/schema behavior changes;
7. native/MSRV qualification passes.

## 10. Handoff notes

Do not gate or absorb the three next capability milestones into this pass.
This is maintenance only.

## 11. Implementation follow-through

- The incorrect C002 implementation SHA was corrected in the active C002 plan
  and closure record. The closure record retains a dated factual erratum.
- The closure-authority guard now checks direct/grouped public re-exports,
  public authority declarations, and all public `finalize_closure_with_*`
  methods. Each invocation runs deterministic positive and negative synthetic
  proofs, including crate-private safe forms.
- No production API or persisted schema changed.
- Local Linux verification passed: formatting, workspace check, clippy,
  workspace tests (100 tests), doc tests (3 compile-fail doctests), Rust 1.89
  check/tests, guard self-proofs, and `git diff --check`.
- Hosted native/MSRV workflow is triggered by the implementation push and will
  be recorded in the closure record when available.
