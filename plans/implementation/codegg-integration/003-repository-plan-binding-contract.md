# CodeGG Integration M003 — Repository Plan Binding Contract

Status: closed — CodeGG consumer landed; post-closure dirty-subject C001 registered separately

Repository baseline:

- Eggplan: `178f72dfc46367944ec059d6b88310f14af8f8a8`
- CodeGG M002 closure consumed: `ffa1c15e654776c3ebe1022f4ce7de2582bc5d98`
- CodeGG M003 status-reconciled baseline:
  `9e93e949e1a4abcc91fd7755d929a54fd6531160`
- CodeGG coordinated M003 plan registration:
  `440403e82304537f6ed9de22103987b16df6f642`
- CodeGG M003 plan:
  `plans/implementation/eggplan-assessment-integration/004-repository-plan-binding-and-writeback.md`

Source roadmap:

- `plans/subsystems/codegg-integration-roadmap.md`

Predecessor closure:

- `plans/closure/codegg-integration/002-closed.md`

Primary class: compatibility contract / repository binding / authority preservation

## 1. Objective

Define the Eggplan side of repository Plan binding so CodeGG can bind a
session or WorkOrder execution to an existing repository-local Eggplan Plan
without reconstructing that Plan through the one-way M002 WorkPlan snapshot
adapter.

M003 adds one pure reverse projection contract:

    Eggplan repository Plan
      -> bounded CodeGG binding projection
      -> CodeGG runtime mirror

The repository Plan remains canonical for:

- objective and item structure;
- dependency graph;
- acceptance criteria and evidence requirements;
- Plan/item lifecycle;
- immutable evidence ledger;
- supersession lineage;
- assessment and guarded closure.

CodeGG remains canonical for:

- session/WorkOrder lifecycle;
- scheduler/jobs/AgentRun/RunStore;
- worktree/workspace leases;
- runtime owner refs;
- Todo/checkpoint/context-epoch state;
- model-facing execution tools.

This milestone MUST NOT turn `eggplan-codegg-compat` into a repository client
or CodeGG scheduler.

## 2. Existing M002 boundary to preserve

M002 intentionally made `eggplan-codegg-compat` a pure one-way CodeGG
snapshot assessment bridge:

- `normalize_snapshot`;
- `assess_codegg_snapshot`;
- no `eggplan-repo` production dependency;
- source CodeGG lifecycle/provenance never becomes repository authority.

M003 adds a second pure direction for binding/projection. It does not remove or
reinterpret the M002 API.

Production repository I/O remains CodeGG application-layer responsibility
through the separately pinned `eggplan-repo` crate.

## 3. Repository Plan projection DTO

Add a versioned bounded DTO family to `eggplan-codegg-compat`, recommended:

    RepositoryPlanProjectionV1
    RepositoryItemProjectionV1
    RepositoryCriterionProjectionV1
    RepositoryRequirementProjectionV1
    RepositoryPlanBindingManifestV1

The exact names may vary.

The projection must include enough structured intent for CodeGG to construct a
runtime mirror without parsing Markdown or flattening evidence semantics:

Plan:

- Eggplan PlanId;
- revision;
- objective;
- Active/Blocked lifecycle;
- current plan content digest;
- structural intent digest;
- ordered items.

Item:

- Eggplan PlanItemId;
- position;
- parent Eggplan item ID;
- dependency Eggplan item IDs;
- exact lifecycle status;
- description;
- blocker/next action;
- criteria.

Criterion:

- criterion ID;
- statement;
- human-judgment flag;
- bounded requirement summaries.

Requirement summary:

- evidence kind;
- provider restriction if present;
- exact-subject policy;
- cardinality;
- min count;
- expected verification digest if present;
- bounded description.

The projection MUST NOT contain:

- persisted EvidenceObservation rows;
- provider trust enrollment;
- ClosureCandidate/ClosureRecord;
- caller-selected repository identity;
- CodeGG Job/Run IDs;
- model-authored completion claims.

## 4. Bindable lifecycle

Live repository binding in M003 supports only:

- PlanStatus::Active;
- PlanStatus::Blocked.

Reject as non-bindable:

- Draft — caller must explicitly activate it first;
- Closed — historical record, not live execution intent;
- Cancelled — terminal history.

Projection helpers may expose diagnostic status for terminal plans, but the
live binding constructor must fail closed rather than re-open them.

