# Deep dive: `eggplan-repo`

Part of the [overview index](overview.md).
Normative companions: [repository storage](repository.md),
[evidence and assessment](evidence.md), [core](core.md).

Status context, per `plans/registry.md` as the authority: foundation M001 is
closed and M003 is closed with cross-platform qualification, while M002 is
recorded **conditionally closed** — a historical closure whose platform caveat
was resolved by M003 (`plans/registry.md:66-68`,
`plans/subsystems/foundation-core-roadmap.md`); evidence-closure M002 plus its
C001/C002/C003 are closed (`plans/registry.md:71-74`,
`plans/subsystems/evidence-closure-roadmap.md`); and CodeGG integration M003
C001, the dirty-subject fingerprint work described in §3a, is closed on the
Eggplan side at `0dd33b7` (`plans/registry.md:81`). This note reviews the code
as read; it is not closure evidence.

## 1. Crate role

`eggplan-repo` (`crates/eggplan-repo/Cargo.toml:1-22`) is the safe local
persistence layer: `.eggplan/` layout, plan CAS, the append-only evidence
ledger and supersession store, and the guarded closure finalizer that owns
current-subject authority. Dependencies are deliberately narrow
(`eggplan-core`, `fs2`, `git2`, `serde`/`serde_json`, `sha2`, `tempfile`,
`thiserror`, `toml`, `uuid`) — no process execution, network, or provider
acquisition in `src/` (the sole `std::process` use is a test fixture, §5),
matching the [repository](repository.md) and
[evidence](evidence.md) boundaries.

- **`.eggplan` layout.** `open`/`open_with_options`
  (`crates/eggplan-repo/src/store.rs:574-616`) ensure `root/` + `plans/`,
  create or validate `config.toml` (schema version 1, stable `epr_` identity;
  validation at `store.rs:1123-1137`), then run pending-closure recovery and a
  full `list()` validation pass. Per-plan directories hold `plan.json`,
  `evidence/<id>.json`, `supersessions/<id>.json`, `closure.pending.json`,
  and `closure.json`, as documented in [repository](repository.md).
- **Plan CAS semantics.** `StoredPlan` (`store.rs:46-52`) wraps the plan with
  `storage_version: 1` and a canonical digest. `create` (`store.rs:851-878`)
  requires revision 0 + Draft; `compare_and_swap` (`store.rs:909-957`)
  requires `next.revision == expected + 1`, checks the persisted revision
  under the lock, and returns typed `Conflict` on staleness. Ordinary CAS can
  never *enter* Closed from a non-Closed plan (`GuardedClosureRequired`,
  `store.rs:928-932`) — but it can *rewrite* a plan that is already Closed; see
  finding 6.1, which is a real code defect, not a documentation nuance.
- **Evidence ledger.** `append_observation` (`store.rs:959-998`) validates,
  then compares canonical bytes for idempotent replay vs
  `ObservationConflict`; `list_observations_unlocked`
  (`store.rs:1038-1085`) sorts by typed ID, skips `.tmp-*` staging remnants,
  and rejects non-`.json` entries. Limits: 10,000 observations per plan
  (`store.rs:969,1078` via core bounds), 1 MiB per observation
  (`store.rs:26,1168-1173`), 16 MiB per plan (`store.rs:25,696-700`).
- **Closure finalizer as subject authority.** `finalize_closure`
  (`store.rs:358-367`) is the only public finalizer; it builds a
  `GitSubjectCapture` from `self.subject_source()` and delegates to the
  crate-private hook. It recaptures the Git `SubjectRevision` under the lock,
  never accepting a caller-owned subject. See sections 2–4.

## 2. Module / function walkthrough

`lib.rs` (`crates/eggplan-repo/src/lib.rs:1-84`) forbids `unsafe_code`,
declares `git_subject` + `store`, and re-exports exactly
`capture_git_subject_fingerprint`, `GitSubjectError`,
`GitSubjectFingerprintV1`, `GitSubjectOptions`, `GitSubjectSource`,
`PlanStore`, `RepoError`, `RepositoryStore`, `StoreOptions`.

- **Open paths.** `open_read_only` (`store.rs:172-197`) reads `config.toml`
  without creating files or recovering pending closures — the inspection
  path for check commands. `pending_closures` (`store.rs:200-230`) lists
  pending intents sorted, rejecting symlinked entries. `open_with_options`
  (`store.rs:578-616`) takes a short-lived init lock, then recovers.
