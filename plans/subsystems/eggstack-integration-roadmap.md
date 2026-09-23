# Eggstack Integration Roadmap

Status: ready for planning

Long-term references: plans/000-long-term-specification.md sections 16-17.

## 1. Purpose

Connect Eggplan to existing Eggstack capabilities through narrow adapters after
the evidence contract is stable.

## 2. Ownership rule

Integrations produce or resolve facts. They do not move their native ownership
into Eggplan.

## 3. Candidate integrations

### Eggwork

Role: fixed-target execution evidence provider.

Map execution identity/generation, terminal state, exit status, bounded
stdout/stderr/artifacts, and subject/workspace provenance into observations.
Eggplan does not supervise processes.

### Eggsearch

Role: repository/research evidence provider.

Preserve source identities, trust markers, failure/absence state, and bounded
bundle references. External-untrusted research does not become host proof by
default.

### Eggbench

Role: qualification/performance evidence.

Reference or verify immutable .eggb bundles and their manifest/digests. Avoid
copying large bundle payloads into Eggplan control records.

### Eggsact

Role: optional deterministic in-process/preflight utilities.

Use only where a stable library seam reduces duplicate deterministic helpers;
do not require MCP.

### Eggup

Role: eventual distribution/update integration, not evidence-core dependency.

## 4. Milestones

### M001 — Evidence provider SPI

Evidence M001 C001 is closed. The corrective dependency gate is cleared;
re-check sibling provider interfaces and write a bounded implementation plan
before implementation. Provider adapters must emit the corrected
verification-spec binding.

After the corrective gate, freeze adapter traits, capability/failure reporting, sync/async boundary, provider identity construction, and verification-binding handoff.

### M002 — Eggwork and Eggsearch

After M001. Two contrasting providers qualify execution and research trust
semantics.

### M003 — Eggbench plus CI/forge

After M002. Add bundle, CI, commit, and artifact evidence and forge adapters.

## 5. Verification

Provider-specific native failures must remain observable after normalization.
Adapters cannot silently downgrade route, authorization, integrity, or
availability failures or report unavailable evidence as passed.

## 6. Completion

Multiple external producers can supply auditable observations without
eggplan-core owning their transports or execution systems.
