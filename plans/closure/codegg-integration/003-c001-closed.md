# CodeGG Integration M003 C001 Closure Record — Dirty-Subject Fingerprint Contract and Bound-Evidence Requalification

Status: closed

Both cross-repository conditions recorded as conditional have been discharged by
CodeGG PR `dbowm91/codegg#90` (merged; CodeGG `main` `3623f65e` + `b470865a`,
canonical hosted run `37084905013`) and by this repository's requalification of
the CodeGG evidence. The original conditional finding is preserved verbatim in
section 1a as the record of what was outstanding when the first closure was
written.

Eggplan production implementation is complete, verified locally and on hosted
Ubuntu/macOS/Windows plus Rust 1.89, and the frozen subject digest bytes are
unchanged. This is a post-closure correctness corrective: the M003 closure
(`plans/closure/codegg-integration/003-conditionally-closed.md`) and its
preserved conditional-closure narrative are unchanged historical evidence.

The cross-repository contract is now complete and qualified on both sides.

Source implementation plan:
`plans/implementation/codegg-integration/003-c001-dirty-subject-fingerprint-and-bound-evidence-requalification.md`

Source roadmap: `plans/subsystems/codegg-integration-roadmap.md`

Predecessor plan/closure (preserved, not rewritten):
`plans/implementation/codegg-integration/003-repository-plan-binding-contract.md`
and `plans/closure/codegg-integration/003-conditionally-closed.md`

Reviewed baseline: `52a4be76b631822799e7f1e76fe5b439c94fd058` (pre-refactor
`GitSubjectSource` implementation used to freeze the golden digests)

Implementation commits:

- `352a0f782b0166aad8e850185019d78ec162d487` — the fingerprint contract,
  shared capture refactor, golden matrix, and tests.
- `faa6c873583c24974868dac8acb70e55fea05181` — portability repair for hosted
  Windows compilation.
- `0dd33b761e85f1364320a9208aaebd5be281c6a5` — LF-normalized golden sidecar
  assertion. This is the exact hosted-qualified revision CodeGG must pin.

