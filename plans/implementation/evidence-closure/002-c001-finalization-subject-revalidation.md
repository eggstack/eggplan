# Evidence and Closure M002 C001 — Finalization Subject Revalidation

Status: ready for handoff

Repository baseline: 7fd3034bd847fb32889b679e7a2092e38ac16356

Source roadmap:

- plans/subsystems/evidence-closure-roadmap.md

Predecessor implementation and closure:

- plans/implementation/evidence-closure/002-closure-records-integrity-and-recovery.md
- plans/closure/evidence-closure/002-closed.md
- plans/implementation/foundation-core/003-subject-scope-strict-schema-and-platform-hardening.md
- plans/closure/foundation-core/003-closed.md
- plans/implementation/projection-cli/001-cli-control-surface-and-derived-registry.md
- plans/closure/projection-cli/001-closed.md

Long-term requirements:

- plans/000-long-term-specification.md sections 4.3, 4.6-4.8, 11-13, 19-20
- plans/001-terminology-and-domain-model.md sections 5-7
- plans/002-long-term-roadmap.md Evidence M002 corrective lineage

Applicable ADRs:

- ADR-0002-repository-first-versioned-canonical-state
- ADR-0003-immutable-revision-scoped-evidence

Primary class: invariant / corrective hardening

## 1. Objective

Close the remaining avoidable subject time-of-check/time-of-use gap in guarded
closure finalization.

At the reviewed baseline, callers capture a Git SubjectRevision, build a
ClosureCandidate, and pass that same caller-owned snapshot into
`RepositoryStore::finalize_closure`. The finalizer reloads Plan/evidence and
recomputes assessment under the Eggplan repository lock, but it does not itself
recapture the repository subject. A source-tree change between caller capture
and finalization can therefore allow a canonical Closed Plan to be committed
for a subject that was already stale before the finalizer began.

Move authoritative subject recapture into the repository finalization boundary,
revalidate it again immediately before the first canonical closure write, and
make subject drift a typed failure with no partial closure state.

Preserve the historical M002 closure record. This corrective records the later
finding and the stronger current guarantee.

## 2. Current finding

### E-M002-C001-01 — caller-owned "current subject" is not authoritative

Current production shape:

    caller captures subject A
      -> caller assesses/builds candidate for A
      -> source may change to B
      -> finalize_closure(candidate, A, ...)
      -> finalizer compares candidate.subject == caller-supplied A
      -> assessment is recomputed against A
      -> Closed may be persisted

The Eggplan state lock protects managed .eggplan state. It does not establish
that an arbitrary SubjectRevision argument reflects the Git worktree at
finalization time.

This does not invalidate the integrity of existing historical ClosureRecords;
they still truthfully identify the subject they contain. It does mean guarded
closure currently accepts an avoidably stale caller snapshot as its
finalization precondition.

## 3. Controlling semantics

The corrected contract is:

1. ClosureCandidate remains a pure precomputed proposal for one exact Plan
   revision and SubjectRevision.
2. RepositoryStore finalization owns the authoritative Git subject recapture.
3. The finalizer MUST NOT accept a caller-supplied value labeled/currently
   treated as the authoritative current subject.
4. The captured finalization subject must exactly equal candidate.subject.
5. Assessment is recomputed using the finalizer-captured subject.
6. The subject is recaptured a second time immediately before writing
   `closure.pending.json`; any drift aborts with no canonical write.
7. The second successful capture is the closure finalization linearization
   point for source applicability.
8. Worktree changes after that point are subsequent repository changes. They do
   not rewrite historical closure; existing `check`/status behavior may report
   the closure subject as stale relative to the later worktree.

Eggplan does not claim to lock arbitrary external Git/worktree writers. The
goal is to remove the avoidable caller-snapshot gap and detect drift across the
assessment/revalidation phase, not to invent a global filesystem transaction.

## 4. Required API correction

### RepositoryStore::finalize_closure

Change the public repository API so finalization no longer accepts
`current_subject: &SubjectRevision` as authority.

Conceptually:

    finalize_closure(
        &self,
        candidate: &ClosureCandidate,
        closure_id: ClosureId,
        finalized_at_unix_ms: u64,
    ) -> Result<(Plan, ClosureRecord), RepoError>

The implementation MUST acquire the Eggplan repository lock first and then
capture the Git subject through the store's configured subject source.

Do not move Git capture into eggplan-core. The pure core ClosureCandidate and
ClosureRecord remain subject-source agnostic.

### Subject capture errors

Add a typed repository error for closure subject mismatch/drift rather than
collapsing it into `InvalidUpdate`.

The error must distinguish at least:

- candidate subject does not equal the first finalizer capture;
- subject changed between first and pre-write capture;
- subject capture itself failed/unavailable.

