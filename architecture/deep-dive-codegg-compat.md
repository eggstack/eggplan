# Deep dive: `eggplan-codegg-compat`

See also [overview.md](overview.md) and [codegg-compat.md](codegg-compat.md)
(this file is the review; that file is the contract).

## 1. Crate role: pure two-direction bridge

`eggplan-codegg-compat` is a single-file pure mapping/assessment view
(`crates/eggplan-codegg-compat/src/lib.rs`, 1118 lines, and it carries no
`#[cfg(test)]` module). It owns no runtime,
no WorkOrder/scheduler state, no storage, and no evidence acquisition:

- Production deps are `eggplan-core`, `serde`, `serde_json` only
  (`crates/eggplan-codegg-compat/Cargo.toml:8-11`). `eggplan-repo` and
  `tempfile` appear only under `[dev-dependencies]`
  (`Cargo.toml:13-15`) for test-only snapshot persistence.
- The `create_snapshot` helper the `MappedPlan` doc comment mentions
  (`src/lib.rs:494-496`) lives in `tests/parity.rs:23-45`, not in `src` —
  matching M002 §3 (repository coupling moved to test-only).
- Source grep confirms `src/` contains no `register_trusted`,
  `SubjectCapture`, `finalize_closure`, `PlanStore`, `eggplan-repo`,
  `tokio`/`reqwest`/`Command` references, and no `repository_id` string at
  all. `src/` names `ProviderRegistry` only as an imported type and an
  `&ProviderRegistry` parameter (`src/lib.rs:7,909`); it never constructs one.
  The test files do enroll trust — `tests/parity.rs:185,380` call
  `register_trusted` on a host-built registry, and `tests/parity.rs:13`
  imports the real `eggplan_repo` store — so the prohibition is a
  production-source rule, not a whole-crate one.
- The boundary script enforces the dependency and ownership edges with FIVE
  guards, and only the first is `[dependencies]`-scoped
  (`scripts/check-codegg-compat-boundary.sh`). Each guard is a named function
  and the header at `:20-42` states all five with their scan scope:
  - `no_production_repo_dep` (`:45-52`) `awk` — `eggplan-repo` must not appear
    under `[dependencies]`; it deliberately allows the `[dev-dependencies]`
    entry at `Cargo.toml:14`.
  - `no_codegg_dependency` (`:56-58`) `rg` over the whole manifest — any
    `codegg`/`codegg-core` dependency line, in *any* section, fails; it is not
    section-scoped.
  - `no_client_dependency` (`:62-64`) `rg` over the whole manifest —
    `tokio`/`sqlx`/`reqwest`/`hyper`/`ureq`/`surf`/`isahc`/`async-std`
    dependency lines, also not section-scoped.
  - `no_owned_identity` (`:69-71`) `rg` over `src/` — a
    `pub struct|enum|trait|type` declaration named `WorkOrder`, `Goal`,
    `GoalVerification`, `TodoState`, `WorkPlanCheckpoint`, `ContextEpoch`,
    `AgentRunExecutor`, `JobExecutor`, `WorktreePolicy`, or `SandboxPolicy`. It
    catches *declarations*, not mentions, so a CodeGG type named in a doc
    comment or a private field type is not a failure.
  - `no_impure_source` (`:77-79`) `rg` over `src/` — no process, **filesystem**,
    network, or database access in production source. It matches both the
    fully-qualified and the `use`-imported call forms, so `std::fs::read` and a
    bare `fs::read` behind `use std::fs` are both caught, plus `File::open`,
    `File::create`, `std::path`, and `tempfile::`. Filesystem coverage was added
    by corrective C002; before it the alternation covered only process/network/
    database, leaving on-disk I/O in `src/` unenforced.
- `run_guards` (`:81-109`) applies all five to the real manifest and source tree
  and exits on the first failure. Deterministic synthetic self-proofs
  (`:114-260`) run the same functions against fixtures in a `mktemp -d`
  directory, so every guard has a negative and a positive case and no tracked
  file is ever modified; cleanup is a targeted non-recursive `rm -f`/`rmdir`.
  Proof coverage: production-vs-dev `eggplan-repo`, `codegg` in both sections,
  all eight client crates, all ten owned identities as `struct`/`enum`/`trait`,
  incidental mentions (comment, private struct, `pub const` string), and
  eighteen process/filesystem/network/database source forms.
