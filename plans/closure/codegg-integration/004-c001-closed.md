# CodeGG Integration M004 C001 — Closed

Status: closed

Source plan: plans/implementation/codegg-integration/004-c001-closure-naming-and-planning-record-truthfulness.md

Source roadmap: plans/subsystems/codegg-integration-roadmap.md

Reviewed baseline: `ee54a0b8e2cb0d843565c54930ad2c21111320db`.

Implementation commit: recorded as a factual amendment after the fact, following
the precedent of `plans/closure/evidence-closure/002-c004-closed.md`.

Hosted CI was not run for this change, and none is required: no production
source, dependency set, or persisted byte changed.

## Executive finding

CodeGG Integration M004 C001 closes. The planning record now says what it
means.

Three findings were recorded as open in earlier closure records.

**One closure record's filename contradicted its recorded status.** The CodeGG
M003 record's *status line* read `closed (condition satisfied)`, the registry
row read `closed`, the subsystem roadmap read `closed`, and the registry's own
roadmap table read `closed/current` — while the *filename* said
`003-conditionally-closed.md`. The history explains it: commit `e5cd212`
created the file with `Status: conditionally closed` and set the registry row
to the same; commit `6644725` then **amended the file in place** — no rename
accompanied it — changing the status line and the registry row while the
filename was left behind. `git log --diff-filter=R -- plans/closure/` returns
nothing, confirming no rename ever happened.

`conditionally closed` **is** a legal registry status
(`plans/registry.md:22`). The vocabulary was never the problem; the filename
was.

**The tooling-governance record repeated the stale combination.**
`architecture/deep-dive-tooling-governance.md` described the CodeGG M003
`closed` row pointing at `003-conditionally-closed.md` as a live historical
qualification. Since `6644725` the closure record itself read closed, so the
combination described nothing real.

**No written rule governed how a closure-record commit is staged.**
`plans/closure/eggstack-integration/003b-closed.md:236` and `:252` record that
a closure record "was committed with a blanket `git add -A` while unrelated
…" work was in flight, and that "The correct workflow going forward is to stage
closure-record changes by explicit path, never `git add -A`, while another
milestone is in progress." The incident was a one-off lapse with **no written
rule to violate**. Verified across the whole repository: `plans/README.md`,
`plans/003-planning-process.md`, `AGENTS.md`, the registry's "Planning
hygiene" section, and `.skills/eggplan-milestone/SKILL.md`'s "## Hygiene"
section contained no staging or commit guidance, and
`.github/workflows/ci.yml` never stages or commits anything.

## Requirement-to-evidence matrix

| Plan requirement (§5) | Evidence | Status |
|---|---|---|
| 1 — rename the record to `plans/closure/codegg-integration/003-closed.md` | the file exists at the new path; the old path does not | met |
| 2 — append a dated erratum recording the former path, the rename's date, and that `6644725` amended the status line in place while leaving the filename behind | see "The erratum" below | met |
| 3 — update every reference to the old path | see "References updated" below | met |
| 4 — correct the tooling-governance record's stale description of the CodeGG M003 combination, and its registry line anchors | `architecture/deep-dive-tooling-governance.md` §4 now names the renamed record and cites registry rows `:58`, `:79`–`:80`, `:83`, `:95`–`:98`; §5 findings 4 and 5 re-anchored to `registry.md:311`–`:313` and `registry.md:107`–`:129` | met |
| 5 — add the staging rule to the skill's "## Hygiene" section and one mirrored bullet to the registry's "Planning hygiene" section | `.skills/eggplan-milestone/SKILL.md:82-85` and `plans/registry.md:538-539` | met |
| 6 — change no closure record's status, finding, severity, or conclusion | see "No conclusion was rewritten" below | met |

## The rename

```text
R  003-conditionally-closed.md   (in plans/closure/codegg-integration/)
   -> 003-closed.md
```

The rename was made with `git mv`, so `git log --follow` survives it. The
history of the former path at the reviewed baseline:

```text
6644725 plans(codegg-integration): close M003, CodeGG consumer landed
e5cd212 plans(codegg-compat): conditionally close repository binding contract
```

`e5cd212` is the original creation commit. `git log --follow` on the new path
reaches it through the rename.

## The erratum

The renamed record carries a dated factual erratum recording that:

- the file was created as `003-conditionally-closed.md` in
  `plans/closure/codegg-integration/` by commit `e5cd212`, with
  `Status: conditionally closed`;
