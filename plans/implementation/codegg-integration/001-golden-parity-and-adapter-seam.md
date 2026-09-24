# CodeGG Integration M001 — Golden Parity and Adapter Seam

Status: active

Eggplan repository baseline: 62fb29d65ed76d9cbbf6a354a96d393a0303f37f

Initial reviewed CodeGG baseline: 6e18304546f29426457eb11410957288384fdec2

Execution-time CodeGG baseline, re-checked 2026-09-24:
28b4695661d463dd1675d045ac6299c5fbc9ea31

Source roadmap:

- plans/subsystems/codegg-integration-roadmap.md

Long-term requirements:

- plans/000-long-term-specification.md section 16
- plans/001-terminology-and-domain-model.md section 8
- plans/002-long-term-roadmap.md CodeGG M001

Applicable ADR:

- ADR-0004-codegg-extraction-without-workorder-conflation

Primary class: integration / invariant / compatibility

## 1. Objective

Prove, with a versioned fixture corpus and explicit mapping contract, which
CodeGG `codegg-core::work_plan` semantics Eggplan can replace and which
semantics must remain CodeGG adapters.

M001 does not migrate production CodeGG ownership. It establishes a
non-circular compatibility seam that later CodeGG work can consume.

## 2. Current CodeGG interface re-check

Reviewed at CodeGG
`6e18304546f29426457eb11410957288384fdec2`.

Reusable WorkPlan seam remains:

- `crates/codegg-core/src/work_plan/model.rs`;
- `evidence.rs`;
- `assessment.rs`;
- `projection.rs`;
- `store.rs`;
- WorkPlan foundation/projection/trajectory fixtures.

CodeGG ownership that MUST remain outside Eggplan:

- project WorkOrder/occurrence/release coordination;
- Session/project SQLite authority;
- Goal and GoalVerification;
- Todo projection/feedback;
- continuation checkpoint and context epoch;
- agent-loop continuation/final-answer policy;
- AgentRun/Job/worktree scheduling and execution;
- permissions/sandbox/runtime policy.

No production CodeGG dependency is permitted in eggplan-core or eggplan-repo.

## 3. Compatibility architecture

Create a separate compatibility/testing crate, suggested name
`eggplan-codegg-compat`, or an equivalently isolated crate.

It may depend on Eggplan libraries but MUST NOT add a Git/path dependency on
CodeGG. Instead, consume strict versioned compatibility DTOs and fixture JSON
exported from the reviewed CodeGG contract.

Record the reviewed CodeGG SHA in fixture metadata.

The crate exists to:

- parse bounded CodeGG WorkPlan snapshots/fixture DTOs;
- map them to Eggplan domain intent/provenance;
- normalize host evidence only through an explicit resolver callback;
- compare expected actionability/assessment/projection outcomes;
- document non-isomorphic semantics.

It is not the future production owner of CodeGG storage or runtime policy.

## 4. Identity mapping

CodeGG `wp_`/`wi_` IDs are not Eggplan `ep_`/`epi_` IDs.

Define a deterministic bounded compatibility mapping and preserve the original
CodeGG IDs in explicit provenance. Do not cast strings across typed ID classes.

The mapping must be stable for fixture replay and collision-tested. It must not
claim that an Eggplan PlanId is the canonical CodeGG WorkPlan identity.

Future live adoption may persist an explicit ID mapping in CodeGG; M001 does
not prescribe CodeGG database migration.

## 5. Status and transition differences

Document and test the non-isomorphic transition matrices.

Current CodeGG allows transitions such as direct Pending -> InProgress and
Actionable -> Completed that Eggplan ordinary lifecycle rules may stage
differently. Snapshot conversion must map current state without replaying
historical transitions.

Do not weaken Eggplan transition invariants merely to make a CodeGG transition
history replay byte-for-byte.

M001 must produce a compatibility table identifying:

- exact semantic matches;
- adapter-normalized matches;
- CodeGG-owned behavior;
- unsupported/lossy fields.

## 6. Evidence mapping and authority

CodeGG serialized WorkAcceptance/WorkEvidenceRef fields are intent/references;
they do not become trusted Eggplan passing observations by deserialization.

Required mapping guidance:

- TestJob -> Eggplan Test;
- DelegatedRun -> DelegatedRun;
- SchedulerJob -> execution-derived Command or DelegatedRun according to the
  authoritative CodeGG job type;
- AgentRun -> DelegatedRun/provenance according to authoritative host state;
- Artifact -> Artifact;
- Commit -> Revision.

A CodeGG `Satisfied` acceptance value alone MUST NOT manufacture a passing
Eggplan observation.

The compatibility layer must require a host evidence resolver to supply:

- authoritative native ref resolution;
- normalized status;
- exact SubjectRevision;
- trusted provider identity;
- verification-spec digest for execution-derived evidence;
- artifact refs/digests where available.

CodeGG owner_run_id/owner_job_id remain provenance only.

`RequiresUserJudgment` maps to awaiting-human policy unless an authorized
human judgment observation is separately present.

If the CodeGG host cannot derive a verification digest from the authoritative
job/run specification, the corresponding execution evidence cannot satisfy a
bound Eggplan v2 requirement.

## 7. Assessment compatibility

