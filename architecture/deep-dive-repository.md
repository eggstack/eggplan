# Deep dive: `eggplan-repo`

Part of the [overview index](overview.md).
Normative companions: [repository storage](repository.md),
[evidence and assessment](evidence.md), [core](core.md).

Status context: foundation milestones M001/M002/M003 are closed with Linux
qualified and Windows/macOS qualified via the supported-subset workflow
(`plans/subsystems/foundation-core-roadmap.md`); evidence-closure M002 plus
C001/C002/C003 are closed (`plans/subsystems/evidence-closure-roadmap.md`).
This note reviews the code as read; it is not closure evidence.

## 1. Crate role

`eggplan-repo` (`crates/eggplan-repo/Cargo.toml:1-22`) is the safe local
persistence layer: `.eggplan/` layout, plan CAS, the append-only evidence
ledger and supersession store, and the guarded closure finalizer that owns
current-subject authority. Dependencies are deliberately narrow
(`eggplan-core`, `fs2`, `git2`, `serde`/`serde_json`, `sha2`, `tempfile`,
`thiserror`, `toml`, `uuid`) — no process execution, network, or provider
acquisition, matching the [repository](repository.md) and
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
  never enter Closed (`GuardedClosureRequired`, `store.rs:928-932`).
- **Evidence ledger.** `append_observation` (`store.rs:959-998`) validates,
  then compares canonical bytes for idempotent replay vs
  `ObservationConflict`; `list_observations_unlocked`
  (`store.rs:1038-1086`) sorts by typed ID, skips `.tmp-*` staging remnants,
  and rejects non-`.json` entries. Limits: 10,000 observations per plan
  (`store.rs:969,1078` via core bounds), 1 MiB per observation
  (`store.rs:26,1168-1173`), 16 MiB per plan (`store.rs:25,696-700`).
- **Closure finalizer as subject authority.** `finalize_closure`
  (`store.rs:358-367`) is the only public finalizer; it builds a
  `GitSubjectCapture` from `self.subject_source()` and delegates to the
  crate-private hook. It recaptures the Git `SubjectRevision` under the lock,
  never accepting a caller-owned subject. See sections 2–4.

## 2. Module / function walkthrough

`lib.rs` (`crates/eggplan-repo/src/lib.rs:1-48`) forbids `unsafe_code`,
declares `git_subject` + `store`, and re-exports exactly
`GitSubjectError`, `GitSubjectOptions`, `GitSubjectSource`, `PlanStore`,
`RepoError`, `RepositoryStore`, `StoreOptions`.

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
  equality → verify satisfying-observation digests, provider-policy digest,
  and supersession lineage → build the Closed plan and `ClosureRecord` →
  refuse if `closure.json`/`closure.pending.json` already exist → capture S2
  immediately before the first closure write, require `S2 == S1 ==
  candidate.subject` (`ClosureSubjectDrift` on drift) → write pending, write
  plan, rename pending to final. Neither stale, drift, nor capture failure
  produces pending/final state or a Closed plan.
- **Pending recovery.** `recover_pending_closures` (`store.rs:262-344`),
  run under the lock at open: exact closed target (revision + digest) with a
  valid record promotes pending to final; exact source match discards;
  anything else (including both-files-present) is `Corrupt`. `closure_record`
  (`store.rs:233-260`) revalidates before returning, with an 8 MiB size cap.
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

## 3. Git `SubjectRevision` capture as implemented

`GitSubjectSource` (`crates/eggplan-repo/src/git_subject.rs:46-120`):
`new` + `with_options` + `excluding_path`
(`git_subject.rs:55-73`), then `capture` (`git_subject.rs:75-119`).
`Repository::discover` from the configured root (libgit2 only — no spawned
Git, hooks, or repo-defined commands); HEAD OID required (`Unborn` if none);
`NotGit` outside a worktree. Clean subjects carry repository ID + HEAD OID;
dirty trees get a bounded sorted manifest (`dirty_manifest`,
`git_subject.rs:146-233`): status bits, index blob IDs, regular file bytes,
symlink targets, nested submodule HEAD/dirty manifests; untracked files and
submodules participate, ignored files do not. Default bounds are 10,000
paths, 64 MiB worktree bytes, 8 submodule levels (`git_subject.rs:16-24`);
exceeding a bound fails (`BoundExceeded`) rather than returning a partial
fingerprint. Non-Unicode paths fail (`NonUnicodePath`). Status paths are
confined by `safe_worktree_path` (`git_subject.rs:235-260`), which also
rejects descent through symlinked intermediate directories.

