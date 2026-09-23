# Eggplan Long-Term Architecture and Product Specification

Status: canonical long-term implementation directive

Companion documents:

- plans/001-terminology-and-domain-model.md
- plans/002-long-term-roadmap.md
- plans/003-planning-process.md

The keywords MUST, MUST NOT, REQUIRED, SHOULD, SHOULD NOT, and MAY are
normative.

## 1. Product definition

Eggplan is a Rust-native repository-local work-order and evidence engine.

It represents software work as bounded, versioned plans; derives dependency
readiness; records what was actually observed during implementation and
verification; and computes deterministic closure assessments from structured
state and evidence.

Eggplan is usable by humans, coding agents, CI adapters, and higher-level
orchestrators. No LLM is required by the core.

The conceptual flow is:

    author / agent / orchestrator
              |
              v
          Plan + PlanItems
              |
        readiness / handoff
              |
              v
       external execution owner
       (human, CodeGG, CI, Eggwork)
              |
              v
       EvidenceObservations
              |
              v
       deterministic assessment
              |
              v
       ClosureRecord / registry

Eggplan owns the specification/evidence relationship. It does not own the
execution mechanism that creates the evidence.

## 2. Primary goals

Eggplan MUST provide:

1. Typed, versioned plan and work-item schemas with stable identifiers.
2. Bounded dependency graphs with deterministic readiness and cycle rejection.
3. Explicit acceptance criteria and evidence requirements.
4. Immutable evidence observations that distinguish planned verification from
   observed verification.
5. Subject/repository revision identity so stale evidence is not silently
   treated as proof for newer source state.
6. Deterministic closure assessment with explicit incomplete, blocked,
   failed, unavailable, inconclusive, and human-judgment states.
7. Optimistic revision/CAS semantics so stale writers fail explicitly.
8. A Git-friendly repository store using atomic replacement and reviewable
   structured files; SQLite MUST NOT be required for ordinary repository use.
9. Human-readable Markdown projections without making prose the only
   machine authority.
10. A derived registry/readiness view that can detect planning drift.
11. Library-first Rust APIs with a thin CLI and optional adapters.
12. Integration seams for CodeGG, Eggwork, Eggsearch, Eggbench, Git/forges,
    CI systems, and attestation formats.
13. Explicit schema compatibility and migration behavior.
14. Bounded data, paths, diagnostics, and artifact references.

## 3. Non-goals

Eggplan is not:

- a global or local scheduler;
- a cron/trigger engine;
- a process supervisor or remote executor;
- a CI service;
- a generic workflow/BPM engine;
- a project-management or issue-tracking service;
- an autonomous planning model;
- a model transcript, hidden-reasoning, or scratchpad store;
- a secrets manager;
- a Git hosting service;
- a package/build provenance authority by itself;
- a replacement for CodeGG project-level WorkOrders.

A caller may use Eggplan facts to make scheduling decisions, but Eggplan does
not choose when, where, or by which agent a work item runs.

## 4. Architectural principles

### 4.1 Evidence over claims

Model prose, Markdown status text, filenames, and a requested command are not
proof that work succeeded.

Only a structured EvidenceObservation from an accepted evidence provider, or
an explicitly recorded human judgment where policy permits it, may satisfy an
evidence-backed criterion.

### 4.2 Planned is not observed

A verification command listed in a plan is an EvidenceRequirement, not a
passing result. The system MUST preserve not_run, skipped, unavailable,
failed, and inconclusive rather than collapsing them into success.

### 4.3 Evidence is subject-scoped

Evidence is recorded against a concrete SubjectRevision.

The initial Git subject SHOULD include at least repository identity, commit
OID, and dirty-state information or digest. Evidence for one subject revision
does not automatically satisfy a later revision.

A future compatibility policy MAY permit ancestry-aware reuse for evidence
whose requirement explicitly allows it. Exact subject matching is the safe
default.

### 4.4 Mechanism below policy

Eggplan exposes facts and deterministic assessments. Callers own semantic
retry, scheduling, agent selection, human review policy, and deployment policy.

### 4.5 Structured authority, human projection

Versioned structured objects are the machine contract. Markdown is a
first-class reviewed projection and import surface, but free-form prose MUST
NOT be the sole source of truth for identities, revisions, evidence status, or
closure.

### 4.6 Local first, no database required

The default repository backend stores state in the repository and remains
inspectable with ordinary filesystem and Git tools.

Long-running hosts such as CodeGG MAY implement the same store/domain traits
using SQLite or another durable database.

### 4.7 Immutable evidence, revisioned intent

Plan state is revisioned and may evolve through validated CAS mutations.
Evidence observations are append-only/immutable once finalized. Corrections
are new observations or explicit supersession records, never silent rewriting
of historical evidence.

### 4.8 Bounded by construction

Plans, items, dependencies, criteria, evidence references, text fields,
artifact references, path depth/length, rendered projections, and diagnostic
payloads MUST have explicit bounds.

## 5. Canonical domain

The core domain includes at least:

