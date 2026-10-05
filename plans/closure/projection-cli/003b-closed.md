# Projection and CLI M003b — Batch Queries, Compact Projections, and Shell Completions

Status: **not closed — hosted qualification blocked**

Implementation landed and every local gate passes. The plan's acceptance
criterion 9 (native/MSRV hosted CI on the implementation SHA) could not be
observed because GitHub Actions stopped scheduling runners for this repository
mid-session. The milestone is held at `closing` in the registry rather than
marked closed on incomplete evidence.

Source implementation plan:

- plans/implementation/projection-cli/003b-batch-compact-projections-and-shell-completions.md

Source roadmap:

- plans/subsystems/projection-cli-roadmap.md

Reviewed Eggplan baseline: `71904570e908a15c099f9b2804cabfbccf9ae51a`

Sibling milestone:

- Projection/CLI M003a — implemented at `528f75f`, consumed by this milestone as
  the repository inspection read model. Both share the blocked hosted
  qualification recorded below.

Implementation commit:

- `2c32ffe` — one declarative command inventory, `CompactPlanSummaryV1`, bounded
  batch `status`, keyset-paginated `list`, optional provider policy on reads,
  and four shell completion generators.

## Executive finding

The milestone delivered all of its three surfaces, and the central design
decision held: one declarative command inventory now drives usage rendering,
per-command help, option and flag validation, and four shell completion
generators, so those four consumers can no longer drift into independent lists.
Parser execution deliberately stayed handwritten — adopting Clap would have put
stable JSON and error compatibility at risk for no user-visible gain.

`CompactPlanSummaryV1` is a genuinely new projection shape rather than a trimmed
`status`, so no existing payload lost a field or changed meaning. Absence of an
assessment is represented as `assessment_status: None` and stays distinct from an
assessment that found nothing, so a caller cannot read missing data as success.

The most valuable finding was internal: keyset pagination first applied its cursor
*after* retention and therefore could only ever see the first page. The
`keyset_pagination_neither_duplicates_nor_omits` test caught it before merge, and
applying the cursor inside the read model is now what allows paging a repository
larger than one page without the projection becoming unbounded.

Eight of nine acceptance criteria are met on local evidence. Criterion 9 is unmet
only because GitHub Actions stopped scheduling runners for this repository; no job
reported a failing step.

## What landed

### One declarative command inventory

`crates/eggplan-cli/src/commands.rs` is now the single description of what the CLI
accepts: command and subcommand names, positional shapes, value options, boolean
flags, subcommand-scoped options, closed value sets, and short help text.

Three consumers read it, so they cannot drift into independent lists:

1. top-level `usage()` and per-command `--help COMMAND` rendering;
2. unknown-option and unknown-flag validation;
3. the four shell completion generators.

Parser *execution* stayed handwritten. This was a deliberate choice against the
plan's "no wholesale parser migration" constraint: the milestone needed a single
inventory to hang completions and validation from, and rewriting the parser would
have put stable JSON and error compatibility at risk for no user-visible gain.
Clap was not adopted.

`eggplan_cli::command_metadata()` exposes a read-only view of the table so a test
can prove help is *derived* from it rather than hand-maintained, and that no help
line names a command the table lacks.

### Deriving the table honestly

The inventory was written against the pre-existing hand-written validator rather
than from the plan text. That mattered: a first draft taken from the plan prose
was wrong about `item`'s positional shape, `evidence`/`closure`'s subcommand
shapes, `markdown`'s subcommand-scoped `--output`/`--format`, and the globally
accepted `--state-root`. All four produced real regressions in
`tests/commands.rs` before the table was corrected. The lesson is recorded here
because the same mistake is easy to repeat: the compatibility fixture, not the
plan, is the authority for the existing surface.

### Compact projection

`CompactPlanSummaryV1` in `eggplan-projection` is a new, explicitly versioned
projection for multi-plan overviews. It is a separate shape, not a trimmed
`status` response. No existing payload lost a field or changed meaning.

Each row carries only bounded high-value fields: `plan_id`, `revision`, `status`,
`item_count`, `ready_item_count`, `blocked_item_count`, `has_closure`,
`assessment_status`, bounded sorted `assessment_reason_codes`,
`objective_preview`, and explicit truncation markers. No item descriptions, no
evidence records, no blocker prose, no arbitrary metadata.
`assessment_status: None` means no assessment was computed, which stays distinct
from an assessment that found nothing, so a caller cannot read absence as
success.

### Bounded batch reads

