# Deep dive: projection + CLI

See also [overview](overview.md) and [CLI control surface](cli-control-surface.md).
Roadmap: `plans/subsystems/projection-cli-roadmap.md` — M001, M002, and M003 (M003a and
M003b) closed; M004 closed.

## 1. Roles

`eggplan-projection` (`crates/eggplan-projection/src/lib.rs`) is the reusable,
dependency-free derived-view library. It depends only on `eggplan-core` + `serde`
(`crates/eggplan-projection/Cargo.toml:8-10`) and owns bounded summaries,
deterministic ordering, truncation flags, and the stable JSON envelope shape.

`eggplan-cli` (`crates/eggplan-cli/src/lib.rs`, `crates/eggplan-cli/src/main.rs`)
is the thin command adapter over `eggplan-core`, `eggplan-repo`,
`eggplan-projection`, and `eggplan-markdown`
(`crates/eggplan-cli/Cargo.toml:8-14`). `main.rs:5-12` decodes argv and
forwards it to `eggplan_cli::run`; a non-UTF-8 argument is routed to
`eggplan_cli::report_invalid_argument` (`lib.rs:316-330`) rather than
forwarded. Neither crate may execute work, acquire evidence, run a
network/runtime/MCP service — enforced by
`scripts/check-projection-cli-boundary.sh:10-30`, which also gates
`eggplan-markdown` (`check-projection-cli-boundary.sh:15-18`). The script
polices dependencies and process spawning only, not file writes; see finding 4.

## 2. Projection walkthrough

- Bounds: `OUTPUT_SCHEMA_VERSION = 1`, `MAX_PROJECTED_PLANS = 100`,
  `MAX_PROJECTED_ITEMS = 100`, `MAX_TEXT_CHARS = 512`, `MAX_WARNINGS = 32`
  (`crates/eggplan-projection/src/lib.rs:12-16`).
- Envelope: `OutputEnvelope<T>` with
  `schema_version/command/ok/data/error/warnings/warnings_truncated`
  and `deny_unknown_fields` (`lib.rs:18-29`). `success` bounds warnings
  (`lib.rs:39-49`) and reports whether that bound dropped any, so a machine
  consumer can tell "no warnings" from "warnings truncated"; `failure`
  char-truncates the message to 512 (`lib.rs:51-67`).
- `summarize_plan` (`lib.rs:193-228`): truncates objective, counts item statuses
  via `item_status_code` (`lib.rs:436-445`), sorts/dedups assessment reason codes
  via `reason_code` (`lib.rs:447-476`), reports `projected_item_count` /
  `items_truncated`.
- `readiness_projection` (`lib.rs:230-265`): derives per-item readiness from core
  `readiness()`, sorts by `(position, item_id)`, counts
  `ready/waiting/not_actionable`, truncates to 100 with `truncated` + `total_items`.
- `graph_projection` (`lib.rs:267-328`): sorts nodes by `(position, id)`, takes 100,
  keeps only intra-selection dependency/parent edges (sorted), reports
  `total_*_edges` vs `edges_truncated` plus node-level `truncated`.
- `registry_projection` (`lib.rs:330-398`): sorts plans by `PlanId`, collects
  blockers in position order (truncated at 100), takes 100 plans, computes
  per-plan ready counts from core `readiness()`, truncates reason codes at 32.
- Helpers: char-based `truncate` (`lib.rs:478-482`) and `bound_warnings`
  (`lib.rs:484-490`). All ordering uses `BTreeMap`/`BTreeSet`/explicit sorts, so
  output is deterministic for identical canonical input.

## 3. CLI walkthrough

- Entry: `run` prints pretty JSON envelope to stdout and returns exit 0 on
  success (`crates/eggplan-cli/src/lib.rs:277-308`), and a JSON error envelope
  (exit 2) in `--json` mode, else `command: code: message` to stderr
  (`lib.rs:294-306`). `execute` defaults `--state-root` to `.eggplan`
  (`lib.rs:332-340`).
- Arg parsing is hand-rolled: `parse_args` (`lib.rs:342-402`), value options
  (`lib.rs:404-406`), per-command allow-lists in `validate_command_options`
  (`lib.rs:565-606`), usage string (`crates/eggplan-cli/src/commands.rs:448-468`).
  The value-option set and the per-command allow-lists themselves live in the
  declarative command table (`crates/eggplan-cli/src/commands.rs:210` `COMMANDS`,
  `:424` `options_for`); `lib.rs` delegates to that table.