Freeze a mapping between CodeGG's current five completion families and
Eggplan's richer assessment vocabulary without discarding Eggplan detail.

The compatibility projection may map multiple Eggplan failure/missing/stale
states into CodeGG's existing actionable/blocked surfaces for display, but the
underlying Eggplan assessment and reason codes remain available.

Golden tests must cover at least:

- all items complete with trusted passing evidence;
- completed label without host evidence;
- failed evidence;
- unavailable/dangling evidence;
- in-progress evidence;
- user judgment;
- owner provenance without satisfaction;
- blocked plan/item;
- dependency-gated actionability;
- stale/untrusted/mismatched verification evidence;
- cancellation;
- empty/no-criteria invalid completion.

## 8. CAS and storage parity

CodeGG has per-item revisions plus an owning plan revision bump in SQLite;
Eggplan stores items inside a revisioned Plan.

M001 parity is semantic, not physical-schema equivalence:

- stale writers must fail;
- a successful item mutation must advance the relevant Eggplan Plan revision;
- concurrent completion/cancellation cannot silently both win;
- restart must preserve current state;
- closure must route through Eggplan's guarded closure record, not CodeGG's
  serialized `Completed` label.

Do not import CodeGG's SQLite schema into eggplan-repo.

## 9. Projection parity

Use bounded compatibility projections to freeze the CodeGG-observable subset:

- stable current/actionable ordering;
- bounded item count/text;
- explicit truncation/counts;
- current item and blocker/next-action summaries;
- assessment reason code.

Do not import TodoState or continuation checkpoint semantics into Eggplan.

If Projection/CLI M001 has landed by execution time, reuse its generic bounded
projection DTOs. If it has not, keep any CodeGG projection helper isolated in
the compatibility crate so it can be replaced without changing core schemas.

## 10. Fixture corpus

Create versioned fixtures sourced from the reviewed CodeGG baseline, with
metadata identifying repository SHA and source test/architecture case.

At minimum include representative cases from:

- work_plan_foundation;
- work_plan_projection_arbiter;
- long_horizon_trajectory_qualification.

The corpus should cover actionability, evidence pass/fail/unavailable,
in-flight work, user judgment, stale writer races, cancellation, bounded
projection, restart, missing artifact degradation, and no-hidden-reasoning
guards.

Do not copy secrets, local paths, or arbitrary transcript/model content.

## 11. No-ownership guard

Add a static/documentation guard proving the compatibility crate does not
define or own:

- WorkOrder scheduling;
- Goal budgets/verifier;
- TodoState;
- continuation/context epochs;
- AgentRun/Job executor;
- worktree/sandbox policy.

The adapter seam must remain one-way compatibility machinery.

## 12. Ordered work packages

### WP1 — Baseline/fixture export

Freeze CodeGG SHA, compatibility DTO schema, provenance manifest, and
representative golden cases.

### WP2 — Domain/status/identity mapping

Implement strict DTO parsing and deterministic mapping with explicit
loss/non-isomorphism diagnostics.

### WP3 — Evidence resolver seam

Define host callback interfaces and tests proving serialized CodeGG claims
cannot self-authorize Eggplan evidence.

### WP4 — Assessment/CAS/projection parity

Run fixture cases through Eggplan and freeze expected mappings/reason codes.

### WP5 — Documentation and qualification

Add `architecture/codegg-compat.md`, native tests, and roadmap/registry
closure updates. Make no production CodeGG change.

## 13. Required tests

At minimum:

- deterministic ID mapping and collision bounds;
- strict fixture schema/version rejection;
- actionability ordering parity;
- status/transition compatibility table tests;
- completed-without-evidence stays incomplete;
- host resolver required for passing evidence;
- owner provenance alone never passes;
- execution evidence without verification digest does not pass;
- stale/untrusted/mismatched evidence does not pass;
- user judgment stays explicit;
- stale CAS conflict;
- cancel-vs-close race outcome;
- restart fixture equivalence;
- bounded projection/truncation;
- no WorkOrder/Goal/Todo/checkpoint/executor ownership in compatibility crate;
- native CI and Rust 1.89.

## 14. Required verification

Run Eggplan workspace gates plus focused compatibility tests. Closure must
record the exact CodeGG SHA used to regenerate/verify fixtures.

## 15. Acceptance criteria

M001 closes when representative CodeGG WorkPlan behavior can be expressed and
verified through Eggplan without trusting serialized host claims, weakening
Eggplan v2 evidence semantics, importing CodeGG scheduler/runtime ownership, or
creating a circular dependency.

## 16. Stop conditions

Stop and report if:

- current CodeGG interfaces materially differ from the reviewed SHA;
- parity requires changing WorkOrder/Goal/Todo/checkpoint behavior;
- passing evidence can only be reproduced by trusting WorkAcceptance text;
- a CodeGG dependency must enter eggplan-core/repo;
- transition differences require weakening Eggplan rather than documenting an
  adapter normalization.

## 17. Closure evidence required

Record:

- current CodeGG SHA and fixture manifest;
- exact/normalized/lossy compatibility matrix;
- evidence resolver authority tests;
- parity scenario results;
- CAS/restart/race results;
- projection bounds;
- ownership static guard;
- cross-platform CI.
