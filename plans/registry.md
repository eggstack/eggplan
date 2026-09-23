# Eggplan Active Planning Registry

This file is the compact control surface for current Eggplan planning.
Detailed authority remains in specifications, ADRs, subsystem roadmaps,
implementation plans, closure records, and Git history.

## Canonical direction

- plans/000-long-term-specification.md
- plans/001-terminology-and-domain-model.md
- plans/002-long-term-roadmap.md
- plans/003-planning-process.md

## Status vocabulary

- proposed — document exists but is not dependency-ready.
- ready — hard dependencies/interfaces are satisfied; handoff is authorized.
- active — implementation is in progress.
- blocked — named dependency/evidence prevents progress.
- closing — implementation landed and closure evidence is being gathered.
- closed — closure record accepted.
- conditionally closed — implementation complete with named external evidence outstanding.
- corrective required — later defect must close before current qualification.
- superseded — replaced by another plan/decision.
- archived — retained for traceability.
- deferred — intentionally outside current implementation horizon.

## Repository planning baseline

eggstack/eggplan was an empty Git repository when planning was established on
2026-09-22/23. The repository was initialized with README.md at:

- ce2d6e7a6c1cecbe7ff554daf9d603d2d604d98c — repository initialization.

No Rust workspace or production capability existed at that baseline. Planning
documents are not evidence that implementation exists.

Planning-system bootstrap:

- 42ec41eff3e35c15acacaa0d717320c40543fbc1 — canonical specs, ADRs, subsystem roadmaps, registry, and initial implementation handoffs.

## Accepted architectural decisions

| ADR | Status | Decision |
|---|---|---|
| plans/adrs/ADR-0001-planning-evidence-mechanism-not-execution.md | accepted | Eggplan owns plan/evidence/closure mechanism, not scheduling or execution |
| plans/adrs/ADR-0002-repository-first-versioned-canonical-state.md | accepted | Versioned structured repository state is canonical; Markdown is projection/import |
| plans/adrs/ADR-0003-immutable-revision-scoped-evidence.md | accepted | Observations are immutable, provider-authoritative and subject-revision scoped |
| plans/adrs/ADR-0004-codegg-extraction-without-workorder-conflation.md | accepted | Generalize CodeGG WorkPlan semantics incrementally; WorkOrder remains CodeGG-owned |

## Active subsystem roadmaps

| Subsystem | Status | Current milestone | Authority |
|---|---|---|---|
| Foundation core/repository | ready | M001 ready; M002 blocked on M001 | plans/subsystems/foundation-core-roadmap.md |
| Evidence/closure | blocked | M001 waits on Foundation M002 | plans/subsystems/evidence-closure-roadmap.md |
| Projection/CLI | blocked | M001 waits on Evidence M001 | plans/subsystems/projection-cli-roadmap.md |
| CodeGG integration | blocked | M001 waits on Foundation M002 + Evidence M001 | plans/subsystems/codegg-integration-roadmap.md |
| Eggstack integrations | proposed | provider SPI waits on evidence contract | plans/subsystems/eggstack-integration-roadmap.md |
| Interop/distribution | deferred | waits on local core/CLI/integrations | plans/subsystems/interoperability-distribution-roadmap.md |

## Dependency-ready implementation plans

| Subsystem | Milestone | Status | Implementation plan | Handoff |
|---|---|---|---|---|
| Foundation | M001 repository bootstrap + typed domain contract | ready | plans/implementation/foundation-core/001-repository-bootstrap-and-domain-contract.md | first implementation handoff |

## Registered blocked implementation plans

| Subsystem | Milestone | Status | Implementation plan | Blocker |
|---|---|---|---|---|
| Foundation | M002 repository store/CAS/Git subject | blocked | plans/implementation/foundation-core/002-repository-store-cas-and-subject.md | Foundation M001 closure |
| Evidence | M001 evidence ledger/assessment | blocked | plans/implementation/evidence-closure/001-evidence-ledger-and-assessment.md | Foundation M002 closure |

Later roadmap milestones intentionally do not yet have implementation plans.
Write those only after their prerequisite interfaces are concrete.

## Current execution order

1. Foundation M001 — create workspace + eggplan-core typed/domain/schema contract.
2. Close M001 with domain/bounds/graph/schema evidence.
3. Foundation M002 — repository store, CAS, safe persistence, Git SubjectRevision.
4. Close M002 including contention/interrupted-write/cross-platform evidence.
5. Evidence M001 — immutable observation ledger + subject-aware assessment.
6. After Evidence M001, Projection/CLI M001 and CodeGG Integration M001 may be
   planned/executed independently.
7. Stabilize CodeGG parity before staged CodeGG ownership migration.
8. Add Eggstack provider adapters only against the stable evidence-provider seam.
9. Add attestation/service/distribution work after local contracts are qualified.

## External interface research baselines

Reviewed during planning; these are not dependency pins.

| Project/standard | Reviewed baseline | Relevant boundary |
|---|---|---|
| CodeGG | 2f7d84f88070eee2a4fb9f70b6d7d5d10b01048d | codegg-core::work_plan model/store/evidence/assessment/projection; WorkOrder remains separate |
| Eggwork | 9b34717319bf99cf90061601cc0869a87e1a25ec | fixed-target execution provider; executor not scheduler |
| Eggsearch | 68ae2fa5457c3fb8fa335851d60d0dce3e98aa08 | repo research/fetch and deterministic evidence bundles |
| Eggbench | cbca21a8b3eb24ec8ecbf58febbc8c505c7c8613 | immutable .eggb evidence bundle and environment/trial provenance |
| Eggsact | 576f4b0ac09238a42e5561c2da6da8ff4a47bce6 | deterministic in-process/preflight utilities |
| Eggup | cf5b3d3819c168eb2dbf841daa8332f3eb28c915 | future verified distribution/update consumer |
| SLSA | v1.2 approved provenance docs reviewed 2026-09-22 | subject + provenance separation |
| in-toto | attestation framework stable v1.0 reviewed 2026-09-22 | statement/predicate and signed supply-chain evidence model |
| GitHub attestations | docs reviewed 2026-09-22 | Sigstore-backed artifact provenance; validity does not imply semantic safety |

Implementation agents MUST re-check current sibling interfaces when the
integration milestone is actually handed off.

## Key design gates

1. Exact canonical JSON normalization/digest rules must be frozen in Foundation
   M001 before evidence digests depend on them.
2. Dirty Git subject identity must be deterministic and bounded before evidence
   can claim exact-revision validity.
3. Repository persistence must prove atomicity/recovery/CAS before multiple
   agents use it concurrently.
4. Evidence provider identity is host/adapter authority, never caller prose.
5. A passing observation for stale source remains historical evidence, not
   current closure proof.
6. CodeGG migration must preserve Goal/Todo/checkpoint/context-epoch ownership
   and must not turn Eggplan into CodeGG's WorkOrder scheduler.
7. Eggbench bundle reuse should be preferred over inventing another large
   immutable bundle format.

## Planning hygiene

- Register before handoff.
- Preserve historical closure; use corrective plans.
- Record exact evidence and unrun/blocked checks.
- Do not copy Eggwork/Eggsearch/Eggbench responsibilities into eggplan-core.
- Keep core synchronous/deterministic where practical; async/network belongs in adapters.
- No hidden model reasoning in persisted schemas.
