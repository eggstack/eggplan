# Evidence and Closure M002 C002 — Finalization Test-Seam Containment and Closure Evidence Reconciliation

Status: closed

Repository baseline: 5d2b6c8bac3fd056946e94aaa541a3c1e216abba

Source roadmap:

- plans/subsystems/evidence-closure-roadmap.md

Predecessor implementation and closures:

- plans/implementation/evidence-closure/002-closure-records-integrity-and-recovery.md
- plans/closure/evidence-closure/002-closed.md
- plans/implementation/evidence-closure/002-c001-finalization-subject-revalidation.md
- plans/closure/evidence-closure/002-c001-closed.md

Current C001 implementation commit:

- b872f9dad50825eb255f4f70c43e02e6a958a8a8 — repository-owned double subject recapture and typed closure-subject failures.

Current C001 closure commit:

- 5d2b6c8bac3fd056946e94aaa541a3c1e216abba — historical C001 closure and downstream-unblock bookkeeping.

Long-term requirements:

- plans/000-long-term-specification.md sections 4.3, 4.6-4.8, 11-13, 19-20
- plans/001-terminology-and-domain-model.md sections 5-7
- plans/002-long-term-roadmap.md Evidence M002 corrective lineage

Applicable ADRs:

- ADR-0001-planning-evidence-mechanism-not-execution
- ADR-0002-repository-first-versioned-canonical-state
- ADR-0003-immutable-revision-scoped-evidence

Primary class: invariant / corrective hardening / closure evidence hygiene

## 1. Objective

Contain the deterministic subject-capture test seam introduced by M002 C001 so
it cannot be called by downstream crates, and reconcile the historical C001
closure record with the hosted workflow evidence that became available after
the closure document was committed.

C001 correctly changed the ordinary production entry point so
`RepositoryStore::finalize_closure` owns authoritative Git subject recapture.
However, its deterministic test seam was implemented as hidden-but-public API:

- `SubjectCapture` is `pub`;
- `GitSubjectCapture` is `pub`;
- `ScriptedSubjectCapture` is `pub`;
- those names are re-exported from the crate root;
- `RepositoryStore::finalize_closure_with_capture` is `pub`.

`#[doc(hidden)]` affects generated documentation only. It does not restrict
Rust visibility or downstream use. A downstream crate can therefore implement
or instantiate a subject capture source that returns the candidate subject and
invoke `finalize_closure_with_capture`, bypassing repository-owned
`GitSubjectSource` authority.

The corrective must make the injected capture path impossible to invoke from a
downstream crate while preserving deterministic race tests internally.

This pass also fixes a factual closure-evidence defect: the accepted C001
closure file still contains `<to-be-filled-by-CI>` placeholders even though
the required hosted run subsequently completed successfully.

## 2. Findings

### E-M002-C002-01 — hidden documentation is not private authority

At baseline, `crates/eggplan-repo/src/lib.rs` contains hidden public
re-exports of the subject-capture seam, and `RepositoryStore` exposes a
hidden public alternate finalizer accepting `&dyn SubjectCapture`.

The C001 invariant is stronger than "the CLI uses the safe entry point."
External callers must not be able to choose the authoritative subject source
for closure finalization at all.

### E-M002-C002-02 — C001 closure evidence contains placeholders

`plans/closure/evidence-closure/002-c001-closed.md` currently claims hosted
pass results while retaining placeholder run/job identifiers.

The actual closure-head workflow now exists and is successful:

- workflow run: 35998715018
- Linux job: 107629794960 — success
- macOS job: 107629795068 — success
- Windows job: 107629794971 — success
- Rust 1.89 job: 107629794849 — success

This is a factual evidence-reconciliation issue. Preserve the historical C001
closure semantics and status; replace only the placeholder/incorrect evidence
text with the actual immutable workflow identifiers and accurate runner notes.

### E-M002-C002-03 — no public-API regression prevents authority re-exposure

C001 has runtime regressions for stale/drift/capture-failure behavior, but no
regression proves that an external crate cannot access the injected-capture
finalization path.

The corrective must add a boundary test that would fail if the subject-capture
seam or alternate finalizer becomes externally callable again.

## 3. Controlling invariant

After C002:

> For repository-backed guarded closure, the only supported externally callable
> finalization path is `RepositoryStore::finalize_closure`, and that path
> obtains both authoritative SubjectRevision captures from the
> RepositoryStore-owned GitSubjectSource.

Downstream callers may:

- build a ClosureCandidate containing the subject against which they assessed;
- call `finalize_closure`;
- receive typed subject stale/drift/capture failures.

Downstream callers may NOT:

- implement a closure SubjectCapture authority;
- provide a SubjectCapture instance to a finalizer;
- call an alternate finalizer that accepts injected subject state;
- select S1 or S2 directly.