Exact enum shape is implementation-owned, but CLI/machine diagnostics must be
able to map these cases without parsing error prose.

Do not include source file contents in diagnostics.

## 5. Double-capture finalization protocol

Under the repository lock:

1. reload and validate source Plan revision/digest;
2. load observations and supersession lineage;
3. capture SubjectRevision S1 from the repository subject source;
4. require S1 == candidate.subject;
5. rebuild ProviderRegistry from the candidate's historical policy snapshot;
6. recompute effective observations and PlanAssessment using S1;
7. require exact equality with the candidate assessment and Complete status;
8. revalidate satisfying observation digests, provider-policy digest, and
   supersession lineage as M002 already requires;
9. capture SubjectRevision S2 immediately before creating/writing the pending
   closure;
10. require S2 == S1 and S2 == candidate.subject;
11. only then create the target Closed Plan/ClosureRecord and enter the
    existing pending -> plan -> final closure protocol.

If steps 3-10 fail, neither `closure.pending.json`, a Closed `plan.json`, nor
`closure.json` may be created or changed.

The existing crash-recovery protocol after the first pending write remains
controlling.

## 6. Deterministic test seam

The correction needs deterministic evidence for both stale-before-finalize and
drift-during-finalize behavior.

Prefer one of:

- a small internal/private subject-capture abstraction used by
  RepositoryStore and implemented by GitSubjectSource; or
- a test-only finalization helper/hook that can supply a deterministic sequence
  of captures.

Do not expose a broad public provider/plugin framework merely for the test.

Production `RepositoryStore::finalize_closure` must always use its own
repository subject source.

## 7. Required regressions

### Candidate becomes stale before finalization

1. initialize a temporary Git repository and Eggplan store;
2. create active Plan/evidence sufficient for Complete at subject A;
3. build ClosureCandidate for A;
4. modify an ordinary source file to produce subject B;
5. call guarded finalization;
6. require a typed subject mismatch;
7. assert Plan remains active at the source revision;
8. assert no pending/final closure file exists.

### Subject drifts during finalizer revalidation

Using the deterministic seam:

1. first finalizer capture returns A;
2. assessment and candidate validation succeed for A;
3. second capture returns B;
4. finalization fails with typed subject-drift error;
5. no pending/final closure or Closed Plan is written.

### Stable subject

Both captures return A and existing successful guarded-close/reopen/recovery
tests continue to pass unchanged in semantics.

## 8. CLI correction

Update `eggplan close` to stop supplying a caller-owned current subject to the
repository finalizer.

The CLI may still capture a subject before candidate construction for pure
assessment, but the repository finalizer independently recaptures/revalidates
before committing.

Map subject mismatch/drift to a stable machine error code such as
`closure_subject_changed` or a pair of more specific codes. Do not return a
generic success/failure string that forces clients to parse prose.

Existing explicit provider-policy requirements remain unchanged.

## 9. Reopen and historical closure semantics

Do NOT make ordinary reopen reject an otherwise valid historical closure merely
because the current worktree has moved beyond the closure subject.

A ClosureRecord is immutable evidence about its recorded subject.

Current-state surfaces may continue to report:

- closure valid for recorded subject; and
- closure subject stale relative to current worktree.

These are distinct from closure-record corruption.

No ClosureRecord schema change is required by this corrective unless
implementation discovers that the finalization linearization point cannot be
represented with the existing candidate subject. If a schema change appears
necessary, stop and report rather than silently version-bump.

## 10. Compatibility and migration

Expected compatibility impact:

- no persisted Plan schema change;
- no EvidenceObservation schema change;
- no ClosureRecord schema change;
- existing closure files remain valid;
- public Rust API for `RepositoryStore::finalize_closure` changes;
- CLI command syntax does not need to change;
- downstream callers must stop providing the current-subject argument.

Because Eggplan has not declared a stable public release contract yet, this API
hardening should be made directly rather than preserving an unsafe deprecated
overload that still accepts caller authority.

If a compatibility wrapper is retained temporarily, it MUST ignore/reject the
caller subject as authority and delegate to internal recapture.

## 11. Scope

### In scope

- repository-owned finalization subject capture;
- double-capture revalidation around assessment and pre-write;
- typed closure subject mismatch/drift errors;
- deterministic race regression seam;
- CLI close adaptation and JSON error mapping;
- M002 closure/recovery regression suite;
- architecture/repository.md, architecture/evidence.md, and
  architecture/cli-control-surface.md corrections;
- registry/roadmap lineage cleanup;
- native cross-platform qualification.

### Explicitly out of scope

- locking the entire Git worktree;
- fsmonitor/watch service;
- automatically reopening a historical Closed Plan when source later changes;
- ancestry-aware evidence reuse;
- signed attestations;
- new provider adapters;
- CodeGG staged adoption;
- Markdown import/render.

