# Evidence and Closure Roadmap

Status: ready

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

Status: ready after Foundation M002 conditional closure. Preserve the
Windows/macOS qualification caveat from Foundation M002.

Plan: plans/implementation/evidence-closure/001-evidence-ledger-and-assessment.md

Deliver normalized statuses, provider identities, append-only observations,
criterion matching, exact-subject policy, pure assessment, and explanations.

### M002 — Closure records and integrity/recovery

Roadmap-level after M001.

Add closure candidate snapshots, requirement-to-evidence matrices, corrupt
observation detection, correction/supersession lineage, and repository reopen
qualification.

### M003 — Policy extensions

Deferred. Only after consumers demonstrate need: ancestry-aware subject reuse,
signed-provider requirements, richer all/any/quorum semantics, or policy
profiles. Avoid a general policy language.

## 8. Verification

Golden matrices must cover every normalized status, missing or dangling refs,
stale subjects, invalid provider attribution, corrupt digests, human judgment,
mixed criteria, and deterministic repeated assessment.

## 9. Completion definition

This subsystem closes when closure cannot be manufactured by free-form claims
or status labels and historical observations remain inspectable and
reproducible.
