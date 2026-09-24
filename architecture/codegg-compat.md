# CodeGG WorkPlan compatibility

`eggplan-codegg-compat` is a pure one-way compatibility and assessment bridge.
It accepts a strict bounded live snapshot DTO and emits an Eggplan Plan, a
mapping manifest, host-resolved observations, and deterministic assessment.
Fixture parsing is a separate qualification wrapper; fixtures validate their
recorded repository/SHA, while live snapshots do not use fixture provenance as
runtime authority. The production crate depends on `eggplan-core`, serde, and
serde_json only. Repository persistence remains outside this bridge.

The reviewed fixture baseline is CodeGG
`a3c87fc18ee55aaf630401a562c11bb83112fd82`; current `origin/main`
`5f4532659dbf0df2cd9f2b3bdb024217d2ea7868` was rechecked and the WorkPlan
model/assessment/storage interfaces are unchanged in that range. WorkPlan
model/statuses, bounded projections, CAS storage, and runtime ownership remain
CodeGG-owned. Later checkpoint, context-epoch, Todo, Goal, WorkOrder,
AgentRun/Job, and arbiter control flow remain CodeGG-owned.

`normalize_snapshot` is the live mapper; `normalize_fixture` first validates
fixture provenance and then calls the same mapper. `assess_codegg_snapshot`
is pure over the snapshot, exact host-supplied SubjectRevision, resolved
observations, and explicit ProviderRegistry. It writes no Eggplan repository
state. CodeGG derives verification digests from canonical native execution
specifications through Eggplan's shared `verification_digest` helper. If a
native execution specification or exact evidence subject cannot be
reconstructed, the host must mark that evidence unavailable and retain its
compatibility fallback; reference IDs, prose, and serialized `Satisfied`
dispositions never supply a digest.

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

Run `scripts/check-codegg-compat-boundary.sh` to verify the crate has no
CodeGG dependency or declarations for CodeGG-only runtime ownership. Existing
Eggplan core/repository libraries remain the only persistence and assessment
authority.