- Shell completions are generated from that same table: `completion_script`
  (`crates/eggplan-cli/src/commands.rs:634-643`) builds one per-command model — the
  `CompletionModel` (`crates/eggplan-cli/src/commands.rs:557-559`) — and emits it as a
  Bash, Zsh, Fish, or PowerShell script. All four reach the same depth — commands,
  subcommands, options (including subcommand-scoped options), and option values — which
  the generator states as a constant (`crates/eggplan-cli/src/commands.rs:530-531`).
  Generation reads only the table: no filesystem, process, or repository access.
- Command groups in `dispatch` (`lib.rs:412-542`):
  - `init` opens/creates the store (`lib.rs:421-430`); `new` parses strict Plan
    JSON, requires revision 0 + Draft, then `create` (`lib.rs:431-460`).
  - `show` composes `PlanDetail` from `summarize_plan` plus
    `readiness_projection(...).items` (`lib.rs:461-495`); `status` caps at 100
    plans with `total_plans/truncated` and per-plan
    `current_subject_unavailable` warnings (`lib.rs:676-789`); `ready`
    (`lib.rs:497-510`) and `graph` (`lib.rs:511-526`; human output is a bare
    newline-joined ID list) are pure reads via `open_read_only`.
  - `activate` / `item update` do explicit `--expected-revision` pre-checks, then
    `compare_and_swap` (`lib.rs:1283-1327`, `lib.rs:1582-1670`).
  - `plan` (`lib.rs:1338-1350`) is a three-subcommand family — `set-status`, `edit`,
    `export` — declared in the command table
    (`crates/eggplan-cli/src/commands.rs:292-304`).
    `plan set-status` (`lib.rs:1358-1421`) takes `--to` and `--expected-revision`;
    `--to` accepts only `active`, `blocked`, or `cancelled`
    (`parse_settable_plan_status`, `lib.rs:1549-1580`). `--to closed` is refused with
    `guarded_closure_required` and a message pointing at `eggplan close`, because only
    the guarded closure finalizer may write `Closed`; `--to draft` is refused with
    `invalid_transition`; any other illegal move is `invalid_transition`; and a stale
    revision is `revision_conflict`. Legality is decided by
    `eggplan_core::plan_transition_allowed`
    (`crates/eggplan-core/src/model.rs:396-409`) — the same table the store consults —
    so the CLI holds no per-command pre-state rule.
    `plan edit` (`lib.rs:1431-1493`) replaces the plan's structure with the Plan JSON in
    `--input`, parsed by `eggplan_core::parse_plan`, the same strict parser `new` uses.
    The submitted plan's `id` must match the target (a mismatch is `invalid_plan`); a
    differing submitted status is `invalid_transition` (structure editing is not a back
    door to `Closed`); a `Closed` plan is `closed_plan_immutable`; a stale revision is
    `revision_conflict`; and malformed or unknown-field-bearing input is `invalid_plan`.
    Both mutating subcommands write only through the repository's compare-and-swap
    (`crates/eggplan-repo/src/store.rs:991-1042`), so `Plan::validate`, the graph's
    bounds and acyclicity checks, the closed-plan immutability guard, the
    guarded-closure guard, and the item-status transition rules all still apply.
    `plan export` (`lib.rs:1499-1527`) writes the plan's canonical Plan JSON
    (`eggplan_core::canonical_json`) to stdout or to `--output FILE`, which is exactly
    what `plan edit` consumes.
  - `--expected-revision` is required by exactly the five revisioned mutations —
    `activate` (`lib.rs:1285`), `item update` (`lib.rs:1595`), `close`
    (`lib.rs:1776`), `plan set-status` (`lib.rs:1361`), and `plan edit`
    (`lib.rs:1434`) — and `validate_command_options` (`lib.rs:565-606`) rejects it
    for every other command, so it cannot be smuggled in or dropped. Creation
    commands (`init`, `new`, `markdown import`) are not revisioned and take no
    such flag.
  - `evidence list|show|supersessions` are read-only briefs capped at 100 rows
    (`lib.rs:1672-1759`; caps `lib.rs:66-71`; brief shape `lib.rs:127-136`).
  - `assess` requires `--provider-policy`, reads via `compute_assessment`
    (`lib.rs:1761-1772`, `lib.rs:1968-2006`); `assessment_data` caps at 100 items
    / 32 reasons per row (`lib.rs:2008-2041`).
  - `close` requires `--expected-revision` + `--provider-policy`, pre-checks
    revision, builds `ClosureCandidate`, then `finalize_closure` with a generated
    `ClosureId` — no caller-supplied subject (`lib.rs:1774-1822`).
    `closure show` returns the bounded `ClosureSummary` (`lib.rs:1824-1854`,
    `lib.rs:2067-2078`).
  - `check` (`lib.rs:1020-1152`) opens read-only (`lib.rs:2163-2165`) and fails with
    `recovery_required` unless `--recover-pending` is explicit (`lib.rs:1028-1052`);
    that flag re-opens through the mutating `RepositoryStore::open`
    (`lib.rs:1030`, `lib.rs:1050`), so read-only is a default, not an invariant.
    It classifies per plan via `assessment_state` (`lib.rs:2043-2065`) into 10
    states — `complete`, `incomplete`, `unavailable`, `stale`, `invalid_or_stale`,
    `in_flight`, `blocked`, `failed`, `inconclusive`, `awaiting_human_judgment` —
    where `stale` is contributed by `check` itself (`lib.rs:1102-1103`) and is
    reserved for a stale closure record, so evidence on a superseded subject
    stays an assessment classification (`invalid_or_stale`) and `check` agrees
    with `status`/`list`/`show` for one identical plan and evidence state;
    `unavailable` also covers an uncapturable subject (`lib.rs:1114`). Stale
    closures are flagged with the reason code `closure_subject_stale`
    (`lib.rs:1088-1093`). It warns on unavailable subjects and abandoned staging
    files, and accepts an optional `--provider-policy`
    (`crates/eggplan-cli/src/commands.rs:274-282`, `lib.rs:1053-1069`) that is
    used in memory for the report only; absent it, the empty registry cannot
    promote any observation to trusted proof.
  - `registry render` (`lib.rs:1856-1914`) derives from canonical state. A supplied
    `--provider-policy` is applied to the projected assessments exactly as it is
    for `list`/`status`/`show`; with no policy it derives from the empty provider
    registry and warns `assessment_uses_empty_provider_registry` when plans exist
    (`lib.rs:1910-1912`).
  - `markdown render|inspect|import` (`lib.rs:1154-1281`): render optionally writes
    `--output`; `import` requires explicit `--state-root` (`lib.rs:1194-1201`) and
    creates one Draft via `store.create`.
