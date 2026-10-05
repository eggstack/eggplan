# eggplan

Eggplan is a repository-local planning and evidence mechanism. It models bounded plans,
derives dependency readiness, records observations through provider adapters, and assesses
closure from structured state.

It does **not** schedule or execute work, run your tests, or store model reasoning.

Requires Rust 1.89+.

```sh
cargo build --locked -p eggplan-cli      # builds target/debug/eggplan
```

## Quickstart

Eggplan lives in a Git repository — it assesses evidence against your current Git subject,
so the repository needs at least one commit.

```sh
git init && git add -A && git commit -m "init"     # eggplan needs a resolvable subject
```

Create a state root and a plan from strict Plan JSON:

```sh
eggplan init  --state-root .eggplan
eggplan new   --input docs/examples/plan.json --state-root .eggplan
# Created ep_release at revision 0
```

Activate it and start working. Every mutation is revision-guarded, so pass
`--expected-revision` with the plan's current revision:

```sh
eggplan activate ep_release --state-root .eggplan --expected-revision 0
# ep_release active at revision 1

eggplan item update ep_release epi_tests --state-root .eggplan --expected-revision 1 --status actionable
eggplan item update ep_release epi_tests --state-root .eggplan --expected-revision 2 --status in_progress
```

Item transitions are constrained — you must walk `pending → actionable → in_progress →
completed`. Jumping straight to `in_progress` fails with `invalid_transition`.

Then look around:

```sh
eggplan check    --state-root .eggplan --json    # read-only validation
eggplan status   --state-root .eggplan
eggplan ready    ep_release --state-root .eggplan
eggplan graph    ep_release --state-root .eggplan
eggplan registry render --state-root .eggplan --json
```

Round-trip through Markdown:

```sh
eggplan markdown render  ep_release --output plan.md
eggplan markdown inspect plan.md --format auto --json      # loss report, no mutation
eggplan markdown import  plan.md --state-root .other      # one Draft plan
```

## You cannot close a plan from the CLI alone

This is the most common surprise. **The CLI cannot record evidence** — `evidence` only
reads (`list`, `show`, `supersessions`). A host program must run the tests, normalize the
results through `eggplan-integrations`, and append observations to the ledger in
`eggplan-repo`. Only then can `assess` and `close` succeed:

```sh
eggplan assess ep_release --state-root .eggplan --provider-policy docs/examples/policy.json
# EvidenceMissingOrUnavailable     <- until a host has written observations

eggplan close  ep_release --state-root .eggplan --expected-revision 3 --provider-policy docs/examples/policy.json
# close: closure_not_ready: closure requires complete assessment for exact plan revision and subject
```

`assess` and `close` both require an explicit provider-policy file, because trust is
host-conferred, never self-asserted. There is no default trusted provider.

## Documentation

| Guide | Covers |
|---|---|
| [Getting started](docs/getting-started.md) | The full first-run walkthrough |
| [CLI reference](docs/cli-reference.md) | Every command, flag, status vocabulary, and check state |
| [Evidence and closure](docs/evidence-and-closure.md) | How observations become proof; how closure is guarded |
| [Provider policy](docs/provider-policy.md) | Writing the trust policy `assess` and `close` require |
| [Architecture overview](architecture/overview.md) | Crate map and design invariants |

Design rationale lives in `architecture/`. Planning lives in `plans/`, where
`plans/registry.md` is the only record of current milestone status. Agent and maintainer
workflow lives in `AGENTS.md` and `.skills/`.

## Workspace

| Crate | Role |
|---|---|
| `eggplan-core` | Pure deterministic domain: IDs, bounds, canonical JSON, digests, assessment |
| `eggplan-repo` | Plan store, revision CAS, evidence ledger, Git subject, guarded closure finalizer |
| `eggplan-integrations` | Provider-normalization SPI plus Eggwork/Eggsearch adapters |
| `eggplan-projection` | Reusable derived summaries |
| `eggplan-markdown` | Bounded, loss-aware Markdown import and deterministic render |
| `eggplan-cli` | The thin `eggplan` binary |
| `eggplan-codegg-compat` | Pure two-direction CodeGG bridge; owns no repository I/O |

## Development

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

The five boundary scripts are static guards that keep each crate inside its architecture
boundary. They are CI-gated and skipped on Windows runners, and they need `rg`, `awk`, and
`python3`. Install ripgrep locally — a missing `rg` makes every guard exit **green without
scanning anything**.