- CodeGG owns WorkPlan model/statuses, bounded projections, CAS storage, and
  all runtime (checkpoint, context-epoch, Todo, Goal, WorkOrder, AgentRun/Job,
  arbiter control flow) per `architecture/codegg-compat.md:71-85` and the
  M002 plan §9. This crate only maps a bounded snapshot DTO and assesses it.

## 2. Entry-point walkthrough

- `normalize_snapshot` (`src/lib.rs:638-901`) is the live mapper. It takes a
  parsed `CodeggPlanSnapshot` (`lib.rs:252-262`), an exact host-supplied
  `SubjectRevision`, and a host `EvidenceResolver` (`lib.rs:355-361`). It
  validates bounds (revision/item count at `lib.rs:643`, per-item limits at
  `lib.rs:674-681`, text/ID/detail sizes via `lib.rs:553-584`), maps IDs,
  resolves each evidence ref through the host, builds `EvidenceRequirement`s
  plus host observations, then emits a revision-zero `Plan` with provenance
  (`lib.rs:853-868`) and a digest-pinned `MappingManifest`
  (`lib.rs:871-892`).
- `normalize_fixture` (`lib.rs:620-634`) is the qualification wrapper: it
  rejects anything that is not `schema_version == 1`, `source_repository ==
  "codegg"`, `source_sha == SOURCE_CODEGG_SHA` (`lib.rs:14,625-630`), then
  calls `normalize_snapshot` (`lib.rs:633`). Fixture SHA is provenance
  gating, never runtime authority — as specified in M002 §3 and
  `architecture/codegg-compat.md:8-11`.
- `assess_codegg_snapshot` (`lib.rs:905-943`) is the pure live assessment
  bridge. It normalizes, then applies the source lifecycle
  (`Active`/`Completed` → `PlanStatus::Active`, `Blocked` → `Blocked`,
  `Cancelled` → `Cancelled` at `lib.rs:912-916`), calls core `assess_plan`
  (`lib.rs:917`), folds the status via `completion_family` (`lib.rs:918`),
  and collects sorted/deduped stable reason-code strings from plan, item,
  and criterion reasons (`lib.rs:919-935`). It writes no repository state.
- Verification-digest handling: the bridge never mints a digest. Execution
  kinds must arrive with `expected_verification` equal to the observation's
  own binding, else mapping fails closed (`lib.rs:775-784`). The
  precondition is a kind allowlist in this crate's code, not a host
  assertion: `requires_verification_binding` (`lib.rs:598-607`) returns
  `true` only for `Command`, `Test`, `StaticAnalysis`, `DelegatedRun`, and
  `Benchmark`, so `Artifact` and `Revision` (from CodeGG `Artifact`/`Commit`,
  `lib.rs:586-596`) are exempt by construction and pass with
  `expected_verification: None`.
  Correction to an earlier reading of this file: `src/` **does** call core's
  `digest_json` (imported at `lib.rs:8`) — for derived plan/item IDs
  (`lib.rs:545`), the mapping-manifest content digest (`lib.rs:474,882`), and
  the M003 projection digests (`lib.rs:191-205`). What it never does is
  construct a `VerificationDigest`; there is no `VerificationDigest::new`
  call in `src/`, and the bridge only compares a host-supplied pair. CodeGG
  discharges the derivation duty in `src/work_plan_eggplan.rs` by calling
  `eggplan_core::digest_json` over its own `CodeggVerificationSpecV1` and
  formatting the result with `VerificationDigest::new`.
- `project_bounded` (`lib.rs:976-1118`) is a CodeGG-facing display projection:
  clamp `max_items` to 1..=8 and `max_text_chars` to 1..=200 (`lib.rs:982-983`),
  rank current item first then by status/position/id (`lib.rs:986-1005`),
  exclude completed/cancelled history (`lib.rs:1038-1043`), truncate
  (`lib.rs:1070`), and label with the folded family string (`lib.rs:1108`).

## 3. Mapping contract as implemented

- **ID derivation** (`lib.rs:544-551,654-655,697-698`): `ep_<32 hex>` plan IDs
  and `epi_<32 hex>` item IDs from `digest_json(("eggplan-codegg-compat",
  namespace, source))`, truncated to 32 hex chars — a namespace-separated
  SHA-256, so the same source ID yields the same Eggplan ID across runs and
  the plan/item namespaces cannot collide. Duplicate source IDs
  rejected (`lib.rs:702-707`), derived collisions rejected (`lib.rs:699-701`),
  manifest re-checks duplicate identities plus a content digest
  (`lib.rs:461-489`).