## 5. Lifecycle mapping

Plan lifecycle maps exactly:

- Active -> CodeGG Active mirror;
- Blocked -> CodeGG Blocked mirror.

Item lifecycle maps exactly:

- Pending;
- Actionable;
- InProgress;
- Blocked;
- Completed;
- Cancelled.

Do not derive Completed from evidence during projection. Repository lifecycle is
already canonical.

Because Eggplan assessment requires `PlanItemStatus::Completed` in addition
to complete evidence, CodeGG M003 must synchronize bound item lifecycle through
repository CAS; this projection contract must preserve those statuses exactly.

## 6. Acceptance/requirement projection

CodeGG's legacy WorkAcceptance surface is less expressive than Eggplan
criteria. The reverse projection therefore MUST keep criteria/requirements as
structured binding metadata rather than pretending the legacy WorkAcceptance
DTO is lossless.

Recommended CodeGG mirror display mapping:

- one display acceptance row per Eggplan criterion statement;
- RequiresUserJudgment only when the Eggplan criterion explicitly permits
  human judgment;
- otherwise Unmet;
- never project Satisfied merely because repository evidence currently passes;
- never embed verification digests/provider policy in free-form notes.

The full requirement summary remains in the M003 binding projection/manifest
for diagnostics and drift detection.

## 7. Structural intent digest

Add a deterministic `intent_digest` over repository-owned structure:

- objective;
- item IDs/order;
- parent/dependencies;
- item descriptions;
- criteria statements;
- human-judgment flags;
- complete requirement semantics.

Exclude runtime/progress-only fields:

- Plan revision;
- Plan/item lifecycle;
- blocker/next action;
- repository subject;
- observations/closure;
- CodeGG runtime owner refs.

Purpose:

- CodeGG can distinguish repository structural edits from normal lifecycle
  revisions;
- crash reconciliation may safely refresh lifecycle when intent is unchanged;
- structural edits while bound become an explicit binding conflict rather than
  silent mirror mutation.

Also expose a full `projection_digest` covering all projected fields.

Use Eggplan canonical JSON/digest rules.

## 8. Identity mapping contract

The projection uses Eggplan IDs as source identities only.

It must NOT manufacture:

- CodeGG WorkPlanId;
- CodeGG WorkItemId;
- CodeGG WorkspaceId;
- CodeGG RepositoryId.

CodeGG persists the explicit mapping between its runtime IDs and Eggplan IDs.

Do not add a helper that blindly rewrites
`SubjectRevision.repository_id`.

Repository identity translation is authority-sensitive and belongs to CodeGG's
application integration:

1. independently capture Eggplan repository subject from RepositoryStore;
2. independently capture CodeGG subject for the same canonical workspace;
3. verify kind/revision/state/dirty digest equality;
4. persist the proven CodeGG workspace/repository <-> Eggplan `epr_*`
   association;
5. only then translate historical attempt subjects for evidence writeback.

A generic "replace repository_id" helper is explicitly out of scope.

## 9. Repository APIs

Current `eggplan-repo` public APIs are sufficient for the planned CodeGG
consumer:

- `RepositoryStore::open`;
- `repository_id`;
- `subject_source().capture()`;
- `PlanStore::get/list/create/compare_and_swap`;
- `append_observation`;
- `list_observations`;
- `list_supersessions`;
- `closure_record`;
- `RepositoryStore::finalize_closure`.

Do not expose crate-private subject-capture/finalizer seams for M003.

If implementation discovers a missing repository operation, stop and add the
smallest intent-named public API with separate boundary tests. Do not weaken
closure authority.

## 10. Evidence/writeback contract

The Eggplan-side contract remains normal schema-v2 EvidenceObservation.

M003 does not add a CodeGG-special evidence schema.

CodeGG writes terminal host evidence with:

- fixed host-owned provider identity;
- exact translated Eggplan repository subject;
- authoritative verification digest for execution evidence;
- deterministic observation ID;
- native terminal timestamp;
- bounded metadata/artifact refs.

Repository append idempotence remains the retry/restart primitive.

Ephemeral InProgress observations should not be persisted under an ID later
reused for terminal state. The coordinated CodeGG plan therefore persists only
terminal durable observations in M003.

Artifact evidence may use normal Eggplan ArtifactRef/EvidenceKind::Artifact
only when CodeGG resolves a durable RunStore artifact record, digest, producing
run, and exact execution subject. Commit/Revision authority remains deferred.

