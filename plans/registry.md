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
| Evidence/closure | closed/current | M002 and C001/C002/C003 closed; non-blocking C003 hygiene complete | plans/subsystems/evidence-closure-roadmap.md |
| Projection/CLI | closed/current | M001 and M002 closed; M003 waits for real repository use | plans/subsystems/projection-cli-roadmap.md |
| CodeGG integration | blocked | M001 closed; M002 blocked on CodeGG Eggplan-integration M001 execution-subject provenance, now registered upstream | plans/subsystems/codegg-integration-roadmap.md |
| Eggstack integrations | active | M001 provider SPI closed; M002 Eggwork/Eggsearch DTO adapters in closing | plans/subsystems/eggstack-integration-roadmap.md |
| Interop/distribution | deferred | waits on local core/CLI/integrations | plans/subsystems/interoperability-distribution-roadmap.md |

## Registered implementation plans

| Subsystem | Milestone | Status | Implementation plan | Handoff |
|---|---|---|---|---|
| Foundation | M001 repository bootstrap + typed domain | closed | plans/implementation/foundation-core/001-repository-bootstrap-and-domain-contract.md | closure plans/closure/foundation-core/001-closed.md |
| Foundation | M002 repository store/CAS/Git subject | conditionally closed | plans/implementation/foundation-core/002-repository-store-cas-and-subject.md | historical closure; platform caveat resolved by M003 |
| Foundation | M003 subject scope + strict schema + platform hardening | closed | plans/implementation/foundation-core/003-subject-scope-strict-schema-and-platform-hardening.md | closure plans/closure/foundation-core/003-closed.md |
| Evidence | M001 evidence ledger/assessment | closed | plans/implementation/evidence-closure/001-evidence-ledger-and-assessment.md | historical closure plans/closure/evidence-closure/001-closed.md |
| Evidence | M001 C001 verification binding + end-to-end evidence corrective | closed | plans/implementation/evidence-closure/001-c001-verification-binding-and-end-to-end-evidence-corrective.md | closure plans/closure/evidence-closure/001-c001-closed.md |
| Evidence | M002 guarded closure records + supersession + recovery | closed | plans/implementation/evidence-closure/002-closure-records-integrity-and-recovery.md | historical closure plans/closure/evidence-closure/002-closed.md |
| Evidence | M002 C001 finalization subject revalidation | closed | plans/implementation/evidence-closure/002-c001-finalization-subject-revalidation.md | historical closure plans/closure/evidence-closure/002-c001-closed.md |
| Evidence | M002 C002 finalization test-seam containment + closure evidence reconciliation | closed | plans/implementation/evidence-closure/002-c002-finalization-test-seam-containment-and-closure-evidence-reconciliation.md | closure plans/closure/evidence-closure/002-c002-closed.md; preserves M002/C001 historical closures |
| Evidence | M002 C003 closure reference + authority-guard hygiene | closed | plans/implementation/evidence-closure/002-c003-closure-reference-and-authority-guard-hygiene.md | closure plans/closure/evidence-closure/002-c003-closed.md; non-blocking maintenance |
| Projection/CLI | M001 CLI control surface + derived registry | closed | plans/implementation/projection-cli/001-cli-control-surface-and-derived-registry.md | closure plans/closure/projection-cli/001-closed.md |
| Projection/CLI | M002 loss-aware Markdown import + deterministic render | closed | plans/implementation/projection-cli/002-loss-aware-markdown-import-and-deterministic-render.md | closure plans/closure/projection-cli/002-closed.md |
| CodeGG integration | M001 golden parity + adapter seam | closed | plans/implementation/codegg-integration/001-golden-parity-and-adapter-seam.md | closure plans/closure/codegg-integration/001-closed.md |
| CodeGG integration | M002 staged Eggplan assessment adoption | blocked | plans/implementation/codegg-integration/002-staged-eggplan-assessment-adoption.md | upstream CodeGG M001 provenance plan registered at af0a3e0; resume after positive closure |
| Eggstack integrations | M001 evidence provider SPI | closed | plans/implementation/eggstack-integration/001-evidence-provider-spi.md | closure plans/closure/eggstack-integration/001-closed.md |
| Eggstack integrations | M002 Eggwork + Eggsearch evidence adapters | closing | plans/implementation/eggstack-integration/002-eggwork-and-eggsearch-evidence-adapters.md | local implementation and checks; hosted qualification pending |

## Corrective history and current maintenance

Evidence M002 C001 and C002 are historically closed. C002 made the closure
subject authority boundary mechanically true in the public API and reconciled
C001's hosted qualification evidence.

Recorded hosted evidence remains:

- C001 implementation run: 35998715018
  - Linux 107629794960
  - macOS 107629795068
  - Windows 107629794971
  - Rust 1.89 107629794849
- C002 implementation run: 36004813178
  - Linux 107650129619
  - macOS 107650129837
  - Windows 107650129154
  - Rust 1.89 107650129645