- **Lifecycle**: item statuses map 1:1 with no transition replay
  (`lib.rs:609-618`); plan `Completed` maps to `Active` (`lib.rs:913`) and
  always records `CompletedPlanNeedsGuardedClose` (`lib.rs:667-669`) — only
  Evidence M002 may close it. `Cancelled` maps to cancelled intent
  (`lib.rs:915`).
- **Ref-to-requirement mapping** (`lib.rs:586-596,764-796`): `TestJob`→`Test`,
  `DelegatedRun`/`AgentRun`→`DelegatedRun`, `SchedulerJob`→`Command`,
  `Artifact`→`Artifact`, `Commit`→`Revision`. Each ref becomes one
  `EvidenceRequirement` with `SubjectPolicy::Exact`, provider `None`,
  `EvidenceCardinality::Any`, `min_count: 1`, and
  `allow_human_judgment: false` (`lib.rs:785-794`). Resolver output is
  guarded: subject/kind must match and observation IDs must be unique
  (`lib.rs:769-774`).
- **`RequiresUserJudgment`** (`lib.rs:805-818`): human criteria get empty
  requirements, so ordinary refs cannot satisfy them; non-human criteria
  share the item's full requirement set (item-scoped assignment).
- **Lossy diagnostics** (`Loss`, `lib.rs:365-373`): `SourceRevisionIsProvenanceOnly`
  always recorded (`lib.rs:666`); owner IDs → provenance only
  (`lib.rs:712-721`); serialized `Satisfied` → claim, never evidence
  (`lib.rs:722-725`); notes/details omitted (`lib.rs:731-737,744-751`);
  multi-acceptance sharing item refs flagged (`lib.rs:797-799`). The
  five-family fold (`lib.rs:400-413`) keeps the full Eggplan assessment
  intact in `PlanAssessment` and folds
  failed/missing-or-unavailable/stale/inconclusive into
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
- `tests/parity.rs` (948 lines, 19 tests) covers: live mapping without fixture
  provenance plus pure assessment (`parity.rs:204`); rejection of the superseded
  M001 fixture SHA without relabelling, with the same snapshot body still
  mapping live (`parity.rs:234`); strict corpus/SHA/unknown-field rejection and
  deterministic identity mapping (`parity.rs:263`);
  `Satisfied`/owner/completed-label non-authority and provider-trust gating
  (`parity.rs:332`); completed-without-proof stays incomplete
  (`parity.rs:396`); stale/unbound/mismatched bindings fail closed
  (`parity.rs:422`); every execution kind (TestJob/SchedulerJob/DelegatedRun/
  AgentRun) completing only on a matching authoritative binding, and failing
  closed per kind under missing/mismatched/stale faults (`parity.rs:451`);
  Artifact/Commit completing with no execution binding at all
  (`parity.rs:525`); the five-family fold retaining Eggplan's detailed
  `evidence_status` / `untrusted_provider*` reason codes (`parity.rs:563`);
  judgment staying pending and family folding being edge-only (`parity.rs:609`);
  bounded projection stability; CAS creation, stale-writer conflict, restart;
  cancel-vs-guarded-close single CAS winner; dependency mapping and input
  bounds; artifact/empty-acceptance non-completeness; explicit plan/item status
  tables.
- Recorded baselines: M002 planning baseline `a3c87fc…` (fixture SHA),
  blocker/interface recheck `f4e6e69…`, upstream provenance registration
  `af0a3e0…`, provenance implementation/closure `418fdc8…` with CodeGG CI run
  `36106606574`, Eggplan bridge `088968b`, pinned-by-CodeGG Eggplan head
  `0d4a6af…`, CodeGG M002 implementation `3e992291` + hermeticity follow-up
  `3c7438c7`, CodeGG M002 closure `ffa1c15e` with hosted canonical run
  `36760308368` — per the subsystem roadmap §M002, the M002 plan header, and
  `plans/closure/codegg-integration/002-closed.md`.

