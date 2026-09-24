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
| Foundation core/repository | closed/current | M001 closed; M002 historical caveat resolved by M003; M003 closed and cross-platform qualified | plans/subsystems/foundation-core-roadmap.md |
| Evidence/closure | closed/current | M001/C001 and M002 guarded closure/integrity closed | plans/subsystems/evidence-closure-roadmap.md |
| Projection/CLI | blocked | M001 plan registered; waits on Evidence M002 closure | plans/subsystems/projection-cli-roadmap.md |
| CodeGG integration | blocked | M001 plan registered against current CodeGG baseline; waits on Evidence M002 closure | plans/subsystems/codegg-integration-roadmap.md |
| Eggstack integrations | blocked | Provider SPI M001 plan registered against refreshed sibling baselines; waits on Evidence M002 closure | plans/subsystems/eggstack-integration-roadmap.md |
| Interop/distribution | deferred | waits on local core/CLI/integrations | plans/subsystems/interoperability-distribution-roadmap.md |

## Registered implementation plans

| Subsystem | Milestone | Status | Implementation plan | Handoff |
|---|---|---|---|---|
| Foundation | M001 repository bootstrap + typed domain | closed | plans/implementation/foundation-core/001-repository-bootstrap-and-domain-contract.md | closure plans/closure/foundation-core/001-closed.md |
| Foundation | M002 repository store/CAS/Git subject | conditionally closed | plans/implementation/foundation-core/002-repository-store-cas-and-subject.md | historical closure; platform caveat resolved by M003 |
| Foundation | M003 subject scope + strict schema + platform hardening | closed | plans/implementation/foundation-core/003-subject-scope-strict-schema-and-platform-hardening.md | closure plans/closure/foundation-core/003-closed.md |
| Evidence | M001 evidence ledger/assessment | closed | plans/implementation/evidence-closure/001-evidence-ledger-and-assessment.md | historical closure plans/closure/evidence-closure/001-closed.md |
| Evidence | M001 C001 verification binding + end-to-end evidence corrective | closed | plans/implementation/evidence-closure/001-c001-verification-binding-and-end-to-end-evidence-corrective.md | closure plans/closure/evidence-closure/001-c001-closed.md |
| Evidence | M002 guarded closure records + supersession + recovery | closed | plans/implementation/evidence-closure/002-closure-records-integrity-and-recovery.md | closure plans/closure/evidence-closure/002-closed.md |
| Projection/CLI | M001 CLI control surface + derived registry | blocked | plans/implementation/projection-cli/001-cli-control-surface-and-derived-registry.md | waits on Evidence M002 closure |
| CodeGG integration | M001 golden parity + adapter seam | blocked | plans/implementation/codegg-integration/001-golden-parity-and-adapter-seam.md | waits on Evidence M002 closure; reviewed CodeGG 6e183045 |
| Eggstack integrations | M001 evidence provider SPI | blocked | plans/implementation/eggstack-integration/001-evidence-provider-spi.md | waits on Evidence M002 closure |

## Registered next implementation wave

The corrective gate is closed. The next implementation wave is intentionally
sequenced around durable closure authority:

1. Evidence M002 is closed; see plans/closure/evidence-closure/002-closed.md.
2. Projection/CLI M001, CodeGG Integration M001, and Eggstack Provider SPI M001
   are fully planned; refresh their repository and sibling baselines before
   marking them ready.
3. After baseline refresh, those three M001 handoffs may proceed
   independently because they consume the same stable closure/evidence
   contracts and do not own one another's runtime responsibilities.

No implementation plan is yet registered for Projection/CLI M002, CodeGG M002,
Eggstack M002, or interoperability/distribution.

## Current execution order

1. Foundation M001/M002/M003 — closed/current foundation.
2. Evidence M001 + C001 — closed; v2 verification binding is current.
3. Execute and close Evidence M002:
   - append-only evidence supersession lineage;
   - pure ClosureCandidate;
   - immutable ClosureRecord;
   - provider-policy snapshot;
   - raw CAS-to-Closed prohibition;
   - crash-consistent pending/final closure recovery.
4. Refresh dependent-plan baselines against the closure commit.
5. Then independently execute:
   - Projection/CLI M001 — versioned machine/human control surface;
   - CodeGG Integration M001 — golden parity and non-circular adapter seam;
   - Eggstack Provider SPI M001 — normalization/trust boundary only.
6. Stabilize CodeGG parity before staged CodeGG ownership migration.
7. Implement real Eggwork/Eggsearch providers only after Provider SPI closure.
8. Add attestation/service/distribution work after local contracts are
   qualified.

## External interface research baselines

Reviewed during planning; these are not dependency pins.

| Project/standard | Reviewed baseline | Relevant boundary |
|---|---|---|
| CodeGG | 6e18304546f29426457eb11410957288384fdec2 | current work_plan model/store/evidence/assessment/projection re-checked for M001; WorkOrder/Goal/Todo/checkpoints/runtime remain separate |
| Eggwork | c990ffa2864886c502c1d5581f0d732e26fe56ee | fixed-target execution spec/request digest, execution generations/snapshots, artifacts; executor not scheduler |
| Eggsearch | 5db6e1984a1441787f6d6a54754eb4a685766ec2 | deterministic evidence bundles with source/provider/trust/gap metadata |
| Eggbench | 3b93979a7fd30a08f7e367ffc94f911ae5fb8bec | .eggb manifest v2; execution status separated from comparison verdict |
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
10. A canonical Closed Plan must have a valid guarded ClosureRecord; ordinary
    CAS must never be a bypass around closure assessment.
11. Evidence correction is append-only lineage. Historical observations are
    never rewritten to make a later assessment pass.

## Planning hygiene

- Register before handoff.
- Preserve historical closure; use corrective plans.
- Record exact evidence and unrun/blocked checks.
- Do not copy Eggwork/Eggsearch/Eggbench responsibilities into eggplan-core.
- Keep core synchronous/deterministic where practical; async/network belongs in adapters.
- No hidden model reasoning in persisted schemas.
