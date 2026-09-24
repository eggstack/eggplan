# Eggplan Long-Term Roadmap

Status: active planning baseline

This roadmap sequences product capability. It does not imply implementation.

## 1. Sequencing principles

1. Stabilize typed domain and schema before persistence adapters.
2. Stabilize repository persistence and revision semantics before rich evidence.
3. Stabilize evidence/assessment before CLI automation and external adapters.
4. Prove CodeGG semantic parity before migrating CodeGG ownership.
5. Reuse Eggstack execution/search/evidence machinery through adapters rather
   than importing those responsibilities into eggplan-core.
6. Add attestation/service surfaces only after the local repository contract is
   qualified.

## 2. Dependency map

    Foundation core
      M001 workspace + typed domain/schema
        |
        v
      M002 repository store + CAS + Git subject
        |
        v
      M003 subject/schema/platform hardening
        |
        v
    Evidence and closure
      M001 evidence ledger + deterministic assessment
        |
        v
      M001 C001 verification-binding corrective
        |
        v
      M002 closure records + evidence integrity/recovery
        |
        v
      M002 C001 finalization subject revalidation
        |
        v
      M002 C002 test-seam containment + closure evidence reconciliation
        |
        +--> M002 C003 closure-reference/guard hygiene (non-blocking)
        |
        +----------------------+----------------------+
        |                      |                      |
        v                      v                      v
    Projection/CLI       CodeGG integration     Eggstack integrations
      M001 CLI/status      M001 golden parity     M001 provider SPI
      M002 Markdown        M002 staged adoption   M002 Eggwork/Eggsearch
      import/render                               M003 Eggbench/CI/forge
        |                      |                      |
        +----------------------+----------------------+
                               |
                               v
          Interoperability/distribution
            M001 attestation export/verify
            M002 packaging/update
            M003 optional MCP/service
                  |
                  v
            v0.1 qualification

## 3. Foundation core

### M001 — Repository bootstrap and typed domain contract

Class: infrastructure / invariant

Create the Rust workspace, eggplan-core, schema-v1 types, bounds, stable IDs,
state transitions, dependency validation, canonical serialization/digests, and
tests. No filesystem persistence and no command execution.

Exit: core domain can construct/validate/serialize bounded plans and reject
invalid state/graphs deterministically.

### M002 — Repository store, CAS, and subject identity

Class: capability / invariant

Add eggplan-repo with .eggplan layout, atomic writes, revision/CAS, safe path
handling, cooperative locking, reopen/recovery, and Git SubjectRevision
capture including dirty-state fingerprint.

Exit: two processes/actors cannot silently overwrite revisions, interrupted
writes do not become canonical state, and subject revisions are stable enough
for evidence staleness.

### M003 — Subject scope, strict schema, and platform hardening

Class: invariant / corrective hardening / qualification

Correct post-closure foundation findings before downstream consumers freeze the
contracts: exclude Eggplan-managed state from the source dirty-subject
fingerprint, make schema-v1 nested decoding fail closed, qualify native
Linux/macOS/Windows repository behavior, and repair directly related
documentation drift.

Exit: Eggplan state cannot stale its own source subject at the same Git HEAD,
unknown v1 fields are rejected without changing valid v1 canonical bytes, and
supported-platform evidence is recorded truthfully.

## 4. Evidence and closure

### M001 — Evidence ledger and deterministic assessment

Class: capability / invariant

Implement immutable EvidenceObservation storage/domain, provider identity,
status normalization, criterion/evidence matching, subject-policy enforcement,
and pure assessment.

Exit: passing, failed, not-run, skipped, unavailable, inconclusive, in-flight,
stale, and judgment-only cases are distinguishable and tested.

### M001 C001 — Verification binding corrective

Class: invariant / corrective compatibility hardening

Bind execution-derived requirements and observations to an explicit
verification-spec digest. Preserve v1 state through an explicit compatibility
reader, fail closed for legacy unbound execution requirements, and add an
end-to-end subject-capture -> evidence-persist -> subject-recapture ->
assessment regression.

Exit: unrelated same-kind observations cannot satisfy a specific verification
requirement, legacy state gains no invented proof, and persisted Eggplan
evidence does not self-stale its source subject.

### M002 — Closure records and evidence integrity/recovery

Class: capability

After M001 C001 closure, implement closure candidates/records, exact
requirement-to-evidence matrices, observation digests,
supersession/correction semantics, reopen verification, and fail-closed
handling of corrupt/dangling observations.

Exit: a Plan cannot close without evidence permitted by its criteria, and
historical evidence is not rewritten.

### M002 C001 — Finalization subject revalidation

Class: invariant / corrective hardening

Move authoritative Git SubjectRevision recapture into RepositoryStore guarded
finalization, replay assessment against the internally captured subject, and
recapture immediately before the first closure write. Caller-owned subject
snapshots are no longer finalization authority. Preserve existing persisted
ClosureRecord compatibility and crash recovery.