The earlier `architecture/codegg-compat.md` recheck note that cited
`origin/main 5f45326…` as leaving "model/assessment/storage interfaces
unchanged" was accurate for `crates/codegg-core/src/work_plan/` but was read as
a claim that no WorkPlan path had moved. Provenance, verified by read-only
git: the note was introduced with the live bridge at `088968b` and removed at
`1291799`, so the current `architecture/codegg-compat.md` no longer
carries it and this paragraph is the surviving record. `5f4532659…` was
`dbowm91/codegg` `origin/main` at that time; the same commit is separately
pinned as the reviewed CodeGG head for the Markdown CodeGG subset
(`crates/eggplan-markdown/tests/fixtures.rs:9-16`,
`architecture/deep-dive-markdown.md:6`), which is a different subsystem reusing
one upstream baseline, not a second CodeGG baseline. It is not a valid object in
this repository's own history, so it cannot be checked locally.
It is now stated precisely against the
M002-adopted head `ffa1c15e…`: the `codegg-core` WorkPlan module is
byte-identical from `a3c87fc…` to `ffa1c15e…`, and everything M002 touched is
application-layer. `tests/long_horizon_trajectory_qualification.rs` did change
in that range, but only by an upstream import reformat plus an added
`ExecutionTarget::default()` field from the M001 provenance work — the
dependency-gating and no-hidden-reasoning behavior the fixture records is
unchanged, so the fixture remains valid provenance.

## 5. Review findings

**Strengths.** The M002 §3 refactor is done: live vs fixture paths are split,
production deps are minimal, `create_snapshot` is test-only, the digest
prohibition is structural (the bridge cannot mint digests, only compare
host-supplied bindings), and `Satisfied`/owner/completed labels are
provably non-authoritative in tests. Strict `deny_unknown_fields` DTOs,
explicit `MAX_*` bounds (`lib.rs:13-21`), and digest-pinned manifests make
the mapping auditable.

**Gaps / risks (what M002 still leaves open).**

1. ~~Differential adoption is still ahead.~~ Resolved 2026-09-30: CodeGG
   landed M002 in `3e992291` (facade, canonical verification-spec derivation,
   resolved-evidence adapter, explicit engine selection, 28-case differential
   matrix, production call-site migration, S1/S2 completion revalidation) and
   closed it in `ffa1c15e` with hosted canonical run `36760308368` green on the
   exact head. `assess_codegg_snapshot` is now consumed in production behind
   CodeGG's unchanged `WorkPlanCompletionAssessment` surface. Stale as first
   recorded: "What remains on the Eggplan side is M003" is no longer true. M003
   closed on both sides — Eggplan contract implementation `3f7c603` (hosted run
   `36868055136`), CodeGG consumer `53dea47f` (hosted run `36938461935`) — and
   M003 C001 closed the dirty-subject fingerprint and bound-evidence
   requalification at Eggplan `0dd33b7` (run `37063328954`) and CodeGG
   `36ec9322`, with `0dd33b7` pinned and consumed in `3623f65e`/`b470865a`
   (PR `dbowm91/codegg#90`, hosted `CI` `37084905013` green on `main`).
   `plans/registry.md:79-81` and
   `plans/subsystems/codegg-integration-roadmap.md:3` agree: M001–M003 and C001
   closed, roadmap terminal. Note the two distinct C001s in the registry — CodeGG
   M003 C001 (`plans/registry.md:81`) and Evidence M002 C001
   (`plans/registry.md:72`) are unrelated rows.
2. Trust boundary is caller-side: the `EvidenceResolver` trait
   (`lib.rs:355-361`) is arbitrary host code. In-crate guards (subject/kind/ID
   uniqueness at `lib.rs:769-774`, binding equality at `lib.rs:775-784`) fail
   closed on a faulty resolver, but resolver and `ProviderRegistry` correctness
   are assumed, not verified, by this crate.
3. `architecture/codegg-compat.md:77` previously read as though the
   bridge called the shared `verification_digest` helper. Resolved
   2026-09-25: reworded as an explicit host obligation (the crate only
   compares host-supplied bindings at `lib.rs:775-784`). The split-ownership
   table that now carries that obligation is at
   `architecture/codegg-compat.md:71-85`.
4. `MappedPlan` docs (`lib.rs:494-496`) referenced `create_snapshot` as though
   it were adjacent API. Resolved 2026-09-25: the doc comment now names the
   test-only location (`tests/parity.rs`).
5. `hash_id` truncates SHA-256 hex to 32 chars (`lib.rs:550`); collisions are
   rejected per-run and duplicates per-manifest, but `MappingManifest::validate`
   does not re-derive IDs from sources — determinism rests on the parity test
   (`parity.rs:308-313`), not on the validator. Acceptable but worth stating.
