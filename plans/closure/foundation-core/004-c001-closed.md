# Foundation Core M004 C001 — Closed

Status: closed

Source plan: plans/implementation/foundation-core/004-c001-boundary-guard-dependency-key-patterns.md

Source roadmap: plans/subsystems/foundation-core-roadmap.md

Reviewed baseline: `ee54a0b8e2cb0d843565c54930ad2c21111320db`.

Implementation commit: recorded as a factual amendment after the fact, following
the precedent of `plans/closure/evidence-closure/002-c004-closed.md`.

Hosted CI was not run for this change. The job identifiers are to be added to
this record as a factual amendment once CI has run, following the precedent of
`plans/closure/evidence-closure/002-c004-closed.md`.

## Executive finding

Foundation M004 C001 closes. The boundary guards now fail on every spelling of
the dependency they forbid.

The defect this pass closes was found by re-reading the guards against the
manifests they are meant to police, and it was **verified directly, not
inferred**. Seven anchored manifest rules matched a dependency name only when
the name was immediately followed by whitespace, `=`, or end of line. This
repository declares **every** shared dependency in the workspace-inheritance
form `name.workspace = true`, where a `.` follows the name. A dependency
declared that way was invisible to all seven rules.

Reproduced in a scratch copy of the tree, against the pre-fix scripts:

```text
=== A: plain spelling (expect MATCH) ===
2:tokio = "1"
exit=0
=== B: .workspace spelling (expect NO MATCH = BYPASS) ===
exit=1
=== C: transport, .workspace spelling (expect NO MATCH = BYPASS) ===
exit=1
```

`tokio.workspace = true` and `eggbench-core.workspace = true` passed the exact
guards written to forbid them. The guards protecting the sibling-runtime and
transport boundaries — the rules that exist for the express purpose of keeping
an adapter from acquiring authority it must not have — reported success
against a manifest that declared exactly what they forbid.

Two further instances of the same defect class were confirmed in the same
pass: a renamed dependency (`other = { package = "reqwest" }`) and a
hyphenated sub-crate (`tokio-util = "1"`) were equally invisible, because the
rules modelled a dependency *key* rather than a dependency.

A separate finding, recorded in the CodeGG C002 closure record, stated that a
production-only `tempfile` rule "cannot be added to the unscoped guard 3
without failing the tree". That premise was wrong in a way that mattered: the
rule as written does not match this repository's own spelling of the
dependency, so adding it would have been inert rather than failing. The
core guard's source pattern also matched every other impure capability but
not `tempfile::`, which its two sibling guards did match.

## Requirement-to-evidence matrix

| Plan requirement (§9) | Evidence | Status |
|---|---|---|
| 1 — `tokio.workspace = true` fails the integrations transport rule and the core prohibited-dependency rule | negative proof cases 1, 2, 18, 19; self-proofs "guard1 rejects: tokio.workspace = true" and "guard2 rejects tokio by workspace inheritance" | met |
| 2 — `eggbench-core.workspace = true` fails the integrations sibling-runtime rule | negative proof case 16; self-proof "guard1 rejects a sibling runtime by workspace inheritance" | met |
| 3 — `codegg.workspace = true` and `codegg-core.workspace = true` fail the codegg-compat rule | negative proof cases 11–12; self-proofs "guard2 rejects codegg by workspace inheritance" and "guard2 rejects codegg-core by workspace inheritance" | met |
| 4 — `eggplan-repo.workspace = true` fails in `[dependencies]` and is still allowed in `[dev-dependencies]` | negative proof case 9; self-proofs "guard1 rejects production eggplan-repo by workspace inheritance" and "guard1 allows dev eggplan-repo by workspace inheritance" | met |
| 5 — `secrecy.workspace = true` fails the integrations credential rule | negative proof case 21; self-proof "guard3 rejects secrecy by workspace inheritance" | met |
| 6 — `my-http = { package = "reqwest", version = "0.12" }` fails the integrations transport rule | negative proof cases 20, 22, 14, 5 | met |
| 7 — `tokio-util = "1"` fails the core prohibited-dependency rule | negative proof cases 4, 13, 19; self-proof "guard1 rejects: tokio-util = \"1\"" | met |
| 8 — `tempfile::NamedTempFile::new()` in a core source file fails the core source rule | negative proof case 6; self-proof "guard2 rejects: let _ = tempfile::NamedTempFile::new();" | met |
| 9 — a production `tempfile` dependency fails in all six pure crates and is still allowed in `eggplan-repo` | see "Criterion 9, verified crate by crate" below | met |
| 10 — every legal dependency declaration in the real tree still passes every guard | all five boundary scripts pass against the real tree; see "Verification executed" | met |
| 11 — every guard's self-proof suite still passes, and the new self-proofs are present for each newly-matched spelling | see "Self-proof output" below | met |
| 12 — `architecture/deep-dive-tooling-governance.md` states the guards' actual behavior, and every `path:line` citation in it is verified | see "Design record" below | met |