Exit: a candidate that becomes stale before finalization, or whose subject
drifts during finalizer revalidation, cannot produce pending/final closure
state or a Closed Plan.

### M002 C002 — Finalization test-seam containment and closure evidence reconciliation

Class: invariant / corrective hardening / evidence hygiene

Contain C001's deterministic subject-capture injection seam so it is private
or test-only and cannot be called by downstream crates. Add compile/public-API
regressions proving no external alternate finalizer can inject closure subject
authority. Reconcile the historical C001 closure record with the actual hosted
CI run/job IDs after that workflow completed.

Exit: the only supported external repository closure path owns both subject
captures internally, accidental authority-injection symbols are not public,
and closure records contain actual rather than placeholder hosted evidence.

### M002 C003 — Closure reference and authority-guard hygiene

Class: non-blocking maintenance / evidence hygiene

Correct the C002 implementation-SHA citation, strengthen the static
closure-authority source guard to catch ordinary direct/grouped Rust re-export
shapes, add deterministic negative-proof coverage, and clean stale registry
wording. C002's compile-fail API boundary remains the primary authority
evidence.

Exit: active C002 references point at the actual implementation commit, the
defense-in-depth guard matches its documented scope, and no capability
milestone is blocked by the maintenance pass.

## 5. Projection and CLI

### M001 — CLI control surface and derived registry

Class: capability

Add eggplan-cli with init/new/show/status/ready/graph/check/assess/evidence
commands and JSON output. Generate readiness/registry data from canonical
state; detect inconsistent lifecycle/dependency/evidence references.

### M002 — Loss-aware Markdown import and deterministic render

Class: capability / compatibility

Render versioned Eggplan-native Markdown and import it back as plan intent;
also import a strict documented subset of CodeGG-style implementation-plan
Markdown with deterministic loss reporting. Imported lifecycle is provenance
only. Markdown cannot create EvidenceObservation, provider authority,
SubjectRevision authority, or ClosureRecord state.

## 6. CodeGG integration

### M001 — Golden parity and adapter seam

Class: invariant / integration

Port representative codegg-core::work_plan golden cases against eggplan-core:
bounds, graph actionability, CAS semantics, evidence states, completion
assessment, and bounded projection. Define adapters without changing CodeGG
runtime ownership.

### M002 — Staged Eggplan assessment adoption

Class: integration

Allow CodeGG to consume eggplan-core plus the pure CodeGG compatibility bridge
for generic plan/evidence assessment while retaining SQLite WorkPlan storage,
WorkOrder scheduling, Goal/Todo/checkpoint/context-epoch behavior, and
agent-loop policy in CodeGG-owned adapters. Production CodeGG must not depend
on eggplan-repo for this milestone.

Execution-derived proof must carry a verification digest derived from the
authoritative native execution specification; reference IDs and prose are not
verification identity. No circular repository dependency is permitted.

## 7. Eggstack integrations

### M001 — Evidence provider SPI

Define stable adapter traits and normalized provider result contracts with
bounded async/stream-independent core semantics.

### M002 — Eggwork and Eggsearch evidence adapters

Normalize real Eggwork execution snapshots/artifacts and Eggsearch evidence
bundles through the M001 provider SPI. Acquisition remains host-owned:
eggplan-integrations adds no executor, network client, MCP client, scheduler,
or search runtime. Eggsearch source/content trust remains provenance and cannot
self-enroll Eggplan provider authority.

### M003 — Eggbench, CI, and forge adapters

Reference/verify Eggbench .eggb evidence; consume CI and forge results; support
artifact handles/digests without embedding unbounded payloads.

## 8. Interoperability and distribution

### M001 — Attestation interoperability

Support optional in-toto/SLSA/GitHub-Sigstore style export/verification for
selected evidence. Attestation validity remains separate from criterion
satisfaction policy.

### M002 — Distribution and self-update

Publish library/CLI artifacts using Eggstack's current distribution conventions
and adopt Eggup when its consumer interface is appropriate.

### M003 — Optional MCP/service adapters

Expose bounded machine operations to agents/other hosts without moving network
or daemon authority into eggplan-core.

## 9. Release qualification

Before v0.1, perform:

- schema roundtrip/backward-compat tests;
- property/fuzz tests for bounds/graphs/paths;
- contention/CAS and interrupted-write tests;
- Git clean/dirty subject tests;
- evidence status/staleness/forgery tests;
- closure determinism and corrupt-evidence tests;
- cross-platform supported-subset CI;
- CodeGG golden-parity and staged-adoption tests;
- adapter failure/trust preservation tests;
- CLI JSON/projection consistency tests.

## 10. Deferred work

Deferred until justified by real consumers:

- general workflow branching/loops;
- cross-repository distributed transactions;
- global plan server/database;
- automatic agent/model selection;
- general scheduler/queue;
- plan marketplace/templates;
- arbitrary policy DSL;
- cryptographic signing authority owned by Eggplan.