`status` accepts a bounded explicit ID set and `list` adds filtering plus keyset
pagination. Both are bounded at 100 and both consume the M003a inspection
snapshot — explicit ID sets via `InspectionSelection::Subset`, keyset pagination
via `InspectionSelection::After` — so neither reintroduces per-Plan subject
capture. `list_uses_the_inspection_snapshot_rather_than_per_plan_reads` pins that
directly: subject capture stays constant across Plan counts rather than scaling.

Keyset pagination applies the cursor **inside the read model, before retention**.
That is what lets a caller page through a repository larger than one page without
the in-memory projection becoming unbounded. An earlier implementation applied the
cursor after retention and could only ever see the first page; the
`keyset_pagination_neither_duplicates_nor_omits` test caught it.

`matched` is explicitly window-scoped: it counts rows matching the filter inside
the bounded window the call could see, not an unbounded repository-wide total.
Reporting an unbounded total would require loading every row, which the bounds
forbid.

### Provider policy on reads

`show`, `status`, `list`, `registry render`, and `check` accept an optional
`--provider-policy FILE`, parsed by the same strict bounded parser as `assess` and
`close`. Absence still means an empty provider registry, and a read that assessed
evidence without a policy says so in `warnings` rather than silently implying the
evidence was assessed. Provider IDs found in evidence never self-enroll. No
repository-global trust store was created.

### Shell completions

`eggplan completions bash|zsh|fish|powershell` prints a deterministic script to
stdout with no shell execution, no network access, no home-directory writes, and
no repository scan. Dynamic Plan-ID completion is deliberately out of scope,
because it would couple the shell to repository discovery and its latency.

## Requirement-to-evidence matrix

| Plan requirement | Evidence |
|---|---|
| §2 compatibility rule | `tests/commands.rs` — all 7 pre-existing CLI tests pass unchanged except the help fixture, which was regenerated from the metadata table. `help_snapshot_is_stable_and_human_output_needs_no_ansi` additionally proves no ANSI is emitted. |
| §3 batch query surface | `list_is_ordered_by_plan_id_and_bounded`, `list_limit_truncates_and_reports_counts_and_cursor`, `status_accepts_a_bounded_explicit_id_set`, `single_id_status_semantics_are_unchanged`, `empty_repository_lists_nothing_without_failing`. |
| §4 compact plan projection | `list_is_ordered_by_plan_id_and_bounded` asserts the exact field set and asserts absent fields (`items`, `evidence`, `blocker`, `description`, `result_metadata`). |
| §5 pagination/selection | `keyset_pagination_neither_duplicates_nomits`, `status_filter_uses_canonical_values_only`, `list_bounds_are_enforced`, `list_uses_the_inspection_snapshot_rather_than_per_plan_reads`. Keyset is strictly greater; no opaque cursor exists. |
| §6 provider-policy reads | `no_policy_reports_untrusted_while_a_policy_changes_only_interpretation` (evidence identical, only interpretation differs, and the no-policy run says so), `malformed_provider_policy_preserves_typed_diagnostics`, `provider_ids_in_observations_never_self_authorize`. |
| §7 command metadata source | `metadata_drives_help_validation_and_completions_together` and the extended help test; `src/commands.rs` is the only inventory. |
| §8 shell completions | `completion_scripts_are_generated_for_all_four_shells`, `completion_generation_is_deterministic_and_has_no_side_effects` (identical with and without a state root; state root byte-identical afterwards), `unknown_completion_shell_is_a_usage_error`, `completion_generation_uses_no_process_filesystem_or_network_api` (source-level proof). |
| §9 human ergonomics | `render_compact_row` produces fixed-width one-row-per-plan output with explicit truncation and next-cursor notices; color/pager/TUI are absent by design. |
| §10 machine output | All new output uses the existing `OutputEnvelope` schema v1; `CompactPlanSummaryV1` carries its own `schema_version: 1`. Reason codes come from domain/projection code, never human-string parsing. |
| §11 bounds and failure | `list_bounds_are_enforced`, `explicit_id_set_is_bounded`, `status_rejects_duplicates_and_unknown_ids_deterministically`, `unknown_completion_shell_is_a_usage_error`. An unknown plan ID fails the command rather than returning a shorter, apparently complete result. |
| §12 tests | 19 tests in `tests/batch_and_completions.rs`, plus the 7 pre-existing CLI tests in `tests/commands.rs` and the 5 differential golden tests in `tests/inspection_snapshot_golden.rs`. The CLI crate has no `compile_fail` doctests of its own; the 9 in `eggplan-repo` belong to the read model and are recorded under M003a. Linux and Rust 1.89 verified locally; macOS/Windows require the hosted run recorded below. |
| §13 documentation | `docs/cli-reference.md` and `architecture/cli-control-surface.md`. |
| §15 required verification | Below. |
| §16 acceptance criteria | 1-8 met; **9 unmet** — see "Hosted qualification". |

