# Projection and CLI M003b — Batch Queries, Compact Projections, and Shell Completions

Status: closed

Repository baseline: 71904570e908a15c099f9b2804cabfbccf9ae51a

Source roadmap:

- plans/subsystems/projection-cli-roadmap.md
- plans/002-long-term-roadmap.md Projection/CLI M003

Predecessors:

- Projection/CLI M001 — closed
- Projection/CLI M002 — closed

Parallel sibling plan:

- plans/implementation/projection-cli/003a-repository-inspection-snapshot-and-performance-qualification.md

Dependency note:

Completion metadata and compact DTO work may proceed in parallel. Repository-
wide batch execution should consume the M003a inspection snapshot rather than
reintroducing repeated per-Plan reads.

Primary class: CLI ergonomics / machine projection / compatibility

## 1. Objective

Make real multi-Plan repositories easier for humans, scripts, and agents to
inspect without changing canonical authority or destabilizing existing JSON
contracts.

M003b adds:

- bounded batch Plan queries;
- a compact machine projection for overview use;
- provider-policy-aware read projections;
- shell completion generation;
- one declarative command metadata source so parser validation, help, and
  completions do not drift.

This milestone does not make the CLI an executor or evidence producer.

## 2. Compatibility rule

Existing commands and JSON v1 payloads remain valid.

Do not silently remove fields from:

- `status`;
- `show`;
- `ready`;
- `graph`;
- `registry render`;
- `check`.

"Compact" behavior must use a new explicit projection/command shape rather than
changing the meaning of an existing JSON response.

Human formatting may improve if it does not hide truncation, blockers,
unavailable evidence, or failure.

## 3. Batch query surface

Add a narrow read-only command family. Recommended initial surface:

    eggplan list [--status STATUS] [--limit N] [--after PLAN_ID]
                 [--provider-policy FILE] [--json]

`list` returns a compact deterministic Plan overview rather than full
`PlanSummary` objects.

Also extend:

    eggplan status [PLAN_ID ...]

from zero-or-one positional Plan ID to a bounded list of explicit IDs while
preserving today's behavior:

- no IDs -> repository-wide bounded status;
- one ID -> identical semantics to current single-plan status;
- multiple IDs -> same stable projection for the requested set.

Cap explicit IDs and `--limit` at documented bounds. Reject duplicates
rather than silently deduplicating caller mistakes.

Repository-wide execution must use M003a's command-local inspection snapshot.

## 4. Compact Plan projection

Add a new versioned reusable projection DTO in `eggplan-projection`, for
example `CompactPlanSummaryV1`.

Include only high-value bounded fields:

- plan ID;
- revision;
- lifecycle status;
- item count;
- ready item count;
- blocked item count;
- closure present;
- optional assessment status;
- stable assessment reason codes, bounded;
- objective preview + truncation marker.

Do not include full item descriptions, evidence records, blocker prose, or
arbitrary metadata.

Ordering is deterministic by Plan ID unless the command explicitly documents a
different stable order.

The projection is read-only and contains no repository/filesystem logic.

## 5. Pagination/selection

Use deterministic keyset-style selection rather than offset pagination.

`--after PLAN_ID` means strictly greater Plan IDs in canonical sorted order.

Return:

- selected rows;
- total known matching count when it can be obtained without violating the
  bounded snapshot contract;
- returned count;
- truncation flag;
- optional next cursor/last Plan ID.

Do not introduce opaque random cursors or server-side state.

Status filters use canonical `PlanStatus` values only. Do not filter by prose.

## 6. Provider-policy-aware read projections

Today `assess`/`close` take explicit provider policy, while overview
commands may assess with an empty registry and therefore surface legitimate
observations as untrusted.

Add optional `--provider-policy FILE` to read commands where assessment is
shown:

- `show`;
- `status`;
- `list`;
- `registry render`;
- retain current optional support on `check`.

Rules:

- absence of a policy continues to mean no implicit trust;
- supplied policy uses the existing strict bounded parser;
- one loaded policy is reused across the entire M003a snapshot;
- provider IDs found in evidence never self-enroll;
- human output should indicate when assessment used no provider policy if
  evidence exists and the distinction matters.

Do not create a repository-global trust store in M003b.

## 7. Command metadata source

The current parser/help/option validation is handwritten. Shell completion
support must not create a fourth independent command definition.

Introduce an internal declarative command metadata table describing:

- command/subcommand names;
- positional shapes;
- flags/value options;
- repeatability;
- enum/value hints;
- short help text.

Use that metadata for at least:

- usage/help rendering;
- unknown-option validation;
- shell completion generation.

Parser execution may remain handwritten.

Do not perform a wholesale parser migration solely to obtain completions.

If a completion helper dependency is proposed, it must stay CLI-only and pass
the existing projection/CLI boundary guard. Clap may not take over error/output
authority unless exact CLI JSON/error compatibility is first proven.

## 8. Shell completions

Add:

    eggplan completions bash
    eggplan completions zsh
    eggplan completions fish
    eggplan completions powershell

Output the completion script to stdout and perform no repository mutation.

Requirements:

- deterministic for the same binary version;
- generated entirely from local command metadata;
- no shell execution;
- no home-directory writes;
- no network access;
- no repository scan required to generate the script.

Static completion of command names/options/status enum values is required.
Dynamic Plan-ID completion is explicitly out of scope for M003b because it
would couple shell completion to repository discovery and latency.