- **Plan CAS.** Above plus `load_unlocked` (`store.rs:677-847`): storage
  version check, domain validation, digest check, directory/object identity
  check, `RecoveryRequired` if `closure.pending.json` exists
  (`store.rs:737-739`), and — for Closed plans — full closure-record
  validation including reproduced source digest, provider-policy rebuild,
  reproducible assessment, evidence digests, and supersession lineage
  (`store.rs:744-830`). Unknown/corrupt objects fail closed (`Corrupt`,
  `InvalidPlan`).
- **Evidence append / idempotency.** Byte-identical replay of the same ID
  returns `Ok(())`; same ID with different canonical bytes returns
  `ObservationConflict` (`store.rs:978-987`). Post-write re-read guards
  against partial writes (`store.rs:993-996`).
- **Supersessions.** `append_supersession` (`store.rs:489-520`) validates the
  record, requires both endpoints to exist, rejects writes to Closed plans,
  rejects filename collisions, and re-runs `effective_observations` before
  persisting. `list_supersessions_unlocked` (`store.rs:530-572`) checks
  filename/identity agreement and sorts by ID.
- **`finalize_closure` two-capture protocol** (`store.rs:383-487`): lock →
  reload current and require exact source revision + digest → capture S1,
  require equality with candidate subject (`ClosureSubjectStale` on
  mismatch) → reload observations + supersessions, rebuild the provider
  registry from the candidate's bounded snapshot via `register_trusted`
  (`store.rs:408-415`), recompute `assess_plan` and require Complete +
  equality → verify satisfying-observation digests, re-derive the expected
  observation set from the assessment criteria and require it to equal
  `candidate.satisfying_observations` (`store.rs:428-447`), provider-policy
  digest, and supersession lineage → build the Closed plan and `ClosureRecord` →
  refuse if `closure.json`/`closure.pending.json` already exist → capture S2
  immediately before the first closure write, require `S2 == S1 ==
  candidate.subject` (`ClosureSubjectDrift` on drift) → write pending, write
  plan, rename pending to final. Neither stale, drift, nor capture failure
  produces pending/final state or a Closed plan.
- **Pending recovery.** `recover_pending_closures` (`store.rs:262-344`),
  run under the lock at open: exact closed target (revision + digest) with a
  valid record promotes pending to final; exact source match discards;
  anything else (including both-files-present) is `Corrupt`. `closure_record`
  (`store.rs:234-260`) revalidates before returning, with an 8 MiB size cap.
- **Staging / sync.** `atomic_write` (`store.rs:1204-1226`): stage
  `.tmp-*` in the destination directory, `sync_all` file bytes, refuse to
  replace symlinks/non-files, `persist` (rename), then sync the parent
  directory on Unix (`DurabilityUnknown` on failure).
  `abandoned_staging_files` (`store.rs:628-650`) reports `.tmp-*` remnants
  without loading them.
- **Symlink / path validation.** Every managed root, directory, plan file,
  ledger entry, and lock file goes through `symlink_metadata` +
  `check_dir`/`ensure_dir` (`store.rs:1181-1202`); symlinks and non-
  directories fail as `UnsafePath`. Plan directory names must parse as
  `PlanId` (restricted alphabet), observation filenames must match
  `<id>.json`.
- **Error taxonomy.** `RepoError` (`store.rs:54-108`) declares 24 variants and
  constructs all but one: `Domain` (`store.rs:58-59`) is an unused
  `Box<dyn Error + Send + Sync>` catch-all that can never carry a diagnostic.
  Lifecycle triggers: `InvalidUpdate` for identity/revision/digest/assessment
  mismatch and every pre-write refusal, `InvalidTransition` for plan or item
  status moves the core transition tables forbid (`store.rs:933-948`),
  `GuardedClosureRequired` for CAS entry into Closed, `Conflict` for a stale
  expected-revision, `AlreadyExists`/`NotFound` for directory presence,
  `LockTimeout` for `fs2` contention past the deadline
  (`store.rs:1108-1120`), `DurabilityUnknown` when a rename succeeded but the
  parent-directory `sync_all` did not (`store.rs:482-485,1221-1224`),
  `UnsafePath` for a symlinked or non-directory managed path, `Corrupt` for
  unknown storage schema, digest mismatch, ID/filename disagreement and
  unreproducible closure chains, `InvalidPlan` for a domain-valid plan that
  still fails load-time checks, `Evidence`/`ObservationLimit`/
  `ObservationConflict`/`ObservationNotFound` for ledger conditions,
  `RecoveryRequired` for a present pending record, and
  `ClosureSubjectStale`/`ClosureSubjectDrift`/`ClosureSubjectCapture` for the
  finalizer. `GitSubjectError` (`git_subject.rs:27-49`) adds `NotGit`, `Git`,
  `Io`, `NonUnicodePath`, `BoundExceeded`, `Unborn`, `UnsafePath`,
  `InvalidSubject`, `InvalidExclusion`, `InvalidFingerprint`; §3 and §3a return
  one identical set from the single capture implementation.

