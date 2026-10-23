# Foundation Core and Repository Roadmap

Status: closed / current

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

M003 is ready for planning now that M002 makes the concrete implementation
visible. Prioritize Windows/macOS qualification and any corrections it requires.

## 7. Milestones

### M001 — Repository bootstrap and typed domain contract

Class: infrastructure / invariant

Status: closed.

Plan: plans/implementation/foundation-core/001-repository-bootstrap-and-domain-contract.md

Exit: workspace builds on Rust 1.89+, domain roundtrips, bounds/transition/graph
tests pass, and canonical digest fixtures are frozen.

### M002 — Repository store, CAS, and Git subject

Class: capability / invariant

Status: conditionally closed. Historical closure:
plans/closure/foundation-core/002-conditionally-closed.md. The Windows/macOS
qualification caveat is resolved by M003, which is closed.

Plan: plans/implementation/foundation-core/002-repository-store-cas-and-subject.md

Exit: repository-local state safely persists/reopens; stale revisions conflict;
interrupted writes fail closed; current Git clean/dirty subject is
deterministically captured.

### M003 — Subject scope, strict schema, and platform hardening

Class: invariant / corrective hardening / qualification

Status: closed.

Plan: plans/implementation/foundation-core/003-subject-scope-strict-schema-and-platform-hardening.md

This milestone now owns the concrete post-closure findings discovered after
Evidence M001:

- Eggplan-managed .eggplan state must be excluded from Git dirty subject
  identity so persisting plans/evidence cannot stale their own source subject;
- schema-v1 plan decoding must reject unknown nested fields while preserving
  valid v1 canonical bytes/digests;
- native Linux/macOS/Windows CI must qualify repository lock, replacement,
  path, Git-subject, and durability behavior and correct platform defects;
- directly related README/architecture drift must be repaired.

The historical M001/M002 closure records remain unchanged. M003 closure
establishes the current corrected qualification state.

### M003 corrective lines

Raised by the 2026-10-23 repository-wide audit against baseline
`37ad290ca6b971a68d2871aedb39f4882f4eb2a8`. All planned verification was run
and green at that baseline, and every finding is inference- and
reachability-established against the source rather than a test failure. The
M003 milestone itself remains closed; these are corrective lines against it.
None of them gates another subsystem.

| Line | Class | Scope | Plan | Closure |
|---|---|---|---|---|
| C001 | invariant / trust-rule consolidation | One canonical owner for the execution-evidence kind classification, so a new `EvidenceKind` variant cannot silently escape the verification-binding requirement | plans/implementation/foundation-core/003-c001-canonical-execution-kind-predicate.md | closed — plans/closure/foundation-core/003-c001-closed.md |
| C002 | correctness / diagnostics fidelity | Carry the Git subject capture error instead of collapsing it, in the inspection snapshot and the CLI read path | plans/implementation/foundation-core/003-c002-subject-capture-error-fidelity.md | closed — plans/closure/foundation-core/003-c002-closed.md |
| C003 | invariant / schema integrity | `deny_unknown_fields` parity for the nested `AssessmentReason` enum and reason-level injection-path coverage | plans/implementation/foundation-core/003-c003-core-assessment-reason-schema-fail-closed.md | closed — plans/closure/foundation-core/003-c003-closed.md |
| C004 | non-blocking maintenance | Unreachable public surface, an unconstructed error variant, and a panic path in validation | plans/implementation/foundation-core/003-c004-unreachable-public-surface-and-panic-path-hygiene.md | closed — plans/closure/foundation-core/003-c004-closed.md |
| C005 | tooling governance | Core boundary guard tool precondition and source-purity enforcement, plus tooling-governance design-record truthfulness | plans/implementation/foundation-core/003-c005-core-boundary-guard-and-tooling-governance-reconciliation.md | closed — plans/closure/foundation-core/003-c005-closed.md |

All five lines were implemented together against baseline
`37ad290ca6b971a68d2871aedb39f4882f4eb2a8` and are closed. They did not
serialize against each other: the core-side changes (C001, C003, C004) landed
in `eggplan-core`, the downstream consumption and bridge-decision changes
(C001, C003) landed in `eggplan-integrations` and `eggplan-codegg-compat`, the
inspection and diagnostics changes (C002, C004) landed in `eggplan-repo` and
`eggplan-cli`, and the tooling change (C005) landed in `scripts/` and the
design record. No capability plan waited on any of them.

The closure records carry one behavior change that a reader of this roadmap
should know about: M003 C002 changed the `show` command's read path so that a
subject capture failure is reported as `subject_unavailable` instead of
returning a subjectless summary with no diagnostic. The plan's own §6 and §9.6
conflicted with its §9.3 on this point; the acceptance criterion was followed
and the conflict is recorded in the closure evidence rather than left implicit.



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
schema/migration behavior is documented, Eggplan administrative state cannot
self-perturb the source subject, strict v1 decoding is proven, native supported
platform evidence is recorded, and downstream evidence work can rely on stable
revisions and subjects.
