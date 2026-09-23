# Foundation Core and Repository Roadmap

Status: active

Long-term references:

- plans/000-long-term-specification.md
- plans/001-terminology-and-domain-model.md
- plans/002-long-term-roadmap.md

Related ADRs: ADR-0001, ADR-0002, ADR-0003.

## 1. Purpose and ownership boundary

This subsystem establishes the dependency-light Rust domain and default
repository persistence. It owns typed IDs, plan/item/criterion/requirement
schemas, bounds, validation, graph readiness, canonical serialization,
revisions, repository store, and SubjectRevision capture.

It does not own evidence-provider integrations, command execution, Markdown
import/render policy, scheduling, CodeGG runtime semantics, or network service.

## 2. Work classification

### Invariants

- No hidden reasoning fields.
- Plan and evidence identities remain typed and distinct.
- Graphs are bounded and acyclic.
- Stale writes fail explicitly.
- Canonical serialization and digests are deterministic.
- Repository persistence cannot expose partial writes as finalized state.
- Git dirty state is not misrepresented as clean HEAD.

### Capabilities

- Construct, validate, and serialize a Plan.
- Compute deterministic item readiness.
- Persist and reopen plans locally with CAS.
- Resolve current Git SubjectRevision.

### Infrastructure

- Rust workspace, eggplan-core, eggplan-repo.
- Schema versioning and fixtures.
- Filesystem atomic-write and locking primitives.

## 3. Non-goals

No CLI product workflow beyond test harnesses, evidence execution, remote
storage, SQLite backend, general workflow DSL, or Git mutation.

## 4. Current state

At planning bootstrap there is no Rust workspace or production code. The design
is informed by CodeGG codegg-core::work_plan and sibling Eggstack repository
patterns.

## 5. Target architecture

    eggplan-core
      model / ids / validation
      dependency graph / readiness
      canonical serialization + digest
      subject-neutral types
      store traits

    eggplan-repo
      .eggplan layout
      atomic file persistence
      revision/CAS
      cooperative locking
      Git subject adapter
      reopen/recovery

Core must remain usable without Git/repository persistence.

## 6. Dependency graph

    M001 typed domain/schema
          |
          v
    M002 repository store/CAS/Git subject
          |
          v
    M003 hardening/property/migration guards

M003 stays roadmap-level until M002 makes the concrete implementation visible.

## 7. Milestones

### M001 — Repository bootstrap and typed domain contract

Class: infrastructure / invariant

Status: closed.

Plan: plans/implementation/foundation-core/001-repository-bootstrap-and-domain-contract.md

Exit: workspace builds on Rust 1.89+, domain roundtrips, bounds/transition/graph
tests pass, and canonical digest fixtures are frozen.

### M002 — Repository store, CAS, and Git subject

Class: capability / invariant

Status: ready after M001 closure.

Plan: plans/implementation/foundation-core/002-repository-store-cas-and-subject.md

Exit: repository-local state safely persists/reopens; stale revisions conflict;
interrupted writes fail closed; current Git clean/dirty subject is
deterministically captured.

### M003 — Foundation hardening and migration guards

Class: invariant / polish

Roadmap-level. After M002, add property/fuzz tests, schema migration fixtures,
static dependency guards, corruption/recovery cases, and any cross-platform
filesystem corrections revealed by qualification.

## 8. Cross-cutting requirements

Persistence is local-first; no network. Paths are confined. Schema versions are
explicit. Unknown interpretation-changing variants fail closed. Resource bounds
are enforced in core, not only CLI.

## 9. Verification strategy

Unit tests for IDs, bounds, transitions, graph, canonicalization; fixture tests
for schema/digest; integration tests for CAS, contention, atomic persistence,
reopen; Git fixtures for clean/dirty state; Linux/macOS/Windows supported-subset
CI.

## 10. Risks

Canonical JSON/digest ambiguity is a key early risk. Freeze one algorithm and
golden fixtures in M001 before evidence observations depend on it.

Filesystem atomic/durability differences across Windows and Unix must be
described truthfully rather than overclaimed.

## 11. Completion definition

This subsystem closes when the typed core and repository store are qualified,
schema/migration behavior is documented, and downstream evidence work can rely
on stable revisions and subjects.
