# Evidence and Closure M002 — Guarded Closure Records, Supersession, and Recovery

Status: ready for handoff

Repository baseline: 6b924957fdc37e4266a410803186801db5655e82

Source roadmap:

- plans/subsystems/evidence-closure-roadmap.md

Predecessor implementation and closure:

- plans/implementation/evidence-closure/001-evidence-ledger-and-assessment.md
- plans/closure/evidence-closure/001-closed.md
- plans/implementation/evidence-closure/001-c001-verification-binding-and-end-to-end-evidence-corrective.md
- plans/closure/evidence-closure/001-c001-closed.md
- plans/closure/foundation-core/003-closed.md

Long-term requirements:

- plans/000-long-term-specification.md sections 4.1-4.8, 5, 8-13, 18-20
- plans/001-terminology-and-domain-model.md sections 4-7
- plans/002-long-term-roadmap.md Evidence M002

Applicable ADRs:

- ADR-0001-planning-evidence-mechanism-not-execution
- ADR-0002-repository-first-versioned-canonical-state
- ADR-0003-immutable-revision-scoped-evidence

Primary class: capability / invariant / recovery

## 1. Objective

Complete Eggplan's first end-to-end closure authority.

After this milestone, a repository Plan may reach `Closed` only through a
guarded closure operation that proves the exact Plan revision and current
SubjectRevision are complete under the trusted provider policy, records the
exact evidence used, and persists an immutable ClosureRecord. Raw plan CAS must
not be able to manufacture a closed Plan without that record.

This milestone also adds append-only evidence supersession lineage so a bad or
obsolete observation can be corrected without rewriting history, and defines
crash/reopen semantics for the two canonical objects involved in closure:
`plan.json` and `closure.json`.

## 2. Why this milestone is ready

Foundation M003 and Evidence M001 C001 are closed. The current repository now
has:

- deterministic Plan schema v1/v2 and canonical digests;
- revision/CAS repository persistence;
- exact, self-stable Git SubjectRevision capture;
- immutable evidence observations;
- strict evidence schema compatibility;
- verification-spec binding for execution-derived evidence;
- deterministic assessment;
- native Linux/macOS/Windows CI.

The remaining architectural gap is that ordinary RepositoryStore CAS can still
transition a Plan to `Closed` without materializing closure evidence.

## 3. Required domain additions

### Closure identity

Add a distinct typed ClosureId. Suggested prefix: `epcl_`.

Closure IDs, evidence observation IDs, plan IDs, and supersession IDs must
remain non-interchangeable.

### Evidence supersession record

Add an immutable, append-only EvidenceSupersessionRecord with at least:

- schema version;
- typed supersession ID, suggested prefix `eps_`;
- owning PlanId;
- superseded EvidenceObservationId;
- replacement EvidenceObservationId;
- bounded correction reason;
- recorded Unix-millisecond timestamp supplied by the host;
- canonical content digest.

Both observations must already exist in the same plan ledger. Self-links,
cycles, conflicting active successors, dangling replacements, and cross-plan
links fail closed.

The old observation remains readable forever. Supersession changes which
observation participates in current assessment; it does not rewrite either
observation.

### Effective evidence view

Introduce a deterministic lineage-aware evidence view used by closure
assessment. Preserve the existing no-lineage assessment API as a compatibility
wrapper if practical, but guarded closure must use the lineage-aware path.

Only terminal, non-superseded observations participate as current evidence.
The lineage and effective observation ordering must be deterministic.

### Closure candidate

A ClosureCandidate is a pure snapshot of the exact inputs proposed for closure.
It must contain enough information to detect drift before finalization:

- PlanId;
- source Plan revision and canonical digest;
- exact SubjectRevision;
- deterministic PlanAssessment;
- requirement/criterion matrix;
- exact satisfying observation IDs and content digests;
- relevant evidence-supersession IDs/digests;
- trusted-provider policy snapshot or canonical policy digest;
- candidate creation timestamp supplied explicitly by the host.

Candidate construction requires a complete assessment. It performs no
filesystem mutation.

### Closure record

A finalized ClosureRecord is immutable and versioned. It records at least:

