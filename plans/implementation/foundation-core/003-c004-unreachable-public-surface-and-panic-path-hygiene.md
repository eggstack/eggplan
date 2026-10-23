# Foundation Core M003 C004 — Unreachable Public Surface and Panic-Path Hygiene

Status: closed (see plans/closure/foundation-core/003-c004-closed.md)

Repository baseline: 37ad290ca6b971a68d2871aedb39f4882f4eb2a8

Source roadmap:

- plans/subsystems/foundation-core-roadmap.md

Long-term requirements:

- plans/000-long-term-specification.md (fail-closed validation, explicit errors)

Related ADRs: ADR-0002.

Predecessor plan/closure:

- plans/implementation/foundation-core/003-subject-scope-strict-schema-and-platform-hardening.md
- plans/closure/foundation-core/003-closed.md
- plans/implementation/evidence-closure/002-c003-closure-reference-and-authority-guard-hygiene.md
- plans/closure/evidence-closure/002-c003-closed.md

Primary class: non-blocking maintenance / hygiene

## 1. Objective

Remove unreachable public surface from `eggplan-core` and `eggplan-repo`, and
replace the remaining panic paths in production validation and cleanup code
with typed errors.

This is explicitly non-blocking hygiene. It must not serialize otherwise
independent capability handoffs, and it must not change any observable
behavior of the repository, the domain, or the CLI. Every item in this plan is
either dead code, a panic that cannot occur in practice, or a swallowed error
whose loss is unobservable today.

## 2. Findings

### E-FC-C004-01 — `InspectionSelection::is_repository_wide` has no consumer

`crates/eggplan-repo/src/snapshot.rs:79-82`:

    /// Does this selection enumerate the whole repository?
    pub fn is_repository_wide(&self) -> bool {
        !matches!(self, Self::One(_))
    }

A workspace-wide search for the identifier returns exactly two hits: the
definition above, and `plans/closure/projection-cli/003a-closed.md:368`, which
records that an earlier `debug_assert!(selection.is_repository_wide())` was
removed because it did not cover the intended case. The assertion that
replaced it lives at `crates/eggplan-repo/src/store.rs:1205-1211` and matches
on `InspectionSelection` directly.

`eggplan-repo` is consumed in this workspace by `eggplan-cli`, so unlike the
provider adapters and the CodeGG bridge, this is genuinely unreachable public
surface rather than an externally consumed API.

### E-FC-C004-02 — `RepoError::Domain` is never constructed

`crates/eggplan-repo/src/store.rs:58-59`:

    #[error("plan validation or serialization failed: {0}")]
    Domain(#[from] Box<dyn std::error::Error + Send + Sync>),

A workspace-wide search finds no construction site for this variant. The two
variants that carry the same intent — `RepoError::Validation`
(`store.rs:60-61`) and `RepoError::Serialization` (`store.rs:62-63`) — are
both `#[from]` conversions that are used.

A `#[from]` variant that is never constructed means every generic error
conversion into `RepoError` currently routes through a different variant, so
the variant is either dead or the generic conversion is unreachable. Both
readings are a defect in the error taxonomy: one leaves a public error variant
that no caller will ever observe, the other leaves a conversion path that
cannot be taken.

### E-FC-C004-03 — production validation contains an `expect`

`crates/eggplan-core/src/model.rs:216`, inside `Criterion::validate`:

    let key = serde_json::to_string(req).expect("serializable requirement");

`Criterion::validate` is reached from `Plan::validate`
(`crates/eggplan-core/src/model.rs:288-347`), which runs on every parsed Plan
including Plans read from a state root. The `expect` therefore sits on a path
that handles untrusted input.

`EvidenceRequirement` (`crates/eggplan-core/src/model.rs:85-97`) derives
`Serialize` over `String`, two unit enums, an `Option` of two newtypes, a
`u16`, and a `bool`. `serde_json::to_string` cannot fail for such a value:
there is no map with non-string keys, no custom `Serialize` impl, and no
`f64`. The panic is therefore unreachable in practice.

It is still a panic in a fail-closed validation path, and the surrounding
function already returns `Result<(), ValidationError>`, so the panic is
unnecessary. The failure mode is a low-probability abort of a read rather
than a typed error.

**Detection gap.** No test exercises the failure branch, because there is no
failure branch. The defect is the shape of the code, not a broken behavior.

### E-FC-C004-04 — cleanup errors are discarded

Two cleanup sites discard their error with `let _ =`:

- `crates/eggplan-repo/src/store.rs:935` — `let _ = fs::remove_dir(&dir);` on
  the over-size plan path;
- `crates/eggplan-repo/src/store.rs:942` — `let _ = fs::remove_dir(&dir);` on
  the failed `atomic_write` path, deliberately skipped for
  `RepoError::DurabilityUnknown` so that a possibly-committed write is not
  followed by a directory removal.

Both are correct in intent: the primary error is returned, and the cleanup is
best-effort. The loss is unobservable today, but the discarded result means a
failed cleanup leaves an empty plan directory that `store.list()`
(`store.rs:953-964`) will later enumerate.

`crates/eggplan-repo/src/store.rs:1343` and
`crates/eggplan-repo/src/snapshot.rs:271` discard `File::unlock()` errors in
`Drop` implementations. Those are also correct in intent — `Drop` cannot
return an error — and no action is possible in a destructor.

### E-FC-C004-05 — the lock-file symlink guard is check-then-use

`crates/eggplan-repo/src/store.rs:1349-1359` and
`crates/eggplan-repo/src/snapshot.rs:239-249` share the same shape:

    if let Ok(meta) = fs::symlink_metadata(&path)
        && (meta.file_type().is_symlink() || !meta.is_file())
    {
        return Err(RepoError::UnsafePath(path));
    }
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)?;

