# Eggplan architecture overview

Eggplan is a repository-local planning and evidence mechanism. It models
bounded plans, derives dependency readiness, normalizes externally acquired
observations into immutable evidence, and assesses closure from structured
state. It does not schedule work, execute verification, fetch evidence,
authenticate producers, or run model reasoning (see
`plans/adrs/ADR-0001-planning-evidence-mechanism-not-execution.md`).

Rust workspace (edition 2024, MSRV 1.89): seven crates in `crates/`,
static boundary scripts in `scripts/`, planning system in `plans/`.
Agent workflow lives in `AGENTS.md`.

## Module map

| Crate | Role | Deep dive | Normative doc |
|---|---|---|---|
| `eggplan-core` | Pure deterministic domain: typed IDs, Plan schema v1/v2, bounds, canonical JSON/digests, readiness graph, evidence/assessment/closure semantics. No fs, Git, process, net. | [deep-dive-core-domain](deep-dive-core-domain.md) | [core](core.md), [evidence](evidence.md) |
| `eggplan-repo` | Canonical persistence under `.eggplan/`: Plan store with revision CAS, append-only evidence/supersession ledger, Git `SubjectRevision` capture, guarded closure finalizer (owns subject recapture). | [deep-dive-repository](deep-dive-repository.md) | [repository](repository.md), [evidence](evidence.md) |
| `eggplan-integrations` | Pure provider-normalization SPI plus Eggwork and Eggsearch adapters. Normalizes host-supplied facts into observations; never acquires, never enrolls trust. | [deep-dive-integrations](deep-dive-integrations.md) | [provider-spi](provider-spi.md), [eggwork-adapter](eggwork-adapter.md), [eggsearch-adapter](eggsearch-adapter.md) |
| `eggplan-projection` | Reusable derived summaries (registry/status views) over canonical state. No CLI, repo, terminal, process, or transport deps. | [deep-dive-projection-cli](deep-dive-projection-cli.md) | [cli-control-surface](cli-control-surface.md) |
| `eggplan-cli` | Thin `eggplan` command adapter over core/repo/projection/markdown. Stable `--json` envelope, per-invocation provider policy, read-only `check`. Never executes or acquires evidence. | [deep-dive-projection-cli](deep-dive-projection-cli.md) | [cli-control-surface](cli-control-surface.md) |
| `eggplan-markdown` | Bounded Eggplan Markdown v1 render plus strict CodeGG-subset import as Draft intent with a loss report. Never canonical state; never imports evidence, trust, subject, or closure. | [deep-dive-markdown](deep-dive-markdown.md) | [markdown-interchange](markdown-interchange.md) |
| `eggplan-codegg-compat` | Pure one-way CodeGG WorkPlan snapshot bridge: deterministic ID mapping, host-resolved observations, pure assessment. CodeGG keeps runtime, storage, and WorkOrder ownership. | [deep-dive-codegg-compat](deep-dive-codegg-compat.md) | [codegg-compat](codegg-compat.md) |
| scripts / CI / `plans/` | Five static boundary guards, GitHub CI matrix (ubuntu/macos/windows + MSRV job), and the plan/registry/closure governance process. | [deep-dive-tooling-governance](deep-dive-tooling-governance.md) | `plans/README.md`, `plans/registry.md` |

## How it fits together

```text
author Plan (strict JSON in, or Markdown import as Draft intent)
        │  eggplan-cli ──► eggplan-repo (.eggplan/ store, revision CAS)
        ▼
external facts ──► eggplan-integrations ──► immutable EvidenceObservations
(host acquires)    (pure normalization)      (append-only ledger, subject-scoped)
        │                                           │
        └──────── verification digest binds ────────┘
              requirement ↔ observation matching

assess(plan, current Git subject, observations, host ProviderRegistry)
  ──► pure deterministic assessment (exact subject equality)
        │  ├─► eggplan-projection / CLI show, status, ready, graph, check
        │  └─► ClosureCandidate ──► RepositoryStore::finalize_closure
        │        (recaptures subject twice under lock; ordinary CAS
        │         can never enter Closed; crash-safe pending protocol)
        ▼
codegg-compat: CodeGG snapshots in ──► Eggplan plan + pure assessment
markdown: canonical plans out as Markdown v1; CodeGG Markdown in as intent
```

Three invariants hold the design together:

1. **Canonical bytes are the contract.** Compact `serde_json` in declared
   field order; digests are `sha256:<64 hex>`. Golden fixtures freeze bytes;
   schema v1 is frozen and unknown fields are rejected.
2. **Subject equality is applicability.** A passing observation for any other
   source tree is stale history, never current proof. Only the closure
   finalizer speaks for the *current* subject, captured under its own lock.
3. **Trust is host-conferred, never self-asserted.** Provider IDs in
   observations are labels; only the host-constructed registry (CLI: an
   explicit per-invocation policy file, no defaults) decides what counts.

## Deep-dive index (review entry points)

Each deep dive is a code-verified review of one discrete component —
file:line references, boundary compliance, test strategy, findings, and
verification pointers:

- [deep-dive-core-domain](deep-dive-core-domain.md) — domain model, canonical encoding, assessment, closure types
- [deep-dive-repository](deep-dive-repository.md) — store layout, CAS, ledger, Git subject, guarded finalization
- [deep-dive-integrations](deep-dive-integrations.md) — SPI contract, Eggwork/Eggsearch mappings, fixture baselines
- [deep-dive-projection-cli](deep-dive-projection-cli.md) — derived views, command surface, policy file, diagnostics
- [deep-dive-markdown](deep-dive-markdown.md) — native v1 grammar, CodeGG subset, loss reporting
- [deep-dive-codegg-compat](deep-dive-codegg-compat.md) — snapshot mapping, assessment bridge, M002 status
- [deep-dive-tooling-governance](deep-dive-tooling-governance.md) — boundary scripts, CI matrix, verify order, planning hygiene

Start a discrete review from the applicable deep dive above; fall back to the
normative doc in the module-map table when they disagree (deep dives record
known doc/code mismatches as findings). Current milestone status lives in
`plans/registry.md`, not here.