- ClosureId;
- source Plan revision/digest assessed;
- final closed Plan revision/digest;
- SubjectRevision;
- provider-policy snapshot/digest used for assessment;
- exact criterion -> requirement -> satisfying observation matrix;
- observation IDs plus content digests;
- supersession lineage needed to interpret those observations;
- finalized timestamp;
- canonical ClosureRecord digest.

Do not embed large evidence payloads. Artifact content stays artifact-backed.

"Conditionally closed" remains a caller/policy concept. Core closure records
represent deterministic Eggplan closure only; unavailable requirements do not
become passing closure because a caller labels work substantially complete.

## 4. Guarded closure semantics

### Ordinary mutations

Ordinary PlanStore CAS MUST reject a transition to `PlanStatus::Closed`.

If necessary, split ordinary lifecycle transition validation from a dedicated
closure transition so callers cannot accidentally bypass the closure gate.

Cancellation remains an ordinary terminal transition.

### Finalize closure

Add a dedicated repository closure operation. Exact naming may vary, but its
semantics must be equivalent to:

1. acquire the repository lock;
2. reload the Plan and require the exact expected source revision/digest;
3. load and validate the current evidence ledger and supersession ledger;
4. require the caller's provider policy snapshot/digest;
5. require the candidate SubjectRevision to equal the current subject supplied
   for finalization;
6. recompute lineage-aware assessment and require `Complete`;
7. require every matrix observation ID/digest to match canonical ledger state;
8. create the target closed Plan at source revision + 1;
9. persist closure with the recovery protocol below;
10. return the exact finalized ClosureRecord and closed Plan.

A model/free-form caller cannot supply a predeclared "complete" result in lieu
of recomputation.

## 5. Crash-consistent repository protocol

A closure spans `plan.json` and `closure.json`, so a single rename cannot
make both visible atomically. Use an explicit recoverable protocol rather than
pretending otherwise.

Recommended layout:

    plans/<plan-id>/
      plan.json
      evidence/
      supersessions/
      closure.pending.json
      closure.json

Required protocol:

1. write and sync `closure.pending.json` containing the exact candidate and
   target closed-plan digest;
2. atomically replace `plan.json` with the validated closed revision;
3. atomically promote the pending record to `closure.json`;
4. sync the containing directory where supported.

Reopen/recovery rules:

- pending + source Plan still active at the source revision: the closure did
  not commit; discard/report the pending candidate without treating it as
  closure;
- pending + closed Plan at the exact target revision/digest: validate and
  promote the pending record;
- closed Plan with neither valid `closure.json` nor a recoverable pending
  record: corruption/recovery error;
- `closure.json` with a nonmatching Plan revision/digest: corruption;
- both final closure and inconsistent pending state: corruption;
- finalized `closure.json` is immutable.

Tests should expose crash points through injectable test-only hooks or a small
transaction primitive rather than relying on probabilistic process kills.

## 6. Provider-policy snapshot

Assessment depends on ProviderRegistry, therefore closure must record the trust
policy used.

Add a stable bounded snapshot representation containing only non-secret
authority facts required to reproduce assessment, such as:

- provider ID;
- provider class;
- allowed evidence kinds.

Sort deterministically before hashing/serialization. Do not persist credentials,
tokens, endpoints, or opaque adapter secrets.

The runtime ProviderRegistry remains host-constructed. A stored snapshot is
historical closure evidence, not permission to auto-trust that provider for a
new future assessment.

## 7. Integrity and reopen verification

Repository reopen/check paths must verify:

- ClosureRecord schema and digest;
- source/final Plan revision relationship;
- final closed Plan digest;
- every referenced observation exists and matches its recorded digest;
- every referenced supersession record exists and matches its digest;
- supersession lineage is acyclic and internally consistent;
- provider-policy snapshot/digest is valid;
- closure subject is valid;
- the closure matrix is deterministic and structurally consistent.

A later repository source change does not rewrite historical closure. The
ClosureRecord remains a statement about its recorded subject.

## 8. Scope

### In scope

- ClosureId and closure schema v1;
- EvidenceSupersessionRecord and append-only lineage persistence;
- lineage-aware assessment input;
- pure ClosureCandidate construction;
- guarded repository close;
- rejection of raw CAS-to-Closed;
- crash/reopen recovery;
- closure and supersession canonical digests/fixtures;
- provider-policy closure snapshot;
- deep integrity verification;
- native cross-platform regression coverage;
- architecture/evidence.md and repository docs for closure/recovery.

