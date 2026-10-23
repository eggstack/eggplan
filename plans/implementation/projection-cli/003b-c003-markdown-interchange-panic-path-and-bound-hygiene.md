# Projection/CLI M003b C003 — Markdown Interchange Panic-Path and Bound Hygiene

Status: ready

Repository baseline: 37ad290ca6b971a68d2871aedb39f4882f4eb2a8

Source roadmap:

- plans/subsystems/projection-cli-roadmap.md

Long-term requirements:

- plans/000-long-term-specification.md sections 14-15 and 19-20

Related ADRs: ADR-0002.

Predecessor plan/closure:

- plans/implementation/projection-cli/002-loss-aware-markdown-import-and-deterministic-render.md
- plans/closure/projection-cli/002-closed.md
- plans/implementation/foundation-core/003-subject-scope-strict-schema-and-platform-hardening.md
- plans/closure/foundation-core/003-closed.md

Primary class: non-blocking maintenance / hygiene

## 1. Objective

Remove the only `unreachable!()` from the workspace's production code, replace
the Markdown crate's repeated bare `512` literal with a named bound, and
either align or explicitly document the two helper shapes that the Markdown
crate shares with its sibling crates.

This is non-blocking hygiene. It changes no imported plan, no rendered
document, no schema, and no CLI behavior.

## 2. Findings

### E-PC-C003-01 — the Markdown import path contains an `unreachable!()` arm

`crates/eggplan-markdown/src/lib.rs:109-117`:

    let format = match format {
        ImportFormat::Auto => detect_format(input)?,
        explicit => explicit,
    };
    match format {
        ImportFormat::Auto => unreachable!(),
        ImportFormat::Eggplan => import_native(text, source_name),
        ImportFormat::Codegg => import_codegg(text, source_name),
    }

The `ImportFormat::Auto` arm at `:114` is unreachable, because the preceding
`match` at `:109-112` replaces `Auto` with the result of `detect_format`,
and `detect_format` (`:87-96`) returns only `ImportFormat::Eggplan` or
`ImportFormat::Codegg`.

The reachability is real but relies on two facts that live in different
functions and are not coupled by the type system: `detect_format` must never
return `Auto`, and the `Auto` arm must be replaced before the second `match`.
A future change that makes `detect_format` return `Auto` — for example, to
express "no marker found and the input is not CodeGG-shaped" — turns this
arm into a live panic on an untrusted input path.

**Blast radius.** Today: unreachable. Tomorrow, if `detect_format` grows an
`Auto` result, a panic on the Markdown import path, which is reachable from
the CLI's `markdown import` command and therefore from untrusted file input.

**Detection gap.** `crates/eggplan-markdown/tests/fixtures.rs` exercises the
import path through its public API, so it cannot reach the arm either. There
is no test asserting that `detect_format` never returns `Auto`, because that
is the invariant the arm relies on.

This is the only `unreachable!()` in production code in the workspace. The
other `unreachable!()` in the repository is inside a `#[cfg(test)]` module
(`crates/eggplan-repo/src/snapshot.rs:262`, in the shared-lock contention
arm).

### E-PC-C003-02 — the 512-character bound is a repeated bare literal

`crates/eggplan-markdown/src/lib.rs` uses the literal `512` at six sites:

- `:104` — `source_name.chars().take(513).count() > 512`
- `:623` and `:629` — section text bounds
- `:692` — `section.title.chars().count() > 512`
- `:877` — `source_name.map(|s| s.chars().take(512).collect())`
- `:886` — `truncated: source_name.is_some_and(|s| s.chars().take(513).count() > 512)`

Each site also re-derives the "one past the limit" probe as `513`, so the
bound and its probe are written twice per site.

`eggplan-core` publishes a named bounds module
(`crates/eggplan-core/src/lib.rs:44-62`) with one constant per concept —
`ID_CHARS`, `OBJECTIVE_CHARS`, `DESCRIPTION_CHARS`, `CRITERION_CHARS`, and so
on — and its validation uses those names rather than literals. The Markdown
crate's `512` is a distinct concept from any of core's bounds (it bounds
imported prose and titles, not canonical schema fields), so it should be its
own named constant in the Markdown crate rather than a literal repeated six
times.

**Blast radius.** No behavior change today. The risk is that the six copies
drift, so that one field is bounded at 512 characters and another at 511 or
513, with no compiler signal.

