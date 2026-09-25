# Agent instructions

## Planning hierarchy (read before substantial work)

Read `plans/README.md`, `plans/registry.md`, the applicable subsystem roadmap under `plans/subsystems/`, and the registered implementation plan under `plans/implementation/` before implementing.

Authority order: `plans/000-long-term-specification.md` → `plans/001-terminology-and-domain-model.md` → accepted ADRs in `plans/adrs/` → `plans/002-long-term-roadmap.md` → subsystem roadmaps → implementation plans → closure evidence in `plans/closure/`.

Do not treat a plan as evidence that a capability exists. Do not mark a milestone closed without the closure evidence its implementation plan requires. Record failed, blocked, skipped, unavailable, and unrun verification truthfully.

Eggplan is a planning/evidence mechanism, not a scheduler, process executor, CI system, issue tracker, or model reasoning store. Preserve that boundary.

## Architecture index (check before inferring design)

- `architecture/overview.md` — crate map and product boundary; start here.
- `architecture/core.md` + `deep-dive-core-domain.md` — IDs, bounds, canonical JSON/digest rules.
- `architecture/repository.md` + `deep-dive-repository.md` — store, CAS, Git subject, closure finalizer ownership.
- `architecture/evidence.md` — ledger, assessment, verification binding.
- `architecture/provider-spi.md` + `eggwork-adapter.md` + `eggsearch-adapter.md` (+ `deep-dive-integrations.md`) — normalization contract; adapters never acquire evidence or enroll trust.
- `architecture/cli-control-surface.md` + `deep-dive-projection-cli.md` — commands, JSON envelope, check states, provider policy.
- `architecture/markdown-interchange.md` + `deep-dive-markdown.md` — Markdown v1 grammar, CodeGG subset, loss codes.
- `architecture/codegg-compat.md` + `deep-dive-codegg-compat.md` — pure bridge contract; WorkOrder/scheduler stay CodeGG-owned.
- `architecture/deep-dive-tooling-governance.md` — what each boundary script enforces and CI matrix consequences.
- No `.skills/`, `docs/`, or repo-local agent config exists; `plans/` is the only planning system. Do not invent one.

## Workspace

Rust workspace (edition 2024, MSRV 1.89, `--locked` everywhere): `eggplan-core` (pure deterministic domain, no fs/Git/process/net), `eggplan-repo` (Plan store, CAS, evidence ledger, Git subject, guarded closure finalizer), `eggplan-integrations` (pure provider-normalization SPI, no acquisition), `eggplan-projection` (reusable derived summaries), `eggplan-markdown` (bounded intent import/render), `eggplan-cli` (thin `eggplan` adapter, no execution/evidence acquisition). `eggplan-codegg-compat` is a pure bridge: no `eggplan-repo` or CodeGG dependency, no WorkOrder/scheduler ownership.

Static boundary scripts in `scripts/` are CI-gated (skipped on Windows runners): `check-core-boundary.sh`, `check-codegg-compat-boundary.sh`, `check-integrations-boundary.sh`, `check-projection-cli-boundary.sh`, `check-closure-authority-boundary.sh`. Keep new code inside these boundaries (e.g. no `tokio`/`reqwest`/`std::process::Command` in core/projection/markdown/CLI; adapters must not enroll trust via `register_trusted`/`ProviderRegistry`; never expose `SubjectCapture` or `finalize_closure_with_*` publicly).

## Verify (in this order)

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

This matches `.github/workflows/ci.yml` and the `README.md` dev-checks block exactly. CI runs the same on ubuntu/macos/windows plus an MSRV (1.89.0) `check`+`test`-only job. `fmt` runs Linux-only and all boundary scripts are skipped on Windows runners.

Focused: `cargo test -p <crate>`; `cargo test -p <crate> <filter>`. Run the CLI via `cargo run -p eggplan-cli -- <args>` (binary is `eggplan`).

## CLI / domain gotchas

- Canonical JSON is compact `serde_json` in declared field order (`BTreeMap` key order); digests are `sha256:<64 hex>`. Golden fixtures under `crates/eggplan-core/tests/fixtures/` freeze bytes (most with `.sha256` sidecars) — pretty JSON is not the contract. Schema v1 is frozen; unknown fields are rejected, not an extension point.
- CLI: pass `--state-root .eggplan` explicitly; mutations need `--expected-revision`; `assess`/`close` need an explicit provider-policy JSON file (provider IDs/classes/allowed kinds only, no credentials, no default trusted provider). The CLI never accepts user-authored passing evidence.
- `check` is read-only; pending-closure recovery needs explicit `--recover-pending`. `check` reports 10 states including `inconclusive`, plus check-time `closure_subject_stale` when a stored closure record's subject differs from current. `registry render` never edits `plans/registry.md`. `markdown import` creates one Draft intent only — never evidence, provider trust, SubjectRevision, or closure.
- `RepositoryStore::finalize_closure` owns current-subject recapture under its lock and takes no caller-supplied subject; subject mismatch/drift/unavailable are typed `RepoError`s, not lifecycle errors.
