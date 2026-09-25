# CodeGG Integration M002 — Staged Eggplan Assessment Adoption

Status: ready

Repository baseline: 0904218554c425c30aa6501b58d7fb8bcb839414

Source roadmap:

- plans/subsystems/codegg-integration-roadmap.md

Predecessor closure:

- plans/closure/codegg-integration/001-closed.md
- plans/closure/evidence-closure/002-c002-closed.md

Fresh CodeGG baselines reviewed:

- original M002 planning baseline: dbowm91/codegg @ a3c87fc18ee55aaf630401a562c11bb83112fd82
- current blocker/interface recheck: dbowm91/codegg @ f4e6e69d9e968e2adbb4228b3a7d45f55bd1294c
- upstream provenance-plan registration: dbowm91/codegg @ af0a3e0fb9b6552f45e3ea5d698e7980582493fd
- upstream provenance implementation and closure: dbowm91/codegg @ 418fdc85656e7e1faa57f71e5e7f10f7f4859c60; closure `plans/closure/eggplan-assessment-integration/001-status.md`
- hosted CodeGG CI: run `36106606574` passed on the exact implementation commit
- current Eggplan bridge/head recheck: `eggstack/eggplan` @ 85c4c7ef5dc826dd0be7cf65d71843c3264d842b; pure bridge `088968bd58680ae2b3741e2f1feb0614e0ff81a0`

M001's only hard dependency is satisfied. M002 is ready to resume from this
plan's staged-adoption checkpoint; verification-digest derivation, bridge
qualification, and differential adoption remain required within M002.

Prior compatibility baseline:

- 28b4695661d463dd1675d045ac6299c5fbc9ea31

Baseline comparison:

- current CodeGG is 239 commits ahead of the prior fixture baseline;
- WorkPlan model/assessment/storage implementation files are unchanged in the
  compare range;
- only `crates/codegg-core/tests/work_plan_foundation.rs` changed among
  WorkPlan paths;
- current ownership remains CodeGG SQLite/session/Goal/Todo/runtime.

Primary class: cross-repository compatibility / staged adoption

## 1. Objective

Make CodeGG consume Eggplan's generic plan/evidence assessment semantics where
that removes duplicated completion logic, without moving CodeGG runtime or
storage ownership into Eggplan.

M002 is deliberately staged:

1. refresh and harden the Eggplan-side CodeGG bridge against the current
   CodeGG contract;
2. make the bridge usable as a pure library without `eggplan-repo`;
3. add authoritative verification binding for CodeGG execution evidence;
4. adopt Eggplan assessment inside CodeGG behind CodeGG's existing public
   WorkPlan assessment/arbiter surface;
5. prove parity before deleting duplicate pure assessment logic.

CodeGG remains the owner of:

- WorkPlan/WorkItem SQLite rows and CAS;
- session/project/Goal binding;
- Todo projection;
- continuation checkpoints/context epochs;
- AgentRun/Job/worktree/scheduler execution;
- model tools;
- completion arbiter control flow.

Eggplan supplies generic structural/evidence assessment only.

## 2. Why this is staged

Current CodeGG WorkPlan evidence uses canonical host refs and host-owned status,
but execution-derived references do not carry Eggplan v2
`VerificationDigest` identity.

Eggplan correctly refuses to treat unbound Test/Command/DelegatedRun evidence
as satisfying a specific verification requirement.

M002 MUST NOT invent a digest from:

- prose;
- evidence ref ID alone;
- owner_job_id/owner_run_id alone;
- a CodeGG `Satisfied` disposition.

Before Eggplan assessment becomes authoritative for a CodeGG execution
criterion, the binding must be derived from the authoritative native execution
specification represented by the referenced CodeGG Job/Run/Test.

## 3. Eggplan-side bridge cleanup

Refactor `eggplan-codegg-compat` into a pure compatibility/assessment bridge.

### Remove repository coupling

Production dependencies should be:

- `eggplan-core`;
- serde/serde_json.

Remove the production `eggplan-repo` dependency.

The existing `create_snapshot<S: PlanStore>` helper is useful for fixture
qualification but should move to:

- test-only code; or
- a separate test support module/dev-dependency.

CodeGG must not pull Eggplan repository/Git persistence merely to reuse pure
assessment semantics.

