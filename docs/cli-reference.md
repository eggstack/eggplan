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
| `list [options]` | Compact, bounded plan overview with keyset pagination |
| `show PLAN_ID` | Canonical plan state |
| `status [PLAN_ID ...]` | Derived status; omit IDs for every plan, or pass a bounded explicit set |
| `ready PLAN_ID` | Items whose dependencies are satisfied (derived, not authority) |
| `graph PLAN_ID` | Deterministic dependency graph |
| `check [PLAN_ID]` | Read-only validation; omit the ID to check all |
| `evidence list\|show\|supersessions` | Immutable observation and correction lineage (read-only) |
| `closure show PLAN_ID` | Bounded summary of the validated closure record |
| `completions bash\|zsh\|fish\|powershell` | Print a shell completion script to stdout |

`evidence` is **read-only**. The CLI has no command that records an observation, so
`evidence add` returns a usage error. Writing evidence is a host responsibility — see
[Evidence and closure](evidence-and-closure.md).

## `list` — compact plan overview

`list` returns `CompactPlanSummaryV1`, a new explicitly versioned projection. It is a
separate shape, not a trimmed `status` response: no existing JSON field changed meaning.

```sh
eggplan list [--status STATUS] [--limit N] [--after PLAN_ID] [--provider-policy FILE]
```

Each row carries only bounded high-value fields: `plan_id`, `revision`, `status`,
`item_count`, `ready_item_count`, `blocked_item_count`, `has_closure`,
`assessment_status`, `assessment_reason_codes` (sorted, deduplicated, bounded),
`objective_preview`, and explicit truncation markers. It never carries item
descriptions, evidence records, blocker prose, or arbitrary metadata.

Rows are ordered by Plan ID.

### Status filter

`--status` accepts only the canonical serialized `PlanStatus` values, which are exactly
what `list` emits in its `status` field: `draft`, `active`, `blocked`, `closed`,
`cancelled`. Statuses are never matched by prose, and an unrecognized value is a
`usage` error.

### Keyset pagination

`--after PLAN_ID` returns Plan IDs **strictly greater** than `PLAN_ID` in canonical
sorted order. There are no opaque random cursors and no server-side state.

The response reports `matched` (rows matching the filter inside the bounded window this
call could see — not an unbounded repository-wide total), `returned`, `truncated`, and
`next_after`, the last Plan ID handed out. Pass it back as `--after` to continue. Paging
through a repository never duplicates or omits a row.

Bounds: `--limit` defaults to 100 and must be between 1 and 100. A malformed cursor Plan
ID, a positional argument, or an out-of-range limit is a `usage` or `invalid_id` error.

## Batch `status`

`status` accepts a bounded list of explicit Plan IDs, at most 100 per invocation:

- no IDs — repository-wide bounded status;
- one ID — identical semantics to the previous single-plan `status`;
- several IDs — the same stable projection for exactly the requested set.

Duplicate IDs are a caller mistake and are rejected rather than silently deduplicated. An
unknown ID fails the command with `plan_not_found` rather than quietly returning fewer
rows. Explicit IDs use the M003a inspection snapshot, so a batch read does not
reintroduce per-Plan Git subject capture.

## Provider policy on reads

`show`, `status`, `list`, `registry render`, and `check` accept an optional
`--provider-policy FILE`.

Absence of a policy continues to mean **no implicit trust**: assessments are computed
against an empty provider registry, so legitimate observations read as untrusted. When
evidence exists and no policy was supplied, the response carries the warning
`assessment_uses_empty_provider_registry:no provider policy was supplied` rather than
silently implying the evidence was assessed.

A supplied policy is parsed by the same strict bounded parser used by `assess` and
`close`, and the same single loaded policy is applied across every plan in the read.
Provider IDs that appear in evidence never self-enroll. No repository-global trust store
is created.

See [Provider policy](provider-policy.md).

## Shell completions

`eggplan completions <shell>` prints a completion script to stdout for `bash`, `zsh`,
`fish`, or `powershell`.

```sh
eggplan completions bash > /etc/bash_completion.d/eggplan
```

Generation is deterministic for the same binary version, is produced entirely from the
binary's own command metadata, and performs no shell execution, no network access, no
home-directory writes, and no repository scan. Dynamic Plan-ID completion is
deliberately out of scope, because it would couple the shell to repository discovery and
its latency.

Completion, help text, and option validation are all generated from one declarative
command inventory, so they cannot drift into three separate lists of what the CLI
accepts. Add a command there and help and every completion shell change together.

## Mutating commands

All require `--expected-revision N`.

| Command | Purpose |
|---|---|
| `new --input PLAN.json` | Create from strict Plan JSON |
| `activate PLAN_ID --expected-revision N` | Draft → Active |
| `item update PLAN_ID ITEM_ID --expected-revision N` | Update one of `--status`, `--blocker`, `--next-action` |
| `markdown import FILE --state-root PATH` | Create one Draft plan from Markdown intent |

`item update` takes exactly one of `--status`, `--blocker`, or `--next-action`.

Item status changes are checked against a transition table, so you cannot jump states:

| From | Legal next states |
|---|---|
| `pending` | `actionable`, `blocked`, `cancelled` |
| `actionable` | `in_progress`, `blocked`, `cancelled` |
| `in_progress` | `actionable`, `blocked`, `completed`, `cancelled` |
| `blocked` | `pending`, `actionable`, `cancelled` |
| `completed`, `cancelled` | terminal — no transitions |

An illegal move fails with `invalid_transition`. In particular, `pending → in_progress` is
not legal; go through `actionable` first.

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

Both also require a **resolvable Git subject**. In a repository with no commits — an unborn
branch — subject capture fails and the command exits nonzero with `subject_unavailable`.
Make at least one commit first.

`close` delegates entirely to the repository's guarded finalizer, which revalidates the
current Git subject itself. The CLI cannot assert subject authority at commit time.

Human-readable `assess` output prints the assessment status as a variant name such as
`EvidenceMissingOrUnavailable`; use `--json` for the stable snake_case codes.

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
