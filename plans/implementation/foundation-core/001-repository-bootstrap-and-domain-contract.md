# Foundation Core M001 — Repository Bootstrap and Typed Domain Contract

Status: closed

Repository baseline: ce2d6e7a6c1cecbe7ff554daf9d603d2d604d98c plus the planning bootstrap commit that registers this plan.

Source roadmap:

- plans/subsystems/foundation-core-roadmap.md

Long-term requirements:

- plans/000-long-term-specification.md sections 1-9, 18-21
- plans/001-terminology-and-domain-model.md
- plans/002-long-term-roadmap.md Foundation M001

Applicable ADRs:

- ADR-0001-planning-evidence-mechanism-not-execution
- ADR-0002-repository-first-versioned-canonical-state
- ADR-0003-immutable-revision-scoped-evidence

Primary class: infrastructure / invariant

## 1. Objective

Create the initial Rust workspace and dependency-light eggplan-core crate with
a versioned, bounded, deterministic domain contract for Plans, PlanItems,
acceptance criteria, evidence requirements, subject identities, state
transitions, dependency readiness, and canonical serialization/digests.

This milestone creates no repository persistence, command execution, network
service, CLI product workflow, or external adapters.

## 2. Why this milestone is ready

The repository is empty of production code, so there is no migration or
compatibility code to preserve. The long-term ownership boundary and domain
terminology are accepted. No external Eggstack API is required for M001.

CodeGG baseline 2f7d84f88070eee2a4fb9f70b6d7d5d10b01048d provides useful semantic
reference for WorkPlan bounds, actionability, validation, evidence separation,
and completion discipline, but M001 must not add a CodeGG dependency.

## 3. Current implementation evidence

At the planning baseline:

- no Cargo workspace exists;
- no Rust source exists;
- no persisted schema exists;
- README and plans define intended ownership only.

Do not claim migration of existing runtime state; there is none.

## 4. Invariants that must not regress

- eggplan-core has no scheduler, process runner, HTTP, MCP, database, or model SDK dependency.
- No persisted field stores hidden reasoning/thinking/scratchpad.
- Durable identities are typed, bounded, and prefix-validated.
- Plan revision is monotonic and non-negative.
- Items may depend only on items in the same Plan.
- Self-dependencies and cycles are rejected deterministically.
- Readiness is pure derived state and grants no execution authority.
- Planned evidence requirements are distinct from observed evidence.
- Bounds are core validation rules, not presentation-only limits.
- Canonical digest input is deterministic and covered by golden fixtures.
- Unknown schema versions fail explicitly.

## 5. Scope

### In scope

- Cargo workspace at MSRV Rust 1.89.
- crates/eggplan-core.
- Core IDs and error taxonomy.
- Schema version constants.
- Plan, PlanItem, lifecycle/status, criterion, evidence-requirement,
  provider-ID placeholder, ArtifactRef, and SubjectRevision core types.
- NewPlan/NewPlanItem or equivalent validated construction inputs.
- Explicit field/count/text bounds.
- Plan/item transition functions.
- Dependency and parent graph validation.
- Pure deterministic actionable/ready item ordering.
- Canonical serialization for digest purposes.
- SHA-256 or equivalent approved digest type/format.
- Golden JSON and digest fixtures.
- Unit/integration tests and architecture documentation.
- Static/dependency guard proving core boundary.

### Explicitly out of scope

- .eggplan filesystem persistence.
- File locks/atomic replacement.
- Git command/repository inspection.
- EvidenceObservation storage or closure assessment.
- CLI commands beyond optional developer examples/tests.
- CodeGG adapter.
- Eggwork/Eggsearch/Eggbench integration.
- Signed attestations.
- Async runtime or network service.

## 6. Required production changes

### Workspace

Create a standard Cargo workspace. Keep dependency versions workspace-managed.
Add rust-version = "1.89" or stricter equivalent to publishable crates.

### Core/domain

Prefer modules approximately along these ownership boundaries:

- identity
- model
- validation
- graph/readiness
- schema/canonicalization
- digest
- error

Exact files may differ if a cleaner layout preserves these boundaries.

Define typed IDs with stable textual prefixes, initially:

- PlanId: ep_
- PlanItemId: epi_
- CriterionId: epc_
- EvidenceProviderId: epp_

EvidenceObservationId may be reserved now or introduced in Evidence M001, but
do not create two incompatible ID contracts.

Define PlanStatus and PlanItemStatus with explicit transition matrices. Keep
Draft/Active/Blocked/Closed/Cancelled plan semantics compatible with the
long-term spec; item states should support Pending/Actionable/InProgress/
Blocked/Completed/Cancelled or an equivalently explicit model.

### Bounds

Choose explicit constants for maximum items, dependencies, criteria,
requirements, objective/description/blocker/next-action/provenance text, IDs,
artifact refs, and extension metadata. Initial values may borrow conservatively
from CodeGG WorkPlan, but document the chosen values and why.

All length limits must define whether they count Unicode scalar values or
bytes. Serialized-size bounds may be additional.