## Production implementation evidence

No production Rust source changed. No crate manifest, dependency set, lock
file, schema version, digest, or golden byte changed. The change is confined
to three shell guards, one design record, and one closure-record erratum.

**The shared dependency-key matcher.** Each of the three guards now derives its
manifest patterns from one helper that models every spelling Cargo and this
repository use:

- `scripts/check-core-boundary.sh:105-114` — `dep_pattern`
- `scripts/check-codegg-compat-boundary.sh:67-76` — `dep_pattern`
- `scripts/check-integrations-boundary.sh:62-68` — `dep_pattern`

The four spellings the matcher covers, each anchored to a dependency-key
position so that a comment, a field value, or an unrelated crate whose name
merely contains a forbidden name is not a false positive:

| Spelling | Example |
|---|---|
| plain | `tokio = "1"` |
| workspace inheritance | `tokio.workspace = true` |
| renamed | `other = { package = "tokio" }` |
| table form | `[dependencies.tokio]` |

A hyphenated sub-crate is matched as its family (`tokio-util` is `tokio`).
The one opt-out is `eggplan-repo`: a crate named `eggplan-repo-*` would be a
plausible future sibling rather than a sub-crate of the forbidden
`eggplan-repo`, so that name alone is matched without the suffix
(`scripts/check-codegg-compat-boundary.sh:112`).

**The production-filesystem rule.** The core guard gained a fourth guard,
`no_production_tempfile_dep` (`scripts/check-core-boundary.sh:201-218`), which
extracts the `[dependencies]` section with `production_section`
(`scripts/check-core-boundary.sh:120-127`) and applies the same matcher to it.
The scan set is the six pure-crate manifests at
`scripts/check-core-boundary.sh:11-18`. `eggplan-repo` is excluded by that
list, not by an in-rule exception: it is the one crate whose production
`tempfile` dependency is legitimate, because its atomic write path is built on
it (`crates/eggplan-repo/src/store.rs:1496-1518`).

**Fail-closed manifest rules.** The codegg-compat and integrations guards now
route their manifest rules through `require_clean`
(`scripts/check-codegg-compat-boundary.sh:158-173`,
`scripts/check-integrations-boundary.sh:74-89`), which maps ripgrep's exit
status so that a scan which could not be performed fails closed instead of
being reported as a pass. This matters here because the patterns changed: a
malformed pattern would otherwise have exited non-zero and been read as
"clean".

**The root `[workspace.dependencies]` table remains unscanned.** That is
correct: it legitimately declares `git2` and `tempfile` for `eggplan-repo`.
The dependency *edge* that matters is the one in each crate's own manifest, so
scanning the crate manifests with a complete key matcher is sufficient and
avoids false positives from the shared table.

## Criterion 9, verified crate by crate

Each pure crate's manifest was given a production `tempfile` dependency in a
scratch copy of the tree, and the core guard was run against it:

```text
  eggplan-core             rejected (exit 1)
  eggplan-projection       rejected (exit 1)
  eggplan-markdown         rejected (exit 1)
  eggplan-integrations     rejected (exit 1)
  eggplan-codegg-compat    rejected (exit 1)
  eggplan-cli              rejected (exit 1)
```

With `eggplan-repo` left as it is:

```text
  eggplan-repo          allowed (its production tempfile is excluded by the scan set)
```

The exemption is a deliberate scan-set exclusion rather than an inert rule: the
guard's own self-proof "the eggplan-repo exemption is deliberate, not an inert
rule" asserts that the same rule *does* flag that manifest when it is scanned.

