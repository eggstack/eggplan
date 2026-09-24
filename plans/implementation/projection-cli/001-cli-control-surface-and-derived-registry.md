# Projection and CLI M001 — Control Surface and Derived Registry

Status: active

Repository baseline: dac415e4e52ba2d3ba8af03c1541a7d5f6e59d76

Implementation commit: cb3a8eb4bceefe7bc5a13fb66b3d638077573cdd (cross-platform snapshot correction in progress)

Source roadmap:

- plans/subsystems/projection-cli-roadmap.md

Long-term requirements:

- plans/000-long-term-specification.md sections 14-15, 19-20
- plans/001-terminology-and-domain-model.md sections 6-9
- plans/002-long-term-roadmap.md Projection/CLI M001

Applicable ADR:

- ADR-0002-repository-first-versioned-canonical-state

Primary class: capability / projection / operator surface

## 1. Objective

Add the first supported human and machine control surface over canonical
Eggplan state without moving authority into CLI parsing or rendered prose.

Create an `eggplan-cli` crate and a small reusable projection layer that can:

- initialize/open a repository state root;
- create and inspect Plans;
- perform ordinary revisioned lifecycle/item mutations;
- render readiness and dependency graphs;
- deeply check repository/evidence/closure integrity;
- inspect evidence and supersession lineage;
- assess a Plan against an explicit provider policy;
- perform guarded closure through Evidence M002;
- render a derived runtime registry;
- emit stable bounded JSON for automation.

Markdown import/render remains M002 of this subsystem.

## 2. Dependency gate

Evidence M002 is closed at the recorded repository baseline.

The CLI must expose the guarded closure contract, immutable ClosureRecord,
supersession lineage, and deep integrity checks rather than inventing a
temporary close path that would immediately become incompatible.

## 3. Ownership and security boundaries

- CLI is a thin adapter over eggplan-core/eggplan-repo APIs.
- No command execution, scheduler, network client, MCP server, or model runtime.
- CLI input may author Plan intent but cannot self-authorize passing evidence.
- There is no `evidence add --status passed` escape hatch for arbitrary user
  text.
- Trusted ProviderRegistry construction for `assess`/`close` must come from
  an explicit versioned provider-policy input supplied by the operator/host.
- A provider-policy file contains authority metadata only, never credentials.
- JSON output is bounded and versioned.
- Human output may abbreviate, but must surface truncation and never hide
  failed/unavailable/stale closure state.

## 4. Proposed command surface

Exact flag spelling may be refined, but capability boundaries must remain:

    eggplan init [--state-root PATH]
    eggplan new --input PLAN.json
    eggplan show PLAN_ID
    eggplan status [PLAN_ID]
    eggplan ready PLAN_ID
    eggplan graph PLAN_ID
    eggplan check [PLAN_ID]
    eggplan activate PLAN_ID --expected-revision N
    eggplan item update PLAN_ID ITEM_ID --expected-revision N ...
    eggplan evidence list PLAN_ID
    eggplan evidence show PLAN_ID EVIDENCE_ID
    eggplan evidence supersessions PLAN_ID
    eggplan assess PLAN_ID --provider-policy POLICY.json
    eggplan close PLAN_ID --expected-revision N --provider-policy POLICY.json
    eggplan closure show PLAN_ID
    eggplan registry render

All read-oriented commands support `--json`. Mutating commands return a
versioned machine envelope under `--json`.

Do not add arbitrary shell execution to produce evidence.

## 5. Plan creation and mutation

### New

`new --input` consumes a strict Eggplan Plan JSON document or a bounded
PlanDraft DTO and creates revision zero through RepositoryStore. If a draft DTO
is introduced, it must have its own explicit schema version and must not
become a second persisted Plan schema.

### Activation and item mutation

Expose ordinary legal transitions through CAS. The caller supplies the expected
Plan revision. Item mutations load the Plan, mutate one bounded item field/set,
increment the Plan revision exactly once, validate, and use store CAS.

Do not expose ordinary mutation to `Closed`; closure must route through
Evidence M002.

## 6. Provider-policy input

Define a CLI-only/provider-neutral strict JSON DTO that can build a
ProviderRegistry for one invocation. It should contain only:

- schema version;
- provider ID;
- provider class;
- allowed evidence kinds.

Validate bounds and duplicate IDs. Unknown fields/versions fail closed.

No implicit trust defaults. If `assess` or `close` needs provider authority
and no policy is supplied, return a typed diagnostic rather than guessing.

The CLI policy format is an input surface, not a repository-global trust store.

## 7. Machine output contract

Add a versioned bounded JSON envelope, for example:

    {
      "schema_version": 1,
      "command": "status",
      "ok": true,
      "data": { ... },
      "warnings": []
    }

Requirements:

- deterministic object/array ordering where semantic ordering exists;
- stable reason/status codes from core rather than reparsed human strings;
- bounded warnings/errors;
- no secret-bearing Debug output;
- nonzero exit code for command failure;
- `check` distinguishes invalid, corrupt, recovery-required, stale,
  unavailable, and ordinary incomplete state.

Freeze representative JSON fixtures before closure.

## 8. Derived projections

### Status

Show Plan revision/status, current subject, item counts, current assessment,
closure presence, and concise reason codes.

### Ready

Use core dependency readiness only. Do not reinterpret model prose or
`next_action` as readiness authority.

### Graph

Render a bounded dependency/parent graph in human text and JSON. A cycle or
dangling reference is an error, not a best-effort graph.

### Check

Perform deep read-only verification over:

- repository config/layout;
- Plan envelope/schema/digest;
- evidence observations;
- supersession lineage;
- ClosureRecord and pending-recovery state;
- Plan/closure consistency.

No automatic destructive repair in M001. If Evidence M002 exposes a safe,
idempotent pending-closure recovery routine, `check` may report that routine's
result only when explicitly requested through a narrowly named recovery flag.

### Registry render

Derive a compact runtime registry from canonical .eggplan objects: plan IDs,
revisions/status, readiness/assessment summary, closure state, and blockers.

This is NOT the development repository's hand-maintained
`plans/registry.md`. Do not overwrite development planning Markdown.

## 9. Bounded reusable projection API

Place projection/summary DTOs in a reusable library layer rather than
hard-coding them into terminal formatting. This layer should be usable later
by CodeGG or an MCP/service adapter without depending on clap or terminal IO.

Keep full canonical Plan/Evidence objects separate from bounded projection
DTOs. Projection truncation must report counts/truncation explicitly.

## 10. Cross-platform behavior

- Paths use PathBuf and existing repository safety semantics.
- Human output must not depend on ANSI color for correctness.
- JSON must be identical across supported OSes for equivalent logical state.
- stdout carries requested output; stderr carries diagnostics.
- no pager or interactive prompt is required for M001.

## 11. Ordered work packages

### WP1 — Projection DTOs and JSON envelopes

Add bounded reusable summaries, stable reason codes, and golden JSON fixtures.

### WP2 — eggplan-cli bootstrap/read commands

Implement init/show/status/ready/graph/check/evidence/closure/registry reads.

### WP3 — Revisioned mutations

Implement new/activate/item update with explicit expected revision and conflict
diagnostics.

### WP4 — Assessment and guarded close

Parse explicit provider policy, call core assessment, and route close only
through Evidence M002.

### WP5 — Qualification and docs

Native CLI integration tests, help snapshots, README usage, roadmap/registry
closure updates.

## 12. Required tests

At minimum:

- init is idempotent for a valid repository and rejects unsafe roots;
- new creates exact revision-zero Plan;
- stale mutation returns a conflict and does not overwrite;
- illegal transition diagnostics preserve typed reason;
- ready ordering matches core;
- graph is deterministic and bounded;
- check detects plan/evidence/supersession/closure corruption;
- evidence listing cannot mutate observations;
- provider-policy unknown/duplicate/malformed entries fail;
- no implicit provider trust;
- assess JSON matches core assessment;
- close succeeds only through guarded close;
- close with missing/stale provider policy or subject fails;
- runtime registry is derived, deterministic, and never edits
  plans/registry.md;
- JSON fixtures stable across Linux/macOS/Windows;
- output truncation is explicit;
- Rust 1.89 and native CI pass.

## 13. Required verification

    cargo fmt --all -- --check
    cargo check --workspace --all-targets --locked
    cargo clippy --workspace --all-targets --locked -- -D warnings
    cargo test --workspace --locked
    cargo +1.89.0 check --workspace --all-targets --locked
    cargo +1.89.0 test --workspace --locked
    bash scripts/check-core-boundary.sh
    git diff --check

Also run representative CLI smoke tests on all native CI runners.

## 14. Acceptance criteria

M001 closes when a human or automation client can safely inspect and perform
ordinary Eggplan operations without editing canonical files directly; machine
JSON is versioned/bounded; provider trust stays explicit; and closure cannot be
bypassed through CLI mutation.

## 15. Stop conditions

Stop and report if:

- the CLI needs to become an executor or network provider;
- assessment would require implicit trust of provider IDs found in evidence;
- a command requires editing canonical JSON directly rather than a library API;
- Markdown import is required to complete M001;
- projection behavior would create a second source of domain truth.

## 16. Closure evidence required

Record:

- command/help surface;
- JSON fixture inventory;
- CAS/conflict tests;
- corruption/check matrix;
- provider-policy authority tests;
- guarded-close CLI tests;
- cross-platform workflow IDs;
- proof development `plans/registry.md` is untouched by runtime registry
  rendering.
