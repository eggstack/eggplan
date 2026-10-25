# CodeGG Integration M004 C001 — Closure-Record Naming and Planning-Record Truthfulness

Status: closed

Repository baseline: ee54a0b8e2cb0d843565c54930ad2c21111320db

Source roadmap:

- plans/subsystems/codegg-integration-roadmap.md

Long-term requirements:

- plans/000-long-term-specification.md (the planning record is the evidence of
  what was accepted)

Related ADRs: ADR-0001.

Predecessor plan/closure:

- plans/closure/codegg-integration/003-closed.md
- plans/closure/eggstack-integration/003b-closed.md

Primary class: planning-record truthfulness

## 1. Objective

Make the planning record say what it means.

Three findings, each recorded in an earlier closure record and each still
open:

- the CodeGG M003 closure record's **filename** says "conditionally closed"
  while its status line, the registry row, the subsystem roadmap, and the
  registry's own roadmap table all say `closed`;
- the repository has **no written rule** about how a closure-record commit
  must be staged, even though a blanket stage once swept another milestone's
  in-flight work into a closure commit;
- the tooling-governance design record still describes the CodeGG M003
  combination as a live qualification, which stopped being true when the
  condition was satisfied.

This pass renames one closure record, appends one erratum to it, corrects the
references to it, and adds one procedure rule. No production Rust source is
edited, and no closed record's **conclusion** is changed.

## 2. Findings

### E-CG-C001-01 — one closure record's filename contradicts its recorded status

| Artifact | Recorded status | Location |
|---|---|---|
| Registry row | `closed` | `plans/registry.md:83` |
| Closure record | `closed (condition satisfied)` | `plans/closure/codegg-integration/003-closed.md:3` |
| Subsystem roadmap | `closed` | `plans/subsystems/codegg-integration-roadmap.md:126` |
| Registry roadmap table | `closed/current` | `plans/registry.md:59` |

The registry status and the closure record's status **agree**. What is broken
is the filename.

The history explains it. Commit `e5cd212` created
`003-conditionally-closed.md` with `Status: conditionally closed` and set the
registry row to the same. Commit `6644725` then **amended the file in
place** — `git log --diff-filter=R -- plans/closure/` returns nothing, so no
rename happened — changing line 3 to `Status: closed (condition satisfied)`
and changing the registry row's status cell, while the registry's Handoff
cell kept pointing at the old filename. `git log -S '003-conditionally-closed.md'
-- plans/registry.md` returns only `e5cd212`: the reference has never been
updated since.

The condition was satisfied, and the record says so: CodeGG implementation
`53dea47f`, hosted run `36938461935` success
(`plans/closure/codegg-integration/003-closed.md:108-121`). The
original condition text is preserved verbatim under
`## Unresolved condition (as originally recorded)` (`:128-133`) and the
disposition states "(Originally `conditionally closed`.)" (`:137-139`).

The naming convention is explicit: `plans/README.md:65` — "Closure record:
closure/<subsystem>/NNN-status.md". Enumerating all 33 closure records, this
is the **only** one whose filename contradicts its recorded status.
`plans/closure/foundation-core/002-conditionally-closed.md` is correctly
named, because its status really is `conditionally closed`.

`conditionally closed` **is** a legal registry status
(`plans/registry.md:22`). The vocabulary is not the problem; the filename is.

### E-CG-C001-02 — the tooling-governance record repeats the stale combination

`architecture/deep-dive-tooling-governance.md:331-333` describes the CodeGG
M003 `closed` row pointing at `003-conditionally-closed.md` as a live
historical qualification. Since `6644725` the closure record itself reads
closed, so the combination no longer describes anything real.

The same document's own registry line anchors are also off — `:67`, `:82`,
`:78`-`:79`, `:94`-`:96`, `:97`, and `:57` point at the wrong lines of
`plans/registry.md`, whose rows moved when the audit's corrective lines were
registered.

### E-CG-C001-03 — no written rule governs how a closure-record commit is staged

`plans/closure/eggstack-integration/003b-closed.md:236` and `:252` record:

> "…closure record was committed with a blanket `git add -A` while unrelated …"
> "The correct workflow going forward is to stage closure-record changes by
> explicit path, never `git add -A`, while another milestone is in progress."

The incident was a one-off lapse with **no written rule to violate**.
Verified across the whole repository: `plans/README.md`, `plans/003-planning-process.md`,
`AGENTS.md`, `plans/registry.md`'s "Planning hygiene" section, and
`.skills/eggplan-milestone/SKILL.md`'s "## Hygiene" section contain no
staging or commit guidance. `.github/workflows/ci.yml` never stages or
commits anything.

The rule belongs in `.skills/eggplan-milestone/SKILL.md`'s "## Hygiene"
section. `AGENTS.md:25` states that `.skills/` holds reusable execution
procedures and that `plans/` is the planning system; a Git-index rule is
procedure, not a plan. That skill already carries the sibling rules that
make closure commits safe, and it is already where the citation-drift errata
discipline lives (`:63-66`).

The rule must be worded so it cannot be confused with *filesystem* staging,
which the ADRs already use for atomic writes
(`plans/adrs/ADR-0002-repository-first-versioned-canonical-state.md:53`).

## 3. Controlling semantics

- A closure record's name states its status. A record whose name and status
  disagree is read wrongly by anyone who navigates by name.
