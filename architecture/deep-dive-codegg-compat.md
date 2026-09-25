# Deep dive: `eggplan-codegg-compat`

See also [overview.md](overview.md) and [codegg-compat.md](codegg-compat.md)
(this file is the review; that file is the contract).

## 1. Crate role: pure one-way bridge

`eggplan-codegg-compat` is a single-file pure mapping/assessment view
(`crates/eggplan-codegg-compat/src/lib.rs`, ~909 lines). It owns no runtime,
no WorkOrder/scheduler state, no storage, and no evidence acquisition:

- Production deps are `eggplan-core`, `serde`, `serde_json` only
  (`crates/eggplan-codegg-compat/Cargo.toml:8-11`). `eggplan-repo` and
  `tempfile` appear only under `[dev-dependencies]`
  (`Cargo.toml:13-15`) for test-only snapshot persistence.
- The `create_snapshot` helper the `MappedPlan` doc comment mentions
  (`src/lib.rs:285-287`) lives in `tests/parity.rs:22-44`, not in `src` —
  matching M002 §3 (repository coupling moved to test-only).
- Source grep confirms `src/` contains no `register_trusted`,
  `SubjectCapture`, `finalize_closure`, `PlanStore`, `eggplan-repo`,
  `tokio`/`reqwest`/`Command` references. The boundary script enforces the
  dependency and ownership edges (`scripts/check-codegg-compat-boundary.sh:9-27`):
  no `eggplan-repo` in `[dependencies]`, no CodeGG dependency, no
  `WorkOrder`/`Goal`/`TodoState`/`Checkpoint`/`ContextEpoch`/`AgentRunExecutor`/
  `JobExecutor`/`WorktreePolicy`/`SandboxPolicy` declarations.
- CodeGG owns WorkPlan model/statuses, bounded projections, CAS storage, and
  all runtime (checkpoint, context-epoch, Todo, Goal, WorkOrder, AgentRun/Job,
  arbiter control flow) per `architecture/codegg-compat.md:11-17` and the
  M002 plan §9. This crate only maps a bounded snapshot DTO and assesses it.

## 2. Entry-point walkthrough

- `normalize_snapshot` (`src/lib.rs:429-692`) is the live mapper. It takes a
  parsed `CodeggPlanSnapshot` (`lib.rs:42-54`), an exact host-supplied
  `SubjectRevision`, and a host `EvidenceResolver` (`lib.rs:147-153`). It
  validates bounds (revision/item count at `lib.rs:434`, per-item limits at
  `lib.rs:465-472`, text/ID/detail sizes via `lib.rs:344-375`), maps IDs,
  resolves each evidence ref through the host, builds `EvidenceRequirement`s
  plus host observations, then emits a revision-zero `Plan` with provenance
  (`lib.rs:644-660`) and a digest-pinned `MappingManifest` (`lib.rs:662-683`).
- `normalize_fixture` (`lib.rs:411-425`) is the qualification wrapper: it
  rejects anything that is not `schema_version == 1`, `source_repository ==
  "codegg"`, `source_sha == SOURCE_CODEGG_SHA` (`lib.rs:14,416-421`), then
  calls `normalize_snapshot` (`lib.rs:424`). Fixture SHA is provenance
  gating, never runtime authority — as specified in M002 §3 and
  `architecture/codegg-compat.md:19-28`.
- `assess_codegg_snapshot` (`lib.rs:696-734`) is the pure live assessment
  bridge. It normalizes, then applies the source lifecycle
  (`Active`/`Completed` → `PlanStatus::Active`, `Blocked` → `Blocked`,
  `Cancelled` → `Cancelled` at `lib.rs:703-707`), calls core `assess_plan`
  (`lib.rs:708`), folds the status via `completion_family` (`lib.rs:709`),
  and collects sorted/deduped stable reason-code strings from plan, item,
  and criterion reasons (`lib.rs:710-726`). It writes no repository state.
