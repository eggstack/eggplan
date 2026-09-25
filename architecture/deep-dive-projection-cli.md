# Deep dive: projection + CLI

See also [overview](overview.md) and [CLI control surface](cli-control-surface.md).
Roadmap: `plans/subsystems/projection-cli-roadmap.md` — M001/M002 closed, M003 deferred.

## 1. Roles

`eggplan-projection` (`crates/eggplan-projection/src/lib.rs`) is the reusable,
dependency-free derived-view library. It depends only on `eggplan-core` + `serde`
(`crates/eggplan-projection/Cargo.toml:8-10`) and owns bounded summaries,
deterministic ordering, truncation flags, and the stable JSON envelope shape.

`eggplan-cli` (`crates/eggplan-cli/src/lib.rs`, `crates/eggplan-cli/src/main.rs`)
is the thin command adapter over `eggplan-core`, `eggplan-repo`,
`eggplan-projection`, and `eggplan-markdown`
(`crates/eggplan-cli/Cargo.toml:8-14`). `main.rs:1-3` only forwards argv to
`eggplan_cli::run`. Neither crate may execute work, acquire evidence, run a
network/runtime/MCP service, or edit `plans/registry.md` — enforced by
`scripts/check-projection-cli-boundary.sh:10-30`.

## 2. Projection walkthrough

- Bounds: `OUTPUT_SCHEMA_VERSION = 1`, `MAX_PROJECTED_PLANS = 100`,
  `MAX_PROJECTED_ITEMS = 100`, `MAX_TEXT_CHARS = 512`, `MAX_WARNINGS = 32`
  (`crates/eggplan-projection/src/lib.rs:12-16`).
- Envelope: `OutputEnvelope<T>` with `schema_version/command/ok/data/error/warnings`
  and `deny_unknown_fields` (`lib.rs:18-27`). `success` bounds warnings
  (`lib.rs:37-46`); `failure` char-truncates the message to 512
  (`lib.rs:48-64`).
- `summarize_plan` (`lib.rs:187-222`): truncates objective, counts item statuses
  via `item_status_code` (`lib.rs:430-439`), sorts/dedups assessment reason codes
  via `reason_code` (`lib.rs:441-470`), reports `projected_item_count` /
  `items_truncated`.
- `readiness_projection` (`lib.rs:224-259`): derives per-item readiness from core
  `readiness()`, sorts by `(position, item_id)`, counts
  `ready/waiting/not_actionable`, truncates to 100 with `truncated` + `total_items`.
- `graph_projection` (`lib.rs:261-322`): sorts nodes by `(position, id)`, takes 100,
  keeps only intra-selection dependency/parent edges (sorted), reports
  `total_*_edges` vs `edges_truncated` plus node-level `truncated`.
- `registry_projection` (`lib.rs:324-392`): sorts plans by `PlanId`, collects
  blockers in position order (truncated at 100), takes 100 plans, computes
  per-plan ready counts from core `readiness()`, truncates reason codes at 32.
- Helpers: char-based `truncate` (`lib.rs:472-476`) and `bound_warnings`
  (`lib.rs:478-484`). All ordering uses `BTreeMap`/`BTreeSet`/explicit sorts, so
  output is deterministic for identical canonical input.

## 3. CLI walkthrough

- Entry: `run` prints pretty JSON envelope to stdout on success
  (`crates/eggplan-cli/src/lib.rs:225-241`) and a JSON error envelope (exit 2) in
  `--json` mode, else `command: code: message` to stderr
  (`lib.rs:242-256`). `execute` defaults `--state-root` to `.eggplan`
  (`lib.rs:258-266`).
- Arg parsing is hand-rolled: `parse_args` (`lib.rs:268-328`), value options
  (`lib.rs:330-343`), per-command allow-lists in `validate_command_options`
  (`lib.rs:490-562`), usage string (`lib.rs:1600-1602`).