- PlanId
- PlanRevision
- Plan
- PlanItemId
- PlanItem
- AcceptanceCriterionId
- AcceptanceCriterion
- EvidenceRequirement
- EvidenceObservationId
- EvidenceObservation
- EvidenceProviderId
- EvidenceStatus
- SubjectRevision
- ArtifactRef
- Assessment
- ClosureRecord

Typed IDs are distinct. Paths and display names are locators or labels, not
durable identity.

## 6. Plan model

A Plan records immutable origin plus revisioned work state. It includes:

- schema version and stable ID;
- monotonically increasing revision;
- objective;
- bounded origin/provenance metadata;
- repository/project scope;
- baseline/subject expectation when known;
- lifecycle status;
- items;
- timestamps;
- optional lineage/supersession relationships.

A Plan MUST NOT contain hidden model reasoning.

The initial lifecycle is:

    Draft -> Active <-> Blocked -> Closed
                     \-> Cancelled

Exact transitions may be refined by ADR, but terminal states MUST fail closed
on ordinary mutation.

## 7. Plan-item model and dependency semantics

A PlanItem records:

- stable item ID and owning plan;
- position and optional parent;
- dependency IDs;
- state;
- bounded description;
- acceptance criteria;
- evidence requirements/references;
- blocker and next-action fields;
- attempt/provenance metadata where useful.

Dependencies gate readiness only. Eggplan v1 does not add branch expressions,
loops, cron, recurrence, arbitrary predicates, or distributed workflow
transactions.

Dependency and parent graphs MUST reject missing cross-plan references,
self-dependencies, and cycles.

## 8. Acceptance and evidence requirements

An acceptance criterion states what must be true.

An evidence requirement states what forms of evidence can establish it.

A requirement can declare:

- required evidence kind;
- accepted provider class/identity constraints;
- subject-revision policy;
- cardinality or all/any policy;
- whether human judgment is allowed;
- optional bounded command/spec digest expected from an executor.

Criteria MUST remain unsatisfied when required evidence is absent, stale,
failed, unavailable, or otherwise outside policy.

## 9. Evidence observations

An EvidenceObservation is an immutable normalized record of something an
evidence provider actually observed.

It MUST support:

- stable observation ID;
- schema version;
- provider identity/type;
- evidence kind;
- explicit status;
- subject revision;
- observation time;
- bounded invocation/spec reference;
- bounded result metadata;
- artifact references by handle/path plus digest when available;
- producer/executor metadata where relevant;
- content digest over the canonical observation.

The baseline status vocabulary includes passed, failed, in_progress, not_run,
skipped, blocked, unavailable, and inconclusive.

Provider adapters may expose richer native states, but the normalized status
must remain truthful.

## 10. Evidence authority and trust

A caller cannot widen evidence authority merely by putting a provider name in
payload text.

Provider identity and observation construction are established by the adapter
or host boundary. Model-facing adapters MUST prevent a model from directly
constructing a passing host observation.

A content digest proves integrity of the recorded observation. It does not by
itself prove that the semantic claim is correct or that the producer was
authorized. Authentication/attestation may be layered later.

## 11. Subject revisions

The core MUST model subject identity independently of Git, but Git is the
first-class repository adapter.

The initial Git adapter SHOULD capture repository identity/root identity, HEAD
commit OID, worktree cleanliness, a deterministic dirty-state fingerprint when
dirty, and optional branch/display metadata that is never treated as identity.

Closure assessment compares evidence subject policy against the closure
candidate subject. Staleness is explicit.

## 12. Closure assessment

Assessment is pure and deterministic over plan state, current subject, and
evidence.

At minimum it distinguishes:

- complete;
- actionable work remaining;
- blocked;
- evidence failed;
- evidence unavailable/not run;
- in flight;
- awaiting human judgment;
- inconclusive;
- invalid/stale state.

A Plan cannot close solely because every item is labeled completed. Required
acceptance/evidence must also be satisfied.

A ClosureRecord snapshots the assessment inputs/revisions and records exact
verification evidence. It does not mutate historical observations.

## 13. Repository storage

The default repository adapter uses a private machine-state root such as:

    .eggplan/
      config.toml
      plans/
        <plan-id>/
          plan.json
          evidence/
            <observation-id>.json
          closure.json

Exact filenames are versioned by the repository-storage implementation.

Writes MUST use validated schema, revision comparison, safe path handling,
same-directory temporary files, and atomic replacement where the platform
supports it. Partial writes must never appear as finalized state.

The repository store SHOULD support a short-lived local lock for cooperative
writers, but correctness MUST still depend on revisions rather than assuming
the lock can never be bypassed.

## 14. Human planning projections

Eggplan SHOULD render and validate human-facing planning material, including
implementation-plan Markdown, closure Markdown, dependency/readiness tables,
registry summaries, graph output, and machine JSON output.

Generated projections MUST say they are projections when confusion with
canonical state is possible.

The CodeGG-style plans/ hierarchy is an important import/render target, but
Eggplan's core is not hard-wired to one repository's prose template.

## 15. CLI