## The bypass is closed

The original reproduction, re-run against the repaired guards in a scratch
copy of the tree:

```text
--- integrations guard against the workspace-inheritance manifest ---
integrations boundary self-proofs passed
3:eggbench-core.workspace = true
integrations exit=1
provider adapters must not depend on a sibling runtime in any section, including [dev-dependencies] (scanned: crates/eggplan-integrations/Cargo.toml, all sections)
```

Where the pre-fix guard exited `0` (success), the repaired guard exits `1`.

## Negative proof

The guards' self-proofs are their tests, but the plan (§8) additionally
required that each guard be run against a scratch manifest carrying each
newly-forbidden spelling and be observed to fail. 23 cases were run against a
pristine `git archive` copy of the tree with the working-tree guard scripts
installed over it:

```text
setup: the pristine scratch tree passes all three guards
ok   (exit 1) core guard1 rejects tokio.workspace = true
ok   (exit 1) core guard1 rejects tokio.workspace=true
ok   (exit 1) core guard1 rejects a renamed reqwest dependency
ok   (exit 1) core guard1 rejects the tokio sub-crate
ok   (exit 1) core guard1 rejects the table form
ok   (exit 1) core guard2 rejects tempfile:: in source
ok   (exit 1) core guard4 rejects a production tempfile dependency in eggplan-projection
ok   (exit 1) core guard4 rejects a renamed production tempfile dependency in eggplan-cli
ok   (exit 1) codegg guard1 rejects a production eggplan-repo by workspace inheritance
ok   (exit 1) codegg guard1 rejects a renamed production eggplan-repo
ok   (exit 1) codegg guard1 rejects a production eggplan-repo table
ok   (exit 1) codegg guard2 rejects codegg.workspace = true
ok   (exit 1) codegg guard2 rejects a renamed codegg dependency
ok   (exit 1) codegg guard3 rejects tokio.workspace = true
ok   (exit 1) codegg guard3 rejects the tokio sub-crate
ok   (exit 1) codegg guard3 rejects a renamed reqwest dependency
ok   (exit 1) integrations guard1 rejects a sibling runtime by workspace inheritance
ok   (exit 1) integrations guard1 rejects a renamed sibling runtime
ok   (exit 1) integrations guard2 rejects tokio.workspace = true
ok   (exit 1) integrations guard2 rejects the tokio sub-crate
ok   (exit 1) integrations guard2 rejects a renamed reqwest dependency
ok   (exit 1) integrations guard3 rejects secrecy.workspace = true
ok   (exit 1) integrations guard3 rejects a renamed secrecy dependency

negative proof: 23 rejected, 0 wrongly passed
```

## Self-proof output

Each guard's self-proof suite runs against `mktemp -d` fixtures and covers the
new spellings in both directions — rejecting each forbidden spelling, and
still allowing a clean manifest written in the inheritance form, a legal
dev-dependency, and a commented-out dependency:

```text
eggplan-core boundary self-proofs passed
eggplan-core static boundary check passed
codegg-compat boundary self-proofs passed
eggplan-codegg-compat ownership boundary check passed
integrations boundary self-proofs passed
integration provider SPI boundary check passed
```

## Verification executed

All commands were run from the repository root at the reviewed baseline plus
this pass's changes.

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | pass, no output |
| `cargo check --workspace --all-targets --locked` | pass |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | pass, no warnings |
| `cargo test --workspace --locked` | 348 passed, 0 failed, 3 ignored |
| `bash scripts/check-core-boundary.sh` | pass |
| `bash scripts/check-codegg-compat-boundary.sh` | pass |
| `bash scripts/check-integrations-boundary.sh` | pass |
| `bash scripts/check-projection-cli-boundary.sh` | pass |
| `bash scripts/check-closure-authority-boundary.sh` | pass |
| `cargo +1.89.0 check --workspace --all-targets --locked` | pass |
| `cargo +1.89.0 test --workspace --locked` | 348 passed, 0 failed, 3 ignored |
| `git diff --check` | pass, no output |

Not run, and recorded as such: the hosted CI matrix (ubuntu/macOS/Windows
native plus the MSRV job) has not run for this change. The local evidence
above covers Linux only. The boundary scripts are skipped on Windows runners
by design, so the Windows leg remains unverified here.