Managed-root exclusion: `RepositoryStore::subject_source`
(`store.rs:624-626`) wires `.excluding_path(&self.root)`; `capture`
canonicalizes both roots (handling macOS `/var` → `/private/var` aliasing)
and drops normalized status paths equal to or under the exclusion
(`git_subject.rs:84-100,170-174`), independent of `.gitignore` and
tracked/staged/ignored/untracked state. A sibling sharing only a string
prefix stays in scope (comparison is component-based). Submodule manifests
are evaluated independently (exclusion is not propagated at
`git_subject.rs:215`). Outside-worktree state paths apply no exclusion.
The HEAD OID remains in the subject, so committing administrative state
still changes identity — all per [repository](repository.md).

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
   `finalize_closure_with_capture` from outside fails to compile.
3. **Static guard.** `scripts/check-closure-authority-boundary.sh` strips
   comments and rejects `pub use` re-exports (direct and grouped,
   multiline-aware) of the three hidden types in `lib.rs`/`store.rs`, plus
   any `pub fn finalize_closure_with_*`; it self-tests with synthetic
   positive/negative proofs before scanning the real sources.

## 5. Test strategy

- **Integration (`crates/eggplan-repo/tests/repository.rs`, 976 lines,
  ~20 tests):** CAS/reopen round-trips, read-only-open pending reporting,
  thread *and* multi-process contention (exactly-one-winner via barrier and
  spawned processes), lock timeout + staging reports, stale-revision after
  lock-file loss, symlink rejection (plan dir, plan file, ledger), lifecycle
  rejection, corrupt/unknown-field rejection (plan + evidence artifact),
  ledger idempotency, Git clean/staged/untracked/dirty fingerprints,
  managed-state exclusion, bound-evidence end-to-end, guarded closure
  persistence + reopen, non-Git error, submodule manifests, stale-subject
  abort with no partial state.
- **Unit-in-crate (`store.rs:1228-1478`, `git_subject.rs:267-297`):**
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
symlink discipline are applied uniformly, not just on the plan path.

**Gaps / risks / surprises.**

- **Post-S2 window is documented, not closed.** After the S2 recapture any
  further worktree change leaves the finalized record valid; staleness is
  reported by read surfaces. That matches [evidence](evidence.md) but
  reviewers should not mistake two-capture for a worktree lock.
- **Durability claims are platform-limited.** Unix directory `sync_all` is
  best-effort portable; macOS is not `F_FULLFSYNC`; Windows has no portable
  directory sync here; network filesystems may weaken `fs2` advisory
  locking — all disclosed in [repository](repository.md), with M003
  owning native qualification evidence.
- **`exists()` follows symlinks at two pre-write checks**
  (`store.rs:470-472` for pending/final, `store.rs:737`). A pre-existing
  symlink at those paths yields `InvalidUpdate`/`RecoveryRequired` rather
  than `UnsafePath`; fail-closed either way, but the diagnostic mislabels
  an attack-shaped condition.
- **Cooperative lock only.** `acquire_lock` (`store.rs:1095-1121`) is
  `fs2` advisory locking with a 5 s default timeout (`store.rs:32-38`); a
  holder that bypasses the lock file or a stale NFS lock can break mutual
  exclusion. The contention tests cover cooperating processes only.
- **Lock-file deletion race.** `acquire_lock` opens/creates `.lock` by path
  each time; deleting the lock file while a holder exists can split
  waiters across inodes. There is a reopen-after-delete test
  (`repository.rs:239`), but it asserts staleness rejection, not lock
  integrity.
- No inconsistency with `architecture/*.md` was found in behavior; the one
  documentation gap is that the forward-referenced `architecture/overview.md`
  index does not exist yet in this tree.

## Verification pointers

```sh
cargo test -p eggplan-repo
cargo test -p eggplan-repo --test repository
bash scripts/check-closure-authority-boundary.sh
```