The deterministic injection seam may exist only inside the crate's
implementation/test visibility boundary.

## 4. Required production API containment

### Crate root

Remove all public re-exports of:

- `SubjectCapture`;
- `GitSubjectCapture`;
- `ScriptedSubjectCapture`.

Do not replace them with differently named hidden public exports.

### Subject capture abstraction

Preferred shape:

- `SubjectCapture`: private or `pub(crate)`;
- `GitSubjectCapture`: private or `pub(crate)` if the wrapper remains useful;
- scripted capture test double: `#[cfg(test)]` and private to the module/test
  hierarchy whenever practical.

A test helper does not become public API merely because integration tests are
convenient.

### Alternate finalizer

`finalize_closure_with_capture` must not remain `pub`.

Preferred choices, in order:

1. make the helper private or `pub(crate)` and move deterministic injection
   tests into `eggplan-repo` unit/module tests;
2. make only the internal `finalize_closure_inner` generic/injectable under a
   crate-private abstraction;
3. if external-style integration coverage is important, expose a dedicated
   test-only module only under `#[cfg(test)]` inside the crate rather than a
   library public symbol.

Do not use a Cargo feature that downstream production builds can enable to
restore caller-controlled subject authority.

## 5. Test relocation strategy

Current deterministic drift tests live where they can use the public seam.
Move or split them so privacy can be real.

Recommended layout:

- retain repository integration tests that exercise only public production API:
  - stale candidate caused by a real worktree mutation before finalization;
  - stable normal guarded closure;
  - reopen/recovery;
  - historical closure becomes stale after a later worktree mutation.
- move deterministic scripted S1/S2 tests into `store.rs` or another
  crate-internal `#[cfg(test)]` module:
  - A/A succeeds;
  - A/B returns `ClosureSubjectDrift`;
  - capture error returns `ClosureSubjectCapture`;
  - each failure proves no pending/final closure and no Closed Plan revision.

The test relocation must not reduce semantic coverage.

## 6. Public-API regression

Add a regression that proves the authority seam is not reachable from an
external consumer.

At minimum cover both:

- `eggplan_repo::SubjectCapture` / scripted capture names are not publicly
  importable;
- `RepositoryStore::finalize_closure_with_capture` is not publicly callable.

Preferred dependency-free mechanism:

- crate-level or module documentation `compile_fail` doctests that reference
  the forbidden public names/method item; `cargo test --doc` / workspace tests
  must execute them.

For example, a compile-fail test may attempt to reference:

    use eggplan_repo::SubjectCapture;

and another may attempt:

    let _ = eggplan_repo::RepositoryStore::finalize_closure_with_capture;

These examples must fail specifically because the API is unavailable/private,
not because of an unrelated syntax/type error.

If doctests prove unreliable across the supported toolchain, use a small
external-consumer compile fixture invoked by a portable script/test. Do not add
a large public-API tooling dependency solely for this check.

Also extend an existing boundary script or add
`scripts/check-closure-authority-boundary.sh` to fail if forbidden seam names
are publicly re-exported or if an injected finalizer is declared `pub`.
The script is defense in depth; the compile boundary is the stronger evidence.

## 7. Production behavior that must remain unchanged

C002 is containment, not a redesign. Preserve C001's behavior:

1. `finalize_closure` acquires the Eggplan repository lock.
2. S1 is captured from the store-owned Git subject source.
3. S1 must equal `candidate.subject`.
4. assessment is replayed against S1 and must be exactly Complete/equal.
5. evidence digests, provider policy, and supersession lineage are revalidated.
6. S2 is captured immediately before the first closure write.
7. S2 must equal S1 and `candidate.subject`.
8. only then may `closure.pending.json` be written.
9. existing pending -> closed Plan -> final closure crash recovery remains
   authoritative.

Do not move Git/filesystem dependencies into eggplan-core.

## 8. C001 closure evidence reconciliation

Treat edits to
`plans/closure/evidence-closure/002-c001-closed.md` as a factual erratum only.

Replace the hosted-workflow placeholder block with:

- run 35998715018;
- Linux job 107629794960;
- macOS job 107629795068;
- Windows job 107629794971;
- Rust 1.89 job 107629794849.

Accurately state:

- Linux: fmt/check/clippy/tests plus all shell boundary guards passed;
- macOS: check/clippy/tests plus shell boundary guards passed; fmt is skipped
  by workflow policy;
- Windows: check/clippy/tests passed; fmt and shell boundary guards are skipped
  by workflow policy;
- Rust 1.89: workspace check/tests passed.

Remove the statement that IDs would later be recorded in a final commit
message; that did not occur.

Do not rewrite the C001 implementation finding, acceptance conclusion, or
historical timestamps merely to make the closure look newly authored.

## 9. C002 closure evidence discipline

