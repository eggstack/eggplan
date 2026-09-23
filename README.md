# eggplan

Eggplan is a repository-local work-order and evidence engine.

It is intended to make software planning and closure machine-checkable without
turning planning into a scheduler or an AI-specific workflow runtime. Eggplan
owns typed plans, dependency/readiness state, acceptance requirements,
revision-scoped evidence, deterministic closure assessment, and derived
human-readable projections.

The repository is currently in planning/bootstrap state. Start with:

- `plans/000-long-term-specification.md`
- `plans/001-terminology-and-domain-model.md`
- `plans/002-long-term-roadmap.md`
- `plans/003-planning-process.md`
- `plans/registry.md`

The first implementation handoff is registered in `plans/registry.md`.

Eggplan is designed to remain usable independently, while providing clean
integration boundaries for CodeGG, Eggwork, Eggsearch, Eggbench, and other
Eggstack projects.
# Eggplan

Eggplan is a repository-local planning and evidence mechanism. It models
bounded plans, derives dependency readiness, records observations through
future provider adapters, and assesses closure from structured state. It does
not schedule or execute work.

The initial Rust workspace contains `eggplan-core`, a dependency-light domain
library. See [core architecture](architecture/core.md) and the planning
registry in `plans/registry.md`.

Development checks:

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
bash scripts/check-core-boundary.sh
```
