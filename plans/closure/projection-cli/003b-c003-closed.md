# Projection/CLI M003b C003 — Closed

Status: closed

Source plan: plans/implementation/projection-cli/003b-c003-markdown-interchange-panic-path-and-bound-hygiene.md

Source roadmap: plans/subsystems/projection-cli-roadmap.md

Reviewed baseline: `37ad290ca6b971a68d2871aedb39f4882f4eb2a8`.

Implementation commit: to be recorded once the change is committed; the
commit identifier is to be added to this record as a factual amendment after
the fact, following the precedent of
`plans/closure/evidence-closure/002-c004-closed.md`.

Hosted CI was not run for this change. The job identifiers are to be added to
this record as a factual amendment once CI has run, following the precedent of
`plans/closure/evidence-closure/002-c004-closed.md`.

## Executive finding

Projection/CLI M003b C003 closes. The Markdown interchange path can no longer
panic on untrusted input, and the 512-character bound is one named constant
rather than a literal repeated at every site.

Two findings were addressed.

**The panic path.** `crates/eggplan-markdown/src/lib.rs` resolved the import
format and then matched on it with an `unreachable!()` arm for the
`Auto` variant. The workspace's only production `unreachable!()` sat on the
import path, which is reachable from a file the caller names. The dispatch is
now total by construction rather than by an assertion: a private
`enum ResolvedFormat { Eggplan, Codegg }`
(`crates/eggplan-markdown/src/lib.rs:104-114`) has no representation for
`Auto`, `Auto` is resolved exactly once before the dispatch
(`crates/eggplan-markdown/src/lib.rs:153-161`), and the dispatch match is
exhaustive. A future `ImportFormat` variant becomes a compile error rather
than a panic arm. The last production `unwrap()` on the import path —
`sections[current.unwrap()]` — was also removed
(`crates/eggplan-markdown/src/lib.rs:436-455`).

**The bound hygiene.** The 512-character bound was written as a bare literal
at six sites, with the "one past the limit" probe written separately as 513.
The file actually carried nine bare `512` literals: the six character-bound
sites, plus three occurrences of a distinct concept — a per-entry *byte*
estimate. The character bound is now one private constant
`MAX_SOURCE_TEXT_CHARS: usize = 512`
(`crates/eggplan-markdown/src/lib.rs:29`), referenced by every site that
applies it, and the probe is *derived* from it by
`exceeds_source_text_bound` (`crates/eggplan-markdown/src/lib.rs:1051-1053`)
rather than written separately. The byte estimate became its own named
constant `ESTIMATED_ENTRY_BYTES` (`crates/eggplan-markdown/src/lib.rs:35`).
No bare `512` literal remains outside the two constant definitions and the
test that pins the value.

The behaviour is byte-identical. Ten golden artifacts captured before the
change and re-captured after it are byte-for-byte equal, and those bytes are
now committed as permanent golden fixtures with digest sidecars.

## Requirement-to-evidence matrix

| Plan requirement | Evidence | Status |
|---|---|---|
| The import path contains no `unreachable!()` and no panic reachable from untrusted input | `crates/eggplan-markdown/src/lib.rs:104-114` (`ResolvedFormat`), `:120-129` (`detect_resolved_format`), `:131-136` (`detect_format` wrapper), `:153-161` (the total dispatch); `crates/eggplan-markdown/tests/bound_hygiene.rs:125` (`every_import_format_dispatches_without_a_panic`) and `:181` (`detect_format_never_reports_auto`) | met |
| The 512 bound is one named constant referenced by every site that applies it | `crates/eggplan-markdown/src/lib.rs:29`; application sites `:144`, `:669-680`, `:739-744`, `:924`, `:933` | met |
| The "one past the limit" probe is derived from the constant, not written separately | `crates/eggplan-markdown/src/lib.rs:1051-1053` (`exceeds_source_text_bound`) | met |
| The private truncation helper's shape is aligned with `eggplan-projection`'s equivalent, or the difference is documented | aligned: `crates/eggplan-markdown/src/lib.rs:1063-1072` now returns `(String, bool)`, matching `eggplan-projection`'s `truncate`; call sites `:565-566`, `:609-610`, `:664-665`, `:682-685`, `:688-691` | met |
| The local SHA-256 helper's bare hex prefix is documented as not a canonical digest | `crates/eggplan-markdown/src/lib.rs:1028-1036` (`short_hash`) and `:1038-1044` (`sha256`) | met |
| No imported plan, identifier, loss code, `source_name` handling, or rendered document changes | see "Byte-identity evidence" below; `crates/eggplan-markdown/tests/bound_hygiene.rs:277` and `:296` | met |
| A regression test exercises every `ImportFormat` through the public `import` entry point with no panic | `crates/eggplan-markdown/tests/bound_hygiene.rs:125` | met |
| A test asserts the 512 bound at every site, including the exactly-512 and 513 cases | `crates/eggplan-markdown/tests/bound_hygiene.rs:199`; unit test `crates/eggplan-markdown/src/lib.rs:1426` pins the constant to 512 | met |
| A test asserts the imported identifiers are unchanged | `crates/eggplan-markdown/tests/bound_hygiene.rs:277` (`imported_identifiers_match_the_recorded_fixture`) | met |
| The existing fixtures suite is green with byte-identical golden outputs | `crates/eggplan-markdown/tests/fixtures.rs` — 7 passed, 0 failed | met |