## 9. Human ergonomics

Improve high-volume read output conservatively:

- deterministic one-row compact list output;
- explicit truncation/next-cursor notice;
- concise status/assessment codes;
- preserve full detail in `show` and existing detailed commands.

Do not add ANSI/color as a correctness dependency. Color, pager, and TUI remain
out of scope.

## 10. Machine output

All new machine output uses the existing top-level `OutputEnvelope` schema v1
unless an actual envelope incompatibility is discovered.

New command-specific data DTOs are explicitly versioned when they are intended
as stable agent interfaces.

Freeze golden fixtures for:

- empty list;
- one row;
- filtered rows;
- paginated/truncated result;
- provider-policy assessment;
- no-policy assessment;
- multi-ID status;
- each completion command success metadata where applicable.

Stable reason codes come from domain/projection code, never human-string
parsing.

## 11. Bounds and failure behavior

At minimum:

- explicit Plan IDs per batch are bounded;
- `--limit` has a small documented maximum, initially no greater than current
  `MAX_PROJECTED_PLANS`;
- duplicate IDs fail;
- invalid status filter fails;
- malformed cursor Plan ID fails;
- unknown completion shell fails with `usage`;
- malformed provider policy preserves current typed policy diagnostics.

No batch command should turn one corrupt requested Plan into an apparently
complete successful row. Fail the command or surface an explicit per-row
invalid state only if the existing integrity contract supports that distinction
without hiding corruption.

## 12. Tests

### Batch/list

- empty repository;
- 1/10/100 Plan ordering;
- status filtering;
- keyset pagination without duplication/omission;
- duplicate explicit IDs rejected;
- unknown Plan ID behavior deterministic;
- truncation/count/cursor fields correct;
- repository-wide path uses M003a snapshot.

### Provider policy

- same state with no policy reports untrusted/missing assessment truthfully;
- supplied valid policy changes only assessment interpretation, not evidence;
- malformed/duplicate provider policy rejected;
- provider IDs in observations cannot self-authorize.

### Completions

- bash/zsh/fish/PowerShell generation snapshots;
- output contains current commands and excludes nonexistent commands;
- adding a command metadata entry changes help and completions together;
- generation does not touch repository state;
- no shell/network/process APIs used.

### Compatibility

- pre-M003 command fixtures remain byte/semantically stable where promised;
- `status PLAN_ID` single-ID behavior unchanged;
- native Linux/macOS/Windows;
- Rust 1.89.

## 13. Documentation

Update README quickstart references where useful, `docs/cli-reference.md`,
provider-policy documentation, and architecture CLI control-surface
documentation.

Document the compact list schema, batch bounds, cursor semantics,
provider-policy behavior on reads, and completion installation examples
without automatically editing shell config.

## 14. Ordered work packages

### WP1 — Command metadata and compatibility fixtures

Freeze current CLI behavior and introduce one internal command metadata source.

### WP2 — Compact projection and list query

Add `CompactPlanSummaryV1`, deterministic filtering, keyset pagination, and
bounded machine output.

### WP3 — Batch status and provider-policy reads

Extend status to bounded explicit ID sets and wire optional provider policy
through show/status/list/registry using one M003a snapshot.

### WP4 — Shell completion generation

Generate Bash/Zsh/Fish/PowerShell completion scripts from command metadata.

### WP5 — Docs, cross-platform qualification, and closure

Re-run golden compatibility, native CI/MSRV, and document the completed CLI
surface.

## 15. Required verification

    cargo fmt --all -- --check
    cargo check --workspace --all-targets --locked
    cargo clippy --workspace --all-targets --locked -- -D warnings
    cargo test --workspace --locked
    cargo +1.89.0 check --workspace --all-targets --locked
    cargo +1.89.0 test --workspace --locked
    bash scripts/check-core-boundary.sh
    bash scripts/check-projection-cli-boundary.sh
    bash scripts/check-closure-authority-boundary.sh
    git diff --check

Run representative completion generation on Linux/macOS/Windows CI even though
the scripts target multiple shells.

## 16. Acceptance criteria

M003b closes when:

1. multi-Plan overview queries are bounded and deterministic;
2. compact machine output is a new explicit projection, not a mutation of
   existing JSON contracts;
3. repository-wide batch execution consumes the M003a snapshot;
4. optional provider policy makes read assessments useful without introducing
   implicit trust;
5. no-policy behavior remains explicit and conservative;
6. shell completions are generated for Bash/Zsh/Fish/PowerShell without
   repository or process side effects;
7. help/validation/completion metadata cannot silently drift into independent
   command inventories;
8. existing single-plan CLI semantics remain compatible;
9. native/MSRV CI passes.

## 17. Stop conditions

Stop and record a corrective/design decision if batch queries require a
daemon/database/global cache; compact mode would require silently changing
existing response fields; completions require executing shell commands or
scanning repository state; parser replacement cannot preserve stable
JSON/error behavior; provider-policy convenience would create implicit/global
provider trust; or a projection needs filesystem/network authority.

## 18. Closure evidence

Record the compact DTO schema fixture set; batch ordering/filter/pagination
matrix; provider-policy/no-policy comparison; completion snapshots for four
shells; unchanged legacy CLI fixtures; M003a snapshot integration proof;
implementation SHA; native/MSRV hosted workflow IDs; and residual ergonomics
findings.