### Separate live snapshot mapping from fixture provenance

Current `normalize` rejects any fixture whose `source_sha` is not the
single pinned `SOURCE_CODEGG_SHA`.

Split:

- `normalize_snapshot(snapshot, subject, resolver)` — live strict mapping;
- `normalize_fixture(fixture, ...)` — fixture wrapper that validates recorded
  source repository/SHA/provenance then calls the live mapper.

Source SHA is fixture provenance, not runtime semantic authority.

### Refresh fixture baseline

Set the current reviewed fixture baseline to:

- `a3c87fc18ee55aaf630401a562c11bb83112fd82`.

Regenerate/review the M001 golden cases against current CodeGG source.

Preserve old fixture compatibility only when it is useful and explicit; do not
silently relabel old bytes as the new SHA.

## 4. Verification binding bridge

Add a host-facing bridge contract that can carry the exact verification
identity for CodeGG execution evidence.

Recommended shape:

    ResolvedCodeggEvidence {
        observation: EvidenceObservation,
        expected_verification: Option<VerificationDigest>,
        native_ref: ...
    }

For execution-derived kinds:

- TestJob -> Test;
- SchedulerJob -> Command;
- DelegatedRun / AgentRun -> DelegatedRun;

both requirement and observation must receive the same digest derived from the
authoritative execution specification.

Artifact/Commit remain Artifact/Revision and do not require execution binding.

### CodeGG authoritative digest source

In the coordinated CodeGG change, derive verification identity from the
canonical host-owned execution description already used to run the work.

Examples:

- TestJob: canonical test runner argv/config/working-directory/relevant
  execution policy;
- SchedulerJob: canonical job payload/argv and execution semantics;
- DelegatedRun/AgentRun: canonical delegated task/run verification
  specification, not generated prose.

Use Eggplan's canonical verification-digest helper/format.

Do not include secrets, credentials, volatile timestamps, transport lease IDs,
or mutable progress text.

If a referenced native object cannot reconstruct the verification
specification, mark that evidence unavailable/unbound and preserve current
CodeGG behavior through the compatibility fallback until the contract is
fixed.

## 5. Live assessment bridge

Add a pure function roughly equivalent to:

    assess_codegg_snapshot(
        snapshot,
        subject,
        resolved_evidence,
        provider_registry,
    ) -> CodeggAssessmentBridgeResult

Result should include:

- mapped Eggplan Plan;
- MappingManifest/losses;
- Eggplan PlanAssessment;
- CodeGG completion family;
- stable reason codes;
- source-id mapping.

Do not persist an Eggplan Plan just to assess CodeGG runtime state.

The bridge is a deterministic view over current CodeGG state.

## 6. Completion-family compatibility

Preserve CodeGG's public completion families:

- Complete;
- ActionableWorkRemaining;
- Blocked;
- AwaitingUserJudgment;
- InFlight.

Eggplan's more detailed statuses/reasons may fold into these exactly as the M001
compat layer already specifies.

The adapter must retain detailed Eggplan reason codes for diagnostics/tests so
loss is visible.

Do not change CodeGG arbiter control-flow semantics merely because Eggplan has
additional assessment statuses.

## 7. Coordinated CodeGG adoption

The implementation handoff is cross-repository. Eggplan owns the bridge;
CodeGG consumes it.

At implementation time:

1. recheck both repo heads;
2. land/identify the Eggplan bridge commit;
3. add CodeGG Git dependencies pinned to an immutable Eggplan revision;
4. prefer dependency on `eggplan-core` plus the pure
   `eggplan-codegg-compat` package only;
5. do not depend on `eggplan-repo`, `eggplan-cli`, or
   `eggplan-projection` from CodeGG core.

CodeGG already uses immutable Git dependencies for Eggstack components; follow
that convention.

## 8. CodeGG assessment migration

Replace or delegate the duplicative pure evidence-satisfaction portion of
`codegg-core::work_plan::assessment` only after golden parity is demonstrated.

Keep CodeGG's public types/functions stable during M002 where practical:

- `assess_work_plan` may become a facade around the Eggplan bridge;
- `WorkPlanCompletionAssessment` remains the CodeGG-facing DTO;
- Todo/arbiter callers need not know Eggplan types;
- WorkPlanStore remains unchanged.

