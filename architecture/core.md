# Core architecture

`eggplan-core` owns deterministic domain data and pure validation. It has no
filesystem, Git, process, network, database, scheduler, executor, or model
runtime responsibilities. Repository persistence belongs to `eggplan-repo`;
external observations belong to provider adapters.

## Plan schema v1 and v2

The first schema defines typed `PlanId` (`ep_`), `PlanItemId` (`epi_`),
`CriterionId` (`epc_`), and `EvidenceProviderId` (`epp_`) values; revisioned
Plans and PlanItems; criteria and evidence requirements; artifacts; and
transport-neutral SubjectRevision. IDs are prefix validated, at most 96 ASCII
characters, and use ASCII letters, digits, `_`, or `-` after the prefix.

Canonical JSON is compact `serde_json` serialization of the versioned schema
structs, whose declared field order is fixed. `BTreeMap` values serialize in key order.
Optional `None` values are omitted where annotated. Digests are lowercase
`sha256:<64 hex>` over those exact bytes. Golden files in
`crates/eggplan-core/tests/fixtures/` freeze the initial Plan byte sequence and
digest; pretty JSON is not part of that contract.

Schema v1 is frozen. Newly constructed plans use schema v2, which adds an
optional expected verification-spec digest to requirements and requires it for
execution-derived evidence kinds. See [evidence architecture](evidence.md).
Both versions use the same strict semantic structs and reject unknown fields during deserialization,
including nested criteria, requirements, subjects, and artifact references.
Valid v1 serialization remains byte-for-byte stable; unknown fields are not a
supported extension mechanism.

## Bounds

Text bounds count Unicode scalar values and reject empty strings and NUL:

| Field | Limit |
|---|---:|
| Plan objective | 4,000 |
| Item description | 4,000 |
| Blocker / next action | 2,000 each |
| Provenance value | 2,000 |
| Criterion statement | 2,000 |
| Requirement description | 1,000 |
| Artifact reference | 2,000 |
| Plan items | 512 |
| Dependencies per item | 64 |
| Criteria per item | 128 |
| Requirements per criterion | 32 |
| Provenance entries | 32 |

The domain supports explicit Draft/Active/Blocked/Closed/Cancelled plan
transitions and Pending/Actionable/InProgress/Blocked/Completed/Cancelled
item transitions. Closed and completed states are terminal. Readiness is a
stable position-then-ID ordering and is only a derived statement about
dependencies; it grants no execution authority.

## Evidence schema v1

Evidence observations have typed `epe_` IDs, provider IDs, a closed evidence
kind/status vocabulary, exact SubjectRevision, Unix-millisecond observation
time, bounded optional invocation/result metadata, and bounded artifact refs.
An observation is finalized by hashing compact canonical JSON for its content
fields; the `content_digest` field itself is excluded from that digest.
`evidence-v1-digests.json` freezes status and evidence-kind digest fixtures.

Finalized observation fields are private and have read-only accessors. A
provider ID in an observation is not authority: pure assessment receives an
explicit host-constructed `ProviderRegistry`. Only registered descriptors may
contribute, and each descriptor constrains allowed evidence kinds. This is an
adapter/host trust boundary, not cryptographic authentication; a digest proves
recorded content integrity only.

Assessment is a pure function of Plan, current subject, observations, and the
trusted-provider registry. Exact subject equality is required. It explains
missing, stale, failed, in-progress, blocked, unavailable/not-run/skipped,
inconclusive, invalid attribution, and human-judgment cases with closed reason
codes. Fixed precedence is invalid/stale, failed, blocked, in-flight,
missing/unavailable, awaiting human judgment, inconclusive, actionable work,
then complete. Completed item labels without acceptance criteria and passing
evidence remain incomplete.

## Closure schema v1

Closure IDs (`epcl_`) and evidence-supersession IDs (`eps_`) are distinct typed
identities. Supersession records are append-only digest-protected links;
assessment uses only terminal observations in the validated lineage. A pure
ClosureCandidate snapshots the exact active Plan revision, subject, complete
assessment, satisfying observation digests, supersession digests, and sorted
provider-policy authority facts. A ClosureRecord binds that candidate to the
next Closed Plan revision and its canonical digest. Stored policy is historical
evidence and does not auto-trust providers in later assessments.

The repository alone finalizes closure and owns current-subject authority:
the finalizer recaptures the Git `SubjectRevision` from the configured
`GitSubjectSource` under its lock and again immediately before the first
closure write. The caller may build a candidate, but the finalizer decides
whether the candidate's subject still matches the worktree at the moment of
closure. Reopen verifies the source and final Plan digests, provider-policy
digest, reproducible assessment, exact evidence digests, and supersession
lineage. Ordinary CAS cannot enter Closed. See [repository
recovery](repository.md) for the pending-record crash protocol.

## Verification

Run `scripts/check-core-boundary.sh` and the workspace fmt, check, clippy, and
test commands. Rust 1.89 is the declared MSRV.