## Production implementation evidence

### The total dispatch

`crates/eggplan-markdown/src/lib.rs:104-114` defines the private
`ResolvedFormat` enum; `:120-129` defines `detect_resolved_format`, the single
detection implementation; `:131-136` keeps `detect_format` as a thin public
wrapper with its signature and behaviour unchanged. `:153-161` resolves `Auto`
once, before the dispatch, and the dispatch match is exhaustive — the
`unreachable!()` is deleted.

The public API is unchanged: the diff of every `pub` item against the baseline
is empty. `ResolvedFormat` is private precisely so that no new public type is
introduced.

### The named bounds

`crates/eggplan-markdown/src/lib.rs:29` declares
`MAX_SOURCE_TEXT_CHARS: usize = 512`. It is referenced by the source-name
bound at `:144`, the report source-status and source-revision bounds at
`:669-680`, the unmapped section-title bound at `:739-744`, the report
`source_name` at `:924`, and the report `truncated` probe at `:933`.
`crates/eggplan-markdown/src/lib.rs:35` declares
`ESTIMATED_ENTRY_BYTES: usize = 512` for the distinct per-entry byte estimate.

`exceeds_source_text_bound` at `:1051-1053` derives the probe from the
constant, so the "one past the limit" case cannot drift from the limit. The
error text is unchanged: it is still exactly `source name exceeds 512
characters`.

### The truncation helper

`crates/eggplan-markdown/src/lib.rs:1063-1072` — `truncate_chars` now returns
`(String, bool)` and no longer writes through an out-parameter, matching the
shape of `eggplan-projection`'s `truncate`. The five call sites were updated
to fold the returned flag into the existing truncation flag, preserving the
previous semantics exactly: a flag that was already true is never cleared.

### Byte-identity evidence

Before touching any source file, a temporary integration test dumped, for five
corpus cases, the canonical `canonical_json(&(plan, report))` bytes and the
`render()` bytes — ten artifacts. After the change, the same dump was taken
again and the temporary harness was deleted.

`diff -r` between the before and after directories was **empty**: zero byte
differences across all ten artifacts.

```
3a67570ec3acc7ff1642c080a480f0ebcad129780999ecfe2d721e79c41ba7d2  corrective.import.json
d25443e074d635e619f80c19541d051b07c1fcde1e025ece8adb4d0f816eb941  corrective.render.md
13d34e65c4e6325abae3c8c82b199edb5c30c85c273bec9a06b1da0a7ffde926  ordinary-closure.import.json
ac1532fe940cc156f52b488b1a996731aa3b82544eecbf39a433788b4ca48a0b  ordinary-closure.render.md
13d34e65c4e6325abae3c8c82b199edb5c30c85c273bec9a06b1da0a7ffde926  ordinary.import.json
83960cf21541b0763fd7942190db2437ba4aec38629d893a462d7d71caf299e7  ordinary-native.import.json
de5b40cb2b4727ccffa1e2e7c047d5d1f9a5814acafa067683b3fa8a5fe45473  ordinary-native.render.md
de5b40cb2b4727ccffa1e2e7c047d5d1f9a5814acafa067683b3fa8a5fe45473  ordinary.render.md
45d1e24ab0f8097ce5c550ab1b1c2cfff2b9a6761d5a125fc025c331ab502796  unmapped.import.json
de643955c373899c91d42c33e69b673617cac5807ca52f203aa485abbbf77c6b  unmapped.render.md
```

