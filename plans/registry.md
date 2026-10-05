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
| Evidence/closure | closed with condition | M002 and C001/C002/C003/C004 closed; the C004 durability defect is resolved — ordinary CAS can no longer modify a Closed Plan, so no reachable path desynchronizes it from its ClosureRecord | plans/subsystems/evidence-closure-roadmap.md |
| Projection/CLI | closed/current | M001 and M002 closed; M003 waits for real repository use | plans/subsystems/projection-cli-roadmap.md |
| CodeGG integration | closed/current | M001-M003 and C001/C002 closed; the roadmap is terminal. C002 closed the boundary guard's filesystem gap: `check-codegg-compat-boundary.sh` now fails on filesystem access, every guard message states its actual scan scope, and all five guards have synthetic self-proofs. The bridge source was already pure; the defect was in the enforcement | plans/subsystems/codegg-integration-roadmap.md |
| Eggstack integrations | closed/current | M001 provider SPI and M002 Eggwork/Eggsearch adapters closed; M003 ready for planning against rechecked Eggbench contract | plans/subsystems/eggstack-integration-roadmap.md |
| Interop/distribution | deferred | waits on local core/CLI/integrations | plans/subsystems/interoperability-distribution-roadmap.md |

## Registered implementation plans

| Subsystem | Milestone | Status | Implementation plan | Handoff |
|---|---|---|---|---|
| Foundation | M001 repository bootstrap + typed domain | closed | plans/implementation/foundation-core/001-repository-bootstrap-and-domain-contract.md | closure plans/closure/foundation-core/001-closed.md |
| Foundation | M002 repository store/CAS/Git subject | conditionally closed | plans/implementation/foundation-core/002-repository-store-cas-and-subject.md | historical closure plans/closure/foundation-core/002-conditionally-closed.md; platform caveat resolved by M003 |
| Foundation | M003 subject scope + strict schema + platform hardening | closed | plans/implementation/foundation-core/003-subject-scope-strict-schema-and-platform-hardening.md | closure plans/closure/foundation-core/003-closed.md |
| Evidence | M001 evidence ledger/assessment | closed | plans/implementation/evidence-closure/001-evidence-ledger-and-assessment.md | historical closure plans/closure/evidence-closure/001-closed.md |
| Evidence | M001 C001 verification binding + end-to-end evidence corrective | closed | plans/implementation/evidence-closure/001-c001-verification-binding-and-end-to-end-evidence-corrective.md | closure plans/closure/evidence-closure/001-c001-closed.md |
| Evidence | M002 guarded closure records + supersession + recovery | closed | plans/implementation/evidence-closure/002-closure-records-integrity-and-recovery.md | historical closure plans/closure/evidence-closure/002-closed.md |
| Evidence | M002 C001 finalization subject revalidation | closed | plans/implementation/evidence-closure/002-c001-finalization-subject-revalidation.md | historical closure plans/closure/evidence-closure/002-c001-closed.md |
| Evidence | M002 C002 finalization test-seam containment + closure evidence reconciliation | closed | plans/implementation/evidence-closure/002-c002-finalization-test-seam-containment-and-closure-evidence-reconciliation.md | closure plans/closure/evidence-closure/002-c002-closed.md; preserves M002/C001 historical closures |
| Evidence | M002 C003 closure reference + authority-guard hygiene | closed | plans/implementation/evidence-closure/002-c003-closure-reference-and-authority-guard-hygiene.md | closure plans/closure/evidence-closure/002-c003-closed.md; non-blocking maintenance |
| Evidence | M002 C004 Closed-Plan CAS immutability | closed | plans/implementation/evidence-closure/002-c004-closed-plan-cas-immutability.md | closure plans/closure/evidence-closure/002-c004-closed.md; restores design gate 10: ordinary CAS cannot modify a Closed Plan in any direction |
| Projection/CLI | M001 CLI control surface + derived registry | closed | plans/implementation/projection-cli/001-cli-control-surface-and-derived-registry.md | closure plans/closure/projection-cli/001-closed.md |
| Projection/CLI | M002 loss-aware Markdown import + deterministic render | closed | plans/implementation/projection-cli/002-loss-aware-markdown-import-and-deterministic-render.md | closure plans/closure/projection-cli/002-closed.md |
| CodeGG integration | M001 golden parity + adapter seam | closed | plans/implementation/codegg-integration/001-golden-parity-and-adapter-seam.md | closure plans/closure/codegg-integration/001-closed.md |
| CodeGG integration | M002 staged Eggplan assessment adoption | closed | plans/implementation/codegg-integration/002-staged-eggplan-assessment-adoption.md | closure plans/closure/codegg-integration/002-closed.md; Eggplan bridge 088968b + 1291799, CodeGG adoption 3e992291/3c7438c7, CodeGG closure ffa1c15e |
| CodeGG integration | M003 repository Plan binding contract | closed | plans/implementation/codegg-integration/003-repository-plan-binding-contract.md | Eggplan contract implementation `3f7c603` qualified by hosted run `36868055136`; the CodeGG consumer is implemented at CodeGG `53dea47f` and hosted-qualified there by run `36938461935` (success), satisfying the former condition. Closure: plans/closure/codegg-integration/003-conditionally-closed.md |
| CodeGG integration | M003 C001 dirty-subject fingerprint + bound-evidence requalification | closed | plans/implementation/codegg-integration/003-c001-dirty-subject-fingerprint-and-bound-evidence-requalification.md | Eggplan fingerprint contract implemented at `0dd33b7` (hosted run `37063328954`, all four jobs green; earlier Windows failures `37062437251`/`37062837529` retained as non-passing evidence); CodeGG half implemented at `36ec9322`, pinned `0dd33b7` and consumed the API in `3623f65e` + `b470865a` (PR `dbowm91/codegg#90`, hosted `CI` `37084905013` green on `main`). Closure: plans/closure/codegg-integration/003-c001-closed.md. Historical M003 remains closed. |
| CodeGG integration | M003 C002 compatibility boundary-guard completeness | closed | plans/implementation/codegg-integration/003-c002-boundary-guard-completeness.md | closure plans/closure/codegg-integration/003-c002-closed.md; non-blocking tooling hardening — the bridge source was already pure |
| Eggstack integrations | M001 evidence provider SPI | closed | plans/implementation/eggstack-integration/001-evidence-provider-spi.md | closure plans/closure/eggstack-integration/001-closed.md |
| Eggstack integrations | M002 Eggwork + Eggsearch evidence adapters | closed | plans/implementation/eggstack-integration/002-eggwork-and-eggsearch-evidence-adapters.md | closure plans/closure/eggstack-integration/002-closed.md |

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

