# Foundation Core M003 — Subject Scope, Strict Schema, and Platform Hardening

Status: ready for handoff

Repository baseline: c1fa03b697d68a53eb8d11648e67946f66fe3875

Source roadmap:

- plans/subsystems/foundation-core-roadmap.md

Predecessor implementation and closure records:

- plans/implementation/foundation-core/001-repository-bootstrap-and-domain-contract.md
- plans/closure/foundation-core/001-closed.md
- plans/implementation/foundation-core/002-repository-store-cas-and-subject.md
- plans/closure/foundation-core/002-conditionally-closed.md
- plans/closure/evidence-closure/001-closed.md

Long-term requirements:

- plans/000-long-term-specification.md sections 4.3, 4.6-4.8, 11, 13, 18-20
- plans/001-terminology-and-domain-model.md
- plans/002-long-term-roadmap.md Foundation M003

Applicable ADRs:

- ADR-0002-repository-first-versioned-canonical-state
- ADR-0003-immutable-revision-scoped-evidence

Primary class: invariant / corrective hardening / qualification

## 1. Objective

Correct three foundation-level gaps discovered after M001/M002/Evidence M001
closure:

1. Eggplan-managed repository state can currently participate in Git dirty
   subject capture and therefore can change the exact SubjectRevision merely by
   recording Eggplan state or evidence.
2. Plan schema-v1 deserialization does not consistently reject unknown nested
   fields even though the long-term contract requires unknown
   interpretation-changing state to fail closed.
3. Foundation M002 remains only Linux-qualified; Windows and macOS runtime
   path, lock, replacement, Git-subject, and durability behavior have not been
   exercised on native hosted runners.

This pass also repairs directly related documentation drift and establishes
hosted CI so later closure records can cite cross-platform execution rather
than local-only claims.

The historical M001/M002 closure records remain unchanged.

## 2. Why this milestone is ready

Foundation M001 and M002 implementations exist and Evidence M001 exposed the
integration consequences of their current contracts. No unresolved external
Eggstack interface is required.

The defects are local and reproducible from current source:

- RepositoryStore.subject_source constructs a GitSubjectSource from the
  Eggplan state root.
- GitSubjectSource hashes Git status entries without an administrative-state
  exclusion.
- Plan and nested schema types deserialize without a strict unknown-field
  contract.
- There is no GitHub Actions workflow on current main.
- the root README contains stale bootstrap copy followed by the newer project
  description.

## 3. Findings this plan must close

### F-M003-01 — Eggplan state changes source subject identity

The subject used to decide evidence applicability must describe the worktree
being verified, not Eggplan's own control-plane files.

Writing .eggplan/config.toml, plan.json, evidence observations, staging files,
or closure files MUST NOT alter the dirty-state fingerprint for the same Git
HEAD and unchanged non-Eggplan source tree.

This exclusion is an Eggplan subject-scope rule, not a reliance on .gitignore.
It must work whether the state directory is ignored, untracked, or tracked in
the worktree.

For schema-v1 SubjectRevision, revision remains the Git HEAD OID and the dirty
digest covers applicable non-Eggplan worktree changes. A later metadata-only
commit changes HEAD and therefore changes schema-v1 subject identity; do not
silently broaden that policy in this corrective.

### F-M003-02 — plan schema-v1 accepts unknown nested fields

All schema-v1 persisted planning structures MUST reject unknown fields at every
semantic layer that participates in canonical persisted state, including Plan,
PlanItem, AcceptanceCriterion, EvidenceRequirement, SubjectRevision, and
ArtifactRef.

Repository plan envelopes already reject unknown envelope fields. Nested Plan
objects must receive the same fail-closed treatment.

Strict parsing MUST preserve the existing canonical schema-v1 bytes and digest
fixtures for valid objects.

### F-M003-03 — cross-platform qualification missing

Native GitHub-hosted Linux, macOS, and Windows jobs must compile and exercise
the supported repository contract. Environmental failures are evidence of an
unqualified platform, not passes.

Any platform-specific corrections required for safe atomic replacement,
locking, path handling, Git fixtures, or durability reporting belong in this
pass.

### F-M003-04 — documentation/status drift

The root README must contain one current project description, not the original
bootstrap README concatenated with the implemented-state README.

Architecture and planning docs must describe the final subject-scope exclusion
and cross-platform guarantees without upgrading historical closure claims.

## 4. Invariants that must not regress

- No scheduler, executor, HTTP, MCP, database, or model runtime enters
  eggplan-core.
- No repository-controlled hook or arbitrary command is executed by production
  Git subject capture.
