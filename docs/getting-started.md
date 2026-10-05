# Getting started

Requires Rust 1.89 or newer. The workspace builds with `--locked`; always pass it.

## Build

```sh
cargo build --locked -p eggplan-cli
```

The binary is named `eggplan`. During development, run it through Cargo instead:

```sh
cargo run -p eggplan-cli -- <args>
```

## Create a state root

Eggplan keeps canonical state in a **state root**, conventionally `.eggplan`, held outside
version control. The path is never implicit — pass `--state-root` explicitly on every
invocation.

```sh
cargo run -p eggplan-cli -- init --state-root .eggplan
```

## Create a plan

Two routes.

**Strict Plan JSON** gives full control. New plans are written at schema version 2:

```sh
cargo run -p eggplan-cli -- new --input plan.json --state-root .eggplan
```

**Markdown import** is the bounded path. It reads intent and creates **one Draft plan** —
never evidence, provider trust, a subject revision, or a closure record:

```sh
cargo run -p eggplan-cli -- markdown import plan.md --state-root .eggplan --json
```

Inspect first if you want to see what would be created, plus a loss report, without
mutating anything:

```sh
cargo run -p eggplan-cli -- markdown inspect plan.md --format auto --json
```

## Work the plan

```sh
cargo run -p eggplan-cli -- activate ep_example --state-root .eggplan --expected-revision 0
cargo run -p eggplan-cli -- ready ep_example --state-root .eggplan
cargo run -p eggplan-cli -- status ep_example --state-root .eggplan
cargo run -p eggplan-cli -- graph ep_example --state-root .eggplan
```

`ready` lists items whose dependencies are satisfied. It is a **derived statement, not
permission to start** — Eggplan never schedules or executes anything.

Progress an item with `item update`, changing one field at a time:

```sh
cargo run -p eggplan-cli -- item update ep_example epi_example --state-root .eggplan \
  --expected-revision 1 --status in_progress
```

## Mutations are revision-guarded

Every mutating command requires `--expected-revision N`, which must match the plan's
current revision. If someone else advanced the plan in between, the command fails instead
of clobbering their work. Read the current revision with `show` or `status`.

## Check state at any time

```sh
cargo run -p eggplan-cli -- check --state-root .eggplan --json
```

`check` is read-only. It validates the repository, plans, evidence, supersessions,
closure records, pending transactions, and staging state, and reports a per-plan state.
It never recovers a pending closure transaction unless you pass `--recover-pending`
explicitly.

## Render for humans

```sh
cargo run -p eggplan-cli -- registry render --state-root .eggplan --json
cargo run -p eggplan-cli -- markdown render ep_example --output plan.md
```

`registry render` derives a view from canonical state. It **never** edits this
repository's `plans/registry.md`.

## Next steps

- Every command and flag: [CLI reference](cli-reference.md)
- Turning observations into proof and closing a plan: [Evidence and closure](evidence-and-closure.md)
- The trust policy `assess` and `close` demand: [Provider policy](provider-policy.md)