Do not repeat the C001 mistake.

The C002 closure record may be drafted while CI is running, but it MUST NOT be
marked `closed` with placeholder workflow IDs or with planned jobs described
as passed.

Acceptable sequencing:

1. implementation commit lands;
2. hosted CI runs;
3. obtain actual run/job outcomes;
4. write/finalize C002 closure using those identifiers;
5. update registry/roadmaps to closed/unblocked.

If hosted qualification is not yet available, status remains `closing` or
`conditionally closed` only when the repository planning process actually
permits the named missing evidence. Do not invent passing evidence.

## 10. API compatibility

Expected public compatibility change:

- remove accidental, undocumented `#[doc(hidden)]` public symbols introduced
  by C001.

This is intentional hardening, not a supported-API regression. Eggplan has not
declared these hidden test seams as stable contract, and preserving them would
preserve the authority bypass.

The supported public C001 signature remains:

    RepositoryStore::finalize_closure(
        &self,
        candidate: &ClosureCandidate,
        closure_id: ClosureId,
        finalized_at_unix_ms: u64,
    ) -> Result<(Plan, ClosureRecord), RepoError>

No persisted schema changes are expected:

- Plan v1/v2 unchanged;
- EvidenceObservation v1/v2 unchanged;
- ClosureCandidate unchanged;
- ClosureRecord unchanged;
- repository storage_version/layout unchanged.

## 11. Scope

### In scope

- make the injected SubjectCapture seam truly internal/test-only;
- remove hidden public re-exports;
- make alternate capture-injected finalization non-public;
- relocate deterministic scripted tests as necessary;
- add compile/public-API boundary regression;
- add/extend static closure-authority boundary guard;
- preserve all existing C001 runtime regressions;
- factually repair C001 hosted CI evidence placeholders;
- produce a C002 closure only after actual hosted qualification;
- roadmap/registry corrective lineage and downstream gating/unblocking.

### Explicitly out of scope

- redesigning the double-capture algorithm;
- locking arbitrary external worktree writers;
- changing ClosureRecord schema;
- ancestry-aware subject policy;
- Markdown import/render;
- staged CodeGG adoption;
- live Eggwork/Eggsearch adapters;
- signatures/attestation;
- a general plugin/subject-source system.

## 12. Security and authority review

Treat closure subject capture as a trust boundary.

The public API must make it impossible for an untrusted/downstream caller to
substitute the authority source used at commit time.

`#[doc(hidden)]`, naming conventions, comments, and "for tests only" prose are
not security boundaries.

The only caller-selected subject remains the subject embedded in the
ClosureCandidate proposal; the repository independently determines whether
that proposal is current.

## 13. Ordered work packages

### WP1 — Contain the capture seam

Remove crate-root re-exports. Make SubjectCapture/helper/test-double visibility
private or crate-private/test-only. Make `finalize_closure_with_capture`
non-public.

### WP2 — Preserve deterministic race coverage

Move scripted capture tests into a crate-internal test module and prove A/A,
A/B, capture-failure, and no-partial-state behavior.

### WP3 — Add public authority-boundary regression

Add compile-fail external-consumer coverage and a lightweight static boundary
guard so accidental re-publication fails CI.

### WP4 — Reconcile C001 closure evidence

Update only factual hosted-CI placeholders/claims in the historical C001
closure using run 35998715018 and its four actual job IDs.

### WP5 — Qualify C002 and close honestly

Run the full workspace/MSRV/native matrix, obtain actual hosted IDs, then write
`plans/closure/evidence-closure/002-c002-closed.md` and update registry and
roadmaps.

## 14. Required tests

At minimum:

- public `RepositoryStore::finalize_closure` stable-subject happy path;
- real worktree change before public finalization -> ClosureSubjectStale;
- internal scripted A/A -> success;
- internal scripted A/B -> ClosureSubjectDrift;
- internal scripted capture failure -> ClosureSubjectCapture;
- every pre-write subject failure leaves:
  - source Plan revision unchanged;
  - Plan non-Closed;
  - no closure.pending.json;
  - no closure.json;
- historical closure remains structurally valid after later source change;
- compile-fail: external consumer cannot import SubjectCapture/test double;
- compile-fail: external consumer cannot reference
  `RepositoryStore::finalize_closure_with_capture`;
- static boundary guard detects forbidden public re-export/method visibility;
- existing raw CAS-to-Closed rejection;
- existing evidence/provider/supersession drift rejection;
- existing pending recovery/corruption matrix;
- CLI subject diagnostic regressions remain green;
- CodeGG parity cancel-vs-close regression remains green.

## 15. Required verification

