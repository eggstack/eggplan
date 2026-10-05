# CodeGG WorkPlan compatibility

`eggplan-codegg-compat` is a pure two-direction compatibility bridge. Its M002
direction accepts a strict bounded CodeGG snapshot and emits a transient
Eggplan assessment view. Its M003 direction projects a bindable repository
Plan into a bounded CodeGG mirror contract. Neither direction performs
repository I/O or evidence acquisition.
Fixture parsing is a separate qualification wrapper; fixtures validate their
recorded repository/SHA, while live snapshots do not use fixture provenance as
runtime authority. The production crate depends on `eggplan-core`, serde, and
serde_json only. Repository persistence remains outside this bridge.

## Repository Plan binding projection (M003)

`project_repository_plan` converts only an Active or Blocked, validated Plan
into schema version 1 of `RepositoryPlanProjectionV1`. The projection keeps
Eggplan Plan/item/criterion IDs as source identities and preserves item order,
parent/dependency edges, exact item status, descriptions, blockers/next action,
criterion judgment policy, and each complete evidence requirement (kind,
provider constraint, subject policy, cardinality, minimum count, human policy,
and verification digest). Draft and terminal plans fail closed.

`intent_digest` covers objective and ordered structural intent, including all
criteria and requirement semantics. It excludes revision, lifecycle, blocker,
next action, subject, evidence observations, and closure. `projection_digest`
covers the complete emitted projection contract, so lifecycle/progress changes
are visible while the intent digest stays stable. `content_digest` is the
Eggplan canonical digest of the validated Plan. The projection exposes no
observations, trust enrollment, closure data, caller-selected repository
identity, or CodeGG runtime IDs. In particular, this crate has no helper that
relabels a `SubjectRevision.repository_id`; the CodeGG application must prove
workspace and Eggplan subjects match before recording an identity association.

The host persists the runtime-ID mapping and mirror, synchronizes item
lifecycle/evidence through Eggplan repository APIs, and consumes closure only
after `RepositoryStore::finalize_closure` succeeds. Criteria may be displayed
as CodeGG acceptance rows, but those display rows cannot become satisfied from
projection; structured requirement data stays in the binding manifest.

The reviewed fixture baseline is CodeGG
`a3c87fc18ee55aaf630401a562c11bb83112fd82`. Rechecked at the CodeGG
M002-adopted head `ffa1c15e654776c3ebe1022f4ce7de2582bc5d98`:
`crates/codegg-core/src/work_plan/` (model, assessment, evidence, projection,
store, Todo projection, checkpoint, epoch policy) is byte-identical across that
whole range, so the fixture contract still describes the current WorkPlan
model. The M002 adoption is entirely application-layer (`src/work_plan_eggplan.rs`,
`src/work_plan_arbiter.rs`, `src/work_plan_evidence.rs`, `src/tool/work_plan.rs`)
plus the root `Cargo.toml` pin; `codegg-core` carries no Eggplan dependency.
WorkPlan model/statuses, bounded projections, CAS storage, and runtime ownership
remain CodeGG-owned. Later checkpoint, context-epoch, Todo, Goal, WorkOrder,
AgentRun/Job, and arbiter control flow remain CodeGG-owned.

`normalize_snapshot` is the live mapper; `normalize_fixture` first validates
fixture provenance and then calls the same mapper. `assess_codegg_snapshot`
is pure over the snapshot, exact host-supplied SubjectRevision, resolved
observations, and explicit ProviderRegistry. It writes no Eggplan repository
state. Host obligation (not implemented behavior in this crate, which only
compares host-supplied bindings): CodeGG must derive verification digests
from canonical native execution specifications through Eggplan's shared
digest helper. CodeGG implemented that obligation in its M002 application
facade: `CodeggVerificationSpecV1` is digested with `eggplan_core::digest_json`
and formatted by `VerificationDigest`, so the host digest is Eggplan-canonical
by construction rather than by convention. If a native execution
specification or exact evidence subject cannot be reconstructed, the host
must mark that evidence unavailable and retain its compatibility fallback;
reference IDs, prose, and serialized `Satisfied` dispositions never supply a
digest.

## Verification-binding ownership

Ownership of the verification identity is deliberately split, and the split is
enforced rather than documented:

| Concern | Owner | Enforced by |
|---|---|---|
| Which native object a ref names (`TestJob`, `SchedulerJob`, `DelegatedRun`/`AgentRun`) | CodeGG | `CodeggEvidenceRef` kind in the snapshot DTO |
| Deriving the verification digest from the authoritative execution specification | CodeGG host adapter | `VerificationDigest` equality check in `normalize_snapshot`; Eggplan only compares, never mints |
| Requiring a binding for execution-derived kinds | Eggplan | `requires_verification_binding` gate; missing or mismatched binding fails the whole mapping closed |
| Provider identity and class/trust policy | Eggplan `ProviderRegistry`, supplied by the host | `assess_plan`; `register_trusted` never appears in this crate's `src/` |
| Artifact/Commit refs needing no execution binding | Eggplan | non-execution kinds bypass the binding gate and keep `expected_verification: None` |