Two corrective plans are registered and open as of `60a5f92`, both raised by a
code-verified architecture review rather than by a failed or skipped check. All
planned verification was run and green at that baseline; both defects are
inference- and reachability-established, not test failures.

- Evidence M002 C004 — closed at `623dde9`. Ordinary compare-and-swap accepted a
  `Closed` → `Closed` rewrite at `crates/eggplan-repo/src/store.rs:930-932`:
  the guarded-closure guard only fired when the current status was not `Closed`,
  and the transition check short-circuited on equal statuses, so
  `plan_transition_allowed` was never consulted. `atomic_write` committed at
  `revision + 1` before the re-read failed `ClosureRecord::validate`, which pins
  revision and digest, so the plan and the whole state root stopped opening with
  no recovery path. Blast radius was durable availability, not closure bypass: no
  record was forged and no authority was gained. An unconditional guard now
  refuses any CAS against a Closed Plan before `atomic_write`, with the typed
  `RepoError::ClosedPlanImmutable` (CLI code `closed_plan_immutable`).
  `plan_transition_allowed` is unchanged — `Active` → `Closed` stays legal
  because the guarded finalizer performs that transition. The Evidence/closure
  gate is lifted.
- CodeGG M003 C002 — closed at `8c4f6e6`. The source guard matched only
  process, network, and database access, so a `std::fs`, `File::open`, or
  `read_to_string` in the bridge source would have passed CI. `no_impure_source`
  now matches `std::fs`, `std::path`, `tempfile::`, the `use`-imported `fs::`
  call forms, and the `File::` constructors, covering both the
  fully-qualified and imported forms. The manifest guards stay unscoped by
  deliberate decision and their messages now state the section actually scanned.
  The bridge source was always pure — `src/lib.rs` imports only
  `std::collections` — so the defect was in the enforcement, not the crate.
  No crate file changed.

C004 and C002 are both closed, so no registered corrective remains open in any
subsystem. Neither gated the terminal CodeGG roadmap or any other subsystem.

## Current execution order

1. Foundation M001/M002/M003 — closed/current foundation.
2. Evidence M001 + C001 — closed; v2 verification binding is current.
3. Evidence M002 + C001 + C002 + C003 + C004 — closed guarded closure, subject
   authority, guard hygiene, and Closed-Plan CAS immutability. The C004
   durability defect is resolved, so the Evidence/closure gate is lifted and
   Projection/CLI M002 and Eggstack M002 are no longer blocked behind it. No
   Evidence/closure corrective is currently open.
4. CodeGG Integration M001 — historically closed.
5. Eggstack Provider SPI M001 — historically closed.
6. Projection/CLI M001 — historically closed.
7. CodeGG M002 is closed on both sides. The upstream provenance predecessor is
   `eggplan-assessment-integration/001-durable-execution-subject-provenance.md`
   **in the CodeGG repository `dbowm91/codegg` — not a path in this
   repository.** It closed there at `418fdc85656e7e1faa57f71e5e7f10f7f4859c60`
   (hosted CI run `36106606574` green) and made subject provenance
   attempt-scoped. Eggplan then closed its pure bridge (088968b, test matrix
   1291799), and CodeGG closed staged adoption in `3e992291`/ffa1c15e with
   hosted canonical run `36760308368`. Differential adoption and
   verification-digest derivation are no longer outstanding.