Those bytes are now committed as permanent golden fixtures under
`crates/eggplan-markdown/tests/fixtures/golden/` —
`imported-identifiers.json`, `import-render-digests.json`,
`ordinary.import.json`, and `ordinary.render.md`, each with a `.sha256`
sidecar. `crates/eggplan-markdown/tests/bound_hygiene.rs:296` re-checks all
five cases on every run, and `:317` checks the recorded files against their
sidecars.

The coordinator independently recomputed the digests of the recorded fixtures
with `sha256sum` and confirmed that `ordinary.import.json` is
`13d34e65c4e6325abae3c8c82b199edb5c30c85c273bec9a06b1da0a7ffde926` and
`ordinary.render.md` is
`de5b40cb2b4727ccffa1e2e7c047d5d1f9a5814acafa067683b3fa8a5fe45473` — the
same two digests the before-state capture produced, so the recorded fixtures
are the pre-change bytes and not a re-derivation of the post-change ones.

### Tests

`crates/eggplan-markdown/tests/bound_hygiene.rs` (new, 351 lines) holds seven
tests:

- `:125` `every_import_format_dispatches_without_a_panic`
- `:181` `detect_format_never_reports_auto`
- `:199` `source_text_bound_is_enforced_at_every_site_that_applies_it`
- `:277` `imported_identifiers_match_the_recorded_fixture`
- `:296` `imported_plans_and_rendered_documents_match_the_recorded_bytes`
- `:317` `recorded_golden_files_match_their_digest_sidecars`
- `:333` `the_byte_level_anchor_still_matches_the_imported_plan_and_rendered_document`

Four unit tests were added to `crates/eggplan-markdown/src/lib.rs:1426-1470`:
the constant pinned to 512, the derived probe, the `report_base` source-name
bound, and the `truncate_chars` return shape.

## Verification executed