## 12. Invariants that must not regress

- ordinary Plan CAS cannot enter Closed;
- closure requires Complete recomputed assessment;
- exact subject matching remains the default;
- .eggplan administrative writes do not perturb the Git subject;
- evidence observations and supersession lineage remain immutable/append-only;
- provider policy remains explicit host authority;
- pending closure recovery remains crash-consistent;
- historical valid closure is not treated as corrupt solely because the
  worktree later changes;
- eggplan-core remains Git/filesystem independent.

## 13. Ordered work packages

### WP1 — Internal finalization subject authority

Remove caller subject authority from the public close API, add typed errors,
and wire finalization to RepositoryStore's GitSubjectSource under the lock.

### WP2 — Double-capture guard

Capture S1 before assessment replay and S2 immediately before pending write.
Abort cleanly on either mismatch/drift.

### WP3 — Regression seam and tests

Add deterministic stale-before-finalize and drift-during-finalize tests plus
all existing closure/recovery regressions.

### WP4 — CLI and projection diagnostics

Update `eggplan close` and JSON error mapping without changing command syntax
or provider-policy semantics.

### WP5 — Documentation and planning cleanup

Document the closure linearization semantics and external-writer limitation.
Update roadmap/registry status and remove stale prose that still says to execute
already-closed M001 work.

## 14. Failure, restart, and contention semantics

Subject mismatch/drift occurs before the first closure write and therefore
requires no recovery artifact.

Subject capture failure is a fail-closed closure error.

Once `closure.pending.json` is written, the existing M002 crash-recovery
protocol remains authoritative.

The Eggplan repository lock serializes Eggplan writers only. The closure
guarantee must not imply serialization of arbitrary editor/build/Git processes.

## 15. Required tests

At minimum:

- stale candidate subject before finalizer -> typed failure/no writes;
- first capture A, second capture B -> typed drift/no writes;
- stable A/A -> successful close;
- source change after an already finalized closure -> historical closure stays
  structurally valid while current-state check reports staleness;
- raw CAS-to-Closed still rejected;
- provider-policy/evidence/supersession drift tests still reject;
- pending source-side discard still works;
- pending target-side promotion still works;
- corrupt pending/final combinations still fail;
- CLI close maps subject drift to stable JSON error code;
- existing CLI close happy path passes;
- Linux/macOS/Windows native suite and Rust 1.89 pass.

## 16. Required verification

Closure must record actual results and workflow IDs for:

    cargo fmt --all -- --check
    cargo check --workspace --all-targets --locked
    cargo clippy --workspace --all-targets --locked -- -D warnings
    cargo test --workspace --locked
    cargo +1.89.0 check --workspace --all-targets --locked
    cargo +1.89.0 test --workspace --locked
    bash scripts/check-core-boundary.sh
    bash scripts/check-codegg-compat-boundary.sh
    bash scripts/check-integrations-boundary.sh
    bash scripts/check-projection-cli-boundary.sh
    git diff --check

Hosted qualification must include native Linux, macOS, Windows, and the Rust
1.89 lane.

## 17. Acceptance criteria

C001 closes when:

1. guarded closure no longer accepts a caller-provided current subject as
   finalization authority;
2. the repository finalizer captures and validates S1 under its lock;
3. assessment is replayed against S1;
4. S2 is recaptured immediately before the first canonical closure write and
   must equal S1/candidate.subject;
5. deterministic source-change regressions fail closure with no partial state;
6. existing crash recovery and closure integrity tests remain green;
7. CLI exposes stable machine diagnostics for subject drift;
8. persisted schemas remain compatible;
9. native CI/MSRV pass.

## 18. Stop conditions

Stop and report if:

- implementation would require eggplan-core to depend on Git/filesystem code;
- fixing the gap requires pretending Eggplan can globally lock arbitrary
  worktree writers;
- the existing ClosureRecord schema cannot represent the corrected semantics
  without a version change;
- a compatibility path would preserve the unsafe caller-authoritative API;
- the fix weakens exact-subject assessment.

## 19. Closure evidence required

The corrective closure record must include:

- predecessor M002 finding and exact corrected API;
- stale-before-finalize regression;
- drift-between-captures regression;
- proof no pending/final closure or Closed Plan is written on pre-write drift;
- stable-subject guarded close/recovery evidence;
- CLI JSON diagnostic fixture;
- persisted-schema compatibility statement;
- native CI/MSRV workflow IDs;
- current roadmap/registry disposition for Projection M002, CodeGG M002, and
  Eggstack M002.

## 20. Handoff notes

This is the only closure corrective currently required. Keep it narrow. Do not
fold Markdown import, live provider adapters, or CodeGG staged adoption into
the pass.
