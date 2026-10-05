# CLI reference

The `eggplan` binary is a thin command adapter. It does not execute work, acquire evidence,
or edit the development planning registry.

Every command accepts `--json` for a stable machine-readable envelope.

## Global flags

| Flag | Meaning |
|---|---|
| `--state-root PATH` | Location of canonical state (conventionally `.eggplan`). Never implicit. |
| `--json` | Emit the JSON envelope instead of human output. |
| `--help`, `-h` | Print usage. |

Value-taking flags: `--state-root`, `--input`, `--expected-revision`,
`--provider-policy`, `--status`, `--blocker`, `--next-action`, `--output`, `--format`.

## Read commands

| Command | Purpose |
|---|---|
| `show PLAN_ID` | Canonical plan state |
| `status [PLAN_ID]` | Derived status; omit the ID for every plan |
| `ready PLAN_ID` | Items whose dependencies are satisfied (derived, not authority) |
| `graph PLAN_ID` | Deterministic dependency graph |
| `check [PLAN_ID]` | Read-only validation; omit the ID to check all |
| `evidence list\|show\|supersessions` | Immutable observation and correction lineage |
| `closure show PLAN_ID` | Bounded summary of the validated closure record |

## Mutating commands

All require `--expected-revision N`.

| Command | Purpose |
|---|---|
| `new --input PLAN.json` | Create from strict Plan JSON |
| `activate PLAN_ID --expected-revision N` | Draft → Active |
| `item update PLAN_ID ITEM_ID --expected-revision N` | Update one of `--status`, `--blocker`, `--next-action` |
| `markdown import FILE --state-root PATH` | Create one Draft plan from Markdown intent |

`item update` takes exactly one of `--status`, `--blocker`, or `--next-action`.

## Status vocabularies

**Plan status:** `draft`, `active`, `blocked`, `closed`, `cancelled`. `closed` and
`cancelled` are terminal. Only the guarded finalizer can move a plan to `closed`.

**Item status:** `pending`, `actionable`, `in_progress`, `blocked`, `completed`,
`cancelled`. `completed` and `cancelled` are terminal.

**Evidence status:** `passed`, `failed`, `in_progress`, `not_run`, `skipped`, `blocked`,
`unavailable`, `inconclusive`.

## Assessment and closure

| Command | Purpose |
|---|---|
| `assess PLAN_ID --provider-policy FILE` | Pure assessment against the policy |
| `close PLAN_ID --expected-revision N --provider-policy FILE` | Guarded closure finalization |

Both **require** an explicit provider-policy file. There is no default trusted provider, and
the CLI never accepts user-authored passing evidence. See
[Provider policy](provider-policy.md).

`close` delegates entirely to the repository's guarded finalizer, which revalidates the
current Git subject itself. The CLI cannot assert subject authority at commit time.

## Projection and interchange

| Command | Purpose |
|---|---|
| `registry render` | Compact derived view of canonical state |
| `markdown render PLAN_ID [--output FILE]` | Deterministic Eggplan Markdown v1 |
| `markdown inspect FILE [--format eggplan\|codegg\|auto]` | Proposed intent + loss report, no mutation |
| `markdown import FILE [--format eggplan\|codegg\|auto]` | One Draft plan under an explicit state root |

Markdown never imports evidence, provider trust, a `SubjectRevision`, or a closure record.

`registry render` never edits `plans/registry.md`.

## Check states

`check` reports one state per plan. Current Git subject unavailability is explicit, never
silent.

| State | Meaning |
|---|---|
| `complete` | All criteria satisfied on the current subject |
| `incomplete` | Actionable work remains |
| `in_flight` | Work is in progress |
| `blocked` | Blocked |
| `failed` | Evidence reports failure |
| `awaiting_human_judgment` | Criteria permit human judgment |
| `inconclusive` | Evidence is present but does not resolve |
| `unavailable` | Evidence is unavailable/not-run/skipped, or the subject is unavailable |
| `stale` | A stored closure record's subject no longer matches the current subject |
| `invalid_or_stale` | Evidence is bound to an invalid or superseded subject |

Structural problems surface as typed diagnostics instead: `invalid_plan`, `corrupt_state`,
`recovery_required`.

`check` also emits the check-time reason code `closure_subject_stale`. It is distinct from
the finalization-time diagnostics `closure_subject_changed`,
`closure_subject_drifted`, and `closure_subject_unavailable`.

## JSON envelope

`--json` output is a stable envelope with schema version 1, stable command/status/reason
codes, explicit truncation counts, and bounded warnings.

Failures write diagnostics to stderr in human mode, and a JSON error envelope to stdout
with a nonzero exit code in machine mode.

## Pending closure recovery

`check` is read-only and will not recover a pending closure transaction. Recovery requires
the explicit flag:

```sh
cargo run -p eggplan-cli -- check --state-root .eggplan --recover-pending --json
```

## Exit behavior

Human mode writes diagnostics to stderr. Machine mode writes a JSON error envelope to
stdout and exits nonzero. Human output omits ANSI-dependent correctness cues, so do not
parse it — use `--json` for anything programmatic.