- Verification-digest handling: the bridge never mints a digest. Execution
  kinds must arrive with `expected_verification` equal to the observation's
  own binding, else mapping fails closed (`lib.rs:566-575`). `src/` contains
  no call to core's `verification_digest` helper (core defines it in
  `crates/eggplan-core/src/schema.rs:22`); derivation is a CodeGG-host duty.
- `project_bounded` (`lib.rs:767-909`) is a CodeGG-facing display projection:
  clamp `max_items` to 1..=8 and `max_text_chars` to 1..=200 (`lib.rs:773-774`),
  rank current item first then by status/position/id (`lib.rs:777-796`),
  exclude completed/cancelled history (`lib.rs:827-834`), truncate
  (`lib.rs:861`), and label with the folded family string (`lib.rs:899`).

## 3. Mapping contract as implemented

- **ID derivation** (`lib.rs:335-342,445-450,488-502`): `ep_<32 hex>` plan IDs
  and `epi_<32 hex>` item IDs from `digest_json(("eggplan-codegg-compat",
  namespace, source))`, truncated to 32 hex chars. Duplicate source IDs
  rejected (`lib.rs:493-498`), derived collisions rejected (`lib.rs:490-492`),
  manifest re-checks duplicate identities plus a content digest
  (`lib.rs:252-282`).
- **Lifecycle**: item statuses map 1:1 with no transition replay
  (`lib.rs:400-409`); plan `Completed` maps to `Active` and always records
  `CompletedPlanNeedsGuardedClose` (`lib.rs:458-460,703-704`) — only Evidence
  M002 may close it. `Cancelled` maps to cancelled intent (`lib.rs:706`).
- **Ref-to-requirement mapping** (`lib.rs:377-387,552-587`): `TestJob`→`Test`,
  `DelegatedRun`/`AgentRun`→`DelegatedRun`, `SchedulerJob`→`Command`,
  `Artifact`→`Artifact`, `Commit`→`Revision`. Each ref becomes one
  `EvidenceRequirement` with `SubjectPolicy::Exact`, provider `None`,
  `allow_human_judgment: false` (`lib.rs:576-585`). Resolver output is
  guarded: subject/kind must match and observation IDs must be unique
  (`lib.rs:560-565`).
- **`RequiresUserJudgment`** (`lib.rs:596-608`): human criteria get empty
  requirements, so ordinary refs cannot satisfy them; non-human criteria
  share the item's full requirement set (item-scoped assignment).
- **Lossy diagnostics** (`Loss`, `lib.rs:155-165`): `SourceRevisionIsProvenanceOnly`
  always recorded (`lib.rs:457`); owner IDs → provenance only (`lib.rs:503-512`);
  serialized `Satisfied` → claim, never evidence (`lib.rs:513-516`); notes/details
  omitted (`lib.rs:522-542`); multi-acceptance sharing item refs flagged
  (`lib.rs:588-590`). The five-family fold (`lib.rs:192-205`) keeps the full
  Eggplan assessment intact and folds failed/missing/stale/inconclusive into
  `ActionableWorkRemaining` for the compat display only.

## 4. Fixture qualification strategy and baselines

- Corpus manifest (`tests/fixtures/manifest.json:1-10`) pins `schema_version 1`,
  `source_repository "codegg"`, `source_sha
  a3c87fc18ee55aaf630401a562c11bb83112fd82`, and exactly three files:
  `work_plan_foundation.json` (satisfied-acceptance-requires-host-evidence),
  `work_plan_projection_arbiter.json` (bounded current/actionable ordering),
  `long_horizon_trajectory_qualification.json` (dependency gating, no hidden
  reasoning). Each fixture records its own `source_case` naming the upstream
  CodeGG test.
