# Evidence and Closure Roadmap

Status: closed / current

Long-term references: plans/000-long-term-specification.md sections 8-12 and 17-20.

Related ADRs: ADR-0001 and ADR-0003.

## 1. Purpose and ownership boundary

This subsystem owns evidence requirements, immutable normalized observations,
provider identity/trust metadata, subject applicability, deterministic
criterion/item/plan assessment, and closure records.

It does not execute commands, fetch remote evidence, schedule work, or decide
semantic retry.

## 2. Invariants

- Planned verification is never a passing observation.
- Host/provider identity cannot be self-asserted by caller prose.
- Evidence is immutable after finalization.
- Every observation has a SubjectRevision and digest.
- Exact-subject applicability is the default.
- Every failure or missing state remains distinguishable.
- Completion requires criteria, not merely item status labels.

## 3. Capabilities

- Record and reopen observations.
- Resolve evidence against requirements.
- Explain why evidence does or does not satisfy a criterion.
- Assess item/plan closure deterministically.
- Materialize immutable closure records.

## 4. Non-goals

No generic artifact store, signature authority, command runner, CI engine,
policy language, or remote evidence acquisition.

## 5. Target architecture

    EvidenceRequirement
          |
          v
    EvidenceObservation(s) <--- provider adapters
          |
      subject-policy
          |
          v
    CriterionAssessment
          |
    ItemAssessment
          |
    PlanAssessment
          |
          v
      ClosureRecord

Assessment functions are pure over explicit inputs.

## 6. Dependencies

Foundation M002 is a hard dependency because evidence validity requires stable
subject identity and repository persistence.

## 7. Milestones

### M001 — Evidence ledger and assessment

Status: closed.

Plan: plans/implementation/evidence-closure/001-evidence-ledger-and-assessment.md

Deliver normalized statuses, provider identities, append-only observations,
criterion matching, exact-subject policy, pure assessment, and explanations.
The Windows/macOS qualification caveat inherited from Foundation M002 was
resolved by Foundation M003's native supported-subset workflow; historical
M001 closure remains preserved.

### M001 C001 — Verification binding and end-to-end evidence corrective

Status: closed. See plans/closure/evidence-closure/001-c001-closed.md.

Plan: plans/implementation/evidence-closure/001-c001-verification-binding-and-end-to-end-evidence-corrective.md

Post-closure review found that kind/provider/subject matching is too broad for
specific execution verification: one Test observation can otherwise satisfy an
unrelated Test requirement. C001 adds a verification-spec digest binding,
explicit v1/v2 compatibility, fail-closed treatment for legacy unbound
execution requirements, the integrated persist/recapture/assess regression,
and the missing architecture/evidence.md document.

Historical M001 closure remains preserved.

### M002 — Closure records and integrity/recovery

Status: closed.

Plan: plans/implementation/evidence-closure/002-closure-records-integrity-and-recovery.md

Implemented guarded ClosureCandidate/ClosureRecord semantics, exact
requirement-to-evidence matrices, provider-policy snapshots, append-only
evidence supersession lineage, corruption detection, and crash/reopen
qualification. Ordinary Plan CAS cannot manufacture a Closed Plan without the
guarded closure record. See plans/closure/evidence-closure/002-closed.md.

### M002 C001 — Finalization subject revalidation

Status: closed.

Plan: plans/implementation/evidence-closure/002-c001-finalization-subject-revalidation.md

Closure: plans/closure/evidence-closure/002-c001-closed.md

Post-closure review found that RepositoryStore finalization recomputed
assessment under the Eggplan state lock but accepted the caller's previously
captured SubjectRevision as "current" authority. C001 moved authoritative
Git subject recapture into finalization, added a second pre-write recapture,
introduced typed `ClosureSubjectStale`, `ClosureSubjectDrift`, and
`ClosureSubjectCapture` errors, and preserved existing M002 crash recovery
and closure-record compatibility. CLI `close` no longer passes its own
captured subject to the repository and reports subject drift as a stable
machine diagnostic.

Historical M002 closure remains preserved. Projection/CLI M002, CodeGG M002,
and Eggstack M002 are unblocked.

### M003 — Policy extensions

Deferred. Only after consumers demonstrate need: ancestry-aware subject reuse,
signed-provider requirements, richer all/any/quorum semantics, or policy
profiles. Avoid a general policy language.

## 8. Verification

Golden matrices must cover every normalized status, missing or dangling refs,
stale subjects, invalid provider attribution, corrupt digests, human judgment,
mixed criteria, and deterministic repeated assessment.

## 9. Completion definition

This subsystem closes when closure cannot be manufactured by free-form claims,
status labels, unrelated same-kind observations, or self-perturbing Eggplan
state, and historical observations remain inspectable and reproducible.