## 11. Closure contract

Bound completion MUST use ordinary Eggplan closure machinery:

1. repository lifecycle/evidence synchronized;
2. effective observations + explicit provider policy assessed;
3. `ClosureCandidate::build`;
4. `RepositoryStore::finalize_closure`;
5. only after guarded repository closure succeeds may CodeGG terminalize its
   runtime mirror.

No CodeGG API may construct a Closed repository Plan via ordinary CAS.

## 12. New compatibility functions

Recommended public pure API:

    project_repository_plan(plan: &Plan)
        -> Result<RepositoryPlanProjectionV1, RepositoryProjectionError>

    repository_projection_intent_digest(...)
        // or field on returned projection

No filesystem, Git, SQLite, network, async runtime, or CodeGG crate
dependency enters `eggplan-codegg-compat`.

## 13. Required tests

At minimum:

- Active Plan projection;
- Blocked Plan projection;
- Draft/Closed/Cancelled live-bind rejection;
- exact item lifecycle mapping;
- parent/dependency mapping;
- rich criterion/requirement preservation;
- provider restriction/cardinality/min-count/verification digest preservation;
- human judgment projection;
- no Satisfied fabrication;
- deterministic intent digest;
- lifecycle-only change leaves intent digest unchanged;
- blocker/next-action change leaves intent digest unchanged;
- structural criterion/dependency/objective change changes intent digest;
- full projection digest changes on lifecycle;
- no observation/provider trust/closure rows enter projection;
- IDs remain source identities rather than CodeGG IDs;
- M001/M002 compat fixtures and assessment bridge remain unchanged.

## 14. Boundary guards

Extend `check-codegg-compat-boundary.sh` to prove the compatibility crate
still has no production dependency on:

- eggplan-repo;
- tokio;
- sqlx;
- CodeGG crates;
- process/network clients.

The CodeGG application may consume eggplan-repo; this pure compatibility crate
must not.

## 15. Documentation

Update:

- `architecture/codegg-compat.md`;
- CodeGG compatibility deep dive;
- `plans/subsystems/codegg-integration-roadmap.md`;
- `plans/registry.md`.

Document the two directions distinctly:

- M002 CodeGG snapshot -> transient Eggplan assessment;
- M003 repository Plan -> CodeGG binding projection.

## 16. Verification

At minimum:

    cargo fmt --all -- --check
    cargo check --workspace --all-targets --locked
    cargo clippy --workspace --all-targets --locked -- -D warnings
    cargo test --workspace --locked
    cargo +1.89.0 check --workspace --all-targets --locked
    cargo +1.89.0 test --workspace --locked
    bash scripts/check-codegg-compat-boundary.sh
    bash scripts/check-core-boundary.sh
    bash scripts/check-closure-authority-boundary.sh
    git diff --check

Hosted Linux/macOS/Windows and Rust 1.89 qualification is required.

## 17. Acceptance criteria

Eggplan M003 closes when:

1. a live repository Plan has a deterministic bounded CodeGG binding
   projection;
2. all repository acceptance/evidence requirement semantics needed for drift
   detection survive structurally;
3. no repository evidence/trust/closure authority is fabricated by projection;
4. lifecycle mapping is exact;
5. intent/projection digests have the specified stability/sensitivity;
6. no subject repository-ID relabel helper is exposed;
7. `eggplan-codegg-compat` stays pure and repository-free;
8. CodeGG consumes the contract from one exact immutable Eggplan revision;
9. cross-repository tests prove binding/writeback/closure without scheduler
   ownership transfer;
10. native/MSRV hosted qualification passes.

## 18. Stop conditions

Stop and report if:

- reverse projection requires Markdown parsing;
- repository identity would need blind SubjectRevision relabeling;
- CodeGG IDs must become Eggplan persisted IDs;
- compatibility crate needs SQLite/Git/repository ownership;
- M003 requires weakening guarded closure;
- criteria must be flattened in a way that loses authoritative verification
  semantics.

## 19. Closure evidence

Record:

- Eggplan implementation SHA;
- final reverse-projection schema/version;
- intent-digest matrix;
- compatibility boundary dependency graph;
- CodeGG consuming pin/SHA;
- cross-repo binding and closure test references;
- hosted native/MSRV runs;
- residual limitations, especially deferred Commit authority.