## 3. Git `SubjectRevision` capture as implemented

`GitSubjectSource` (`crates/eggplan-repo/src/git_subject.rs:140-187`):
`new` + `with_options` + `excluding_path`
(`git_subject.rs:148-166`), then `capture` (`git_subject.rs:168-186`),
which is a thin wrapper over the shared `capture_subject`
(`git_subject.rs:211-237`). `Repository::discover` from the configured root
(libgit2 only — no spawned Git, hooks, or repo-defined commands); every
discover failure is collapsed to `NotGit`
(`git_subject.rs:217-218`), so an unreadable or corrupt repository also
reports "not inside a Git worktree"; HEAD OID required (`Unborn` if none).
Clean subjects carry
repository ID + HEAD OID; dirty trees get a bounded sorted manifest
(`dirty_manifest`, `git_subject.rs:305-392`): status bits, index blob IDs,
regular file bytes, symlink targets, nested submodule HEAD/dirty manifests;
untracked files and submodules participate, ignored files do not. Default
bounds are 10,000 paths, 64 MiB worktree bytes, 8 submodule levels
(`git_subject.rs:17-25`); exceeding a bound fails (`BoundExceeded`) rather
than returning a partial fingerprint. Non-Unicode paths fail
(`NonUnicodePath`). Status paths are confined by `safe_worktree_path`
(`git_subject.rs:394-419`), which also rejects descent through symlinked
intermediate directories.

`capture_subject` is the single capture implementation: it discovers the
repository, resolves the HEAD, resolves the exclusion, and returns the
repository-ID-free `CapturedSubject` (revision, state, dirty digest). Both
public entry points derive their public value from that one result, so
`SubjectRevision` and `GitSubjectFingerprintV1` cannot disagree for one
capture. `ExclusionMode` (`git_subject.rs:203-209`) only decides how an
unusable exclusion is treated; it never changes manifest bytes, row ordering,
status bits, index-entry treatment, or bounds.

Managed-root exclusion: `RepositoryStore::subject_source`
(`store.rs:624-626`) wires `.excluding_path(&self.root)`; `resolve_exclusion`
(`git_subject.rs:239-279`) canonicalizes both roots (handling macOS `/var` →
`/private/var` aliasing) and `dirty_manifest` drops normalized status paths
equal to or under the exclusion
(`git_subject.rs:329-334`), independent of `.gitignore` and
tracked/staged/ignored/untracked state. A sibling sharing only a string
prefix stays in scope (comparison is component-based). Submodule manifests
are evaluated independently (exclusion is not propagated at
`git_subject.rs:374`). Outside-worktree state paths apply no exclusion under
`ExclusionMode::Lenient`. The HEAD OID remains in the subject, so committing
administrative state still changes identity — all per
[repository](repository.md).

## 3a. Git subject fingerprint as implemented

`GitSubjectFingerprintV1` (`git_subject.rs:66-103`) and
`capture_git_subject_fingerprint` (`git_subject.rs:118-137`) expose the exact
revision, clean/dirty state, and Eggplan-native dirty digest an external host
must persist for later exact-subject assessment. Added by CodeGG integration
M003 C001 (`plans/registry.md:81`; the algorithm bytes were frozen from the
pre-C001 implementation, see the next bullet).

- **Versioned and strict.** `SCHEMA_VERSION == 1`; `deny_unknown_fields`;
  `validate` rejects a wrong schema version, blank revision, clean-with-digest,
  dirty-without-digest, and any digest that is not `sha256:<64 lowercase
  hex>`.
