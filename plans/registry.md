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
| Evidence/closure | closed/current | M001/C001 and M002/C001 closed historically; M002 C002 closed and cross-platform qualified | plans/subsystems/evidence-closure-roadmap.md |
| Projection/CLI | ready to plan | M001 closed historically; M002 Markdown import/render unblocked and needs a fresh bounded implementation plan | plans/subsystems/projection-cli-roadmap.md |
| CodeGG integration | ready to plan | M001 closed historically; M002 staged adoption unblocked and needs a fresh CodeGG interface recheck | plans/subsystems/codegg-integration-roadmap.md |
| Eggstack integrations | ready to plan | M001 provider SPI closed historically; live-provider M002 unblocked and needs fresh Eggwork/Eggsearch interface rechecks | plans/subsystems/eggstack-integration-roadmap.md |
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
| Projection/CLI | M001 CLI control surface + derived registry | closed | plans/implementation/projection-cli/001-cli-control-surface-and-derived-registry.md | closure plans/closure/projection-cli/001-closed.md |
| CodeGG integration | M001 golden parity + adapter seam | closed | plans/implementation/codegg-integration/001-golden-parity-and-adapter-seam.md | closure plans/closure/codegg-integration/001-closed.md |
| Eggstack integrations | M001 evidence provider SPI | closed | plans/implementation/eggstack-integration/001-evidence-provider-spi.md | closure plans/closure/eggstack-integration/001-closed.md |

## Registered corrective gate

Evidence M002 C001 is historically closed. Evidence M002 C002 is now
closed. The deterministic `SubjectCapture` seam and alternate
capture-injected finalizer that C001 had marked `#[doc(hidden)] pub` are
now crate-private / `#[cfg(test)]` only; no downstream crate can reach
the closure subject authority. Crate-level `compile_fail` doctests in
`eggplan-repo` and `scripts/check-closure-authority-boundary.sh` (wired
into the hosted CI native shell guard step) prevent re-publication. The
historical C001 closure placeholders were reconciled with the actual
hosted workflow:

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

Projection/CLI M002, CodeGG M002, and Eggstack M002 are unblocked; each
still requires its own bounded implementation plan and a fresh
sibling-interface recheck before handoff. No other corrective
implementation plan is currently registered.

## Current execution order

1. Foundation M001/M002/M003 — closed/current foundation.
2. Evidence M001 + C001 — closed; v2 verification binding is current.
3. Evidence M002 — historically closed with guarded closure, supersession, and
   crash recovery.
4. Evidence M002 C001 — historically closed; production S1/S2 recapture path
   implemented; C002 closed the public test-seam exposure.
5. Evidence M002 C002 — closed; deterministic test seam is now
   crate-private; compile-fail doctests and `check-closure-authority-boundary.sh`
   enforce the boundary; C001 CI placeholders reconciled.
6. CodeGG Integration M001 — historically closed.
7. Eggstack Provider SPI M001 — historically closed.
8. Projection/CLI M001 — historically closed.
9. Independently plan the next capability wave:
   - Projection/CLI M002 — Markdown import/render;
   - CodeGG M002 — staged core adoption after a fresh CodeGG interface check;
   - Eggstack M002 — real Eggwork/Eggsearch adapters after sibling re-check.
10. Add attestation/service/distribution work after local contracts and live
    integrations are qualified.

## External interface research baselines

Reviewed during planning; these are not dependency pins.

| Project/standard | Reviewed baseline | Relevant boundary |
|---|---|---|
| CodeGG | 28b4695661d463dd1675d045ac6299c5fbc9ea31 | current WorkPlan model/store/evidence/assessment/projection; WorkOrder/Goal/Todo/checkpoints/runtime remain separate |
| Eggwork | 128f808c62f176d414dd18a705773e45f5e2891a | fixed-target execution spec and canonical request/workspace digests, generations/snapshots, artifacts; executor not scheduler |
| Eggsearch | dfa90e050c5434f3346902aeb4074901c58e90d1 | deterministic evidence bundles with source/provider/trust/gap metadata |
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

## Planning hygiene

- Register before handoff.
- Preserve historical closure; use corrective plans.
- Record exact evidence and unrun/blocked checks.
- Do not copy Eggwork/Eggsearch/Eggbench responsibilities into eggplan-core.
- Keep core synchronous/deterministic where practical; async/network belongs in adapters.
- No hidden model reasoning in persisted schemas.