8. Projection/CLI M002 and Eggstack M002 are closed, independent of the CodeGG
   provenance handoff. Eggstack M003 is ready for planning after rechecking
   Eggbench at `d870512a5a1af16276ff05286ff0b6e2366b7f8f`. Evidence M002 C003
   closed as non-blocking hygiene without gating either capability plan.
9. After positive M002 closures:
   - Eggplan's M003 pure contract is closed: hosted qualification at
     `3f7c603` and the CodeGG consuming implementation landed at CodeGG
     `53dea47f`, hosted-qualified there by run `36938461935` (success). Both
     sides of the cross-repository contract are closed; the CodeGG
     integration roadmap is terminal for M003.
   - CodeGG M003 C001 dirty-subject corrective is closed on both sides
     (Eggplan contract `352a0f7`, qualified `0dd33b7`; CodeGG `36ec9322` plus
     the pin bump in `3623f65e`/`b470865a`). The CodeGG integration roadmap is
     terminal; Projection/CLI M003, Eggstack M003, and other capability work
     never had to serialize on it;
   - CodeGG M003 C002 boundary-guard completeness is closed at `8c4f6e6`. It
     hardened the static ownership guard only; the bridge was already pure, the
     roadmap stayed terminal, and nothing serialized on it;
   - Projection/CLI M003 ergonomics/performance may be planned from real use;
   - Eggstack M003 Eggbench/CI/forge adapters are ready for planning after the
     recheck above.
10. Interoperability/distribution remains deferred until the local capability
    wave is qualified.

## External interface research baselines

Reviewed during planning; these are not dependency pins.

| Project/standard | Reviewed baseline | Relevant boundary |
|---|---|---|
| CodeGG | 3623f65e + b470865a (C001 pin bump and fingerprint consumption, on `main`); 36ec9322 (C001 provenance v2); b5b3b14e (C001 conditional closure, later closed); 53dea47f414641c3f9756c3f8f181f6208be8115 (M003 implementation); aa21cfe1763d7ea00d11e582ed4108233e4088a9 (M003 closure) | M001-M003 and C001 closed on both sides. Post-closure review found dirty-digest representation mismatch for bound exact-subject evidence; Eggplan's bounded repository-ID-free fingerprint contract landed at `0dd33b7` and CodeGG consumes it at both capture sites on the same pin. CodeGG integration is terminal. |
| Eggwork | faaa0b905fa6bc43e46825fdd98530b5533a970f | protocol-neutral execution snapshots/results/generation/artifact records; executor/scheduler remain outside Eggplan |
| Eggsearch | dfa90e050c5434f3346902aeb4074901c58e90d1 | deterministic EvidenceBundle source/provider/trust/gap metadata; full runtime must not become an Eggplan dependency |
| Eggbench | d870512a5a1af16276ff05286ff0b6e2366b7f8f | current .eggb manifest v2; execution status remains separate from comparison verdict |
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
21. Repository Plan binding must prove CodeGG workspace identity and Eggplan
    `epr_*` identity refer to the same Git subject before translation; blind
    SubjectRevision repository-id relabeling is forbidden.
22. For a bound plan, Eggplan repository state is canonical for plan/item
    lifecycle, evidence, and closure. CodeGG may keep a durable runtime mirror
    only with explicit reconciliation semantics; cross-store atomicity must
    never be claimed.
23. Bound dirty CodeGG evidence must carry the exact Eggplan-compatible dirty
    digest captured at execution time. CodeGG-native dirty digest bytes are
    not interchangeable with Eggplan SubjectRevision dirty digests, and
    current-worktree backfill for historical attempts is forbidden. The
    Eggplan-compatible value is obtained from
    `eggplan_repo::capture_git_subject_fingerprint` at the exact Eggplan
    revision the host pins; those digest bytes are frozen by
    `crates/eggplan-repo/tests/git_subject_digest_golden.rs`, so a host that
    derives a different digest fails loudly rather than silently.

## Planning hygiene

- Register before handoff.
- Preserve historical closure; use corrective plans.
- Record exact evidence and unrun/blocked checks.
- Do not copy Eggwork/Eggsearch/Eggbench responsibilities into eggplan-core.
- Keep core synchronous/deterministic where practical; async/network belongs in adapters.
- No hidden model reasoning in persisted schemas.
- Mark maintenance gates explicitly: non-blocking hygiene must not serialize
  otherwise independent capability handoffs.