- Envelope/stdout conventions: machine mode always emits the envelope to stdout;
  human mode prints `human` to stdout and diagnostics to stderr.
- Provider policy (`lib.rs:2095-2161`): 64 KiB cap, `deny_unknown_fields`,
  `schema_version == 1`, max 128 providers, 1–10 unique kinds per provider,
  sorted entries; failures are `invalid_policy/unknown_schema/policy_bound/
  duplicate_provider/duplicate_kind`. No credentials, no default trusted provider.
- Input hardening in `read_bounded` (`lib.rs:2167-2201`): symlink/non-file
  rejection (`unsafe_input`), pre- and post-read size caps (`input_too_large`).
- Error mapping in `repo_failure` (`lib.rs:2265-2326`): guarded-closure subject
  failures become stable machine codes `closure_subject_changed`,
  `closure_subject_drifted`, `closure_subject_unavailable`; revision conflicts,
  `plan_not_found plan_exists recovery_required corrupt_state invalid_plan`,
  and others are preserved 1:1.

## 4. Trust boundaries as implemented

- No execution/evidence acquisition: boundary script rejects
  `Command::new/.output/.spawn` in CLI source and any runtime/network/MCP use
  (`check-projection-cli-boundary.sh:20-28`); close delegates to
  `finalize_closure`, which recaptures the subject under its own lock.
- No `plans/registry.md` edits: `registry render` only reads `.eggplan` Plans and
  formats a derived `RegistryProjection` (`lib.rs:1856-1914`).
- Markdown import creates exactly one Draft Plan and never evidence, provider
  trust, `SubjectRevision`, or closure (`lib.rs:1248-1277`); `inspect` is read-only
  (`lib.rs:1232-1247`).

## 5. Test strategy

- Projection: `crates/eggplan-projection/tests/projections.rs` — stability,
  core-derived readiness/graph/summary, truncation flags, reason-code mapping,
  and versioned-envelope/bounded-warning assertions.
- CLI: `crates/eggplan-cli/tests/commands.rs` — binary integration via
  `CARGO_BIN_EXE_eggplan` with a `json_ok` helper asserting
  `schema_version == 1 && ok == true` (`crates/eggplan-cli/tests/commands.rs:21-32`): smoke
  read/mutate/registry test, loss-aware markdown render/inspect/import, explicit
  `--state-root` enforcement, help snapshot without ANSI, fail-closed provider
  policy, `recovery_required` without touching pending state, and the guarded
  close protocol.