### E-PC-C003-03 — the truncation helper's shape differs from its sibling crates (low)

`crates/eggplan-markdown/src/lib.rs:988-999`:

    fn truncate_chars(value: &str, max: usize, truncated: &mut bool) -> String

`crates/eggplan-projection/src/lib.rs:478-482` returns `(String, bool)`. The
two shapes express the same idea, and a reader moving between the crates has
to translate between them.

### E-PC-C003-04 — the Markdown crate carries its own SHA-256 helper (low)

`crates/eggplan-markdown/src/lib.rs:981-986`:

    fn short_hash(value: &str) -> String {
        sha256(value.as_bytes())[..24].to_string()
    }
    fn sha256(value: &[u8]) -> String {
        format!("{:x}", Sha256::digest(value))
    }

`eggplan-core` exposes `digest_json` (`crates/eggplan-core/src/lib.rs:31`),
which produces the canonical `sha256:<64 hex>` form. The Markdown helper
produces a bare 24-character hex prefix and is used only to derive imported
identities, never as a canonical digest. The two forms are not interchangeable
and the Markdown crate's use is correct, but the distinction is implicit: no
comment states that the bare hex here is deliberately not a canonical digest.

## 3. Controlling semantics

From `architecture/markdown-interchange.md` and the subsystem's §3 invariant
"Markdown import never manufactures evidence from prose": the Markdown crate
is a bounded parser and renderer whose failure modes must be typed, not
panics. From `architecture/core.md`: bounds are named, explicit constants
rather than repeated literals.

Required behavior after this pass:

- the Markdown import path contains no `unreachable!()`, `panic!`, `expect`,
  or `unwrap` reachable from untrusted input;
- the `512`-character bound is one named constant, referenced by every site
  that uses it, with the "one past the limit" probe derived from it rather
  than written separately;
- the truncation helper's shape and the local hash helper's form are each
  either aligned with the sibling crates or explicitly documented as
  deliberate;
- no imported plan, rendered document, schema, digest, or CLI output changes.

## 4. Scope

### In scope

- `crates/eggplan-markdown/src/lib.rs` — the import-format dispatch, the
  repeated `512` literal, the truncation helper, and the local hash helper;
- `crates/eggplan-markdown/tests/fixtures.rs` — regression coverage;
- documentation updates in `architecture/markdown-interchange.md` and
  `architecture/deep-dive-markdown.md`;
- registry and roadmap lineage updates.

### Out of scope

- the Markdown grammar, the loss codes, the native marker and fence
  constants, and the CodeGG subset rules;
- the imported plan's schema, generated identifiers, and their determinism;
- the render format and its byte output;
- the CLI's `markdown` command surface, including `markdown import`'s
  mandatory `--state-root` behavior;
- the `MAX_INPUT_BYTES`, `MAX_LINES`, `MAX_LINE_BYTES`, and `MAX_SECTIONS`
  constants at `crates/eggplan-markdown/src/lib.rs:15-18`, which are already
  named;
- the projection crate's truncation helper shape, unless the implementer
  chooses to align the two, in which case the projection crate's public API
  must not change;
- `eggplan-core`'s bounds module, which is already named and correct;
- the CLI's documentation corrections and the completion-depth work;
- the plan lifecycle and structure-editing capability gap;
- the deferred Interop/distribution subsystem.

## 5. Required production changes

Preferred shape:

- remove the `unreachable!()` arm by making the format dispatch total. The
  recommended shape is to have `detect_format` return the two concrete
  formats it can return, and to have the import path dispatch on a type that
  cannot represent `Auto` — for example by resolving the requested format
  once, before the dispatch, into a local value whose type excludes `Auto`.
  If that is not expressible without a new public type, the alternative is
  to keep the shape and replace the `unreachable!()` with a typed
  `MarkdownError`, so the failure is a diagnostic rather than a panic;
- introduce one named constant for the 512-character bound in
  `crates/eggplan-markdown/src/lib.rs`, and derive the "one past the limit"
  probe from it at every site, so that the bound and its probe cannot drift
  apart;
- for the truncation helper, either align its return shape with
  `crates/eggplan-projection/src/lib.rs:478-482` or record a comment stating
  why the Markdown crate's shape differs. The helper is private, so either
  choice is internal;
- for the local hash helper, add a comment stating that the bare hex prefix
  is deliberately not a canonical `sha256:` digest and is used only to derive
  imported identities.

Explicitly rejected alternatives:

