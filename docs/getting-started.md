# Getting started

Requires Rust 1.89 or newer. The workspace builds with `--locked`; always pass it.

```sh
cargo build --locked -p eggplan-cli      # builds target/debug/eggplan
```

The examples below use `eggplan` directly. During development, prefix with
`cargo run -p eggplan-cli --`.

Copy the two example files from [`examples/`](examples) into your working directory, or
keep pointing `--input` at them in place.

## Before you start: Eggplan needs a Git subject

Eggplan assesses evidence against the **current Git subject** — revision, dirty state, and
dirty digest. A repository with no commits has an unborn branch, and anything that captures
a subject fails with `subject_unavailable`:

```sh
eggplan assess ep_release --state-root .eggplan --provider-policy policy.json
# assess: subject_unavailable: Git operation failed: reference 'refs/heads/master' not found
```

So make at least one commit first. `init`, `new`, `show`, and `status` work without one;
`assess` and `close` do not.

```sh
git init && git add -A && git commit -m "init"
```

## Create a state root

Canonical state lives in a **state root**, conventionally `.eggplan`, held outside version
control. The path is never implicit — pass `--state-root` explicitly on every invocation.

```sh
eggplan init --state-root .eggplan
# Initialized .eggplan
```

## Create a plan

Two routes.

**Strict Plan JSON** gives full control. New plans are written at schema version 2:

```sh
eggplan new --input examples/plan.json --state-root .eggplan
# Created ep_release at revision 0
```

Plan IDs are typed and prefixed: `ep_` for plans, `epi_` for items, `epc_` for criteria.
Re-using an existing plan ID fails with `plan_exists`.

**Markdown import** is the bounded path. It reads intent and creates **one Draft plan** —
never evidence, provider trust, a subject revision, or a closure record:

```sh
eggplan markdown import plan.md --state-root .eggplan --json
```

Inspect first to see what would be created, plus a loss report, without mutating anything:

```sh
eggplan markdown inspect plan.md --format auto --json
```

## Work the plan

```sh
eggplan activate ep_release --state-root .eggplan --expected-revision 0
# ep_release active at revision 1

eggplan ready   ep_release --state-root .eggplan
eggplan status  --state-root .eggplan
eggplan graph   ep_release --state-root .eggplan
```

`ready` lists items whose dependencies are satisfied. It is a **derived statement, not
permission to start** — Eggplan never schedules or executes anything.

### Item transitions are constrained

You must walk the state machine rather than jumping to a terminal-ish state:

```
pending → actionable → in_progress → completed
```

Both steps are required. Jumping straight to `in_progress` fails:

```sh
eggplan item update ep_release epi_tests --state-root .eggplan \
  --expected-revision 1 --status in_progress
# item update: invalid_transition: plan or item transition is not allowed
```

The legal transitions are `pending → actionable|blocked|cancelled`,
`actionable → in_progress|blocked|cancelled`, `in_progress → actionable|blocked|completed|cancelled`,
and `blocked → pending|actionable|cancelled`.

Progress an item one field at a time:

```sh
eggplan item update ep_release epi_tests --state-root .eggplan \
  --expected-revision 1 --status actionable
eggplan item update ep_release epi_tests --state-root .eggplan \
  --expected-revision 2 --status in_progress
```

## Mutations are revision-guarded

Every mutating command requires `--expected-revision N`, which must match the plan's
current revision. If someone else advanced the plan in between, the command fails instead
of clobbering their work. Read the current revision with `show` or `status`.

Note that each successful mutation increments the revision, so the expected revision
advances by one per command.

## Check state at any time

```sh
eggplan check --state-root .eggplan --json
```

`check` is read-only. Pass a plan ID to check one plan, or omit it to check all. It
validates the repository, plans, evidence, supersessions, closure records, pending
transactions, and staging state, and reports a per-plan state. It never recovers a pending
closure transaction unless you pass `--recover-pending` explicitly.

## Render for humans

```sh
eggplan registry render --state-root .eggplan --json
eggplan markdown render ep_release --output plan.md
# Wrote plan.md
```

`registry render` derives a view from canonical state. It **never** edits this repository's
`plans/registry.md`.

Note that Markdown is a lossy interchange format: re-importing a rendered plan resets item
and plan status to Draft/pending, and the import report lists that as a `lossy_mappings`
entry. Round-tripping preserves intent, not progress.

## Assess and close

```sh
eggplan assess ep_release --state-root .eggplan --provider-policy examples/policy.json
# EvidenceMissingOrUnavailable

eggplan close ep_release --state-root .eggplan \
  --expected-revision 3 --provider-policy examples/policy.json
# close: closure_not_ready: closure requires complete assessment for exact plan revision and subject
```

Both commands work correctly here — but the result is not `complete`, because **the CLI
cannot record evidence.** See below.

## The CLI cannot record evidence

This is the most common surprise. The `evidence` command is read-only: it offers `list`,
`show`, and `supersessions`, and nothing else.

```sh
eggplan evidence add ep_release --state-root .eggplan
# evidence: usage: expected evidence list, show, or supersessions
```

A host program must actually run the verification, normalize the results through
`eggplan-integrations`, and append observations to the append-only ledger in
`eggplan-repo`. Until that happens, `assess` reports
`evidence_missing_or_unavailable` and `close` reports `closure_not_ready`.

This is deliberate. The CLI never accepts user-authored passing evidence, and never accepts
a caller-supplied current subject. Trust has to be established by a host that observed the
run. See [Evidence and closure](evidence-and-closure.md).

## Next steps

- Every command and flag: [CLI reference](cli-reference.md)
- The policy `assess` and `close` require: [Provider policy](provider-policy.md)
- How the guarded finalizer works: [Evidence and closure](evidence-and-closure.md)