- commit `6644725` later amended the file in place — no rename accompanied it
  — changing the status line to `Status: closed (condition satisfied)` and
  updating the registry row, while the filename was left behind;
- `plans/README.md` states the convention "Closure record:
  `closure/<subsystem>/NNN-status.md`", and enumerating every closure record
  showed this was the only filename whose status suffix disagreed with its
  recorded status;
- `plans/closure/foundation-core/002-conditionally-closed.md` is correctly
  named, because its status really is `conditionally closed`;
- the status line, the condition-satisfaction evidence, the preserved original
  condition text, the roadmap and registry disposition, every finding, and
  every conclusion in the record are unchanged.

The erratum was appended in the same change as the rename, so the record was
never momentarily inconsistent.

## References updated

Six live pointers named the old path and were updated:

| Location | Change |
|---|---|
| `plans/registry.md:83` | the M003 Handoff cell now names `plans/closure/codegg-integration/003-closed.md` |
| `plans/closure/codegg-integration/003-c001-closed.md:15` | the M003 closure pointer |
| `plans/closure/codegg-integration/003-c001-closed.md:27` | the predecessor-closure pointer |
| `plans/implementation/codegg-integration/003-c002-boundary-guard-completeness.md:14` | the predecessor-closure pointer |

Two bare-filename mentions inside **closed** records' unresolved-finding rows
were deliberately left textually intact, because those rows are the historical
evidence of what was open at the time:

- `plans/closure/codegg-integration/003-c002-closed.md:274` — "Registry row
  for CodeGG M003 reads `closed` while its closure file is
  `003-conditionally-closed.md`".
- `plans/closure/evidence-closure/002-c004-closed.md:200` — the same finding.

The rename is explained from the other side by the erratum on the renamed
record, which names the former filename in those two records' terms.

The design record's mention was rewritten rather than merely re-pointed,
because it described a combination that no longer existed.

## No conclusion was rewritten

The diff over `plans/closure/` is three files:

```text
 .../closure/codegg-integration/003-c001-closed.md  |  4 +--
 plans/closure/codegg-integration/003-closed.md     | 34 ++++++++++++++++++++++
 plans/closure/projection-cli/003b-c002-closed.md   | 15 ++++++++++
 3 files changed, 51 insertions(+), 2 deletions(-)
```

- `003-c001-closed.md` — 2 lines changed, both path pointers. No status,
  finding, severity, conclusion, or evidence table touched.
- `003-closed.md` — 34 lines **added**, all of them the dated erratum. The
  record's status line, condition-satisfaction evidence, preserved original
  condition text, and disposition are untouched.
- `003b-c002-closed.md` — 15 lines **added**, all of them a dated factual
  erratum recording a design-record citation anchor that drifted. That record
  belongs to the Projection/CLI line and the drift was worsened by that
  line's own corrective; its conclusion is unchanged.

No closed record's status, finding, severity, conclusion, or evidence table was
changed anywhere in the repository.

## Every closure record's filename agrees with its recorded status

Enumerating every closure record and comparing the filename's status suffix
against the record's own `Status:` line:

```text
DISAGREEMENTS:

total closure records: 35, agreements: 35
```

At the reviewed baseline there were 32 closure records and exactly **one**
disagreement — the CodeGG M003 record this pass renamed. The count is now 35
because this wave added four closure records. The remaining
`002-conditionally-closed.md` is correctly named: its status really is
`conditionally closed`.

## The staging rule

The rule went to `.skills/eggplan-milestone/SKILL.md`'s "## Hygiene" section,
because `AGENTS.md:25` states that `.skills/` holds reusable execution
procedures and that `plans/` is the planning system. A Git-index rule is
procedure, not a plan. That skill already carries the sibling rules that make
closure commits safe, and it is already where the citation-drift errata
discipline lives (`:63-66`).

`.skills/eggplan-milestone/SKILL.md:82-85`:

> - Stage a closure-record commit by explicit path in the Git index. Never
>   `git add -A` while another milestone's work is in flight — a blanket stage
>   sweeps unrelated in-flight changes into a closure commit. This is Git-index
>   staging, not the filesystem staging the ADRs use for atomic writes.

`plans/registry.md:538-539`, the mirrored bullet:

> - Stage a closure-record commit by explicit path in the Git index; never
>   `git add -A` while another milestone's work is in flight.

The wording is deliberate on the last sentence: `plans/adrs/ADR-0002-repository-first-versioned-canonical-state.md:53`
already uses "staging" in the *filesystem* sense for atomic writes, and the two
senses must not be confused.

