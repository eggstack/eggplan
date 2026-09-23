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
| Foundation core/repository | ready | M001 closed; M002 conditionally closed historically; M003 closed with native Linux/macOS/Windows qualification | plans/subsystems/foundation-core-roadmap.md |
| Evidence/closure | corrective required | M001 closed historically; C001 ready after Foundation M003; M002 blocked on C001 | plans/subsystems/evidence-closure-roadmap.md |
| Projection/CLI | blocked | M001 waits on Foundation M003 + Evidence C001 so it does not bind to known-incomplete schema/subject semantics | plans/subsystems/projection-cli-roadmap.md |
| CodeGG integration | blocked | M001 waits on Foundation M003 + Evidence C001 before golden parity/adaptation | plans/subsystems/codegg-integration-roadmap.md |
| Eggstack integrations | blocked | Provider SPI waits on Evidence C001 corrected matching contract | plans/subsystems/eggstack-integration-roadmap.md |
| Interop/distribution | deferred | waits on local core/CLI/integrations | plans/subsystems/interoperability-distribution-roadmap.md |

## Registered implementation plans

| Subsystem | Milestone | Status | Implementation plan | Handoff |
|---|---|---|---|---|
| Foundation | M001 repository bootstrap + typed domain | closed | plans/implementation/foundation-core/001-repository-bootstrap-and-domain-contract.md | closure plans/closure/foundation-core/001-closed.md |
| Foundation | M002 repository store/CAS/Git subject | conditionally closed | plans/implementation/foundation-core/002-repository-store-cas-and-subject.md | historical closure; platform caveat resolved by M003 |
| Foundation | M003 subject scope + strict schema + platform hardening | closed | plans/implementation/foundation-core/003-subject-scope-strict-schema-and-platform-hardening.md | closure plans/closure/foundation-core/003-closed.md |
| Evidence | M001 evidence ledger/assessment | closed | plans/implementation/evidence-closure/001-evidence-ledger-and-assessment.md | historical closure plans/closure/evidence-closure/001-closed.md |
| Evidence | M001 C001 verification binding + end-to-end evidence corrective | ready | plans/implementation/evidence-closure/001-c001-verification-binding-and-end-to-end-evidence-corrective.md | Foundation M003 closed |

## Corrective gate and later roadmap work

The next two handoffs are intentionally serialized:

1. Foundation M003 — closed at 7656aefff9f809f963bba6f4373ac8f1603445e9; hosted native workflow 35860695866 passed.
2. Evidence M001 C001 — ready and next in execution order.

The following roadmap milestones are temporarily blocked behind that corrective
gate and intentionally do not yet have implementation plans:

| Subsystem | Milestone | Blocker |
|---|---|---|
| Evidence/closure | M002 closure records and integrity/recovery | Evidence M001 C001 closure |
| Projection/CLI | M001 CLI control surface and derived registry | Foundation M003 + Evidence M001 C001 closure |
| CodeGG integration | M001 golden parity and adapter seam | Foundation M003 + Evidence M001 C001 closure |
| Eggstack integrations | M001 provider SPI | Evidence M001 C001 closure |

Do not write or execute these downstream plans merely to increase plan count.
Re-open planning after the corrective contracts are qualified.

## Current execution order

1. Foundation M001 — historically closed at 08a9cdb.
2. Foundation M002 — historically conditionally closed at d154234; its
   cross-platform caveat and later subject-scope finding are carried forward.
3. Evidence M001 — historically closed at e711355; later review found
   verification matching too broad for specific execution requirements.
4. Execute Foundation M003 (closed):
   - exclude Eggplan-managed state from source dirty-subject identity;
   - make schema-v1 nested parsing fail closed;
   - add native Linux/macOS/Windows CI and close or narrowly document platform gaps;
   - repair directly related README/architecture drift.
5. After Foundation M003 closed, execute Evidence M001 C001:
   - introduce verification-spec digest binding with explicit v1/v2 compatibility;
   - fail closed for legacy unbound execution requirements;
   - add the persist/recapture/assess regression and architecture/evidence.md.
6. Only after both correctives close, plan/execute Evidence M002,
   Projection/CLI M001, CodeGG Integration M001, and Eggstack Provider SPI M001
   against the corrected contracts.
7. Stabilize CodeGG parity before staged CodeGG ownership migration.
8. Add attestation/service/distribution work after local contracts are qualified.

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
8. Eggplan's own managed state must not perturb the source SubjectRevision used
   to judge evidence applicability.
9. Execution-derived evidence must bind to the verification specification it
   actually observed; kind/provider/subject matching alone is insufficient for
   specific verification authority.

## Planning hygiene

- Register before handoff.
- Preserve historical closure; use corrective plans.
- Record exact evidence and unrun/blocked checks.
- Do not copy Eggwork/Eggsearch/Eggbench responsibilities into eggplan-core.
- Keep core synchronous/deterministic where practical; async/network belongs in adapters.
- No hidden model reasoning in persisted schemas.
