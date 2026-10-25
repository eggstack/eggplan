# CLI control surface and derived projections

The `eggplan` binary is a thin command adapter over `eggplan-core`,
`eggplan-repo`, and the reusable `eggplan-projection` library. It does not
execute work, acquire evidence, or edit the development planning registry.

## Commands

- `init` creates or opens a local state root.
- `new`, `activate`, and `item update` use strict Plan input or revisioned
  repository compare-and-swap.
- `plan set-status PLAN_ID --to STATUS --expected-revision N` changes a plan's
  lifecycle status. `--to` accepts `active`, `blocked`, or `cancelled`. `--to closed` is
  refused with `guarded_closure_required` and a message pointing at `eggplan close`,
  because only the guarded closure finalizer may write `Closed`; `--to draft` is refused
  with `invalid_transition`; any other illegal move is `invalid_transition`; and a stale
  revision is `revision_conflict`. Legality is decided by
  `eggplan_core::plan_transition_allowed`, the same table the store consults, so the CLI
  holds no per-command pre-state rule.
- `plan edit PLAN_ID --input FILE --expected-revision N` replaces a plan's structure
  with the Plan JSON in `FILE`, parsed by `eggplan_core::parse_plan` — the same strict
  parser `new` uses, so unknown fields are rejected. The submitted plan's `id` must
  match the target (a mismatch is `invalid_plan`), and the command cannot change the
  plan's status: a differing submitted status is `invalid_transition`. A `Closed` plan
  is `closed_plan_immutable`, a stale revision is `revision_conflict`, and malformed or
  unknown-field-bearing input is also `invalid_plan`.
- `plan export PLAN_ID [--output FILE]` writes the plan's canonical Plan JSON
  (`eggplan_core::canonical_json`) to stdout or to the named file. It is exactly what
  `plan edit` consumes, so the export/edit round trip uses one representation of a plan.
- The `plan` family writes only through the repository's compare-and-swap, under the
  store's lock, with the revision conflict check and the atomic write the store already
  performs. It never creates evidence, enrolls a provider, captures a subject revision,
  or writes a closure record; a structural edit cannot smuggle a `Closed` plan into
  existence, and `markdown import` still creates only a Draft plan.
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

## Current-subject diagnostics

The current Git subject can fail to be captured for two different reasons, and
the CLI keeps them apart:

- **Unavailability** — the subject could not be captured at all. The read path
  reports it with the stable `subject_unavailable` diagnostic code and carries
  the underlying capture cause in the message, matching the guarded closure
  path that already reported `subject_unavailable` the same way. `show`
  therefore fails with `subject_unavailable` instead of returning a
  subjectless summary: a subjectless result is never presented as a complete
  one.
- **Drift** — the subject changed under the read. The repository layer reports
  that as `RepoError::SubjectDrift`, and the CLI maps it to
  `subject_drifted_during_inspection`.

A read that could not capture the subject at either end of its scan degrades
instead of failing: the read succeeds, the Plan is reported with the
`unavailable` check state or without an assessment, and the CLI emits the
`current_subject_unavailable` warning. The warning token is exactly
`current_subject_unavailable`; the specific capture cause is appended to the
warning text and never enters the reason-code list, so a consumer that keys on
the token or on `reason_codes` is unaffected. The set of commands that warn and
the number of warnings they emit are unchanged.

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

### Shell completion depth

The four completion generators reach the *same* depth, and the depth is stated rather
than implied: the top-level command names; the subcommands of the command being
completed; that command's options and flags, including the options scoped to one of
its subcommands; and the fixed values of the option being completed, when that option
declares a closed value set. `crates/eggplan-cli/src/commands.rs:530-531` records the depth
as a constant that every generated script states in its own header.

The generators are written from one per-command model — the `CompletionModel`
(`crates/eggplan-cli/src/commands.rs:557-559`) — rather than from four hand-written fragments,
so a shell cannot quietly consume less of the table than another. The CLI test
`completion_scripts_reach_the_same_declared_depth_in_every_shell`
(`crates/eggplan-cli/tests/batch_and_completions.rs:1555`) asserts the depth per shell
by reading each script's candidate lists rather than by looking for a name in the
script text; `completion_scripts_declare_no_unused_shell_variables`
(`crates/eggplan-cli/tests/batch_and_completions.rs:1625`) asserts that no generated
script declares a shell variable it never reads, which is the mechanism by which the
earlier depth gap hid — the shell fragments are format strings, so an unused local is
invisible to the Rust compiler. `completion_option_value_sets_match_the_metadata`
(`crates/eggplan-cli/tests/batch_and_completions.rs:1665`) ties the closed value sets
the scripts offer back to the ones the table declares.

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