The non-blocking maintenance correction is closed:

- Evidence M002 C003 — correct the C002 implementation-SHA citation and
  strengthen the static closure-authority guard/negative proof.

C003 does not gate Projection/CLI M002, CodeGG M002, or Eggstack M002.

## Current execution order

1. Foundation M001/M002/M003 — closed/current foundation.
2. Evidence M001 + C001 — closed; v2 verification binding is current.
3. Evidence M002 + C001 + C002 — closed/current guarded closure and subject
   authority.
4. CodeGG Integration M001 — historically closed.
5. Eggstack Provider SPI M001 — historically closed.
6. Projection/CLI M001 — historically closed.
7. CodeGG M002 is blocked on one explicit upstream CodeGG handoff:
   `plans/implementation/eggplan-assessment-integration/001-durable-execution-subject-provenance.md`,
   registered in dbowm91/codegg at `af0a3e0`. That plan captures/persists
   attempt-scoped execution subjects and never backfills legacy evidence from
   the current worktree.
8. Projection/CLI M002 is closed. Eggstack M002 remains ready and independent
   of the CodeGG provenance handoff; its sibling interfaces must be rechecked
   at handoff. Evidence M002 C003 closed as non-blocking hygiene without
   gating either capability plan.
9. After positive M002 closures:
   - Projection/CLI M003 ergonomics/performance may be planned from real use;
   - CodeGG M003 repository Plan binding may be planned;
   - Eggstack M003 Eggbench/CI/forge adapters may be planned after a fresh
     Eggbench interface recheck.
10. Interoperability/distribution remains deferred until the local capability
   wave is qualified.

## External interface research baselines

Reviewed during planning; these are not dependency pins.

| Project/standard | Reviewed baseline | Relevant boundary |
|---|---|---|
| CodeGG | f4e6e69d9e968e2adbb4228b3a7d45f55bd1294c (interfaces); af0a3e0fb9b6552f45e3ea5d698e7980582493fd (registered provenance handoff) | WorkPlan runtime ownership remains CodeGG; current blocker is attempt-scoped execution subject provenance; M001 Eggplan fixture baseline remains a3c87fc/28b469 historical |
| Eggwork | 128f808c62f176d414dd18a705773e45f5e2891a | protocol-neutral execution snapshots/results/generation/artifact records; executor/scheduler remain outside Eggplan |
| Eggsearch | 5db6e1984a1441787f6d6a54754eb4a685766ec2 | deterministic EvidenceBundle source/provider/trust/gap metadata; full runtime must not become an Eggplan dependency |
| Eggbench | d7d1fd9a9b67a5b2ca6a816c841d2588a368aae9 | .eggb manifest v2; execution status separated from comparison verdict |
| Eggsact | 40959b704431430668e9ca2bfe959a8ef32495d8 | deterministic in-process/preflight utilities |
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
12. Guarded closure finalization owns its current SubjectRevision capture;
    callers may propose a candidate subject but cannot assert current subject
    authority at commit time. `RepositoryStore::finalize_closure` no longer
    accepts a caller-supplied current subject.
13. The finalizer revalidates subject stability immediately before its first
    canonical closure write; arbitrary external worktree writers remain
    outside Eggplan's lock and later changes make closure historical/stale, not
    corrupt. Subject mismatch, drift, and capture failure are typed
    `RepoError` variants distinct from invalid-update / lifecycle / corrupt
    errors.
14. Test seams are not authority seams: no downstream/public API may inject the
    SubjectRevision source used by guarded closure finalization. `#[doc(hidden)]`
    is not an access-control boundary.
15. Closure records may cite only observed completed verification; placeholder
    workflow/job IDs are not passing evidence.
16. Markdown import is plan-intent ingestion only: source lifecycle, prose,
    evidence claims, provider trust, SubjectRevision, and closure are not
    authority.
17. CodeGG staged adoption must use a pure Eggplan assessment bridge in
    production, keep CodeGG storage/runtime ownership, and derive execution
    VerificationDigest values from authoritative native execution specs rather
    than prose/reference IDs.
18. Eggsearch trust labels are content provenance, never Eggplan provider
    enrollment; Eggwork execution verification binding is host-supplied.
19. Sibling commit SHAs recorded by compatibility/adapters are fixture/review
    provenance, not runtime trust or branch dependencies.
20. Historical CodeGG execution subjects must come from durable attempt-time
    provenance. Eggplan integration must never reconstruct an older job/run
    subject from CodeGG's current worktree.

## Planning hygiene

- Register before handoff.
- Preserve historical closure; use corrective plans.
- Record exact evidence and unrun/blocked checks.
- Do not copy Eggwork/Eggsearch/Eggbench responsibilities into eggplan-core.
- Keep core synchronous/deterministic where practical; async/network belongs in adapters.
- No hidden model reasoning in persisted schemas.
- Mark maintenance gates explicitly: non-blocking hygiene must not serialize
  otherwise independent capability handoffs.
