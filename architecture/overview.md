# Eggplan architecture overview

Eggplan is a repository-local planning and evidence mechanism. It models
bounded plans, derives dependency readiness, normalizes externally acquired
observations into immutable evidence, and assesses closure from structured
state. It does not schedule work, execute verification, fetch evidence,
authenticate producers, or run model reasoning (see
`plans/adrs/ADR-0001-planning-evidence-mechanism-not-execution.md`).

This page is the **birds-eye view**: one screen per discrete component, what
each one owns, the one rule it must never break, and a link to the deep dive
that holds the `file:line` detail. Read this to orient; read a deep dive to
review. Current milestone status lives in `plans/registry.md`, never here.

Workspace: edition 2024, MSRV 1.89, `--locked` everywhere. Seven crates in
`crates/`, five static boundary scripts in `scripts/`, the planning system in
`plans/`, user-facing guides in `docs/`, agent workflow in `AGENTS.md` with
reusable procedures in `.skills/`.

## Component map

| Component | Owns | Must never | Deep dive | Normative |
|---|---|---|---|---|
| `eggplan-core` | Typed IDs, Plan/observation schema v1+v2, bounds, canonical JSON + digests, readiness graph, assessment, closure shapes, supersession lineage | Touch fs, Git, process, net, or a scheduler | [deep-dive-core-domain](deep-dive-core-domain.md) | [core](core.md), [evidence](evidence.md) |
| `eggplan-repo` | `.eggplan/` Plan store with revision CAS, append-only evidence + supersession ledger, Git `SubjectRevision` capture, subject fingerprint, guarded closure finalizer | Let ordinary CAS enter `Closed`, or accept a caller-supplied subject | [deep-dive-repository](deep-dive-repository.md) | [repository](repository.md), [evidence](evidence.md) |
| `eggplan-integrations` | Provider-normalization SPI + Eggwork, Eggsearch, and Eggbench adapters | Acquire evidence, or enroll trust | [deep-dive-integrations](deep-dive-integrations.md) | [provider-spi](provider-spi.md), [eggwork-adapter](eggwork-adapter.md), [eggsearch-adapter](eggsearch-adapter.md), [eggbench-adapter](eggbench-adapter.md) |
| `eggplan-projection` | Reusable derived summaries (registry/status views) over canonical state | Depend on a CLI, terminal, process, or transport | [deep-dive-projection-cli](deep-dive-projection-cli.md) | [cli-control-surface](cli-control-surface.md) |
| `eggplan-cli` | The thin `eggplan` binary: command surface, stable JSON envelope, per-invocation provider policy, read-only `check` | Execute work, accept user-authored evidence, or edit `plans/registry.md` | [deep-dive-projection-cli](deep-dive-projection-cli.md) | [cli-control-surface](cli-control-surface.md) |
| `eggplan-markdown` | Deterministic Markdown v1 render; strict CodeGG-subset import as Draft intent with a loss report | Become canonical state, or import evidence/trust/subject/closure | [deep-dive-markdown](deep-dive-markdown.md) | [markdown-interchange](markdown-interchange.md) |
| `eggplan-codegg-compat` | Pure two-way CodeGG bridge: WorkPlan snapshot mapping, repository-Plan projection, pure assessment | Own CodeGG runtime/storage, or depend on `eggplan-repo` in production | [deep-dive-codegg-compat](deep-dive-codegg-compat.md) | [codegg-compat](codegg-compat.md) |
| scripts + CI | Five static boundary guards, ubuntu/macos/windows matrix, MSRV 1.89 check+test job, the verify order | Drift from the order in `AGENTS.md` / `README.md` | [deep-dive-tooling-governance](deep-dive-tooling-governance.md) | `plans/README.md`, `plans/registry.md` |

Boundary enforcement is static and CI-gated (skipped on Windows runners):
`check-core-boundary.sh`, `check-codegg-compat-boundary.sh`,
`check-integrations-boundary.sh`, `check-projection-cli-boundary.sh`,
`check-closure-authority-boundary.sh`.

## Capabilities, end to end

These are the discrete verbs the system actually performs. Anything not on
this list is a non-goal, not a gap.

1. **Author a bounded plan** — strict Plan JSON via `new --input`, or Markdown
   import as Draft *intent* only. Every field is bounded; unknown fields fail
   closed.