- CLI completions: `crates/eggplan-cli/tests/batch_and_completions.rs` — the
  per-shell completion depth (commands, subcommands, options, option values) read from
  each generated script's candidate lists, the absence of a declared-but-unread shell
  variable, the match between the declared option value sets and the CLI's own `help`
  text, determinism, and the absence of repository or process side effects.
- CLI plan lifecycle: `crates/eggplan-cli/tests/plan_lifecycle.rs` — the `plan` family
  end to end through the binary: every legal `plan set-status` transition and the
  revision it produces, the transitions the core table forbids refused with
  `invalid_transition`, `--to closed` refused with `guarded_closure_required` while the
  plan stays Draft at revision 0, and `--expected-revision` required;
  `plan edit` changing structure while keeping the plan identity and status, and
  refusing a stale revision (`revision_conflict`), a mismatched plan id
  (`invalid_plan`), a status change (`invalid_transition`), and a Closed plan
  (`closed_plan_immutable`); `plan export` writing canonical JSON that `plan edit` then
  consumes byte for byte; the `plan` family appearing in the usage text and in all four
  completion scripts; and `markdown import` still creating only a Draft plan.

## 6. Review findings

Strengths: genuinely thin CLI (all domain math stays in core/projection);
deterministic sorted/truncated projections with explicit flags; read-only default
for `show/status/ready/graph/evidence/check/registry`; fail-closed policy and
input handling; stable closure-subject diagnostic codes.

Gaps/risks/surprises:

1. The `check` state vocabulary is 10 values and both documents now agree, so
   this is no longer an open gap: `assessment_state` (`lib.rs:2043-2065`) returns
   `complete`, `incomplete`, `unavailable`, `invalid_or_stale`, `in_flight`,
   `blocked`, `failed`, `inconclusive`, `awaiting_human_judgment`, and `check`
   adds `stale` (`lib.rs:1102-1103`); `cli-control-surface.md:90-97` lists the same
   ten. Worth stating explicitly: `closure_subject_stale` is a *reason code*
   (`lib.rs:1092`), not an eleventh state — it is what forces `stale`.
2. Stale-subject naming is asymmetric: `check` reports the reason code
   `closure_subject_stale` (`lib.rs:1088-1093`) while finalization reports
   `closure_subject_changed` (`lib.rs:2291-2294`). The distinction is now spelled
   out in `cli-control-surface.md:50-54`, so it is documented rather than
   accidental, but the two names still invite machine-consumer confusion.
3. Diagnostic codes are string literals scattered across `failure(...)` call sites
   plus `repo_failure`; there is no central code registry or stability test, so
   typo/drift risk is real. Pretty-printed (`to_string_pretty`) JSON envelopes
   (`lib.rs:283-301`) are also a mild surprise against the compact-canonical-JSON
   domain contract — fine for envelopes, but worth stating explicitly. Truncation
   caps are duplicated between projection (`crates/eggplan-projection/src/lib.rs:12-16`) and CLI
   (`lib.rs:66-71`, `retain: Some(100)` at `lib.rs:709,1061`) rather than shared.
4. There are now two arbitrary-path writes in the CLI — `markdown render --output`
   (`lib.rs:1171-1176`) and `plan export --output` (`lib.rs:1511-1514`) — and both
   are unchecked: no containment, no symlink rejection, and no size cap, unlike
   `read_bounded` (`lib.rs:2167-2201`). So `registry render` indeed never edits
   `plans/registry.md`, but `markdown render PLAN --output plans/registry.md` — and
   `plan export PLAN --output plans/registry.md` — can still clobber it. The
   "cannot edit the registry" property holds for the projection path only, not
   for the process as a whole. The severity is unchanged by the second site: both
   write caller-named paths with plan-derived bytes, neither creates evidence,
   provider trust, a subject revision, or a closure, and a clobbered file is
   recoverable only by the user. The count of such writes changed, not their class.
5. `check` is described as read-only, but `--recover-pending` deliberately
   takes the mutating open path (`lib.rs:1030`, `lib.rs:1050`) to run
   `recover_pending_closures` (`crates/eggplan-repo/src/store.rs:273`). This
   matches `cli-control-surface.md:99-101` and is the intended recovery contract,
   but "read-only" is a default rather than an invariant and should not be
   restated without that qualifier.

## Verification pointers

```sh
cargo test -p eggplan-projection
cargo test -p eggplan-cli
bash scripts/check-projection-cli-boundary.sh
```