- **Repository-ID-free by construction.** No repository identity, path list,
  index entry, symlink target, content, or manifest byte is present. Three
  crate-root `compile_fail` doctests (`lib.rs:53-75`, restated in §4) fail the
  build if such a field is ever added.
- **Same algorithm, one implementation.** Fingerprint and subject read the
  same `capture_subject`; the digest bytes are frozen by
  `crates/eggplan-repo/tests/git_subject_digest_golden.rs` (fixture
  `tests/fixtures/git-subject-digests-v1.json` + `.sha256` sidecar) captured
  from the pre-C001 implementation at revision `52a4be76`.
- **Fail-closed exclusion.** `ExclusionMode::Strict` rejects an exclusion that
  does not resolve inside the discovered worktree, names the worktree root
  itself, or is requested against a repository with no worktree
  (`InvalidExclusion`). `GitSubjectSource` keeps its historical lenient
  resolution (`ExclusionMode::Lenient`).
- **Same typed failures and bounds.** Bound overflow, unsafe/symlinked paths,
  non-Unicode paths, Git errors, unborn HEAD, and filesystem read failures
  return the same `GitSubjectError` classes as normal subject capture and never
  return a partial digest.
- **Not an authority.** A fingerprint grants no evidence, provider, or closure
  authority, and Eggplan provides no repository-ID relabeling constructor for
  it: identity proof stays the host's responsibility. All four fields are
  `pub` and the struct derives `Deserialize` with no validating hook, so a host
  can build or decode an arbitrary fingerprint; `validate()` is a separate
  public method the host must call itself (the only production call site is
  `git_subject.rs:135`).

## 4. Closure-authority boundary as implemented

Three layers, matching [repository](repository.md) and
[evidence](evidence.md):

1. **Crate-private seam.** `SubjectCapture` trait + `GitSubjectCapture`
   adapter + `finalize_closure_with_capture` hook are all `pub(crate)`
   (`store.rs:110-136,373-381`); the `ScriptedSubjectCapture` test double
   lives inside `store.rs`'s `#[cfg(test)]` module (`store.rs:1255-1275`)
   and cannot be named from another crate.
2. **Compile-fail doctests.** `lib.rs:15-42` assert that naming
   `SubjectCapture` / `ScriptedSubjectCapture` or calling
   `finalize_closure_with_capture` from outside fails to compile;
   `lib.rs:44-75` assert that a fingerprint exposes no `repository_id`,
   `paths`, or `dirty_manifest` field. Six `compile_fail` doctests in all.
3. **Static guard.** `scripts/check-closure-authority-boundary.sh` strips
   comments and rejects `pub use` re-exports (direct and grouped,
   multiline-aware) of the three hidden types in `lib.rs`/`store.rs`, plus
   `pub trait`/`pub struct` declarations of those names and any
   `pub fn finalize_closure_with_*`; it self-tests with synthetic
   positive/negative proofs before scanning the real sources. `git_subject.rs`
   is not scanned — the fingerprint is public API by design, so nothing in it
   needs this guard.

## 5. Test strategy

- **Integration (`crates/eggplan-repo/tests/repository.rs`, 976 lines,
  22 tests):** CAS/reopen round-trips, read-only-open pending reporting,
  thread *and* multi-process contention (exactly-one-winner via barrier and
  spawned processes), lock timeout + staging reports, stale-revision after
  lock-file loss, symlink rejection (plan dir, plan file, ledger), lifecycle
  rejection, corrupt/unknown-field rejection (plan + evidence artifact),
  ledger idempotency, Git clean/staged/untracked/dirty fingerprints,
  managed-state exclusion, bound-evidence end-to-end, guarded closure
  persistence + reopen, non-Git error, submodule manifests, stale-subject
  abort with no partial state, and
  `historical_closure_subject_becomes_stale_after_worktree_change`
  (`repository.rs:942`) — the read-surface staleness assertion behind finding
  6.2. The one lifecycle-rejection test (`repository.rs:292-311`) covers only
  Draft→Closed, which is why 6.1 is untested.
