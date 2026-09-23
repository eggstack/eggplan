# Interoperability and Distribution Roadmap

Status: deferred

Long-term references: plans/000-long-term-specification.md sections 17 and 19-21.

## 1. Purpose

After the local planning/evidence contract is stable, add standards
interoperability and normal Eggstack distribution without polluting the core.

## 2. Standards baseline

Planning reviewed:

- SLSA v1.2 approved provenance documentation.
- in-toto Attestation Framework stable v1.0.
- GitHub artifact attestations using Sigstore-backed provenance.

These systems establish provenance, integrity, or producer facts under their
own models. Eggplan criterion satisfaction remains a separate policy decision.

## 3. Milestones

### M001 — Attestation import/export

Optional adapters for selected EvidenceObservation and ClosureRecord facts to
in-toto-style statements and verification of supported external attestations.

Eggplan does not own a signing-key infrastructure.

### M002 — Packaging and update

Release library and CLI artifacts for the current Eggstack target matrix, then
adopt Eggup verified self-update if its consumer seam is suitable.

### M003 — MCP/service adapters

Optional bounded agent/service access, likely stdio MCP first and loopback HTTP
only if justified. Reuse Eggserve and Eggfetch as appropriate rather than
creating custom network stacks.

### M004 — v0.1 qualification

Full matrix across Linux/macOS/Windows supported behavior, schema fixtures,
CodeGG parity/adoption, integrations, packaging, and negative trust/path cases.

## 4. Completion

Eggplan is independently installable and interoperable without making
attestation, network, or update code mandatory in eggplan-core.