- Historical closure remains evidence of what was accepted at the time
  (`plans/closure/README.md:36-38`). A rename changes no conclusion; it makes
  the record's name agree with the conclusion it already states.
- A rename must not be silent. The renamed record carries a dated erratum
  naming its former path, so `git log --follow` and any reader of the old
  path can find the history.
- A forward-looking process rule belongs in the procedure file, not in a
  closure record and not in a plan.
- The design record must not describe a state that no longer exists.

## 4. Scope

In scope:

- the file `003-conditionally-closed.md` in
  `plans/closure/codegg-integration/` — renamed
  to `003-closed.md`, with an appended erratum
- `plans/registry.md` — the CodeGG M003 Handoff cell, and the Planning
  hygiene section
- `plans/closure/codegg-integration/003-c001-closed.md`
- `plans/closure/codegg-integration/003-c002-closed.md`
- `plans/closure/evidence-closure/002-c004-closed.md`
- `plans/implementation/codegg-integration/003-c002-boundary-guard-completeness.md`
- `architecture/deep-dive-tooling-governance.md`
- `.skills/eggplan-milestone/SKILL.md`

Out of scope:

- the **content** of any closed closure record — no conclusion, no finding,
  and no evidence table is rewritten;
- any production Rust source;
- `plans/implementation/` plan files other than the one named above, which
  only carries a path reference;
- the status of any milestone in the registry — no status changes here.

## 5. Required production changes

1. Rename the file `003-conditionally-closed.md` in
   `plans/closure/codegg-integration/` to
   `plans/closure/codegg-integration/003-closed.md`.
2. Append to the renamed record a dated erratum recording the former path,
   the rename's date, and that the status line was amended in place by
   `6644725` while the filename was left behind.
3. Update every reference to the old path:
   - `plans/registry.md:83` — the Handoff cell;
   - `plans/closure/codegg-integration/003-c001-closed.md:15` and `:27`;
   - `plans/closure/codegg-integration/003-c002-closed.md:274`;
   - `plans/closure/evidence-closure/002-c004-closed.md:200`;
   - `plans/implementation/codegg-integration/003-c002-boundary-guard-completeness.md:14`;
   - `architecture/deep-dive-tooling-governance.md:332`.
4. Correct `architecture/deep-dive-tooling-governance.md`'s stale description
   of the CodeGG M003 combination, and its registry line anchors.
5. Add the staging rule to `.skills/eggplan-milestone/SKILL.md`'s "##
   Hygiene" section, and one mirrored bullet to `plans/registry.md`'s
   "Planning hygiene" section.
6. Change no closure record's status, finding, severity, or conclusion.

## 6. Failure, cancellation, restart, and contention semantics

This pass touches planning documents only. A rename is a Git operation; the
erratum must be appended in the same commit so the record is never
momentarily inconsistent.

## 7. Required tests

There is no production code to test. The checks are:

- every `plans/` path referenced by `plans/registry.md` exists;
- every closure record referenced by a registry row exists, and no closure
  record is orphaned;
- exactly one closure record per registry row resolves;
- the renamed record's status line and its filename agree;
- the two unresolved-finding rows that mention the old filename remain
  textually intact.

## 8. Verification

    git diff --check
    git status --short
    git log --follow --oneline -- plans/closure/codegg-integration/003-closed.md
    python3 - <<'PY'   # the registry path-consistency check, in this plan's §11
    ...               # every plans/... path in plans/registry.md must exist
    PY

The workspace build and test suite are not required, because no production
source changes. If any production file changes, the full suite must be run.

## 9. Acceptance criteria

1. `plans/closure/codegg-integration/003-closed.md` exists and
   `003-conditionally-closed.md` does not.
2. The renamed record carries a dated erratum naming its former path.
3. No reference to the old path remains anywhere in the repository.
4. `git log --follow` reaches the original creation commit through the
   rename.
5. Every closure record's filename agrees with its recorded status.
6. No closed record's status, finding, severity, conclusion, or evidence was
   changed.
7. `architecture/deep-dive-tooling-governance.md` describes the CodeGG M003
   combination accurately and its registry line anchors resolve.
8. The staging rule is present in both the procedure file and the registry's
   Planning hygiene section.
9. Every `plans/` path in `plans/registry.md` resolves, and no closure record
   is orphaned.

## 10. Stop conditions

- The rename would require changing a closed record's status or conclusion.
- The rename would orphan a closure record or break a reference the rename
  cannot reach.
- The staging rule cannot be worded without colliding with the filesystem
  staging sense the ADRs already use.

## 11. Closure evidence required

- The registry path-consistency check's output.
- The enumeration of all closure records with their filename and recorded
  status, showing that exactly one disagreement existed and that none
  remains.
- The `git log --follow` output for the renamed record.
- The diff showing that no closure record's status, finding, or conclusion
  changed.
- An explicit statement that no production Rust source, schema version,
  digest, or golden byte changed.

## 12. Handoff notes

- This plan is **ready**. The defect is reproduced, the history that produced
  it is identified, and the eight references to update are enumerated.
- The findings being closed are recorded in
  `plans/closure/codegg-integration/003-c002-closed.md:274` and
  `plans/closure/evidence-closure/002-c004-closed.md:200` (the registry-row
  finding) and `plans/closure/eggstack-integration/003b-closed.md:236`
  (the staging rule). Their conclusions are preserved; this plan makes the
  planning record consistent with them.
- The CodeGG integration roadmap is terminal. This plan does not reopen it;
  it corrects the naming of one of its records.