The eventual CLI SHOULD include conceptual operations such as:

    eggplan init
    eggplan new
    eggplan show
    eggplan status
    eggplan ready
    eggplan graph
    eggplan check
    eggplan evidence ...
    eggplan assess
    eggplan close
    eggplan registry render

Machine-readable JSON is a first-class output mode. CLI commands MUST remain
thin adapters over library APIs.

## 16. Integration boundaries

### CodeGG

CodeGG remains owner of project WorkOrders, Sessions, Goals, Todos, AgentRuns,
Jobs, scheduling, model loops, worktrees, and context rollover.

Eggplan is intended to generalize reusable semantics currently present in
codegg-core::work_plan: typed plan/items, graph validation, evidence
correlation, revision/CAS behavior, bounded projection, and completion
assessment.

CodeGG-specific Goal binding, Todo projection, continuation checkpoints,
context epochs, model tools, and agent-loop policy remain CodeGG adapters.

Migration MUST be incremental and golden-test driven. No flag-day replacement
is required.

### Eggwork

Eggwork is an execution provider. Eggplan may describe a verification request
through an adapter and record returned execution/result facts. Eggplan MUST NOT
duplicate Eggwork's process or remote-execution ownership.

### Eggsearch

Eggsearch can produce repository/research evidence and bounded evidence
bundles. Remote/web material remains externally untrusted unless an Eggplan
policy says otherwise.

### Eggbench

Eggbench's immutable .eggb bundle model is a natural artifact/evidence
reference. Eggplan SHOULD reuse or reference that machinery rather than invent
a competing large-evidence container.

### Eggsact

Eggsact deterministic utilities may support adapters, preflight, text/JSON,
path, and repository checks. Eggplan core must not require an MCP process.

## 17. Interoperability

Eggplan's internal schema is not SLSA provenance or in-toto attestation, but
the concepts are intentionally compatible with external provenance systems.

Future adapters MAY export selected finalized evidence/closure facts as
in-toto-style attestations or consume GitHub/Sigstore artifact attestations.
Such attestations prove provenance/integrity under their policy; they do not
automatically satisfy arbitrary Eggplan acceptance criteria.

SLSA v1.2 treats provenance as verifiable information about where, when, and
how an artifact was produced. Eggplan uses the same separation between
subject, producer/invocation, and result while applying it to repository work
evidence.

## 18. Security requirements

Eggplan MUST reject unsafe relative paths and traversal where relevant; avoid
storing secrets in plans/evidence by default; provide bounded diagnostics with
redaction hooks; reject malformed/unknown required schema variants fail-closed;
prevent model or untrusted-source text from manufacturing trusted evidence;
distinguish integrity digests from authenticity; avoid executing arbitrary
commands merely by parsing a plan; and treat external research/fetch content
as untrusted input.

## 19. Compatibility and schema evolution

Every persisted machine object has an explicit schema version.

Readers SHOULD accept known older schemas through explicit migration/compat
paths. Unknown interpretation-changing variants fail closed. Additive fields
may be tolerated only where the schema version contract says they are safe.

Writers emit one current version. Historical finalized evidence is never
silently rewritten during ordinary reads.

## 20. Testing and qualification

Before a 0.1 stability claim, qualification must cover bounds and malformed
inputs; state transitions; graph cycles/readiness; CAS conflicts; interrupted
writes; path safety; subject-revision staleness; every evidence status;
attempted evidence forgery; deterministic closure; restart/reopen;
Markdown/JSON projection consistency; migration/unknown schema; CodeGG golden
parity; and Linux/macOS/Windows supported behavior.

Compilation alone is never sufficient closure evidence.

## 21. Initial dependency policy

The core should remain small and auditable. Serde, a cryptographic hash crate,
typed errors, UUIDs or equivalent stable IDs, and time handling are reasonable
foundational dependencies. Git, async runtime, HTTP, database, MCP, and
executor dependencies belong in adapters unless a later ADR demonstrates a
core requirement.

MSRV starts at Rust 1.89 to match current Eggstack distribution targets.


## 22. External design basis reviewed at planning bootstrap

These sources informed the initial architecture. They are research references,
not permanent dependency pins:

- OpenAI Codex ExecPlans / PLANS.md guidance:
  https://github.com/openai/openai-cookbook/blob/main/articles/codex_exec_plans.md
- SLSA v1.2 provenance:
  https://slsa.dev/spec/v1.2/provenance
- in-toto specifications and Attestation Framework:
  https://in-toto.io/docs/specs/
- GitHub artifact attestations:
  https://docs.github.com/en/actions/concepts/security/artifact-attestations
- CodeGG WorkPlan architecture at the reviewed baseline:
  https://github.com/dbowm91/codegg/blob/2f7d84f88070eee2a4fb9f70b6d7d5d10b01048d/architecture/work_plan.md
- Eggbench evidence bundle design:
  https://github.com/eggstack/eggbench/blob/cbca21a8b3eb24ec8ecbf58febbc8c505c7c8613/docs/evidence-bundle.md

The design deliberately borrows the useful separation between planned steps,
observed execution, subject identity, provenance, and verification without
claiming that Eggplan itself provides SLSA/in-toto authenticity guarantees.
