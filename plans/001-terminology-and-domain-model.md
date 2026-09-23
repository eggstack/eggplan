# Eggplan Canonical Terminology and Domain Model

Status: normative companion to plans/000-long-term-specification.md

## 1. Naming rules

1. Durable concepts use typed identities.
2. A path, branch name, command string, display title, or Markdown filename is
   not durable identity.
3. Plan intent, execution, evidence, and closure are distinct.
4. Planned verification must not be called evidence until it was observed.
5. Integrity and authenticity are distinct claims.
6. CodeGG WorkOrder and Eggplan Plan are not synonyms.

## 2. Repository scope

### Repository

The source/control domain whose work is being planned. Git is the initial
adapter, but the core does not require Git semantics.

### Repository identity

A stable bounded identifier for the repository lineage/configured subject.
A filesystem path is only a locator.

### Subject revision

The exact source state to which an evidence observation applies.

Suggested shape:

    SubjectRevision
    |-- subject_kind
    |-- repository_id
    |-- revision
    |-- clean/dirty state
    `-- optional dirty_digest

For Git, revision is normally a commit OID. A dirty worktree must not be
represented as though it were the clean HEAD commit.

### Baseline

The subject revision against which a plan was researched or authored.
Baselines establish context; they do not make later evidence valid
automatically.

## 3. Planning terms

### Plan

One durable bounded specification of an objective and the work required to
satisfy it.

Suggested identifier: PlanId with an ep_ textual prefix.

A Plan has a monotonic PlanRevision. The revision is a concurrency token, not a
Git commit number.

### Plan item

One bounded unit of required work inside a Plan.

Suggested identifier: PlanItemId with an epi_ prefix.

A PlanItem can depend on other items in the same Plan. Dependency semantics
only determine readiness; they do not schedule execution.

### Plan lineage

The explicit supersedes/replaces/derived-from relationship between Plans or
plan revisions. Historical plans are retained for traceability.

### Objective

The user/operator outcome the Plan exists to achieve. It is not a list of file
edits.

### Dependency

A typed relation saying another PlanItem must reach an accepted terminal state
before the dependent item becomes ready.

### Readiness

A pure derived state indicating whether an item may be acted on according to
plan status, item status, and dependencies.

Readiness does not grant execution authority.

### Blocker

A bounded explicit reason why an item or Plan cannot currently progress.

### Next action

A bounded human/agent-facing suggestion for the next concrete step. It is not
an authoritative executor instruction.

## 4. Acceptance terms

### Acceptance criterion

A statement that must be satisfied before the associated item can be treated
as successfully complete.

Suggested identifier: CriterionId.

### Evidence requirement

A machine-readable rule describing acceptable evidence for a criterion.

A requirement may constrain evidence kind, provider, subject policy, and
all/any/cardinality behavior.

### Human judgment requirement

A criterion intentionally requiring an authorized human decision rather than
deterministic host evidence.

It must remain explicit; the system must not convert uncertainty into an
implicit pass.

## 5. Evidence terms

### Evidence provider

The trusted adapter/host component that converts an external observation into
an Eggplan EvidenceObservation.

Suggested identifier: EvidenceProviderId.

Examples include a local command runner adapter, Eggwork, CodeGG job/test
stores, CI, Git commit inspection, Eggsearch, or an attestation verifier.

A provider is not trusted merely because input text names it.

### Evidence observation

One immutable normalized record of something a provider actually observed.

Suggested identifier: EvidenceObservationId with an epe_ prefix.

An observation includes its subject revision, provider, kind, status,
timestamp, optional invocation reference, artifacts, and canonical digest.

### Evidence status

The normalized result state is passed, failed, in_progress, not_run, skipped,
blocked, unavailable, or inconclusive.

not_run means the requested observation was never performed. unavailable means
the referenced source/result cannot currently be resolved. skipped means an
execution path deliberately omitted the check. These are not interchangeable.

### Evidence kind

A versioned semantic category of observation, for example command/test
execution, static analysis, commit/revision observation, artifact,
delegated run, benchmark/qualification bundle, repository research, human
judgment, or attestation verification.

The initial core should use a closed built-in set plus a controlled extension
representation rather than free-form strings with no compatibility rules.

### Invocation reference

A bounded description or digest of what was asked to execute/observe. It is
provenance, not authority.

### Artifact reference

A handle/path/URI plus media/role metadata and digest where available.

Large payloads should remain artifact-backed rather than embedded in control
records.

### Evidence digest

A cryptographic digest of canonical serialized observation content.

It provides integrity detection. It does not prove producer authenticity.

### Stale evidence

Evidence whose subject does not satisfy the current requirement's subject
policy.

Stale evidence remains historical evidence but cannot silently close current
work.

## 6. Assessment and closure terms

### Assessment

A pure result derived from Plan state, current subject, and evidence.

The baseline assessment families are Complete, ActionableWorkRemaining,
Blocked, EvidenceFailed, EvidenceMissingOrUnavailable, InFlight,
AwaitingHumanJudgment, Inconclusive, and InvalidOrStale.

Assessment does not mutate the Plan.

### Closure candidate

The exact plan revision plus subject revision being considered for closure.

### Closure record

An immutable snapshot recording a closure decision, source Plan revision,
subject revision, criterion/evidence matrix, verification observations, known
gaps, and disposition.

Suggested identifier: ClosureId.

### Closed

A plan lifecycle state reached only through the closure rules. It must not be
used as shorthand for "implementation agent reported done."

### Conditionally closed

A policy-level disposition meaning implementation is substantially complete
but explicitly named external/operational evidence remains. The exact use is
adapter/policy-owned; core assessment still reports the underlying unsatisfied
or unavailable requirement truthfully.

## 7. Repository-store terms

### Eggplan state root

The repository-local directory containing canonical Eggplan machine state,
initially targeted as .eggplan/.

### Canonical object

A versioned structured object whose parsed fields, IDs, revisions, and digest
participate in machine decisions.

### Projection

A derived human- or model-facing representation of canonical objects.

Markdown registry/implementation/closure documents may be projections. A
projection does not gain mutation authority merely because it is editable.

### Import

A deliberate parse/translation of supported external or legacy planning
material into canonical objects. Import must report information it cannot
represent.

### Repository store

The filesystem/Git-friendly PlanStore implementation. It owns safe paths,
atomic writes, locks, schema loading, and CAS enforcement.

### CAS conflict

A mutation failure because the caller's expected revision differs from current
state. Eggplan must report this explicitly rather than silently last-write-win.

## 8. Integration terms

### Executor

A component that actually performs commands or other side effects. Eggwork,
CodeGG jobs, CI, or a human shell may be executors. Eggplan core is not.

### Orchestrator

A component that decides when/where/who performs work. CodeGG WorkOrders are an
example. Eggplan core is not.

### Evidence adapter

Integration code that resolves native external results into normalized,
bounded Eggplan observations.

### CodeGG WorkPlan adapter

The future integration mapping reusable Eggplan domain semantics into CodeGG's
session-local WorkPlan behavior.

### CodeGG WorkOrder

A project-level CodeGG object that decides when a normal session may be
materialized. It remains distinct from Eggplan Plan.

## 9. Planning-repository terms

The Eggplan source repository itself uses CodeGG-style development planning:
long-term specification, terminology/domain model, ADR, subsystem roadmap,
implementation plan, closure record, and active planning registry.

These development documents govern building Eggplan. They are not themselves
the final persisted runtime schema of Eggplan.