Record actual results for:

    cargo fmt --all -- --check
    cargo check --workspace --all-targets --locked
    cargo clippy --workspace --all-targets --locked -- -D warnings
    cargo test --workspace --locked
    cargo test --workspace --doc --locked
    cargo +1.89.0 check --workspace --all-targets --locked
    cargo +1.89.0 test --workspace --locked
    bash scripts/check-core-boundary.sh
    bash scripts/check-codegg-compat-boundary.sh
    bash scripts/check-integrations-boundary.sh
    bash scripts/check-projection-cli-boundary.sh
    bash scripts/check-closure-authority-boundary.sh
    git diff --check

If `cargo test --workspace --locked` already runs all relevant doctests for
the workspace configuration, record that fact, but still ensure the
compile-fail authority regression actually executed.

Hosted qualification must include:

- native Linux;
- native macOS;
- native Windows;
- Rust 1.89.

Update CI to invoke the new boundary guard on platforms where shell guards are
supported.

## 16. Documentation updates

Update as needed:

- architecture/repository.md — explicitly state the injected test seam is not
  part of the public API;
- architecture/evidence.md — repository-owned subject authority has no public
  alternate source-injection path;
- architecture/cli-control-surface.md only if diagnostic/API wording changes;
- README only if public API examples currently expose removed test helpers;
- C001 closure factual CI block;
- evidence subsystem roadmap;
- master roadmap;
- registry.

Do not document private test helpers as supported integration points.

## 17. Acceptance criteria

C002 closes only when:

1. no subject-capture abstraction/test double used by closure finalization is
   publicly importable from eggplan-repo;
2. no public RepositoryStore method accepts an injected SubjectCapture or
   equivalent caller-controlled finalization subject source;
3. the supported `finalize_closure` path still performs C001 S1/S2 recapture
   and all semantic regressions pass;
4. a compile/public-API regression would fail if the alternate authority path
   is exposed again;
5. a static boundary guard provides defense-in-depth visibility checking;
6. the historical C001 closure contains the actual successful hosted run/job
   IDs and no placeholders;
7. C002's own closure contains actual, completed native/MSRV workflow evidence
   with no placeholders;
8. persisted schemas remain unchanged;
9. native Linux/macOS/Windows and Rust 1.89 qualification passes.

## 18. Stop conditions

Stop and report if:

- deterministic testing cannot be retained without a production-public
  authority injection seam;
- making the helper private would require moving Git/filesystem authority into
  eggplan-core;
- an external consumer legitimately depends on the hidden C001 test seam and
  removing it would imply an intentional supported extension API decision;
- the fix requires a persisted schema version change;
- the C001 historical CI evidence cannot be matched to the closure-head commit;
- any native qualification lane fails for a substantive C002 regression.

## 19. Closure evidence required

The C002 closure record must include:

- exact public symbols removed/contained;
- before/after supported public finalization API inventory;
- compile-fail authority-boundary evidence;
- static boundary-guard evidence;
- deterministic internal A/A, A/B, capture-failure results;
- no-partial-state assertions;
- confirmation C001 closure placeholders were replaced with:
  - run 35998715018;
  - Linux 107629794960;
  - macOS 107629795068;
  - Windows 107629794971;
  - MSRV 107629794849;
- exact C002 hosted run/job IDs and outcomes;
- schema compatibility statement;
- downstream roadmap disposition.

## 20. Roadmap disposition after closure

Only after C002 is closed and its hosted evidence is recorded may these return
to ready-for-planning:

- Projection/CLI M002 — Markdown import/render;
- CodeGG Integration M002 — staged core adoption after a fresh CodeGG baseline
  check;
- Eggstack Integrations M002 — real Eggwork/Eggsearch adapters after fresh
  sibling baseline checks.

Interop/distribution remains deferred.

## 21. Handoff notes

Keep this pass narrow.

Do not start the next capability wave in the same implementation change. The
point of C002 is to make the closure authority boundary mechanically true in
the public API and to restore the repository's evidence discipline before new
consumers bind to it.

## 22. Closure record

Status: closed.

Implementation commit: `0c0484afe5be83ed92e6e4a4fdbcf6af6dbf6f3c`.

Closure record: plans/closure/evidence-closure/002-c002-closed.md.

Hosted C002 implementation run:

- Run: https://github.com/eggstack/eggplan/actions/runs/36004813178
- Linux 107650129619
- macOS 107650129837
- Windows 107650129154
- Rust 1.89 107650129645

C001 implementation run placeholders reconciled in
plans/closure/evidence-closure/002-c001-closed.md:

- Run: https://github.com/eggstack/eggplan/actions/runs/35998715018
- Linux 107629794960
- macOS 107629795068
- Windows 107629794971
- Rust 1.89 107629794849

Projection/CLI M002, CodeGG M002, and Eggstack M002 are unblocked; each
still requires a registered bounded implementation plan and a fresh
sibling-interface recheck before handoff. Interop/distribution remains
deferred.