- **Fingerprint (`crates/eggplan-repo/tests/git_subject_fingerprint.rs`,
  632 lines, 13 tests, one Unix-gated) plus the frozen digest matrix
  (`tests/git_subject_digest_golden.rs`, 320 lines, 2 tests + 1 `#[ignore]`
  recorder):**
 fingerprint/subject equality across clean, unstaged, staged,
  staged+unstaged, untracked, deleted, symlink, staged-rename, and dirty
  nested-submodule states; single administrative-root exclusion including
  prefix-sharing siblings; strict invalid/outside/root/missing/bare
  exclusions; path/content/depth bounds; typed discovery failures; the
  two-capture dirty-change sandwich primitive; equality with the
  `RepositoryStore` subject a fingerprint will be bound to; schema-versioned
  strict serde round-trip and validation. The matrix harness carries two
  deliberate portability accommodations: the `.sha256` sidecar is asserted
  over the LF-normalized fixture because hosted Windows runners check out
  CRLF (`git_subject_digest_golden.rs:238-245`), and `symlink_typechange`
  stays frozen in the fixture but is asserted only where Unix symlinks exist
  (`:42-51,266-280`). The submodule fixture is this crate's only process
  execution and it is test-only: it shells out to the `git` CLI with
  `protocol.file.allow=always` and `core.hooksPath=/dev/null` (`:32,154-195`).
- **Unit-in-crate (`store.rs:1228-1478`, `git_subject.rs:436-549`):**
  deterministic S1/S2 regressions via the scripted seam (stable success,
  drift → `ClosureSubjectDrift`, S2 failure → `ClosureSubjectCapture`, all
  asserting no partial closure state), a visibility test pinning the
  `pub(crate)` hook, and `safe_worktree_path` traversal/symlink tests.
- The split is deliberate: authority-dependent determinism tests stay
  crate-internal; everything externally observable is integration-tested.

## 6. Review findings

**Strengths.** The two-capture protocol genuinely narrows the TOCTOU
window (S2 immediately precedes the first closure write); error taxonomy
keeps subject failures (`ClosureSubjectStale`/`Drift`/`Capture`) as typed
`RepoError`s distinct from core lifecycle errors; reopen re-validates the
entire closure chain rather than trusting `closure.json`; atomic-write and
symlink discipline are applied uniformly, not just on the plan path; and
`src/` contains **no** production panic path — every `unwrap`/`expect` in
`store.rs` and `git_subject.rs` is inside a `#[cfg(test)] mod tests`
(`store.rs:1228-1478`, `git_subject.rs:435-549`), and the only other
`unimplemented!` is inside a `compile_fail` doctest (`lib.rs:23`).

**Gaps / risks / surprises.**

- **6.1 `compare_and_swap` can rewrite an already-Closed plan, and the store
  then refuses to open (code defect).** The guard at `store.rs:928-932` fires
  only when `current.status != Closed`; for Closed→Closed the status-equality
  short-circuit at `store.rs:933` skips `plan_transition_allowed`, and
  `plan_transition_allowed` has no `Closed` row at all
  (`crates/eggplan-core/src/model.rs:391-403`). Nothing else blocks it:
  `load_unlocked` validates the *pre-write* plan (`store.rs:920`) and
  `Plan::validate` constrains no Closed-plan content or revision
  (`crates/eggplan-core/src/model.rs:288-347`). So `atomic_write` at
  `store.rs:955` persists a Closed plan at `revision + 1` that no longer
  matches `closure.json`, and the trailing re-read at `store.rs:956` then
  returns `Corrupt` — `ClosureRecord::validate` pins both
  `final_plan_revision` and `final_plan_digest`
  (`crates/eggplan-core/src/closure.rs:253-262`, enforced at
  `store.rs:750-755`). The error is reported *after* the bad bytes land, the
  plan directory is left permanently unreadable, and because `open` runs
  `store.list()` over every plan (`store.rs:614`) the whole state root stops
  opening, with no recovery path (`recover_pending_closures` only handles
  `closure.pending.json`, `store.rs:262-344`). The resulting state is exactly
  the one [evidence](evidence.md) calls corruption — "a Closed Plan without a
  matching record is corruption" (`evidence.md:85-86`) — reachable by an
  ordinary CAS rather than by tampering. No closure record is forged and
  no evidence, provider, or closure authority is gained — the record still
  pins the pre-write plan — so the blast radius is durable availability, not
  closure bypass. It is reachable from the public `PlanStore::compare_and_swap`
  with only a legitimately closed plan, and untested:
  `tests/repository.rs:291-311` asserts only the Draft→Closed direction. What
  would defend it: make the guard unconditional (`if next.status == Closed`),
  or refuse any write to a Closed plan at `store.rs:920`.
