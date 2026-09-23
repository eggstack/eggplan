# eggplan

Eggplan is a repository-local planning and evidence mechanism. It models
bounded plans, derives dependency readiness, records observations through
future provider adapters, and assesses closure from structured state. It does
not schedule or execute work.

The initial Rust workspace contains `eggplan-core`, a dependency-light domain
library with immutable evidence and deterministic assessment, plus
`eggplan-repo`, the local Plan store, append-only evidence ledger, and Git
subject adapter. See
[core architecture](architecture/core.md),
[repository architecture](architecture/repository.md), and the planning
registry in `plans/registry.md`.

Development checks (Rust 1.89 or newer):

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
bash scripts/check-core-boundary.sh
```