### Explicitly out of scope

- CLI/user command surface;
- provider acquisition/networking;
- CodeGG migration;
- Markdown closure projection/import;
- ancestry-aware subject reuse;
- cryptographic signatures/attestations;
- reopening a closed Plan as ordinary work;
- arbitrary closure policy language.

## 9. Ordered work packages

### WP1 — Closure and supersession domain

Implement typed IDs, strict versioned DTOs, bounds, canonicalization, digest
fixtures, provider-policy snapshot, and lineage validation.

### WP2 — Lineage-aware assessment

Add effective evidence resolution and compatibility wrappers. Prove
superseded observations remain historical but no longer influence current
assessment.

### WP3 — Guarded close API

Make raw CAS reject `Closed`, build/revalidate ClosureCandidate, and finalize
only after deterministic recomputation.

### WP4 — Persistence and crash recovery

Implement supersession ledger, pending/final closure protocol, reopen recovery,
corruption detection, and idempotent reads.

### WP5 — Qualification and documentation

Add integration/crash matrix tests, run native CI/MSRV, and update
architecture/core.md, architecture/evidence.md, architecture/repository.md,
roadmap, registry, and closure record.

## 10. Required tests

At minimum:

- complete active plan can produce a candidate;
- incomplete/failed/stale/in-flight/judgment-required plan cannot finalize;
- ordinary CAS cannot set `Closed`;
- successful guarded close writes exactly one immutable record;
- stale Plan revision/digest rejects close;
- changed subject rejects candidate finalization;
- changed/missing/corrupt observation rejects close/reopen;
- provider-policy drift rejects stale candidate;
- old and replacement observations remain readable;
- superseded old failed observation does not poison an `All` requirement when
  the valid replacement is the effective observation;
- supersession cycles, self-links, dangling refs, cross-plan refs, and
  conflicting successors fail;
- crash before plan replacement leaves Plan active and no finalized closure;
- crash after plan replacement recovers pending closure;
- closed Plan without closure/pending fails closed;
- closure/Plan digest mismatch fails;
- finalized closure cannot be overwritten;
- reopening produces identical closure and assessment matrix;
- Linux/macOS/Windows native tests and Rust 1.89 pass.

## 11. Required verification

Closure must record actual command results and hosted workflow IDs:

    cargo fmt --all -- --check
    cargo check --workspace --all-targets --locked
    cargo clippy --workspace --all-targets --locked -- -D warnings
    cargo test --workspace --locked
    cargo +1.89.0 check --workspace --all-targets --locked
    cargo +1.89.0 test --workspace --locked
    bash scripts/check-core-boundary.sh
    git diff --check

## 12. Acceptance criteria

M002 closes when:

1. no ordinary repository mutation can produce a canonical Closed Plan;
2. guarded closure recomputes a Complete assessment against exact current
   inputs;
3. a finalized immutable ClosureRecord identifies exact Plan, subject,
   provider policy, evidence, and supersession lineage;
4. crash points recover without accepting partial closure as finalized;
5. evidence correction is append-only and deterministic;
6. reopen detects corrupt/dangling closure evidence;
7. native supported-platform CI and MSRV pass.

## 13. Stop conditions

Stop and report if:

- reliable recovery would require cross-file atomicity the filesystem cannot
  provide and no bounded journal/pending protocol can express;
- closure would need to trust caller prose instead of recomputing assessment;
- correction requires rewriting a finalized observation;
- provider-policy snapshot would require persisting secrets;
- closure semantics require reopening terminal Plans or a general workflow DSL.

## 14. Closure evidence required

The closure record for this development milestone must include:

- raw-CAS-to-Closed negative evidence;
- guarded-close matrix;
- supersession lineage matrix;
- crash-point/recovery matrix;
- canonical fixture digests;
- corrupt/dangling reopen matrix;
- cross-platform workflow IDs;
- exact remaining downstream gates.

## 15. Handoff notes

This milestone is the final prerequisite before the first CLI, CodeGG parity,
and provider-SPI implementation wave. Do not merge those surfaces into this
plan.
