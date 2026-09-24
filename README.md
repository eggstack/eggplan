# eggplan

Eggplan is a repository-local planning and evidence mechanism. It models
bounded plans, derives dependency readiness, records observations through
future provider adapters, and assesses closure from structured state. It does
not schedule or execute work.

The Rust workspace contains `eggplan-core`, a dependency-light domain library
with immutable evidence and deterministic assessment; `eggplan-repo`, the
local Plan store, append-only evidence ledger, and Git subject adapter;
`eggplan-integrations`, a provider-neutral evidence normalization SPI;
`eggplan-projection`, bounded derived summaries; and `eggplan-cli`, the
`eggplan` command-line control surface. `eggplan-markdown` provides bounded,
loss-aware Markdown intent import and deterministic rendering. See
[core architecture](architecture/core.md),
[repository architecture](architecture/repository.md), the
[evidence architecture](architecture/evidence.md), the
[provider SPI](architecture/provider-spi.md), and the
[CLI control surface](architecture/cli-control-surface.md).
The [Markdown interchange guide](architecture/markdown-interchange.md)
documents the native format and supported CodeGG subset.

## CLI quick start

```sh
cargo run -p eggplan-cli -- init --state-root .eggplan
cargo run -p eggplan-cli -- registry render --state-root .eggplan --json
cargo run -p eggplan-cli -- check --state-root .eggplan --json
cargo run -p eggplan-cli -- markdown render ep_example --output plan.md
cargo run -p eggplan-cli -- markdown inspect plan.md --format auto --json
cargo run -p eggplan-cli -- --state-root .eggplan markdown import plan.md --json
```

Plans can be created from strict Eggplan Plan JSON with `new --input PLAN.json`
or imported as bounded Draft intent with `markdown import FILE --state-root PATH`.
Mutations require `--expected-revision`. `assess` and guarded `close` require
an explicit provider-policy JSON file; policy files contain provider IDs,
classes, and allowed evidence kinds only. The CLI never accepts user-authored
passing evidence. `check` is read-only by default; pending closure recovery
requires the explicit `--recover-pending` flag.

Development checks (Rust 1.89 or newer):

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
bash scripts/check-core-boundary.sh
bash scripts/check-projection-cli-boundary.sh
```