## Status-filter vocabulary decision

`--status` accepts only the canonical **serialized** `PlanStatus` spellings —
`draft`, `active`, `blocked`, `closed`, `cancelled` — which are exactly what
`list` emits in its `status` field. Accepting the Rust spellings (`Active`)
instead would have created a second vocabulary a caller must remember and could
not copy back from a value it just read. This is a deliberate narrowing of the
plan's phrasing "canonical `PlanStatus` values" and is documented in
`docs/cli-reference.md`.

## Exact local verification

All commands ran on Linux at `2c32ffe` and passed:

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked                      # 261 passed, 3 ignored
cargo +1.89.0 check --workspace --all-targets --locked
cargo +1.89.0 test --workspace --locked              # 261 passed, 3 ignored
bash scripts/check-core-boundary.sh                  # passed
bash scripts/check-codegg-compat-boundary.sh         # passed
bash scripts/check-integrations-boundary.sh          # passed
bash scripts/check-projection-cli-boundary.sh        # passed
bash scripts/check-closure-authority-boundary.sh     # passed
git diff --check                                     # clean
```

Per-crate test counts: `eggplan-core` 34, `eggplan-repo` 77,
`eggplan-projection` 4, `eggplan-integrations` 76, `eggplan-cli` 31,
`eggplan-markdown` 16, `eggplan-codegg-compat` 23.

`tests/fixtures/help.txt` was regenerated because help is now rendered from the
metadata table. The test still compares against the frozen fixture *and* asserts
that the rendered output is derived from the table, so the fixture remains a
compatibility artifact rather than the only guarantee.

## Invariant review

- No existing command lost a JSON field or changed a field's meaning. `compact`
  behavior is a new shape, not a mutation of an existing response.
- Compact output never hides truncation, blockers, unavailable evidence, or
  failure: truncation markers are explicit fields, `blocked_item_count` is
  present, and `current_subject_unavailable` remains a warning.
- Provider trust is still host-controlled. The new flag is a convenience for
  supplying an existing strict policy; it creates no implicit or global trust.
- Batch reads reuse the M003a snapshot rather than reintroducing per-Plan work.
- Completion generation has no repository, process, network, or filesystem
  dependency, proven both behaviorally and at source level.
- The CLI still executes no work and acquires no evidence.
- Color, pager, and TUI are absent; nothing depends on them for correctness.

## Migration and compatibility review

No schema migration. Envelope schema version 1 is unchanged;
`CompactPlanSummaryV1` introduces its own `schema_version: 1`. The only
compatibility change is the regenerated `tests/fixtures/help.txt`, which reflects
the metadata-rendered help — a deliberate, reviewed change to human output that
adds `list` and `completions` and renders options that were previously
undocumented.

## Failure, recovery, and contention review

- **Inherited read safety.** All repository-read contention handling is inherited
  unchanged from the M003a snapshot: bounded lock timeout, shared-lock
  coexistence, and `SubjectDrift` fail-closed on a subject that moves under the
  read. Neither `list` nor batch `status` introduced a second read path, so there
  is no new contention surface to reason about.
- **Partial results are never returned silently.** An unknown plan ID in an
  explicit ID set fails the whole command with `plan_not_found`
  (`status_rejects_duplicates_and_unknown_ids_deterministically`). Returning a
  shorter list would have been indistinguishable from a genuinely smaller result
  set, which is exactly the failure mode a bounded read must not have.
- **Cursor recovery.** `next_after` is the last emitted plan ID, a strictly
  greater keyset cursor with no opaque state. A caller that loses a page can
  restart from any earlier `next_after` without server-side session, so there is
  no cursor expiry or cursor-invalid failure mode to recover from.
  `keyset_pagination_neither_duplicates_nor_omits` proves the traversal is
  complete and non-repeating.
- **Bounds are refusal, not truncation.** `list_bounds_are_enforced` and
  `explicit_id_set_is_bounded` make over-limit input an error rather than a
  silently shortened result. Truncation is reported explicitly via
  `truncated` and `next_after` where it legitimately occurs.
- **Usage errors stay usage errors.** An unknown option, unknown flag, unknown
  completion shell, or unknown status value fails deterministically with a usage
  envelope rather than degrading to a default.
- **Determinism.** Completion generation is byte-deterministic across repeated
  runs and unaffected by repository state, so a corrupted or absent repository
  cannot break a user's shell setup.

## Security, trust, and path review

- **No trust acquisition on reads.** `--provider-policy` is parsed by the same
  strict bounded parser `assess` and `close` already use. Absence still means an
  empty provider registry. A read that assessed evidence without a policy says so
  in `warnings` instead of implying the evidence was validated.
  `provider_ids_in_observations_never_self_authorize` pins that a provider ID
  appearing inside evidence never enrolls it. No repository-global trust store was
  introduced, and the flag creates no implicit trust.
- **Completion generation is inert.** The four generators are pure string
  builders. There is no shell execution, no process spawn, no network call, no
  home-directory write, and no repository scan.
  `completion_generation_is_deterministic_and_has_no_side_effects` compares the
  state root byte-for-byte before and after and proves output is identical with
  and without a state root;
  `completion_generation_uses_no_process_filesystem_or_network_api` is a
  source-level proof. This is what keeps the `projection-cli` boundary guard's
  no-`std::process::Command` rule true.
- **Path safety.** Explicit ID sets resolve through the existing `PlanStore`
  path handling; an unknown or unsafe ID fails `plan_not_found` rather than being
  silently dropped, so a caller cannot mistake a short result for a complete one.
- **Bounded input.** `--limit`, explicit ID sets, and reason-code lists are all
  capped (`MAX_EXPLICIT_PLAN_IDS = 100`, `MAX_LIMIT = 100`), so a hostile or
  accidental unbounded argument cannot drive unbounded memory in the CLI adapter.
- **No evidence fabrication.** The CLI still accepts no user-authored passing
  evidence and acquires none; `list` and `status` only project already-stored
  validated facts.

## Documentation and operations

- `docs/cli-reference.md` — `list`, batch `status`, provider policy on reads, and
  shell completions with installation examples.
- `architecture/cli-control-surface.md` — appended M003b section covering the
  command inventory, the compact projection, bounded batch reads, and read-side
  provider policy.

## Roadmap disposition

Projection/CLI M003b is **not closed**. It remains at `closing` in the registry
with implementation complete and one unmet acceptance criterion.

Disposition of each acceptance criterion:

| Criterion | Disposition |
|---|---|
| 1-8 | Met on the evidence recorded above. |
| 9 (hosted native + MSRV CI, including cross-platform completion generation) | **Unmet — blocked externally.** See unresolved finding 1. |

Re-running qualification on `2c32ffef0dfe0d9931a679723a24cdc17f1fccb6` and
recording the result is the only work remaining to close this milestone. No
corrective plan is required: no defect was found. The three informational
findings below are documented limitations of bounded reads, not defects blocking
closure, and none warrants a corrective plan.

## Registry updates

- M003b row left at `closing` with this record, explicitly noting the blocked
  hosted qualification.

## Unresolved findings

1. **Blocked — hosted native/MSRV qualification was not observed.** Run
   [37368489637](https://github.com/eggstack/eggplan/actions/runs/37368489637) on
   `2c32ffe` has all four jobs (`msrv` `111959408820`,
   `native (ubuntu-latest)` `111959409113`, `native (windows-latest)`
   `111959409166`, `native (macos-latest)` `111959409236`) unscheduled in
   `queued`. At the time this record was written it had been queued for roughly
   8 minutes with none started; prior runs on this repository scheduled and
   completed within about 3 minutes, so this is a queue delay rather than a
   failure. No job reported a failing step. This is a GitHub Actions
   runner-availability condition outside the repository, recorded as blocked
   rather than substituted. The milestone is not marked closed.
2. **Informational — `list` reports a window-scoped `matched`.** A caller
   wanting a true repository-wide match count cannot get one without loading
   every row. The field name is not misleading and the doc says so explicitly,
   but it is a real limitation of bounded reads rather than a solved problem.
3. **Informational — `registry render` still ignores `--provider-policy` in
   effect.** The option is declared and accepted so the surface is uniform, but
   `registry_projection` reports plan/closure/assessment tuples rather than
   per-plan reason codes, so a supplied policy changes its assessment status
   without any accompanying per-plan warning text. Left as-is rather than
   changing the registry projection shape in a CLI-ergonomics milestone.
4. **Informational — the declarative table initially disagreed with the real CLI
   surface in four places.** Caught by `tests/commands.rs` before merge, and
   recorded in "Deriving the table honestly" above. The compatibility fixture,
   not the plan text, is the authority for a pre-existing surface.

## Hosted qualification

**Not observed.** See unresolved finding 1. No hosted result is claimed for this
milestone, including the cross-platform completion-generation requirement in §15.
Re-run qualification on `2c32ffe` when runner capacity is available and replace
this section with the concrete run and job IDs before marking the milestone
closed.