- making `detect_format` return `ImportFormat::Auto` for unrecognized input.
  That is a behavior change to the import contract, and the arm's current
  existence is a symptom of a design choice, not a defect in the dispatch
  itself;
- removing the `512` bound entirely. Bounds are required by the subsystem's
  invariant;
- changing the imported plan's identifier derivation to use core's canonical
  digest, which would change every imported identifier and therefore every
  imported plan's identity.

## 6. Failure, cancellation, restart, and contention semantics

- No imported plan may change. In particular, the generated identifiers, the
  truncated-field markers, the loss codes, and the `source_name` handling must
  be byte-identical before and after.
- No rendered document may change.
- The `512` bound must remain 512 characters. Introducing a named constant
  must not become an opportunity to change the value.
- A change in the import path's failure mode from panic to typed error is
  strictly an improvement, but no input that previously imported successfully
  may fail, and no input that previously failed with a typed error may
  succeed or change its error message.
- No locking, persistence, or restart behavior is involved.

## 7. Required tests

At minimum:

- a regression test that exercises every `ImportFormat` value through the
  public `import` entry point, asserting that each behaves as specified and
  that none produces a panic;
- a regression test asserting that `detect_format` never returns `Auto`, if
  the arm is retained in any form;
- a test asserting the 512-character bound is enforced at every site that uses
  it, including the boundary cases at exactly 512 and 513 characters;
- the existing `crates/eggplan-markdown/tests/fixtures.rs` suite, green and
  byte-identical in its golden outputs;
- a test asserting the imported plan's identifiers are unchanged by this
  pass, by comparing against a recorded fixture.

## 8. Verification

Record actual results for:

    cargo fmt --all -- --check
    cargo check --workspace --all-targets --locked
    cargo clippy --workspace --all-targets --locked -- -D warnings
    cargo test --workspace --locked
    cargo test -p eggplan-markdown --locked
    cargo +1.89.0 check --workspace --all-targets --locked
    cargo +1.89.0 test --workspace --locked
    bash scripts/check-projection-cli-boundary.sh
    git diff --check

The `cargo +1.89.0` rows are the MSRV gate and must be recorded truthfully,
including if the toolchain is unavailable in the implementing environment.
Hosted Linux/macOS/Windows and Rust 1.89 qualification must remain green.

## 9. Acceptance criteria

C003 closes when:

1. the Markdown crate's production source contains no `unreachable!()` and no
   panic reachable from untrusted input;
2. the 512-character bound is one named constant used by every site that
   applies it, with no separately written "one past the limit" literal;
3. the truncation helper's shape and the local hash helper's form are each
   aligned or explicitly documented as deliberate;
4. every imported plan and every rendered document is byte-identical to its
   pre-pass form;
5. no schema, digest, loss code, CLI output, or public API changes;
6. `architecture/markdown-interchange.md` and
   `architecture/deep-dive-markdown.md` state the bounds and the import
   format dispatch accurately;
7. native and MSRV qualification pass, with the projection/CLI boundary guard
   green.

## 10. Stop conditions

Stop and report rather than improvise if:

- making the format dispatch total requires changing `detect_format`'s public
  behavior or its contract, which is a separate decision;
- any imported plan, generated identifier, loss code, or rendered document
  changes, which means the change was not behavior-preserving;
- the 512-character bound would have to change value;
- the projection crate's public API would have to change to align the
  truncation helper shapes;
- required verification cannot be obtained honestly.

## 11. Closure evidence required

- implementation commit(s) and PR reference;
- the requirement-to-evidence matrix for every acceptance criterion in §9;
- the recorded before/after golden comparison for imported plans and rendered
  documents;
- explicit confirmation that the 512-character bound value is unchanged;
- exact verification output, including the MSRV rows, distinguishing planned
  from actually run commands;
- hosted native/MSRV run and job identifiers where available;
- unresolved findings with severity, and roadmap disposition;
- registry updates.

Do not convert a planned command into a passing result because it appears in
§8. If a row was not run, record it as not run or unavailable.

## 12. Handoff notes

- This is non-blocking hygiene and must not gate any capability handoff.
- Record truthfully that the `unreachable!()` is unreachable today. The defect
  is that a panic arm in an untrusted-input path depends on a coupling the
  type system does not enforce.
- Do not fold the CLI documentation corrections, the completion-depth work, or
  the plan-lifecycle capability gap into this pass.
