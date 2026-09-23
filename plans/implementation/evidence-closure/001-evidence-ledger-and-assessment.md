# Evidence and Closure M001 — Evidence Ledger and Deterministic Assessment

Status: active

Repository baseline: 79dcf41 (`plans: unblock evidence M001`), re-established
after Foundation M002 conditional closure on 2026-09-23.

Source roadmap:

- plans/subsystems/evidence-closure-roadmap.md

Long-term requirements:

- plans/000-long-term-specification.md sections 8-12 and 18-20
- plans/001-terminology-and-domain-model.md
- plans/002-long-term-roadmap.md Evidence M001

Applicable ADRs: ADR-0001, ADR-0002, ADR-0003.

Primary class: capability / invariant

Hard dependency: Foundation M002 closure.

Platform qualification carried from Foundation M002: Linux storage/subject
behavior is verified. Windows/macOS runtime and cross-target evidence remain
outstanding; this milestone must preserve the documented durability limits and
must not claim those platforms qualified.

CodeGG interface re-check at handoff (2026-09-23): the available sibling
checkout is `28b46956` (`codegg-core` WorkPlan architecture/evidence/assessment
model). Its assessment consumes a bounded host-owned snapshot, treats missing
refs as unavailable, does not treat owner/run/job provenance alone as
satisfaction, and keeps explicit user judgment distinct from host evidence.
Its WorkPlan has host-specific owner and scheduler concepts that remain outside
Eggplan. These semantics inform the fixture matrix only; no CodeGG dependency or
runtime ownership is introduced.

## 1. Objective

Implement immutable EvidenceObservation domain/persistence plus deterministic,
subject-aware requirement/criterion/item/plan assessment. Establish the trust
boundary that prevents free-form claims, stale results, and merely planned
commands from closing work.

ClosureRecord persistence itself remains M002 unless a minimal candidate type
is needed for assessment tests.

## 2. Readiness gate

Foundation M002 must provide stable canonical serialization/digest,
SubjectRevision, repository object persistence, and CAS/reopen semantics.

Re-check current CodeGG work_plan evidence/assessment semantics at handoff for
golden cases, but do not introduce a CodeGG dependency.

## 3. Invariants

- Observation provider identity comes from trusted adapter construction, not
  arbitrary serialized caller text alone.
- Finalized observations are immutable.
- Every observation has schema version, subject, normalized status, provider,
  kind, timestamp, and digest.
- Missing evidence never satisfies.
- passed is not equivalent to completed-item prose.
- failed, not_run, skipped, unavailable, blocked, and inconclusive remain
  distinguishable.
- Exact SubjectRevision match is the default.
- Stale evidence remains readable but does not satisfy current criteria.
- Human judgment is explicit and policy-gated.
- Assessment is pure/deterministic and performs no network/process side effects.

## 4. Scope

### Core/domain

Add EvidenceObservationId, EvidenceStatus, EvidenceKind, provider descriptor or
trusted construction token boundary, immutable observation, applicability
result, CriterionAssessment, ItemAssessment, PlanAssessment, and bounded
explanation/reason codes.

### Repository

Persist observations append-only under their Plan. Finalize via staging plus
digest verification. Reject ID collision with different content. Same
observation replay may be idempotent only if byte/canonical digest identical.

### Matching

Implement EvidenceRequirement matching for the minimum schema-v1 policies:
kind, accepted provider identity/class if specified, exact subject, and
all/any/cardinality behavior already frozen in core.

Do not add an arbitrary expression language.

### Assessment

Define fixed precedence so the same state always produces the same result.
At least distinguish Complete, ActionableWorkRemaining, Blocked,
EvidenceFailed, EvidenceMissingOrUnavailable, InFlight,
AwaitingHumanJudgment, Inconclusive, and InvalidOrStale.

A PlanItem labeled Completed without satisfying required criteria must not make
the plan Complete.

## 5. Ordered work packages

### A — Observation domain and canonical digest

Golden fixtures for each status/kind and provider/subject fields.

### B — Append-only repository ledger

Atomic finalize/reopen/list/get, corruption/dangling detection, ID replay rules.

### C — Requirement matching and subject applicability

Exact-subject policy with explicit stale reason.

### D — Pure assessment

Criterion -> item -> plan assessment, deterministic precedence, bounded reasons.

### E — CodeGG semantic reference tests

Recreate representative cases: missing evidence unavailable, passed evidence
satisfies, failed/in-progress prevents closure, owner/provenance alone is not
success, human judgment only when explicitly allowed, completed-without-proof
does not close.

These are semantic fixtures, not a CodeGG dependency.

## 6. Failure/recovery/contention

Observation creation failure leaves no finalized observation.

Corrupt digest/schema yields explicit invalid/unavailable assessment rather
than pass.

Concurrent creation of the same observation ID with different content fails.
Different observation IDs may append independently subject to repository lock
rules.

Restart/reopen must produce the same assessment from the same canonical files.

## 7. Compatibility

Evidence schema v1 becomes a durable baseline. Do not auto-rewrite finalized v1
observations during reads.

Provider-specific native payloads must stay bounded and optional; core
compatibility cannot depend on arbitrary external JSON blobs.

## 8. Required tests

- every normalized status;
- provider mismatch/invalid attribution;
- exact subject match and clean/dirty staleness;
- missing/dangling evidence;
- corrupt digest;
- idempotent identical replay;
- conflicting replay;
- mixed all/any criteria;
- human judgment;
- completed label without proof;
- deterministic precedence;
- reopen/reassessment equivalence;
- bounded explanations;
- no hidden reasoning/secrets marker fields.

## 9. Verification commands

At minimum, adapted to the M002 workspace:

    cargo fmt --all -- --check
    cargo check --workspace --all-targets --locked
    cargo clippy --workspace --all-targets --locked -- -D warnings
    cargo test --workspace --locked
    cargo +1.89.0 check --workspace --all-targets --locked

Add focused race/reopen tests as separate commands when useful.

## 10. Stop conditions

Stop if provider trust cannot be represented without letting caller-controlled
serialized fields self-authorize; if subject identity from M002 is insufficient
for exact applicability; or if closure requires adding an executor/scheduler to
the core.

## 11. Acceptance criteria

A Plan with required evidence can be assessed from durable state alone, and no
unrun, unavailable, failed, stale, or free-form claimed evidence can be
misreported as satisfying the requirement.

## 12. Closure evidence required

Record schema v1, status mapping, provider authority boundary, subject matching,
append-only/recovery evidence, full assessment matrix, CodeGG semantic fixture
results, and whether Projection/CLI and CodeGG Integration M001 may proceed.