`Artifact` and `Commit` map to `Artifact`/`Revision` and are deliberately exempt:
they are content/revision references, not executions, so a digest would be
invented identity. The exemption is a kind allowlist, not a host assertion.


## Mapping contract

| CodeGG value | Eggplan mapping | Qualification |
|---|---|---|
| `wp_` and `wi_` IDs | Namespace-separated SHA-256-derived `ep_` / `epi_` IDs | Deterministic mapping manifest retains the original identifiers and rejects collisions in its bounded input set. It does not make Eggplan IDs canonical CodeGG IDs. |
| Pending, Actionable, InProgress, Blocked, Completed, Cancelled item | Same snapshot status | Snapshot conversion does not replay CodeGG transition history. |
| Active / Blocked / Cancelled plan | Draft-at-revision-zero, then legal CAS to the source lifecycle | `Completed` maps to Active and carries `CompletedPlanNeedsGuardedClose`; only Evidence M002 may close it. |
| Acceptance description | Eggplan acceptance criterion | Serialized `Satisfied` remains a claim. It cannot create an observation. An empty evidence set stays incomplete. |
| TestJob / DelegatedRun / SchedulerJob / AgentRun / Artifact / Commit refs | Test / DelegatedRun / Command / DelegatedRun / Artifact / Revision requirements | A host resolver must return an exact-subject normalized observation. Execution-derived refs require the resolver's authoritative verification binding to equal the observation binding. |
| `RequiresUserJudgment` | Awaiting human judgment | Ordinary refs do not satisfy this disposition; the adapter preserves it as unresolved human judgment. |
| owner run/job IDs | Mapping-manifest provenance | They never enter provider authority or satisfying evidence. |
| Evidence status and reason details | Eggplan's full status/reason codes | A five-family compatibility view folds failed/missing/stale/inconclusive states into actionable while retaining the Eggplan assessment. |

CodeGG acceptance rows do not identify one-to-one evidence-reference ownership.
The mapping records this as a lossy diagnostic when multiple acceptance rows
share item-level refs. Free-form evidence details and acceptance notes are
omitted from persisted compatibility metadata and are marked as losses.

## Qualification fixtures

The versioned corpus records source SHA and source test case for foundation,
projection/arbiter, and long-horizon trajectory behavior. It contains only
bounded synthetic IDs and no paths, credentials, transcripts, or model
reasoning. Tests cover pass/unavailable evidence, completed-without-proof,
failed authority, stale subject and verification binding, judgment, blocked
dependencies, current/actionable projection bounds, stale CAS, restart, and
cancel-versus-guarded-close contention.

Provenance is gated, not relabelled. `normalize_fixture` accepts only
`schema_version == 1`, `source_repository == "codegg"`, and
`source_sha == SOURCE_CODEGG_SHA`, so a fixture stamped with the superseded M001
baseline `28b4695661d463dd1675d045ac6299c5fbc9ea31` is rejected outright while
the identical snapshot body still maps through `normalize_snapshot`. The M001
corpus was not kept as a second accepted baseline: it described a different
CodeGG contract, and dual acceptance would let stale bytes masquerade as
current provenance.

The live assessment surface is qualified independently of the corpus. The
bridge tests drive `assess_codegg_snapshot` over synthesized snapshots covering
every evidence kind (Test/Command/DelegatedRun/Artifact/Revision), proving that
execution kinds complete only with a matching authoritative binding, that
Artifact/Commit complete without one, that missing or mismatched bindings fail
the whole assessment closed rather than degrading, and that the five-family fold
never discards Eggplan's detailed reason codes.

Run `scripts/check-codegg-compat-boundary.sh` to verify the crate has no
CodeGG dependency or declarations for CodeGG-only runtime ownership. Existing
Eggplan core/repository libraries remain the only persistence and assessment
authority.

The script enforces five guards and states each one with the scope it scans, so
a failure message never claims more than was checked. One guard is
`[dependencies]`-scoped: production dependencies must not include `eggplan-repo`.
The other two manifest guards deliberately match *any* section, so a
dev- or build-dependency on CodeGG, or on an async/database/network client,
also fails. The section-scoped exception exists because the test suite
legitimately declares `eggplan-repo` and `tempfile` as dev-dependencies to
exercise the real store; the production dependency graph itself must not reach
the persistence layer. The two source guards reject a public declaration of a
CodeGG-owned identity, and any process, filesystem, network, or database access
in the production source. Every guard has deterministic synthetic self-proofs
that run against fixtures in a temporary directory, so a guard that stops
detecting is caught by its own script.