### Acceptance requirements

AcceptanceCriterion must distinguish criterion statement from
EvidenceRequirement. Human judgment must be representable without pretending it
is host evidence.

Do not implement provider execution yet.

### SubjectRevision

Define transport-neutral subject identity sufficient for later Git mapping.
The core type must not shell out to Git.

### Canonicalization/digest

Specify the exact canonical bytes covered by digests. Do not rely on incidental
HashMap order or pretty-print formatting.

One acceptable approach is a dedicated canonical DTO with ordered collections
and compact serde_json serialization, with fields serialized in a deliberately
stable struct order. If using another canonical JSON scheme, document it.

Digest formatting should be stable, for example sha256:<lowercase hex>.

### Documentation/static guard

Add architecture/core.md and a small automated dependency/static guard that
fails if prohibited dependency families or reasoning-field names enter
eggplan-core.

## 7. Ordered work packages

### A — Workspace and core skeleton

Create Cargo manifests, crate skeleton, CI-friendly check script if useful, and
MSRV metadata.

Acceptance evidence: cargo metadata/check succeeds on the new workspace.

### B — Identity, schema, bounds, and validation

Implement typed IDs, versions, domain inputs/types, bounds, and validation
errors.

Acceptance evidence: focused tests for valid/invalid prefixes, NUL/empty/text
bounds, maximum collection sizes, and unknown schema.

### C — State transitions and graph readiness

Implement transition matrices, graph validation, cycle rejection, and stable
ready ordering.

Acceptance evidence: transition table tests plus DAG/cycle/missing/self/cross
plan cases.

### D — Canonical serialization and digest

Implement deterministic canonical serialization and golden fixtures.

Acceptance evidence: repeated construction/order variations produce the same
canonical bytes/digest where semantically identical; meaningful field changes
change the digest.

### E — Documentation and boundary guards

Document public ownership and prohibited responsibilities.

Acceptance evidence: static guard runs in verification.

## 8. Failure, restart, and contention semantics

M001 has no durable store, so restart/recovery is not implemented. Domain
functions must be pure and deterministic.

Invalid data returns typed errors and never panics for ordinary input.

Do not introduce global mutable state.

Contention/CAS persistence belongs to M002, but PlanRevision types and expected
revision semantics should not preclude it.

## 9. Compatibility and migration

There is no previous Eggplan runtime schema.

Schema v1 becomes the first compatibility baseline. Do not call an unstable
internal struct schema v1 unless it has the explicit fields and fixtures needed
for later read compatibility.

No CodeGG migration occurs here.

## 10. Required tests

### Unit

- ID parse/generate/display.
- all bounds.
- plan/item transition matrices.
- blocker discipline.
- criteria/requirement validation.
- no hidden reasoning field guard.
- canonical digest stability.

### Graph

- deterministic actionable ordering.
- dependencies satisfied/unsatisfied.
- parent existence.
- missing dependency.
- self dependency.
- dependency cycle.
- parent cycle.
- maximum-node behavior.

### Serialization

- schema-v1 golden fixtures.
- JSON roundtrip.
- unknown/invalid variant failure as intended.
- deterministic canonical digest.

### Boundary

- eggplan-core prohibited dependency/static ownership check.
- MSRV compilation.

## 11. Required verification commands

Expected commands after implementation; closure must record what actually ran.

    cargo fmt --all -- --check
    cargo check --workspace --all-targets --locked
    cargo clippy --workspace --all-targets --locked -- -D warnings
    cargo test --workspace --locked
    cargo +1.89.0 check --workspace --all-targets --locked

If a project check script is introduced, run it as an additional aggregate,
not as a substitute for recording the underlying gates.

## 12. Documentation updates

- root README: status/quick development.
- architecture/core.md.
- public schema/bounds documentation or docs/domain.md.
- AGENTS.md only if implementation discovers a needed standing instruction.
- plans registry/roadmap only in closure/status updates.

## 13. Acceptance criteria

M001 is complete when a downstream Rust crate can construct, validate,
serialize, digest, and inspect a bounded Plan and derive deterministic item
readiness without filesystem, network, process, database, CodeGG, or LLM
dependencies.

## 14. Stop conditions

Stop and report if:

- canonical digest semantics cannot be made deterministic with the selected
  representation;
- a proposed dependency would move execution/network/database/model ownership
  into core;
- the domain requires a major change to accepted Plan/PlanItem/evidence
  separation;
- CodeGG-specific runtime concepts appear necessary in the generic core;
- Rust 1.89 support is incompatible with a required dependency.

## 15. Closure evidence required

The closure must include:

- final workspace/crate ownership map;
- exact schema v1 and bound summary;
- transition/graph evidence;
- canonical digest fixture evidence;
- dependency/static boundary evidence;
- MSRV/full test results;
- unresolved findings and whether M002 may proceed.

## 16. Handoff notes

Prefer clarity over premature abstraction. M001 should be small enough that
M002 can build repository persistence on a stable domain without needing to
undo executor/service machinery.