- `tests/parity.rs` (~740 lines) covers: live mapping without fixture
  provenance plus pure assessment (`parity.rs:202-227`); strict corpus/SHA/
  unknown-field rejection and deterministic identity mapping (`parity.rs:229-296`);
  `Satisfied`/owner/completed-label non-authority and provider-trust gating
  (`parity.rs:298-360`); completed-without-proof stays incomplete
  (`parity.rs:362-386`); stale/unbound/mismatched bindings fail closed
  (`parity.rs:388-398`); judgment stays pending and family folding is edge-only
  (`parity.rs:400-438`); bounded projection stability (`parity.rs:440-467`);
  CAS creation, stale-writer conflict, restart (`parity.rs:469-499`);
  cancel-vs-guarded-close single CAS winner (`parity.rs:501-565`); dependency
  mapping and input bounds (`parity.rs:567-604`); artifact/empty-acceptance
  non-completeness (`parity.rs:606-648`); explicit plan/item status tables
  (`parity.rs:650-740`).
- Recorded baselines: M002 planning baseline `a3c87fc…` (fixture SHA),
  blocker/interface recheck `f4e6e69…`, upstream provenance registration
  `af0a3e0…`, provenance implementation/closure `418fdc8…` with CodeGG CI run
  `36106606574`, Eggplan bridge `088968b` — per the subsystem roadmap §M002
  and the M002 plan header. The `architecture/codegg-compat.md:11-14` recheck
  notes `origin/main 5f45326…` left model/assessment/storage interfaces unchanged.

## 5. Review findings

**Strengths.** The M002 §3 refactor is done: live vs fixture paths are split,
production deps are minimal, `create_snapshot` is test-only, the digest
prohibition is structural (the bridge cannot mint digests, only compare
host-supplied bindings), and `Satisfied`/owner/completed labels are
provably non-authoritative in tests. Strict `deny_unknown_fields` DTOs,
explicit `MAX_*` bounds (`lib.rs:13-21`), and digest-pinned manifests make
the mapping auditable.

**Gaps / risks (what M002 still leaves open).**

1. Differential adoption is still ahead: `assess_codegg_snapshot` is a pure
   view; CodeGG has not yet swapped its assessor behind the existing surface
   (M002 plan §§7–8, acceptance criteria 4–5). The upstream subject-provenance
   handoff closed the M001 blocker, but verification-digest derivation and the
   parity matrix remain M002 work (roadmap §M002, plan §21).
2. Trust boundary is caller-side: the `EvidenceResolver` trait
   (`lib.rs:147-153`) is arbitrary host code. In-crate guards (subject/kind/ID
   uniqueness at `lib.rs:560-565`, binding equality at `lib.rs:566-575`) fail
   closed on a faulty resolver, but resolver and `ProviderRegistry` correctness
   are assumed, not verified, by this crate.
3. `architecture/codegg-compat.md:23-28` previously read as though the
   bridge called the shared `verification_digest` helper. Resolved
   2026-09-25: reworded as an explicit host obligation (the crate only
   compares host-supplied bindings at `lib.rs:566-575`).
4. `MappedPlan` docs (`lib.rs:285-287`) referenced `create_snapshot` as though
   it were adjacent API. Resolved 2026-09-25: the doc comment now names the
   test-only location (`tests/parity.rs`).
5. `hash_id` truncates SHA-256 hex to 32 chars (`lib.rs:341`); collisions are
   rejected per-run and duplicates per-manifest, but `MappingManifest::validate`
   does not re-derive IDs from sources — determinism rests on the parity test
   (`parity.rs:272-282`), not on the validator. Acceptable but worth stating.
6. `project_bounded` clamps and history-exclusion (`lib.rs:773-774,827-834`)
   are CodeGG-view choices, not core projection authority; the folded
   `assessment_reason_code` string (`lib.rs:899`) is lossy by design with detail
   retained only in `CodeggAssessmentBridgeResult::reason_codes`. No issue found,
   but callers must not treat the projection label as the assessment.

## Verification pointers

```sh
cargo test -p eggplan-codegg-compat --locked
cargo test -p eggplan-codegg-compat --locked --test parity
bash scripts/check-codegg-compat-boundary.sh
```
