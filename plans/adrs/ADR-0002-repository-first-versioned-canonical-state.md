# ADR-0002: Repository-First Versioned Canonical State

Status: accepted

Date: 2026-09-22

## Context

CodeGG's live daemon needs SQLite, while a reusable repository work-order tool
needs reviewable local state that travels naturally with branches/worktrees and
can be inspected without a service.

The existing CodeGG planning convention uses rich Markdown, but prose alone is
too ambiguous for IDs, revisions, evidence states, and deterministic closure.

## Decision drivers

- Git-friendly diffs and portability.
- No database required for ordinary use.
- Strong machine validation/versioning.
- Human-readable projections.
- Explicit concurrency rather than last-write-wins.
- Safe recovery after interrupted writes.

## Options

### A. Markdown as the only authority

Rejected. Human-friendly but too ambiguous for machine closure and CAS.

### B. SQLite as mandatory authority

Rejected for default repository use. It is poor for code review/merge and
creates needless service/storage coupling, although host adapters may use it.

### C. Versioned structured files plus Markdown projections

Selected.

## Decision

The default repository backend stores canonical machine objects beneath a
private state root, initially .eggplan/. The current writer emits explicitly
versioned JSON objects and uses a small TOML configuration file where useful.

Plans are revisioned. Mutations require the expected revision and fail on
conflict.

Repository writes use:

- validated relative paths;
- cooperative short-lived lock where useful;
- same-directory staging/temp file;
- fsync/flush to the supported platform level;
- atomic rename/replace when supported;
- reopen validation.

Partial staging files are never treated as finalized canonical state.

Markdown implementation plans, closure reports, and registries are
projections/import surfaces. They do not become evidence merely because they
contain a status sentence.

Host applications MAY implement the same domain/store traits with SQLite or
other durable stores.

## Canonical serialization

Foundation M001 must freeze deterministic canonical JSON rules sufficient for
stable digests. Pretty-printing is not the digest contract unless explicitly
defined.

## Consequences

Positive: state is reviewable, portable, service-independent, and suitable for
multiple frontends.

Negative: repository merge conflicts still require explicit resolution;
filesystem locking is cooperative; schema migration must be maintained.

## Verification

Required tests include atomic-write interruption, reopen, conflict, symlink and
path traversal rejection, stale revision, unknown schema, and deterministic
canonical digest fixtures.