- Non-Eggplan dirty source changes remain represented in the subject digest.
- Untracked, staged, deleted, symlink, and submodule source changes remain
  distinguishable under the documented bounds.
- Exact-subject evidence remains fail-closed.
- Schema-v1 valid canonical bytes/digests do not change.
- Unknown schema versions still fail explicitly.
- CAS, locking, atomic staging, corruption detection, and symlink/path
  confinement remain intact.
- Platform guarantees are stated no stronger than the primitives actually
  verified.

## 5. Scope

### In scope

- explicit Git subject exclusion for the active Eggplan managed state root;
- normalization/canonicalization of that exclusion relative to the discovered
  Git worktree;
- tracked, untracked, ignored, nested-state-root, and state-outside-worktree
  behavior;
- end-to-end regression proving Eggplan writes do not change the subject;
- strict schema-v1 unknown-field rejection throughout Plan persistence;
- parser fixtures for top-level and nested unknown fields;
- migration-reader scaffolding only where necessary to avoid painting future
  schema versions into a corner;
- native GitHub Actions Linux/macOS/Windows checks;
- stable and Rust 1.89 qualification at appropriate matrix points;
- repository/runtime tests on each supported OS;
- corrections required by native platform failures;
- README and directly affected architecture docs.

### Explicitly out of scope

- evidence verification-spec binding, which is Evidence M001 C001;
- ClosureRecord persistence, which remains Evidence M002;
- ancestry-aware or metadata-only-commit subject equivalence;
- a general subject path-filter policy language;
- command execution or provider integrations;
- CodeGG, CLI, or Eggstack provider implementation.

## 6. Required production changes

### A. Subject-scope model

Add an explicit administrative exclusion to GitSubjectSource rather than
depending on ignore files.

RepositoryStore.subject_source should configure the store's own managed root as
excluded when that root is inside the discovered worktree.

The exclusion implementation must:

- compare normalized worktree-relative paths;
- exclude the state root and all descendants;
- reject ambiguous traversal rather than canonicalizing through unsafe
  symlinks;
- avoid excluding sibling paths with a shared string prefix;
- compose with submodule recursion without accidentally excluding source inside
  unrelated submodules;
- not leak excluded file contents into diagnostics.

If the state root is outside the discovered worktree, no worktree path
exclusion is needed.

### B. Strict schema-v1 decoding

Make v1 DTO decoding strict at every nested persisted type.

Preferred implementation is either:

- deny_unknown_fields on the actual stable v1 DTOs; or
- dedicated strict v1 wire DTOs converted into domain types.

Do not use a generic serde_json::Value cleanup step that silently discards
fields.

Add a parser path suitable for explicit future version dispatch, but do not
invent schema v2 in this plan.

### C. Repository envelope/reopen tests

Inject unknown fields at:

- StoredPlan envelope;
- Plan;
- PlanItem;
- AcceptanceCriterion;
- EvidenceRequirement;
- SubjectRevision;
- ArtifactRef where present.

Every injected unknown field must fail closed on direct parse and repository
reopen.

### D. Hosted CI and platform behavior

Add GitHub Actions workflow(s) that at minimum run:

- format/static guard on Linux;
- stable check, clippy, and tests on Linux, macOS, and Windows;
- Rust 1.89 check/test on at least the platforms needed to support the declared
  MSRV contract.

Prefer native hosted runners instead of cross-compiling from Linux.

Adapt test fixtures to be cross-platform without weakening production checks.
Test-only Git CLI use is acceptable when configured not to execute hooks and
available on hosted runners.

Qualify or correct:

- fs2 lock behavior;
- replacement of an existing plan file;
- staging cleanup;
- directory/file sync error reporting;
- Windows reparse/symlink handling where test privileges permit;
- path separator/case behavior;
- Git dirty-state capture and submodule fixture behavior.

If a platform primitive cannot provide the Unix durability guarantee, preserve
a narrower documented guarantee instead of simulating success.

### E. Documentation cleanup

Replace the duplicate root README with one current description.

Update architecture/repository.md and architecture/core.md only as required by
the corrected subject/parser/platform contracts.

Do not rewrite historical closure records except factual errata; M003 closure
records the current qualification.

## 7. Ordered work packages

### WP1 — Reproduce and freeze the self-staleness bug

Create an integration fixture in a Git repository with RepositoryStore rooted
at the normal .eggplan location.

Capture subject A, create plan/evidence state, recapture subject B with no
non-Eggplan source change, and first prove the pre-fix behavior would differ.

The committed regression must require A == B after the fix.

Also prove changing a normal source file still changes the dirty subject.