Hosted qualification: Eggplan CI run
[`37063328954`](https://github.com/eggstack/eggplan/actions/runs/37063328954) on
head `0dd33b7`: `native (ubuntu-latest)`, `native (macos-latest)`,
`native (windows-latest)`, and `msrv` (Rust 1.89.0) all succeeded. Two earlier
runs on the same work failed on Windows and are retained as non-passing
evidence; see section 8.

Coordinated CodeGG plan:
`dbowm91/codegg:plans/implementation/eggplan-assessment-integration/005-m003-c001-dirty-subject-provenance-and-bound-evidence.md`
registered at CodeGG `dc9ae6ccf1cfa8dea51522ddc6bf33976562edb9`.

## 1. Executive finding

C001 is **closed on both sides**. Eggplan exposes the exact dirty-subject
primitive CodeGG was missing and proves it cannot have changed Eggplan's
existing subject identity; CodeGG pinned this revision, routed both of its
Eggplan capture sites through the new contract, and requalified the dirty bound
evidence/closure path. All eleven acceptance criteria of plan §15 are met
with recorded evidence.

The CodeGG side, in summary:

- CodeGG `main` `3623f65e` moves `eggplan-core`, `eggplan-codegg-compat`, and
  `eggplan-repo` from `3f7c603` to `0dd33b76`, updates the `EGGPLAN_PIN`
  constant (`b470865a`), and replaces `store.subject_source().capture()` with
  `capture_git_subject_fingerprint` in the attempt-start/seal helper and in both
  halves of the binding identity sandwich. The read-only store validation is
  retained, and its ownership guard now confines raw fingerprint calls to the
  same two owner modules.
- The binding identity tuple still comes from the store, because a fingerprint
  carries no repository identity, and the assessment/guarded-closure paths still
  read the repository's own current subject.
- The CodeGG dirty matrix was rerun unchanged on the new pin: stable dirty
  execution → observation → item completion → guarded closure, drift refusal,
  and legacy-dirty fail-closed all pass, and the 34 pre-existing clean binding
  cases are untouched.
- CodeGG closure `dbowm91/codegg:plans/closure/eggplan-assessment-integration/005-m003-c001-status.md`
  is now `closed` with its original conditional narrative preserved.

## 1a. Original conditional finding (preserved verbatim as history)

Eggplan now exposes the exact dirty-subject primitive CodeGG was missing, and
proves it cannot have changed Eggplan's existing subject identity. Acceptance
criteria 1-6 and 10-11 are met with evidence. Criteria 7-9 — the CodeGG side
pinning this revision, persisting through it, and requalifying the dirty bound
evidence/closure path on hosted CI — are not yet met: CodeGG implemented its
half (`36ec9322`) but still pins the pre-C001 revision `3f7c603` and its own
closure is conditional.

The substitution CodeGG still owes is bounded: bump three `rev =` pins from
`3f7c603` to `0dd33b761e85f1364320a9208aaebd5be281c6a5`, call
`capture_git_subject_fingerprint(root, GitSubjectOptions::default(), Some(&state_root))`
in its capture helper in place of
`store.subject_source().capture()`, and rerun its dirty matrix plus hosted CI.
By C001 §4/§5 the digest bytes are identical between those two calls, so the
behavioral risk of the bump is low, but the evidence does not exist yet and is
therefore not claimed.

## 2. Public API shape as landed

From `crates/eggplan-repo/src/git_subject.rs`, re-exported by
`crates/eggplan-repo/src/lib.rs`:

    pub struct GitSubjectFingerprintV1 {
        pub schema_version: u16,          // SCHEMA_VERSION == 1
        pub revision: String,
        pub state: SubjectState,
        pub dirty_digest: Option<String>, // sha256:<64 lowercase hex> when Dirty
    }

    pub fn capture_git_subject_fingerprint(
        root: impl AsRef<Path>,
        options: GitSubjectOptions,
        excluded_path: Option<&Path>,
    ) -> Result<GitSubjectFingerprintV1, GitSubjectError>

`GitSubjectFingerprintV1` derives `Serialize`/`Deserialize` with
`deny_unknown_fields`; `dirty_digest` is `#[serde(default,
skip_serializing_if = "Option::is_none")]`. `validate()` rejects a wrong schema
version, a blank revision, clean-with-digest, dirty-without-digest, and any
digest outside `sha256:<64 lowercase hex>`. Serialized form is exactly
`{"schema_version":1,"revision":"<oid>","state":"clean"}` when clean and adds
`"dirty_digest"` when dirty.

`excluded_path` is `Option<&Path>` rather than the plan's suggested
`Option<impl AsRef<Path>>` because the latter cannot infer its type argument
from a `None` call, which is a required and common call shape.

The value carries no repository ID, path list, file content, index entry,
symlink target, raw manifest byte, or provider/evidence/closure state. Eggplan
provides no repository-ID relabeling constructor for it: host identity proof
stays with the host, consistent with registry gate 21.

## 3. Before/after digest golden matrix

Captured from the pre-refactor implementation at `52a4be76` by running the
committed `record_golden_dirty_manifest_digests` test on a Git worktree of that
revision, then frozen into
`crates/eggplan-repo/tests/fixtures/git-subject-digests-v1.json` with a
`sha256:215dbf724b725a209df43b12f734e82efc17ec3fe47403a935bec1443238cd92`
sidecar. `golden_dirty_manifest_digests_are_frozen` re-derives the matrix on
the post-refactor tree and asserts both the sidecar and every case value.

| Case | State | Dirty digest before (52a4be76) | Dirty digest after (0dd33b7) |
|---|---|---|---|
| clean | clean | none | none |
| unstaged_tracked_edit | dirty | `sha256:75ceabb95bc153177c3d553d70ec45e510e0f8565678cee8178f3f118669fe91` | identical |
| staged_tracked_edit | dirty | `sha256:f810470f083d27e4df127667ad166384075e1040bb4a859e00e591ea2260831b` | identical |
| staged_and_unstaged_same_path | dirty | `sha256:06f87a2312fd63a0a46c1ffd30855308c3efc26c9fb6a6269dcdc9f55f3bdad4` | identical |
| untracked_file | dirty | `sha256:073df2ab0ef7799b5ac7f1de0d211791bb16b2e7974f7efc757a43955526163b` | identical |
| deleted_tracked_file | dirty | `sha256:5a8498102a06ae2172241780cb7e7f01f861dea98ea3f6e5d0fadc43e7b30300` | identical |
| symlink_typechange | dirty | `sha256:ede7c75daa2c0a8e5fc08b7571b6f6613322c2dc03ef87798aa3f731ac424945` | identical |
| staged_rename | dirty | `sha256:e2e06fb08fd6512e6840834c047a914d717f4d076105d1235a7d10039508100c` | identical |
| nested_dirty_submodule | dirty | `sha256:dc3078228eb110bd4c309c83c33ec6019774026fd3c3bb035e3b5b0489f06568` | identical |

Notes on the matrix:

- Fixtures use fixed file contents and fixed commit identities, so status bits,
  index blob ids, and content bytes are reproducible. The digests were also
  reproduced across two independent runs on the pre-refactor tree before
  freezing.
- `staged_rename` is recorded as libgit2 reports it. Rename/copy detection is
  not enabled for this subject algorithm, so the case is a staged add plus a
  removal; enabling `renames_head_to_index` would change digest bytes and is
  explicitly out of scope.
- `nested_dirty_submodule` is assembled with the `git` CLI, whose commit
  timestamp is not controlled, so no HEAD OID is frozen for it; the nested
  manifest bytes it contributes are frozen.
- `symlink_typechange` requires Unix symlink creation. It is asserted on
  Linux/macOS and is checked to remain present in the fixture (never silently
  dropped) on Windows.

## 4. Exclusion and bounds matrix

| Situation | `GitSubjectSource` (historical) | `capture_git_subject_fingerprint` (C001) |
|---|---|---|
| No exclusion | full manifest | full manifest |
| `.eggplan` inside worktree | excluded, and descendants too | identical bytes |
| Sibling shares string prefix (`.eggplan-admin`) | still in scope | identical bytes |
| Exclusion outside worktree | no exclusion applied (lenient) | `InvalidExclusion` (fail closed) |
| Exclusion is the worktree root | swallows whole manifest (lenient, historical) | `InvalidExclusion` (fail closed) |
| Exclusion path does not exist | `Io` from canonicalize | identical `Io` |
| Exclusion with no worktree (bare) | no exclusion applied | `InvalidExclusion`, or the same Git failure as the subject source when HEAD is unreadable |
| `max_paths` exceeded | `BoundExceeded` | identical |
| `max_content_bytes` exceeded | `BoundExceeded` | identical |
| `max_submodule_depth` exceeded | `BoundExceeded` | identical |
| Not a Git worktree | `NotGit` | identical |
| Unborn HEAD | `Git(UnbornBranch)` (identical to the source) | identical |
| Unsafe/symlinked status path | `UnsafePath` | identical |
| Non-Unicode path | `NonUnicodePath` | identical |

No partial digest is returned on any failure path: the fingerprint is only
constructed after `capture_subject` succeeds and then re-validated.

`ExclusionMode` is the only behavioral fork between the two entry points. It
never touches manifest bytes, row ordering, status-bit encoding, index-entry
treatment, symlink/deleted/submodule encodings, or the default bounds
(10,000 paths / 64 MiB content / 8 submodule levels, asserted by
`fingerprint_and_source_agree_on_the_default_options_path_only`).

## 5. Requirement-to-evidence matrix

| Plan § | Requirement | Evidence and disposition |
|---|---|---|
| 2 | Compatibility fingerprint, no authority change | `GitSubjectFingerprintV1` is a read-only capture result. No persisted schema, no new crate, no scheduler/executor/net code. Only `eggplan-repo` changed. |
| 3 | Bounded, versioned, repository-ID-free API with the four permitted fields | `git_subject.rs:66-137`; `compile_fail` doctests in `lib.rs:44-83` prove `repository_id`, `paths`, and `dirty_manifest` do not exist; strict serde test rejects a `paths` field at decode time. |
| 4 | One implementation source for both APIs | `capture_subject` (`git_subject.rs:211-237`) returns `CapturedSubject`; `GitSubjectSource::capture` and `capture_git_subject_fingerprint` are thin projections. `fingerprint_matches_subject_across_representative_dirty_states` and its siblings assert the equality invariant. |
| 5 | Historical digest bytes and semantics frozen | Section 3 golden matrix, captured pre-refactor at `52a4be76`. No domain separator, ordering, status-bit, index-entry, encoding, exclusion, or bound change for the existing API. |
| 6 | Single administrative-root exclusion with fail-closed invalid/outside handling | Section 4 matrix; `strict_exclusion_fails_closed_and_lenient_exclusion_is_unchanged`; `fingerprint_excludes_the_single_administrative_root`. No glob or multi-exclusion policy was introduced. |
| 7 | Same bounds and typed error classes, no partial digest | Section 4; `fingerprint_fails_closed_on_configured_bounds`; `fingerprint_reports_the_same_typed_failures_as_the_subject_source`. |
| 8 | Cross-implementation role | Documented in `architecture/repository.md` ("not a universal Git digest standard") and in the public rustdoc. Eggplan offers no translation helper; the host decides. |
| 9 | Binding-time sandwich primitive | `fingerprint_detects_dirty_content_change_between_two_captures` proves E1 != E2 for same-HEAD, same-dirty-class, changed contents. Eggplan does not implement CodeGG's sandwich. |
| 10 | Execution-time persistence contract | Host responsibility. Verified for the consumption shape only, out of tree: see section 6. |
| 11 | Focused test matrix + golden fixture set | `crates/eggplan-repo/tests/git_subject_fingerprint.rs` (13 tests, one Unix-gated) plus `git_subject_digest_golden.rs`. Coverage: clean, unstaged, staged, staged+unstaged, untracked, deleted, symlink, staged rename, dirty nested submodule, excluded `.eggplan` subtree, path/content/depth bounds, invalid/outside exclusion. |
| 12 | CodeGG consuming qualification | **Met.** CodeGG implemented its half at `36ec9322`, then pinned `0dd33b7` and consumed the fingerprint API in `3623f65e` + `b470865a`, requalified by CodeGG hosted run `37084905013` (job `111109235288`) on `main` head `b470865a`. See sections 7 and 7a. |
| 13 | Documentation reconciliation | Historical M003 plan header already reads "closed — CodeGG consumer landed" (reconciled in `6644725`); registry external CodeGG baseline advanced to the landed M003 implementation/closure plus the C001 commits; M003 closure narrative untouched; this record files C001 as a post-closure corrective. |
| 14 | Verification | Section 8: local + MSRV pass; hosted qualification run `37063328954` (all four jobs) on head `0dd33b7`; earlier Windows failures `37062437251` and `37062837529` retained as non-passing evidence. |
| 15 | Acceptance criteria | All 11 met: 1-6 and 10-11 by this repository, 7-9 by the CodeGG work in section 7a. |
| 16 | Stop conditions | None triggered. No historical digest byte changed, no manifest is exposed, no `SubjectRevision` equality was weakened, and no backfill or native-digest substitution exists in Eggplan. |
| 17 | Closure evidence | This record. |

## 6. Production evidence

- `crates/eggplan-repo/src/git_subject.rs`: `GitSubjectFingerprintV1`,
  `capture_git_subject_fingerprint`, `CapturedSubject`, `ExclusionMode`,
  `capture_subject`, `resolve_exclusion`, `valid_digest`, two new typed errors
  (`InvalidExclusion`, `InvalidFingerprint`), and two new in-crate tests.
- `crates/eggplan-repo/src/lib.rs`: re-exports plus three `compile_fail`
  doctests asserting the fingerprint exposes no repository ID, path list, or
  manifest.
- `crates/eggplan-repo/tests/git_subject_fingerprint.rs`: 13 integration tests.
- `crates/eggplan-repo/tests/git_subject_digest_golden.rs` +
  `crates/eggplan-repo/tests/fixtures/git-subject-digests-v1.json` and
  `.sha256`: the frozen digest matrix and its recorder.
- `architecture/repository.md`, `architecture/deep-dive-repository.md`
  (new section 3a), `architecture/overview.md`, `AGENTS.md`: contract
  documentation and the freeze warning.
- Out-of-tree consumer check (not committed; local evidence only): a separate
  crate depending on `eggplan-core`/`eggplan-repo` by path performed the CodeGG
  C001 consumption pattern — open a store, create a Plan, capture the
  fingerprint with `.eggplan` excluded before and after an administrative
  write, and compare it with `store.subject_source().capture()`. Observed: clean
  fingerprint with no digest; dirty fingerprint digest equal to the repository
  subject digest; fingerprint unchanged by a subsequent evidence append; E1 vs
  E2 differing when only dirty contents changed; `Err` for an outside
  exclusion. This validates public ergonomics and the exact-subject match; it is
  not a substitute for CodeGG's own requalification.

## 7. CodeGG consuming side (as first reviewed)

Exact consuming state reviewed 2026-10-02 at `dbowm91/codegg` `origin/main`
`13d64ffe`:

- Implementation: `36ec9322` — nested `ExecutionSubjectRevision` v2 with
  `eggplan_dirty_digest` (`SCHEMA_VERSION = 2`, `SCHEMA_VERSION_V1 = 1`),
  single application-layer capture helper in `src/execution_subject_capture.rs`,
  E1/C/E2 sandwich in `prove_identity` with
  `repository_subject_changed_during_identity_proof`, bound dirty translation
  from the persisted Eggplan digest only, and legacy dirty fail-closed with
  `legacy_dirty_subject_missing_eggplan_digest`. No SQLite migration
  (`STORAGE_LAYOUT_VERSION` stays 68), no `codegg-core` Eggplan dependency.
- CodeGG closure: `b5b3b14e`,
  `dbowm91/codegg:plans/closure/eggplan-assessment-integration/005-m003-c001-status.md`,
  status "conditionally closed".
- CodeGG currently pins Eggplan `3f7c603`, not `0dd33b7`, and captures through
  `store.subject_source().capture()`.

CodeGG dirty matrix reported by that record (their evidence, not re-run here):

| CodeGG case | Reported result |
|---|---|
| `dirty_stable_execution_binds_evidence_and_guarded_closes` | pass; observation `dirty_digest` equals live `subject_source().capture()`; item completion and guarded closure under a stable dirty state |
| `dirty_content_change_during_attempt_never_becomes_passing_evidence` | pass; `evidence_subject_unstable`, zero observations |
| `legacy_v1_dirty_provenance_fails_closed_without_backfill` | pass; `legacy_dirty_subject_missing_eggplan_digest`, zero observations |
| `sandwich_stable_dirty_proves_both_digests` | pass |
| `sandwich_dirty_change_during_proof_fails_closed` | pass |
| `bound_dirty_translation_uses_only_the_eggplan_digest` | pass |
| `bound_dirty_v1_fails_closed_without_backfill` | pass |
| `revision_v1_parse_validate_roundtrip`, `revision_v2_clean_and_dirty_shapes`, `revision_v2_rejects_malformed_eggplan_digest`, `revision_v2_start_seal_equality_and_drift` | pass |
| `cargo test -p codegg --test work_plan_repository_binding` | 37 pass (34 pre-existing + 3 new) |
| `cargo test -p codegg --test work_plan_eggplan_differential` | 28 pass |
| Hosted canonical CodeGG CI on the C001 head | CodeGG's record states pending; this repository observes CodeGG run `37051423825` (success) whose head `41513fd3` contains `36ec9322` and `b5b3b14e`. CodeGG's own record does not yet cite a run ID, so this is recorded as an observation, not as CodeGG's accepted evidence. |

Missing on the CodeGG side at the time of this record's first write, and
therefore the then-open conditions (both now discharged — see section 7a):

1. Bump the `eggplan-core` / `eggplan-codegg-compat` / `eggplan-repo` pins from
   `3f7c603` to `0dd33b761e85f1364320a9208aaebd5be281c6a5` and route the capture
   helper through `capture_git_subject_fingerprint`.
2. Rerun the CodeGG dirty matrix and hosted canonical CI on that pin, then
   promote `plans/closure/eggplan-assessment-integration/005-m003-c001-status.md`
   to closed.

## 7a. CodeGG condition discharge (as later reviewed)

Reviewed 2026-10-03 on CodeGG `main` after PR `dbowm91/codegg#90` merged:

| Item | Evidence |
|---|---|
| Eggplan pins moved | CodeGG `main` `3623f65e`: `eggplan-core`, `eggplan-codegg-compat`, `eggplan-repo` all at `0dd33b761e85f1364320a9208aaebd5be281c6a5`; `EGGPLAN_PIN` constant updated in `b470865a` |
| Fingerprint consumed at both sites | `src/execution_subject_capture.rs` (attempt start/seal) and `src/work_plan_repository_binding.rs` (E1/E2 sandwich) call `eggplan_repo::capture_git_subject_fingerprint`; `open_read_only` validation retained so a workspace without a real Eggplan store still yields no Eggplan-compatible digest |
| Ownership preserved | The identity tuple still comes from the store (a fingerprint carries no repository identity); assessment and guarded-closure paths still read the repository's own current subject; `codegg-core` still has no Eggplan dependency; storage layout still v68 with no migration |
| Guards strengthened | `check_execution_subject_ownership.py` rule 1d confines raw `capture_git_subject_fingerprint` calls to the two owner modules; rule 6 requires the sandwich to capture the Eggplan side twice through that API |
| Dirty matrix requalified on the new pin | `cargo test -p codegg --test work_plan_repository_binding` 37 pass (34 pre-existing clean + 3 C001 dirty/drift/legacy); `cargo test --lib -- work_plan_repository_binding` 10 pass; `cargo test --lib -- work_plan_eggplan` 14 pass including `eggplan_pin_matches_manifest_and_lock`; `work_plan_eggplan_differential` 28 pass |
| CodeGG local verification | fmt clean, `cargo clippy --workspace --all-targets --locked -- -D warnings` clean, `verify.sh quick` passed, all three Python ownership guards plus the core boundary script pass |
| CodeGG hosted qualification | `CI` run `37084905013`, `verify` job `111109235288`, green on `main` head `b470865a`; PR run `37072794090` attempt 3, job `111070238089`, green on the byte-identical tree at `11354fc9` |
| CodeGG failure evidence retained | `37069982736` failed the `EGGPLAN_PIN` manifest/lock assertion — a real defect of the bump, fixed in `b470865a`; attempts 1 and 2 of `37072794090` failed the unrelated tool-advisor `causal_active_m005::m005_holdout_structural_gates` single-sample 5 ms latency gate on two different holdouts, the same load-sensitive flake class already classified for CodeGG run `37047118019` |
| CodeGG closure record | `dbowm91/codegg:plans/closure/eggplan-assessment-integration/005-m003-c001-status.md` is `closed`, with the original conditional finding and both original findings preserved verbatim as history |

No Eggplan-side code changed to accept this evidence, and no Eggplan digest
byte, schema, or invariant was affected by the CodeGG pin bump: Eggplan's own
golden matrix is the proof that the bumped fingerprint contract returns the same
digest bytes the previous store-subject call returned for the same root,
options, and exclusion.

## 8. Verification

Executed from Eggplan on the implementation head
`0dd33b761e85f1364320a9208aaebd5be281c6a5`:

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo check --workspace --all-targets --locked` | pass |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | pass |
| `cargo test --workspace --locked` | pass; 154 tests (baseline 134), 1 ignored recorder |
| `cargo test --workspace --locked --doc` | pass; 6 doctests, including 3 new `compile_fail` fingerprint proofs |
| `cargo +1.89.0 check --workspace --all-targets --locked` | pass |
| `cargo +1.89.0 test --workspace --locked` | pass; same 154 tests plus doctests |
| `bash scripts/check-core-boundary.sh` | pass |
| `bash scripts/check-codegg-compat-boundary.sh` | pass |
| `bash scripts/check-integrations-boundary.sh` | pass |
| `bash scripts/check-projection-cli-boundary.sh` | pass |
| `bash scripts/check-closure-authority-boundary.sh` | pass |
| `git diff --check` | pass |
| Pre-refactor golden recording on a `52a4be76` worktree | pass; matrix captured and reproduced across two runs |

Hosted runs on the C001 work (all on `eggstack/eggplan` CI):

| Run | Head | Result | Detail |
|---|---|---|---|
| [`37062437251`](https://github.com/eggstack/eggplan/actions/runs/37062437251) | `352a0f7` | **fail** | `native (windows-latest)` failed `cargo check --all-targets`: the golden test's match arm referenced `std::os::unix::fs::symlink` outside a `cfg` gate. Ubuntu, macOS, and MSRV succeeded. Repaired in `faa6c87`. |
| [`37062837529`](https://github.com/eggstack/eggplan/actions/runs/37062837529) | `faa6c87` | **fail** | `native (windows-latest)` compiled and ran, then failed the golden freeze: Windows checks the multi-line fixture out with CRLF, so the sidecar hash over raw bytes did not match the stored LF form. Ubuntu, macOS, and MSRV succeeded. Repaired in `0dd33b7`. |
| [`37063328954`](https://github.com/eggstack/eggplan/actions/runs/37063328954) | `0dd33b7` | **pass** | `native (ubuntu-latest)` 111024961828, `native (macos-latest)` 111024961887, `native (windows-latest)` 111024961786, `msrv` 111024961313 all succeeded. This is the qualification evidence for the contract. |

The two failed runs are retained deliberately: they are the only reason the
fingerprint tests are now portable and the freeze is line-ending independent.
They are not counted as passing evidence for anything.

The hosted workflow skips boundary scripts on Windows and formatting on
non-Ubuntu platforms, unchanged from `.github/workflows/ci.yml`. The dirty
fingerprint/golden tests are cross-platform except the two symlink cases, which
are Unix-gated by construction and skipped rather than removed on Windows.

Not run / not available: nothing in the plan's verification list was skipped.
The CodeGG-side commands in C001 §12 are CodeGG repository commands and were
not run from this repository; their results are quoted from CodeGG's closure
record and explicitly attributed there.

## 9. Invariant review

- Digest bytes are frozen by fixture, not by assertion of intent. Any change to
  the manifest algorithm, ordering, status bits, index treatment, encodings, or
  bounds fails `golden_dirty_manifest_digests_are_frozen`.
- One capture implementation: fingerprint and subject cannot diverge for the
  same capture.
- Exact-subject authority unchanged: a fingerprint is an input to a host's
  translation decision; Eggplan still requires full `SubjectRevision` equality
  in assessment and still recaptures under its own lock in the guarded closure
  finalizer.
- Closure authority unchanged: `finalize_closure` still owns recapture, the
  crate-private `SubjectCapture` seam is still not public, and
  `check-closure-authority-boundary.sh` still passes.
- Eggplan is still not a scheduler, executor, or issue tracker: the change adds
  one read-only capture function and one DTO.
- Managed state still does not perturb the source subject: covered by
  `fingerprint_matches_the_repository_store_subject_it_will_be_bound_to` and
  by the pre-existing `repository_managed_state_does_not_perturb_subject`.

## 10. Failure and recovery review

- Fingerprint failure never produces a partial value; the caller sees a typed
  `GitSubjectError` and no digest.
- An unusable exclusion fails closed for the new API, so a misconfigured host
  cannot obtain a falsely clean subject. The lenient historical behavior of
  `GitSubjectSource` is unchanged and explicitly pinned by tests.
- `GitSubjectFingerprintV1::validate` rejects inconsistent decoded values, so a
  host that round-trips the DTO cannot manufacture a clean-with-digest subject.
- No migration, no repair path, and no rollback concern: nothing persisted
  changed.

## 11. Migration and compatibility review

- No persisted Eggplan schema changed. Storage schema version, plan/evidence/
  closure encodings, and canonical JSON are untouched.
- The public `GitSubjectError` enum gained `InvalidExclusion` and
  `InvalidFingerprint`. Additive for consumers that only format or wrap the
  error (as CodeGG does); a downstream exhaustive `match` on `GitSubjectError`
  would need a new arm. Crate version is 0.1.0 and the workspace is
  version-locked to `3f7c603`/`0dd33b7` consumers, so this is a single
  coordinated bump.
- `GitSubjectSource` behavior is unchanged for every existing caller, including
  `RepositoryStore::subject_source`.

## 12. Security review

- The public DTO is bounded to four small fields: an OID-sized revision, an
  enum, and an optional digest. No paths, contents, index entries, symlink
  targets, or manifest bytes can cross the boundary, and no repository ID is
  exposed, so a fingerprint cannot be used to identify a repository on its own.
- No new I/O beyond the existing libgit2 + filesystem capture; no process
  execution, no network, no Git hooks, no repository-defined commands.
- `safe_worktree_path` symlink-escape confinement is unchanged and still
  enforced inside the shared capture.
- The fingerprint grants no evidence, provider, or closure authority and cannot
  satisfy assessment on its own.

## 13. Unresolved findings and open conditions

None outstanding. Both open conditions are discharged; the original table is
preserved in section 13a.

| Finding | Severity | Owner / disposition |
|---|---|---|
| CodeGG must pin `0dd33b7` and consume `capture_git_subject_fingerprint` | resolved | Discharged in `3623f65e` + `b470865a`; see section 7a. |
| CodeGG hosted requalification of the dirty bound evidence/closure path on the C001 pin | resolved | CodeGG `CI` run `37084905013` green on `main`; see section 7a. |
| `excluded_path: Option<&Path>` requires `Some(&path)` at call sites instead of the plan's suggested `Option<impl AsRef<Path>>` | low (ergonomics) | Deliberate: `Option<impl Trait>` cannot infer its type from `None`. Documented in the public signature. CodeGG's real call sites use `Some(&state_root)`. |
| `GitSubjectError` gained two variants | low | Additive; documented above. |
| A CodeGG host that still calls `store.subject_source().capture()` instead of the fingerprint gets byte-identical digests today, but is not protected against a future Eggplan algorithm change | low | Registry gate 23 and the roadmap now name the fingerprint as the required host call. The golden matrix would fail loudly on such a change. |
| No unresolved Eggplan-side defect | — | — |

## 13a. Original open conditions (preserved verbatim as history)

| Finding | Severity | Owner/disposition as first written |
|---|---|---|
| CodeGG must pin `0dd33b7` and consume `capture_git_subject_fingerprint`; until then the fingerprint contract has no real consumer | high (cross-repository closure) | CodeGG C001 (`005-m003-c001-...`), condition 1 in its closure record. Bounded substitution plus requalification. |
| CodeGG hosted requalification of the dirty bound evidence/closure path on the C001 pin | high (cross-repository closure) | CodeGG C001, condition 2. CodeGG's own record does not yet cite a run ID. |
| `excluded_path: Option<&Path>` requires `Some(&path)` at call sites instead of the plan's suggested `Option<impl AsRef<Path>>` | low (ergonomics) | Deliberate: `Option<impl Trait>` cannot infer its type from `None`. Documented in the public signature. |
| `GitSubjectError` gained two variants | low | Additive; documented above. |
| A CodeGG host that still calls `store.subject_source().capture()` instead of the fingerprint gets byte-identical digests today, but is not protected against a future Eggplan algorithm change | low | Registry gate 23 and the roadmap now name the fingerprint as the required host call. The golden matrix would fail loudly on such a change. |
| No unresolved Eggplan-side defect | — | — |

The third row is the only one that remains a live observation rather than a
closed condition, and it is a deliberate API-shape decision rather than a
defect. The fifth row is now retired: CodeGG consumes the fingerprint API at
both sites.

## 14. Roadmap disposition

- Eggplan implementation plan C001: `closed`. Production implementation
  complete, both cross-repository conditions discharged, all eleven acceptance
  criteria met.
- CodeGG integration subsystem roadmap: M001-M003 and C001 closed; the roadmap
  is terminal. No new milestone is opened.
- Projection/CLI: M001-M002 closed; M003 remains roadmap-level work waiting for
  real repository use. Not unblocked by C001.
- Eggstack integrations: M001-M002 closed; M003 remains `ready for planning`
  pending its Eggbench recheck at `d870512a5a1af16276ff05286ff0b6e2366b7f8f`.
  Not unblocked by C001, and it does not depend on C001.
- Interoperability/distribution: remains `deferred` by its own disposition. Its
  stated condition is a stable local planning/evidence contract, which C001
  strengthens; promoting it is a separate, deliberate decision, not an
  automatic consequence of this closure.
- No registered Eggplan plan lists C001 as a hard or interface dependency, so
  nothing was unblocked by it.

## 15. Registry updates

- Subsystem roadmap table: CodeGG integration `active corrective`, current
  milestone "M001-M003 closed; C001 conditionally closed on the CodeGG
  requalification".
- Dependency-ready table: C001 `ready` → `conditionally closed` → `closed`,
  linked to this record, Eggplan implementation `0dd33b7`, hosted run
  `37063328954`, and the CodeGG consumption evidence `3623f65e` / `b470865a`
  with CodeGG hosted run `37084905013`. Two earlier Windows CI failures
  (`37062437251`, `37062837529`) are retained in the record as non-passing
  evidence, as are the CodeGG-side failures summarized in section 7a.
- External interface research baselines: CodeGG row advanced from the M003-only
  baseline to the landed C001 implementation/closure commits and the merged pin
  bump.
- Key design gate 23 now names `capture_git_subject_fingerprint` as the host
  call and points at the frozen golden matrix.
- Current execution order item 9: C001 is now implemented on both sides and
  blocked only on the CodeGG pin bump; unrelated capability work does not
  serialize on it.