The guard correctly rejects an existing symlink or non-regular file, and it
correctly handles the not-found case by falling through to `create(true)`.
The remaining gap is the interval between the check and the open: a process
with write access to the state root could replace the lock path with a
symlink in that window, and `create(true)` follows symlinks.

The exposure requires write access to the state root and a successful race.
The repository's threat model treats the state root as owned by the caller,
so this is a defense-in-depth gap rather than a boundary violation.

## 3. Controlling semantics

From `plans/registry.md` planning hygiene: "Mark maintenance gates explicitly:
non-blocking hygiene must not serialize otherwise independent capability
handoffs." From `architecture/deep-dive-repository.md`, the repository's error
taxonomy is the machine-stable contract the CLI maps to diagnostic codes, and
a variant that cannot occur weakens that contract's meaning.

Required behavior after this pass:

- no public item in `eggplan-core` or `eggplan-repo` is unreachable from a
  consumer, unless it is explicitly documented as an externally consumed
  contract;
- the `RepoError` taxonomy contains no variant that cannot be constructed, or
  an explicit note that it exists for a future path;
- production validation returns typed errors and contains no `expect`,
  `unwrap`, or `panic!` on a path reachable from parsed input;
- no observable behavior changes: same bytes, same errors, same diagnostics.

## 4. Scope

### In scope

- `crates/eggplan-repo/src/snapshot.rs` — the unreachable
  `InspectionSelection::is_repository_wide` accessor;
- `crates/eggplan-repo/src/store.rs` — the `RepoError::Domain` variant, the
  discarded cleanup results, and the lock guard's check-then-use interval;
- `crates/eggplan-core/src/model.rs` — the `expect` in `Criterion::validate`;
- regression coverage for the changed validation path;
- documentation updates in `architecture/repository.md`,
  `architecture/core.md`, and the affected deep dives;
- registry and roadmap lineage updates.

### Out of scope

- `InspectionSnapshot::subject_error()`, whose disposition is owned by
  plans/implementation/foundation-core/003-c002-subject-capture-error-fidelity.md;
- the guarded closure finalizer, `SubjectCapture` containment, and the frozen
  Git fingerprint bytes;
- the atomic-write durability contract, including the deliberate
  `DurabilityUnknown` skip at `store.rs:941-943`;
- the lock protocol, the pending-closure protocol, and cooperative locking
  semantics;
- the CLI's error and diagnostic mapping;
- the `AssessmentReason` schema, owned by
  plans/implementation/foundation-core/003-c003-core-assessment-reason-schema-fail-closed.md;
- the execution-kind predicate, owned by
  plans/implementation/foundation-core/003-c001-canonical-execution-kind-predicate.md;
- `eggplan-markdown`, `eggplan-integrations`, `eggplan-projection`, and
  `eggplan-codegg-compat` hygiene;
- the deferred Interop/distribution subsystem.

## 5. Required production changes

Preferred shape:

- remove `InspectionSelection::is_repository_wide` and its doc comment, and
  confirm that the replacement assertion at `store.rs:1205-1211` continues to
  express the intended coverage;
- resolve `RepoError::Domain` explicitly. Either remove it, or — if a generic
  error conversion is genuinely needed — construct it at the site that should
  use it and keep the variant in the taxonomy. Record which resolution was
  taken and why. Do not leave a `#[from]` variant with no construction path
  and no explanation;
- replace the `expect` in `Criterion::validate` with an error path that
  returns `ValidationError`, so the function's `Result` signature covers every
  failure it can have;
- for the discarded cleanup results, keep the best-effort behavior but make it
  explicit — for example by naming the discarded result and documenting why
  the primary error wins. If an empty plan directory left by a failed cleanup
  is found to affect `store.list()` enumeration, that is a separate finding
  and a stop condition, not something to fix silently here;
- for the lock guard, either close the check-then-use interval — for example
  by opening the lock path without following symlinks where the platform
  permits it — or record an explicit, documented statement that the state root
  is caller-owned and that the residual race is accepted defense-in-depth.
  Prefer the explicit statement if a portable no-follow open is not
  available across the supported platforms.

Explicitly rejected alternatives:

- adding a `#[allow(dead_code)]` attribute to keep dead public items. That
  hides the signal rather than resolving it;
- changing the `DurabilityUnknown` behavior at `store.rs:941-943`. That
  behavior is deliberate and is not in scope;