### WP2 — Implement administrative subject exclusion

Add the minimal explicit exclusion machinery and tests for exact path
boundaries, tracked/untracked state root, nested roots, source siblings, and
external state root.

### WP3 — Make schema-v1 parsing strict

Apply strict v1 wire decoding and add direct plus repository-reopen negative
fixtures. Preserve existing valid golden bytes/digests exactly.

### WP4 — Add native cross-platform CI and fix portability defects

Introduce hosted workflow matrix and iterate on actual macOS/Windows failures
without weakening path or durability semantics.

### WP5 — Documentation and qualification cleanup

Deduplicate README, document final platform guarantees, update roadmap/registry
on closure, and leave historical closure records intact.

## 8. Failure, restart, and contention semantics

Subject capture failure remains explicit; it must never fall back to a partial
or silently unscoped digest.

If the state-root exclusion cannot be represented safely relative to the
worktree, return an error rather than risk excluding an unintended source path.

Strict parsing errors are corruption/compatibility failures, not warnings.

Existing lock timeout, CAS, staging, and DurabilityUnknown behavior remain
controlling unless native qualification proves a defect that this plan then
corrects.

## 9. Compatibility and migration

Valid schema-v1 serialized bytes and digests are immutable compatibility
fixtures and must remain readable.

Unknown fields were never part of the accepted schema-v1 contract, so
rejecting them is a correctness fix, not a migration of valid state.

Do not write schema v2 in this milestone.

The subject-scope correction changes dirty-digest results when .eggplan state
was previously included. That prior digest is considered incorrectly scoped
and MUST NOT be preserved for compatibility. Closure must call out this
behavioral correction.

## 10. Required tests

At minimum:

- state-root write does not change captured subject;
- appending evidence does not change captured subject;
- modifying normal tracked source does change subject;
- staged/untracked/deleted normal source changes subject;
- state root ignored/untracked/tracked cases;
- exact exclusion path boundary;
- nested and external state-root cases;
- submodule behavior;
- all strict unknown-field injection cases;
- existing schema-v1 golden bytes/digest unchanged;
- CAS/contention/reopen/abandoned staging regression suite;
- native Windows replacement/lock/path tests;
- native macOS replacement/lock/path tests;
- Linux existing symlink/path tests;
- MSRV and boundary guard.

## 11. Required verification

Closure must record the exact commands and hosted workflow run IDs actually
executed. Expected local gates include:

    cargo fmt --all -- --check
    cargo check --workspace --all-targets --locked
    cargo clippy --workspace --all-targets --locked -- -D warnings
    cargo test --workspace --locked
    cargo +1.89.0 check --workspace --all-targets --locked
    bash scripts/check-core-boundary.sh
    git diff --check

Hosted CI must provide native Linux, macOS, and Windows evidence. A green Linux
job cannot substitute for an unrun Windows or macOS job.

## 12. Acceptance criteria

M003 closes when:

1. Eggplan state/evidence writes cannot alter the schema-v1 dirty subject for
   unchanged source at the same HEAD.
2. Normal source changes still alter the subject.
3. Unknown fields fail closed at every schema-v1 plan layer.
4. Valid schema-v1 golden bytes and digest remain unchanged.
5. Linux/macOS/Windows native CI runs the supported repository contract and all
   required jobs pass, or any remaining platform gap is explicitly
   conditionally closed with a narrow documented blocker.
6. README and architecture documentation reflect the implemented state.

## 13. Stop conditions

Stop and report if:

- excluding Eggplan state requires a general user-controlled path-filter DSL;
- safe state-root/worktree relationship cannot be determined without following
  unsafe symlinks;
- strict parsing would invalidate previously documented valid v1 fixtures;
- Windows/macOS require materially different persistence semantics needing a
  new ADR;
- a fix would weaken exact-subject semantics for ordinary source changes.

## 14. Closure evidence required

The closure record must include:

- the exact post-closure finding-to-fix matrix;
- self-staleness reproduction and regression evidence;
- subject-scope algorithm and limitations;
- strict parser fixture matrix;
- proof the v1 golden fixture/digest is unchanged;
- native CI workflow URLs/run IDs and per-platform outcomes;
- platform-specific persistence/durability statements;
- README/docs corrections;
- disposition of Foundation M002's inherited platform caveat;
- explicit statement whether Evidence M001 C001 may proceed.

## 15. Handoff notes

Prioritize correctness over preserving the old dirty digest. Do not solve the
self-reference bug by merely adding .eggplan to .gitignore: the runtime must
enforce its own administrative-state exclusion.