6. `project_bounded` clamps and history-exclusion
   (`lib.rs:982-983,1038-1043`) are CodeGG-view choices, not core projection
   authority; the folded `assessment_reason_code` string (`lib.rs:1108`) is
   lossy by design with detail retained only in
   `CodeggAssessmentBridgeResult::reason_codes`. No issue found, but callers
   must not treat the projection label as the assessment.
7. Fail-closed is whole-assessment, not per-item. One unbound execution ref
   makes `normalize_snapshot` return `Err` and discards every other resolved
   observation for that snapshot (`lib.rs:767-796`). That is the intended
   stricter-than-legacy behavior, but it means a single bad ref suppresses
   otherwise-valid evidence, so callers must surface the error rather than
   retry with a filtered snapshot. CodeGG's engine selection honors this by
   treating a resolver error as a hard failure and never degrading to legacy
   satisfaction.
8. Resolver-supplied provider identity is accepted verbatim. The bridge checks
   that the observation's subject/kind match and that the verification binding
   is equal, but it does not check *which* provider the host used — that is
   `ProviderRegistry`'s job, and an untrusted or class-mismatched provider
   degrades assessment to non-completion rather than an error
   (`parity.rs:563` asserts the fold). Residual risk is bounded: misconfigured
   trust can only ever withhold completion, never manufacture it.
9. `src/` is panic-free by construction — no `unwrap`, `expect`, `panic!`,
   `unreachable!`, `todo!`, or `unimplemented!` anywhere in the 1118 lines, and
   no `#[cfg(test)]` module to separate from. The one indexing operation that
   *could* panic is `hex[..32]` in `hash_id` (`lib.rs:550`); it is unreachable
   in practice because `strip_prefix("sha256:")` already ran (`lib.rs:547-549`)
   and `digest_json` is core's own well-formed 64-hex output. A
   `get(..32).ok_or(...)?` would make the guarantee structural rather than
   argued. No other production panic path found.
10. The boundary script's source guards now cover filesystem I/O — **closed by
    corrective C002**. The alternation previously matched only
    process/network/database, so `std::fs`, `File::open`, and `read_to_string`
    in `src/` would have passed CI; `src/` contained none (verified by grep), so
    this was a guard gap, not a live defect. `no_impure_source`
    (`scripts/check-codegg-compat-boundary.sh:77-79`) now matches the
    fully-qualified and `use`-imported filesystem forms plus `std::path` and
    `tempfile::`, and a synthetic self-proof asserts each one fails. Closed by
    `plans/closure/codegg-integration/003-c002-closed.md`.
11. `assess_codegg_snapshot` mutates `plan.status` after `normalize_snapshot`
    already ran `plan.validate()` (`lib.rs:911-916` vs `lib.rs:869`), so the
    result is never re-validated by core before `assess_plan` consumes it
    (`lib.rs:917`). This is narrower than it first looks: core *does* have
    status-dependent rules, but they live on `PlanItem`
    (`eggplan-core/src/model.rs:246-250`, blocker required iff item is
    `Blocked`) and this mutation touches only `Plan.status`, whose five variants
    carry no dependent validation rule. So there is no present impact, and the
    item-level discipline the mapper established at `lib.rs:869` is intact — but
    the invariant "the bridge only ever hands core a validated plan" holds by
    accident rather than by construction. The compensating control is strong: a
    CodeGG `Completed` snapshot becomes `Active`, never `Closed`
    (`lib.rs:913`), so this crate cannot manufacture a closed plan.
12. Exploit attempt that did not succeed: a serialized `Satisfied` disposition
    is recorded in the manifest as provenance
    (`lib.rs:722-730`) and sets the `SerializedDispositionIsNotEvidence` loss
    (`lib.rs:723-725`), but `requirements` is built *only* from
    `item.evidence` refs the host resolved (`lib.rs:766-796`) — the
    `acceptance` loop (`lib.rs:800-818`) can add requirements to a criterion
    and can only ever attach ones already collected from the resolver, or
    none at all. There is no path from `Satisfied`, from a CodeGG
    `owner_run_id`/`owner_job_id` (`lib.rs:712-721` is provenance only), or
    from a bare `ref_id` string to an `EvidenceObservation`.

## Verification pointers