- introducing a new error variant to carry the discarded cleanup errors. The
  cleanup is best-effort by design and the primary error is the meaningful
  one.

## 6. Failure, cancellation, restart, and contention semantics

- No observable behavior may change. The same inputs produce the same errors,
  the same diagnostics, and the same bytes.
- The `RepoError` variant set may shrink. If it does, the CLI's error mapping
  must remain exhaustive over the surviving variants, and the workspace must
  compile with `-D warnings` — a removed variant must not leave an
  unreachable mapping arm behind.
- Removing `is_repository_wide` must not remove an assertion. The coverage
  expressed at `store.rs:1205-1211` must remain, or be replaced by an
  equivalent.
- Locking, contention, timeout, and restart semantics are unchanged. In
  particular, the shared-lock and exclusive-lock paths must continue to
  interoperate exactly as before.
- The `DurabilityUnknown` skip at `store.rs:941-943` must remain byte-for-byte
  equivalent in behavior: a possibly-committed write must never be followed
  by a directory removal.

## 7. Required tests

At minimum:

- a core test that asserts `Criterion::validate` returns a typed
  `ValidationError` rather than panicking for a requirement that cannot be
  serialized, if such an input is constructible; if it is not constructible,
  the test must instead pin that the function's error surface covers every
  declared failure mode;
- a repository test asserting that the surviving `RepoError` variants are
  each reachable, or an explicit comment in the error enum documenting the
  variant that is not;
- the existing `eggplan-core` and `eggplan-repo` suites, green and unchanged
  apart from the new regressions;
- the existing lock, contention, and pending-closure tests, green and
  unchanged.

A test that asserts a removed item no longer exists is not useful. The
coverage here is behavioral: the same inputs still produce the same results.

## 8. Verification

Record actual results for:

    cargo fmt --all -- --check
    cargo check --workspace --all-targets --locked
    cargo clippy --workspace --all-targets --locked -- -D warnings
    cargo test --workspace --locked
    cargo test -p eggplan-core --locked
    cargo test -p eggplan-repo --locked
    cargo +1.89.0 check --workspace --all-targets --locked
    cargo +1.89.0 test --workspace --locked
    bash scripts/check-core-boundary.sh
    bash scripts/check-closure-authority-boundary.sh
    git diff --check

The `cargo +1.89.0` rows are the MSRV gate and must be recorded truthfully,
including if the toolchain is unavailable in the implementing environment.
Hosted Linux/macOS/Windows and Rust 1.89 qualification must remain green.

## 9. Acceptance criteria

C004 closes when:

1. no public item in `eggplan-core` or `eggplan-repo` is unreachable from a
   consumer, and no removed item is replaced by an `#[allow]` suppression;
2. the `RepoError` taxonomy contains no variant that cannot be constructed,
   or each such variant carries an explicit documented reason;
3. production validation contains no `expect`, `unwrap`, or `panic!` on a
   path reachable from parsed input;
4. no observable behavior changes: identical bytes, errors, and diagnostics
   for identical inputs;
5. the atomic-write durability contract and the `DurabilityUnknown` behavior
   are unchanged;
6. the lock protocol, contention behavior, and timeout behavior are unchanged;
7. the CLI's error mapping remains exhaustive and compiles with `-D warnings`;
8. native and MSRV qualification pass, with the two named boundary guards
   green.

## 10. Stop conditions

Stop and report rather than improvise if:

- removing or keeping `RepoError::Domain` would require changing the CLI's
  diagnostic-code contract, which is a public machine interface;
- the `expect` in `Criterion::validate` turns out to be reachable, which would
  mean the validation path has a real defect and needs its own assessment;
- the discarded cleanup results turn out to affect `store.list()` enumeration,
  which would mean the repository admits empty plan directories and needs a
  separate finding;
- closing the lock check-then-use interval would require platform-specific
  behavior that cannot be qualified across Linux, macOS, and Windows;
- any change would alter the atomic-write or durability contract;
- required verification cannot be obtained honestly.

## 11. Closure evidence required

- implementation commit(s) and PR reference;
- the requirement-to-evidence matrix for every acceptance criterion in §9;
- explicit confirmation that no observable behavior changed;
- the recorded resolution of the `RepoError::Domain` variant and its rationale;
- the recorded resolution of the lock guard's residual race;
- exact verification output, including the MSRV rows, distinguishing planned
  from actually run commands;
- hosted native/MSRV run and job identifiers where available;
- unresolved findings with severity, and roadmap disposition;
- registry updates.

Do not convert a planned command into a passing result because it appears in
§8. If a row was not run, record it as not run or unavailable.

## 12. Handoff notes

- This pass is explicitly non-blocking. It must not gate any capability
  handoff in any subsystem.
- Record truthfully that none of these findings was an observable defect.
  They are unreachable surface, a dead error variant, an unreachable panic,
  and best-effort cleanups whose loss is currently unobservable.
- Do not fold the `AssessmentReason` schema, the execution-kind predicate, or
  the subject-error fidelity work into this pass. Each has its own corrective
  plan, and each is a correctness matter rather than hygiene.