2. **Derive readiness** — pure dependency-graph analysis. `Ready` is a derived
   statement about which items are unblocked, never authority to run them.
3. **Record observations** — externally acquired facts normalized by adapters
   and stored append-only, subject-scoped and digest-protected.
4. **Assess** — pure, deterministic fold of (plan, current subject,
   observations, host registry) into per-criterion, per-item, and per-plan
   statuses with stable reason codes.
5. **Close** — only through `RepositoryStore::finalize_closure`, which
   recaptures the Git subject twice under its own lock before writing.
6. **Project and read** — deterministic derived views (`registry render`,
   `status`, `ready`, `graph`, `check`) for humans and machines.
7. **Interchange** — deterministic Markdown v1 out, bounded CodeGG-subset in,
   and a pure CodeGG WorkPlan bridge both directions.

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

Canonical state lives on disk under the state root (`.eggplan` by convention):

```text
.eggplan/
  .lock                              # process lock for mutations
  config.toml                        # store options
  plans/<plan_id>/
    plan.json                        # canonical Plan, revision-scoped
    evidence/                        # append-only observations
    supersessions/                   # append-only correction lineage
    closure.json                     # promoted immutable closure record
    closure.pending.json             # crash-recovery intermediate
```

## Three invariants hold the design together

1. **Canonical bytes are the contract.** Compact `serde_json` in declared field
   order (`BTreeMap` key order); digests are `sha256:<64 hex>`. Golden fixtures
   freeze bytes — pretty JSON is not the contract. Plan and evidence schemas are
   at version 2; v1 stays frozen and byte-stable, both versions parse, and
   unknown fields are rejected, not an extension point.
2. **Subject equality is applicability.** A passing observation for any other
   source tree is stale history, never current proof. Only the closure
   finalizer speaks for the *current* subject, captured under its own lock.
3. **Trust is host-conferred, never self-asserted.** Provider IDs inside
   observations are labels; only the host-constructed registry decides what
   counts (CLI: an explicit per-invocation policy file, no defaults).

## Deep-dive index (review entry points)

Each deep dive is a code-verified review of one discrete component —
`file:line` references, boundary compliance, test strategy, findings, and
verification pointers:

- [deep-dive-core-domain](deep-dive-core-domain.md) — domain model, canonical encoding, assessment precedence, closure types
- [deep-dive-repository](deep-dive-repository.md) — store layout, CAS, ledger, Git subject, guarded finalization
- [deep-dive-integrations](deep-dive-integrations.md) — SPI contract, Eggwork/Eggsearch mappings, fixture baselines
- [deep-dive-projection-cli](deep-dive-projection-cli.md) — derived views, command surface, policy file, diagnostics
- [deep-dive-markdown](deep-dive-markdown.md) — native v1 grammar, CodeGG subset, loss reporting
- [deep-dive-codegg-compat](deep-dive-codegg-compat.md) — snapshot mapping, assessment bridge, projection binding
- [deep-dive-tooling-governance](deep-dive-tooling-governance.md) — boundary scripts, CI matrix, verify order, planning hygiene

## How to use these documents

1. Start here to pick the component under review.
2. Read that component's deep dive for `file:line` claims and known findings.
3. Fall back to the normative doc in the component map when the two disagree —
   deep dives record known doc/code mismatches as explicit findings.
4. Confirm with the verification pointers at the end of the deep dive, or run
   the whole suite below.

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
bash scripts/check-core-boundary.sh
bash scripts/check-codegg-compat-boundary.sh
bash scripts/check-integrations-boundary.sh
bash scripts/check-projection-cli-boundary.sh
bash scripts/check-closure-authority-boundary.sh
```

Planning authority, in order: `plans/000-long-term-specification.md` →
`plans/001-terminology-and-domain-model.md` → accepted ADRs in `plans/adrs/` →
`plans/002-long-term-roadmap.md` → subsystem roadmaps → implementation plans →
closure evidence in `plans/closure/`. A plan is not evidence that a capability
exists, and a milestone is not closed without its required closure evidence.

These documents describe the design and are authoritative for it. Adopter-facing
guides live in `docs/`; they may simplify but must never contradict what is
written here. Reusable maintenance procedures live in `.skills/`.