Do not delete the old assessor until differential tests prove parity for the
supported semantic subset.

A temporary test-only legacy assessor is acceptable for differential
qualification; it must not become a second production authority after closure.

## 9. Semantics that remain CodeGG-specific

The bridge must not absorb or reinterpret:

- one-active-plan-per-session;
- Goal binding/scope;
- objective provenance;
- current phase/current item preference;
- owner_run_id/owner_job_id write authority;
- Todo feedback revision rules;
- context epoch/checkpoint provenance;
- terminal-answer retry/continuation logic;
- GoalVerificationService;
- scheduler/job admission.

These may influence how CodeGG builds the snapshot/resolved evidence, but they
remain outside Eggplan core.

## 10. Status/lifecycle mapping

Preserve M001 safety:

- CodeGG Completed source label does not create an Eggplan ClosureRecord;
- CodeGG Cancelled maps to cancelled intent only;
- serialized `AcceptanceDisposition::Satisfied` is not evidence;
- owner refs remain provenance;
- source revision remains provenance/CAS context, not Eggplan revision
  authority.

Live assessment may conclude Complete, but CodeGG retains its own plan status
transition and Goal/turn arbitration.

## 11. Differential parity matrix

Run both legacy and Eggplan-backed assessment over representative states:

- no evidence;
- passing TestJob;
- failed TestJob;
- missing referenced job;
- in-flight job/run;
- blocked item;
- user-judgment criterion;
- forged `Satisfied` disposition without host evidence;
- stale/mismatched subject;
- verification digest mismatch;
- completed item without host proof;
- mixed multi-item dependency plan;
- delegated child run;
- artifact/commit evidence.

Expected result:

- same CodeGG completion family for cases whose semantics overlap;
- intentional deltas documented where Eggplan is stricter;
- no case becomes more permissive than the legacy host-owned rule.

Any permissive delta is a stop condition.

## 12. Subject mapping

CodeGG must supply an explicit SubjectRevision representing the repository/worktree
state against which evidence applies.

Do not fabricate a clean subject from HEAD when CodeGG worktree state is dirty.

If CodeGG cannot provide an exact subject for a runtime path, Eggplan-backed
assessment for that path must remain unavailable/invalid rather than silently
falling back to subjectless proof.

M002 does not move CodeGG to Eggplan repository subject capture; use a narrow
CodeGG host adapter that derives equivalent exact repository identity.

## 13. Tests in Eggplan

- current CodeGG fixture parse/mapping;
- old fixture provenance rejection/explicit legacy handling;
- live snapshot mapper does not require fixture SHA;
- bridge crate has no `eggplan-repo` dependency;
- verification-bound Test/Command/DelegatedRun satisfaction;
- missing/mismatched binding fails closed;
- serialized Satisfied remains non-authoritative;
- owner refs remain provenance;
- completion-family projection preserves detailed reason codes;
- mapping deterministic at current CodeGG baseline.

## 14. Tests in CodeGG

At minimum:

- existing WorkPlan foundation/projection/trajectory suites;
- differential assessment matrix;
- current Goal-bound trajectory;
- Todo feedback cannot manufacture evidence;
- host evidence snapshot resolution with verification digest;
- cancellation/completion CAS race unchanged;
- long-horizon trajectory qualification unchanged;
- checkpoint/context-epoch behavior unchanged;
- legacy DB migration unchanged;
- no Eggplan repository files/state created by CodeGG.

## 15. Dependency and release considerations

Because CodeGG uses a Git dependency during staged adoption:

- pin exact immutable Eggplan rev;
- record the rev in CodeGG planning/closure evidence;
- do not use branch dependencies;
- keep the dependency surface pure and small;
- do not introduce libgit2/filesystem persistence transitively through the
  bridge.

Before a future crates.io release requiring fully registry-resolvable
dependencies, decide whether Eggplan crates are published or vendored through a
release process. That packaging decision is not part of M002.

## 16. Documentation

Eggplan:

- update `architecture/codegg-compat.md`;
- document live bridge vs fixture provenance;
- document verification-binding ownership.

CodeGG coordinated change:

