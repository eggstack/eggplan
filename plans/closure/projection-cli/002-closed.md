# Projection and CLI M002 — Closed

Status: closed

Source implementation plan:

- plans/implementation/projection-cli/002-loss-aware-markdown-import-and-deterministic-render.md

Source roadmap:

- plans/subsystems/projection-cli-roadmap.md

Reviewed baseline: `bfbec4a77d1e35e0a422af1edc9c3f358f7ad982`

Implementation commits:

- `d4a19a71f467b848e677a0b9d1b5b0250f10bc86` — native Markdown interchange,
  CodeGG subset, CLI, fixtures, and documentation.
- `1f4c63ce522e686fb57be374c8e44d5c2616a96f` — exact loss-code regression.

Hosted qualification: GitHub Actions run
[36036043263](https://github.com/eggstack/eggplan/actions/runs/36036043263)

## Executive finding

Eggplan now renders deterministic native Markdown v1 and imports native plan
intent plus a strict CodeGG implementation-plan subset with explicit loss
reporting. The new `eggplan-markdown` crate owns grammar/rendering; the CLI
provides read-only render/inspect and explicit-state-root Draft import. The
Markdown path creates no evidence, provider trust, SubjectRevision authority,
or closure. All required local and hosted verification passed.

## Requirement-to-evidence matrix

| Requirement | Evidence |
|---|---|
| Deterministic native v1 render and intent roundtrip | `eggplan-markdown` tests render identical input identically and roundtrip IDs/order, parent/dependency links, descriptions, criteria/requirements, blocker and next-action data. CRLF Markdown imports. Schema-v1 unbound execution requirements remain schema v1 and preserve intent. |
| Reset imported lifecycle and preserve provenance | Native Draft/Active/Blocked/Closed source Plans import as Draft/revision 0; source status/revision are report/provenance only. CodeGG `Status: implemented` never creates a completed or closed Plan. Imported completed item labels reset to Pending. |
| Supported CodeGG subset and current fixture baseline | Three source fixtures and their manifest are pinned to `dbowm91/codegg` commit `5f4532659dbf0df2cd9f2b3bdb024217d2ea7868`: ordinary implementation plan, corrective plan with Findings/Work packages/Binary acceptance criteria, and a plan with unsupported sections. Deterministic generated IDs and explicit dependency references are tested. |
| No inferred dependency edges; acceptance intent mapping | Ordinary fixture work packages have no edges from order. Plan-level acceptance maps to a final intent item; explicitly scoped acceptance maps to its named item. Unknown or malformed references fail. |
| Bounded, deterministic loss report | Tests assert exact stable warning/loss codes, including source lifecycle/revision, evidence/closure omission, order semantics, generated IDs, unmapped sections, and `text_truncated`; reports identify dropped sections and truncation. Input, line, section, text, item, and source-name bounds are enforced. |
| No evidence/closure/provider/subject authority from Markdown | Native payload schema contains plan intent only. Negative tests include “tests passed,” fake ClosureRecord JSON, status claims, fenced content, and fake native markers inside fences; they prove no subject or evidence requirements are manufactured from prose. CLI import leaves observations and closure absent. |
| CLI render/inspect/import contract | Integration tests exercise JSON render, output-file render, human and JSON inspect, explicit target root, Draft/revision-zero import, inspection without state mutation, no observations/closure, and collision rejection with repository contents unchanged. |
| No network/process parser dependency | `eggplan-markdown` depends only on `eggplan-core`, serde, serde_json, and sha2. The projection/CLI boundary guard covers the Markdown crate; no AST, process, transport, runtime, or persistence dependency was introduced. |

## Exact local verification

All commands ran on Linux at final implementation commit `1f4c63ce522e686fb57be374c8e44d5c2616a96f` and passed:

```text
rtk cargo fmt --all -- --check
rtk cargo check --workspace --all-targets --locked
rtk cargo clippy --workspace --all-targets --locked -- -D warnings
rtk cargo test --workspace --locked                 # 118 passed
rtk cargo test --workspace --doc --locked           # 3 compile-fail doctests passed
rtk cargo +1.89.0 check --workspace --all-targets --locked
rtk cargo +1.89.0 test --workspace --locked         # 118 passed
rtk bash scripts/check-core-boundary.sh
rtk bash scripts/check-projection-cli-boundary.sh
rtk bash scripts/check-closure-authority-boundary.sh
rtk git diff --check
```

Hosted run `36036043263` passed:

- Linux job `107756268846`: format, check, clippy, tests, and all five boundary
  guards.
- macOS job `107756268791`: check, clippy, tests, and all five boundary
  guards; formatting is skipped by workflow policy on macOS.
- Windows job `107756268906`: check, clippy, tests, and all five boundary
  guards; formatting is skipped by workflow policy on Windows.
- Rust 1.89 job `107756268481`: workspace check and tests.

## Invariant, failure, and compatibility review

- Eggplan remains a planning/evidence mechanism; the parser never executes
  code, fetches URLs/includes, or opens paths named by Markdown.
- Repository JSON remains canonical. Markdown is rendered projection or
  Draft-intent import and is not persisted as a second store.
- Import parses and validates before opening the destination repository;
  creation goes through repository APIs, starts at revision zero/Draft, rejects
  ID collisions, and writes no evidence, provider policy, subject, or closure.
- Invalid UTF-8, NUL, oversized input/lines, malformed canonical JSON,
  unsupported native versions, duplicate payloads, ambiguous sections,
  malformed dependencies, invalid Plan graphs, and out-of-bound text fail
  explicitly.
- No schema migration is needed. Native Plan schema version is retained where
  required to preserve valid legacy v1 intent; Markdown format version is
  independently fixed at v1.
- The parser does not use a general Markdown AST. The supported grammar is
  intentionally line-oriented and unknown sections are reported as dropped.

## Documentation and roadmap disposition

Added `architecture/markdown-interchange.md`; updated the CLI architecture,
README, command help snapshot, boundary guard, subsystem roadmap, registry, and
implementation plan.

M002 is closed. M003 ergonomics/performance remains roadmap-level until real
repositories use the Markdown surface. Eggstack M002 remains ready and
independent; its sibling interfaces must be rechecked at that next handoff.
CodeGG M002 remains blocked only by its separately registered upstream
attempt-scoped provenance closure. Interoperability/distribution remains
deferred while Eggstack M002 is outstanding.

## Unresolved findings

None.