- Command groups in `dispatch` (`lib.rs:345-488`):
  - `init` opens/creates the store (`lib.rs:349-358`); `new` parses strict Plan
    JSON, requires revision 0 + Draft, then `create` (`lib.rs:359-388`).
  - `show` composes `PlanDetail` from `summarize_plan` plus
    `readiness_projection(...).items` (`lib.rs:389-411`); `status` caps at 100
    plans with `total_plans/truncated` and per-plan
    `current_subject_unavailable` warnings (`lib.rs:412-445`); `ready`
    (`lib.rs:446-459`) and `graph` (`lib.rs:460-475`; human output is a bare
    newline-joined ID list) are pure reads via `open_read_only`.
  - `activate` / `item update` do explicit `--expected-revision` pre-checks, then
    `compare_and_swap` (`lib.rs:839-883`, `lib.rs:885-973`).
  - `evidence list|show|supersessions` are read-only briefs capped at 100 rows
    (`lib.rs:975-1062`; caps `lib.rs:27-32`; brief shape `lib.rs:86-97`).
  - `assess` requires `--provider-policy`, reads via `compute_assessment`
    (`lib.rs:1064-1075`, `lib.rs:1258-1286`); `assessment_data` caps at 100 items
    / 32 reasons per row (`lib.rs:1288-1321`).
  - `close` requires `--expected-revision` + `--provider-policy`, pre-checks
    revision, builds `ClosureCandidate`, then `finalize_closure` with a generated
    `ClosureId` — no caller-supplied subject (`lib.rs:1077-1132`).
    `closure show` returns the bounded `ClosureSummary` (`lib.rs:1134-1164`,
    `lib.rs:1347-1358`).
  - `check` (`lib.rs:564-708`) opens read-only (`lib.rs:1443-1445`), fails with
    `recovery_required` unless `--recover-pending` is explicit (`lib.rs:572-596`),
    classifies per plan via `assessment_state` (`lib.rs:1323-1345`), flags stale
    closures (`lib.rs:647-656`), and warns on unavailable subjects and abandoned
    staging files.
  - `registry render` (`lib.rs:1166-1221`) derives from canonical state with an
    empty provider registry and always warns
    `assessment_uses_empty_provider_registry` when plans exist (`lib.rs:1216-1219`).
  - `markdown render|inspect|import` (`lib.rs:710-837`): render optionally writes
    `--output`; `import` requires explicit `--state-root` (`lib.rs:750-757`) and
    creates one Draft via `store.create`.
- Envelope/stdout conventions: machine mode always emits the envelope to stdout;
  human mode prints `human` to stdout and diagnostics to stderr.
- Provider policy (`lib.rs:1375-1441`): 64 KiB cap, `deny_unknown_fields`,
  `schema_version == 1`, max 128 providers, 1–10 unique kinds per provider,
  sorted entries; failures are `invalid_policy/unknown_schema/policy_bound/
  duplicate_provider/duplicate_kind`. No credentials, no default trusted provider.
- Input hardening in `read_bounded` (`lib.rs:1447-1481`): symlink/non-file
  rejection (`unsafe_input`), pre- and post-read size caps (`input_too_large`).
- Error mapping in `repo_failure` (`lib.rs:1545-1591`): guarded-closure subject
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
  formats a derived `RegistryProjection` (`lib.rs:1166-1221`).
- Markdown import creates exactly one Draft Plan and never evidence, provider
  trust, `SubjectRevision`, or closure (`lib.rs:804-833`); `inspect` is read-only
  (`lib.rs:788-803`).

## 5. Test strategy

- Projection: `crates/eggplan-projection/tests/projections.rs` — stability,
  core-derived readiness/graph/summary, truncation flags, reason-code mapping.
- CLI: `crates/eggplan-cli/tests/commands.rs` — binary integration via
  `CARGO_BIN_EXE_eggplan` with a `json_ok` helper asserting
  `schema_version == 1 && ok == true` (`commands.rs:21-32`): smoke
  read/mutate/registry test, loss-aware markdown render/inspect/import, explicit
  `--state-root` enforcement, help snapshot without ANSI, fail-closed provider
  policy, `recovery_required` without touching pending state, and the guarded
  close protocol.

## 6. Review findings

Strengths: genuinely thin CLI (all domain math stays in core/projection);
deterministic sorted/truncated projections with explicit flags; read-only default
for `show/status/ready/graph/evidence/check/registry`; fail-closed policy and
input handling; stable closure-subject diagnostic codes.

Gaps/risks/surprises:

1. `assessment_state` (`lib.rs:1323-1345`) can emit `"inconclusive"`, which is not
   in the `check` classification list in `cli-control-surface.md:63-70`. Either
   the doc or the code needs updating; machine consumers matching the doc list
   will see an undocumented state.
2. Stale-subject naming is asymmetric: `check` reports `closure_subject_stale`
   (`lib.rs:647-656`) while finalization reports `closure_subject_changed`
   (`lib.rs:1567-1570`). Consider aligning names or documenting the distinction.
3. Diagnostic codes are string literals scattered across `failure(...)` call sites
   plus `repo_failure`; there is no central code registry or stability test, so
   typo/drift risk is real. Pretty-printed (`to_string_pretty`) JSON envelopes
   (`lib.rs:231-249`) are also a mild surprise against the compact-canonical-JSON
   domain contract — fine for envelopes, but worth stating explicitly. Truncation
   caps are duplicated between projection (`lib.rs:12-16`) and CLI
   (`lib.rs:27-32`, `take(100)` at `lib.rs:426,684`) rather than shared.

## Verification pointers

```sh
cargo test -p eggplan-projection
cargo test -p eggplan-cli
bash scripts/check-projection-cli-boundary.sh
```