- update `architecture/work_plan.md`;
- document Eggplan as pure assessment substrate only;
- explicitly retain WorkPlanStore/arbiter/Goal/Todo/checkpoint ownership.

## 17. Verification

Eggplan:

    cargo fmt --all -- --check
    cargo check --workspace --all-targets --locked
    cargo clippy --workspace --all-targets --locked -- -D warnings
    cargo test --workspace --locked
    cargo +1.89.0 check --workspace --all-targets --locked
    cargo +1.89.0 test --workspace --locked
    bash scripts/check-codegg-compat-boundary.sh
    bash scripts/check-core-boundary.sh
    git diff --check

CodeGG coordinated verification should include at least:

    cargo test -p codegg-core --lib -- work_plan
    cargo test -p codegg-core --test work_plan_foundation
    cargo test -p codegg-core --test work_plan_projection_arbiter
    cargo test --test work_plan_projection_arbiter
    cargo test --test long_horizon_trajectory_qualification
    cargo test -p codegg-core -- migration
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings
    ./scripts/verify.sh quick
    git diff --check

Record actual equivalents if names change.

## 18. Acceptance criteria

M002 closes when:

1. current CodeGG snapshots map through a live bridge independent of fixture
   SHA gating;
2. the bridge production dependency graph excludes eggplan-repo;
3. CodeGG execution evidence carries authoritative verification binding;
4. Eggplan-backed assessment is adopted behind CodeGG's existing public
   assessment surface;
5. differential tests show no permissive completion regression;
6. WorkPlanStore/Goal/Todo/checkpoint/scheduler ownership remains CodeGG;
7. no Eggplan repository state is created by CodeGG;
8. current long-horizon trajectory tests remain green;
9. both repositories record exact implementation revisions and native/MSRV CI
   evidence where applicable.

## 19. Stop conditions

Stop and report if:

- CodeGG cannot derive verification identity from authoritative execution
  specifications;
- adopting Eggplan would require treating ref IDs/prose as verification
  identity;
- CodeGG would need to move SQLite WorkPlan storage into Eggplan;
- Goal/Todo/checkpoint/scheduler behavior must be changed to make parity pass;
- any differential case becomes more permissive;
- the bridge requires eggplan-repo in CodeGG production.

## 20. Closure evidence

The Eggplan closure record must cite:

- Eggplan bridge implementation SHA;
- CodeGG adoption SHA;
- reviewed CodeGG baseline;
- pinned Eggplan dependency rev used by CodeGG;
- before/after dependency graph;
- verification-binding derivation matrix;
- differential parity matrix;
- WorkPlan ownership invariance evidence;
- focused/full test results from both repos;
- hosted workflow IDs;
- residual findings and M003 disposition.

## 21. Implementation checkpoint — blocked on exact execution subject

Eggplan-side bridge work is committed as `088968b` (pure live snapshot mapper,
fixture-only source-SHA validation, detailed deterministic assessment result,
and shared verification-digest helper). CodeGG was rechecked through
`f4e6e69d9e968e2adbb4228b3a7d45f55bd1294c`; the relevant WorkPlan evidence
shape still lacks exact attempt-scoped source provenance.

CodeGG currently stores only status in `WorkPlanEvidenceSnapshot`. A current
worktree capture cannot establish the subject of an older completed job.
Attaching that current subject to an observation would manufacture authority,
and leaving the observation subjectless cannot satisfy Eggplan's exact-subject
policy. The live bridge therefore correctly fails closed.

The required upstream handoff is now registered in CodeGG at
`af0a3e0fb9b6552f45e3ea5d698e7980582493fd`:

- `plans/subsystems/eggplan-assessment-integration-roadmap.md`
- `plans/implementation/eggplan-assessment-integration/001-durable-execution-subject-provenance.md`

Its M001 captures and persists CodeGG-native attempt-scoped execution subjects,
handles live-workspace drift and immutable/materialized seal points, keeps
legacy records explicitly subject-unavailable, and exposes enriched host
evidence without replacing CodeGG's current assessor.

Resume this Eggplan M002 only after that CodeGG M001 closes positively. Then
complete the remaining verification-digest derivation, CodeGG dependency pin,
differential parity, and production assessor adoption. Evidence C003,
Projection/CLI M002, and Eggstack M002 remain independent and may proceed in
parallel.