```sh
cargo test -p eggplan-codegg-compat --locked
cargo test -p eggplan-codegg-compat --locked --test parity
cargo test -p eggplan-codegg-compat --locked --test repository_projection
bash scripts/check-codegg-compat-boundary.sh
```

`tests/repository_projection.rs` (159 lines) is the M003-direction suite. Note
the section order: this pointer block sits before §6, and the M003 tests it now
names are described there.

## 6. M003 repository Plan projection

`project_repository_plan` (`src/lib.rs:120-217`) is the reverse direction from
M002: it validates a repository `Plan` (`lib.rs:123-124`) and returns
`RepositoryPlanProjectionV1` only for Active or Blocked lifecycle
(`lib.rs:125-127`, failing closed with `RepositoryProjectionError::NonBindableLifecycle`).
That rejects `Draft` — which is exactly what core's `Plan::new` produces
(`eggplan-core/src/model.rs:278-280`), so an M002 `normalize_snapshot` plan is
*not* bindable — as well as the terminal `Closed` and `Cancelled`
(`PlanStatus` has five variants: `eggplan-core/src/model.rs:9-15`). Projection
DTOs are strict `deny_unknown_fields` serde structures (`lib.rs:29-85`) and
include source Eggplan identities, exact lifecycle, complete criteria and
requirements, and no observations, provider descriptors, or closure records.
Precision on "no trust state": the projection *does* echo the
requirement-level `provider: Option<EvidenceProviderId>` and
`expected_verification_digest: Option<VerificationDigest>` copied from the plan
(`lib.rs:66-75`). Those are requirement declarations carried across, not enrolled
trust — no `ProviderDescriptor`, no trust class, no observation rows, no
closure/supersession record is reachable from this function.

The three digests cover different sets, and the distinctions matter for
reconciliation:

- `intent_digest` (`lib.rs:167-192`) digests objective plus, per item, ID,
  position, parent, dependencies, description, and per criterion ID, statement,
  `human_judgment_allowed`, and the full `EvidenceRequirement` set — so it *does*
  move when a requirement's kind, provider hint, policy, cardinality,
  `min_count`, human-judgment flag, or `expected_verification_digest` changes. It
  does **not** cover plan ID, revision, plan status, item status, `blocker`,
  `next_action`, `subject`, `provenance`, observations, trust, or closure. It is
  therefore stable across lifecycle and progress churn, which is what makes it
  usable as a "same intent" anchor.
- `content_digest` (`lib.rs:193-194`) is `digest_json(plan)` — the *whole* plan,
  including subject, provenance, revision, status, and the progress fields
  `intent_digest` omits. It is emitted in the projection but is **not** pinned
  by the binding manifest, so a consumer comparing manifests does not compare
  it directly; it reaches the manifest transitively through
  `projection_digest`.
- `projection_digest` (`lib.rs:195-205`) digests schema version, plan ID,
  revision, objective, plan status, `content_digest`, `intent_digest`, and the
  emitted `items` — i.e. every emitted field. It changes with any emitted
  lifecycle/progress change, which is the claim the earlier revision of this file
  made about it.

`repository_plan_binding_manifest` (`lib.rs:219-229`) pins exactly schema
version, plan ID, revision, `intent_digest`, and `projection_digest` for CodeGG
restart and drift reconciliation.

Each of the claims above is test-backed in
`tests/repository_projection.rs` (159 lines, 4 tests): the fail-closed set is
asserted literally as `[Draft, Closed, Cancelled]` → `NonBindableLifecycle`
(`tests/repository_projection.rs:135-141`); the digest split is asserted as
`intent_digest_ignores_progress_while_projection_digest_tracks_it`
(`tests/repository_projection.rs:95`); structured-requirement and
source-identity preservation is
`projection_preserves_structured_requirements_and_source_identity`
(`tests/repository_projection.rs:62`); and determinism plus strict versioning is
`projection_is_deterministic_and_strictly_versioned`
(`tests/repository_projection.rs:150`).

The compatibility crate has no repository client and does not translate
repository identity: the string `repository_id` appears nowhere in `src/`, so
there is no helper here that could relabel a `SubjectRevision`. CodeGG must
independently capture both subjects and verify Git subject equality before
persisting its explicit workspace/repository to Eggplan repository association.
Bound completion remains an application workflow through repository CAS,
evidence append, assessment, and guarded closure finalization. No ordinary CAS
may close the canonical Plan.
