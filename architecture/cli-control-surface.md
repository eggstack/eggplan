# CLI control surface and derived projections

The `eggplan` binary is a thin command adapter over `eggplan-core`,
`eggplan-repo`, and the reusable `eggplan-projection` library. It does not
execute work, acquire evidence, or edit the development planning registry.

## Commands

- `init` creates or opens a local state root.
- `new`, `activate`, and `item update` use strict Plan input or revisioned
  repository compare-and-swap.
- `show`, `status`, `ready`, and `graph` read canonical Plan state. Readiness
  comes from core dependency semantics; graph output is deterministic.
- `evidence list|show|supersessions` reads immutable observation and correction
  lineage.
- `assess` requires a versioned provider-policy file for one invocation.
- `close` requires that policy and an expected Plan revision, then delegates
  exclusively to guarded Evidence M002 finalization. The CLI does not pass
  its own captured subject to the repository; finalization recaptures under
  its own lock and reports subject-stale / subject-drift / subject-unavailable
  as stable machine diagnostics (`closure_subject_changed`,
  `closure_subject_drifted`, `closure_subject_unavailable`).
- `closure show` returns a bounded summary of the validated immutable closure.
- `check` validates repository, Plans, evidence, supersessions, closure
  records, pending transactions, and staging state. It opens the repository in
  read-only mode and never recovers a pending transaction unless
  `--recover-pending` is explicit. `check` also emits a check-time
  `closure_subject_stale` code (state `stale`) when a stored closure record's
  subject no longer matches the current subject; this is distinct from the
  finalization-time `closure_subject_changed` / `closure_subject_drifted` /
  `closure_subject_unavailable` diagnostics above.
- `registry render` derives a compact view from canonical `.eggplan` Plans. It
  never edits this repository's `plans/registry.md`.
- `markdown render` emits deterministic Eggplan Markdown v1; `markdown inspect`
  returns proposed Plan intent and a loss report without mutation; `markdown
  import` creates one Draft Plan only under an explicit state root. Markdown
  never imports evidence, provider trust, SubjectRevision, or closure. See
  [Markdown interchange](markdown-interchange.md) for grammar and bounds.

All commands support the stable JSON envelope via `--json`. Output uses schema
version 1, stable command/status/reason codes, explicit truncation counts, and
bounded warnings. Human output omits ANSI-dependent correctness cues. Failures
write diagnostics to stderr in human mode and a JSON error envelope to stdout
with a nonzero exit code in machine mode.

## Provider policy

An operator-supplied policy resembles:

```json
{
  "schema_version": 1,
  "providers": [
    {
      "provider_id": "epp_local_tests",
      "class": "test_runner",
      "allowed_kinds": ["test"]
    }
  ]
}
```

Unknown fields/versions, duplicate providers/kinds, empty kind sets, oversized
files, and invalid descriptors fail closed. The policy is scoped to that one
invocation and contains no credentials. There is no default trusted provider.

## Check classifications

Structural failures are returned as typed `invalid_plan`, `corrupt_state`, or
`recovery_required` diagnostics. Valid stored state is reported per Plan as
`complete`, `incomplete`, `unavailable`, `stale`, `invalid_or_stale`,
`in_flight`, `blocked`, `failed`, `inconclusive`, or `awaiting_human_judgment`. If no provider
policy is supplied to `check`, its empty host registry cannot promote passing
observations to trusted proof. Current Git subject unavailability is explicit.

`RepositoryStore::open_read_only` does not initialize files or recover pending
closure transactions. The separate normal open path retains repository
startup recovery semantics for mutation and explicit `check --recover-pending`.


## M003b additions

### One declarative command inventory

`crates/eggplan-cli/src/commands.rs` is the single description of what the CLI accepts:
command and subcommand names, positional shapes, value options, boolean flags,
subcommand-scoped options, closed value sets, and short help text.

Three things read it, so they cannot drift into independent inventories:

1. the top-level `usage()` line and per-command `--help COMMAND` rendering;
2. unknown-option and unknown-flag validation in `validate_command_options`;
3. the four shell completion generators.

Parser *execution* remains handwritten. This is deliberately not a parser migration:
the milestone needed a single inventory to hang completions and validation from, and
rewriting the parser would have put stable JSON and error compatibility at risk for no
user-visible gain. Clap was not adopted; it must not take over error or output authority
unless exact CLI JSON/error compatibility is proven first.

`eggplan_cli::command_metadata()` exposes a read-only view of the table so a test can
prove help is *derived* from it, and `metadata_drives_help_validation_and_completions_together`
proves every declared option is accepted while an undeclared one still fails.

### Compact plan projection

`CompactPlanSummaryV1` in `eggplan-projection` is a new, explicitly versioned projection
for multi-plan overviews. It never replaces or trims `PlanSummary`, `show`, or `status`:
those payloads are unchanged. The compact row carries identity, revision, lifecycle,
item and readiness counts, closure presence, an optional assessment verdict, bounded
sorted reason codes, and an objective preview with an explicit truncation marker. It
carries no item descriptions, evidence records, blocker prose, or arbitrary metadata.

`assessment_status: None` means no assessment was computed. That is deliberately distinct
from an assessment that found nothing, so a caller cannot read absence as success.

### Bounded batch reads

`status` accepts a bounded explicit ID set and `list` adds filtering and keyset
pagination. Both are bounded at 100, and both go through the M003a repository inspection
snapshot — including explicit ID sets, which use `InspectionSelection::Subset` and
`InspectionSelection::After` so a batch never reintroduces per-Plan subject capture.

`--status` accepts only the canonical serialized `PlanStatus` spellings that `list` itself
emits, so a caller can filter on a value it just read.

Keyset pagination applies the cursor inside the read model, before retention. That is
what lets a caller page through a repository larger than one page without the projection
becoming unbounded.

`matched` is explicitly window-scoped: it counts rows matching the filter inside the
bounded window the call could see, not an unbounded repository-wide total. Reporting an
unbounded total would require loading every row, which the bounds forbid.

### Provider policy on read commands

`show`, `status`, `list`, `registry render`, and `check` accept an optional
`--provider-policy FILE`, parsed by the same strict bounded parser as `assess` and
`close`. Absence still means an empty provider registry, and a read that assessed
evidence without a policy says so in `warnings`. Provider IDs found in evidence never
self-enroll, and no repository-global trust store exists.