## Invariant review

- **Core stays pure.** `eggplan-core`'s manifest and source are unchanged. The
  guards now enforce that property against every spelling rather than one.
- **Adapters stay authority-free.** The sibling-runtime and transport
  boundaries are the rules that keep an adapter from acquiring authority it
  must not have. They were previously defeatable with a one-character change
  to a manifest line; they now are not.
- **No silent weakening.** The new patterns are strictly stronger on the
  forbidden spellings and strictly more precise on the permitted ones: a
  commented-out dependency is not a dependency, and `eggplan-codegg-compat` is
  not a sibling runtime.
- **Section scoping preserved.** The two production-only rules are
  section-scoped, so the repository's legal dev-dependencies on `tempfile`
  (`crates/eggplan-cli/Cargo.toml:22`,
  `crates/eggplan-codegg-compat/Cargo.toml:15`) still pass.
- **No schema, digest, or golden byte moved.** No crate manifest, dependency
  set, or lock file changed; `cargo test --locked` is green unchanged.

## Failure, recovery, and contention review

The guards are read-only and idempotent. They take no lock and hold no state.
Each now fails closed: a missing tool is refused by the precondition, a
manifest that cannot be read is reported and exits non-zero, and a ripgrep
exit status above 1 is treated as "the scan could not be performed" rather
than "clean". A guard that cannot scan never reports a pass.

The three guards read only their own manifests and their own source trees, so
they are safe to run concurrently with each other and with a build.

## Migration and compatibility review

No migration. No data format changed. No public API changed. The only
user-visible effect is that a manifest spelling which previously passed a
guard now fails it, which is the point of the pass.

## Security and trust review

No trust boundary moved. The change strictly tightens three enforcement
points. It does not add a dependency, does not change any credential or
identity handling, and does not expose anything new.

`eggplan-repo`'s legitimate production `tempfile` dependency is preserved: the
exemption is by scan set, so a *new* crate taking a production filesystem
dependency fails the guard without any further configuration.

## Documentation and operations

`architecture/deep-dive-tooling-governance.md` is the live design record for
this tooling and was corrected in place:

- §1 now describes all four core guards, all five codegg-compat guards, and
  all seven integrations checks, including the shared dependency-key matcher
  and the four declaration spellings it covers.
- §5 gains two findings (7 and 8) recording the two defects this pass closed.
- Every `path:line` citation in the document was re-verified against the file
  it names: 21 citation tokens, 0 out of range, 0 unresolved.

`plans/closure/codegg-integration/003-c002-closed.md` carried the
`tempfile` finding as open with a premise this pass disproved. That record is
closed history, so it was **not** rewritten; it carries a dated factual
erratum instead, and its conclusions are unchanged.

## Unresolved findings

| Finding | Severity | Disposition |
|---|---|---|
| The hosted CI matrix has not run for this change | Informational | Recorded above as not run. The job identifiers are to be added to this record as a factual amendment once CI completes. |
| The boundary scripts are skipped on Windows runners, so the repaired guards are unverified on Windows | Low | Pre-existing and unchanged by this pass; recorded in `architecture/deep-dive-tooling-governance.md` finding 1. Not a new gap and not a new risk. |
| A dependency declared in a `[dependencies.<name>]` table *body* rather than as a key (for example a `[dependencies.foo]` section whose `bar = "1"` line names a forbidden crate as a feature of `foo`) is not matched | Low | A feature name, not a dependency. The dependency itself is matched by its table header, which the matcher does cover. Recorded as a known limit of the key-position model, not a bypass of the forbidden dependency. |

No finding is left open against a claim this pass makes.

## Roadmap and registry disposition

- Foundation core/repository subsystem: unchanged at `closed/current`. This
  corrective did not open or gate any milestone and did not serialize another
  subsystem.
- The registry row for this plan moves from `ready` to `closed` and points at
  this record.

## Registry updates

`plans/registry.md` — the row for
`plans/implementation/foundation-core/004-c001-boundary-guard-dependency-key-patterns.md`
is moved from `ready` to `closed` and points at this record. The subsystem
row for Foundation core/repository is unchanged at `closed/current`.