Coordinator, against the integrated tree:

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo check --workspace --all-targets --locked` | pass |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | pass, no warnings |
| `cargo test --workspace --locked` | **339 passed, 0 failed, 3 ignored** |
| `cargo test -p eggplan-markdown --locked` | **32 passed, 0 failed** (18 lib unit tests, 7 `bound_hygiene`, 7 `fixtures`) |
| `bash scripts/check-core-boundary.sh` | pass |
| `bash scripts/check-codegg-compat-boundary.sh` | pass |
| `bash scripts/check-integrations-boundary.sh` | pass |
| `bash scripts/check-projection-cli-boundary.sh` | pass (`projection/CLI boundary check passed`) |
| `bash scripts/check-closure-authority-boundary.sh` | pass |
| `cargo +1.89.0 check --workspace --all-targets --locked` | pass |
| `cargo +1.89.0 test --workspace --locked` | recorded below |
| `git diff --check` | pass |

Implementation agent, scoped to the crate it owns:

| Command | Result |
|---|---|
| `cargo fmt --package eggplan-markdown -- --check` | pass (after one `cargo fmt --package eggplan-markdown` to fix wrapping) |
| `cargo clippy -p eggplan-markdown --all-targets --locked -- -D warnings` | pass, no warnings |
| `cargo test -p eggplan-markdown --locked` | pass — 32 tests, 0 failed |
| `bash scripts/check-projection-cli-boundary.sh` | pass |
| `git diff --check` | pass |

The three ignored tests are the pre-existing characterization tests carried
from the baseline.

## Invariant review

- The Markdown interchange is a loss-aware, bounded import. Every bound it
  applies is now stated in one place and derived, so the limit cannot drift
  from its probe.
- The import path is total. There is no input for which it panics, and no
  future `ImportFormat` variant can introduce one without a compile error.
- The public API of the crate is unchanged. The new types and constants are
  private.
- Import and render bytes are unchanged. The golden fixtures pin them.

## Failure, recovery, and compatibility review

- The behaviour change is nil by construction, and the byte-identity evidence
  proves it for five corpus cases across both import formats and the unmapped
  case.
- The only observable difference in a failure path is that the `unreachable!()`
  arm and the `sections[current.unwrap()]` are gone; both were previously
  unreachable-in-practice, and neither is reachable now.
- No schema version, digest, or persisted byte changed.

## Security, trust, and path review

- The import path is the one place where untrusted file content enters the
  system. Removing the panic path removes the only way a crafted document
  could terminate the process rather than produce a bounded error.
- The bound constant is private and cannot be changed by a caller.
- The golden fixtures contain imported plan content from the repository's own
  corpus. They introduce no new material and contain no credentials.
- No provider trust, evidence, subject authority, or closure is created or
  implied by this change.

## Documentation and operations

- `architecture/deep-dive-markdown.md` — the §4 bounds paragraph, the §4 panic
  audit, the §5 test-strategy paragraphs, and the §3 and §6 finding 6 text
  were rewritten around the named constant and the total dispatch. All 141
  `src/lib.rs` citations in that document were re-verified: 118 were remapped
  by exact-content match, 45 by hand, and each was then checked to resolve in
  range with content matching its claim. The document's status context, which
  previously claimed M003 had "**no** registered implementation plan", was
  corrected.
- `architecture/markdown-interchange.md` — the bounds paragraph now states
  that the 512-character bound applies to the source name, the source
  `Status:` and `Repository baseline:` metadata, and unmapped section titles,
  and a new paragraph states that format selection is total and cannot panic.

Four pre-existing citation errors in the deep dive were corrected: `:978-989`
claimed `truncate_chars` but pointed at `normalize_label`; `:971-976` claimed
`short_hash`/`sha256` but pointed at `normalize_heading`; `:715-735` was one
line off `check_input`; and `:672-674` claimed
`ordered_sections_not_dependency_edges` but pointed at the native warning-code
push.

Three further citations that a previous change could not verify were
re-verified and corrected in a later pass: one into
`crates/eggplan-cli/src/lib.rs` (which had pointed at `list_command` rather
than the Markdown import's `store.create`), one into
`crates/eggplan-repo/src/store.rs` (which had pointed at closure-record
provider-registry code rather than `create`'s `AlreadyExists` and revision-0 /
Draft guards), and the registry and roadmap citations in the document's status
context.

That pass also found a **false status claim** in
`architecture/deep-dive-markdown.md`: it asserted that roadmap M003 "has **no**
registered implementation plan and no closure evidence", while
`plans/registry.md` and `plans/subsystems/projection-cli-roadmap.md` both
record M003 (M003a and M003b) and corrective line C003 as closed. The sentence
now states that M003 and C003 are closed and cites C003's closure record, and
§4 and §6 are re-framed as describing the state of the crate after that line
closed.

## Unresolved findings

1. **Medium — hosted CI has not run for this change.** The ubuntu, macOS, and
   Windows jobs and the MSRV job are outstanding. This record does not claim
   them as passing.
2. **Low — the report/provenance bound asymmetry is unchanged and still
   reported.** `architecture/deep-dive-markdown.md` §6 finding 6 records that
   the same `Status:`/baseline metadata is bounded at 2,000 characters in
   `plan.provenance` but 512 characters in the loss report. This pass made the
   literal a named constant; it did not change the asymmetry, which is a
   separate product decision and is not in this plan's scope.
3. **Informational — `MAX_SOURCE_TEXT_CHARS` is private while the crate's
   other bound constants are public.** The plan forbids public API changes and
   never requires public exposure, so the bound is private and pinned by test
   rather than published. A caller that needs the bound must read the
   documentation.

## Deviations from the plan

1. **The file carried nine bare `512` literals, not six.** The plan enumerates
   six character-bound sites. The other three belong to a distinct concept —
   a per-entry byte estimate — and became `ESTIMATED_ENTRY_BYTES` rather than
   being folded into the character bound. The result is that no bare `512`
   literal remains outside the two constant definitions.
2. **The truncation helper was aligned rather than documented as different.**
   The plan's first-listed option was alignment with `eggplan-projection`'s
   equivalent helper; alignment was achievable and was what was done.
3. **The last production `unwrap()` was removed.** The plan's §3 and §9.1
   require no `unwrap` reachable from untrusted input, and
   `sections[current.unwrap()]` sat on the CodeGG import path.

## Roadmap disposition

`plans/subsystems/projection-cli-roadmap.md` — the M003b corrective lines
C001, C002, and C003 are now all closed. The "### M003b corrective lines"
table gains a Closure column recording this closure record for C003. M004 is
no longer `proposed`; its decisions are recorded in
`plans/adrs/ADR-0005-plan-lifecycle-and-structure-editing-through-the-cli.md`
and its closure is recorded separately.

## Registry updates

`plans/registry.md` — the row for
`plans/implementation/projection-cli/003b-c003-markdown-interchange-panic-path-and-bound-hygiene.md`
is moved from `ready` to `closed` and points at this record. The "Correctives
raised by the 2026-10-23 repository audit" section records this closure in its
Projection/CLI subsection.