## Verification executed

| Command | Result |
|---|---|
| `git diff --check` | pass, no output |
| `git status --short` | reviewed; the change set is the rename, the reference updates, the errata, the skill rule, and the registry edits |
| `git log --follow --oneline -- plans/closure/codegg-integration/003-closed.md` | reaches `e5cd212`, the original creation commit, through the rename |
| closure-record filename/status enumeration | 35 records, 0 disagreements |
| `git diff --stat -- plans/closure/` | 3 files, 51 insertions, 2 deletions — all additions are errata, both deletions are path pointers |
| registry path-consistency check | 0 dangling paths in `plans/registry.md` |
| implementation/closure path check | 20 dangling paths, unchanged from the reviewed baseline; all 20 are unqualified `plans/eggplan-assessment-integration/…` paths in the CodeGG repository, a pre-existing condition documented in the registry |

The workspace build and test suite is not required by this plan, because no
production source changed. It was nevertheless run for the wave as a whole and
is recorded in the sibling closure records:

```text
cargo fmt --all -- --check                     pass
cargo check --workspace --all-targets --locked pass
cargo clippy ... -- -D warnings                pass
cargo test --workspace --locked                348 passed, 0 failed, 3 ignored
```

Not run, and recorded as such: the hosted CI matrix. No production source
changed, so no CI qualification is outstanding for this pass.

## Invariant review

- **Historical closure remains evidence.** No closed record's conclusion was
  rewritten. The one record whose filename disagreed with its own status now
  agrees, and the record carries an erratum so the rename is discoverable.
- **A plan is never evidence of a capability.** The staging rule is a
  procedure, placed in the procedure file, and does not claim any capability.
- **A live design record is corrected in place.** The tooling-governance note
  is a live design record, so its stale description was corrected there
  rather than annotated.

## Failure, recovery, and contention review

This pass touches planning documents only. The rename is a Git operation; the
erratum was appended in the same change, so the record was never momentarily
inconsistent — a reader could not observe a window in which the registry
pointed at a file that did not exist under that name.

The staging rule exists because a blanket `git add -A` during concurrent
milestone work sweeps unrelated in-flight changes into a closure commit. The
rule is a procedure, so it cannot fail at runtime; it can only be followed or
not.

## Migration and compatibility review

No migration. The only consumer-visible effect is that a reader navigating by
filename now finds the CodeGG M003 record under the name its own status line
states. Every live pointer to the record was updated in the same change.

## Security and trust review

No trust boundary moved. The pass changes one filename, seven pointers, two
errata, and one procedure rule. It changes no code, no dependency, no
credential handling, and no identity handling.

## Documentation and operations

- `architecture/deep-dive-tooling-governance.md` — the CodeGG M003
  description is corrected and its registry anchors re-anchored; §5 gains
  findings 7 and 8 for the boundary-guard defects closed by the Foundation
  M004 C001 line; every `path:line` citation in the document was re-verified
  (21 tokens, 0 out of range, 0 unresolved).
- `.skills/eggplan-milestone/SKILL.md` — the staging rule.
- `plans/registry.md` — the mirrored hygiene bullet, the corrected M003
  Handoff cell, and this plan's row.

## Unresolved findings

| Finding | Severity | Disposition |
|---|---|---|
| Two closed records' unresolved-finding rows still name the former filename | Low | Deliberate. Those rows are the historical evidence of what was open at the time and are preserved textually intact, per the source plan's §7 and §9.6. The rename is explained from the renamed record's erratum, which names the former filename in the same terms. |
| 20 pre-existing dangling `plans/eggplan-assessment-integration/…` paths in the implementation and closure trees | Low | Pre-existing and unchanged by this pass; these are paths in the CodeGG repository rendered without qualification. Recorded in the registry. Not opened here. |

No finding is left open against a claim this pass makes.

## Roadmap and registry disposition

- CodeGG integration subsystem: unchanged at `closed/current`. The roadmap is
  terminal and this pass does not reopen it; it corrects the naming of one of
  its records.
- The registry row for this plan moves from `ready` to `closed` and points at
  this record.

## Registry updates

`plans/registry.md` — the row for
`plans/implementation/codegg-integration/004-c001-closure-naming-and-planning-record-truthfulness.md`
is moved from `ready` to `closed` and points at this record. The CodeGG M003
Handoff cell is corrected to the renamed record. The subsystem row for CodeGG
integration is unchanged at `closed/current`.
