# ADR-0003: Immutable Revision-Scoped Evidence

Status: accepted

Date: 2026-09-22

## Context

Coding agents and human workflows frequently conflate "I planned to run this
test", "the test ran", and "the test passed." They also commonly keep citing a
passing test after changing the repository again.

CodeGG's WorkPlan already separates host evidence from model prose. Eggplan
needs to generalize and strengthen that boundary for repository-scoped work.

SLSA provenance similarly separates a subject from information about where,
when, and how it was produced. in-toto separates planned supply-chain steps
from recorded link metadata. GitHub artifact attestations bind artifacts to
workflow/repository/commit provenance but explicitly do not establish that an
artifact is semantically safe.

## Decision drivers

- Never manufacture proof from prose.
- Preserve failed/unrun/skipped/unavailable results.
- Prevent stale evidence from closing newer source.
- Keep historical evidence auditable.
- Permit multiple evidence producers.
- Distinguish integrity from authenticity.

## Decision

EvidenceObservation is immutable after finalization.

It records at least:

- stable observation ID and schema version;
- provider identity/type established by host/adapter boundary;
- evidence kind;
- normalized status;
- SubjectRevision;
- observation timestamp;
- bounded invocation/spec reference;
- bounded result metadata;
- artifact references/digests;
- canonical content digest.

The normalized baseline statuses are passed, failed, in_progress, not_run,
skipped, blocked, unavailable, and inconclusive.

Evidence applicability is evaluated against the current closure candidate's
SubjectRevision. Exact subject matching is the default.

A correction creates a new observation and optionally an explicit supersedes
relationship. Ordinary reads/migrations do not rewrite finalized historical
observations.

A digest establishes recorded-content integrity only. Authentication and
signed attestation are separate layers.

Models/untrusted inputs may propose evidence references or verification
requirements but may not directly create trusted passing host observations.

## Consequences

Positive: closure becomes auditable and robust to post-test source changes.

Negative: some evidence becomes stale frequently on dirty development trees;
providers need accurate subject capture.

## Verification

Tests must cover all statuses, stale evidence, changed dirty fingerprint,
provider spoof attempts, digest corruption, dangling observations, correction
lineage, and deterministic re-assessment after reopen.