- **6.2 Post-S2 window is documented, not closed.** After the S2 recapture any
  further worktree change leaves the finalized record valid; staleness is
  reported by read surfaces. That matches [evidence](evidence.md) but
  reviewers should not mistake two-capture for a worktree lock.
- **6.3 S1/S2 themselves cannot race, and pending recovery is idempotent.**
  Both captures run under one `LockGuard` taken at `store.rs:390` and held to
  `store.rs:486`, and every other mutator (`create` 853, `compare_and_swap`
  919, `append_observation` 965, `append_supersession` 498,
  `recover_pending_closures` 263) takes the same `.lock`, so no interleaved
  finalization or ledger write is possible. The only writers S1/S2 do not
  exclude are non-cooperating actors outside the process, which is finding
  6.2. The three-write order at `store.rs:479-481` has exactly two reachable
  crash points — pending+source plan, discarded at `store.rs:327-331`, and
  pending+Closed plan, promoted at `store.rs:312-326` — so recovery is
  genuinely idempotent for both, and both-files-present fails closed as
  `Corrupt` (`store.rs:306-311`).
- **6.4 Read surfaces take no lock.** `get`/`list`/`list_observations`/
  `closure_record` (`store.rs:880-882,884-907,1031-1034,234-260`) never
  acquire `.lock`, so a reader can observe the pre-finalize snapshot, and
  `closure_record` racing the rename sees `RecoveryRequired`
  (`store.rs:737-739`) rather than a half-record. Fail-closed but not
  snapshot-consistent; callers must tolerate the error.
- **6.5 Durability claims are platform-limited.** Unix directory `sync_all` is
  best-effort portable; macOS is not `F_FULLFSYNC`; Windows has no portable
  directory sync here; network filesystems may weaken `fs2` advisory
  locking — all disclosed in [repository](repository.md), with M003
  owning native qualification evidence.
- **6.6 `exists()` follows symlinks at all six decision sites**
  (`store.rs:470,512,587,681,737,857`). Each mislabels a symlinked path:
  a symlinked `closure.json`/`closure.pending.json` yields `InvalidUpdate`
  (470), a symlinked supersession filename yields `InvalidUpdate` (512), a
  symlinked plan directory yields `AlreadyExists` (857) or `NotFound` when
  dangling (681), and a symlinked pending record yields `RecoveryRequired`
  (737) instead of `UnsafePath`. A symlinked `config.toml` is the one case
  that still reports `UnsafePath`, either at `store.rs:590` (live target) or
  from `atomic_write` (`store.rs:1212-1215`) after a dangling link takes the
  create branch. Fail-closed in every case, but the diagnostics hide an
  attack-shaped condition. Every other path check uses `symlink_metadata` and
  does report `UnsafePath`.
- **6.7 Cooperative lock only.** `acquire_lock` (`store.rs:1095-1121`) is
  `fs2` advisory locking with a 5 s default timeout (`store.rs:32-38`); a
  holder that bypasses the lock file or a stale NFS lock can break mutual
  exclusion. The contention tests cover cooperating processes only.
- **6.8 Lock-file deletion race.** `acquire_lock` opens/creates `.lock` by path
  each time; deleting the lock file while a holder exists can split
  waiters across inodes. There is a reopen-after-delete test
  (`repository.rs:239`), but it asserts staleness rejection, not lock
  integrity.
- No *behavioral* inconsistency with `architecture/*.md` was found. Two
  cross-document notes, for the owners of those files: `repository.md:37` and
  `evidence.md:86` both say CAS "cannot transition to" / "rejects transitions
  to" Closed, which is accurate as written but silent on the Closed→Closed
  write of finding 6.1, so neither normative doc currently states that a
  Closed Plan is immutable; and `repository.md:59-60` cites "a crate-level
  `compile_fail` doctest" where `lib.rs` carries six. No other
  documentation gap remains in this tree.

## Verification pointers

```sh
cargo test -p eggplan-repo
cargo test -p eggplan-repo --test repository
bash scripts/check-closure-authority-boundary.sh
```
