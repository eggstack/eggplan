# Eggstack Integration Roadmap

Status: active

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

## 3.1 Reviewed baselines

Initial research baselines from 2026-09-23:

- Eggwork: c990ffa2864886c502c1d5581f0d732e26fe56ee
- Eggsearch: 5db6e1984a1441787f6d6a54754eb4a685766ec2
- Eggbench: 3b93979a7fd30a08f7e367ffc94f911ae5fb8bec
- Eggsact: 576f4b0ac09238a42e5561c2da6da8ff4a47bce6

Execution-time re-check on 2026-09-24:

- Eggwork: 128f808c62f176d414dd18a705773e45f5e2891a
- Eggsearch: dfa90e050c5434f3346902aeb4074901c58e90d1
- Eggbench: d7d1fd9a9b67a5b2ca6a816c841d2588a368aae9
- Eggsact: 40959b704431430668e9ca2bfe959a8ef32495d8

These are research baselines, not dependency pins; re-check before provider
adapter implementation.

## 4. Milestones

### M001 — Evidence provider SPI

Status: ready.

Plan: plans/implementation/eggstack-integration/001-evidence-provider-spi.md

Sibling execution/research/bundle interfaces were re-checked at the execution
baselines above. Freeze a normalization-focused SPI,
capability/failure reporting, host-controlled provider identity, sync/async
boundary, artifact references, and verification-binding handoff.

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
