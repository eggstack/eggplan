# Foundation M003 C004 — Closed

Status: closed

Source plan: plans/implementation/foundation-core/003-c004-unreachable-public-surface-and-panic-path-hygiene.md

Source roadmap: plans/subsystems/foundation-core-roadmap.md

Reviewed baseline: `37ad290ca6b971a68d2871aedb39f4882f4eb2a8`.

Implementation commit: `1e33069271f8a802190078e640f3db942b15ce7a`
("fix: resolve the Foundation M003 corrective lines from the 2026-10-23
repository audit").

Hosted CI was not run for this change. The job identifiers are to be added to
this record as a factual amendment once CI has run, following the precedent of
`plans/closure/evidence-closure/002-c004-closed.md`.

## Executive finding

Foundation M003 C004 closes. The workspace no longer carries public items that
nothing can reach, a production validation path that can panic, or error
variants that nothing constructs.

- `InspectionSelection::is_repository_wide` is removed. The replacement
  assertion at the store site still expresses the intended coverage.
- `RepoError::Domain` is removed. It had zero construction sites anywhere in
  the workspace, and the CLI's error mapping has a catch-all arm, so the
  mapping remains exhaustive over the surviving variants.
- The sole production `expect` in the workspace is gone. The
  `AcceptanceCriterion::validate` duplicate check now reports through the same
  typed `ValidationError` surface as every other validation failure
  (`crates/eggplan-core/src/model.rs:216-221`).
- The discarded cleanup results are named and explained rather than dropped
  silently, and the best-effort behavior is unchanged.
- The lock file check-then-use race is closed by an explicit, documented
  statement that the state root is caller-owned and that the residual window
  is accepted defense-in-depth.

No `#[allow]` suppression was added anywhere.

## Requirement-to-evidence matrix

| # | Acceptance criterion (plan §9) | Evidence |
|---|---|---|
| 1 | No public item in the workspace is unreachable from a consumer | `InspectionSelection::is_repository_wide` removed from `crates/eggplan-repo/src/snapshot.rs` (it had no consumer); `RepoError::Domain` removed from `crates/eggplan-repo/src/store.rs:62` (it had no construction site); `InspectionSnapshot::subject_error()` was disposed of under C002 — kept and consumed at `crates/eggplan-cli/src/lib.rs:745`, `:899`, `:1094`, `:1628`. No `#[allow]` was added |
| 2 | The `RepoError` taxonomy contains no variant that cannot be constructed | 26 variants survive. 23 are constructed directly; the remaining 3 are reachable through `#[from]` conversions on live paths — `Validation` (`crates/eggplan-repo/src/store.rs:926`), `Serialization` (`crates/eggplan-repo/src/store.rs:490`), `Evidence` (`crates/eggplan-repo/src/store.rs:1049`). The invariant is stated in the enum's doc comment (`crates/eggplan-repo/src/store.rs:56-60`) and pinned by `repo_error_variants_are_exhaustively_nameable` (`crates/eggplan-repo/tests/repository.rs:1270`), an exhaustive `match` over the surviving set that also constructs several of them |
| 3 | Production validation contains no `expect`, `unwrap`, or `panic!` on a parsed-input path | `crates/eggplan-core/src/model.rs:216-221` replaces the `expect`. `crates/eggplan-core/src/model.rs:663` — `production_validation_contains_no_panic_macro` scans the production half of the module for `.expect(`, `panic!(`, and `unreachable!(` and fails if any is present. The scan stops at the `#[cfg(test)]` marker so the test's own needles cannot match themselves. Independently verified by the implementing agent: zero panic macros in the production half of any `eggplan-core/src/*.rs` file |
| 4 | No observable behavior change beyond the documented diagnostics changes | The duplicate-requirement check still returns `ValidationError::Duplicate("evidence requirement")` — pinned by `criterion_validation_reports_typed_errors_instead_of_panicking` (`crates/eggplan-core/src/model.rs:610`). The `DurabilityUnknown` skip at `crates/eggplan-repo/src/store.rs:948-957` is behaviorally identical. The lock protocol, timeout behavior, and abandoned-staging handling are unchanged — comments and naming only |
| 5 | The atomic-write durability contract and the `DurabilityUnknown` behavior are unchanged | `crates/eggplan-repo/src/store.rs:948-957` — the `if !matches!(error, RepoError::DurabilityUnknown(_))` guard is preserved verbatim; a possibly-committed write is still never followed by a directory removal |
| 6 | The lock protocol, contention behavior, and timeout behavior are unchanged | Only comments were added at `crates/eggplan-repo/src/store.rs:1369-1375` and `crates/eggplan-repo/src/snapshot.rs:240-246`. `lock_timeout_and_abandoned_staging_are_reported` and `a_sanctioned_writer_cannot_interleave_with_a_held_snapshot` pass |
| 7 | The CLI's error mapping remains exhaustive and the workspace compiles with `-D warnings` | `cargo clippy --workspace --all-targets --locked -- -D warnings` passes. `crates/eggplan-cli/src/lib.rs:2049-2054` still maps `RepoError::InspectionSubject(_)` explicitly and retains its catch-all arm |
| 8 | Native qualification, MSRV qualification, and the boundary guards green | See "Verification executed". Hosted macOS/Windows native runs were **not** performed — see "Unresolved findings" #2 |

## Production implementation evidence

### Unreachable public surface removed

`InspectionSelection::is_repository_wide` is deleted from
`crates/eggplan-repo/src/snapshot.rs`. The store site that previously used it
now states the coverage directly, so the assertion the accessor encoded is not
lost — only the unreachable wrapper is.

`RepoError::Domain` is deleted from `crates/eggplan-repo/src/store.rs:62`. A
workspace-wide search found zero construction sites and no `?`-conversion site
that relied on its `From<Box<dyn Error + Send + Sync>>` implementation. The
CLI's `repo_failure` mapping has a catch-all `other` arm, so removing the
variant cannot leave the mapping non-exhaustive.

The `RepoError` enum now carries the invariant in its own doc comment
(`crates/eggplan-repo/src/store.rs:56-60`): every variant is constructed on a
reachable path, and a variant with no reachable construction must either be
constructed or explicitly documented as reserved.

### Production validation no longer panics

`crates/eggplan-core/src/model.rs:216-221`:

```rust
// The requirement is serializable by construction, but validation runs on
// parsed input and must return a typed error rather than panic if that
// ever stops being true.
let key = serde_json::to_string(req).map_err(|_| {
    ValidationError::Invalid("evidence requirement is not serializable")
})?;
```

`ValidationError::Invalid(&'static str)` already existed, so no new error
variant was introduced and the error taxonomy did not change.

`crates/eggplan-core/src/model.rs:610` —
`criterion_validation_reports_typed_errors_instead_of_panicking` pins that the
duplicate check, the human-judgment check, and the over-bound text check each
return their expected typed `ValidationError`.

`crates/eggplan-core/src/model.rs:663` —
`production_validation_contains_no_panic_macro` scans the production half of
`model.rs` for `.expect(`, `panic!(`, and `unreachable!(` and fails if any is
present. The scan splits the file at the `#[cfg(test)]` marker so the test's
own needles are outside the scanned region.

### Discarded cleanup results made explicit

`crates/eggplan-repo/src/store.rs:943` and `:955` now name the discarded
result (`let _cleanup = fs::remove_dir(&dir);`) and state why the primary
error is the meaningful one. `crates/eggplan-repo/src/store.rs:1363` and
`crates/eggplan-repo/src/snapshot.rs:276` do the same for the `File::unlock()`
result in the two `Drop` implementations, where the destructor cannot return
an error and closing the handle releases the lock regardless.

The deliberate `DurabilityUnknown` skip — a possibly-committed write must never
be followed by a directory removal — is preserved verbatim.

### The lock check-then-use race: recorded, not silently accepted

The symlink guard at `crates/eggplan-repo/src/store.rs:1369-1375` and
`crates/eggplan-repo/src/snapshot.rs:240-246` is a check-then-use pair:
`create(true)` follows symlinks, so a process that can write to the state root
could replace the lock path between the check and the open.

The plan offered two options: close the interval, or record an explicit
statement that the state root is caller-owned and the residual race is accepted
defense-in-depth, preferring the statement when a portable no-follow open is
not available across the supported platforms.

The statement option was taken. A portable no-follow open is not available
across Linux, macOS, and Windows with the current dependency set — `O_NOFOLLOW`
is a Unix-only open flag and Rust's standard library exposes no portable
equivalent — and adding a platform-specific dependency to express it would
have been a larger change than the defect warrants. The statement is recorded
at both lock sites in code and in `architecture/repository.md` ("Cooperative
lock and the accepted residual race"). The guard itself, the lock mode, the
timeout, and the contention behavior are unchanged.

### Tests

| Test | File | Purpose |
|---|---|---|
| `criterion_validation_reports_typed_errors_instead_of_panicking` | `crates/eggplan-core/src/model.rs:610` | Every reachable validation failure mode is a typed `ValidationError` |
| `production_validation_contains_no_panic_macro` | `crates/eggplan-core/src/model.rs:663` | The production half of the module contains no panic macro |
| `repo_error_variants_are_exhaustively_nameable` | `crates/eggplan-repo/tests/repository.rs:1270` | The surviving `RepoError` set is pinned and each variant is constructible |

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
| `cargo test -p eggplan-repo --locked` | pass — 85 passed, 0 failed, 3 ignored (pre-existing) |
| `cargo clippy -p eggplan-repo --all-targets --locked -- -D warnings` | pass |
| `cargo test -p eggplan-cli --locked` | pass — 42 passed, 0 failed |
| `cargo clippy -p eggplan-cli --all-targets --locked -- -D warnings` | pass |

**Not run:** the hosted CI matrix on ubuntu, macOS, and Windows native
runners. CI job identifiers must be added to this record as an amendment once
the change is pushed.

## Invariant review

- **The error taxonomy did not change meaning.** `RepoError::Domain` was
  removed, not repurposed. Every surviving variant keeps its message, its
  CLI mapping, and its semantics.
- **The atomic-write durability contract is untouched.** The
  `DurabilityUnknown` path — a write that may have been committed, whose
  directory must therefore not be removed — is preserved verbatim.
- **The lock protocol is untouched.** Shared and exclusive acquisition, the
  timeout, the abandoned-staging handling, and the symlink guard are all
  unchanged; only comments were added.
- **No panic path was introduced.** The new source-scan test fails the build if
  a panic macro reappears in the production half of `model.rs`.
- **`SubjectCapture` and `finalize_closure_with_capture` remain crate-private.**
  The closure-authority boundary script passes.

## Failure, recovery, and compatibility review

- **Recovery:** unchanged. The pending-closure recovery protocol still
  requires an explicit `--recover-pending`.
- **Contention:** unchanged, and verified by the unchanged contention tests.
- **Compatibility:** removing a public enum variant is a source-level breaking
  change for any downstream that matched on `RepoError::Domain`. No such
  consumer exists in this workspace, and `Domain` was never constructible, so
  no downstream could have observed a `Domain` value except by exhaustive
  matching. The CLI's catch-all mapping arm is unaffected.

## Security, trust, and path review

- **Path safety:** the symlink guard on the lock path is unchanged; the
  residual window is now documented rather than implicit. The state root is
  caller-owned, and the guard is defense in depth for a caller that is not.
- **Trust:** no trust rule changed. No provider trust enrollment was added or
  removed.
- **Failure handling:** replacing a panic with a typed error means a malformed
  input that would previously have aborted the process now produces a normal
  validation failure that the caller can report.

## Documentation and operations

- `architecture/repository.md` — new sections "Error taxonomy" (the invariant)
  and "Cooperative lock and the accepted residual race" (the explicit
  statement).
- `architecture/deep-dive-repository.md` — the `RepoError` variant count and
  range citation were corrected as part of the C005 finding, and the enum's
  invariant is described.
- `architecture/deep-dive-core-domain.md` — the review finding that described
  `model.rs` as having a sole production `expect` was rewritten to record the
  fix and the two pinning tests.

## Unresolved findings

| # | Severity | Finding | Disposition |
|---|---|---|---|
| 1 | low | The lock file check-then-use race is **not closed**; it is documented and accepted. | Recorded at `crates/eggplan-repo/src/store.rs:1369-1375`, `crates/eggplan-repo/src/snapshot.rs:240-246`, and in `architecture/repository.md`. Closing it would require a platform-specific no-follow open, which the plan's scope excluded. The residual risk requires write access to the caller-owned state root. |
| 2 | medium | The hosted CI matrix (ubuntu, macOS, Windows native) was not run. | Must be run by CI after the change is pushed; record the job identifiers here as an amendment. |
| 3 | low | `architecture/deep-dive-repository.md` retains further pre-existing stale citations outside the paragraphs this pass touched. | Pre-existing debt, out of this corrective's scope. |

## Roadmap disposition

Foundation-core roadmap M003 corrective line C004 is closed. The
"M003 corrective lines" table in `plans/subsystems/foundation-core-roadmap.md`
is updated to record this closure and its closure record.

The findings that opened this corrective — an unreachable public accessor, an
unconstructible error variant, a production `expect` on a parsed-input path,
silently discarded cleanup results, and an undocumented lock race — are closed.
The lock race is closed by an explicit accepted-risk statement rather than by
a code change, and that disposition is recorded here rather than presented as
a code fix.

## Registry updates

- `plans/registry.md`: the Foundation M003 C004 row in *Registered
  implementation plans* is updated to `closed`, with a pointer to
  `plans/closure/foundation-core/003-c004-closed.md`.
- `plans/registry.md`: the *Open correctives raised by the 2026-10-23
  repository audit* subsection loses its C004 entry, which moves to the closed
  list.
- `plans/subsystems/foundation-core-roadmap.md`: M003 corrective line C004 is
  marked closed.
- No historical closure record was rewritten.
