# Projection and CLI M003a — Repository Inspection Snapshot and Performance Qualification

Status: **not closed — hosted qualification blocked**

Implementation landed and every local gate passes. The plan's acceptance
criterion 10 (native/MSRV hosted CI on the implementation SHA) could not be
observed because GitHub Actions stopped scheduling runners for this repository
mid-session. The milestone is therefore held at `closing` in the registry rather
than marked closed on incomplete evidence. See "Hosted qualification" below for
the exact run IDs and observed job states.

Source implementation plan:

- plans/implementation/projection-cli/003a-repository-inspection-snapshot-and-performance-qualification.md

Source roadmap:

- plans/subsystems/projection-cli-roadmap.md

Reviewed Eggplan baseline: `71904570e908a15c099f9b2804cabfbccf9ae51a`

Implementation commits:

- `528f75f21bc19f9ae35ab8b83006e0e706210df1` — the inspection snapshot, the
  `status`/`registry render`/`check` migration, the differential golden test, the
  characterization harnesses, and `architecture/repository-inspection-snapshot.md`.
- `2c32ffef0dfe0d9931a679723a24cdc17f1fccb6` — Projection/CLI M003b, which builds
  on this snapshot. Hosted qualification is being sought on this combined commit.

## Executive finding

The milestone delivered its central claim. Subject capture is now a constant two
per repository-wide read instead of scaling with Plan count, and the discarded
enumeration pass that made every file decode twice is gone — a 100-Plan clean
repository drops from 1,200 canonical decodes to 1,100, and from ~244 ms to ~48 ms.
On a dirty worktree, where each capture re-hashes the tree, the gap widens to
~545 ms → ~50 ms at 100 Plans.

Correctness was preserved rather than traded for the speedup. Deep integrity
verification is unchanged: every Plan in the repository is still validated exactly
once on every snapshot, including Plans a selection does not retain, because
`PlanStore::list` always did so and weakening it would be a silent integrity
regression dressed up as an optimization. The S1/S2 recapture detects a subject
that moves under the read and fails closed rather than projecting facts from two
different worlds.

Nine of ten acceptance criteria are met on local evidence. Criterion 10 is unmet
only because GitHub Actions stopped scheduling runners for this repository; no job
reported a failing step.

## What landed

`RepositoryStore::inspection_snapshot` is a bounded, command-local read model in
`eggplan-repo`. It replaces the pattern where repository-wide CLI reads captured
the Git subject once per Plan and reopened the same canonical files repeatedly —
work that scaled with Plan count, and whose worst case (dirty subject capture
enumerating 10,000 paths and hashing 64 MiB) dominated a repository-wide read.

The consistency model, under the shared repository lock writers also use:

1. capture subject `S1` after the lock is established;
2. enumerate Plan IDs deterministically;
3. deep-validate each selected Plan exactly once;
4. recapture subject `S2` immediately before releasing the lock;
5. return `RepoError::SubjectDrift` if `S1 != S2`.

Two further cases fail closed rather than guessing: a subject available at `S1`
but unavailable at `S2` is `InspectionSubject`, and a subject *un*available at
`S1` but available at `S2` is drift — the worktree came into existence mid-scan.
Unavailable at both ends degrades to `subject: None` so `status` can still report
`current_subject_unavailable` instead of failing outright.

## Single-pass loading, and what it fixed

`load_snapshot_unlocked` decodes the Plan, its observations, its supersessions,
its closure, and the `effective_observations` projection exactly once.
`load_unlocked` is that same function with the extra values dropped, so the two
paths cannot drift into different validation.

Loading revealed a second inefficiency the plan had not named.
`PlanStore::list` deep-loads **every** Plan and then discards the result, so
repository-wide `status` used to decode every Plan once for enumeration and then
decode the retained prefix a second time. The snapshot enumerates with a private
`plan_ids` helper and deep-loads once. On a 100-Plan repository with 10
observations each, the characterization harness records the snapshot decoding
1,100 canonical files against the previous shape's 1,200 — the 100-Plan-count
difference being exactly the discarded enumeration pass.

## Selection, retention, and memory

Every Plan is deep-validated exactly once regardless of selection. That is
deliberate: `PlanStore::list` has always deep-validated every Plan directory, so
a selection that stopped validating the remainder would weaken existing integrity
verification rather than merely change cost. The only caller choice is how much
record data to retain.

| Selection | Validation | Retention | Used by |
|---|---|---|---|
| `Repository { retain }` | every Plan | first `retain` | `status`, `check`, `list` |
| `After { after, retain }` | every Plan | first `retain` strictly after the cursor | `list` pagination |
| `Subset { ids }` | every Plan | only the requested IDs | multi-ID `status` |
| `One(id)` | that Plan | that Plan | single-Plan commands |

`check` keeps its existing shape exactly: it counts observations, supersessions,
and closures over *all* plans while projecting the first 100.
`registry render` retains everything, which was already its pre-existing
behavior and is not made unbounded by this change.

## Command adoption

`status`, `registry render`, and `check` now derive their projections from one
snapshot and one captured subject. `check --recover-pending` remains an explicit
mutation through the existing store path; only then is a read snapshot opened.
Single-plan commands keep their existing paths.

## Requirement-to-evidence matrix

| Plan requirement | Evidence |
|---|---|
| §3 read snapshot ownership | `InspectionSnapshot` and `LoadedPlanSnapshot` live in `eggplan-repo`; `eggplan-projection` stays repository-free. `snapshot_is_not_persisted_under_the_state_root` walks the whole state root before and after a snapshot and asserts byte-identical contents. |
| §4 cooperative consistency model | `subject_drift_during_inspection_fails_closed` and `a_subject_that_appears_mid_scan_is_drift_not_trust` cover the S1/S2 matrix; `a_sanctioned_writer_cannot_interleave_with_a_held_snapshot` and `shared_lock_timeout_is_bounded` cover the lock; `concurrent_snapshots_are_both_served` confirms readers do not block each other. Documented in `architecture/repository-inspection-snapshot.md`. |
| §5 single-pass loading | `each_canonical_file_is_decoded_at_most_once_per_snapshot`, `closed_plan_evidence_is_decoded_once_and_closure_once`, and the characterization decode counts. `load_unlocked` delegates to the same function. |
| §6 CLI adoption | `status_projection_is_identical_for_equivalent_logical_state`, `registry_projection_is_identical_for_equivalent_logical_state`, `check_counts_and_plan_states_are_identical_for_equivalent_logical_state` in `tests/inspection_snapshot_golden.rs`. `pending_closures_are_reported_never_silently_recovered` pins the explicit-recovery rule. |
| §7 provider-policy behavior | The snapshot carries no registry and no policy. `read_registry` on the CLI side resolves to an empty registry when no policy is supplied; `provider_identity_is_fixed_and_trust_is_host_controlled` in the integrations suite remains the trust boundary. |
| §8 projection boundary | `check-projection-cli-boundary.sh` passes unchanged; the snapshot is in the crate that already owns filesystem access and subject capture. |
| §9 algorithmic qualification | `subject_is_captured_exactly_twice_regardless_of_plan_count` (1/10/100 plans), `each_canonical_file_is_decoded_at_most_once_per_snapshot`, `selection_order_is_deterministic`, `retention_bound_limits_memory_without_reducing_validation`, `characterization_counter_matrix_runs_in_ci`. Counters are `pub(crate)` and reachable by no downstream crate. |
| §10 performance characterization | `characterization_counter_matrix_runs_in_ci` (gated, counts only) and the `#[ignore]`d `characterization_snapshot_matrix` / `characterization_dirty_subject_dominates_and_does_not_scale_with_plans` (evidence). Numbers below. |
| §11 memory and bounds | `retention_bound_limits_memory_without_reducing_validation`, `unlimited_retention_matches_the_previous_registry_render_behavior`. |
| §12 error semantics | `RepoError::SubjectDrift` → CLI `subject_drifted_during_inspection`; `RepoError::InspectionSubject` → `subject_unavailable`. `LockTimeout`, `RecoveryRequired`, `Corrupt`, `InvalidPlan`, `UnsafePath` keep their existing codes. |
| §13 required tests | Clean 1/10/100 Plan snapshots, dirty repository snapshots, S1/S2 drift, concurrent writer refusal, bounded lock timeout, closed-plan corruption detection, pending-closure reporting, golden equivalence, no public injection seam, and Rust 1.89 (below). |
| §14 documentation | `architecture/repository-inspection-snapshot.md`. |
| §16 required verification | Below. |
| §17 acceptance criteria | 1-9 met; **10 unmet** — see "Hosted qualification". |

## No public injection seam

`crates/eggplan-repo/src/lib.rs` carries 9 `compile_fail` doctests, 6 of them
pre-existing from Foundation M003 and 3 added by this milestone:

- `SnapshotCounters` cannot be named from outside the crate;
- a caller cannot inject a `SubjectCapture` into the snapshot path;
- `InspectionSnapshot` is not serializable.

The other 6 continue to pin that `SubjectCapture` and `ScriptedSubjectCapture`
are not public, that no injected closure finalizer is reachable, and that
`GitSubjectFingerprintV1` exposes no repository ID, path, or manifest bytes. All 9
doctests pass, which is why `eggplan-repo` reports 9 in its doctest count.

## Performance characterization

Recorded on a developer workstation (Linux x86_64, release-equivalent debug build),
plans x observations per plan, clean and dirty worktrees. Wall time is recorded
and **not asserted**; the gates are the counts.

| plans | obs | subject | snapshot ms | previous shape ms | snapshot captures | previous captures | snapshot decodes | previous decodes |
|---|---|---|---|---|---|---|---|---|
| 1 | 0 | clean | ~7 | ~4 | 2 | 1 | 1 | 2 |
| 10 | 0 | clean | ~13 | ~25 | 2 | 10 | 10 | 20 |
| 100 | 0 | clean | ~48 | ~244 | 2 | 100 | 100 | 200 |
| 100 | 10 | clean | ~144 | ~772 | 2 | 100 | 1100 | 1200 |
| 100 | 0 | dirty | ~50 | ~545 | 2 | 100 | 100 | 200 |
| 100 | 10 | dirty | ~127 | ~1170 | 2 | 100 | 1100 | 1200 |

The subject-capture column is the M003a claim: a constant 2 against 1, 10, or
100. The decode column shows the discarded enumeration pass removed. Wall time
improves substantially at 100 Plans, most sharply on a dirty worktree where each
extra capture re-hashes the worktree.

The `100 x 100` cell is skipped by the harness and reported as such rather than
silently dropped: 10,000 observations is inside the domain bound but is dominated
by fixture disk cost on any runner. A hosted Linux evidence run is recorded as
not observed below.

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

`cargo test --workspace` was run twice at `528f75f` with identical results (261
passed, zero failures) to confirm the suite is stable under parallel load.

Per-crate test counts at `2c32ffe`: `eggplan-core` 34, `eggplan-repo` 77,
`eggplan-projection` 4, `eggplan-integrations` 76, `eggplan-cli` 31,
`eggplan-markdown` 16, `eggplan-codegg-compat` 23.

The 3 ignored tests are the 2 characterization harnesses introduced by this
milestone (`characterization_snapshot_matrix`,
`characterization_dirty_subject_dominates_and_does_not_scale_with_plans`, both in
`snapshot.rs`) plus the 1 pre-existing ignored test
`record_golden_dirty_manifest_digests` in `git_subject_digest_golden.rs`. The
CI-gated `characterization_counter_matrix_runs_in_ci` is *not* ignored: it asserts
the counter matrix on every run, and only the wall-time replay is opt-in.

`eggplan-repo`'s 77 is 30 lib tests + 9 doctests + 38 integration tests across
`repository.rs`, `git_subject_fingerprint.rs`, and `git_subject_digest_golden.rs`.

New in this milestone: 24 `eggplan-repo` lib tests in `snapshot.rs`, of which 22
are active (including the CI-gated
`characterization_counter_matrix_runs_in_ci`) and 2 are `#[ignore]`d evidence
harnesses, plus 5 differential golden tests in
`crates/eggplan-cli/tests/inspection_snapshot_golden.rs`. `eggplan-repo` has 32
lib tests in total (24 `snapshot.rs`, 4 `git_subject.rs`, 4 `store.rs`), reported
as 30 passed and 2 ignored.

## Golden-output equivalence

A frozen byte fixture would only show the output did not change once. The
stronger proof used here is differential: both read shapes run over the same
repository and the derived projections are compared as JSON. `status`, `registry`,
and `check` counts agree across 1x0, 1x1, 10x3, and 12x2 Plan repositories, and
across a 101-Plan repository that exercises `check` truncation. A closed plan
closed through the guarded finalizer is included, covering closure presence,
lineage validation, and closure assessment reproduction. A dirty-subject variant
is covered separately.

## Invariant review

- Eggplan remains a planning/evidence mechanism. The snapshot changes how facts
  are read, never what they mean.
- Deep integrity verification is at least as strict: every Plan is still
  deep-validated exactly once, and closed-plan corruption is still detected.
- The snapshot is ephemeral process state. Nothing is serialized under
  `.eggplan`, and no cache, daemon, database, or watcher exists.
- Assessment and provider trust are untouched: the snapshot carries raw
  validated facts and no registry.
- Stable JSON projection meaning is unchanged, proved differentially.
- No mutable reference is shared into internal store state.

## Migration and compatibility review

No schema migration. Plan `SCHEMA_VERSION` and `EVIDENCE_SCHEMA_VERSION` remain
2; no envelope changed. `PlanStore::get`, `list_observations`,
`list_supersessions`, and `closure_record` keep their public safety semantics and
cost. Two new `RepoError` variants and two new CLI reason codes were added; no
existing code was replaced.

## Failure, recovery, and contention review

- **Lock contention.** A snapshot takes the shared repository lock. Multiple
  concurrent snapshots do not block each other
  (`concurrent_snapshots_are_both_served`), but a sanctioned writer cannot
  interleave with a held snapshot
  (`a_sanctioned_writer_cannot_interleave_with_a_held_snapshot`), so a read never
  observes a torn intermediate state.
- **Lock timeout.** Contention resolves through a bounded timeout, not an
  indefinite wait, so a stuck writer surfaces as a typed `LockTimeout`
  (`shared_lock_timeout_is_bounded`) rather than an unresponsive CLI.
- **Subject drift.** The failure mode this milestone exists to make safe is a
  subject that moves between `S1` and `S2`. It is detected and reported as
  `RepoError::SubjectDrift`, never silently resolved in favor of one capture.
  The asymmetric cases are handled deliberately: available-then-unavailable is
  `InspectionSubject`; unavailable-then-available is drift, because a worktree
  that appeared mid-scan is a different observation, not a better one.
- **Subject unavailable.** Unavailable at both ends degrades to `subject: None` so
  `status` can still report `current_subject_unavailable` as a warning instead of
  failing outright. This is the one deliberate non-fail-closed path, and it is
  non-fail-closed only when the fact was absent before the read began.
- **Corruption.** A closed Plan whose canonical files are damaged is still
  detected and reported as `Corrupt`, covered in the snapshot test matrix. The
  single-pass refactor did not weaken detection because `load_unlocked` now
  delegates to the same loader the snapshot uses.
- **Pending closure.** `check` still reports pending closures and never recovers
  them implicitly; `pending_closures_are_reported_never_silently_recovered` pins
  that, and `--recover-pending` remains an explicit mutation.

## Security, trust, and path review

- **Path safety.** The snapshot changed how canonical files are located, not which
  files are trusted. Every Plan path is still resolved through the existing
  `UnsafePath` rejection, and `snapshot_is_not_persisted_under_the_state_root`
  confirms the walk observes no new paths under `.eggplan`.
- **No subject-content exposure.** The snapshot returns the existing
  `GitSubjectFingerprint` value unchanged. No repository ID, path, file content, or
  manifest byte is read into the snapshot, and the subject-capture API was not
  widened.
- **No trust acquisition.** The snapshot carries validated raw facts and no
  provider registry. It cannot enroll trust, and `read_registry` still resolves to
  an empty registry when no policy is supplied.
- **No injection seam.** `SubjectCapture` cannot be supplied by a caller, so tests
  exercise drift through the crate-internal `seal` rather than by substituting a
  capture function. Three `compile_fail` doctests pin that.
- **Lock discipline.** The snapshot takes the same shared lock writers use and
  cannot interleave with a sanctioned writer
  (`a_sanctioned_writer_cannot_interleave_with_a_held_snapshot`). It writes
  nothing, so it cannot create a partial-write window.
- **Concurrency bound.** `shared_lock_timeout_is_bounded` pins a finite timeout, so
  a stuck writer surfaces as a typed `LockTimeout` rather than a hang.

## Documentation and operations

- `architecture/repository-inspection-snapshot.md` — new: why the read model
  exists, the lock and S1/S2 contract with the full terminality table, selection
  and memory behavior, the no-persistence/no-cache rule, command adoption, error
  semantics, and the no-injection-seam guarantees.

## Roadmap disposition

Projection/CLI M003a is **not closed**. It remains at `closing` in the registry
with implementation complete and one unmet acceptance criterion, and it is not a
precondition for any other milestone being worked: M003b was planned against it
explicitly and built on it, and nothing further in the Projection/CLI roadmap
depends on the hosted run.

Disposition of each acceptance criterion:

| Criterion | Disposition |
|---|---|
| 1-9 | Met on the evidence recorded above. |
| 10 (hosted native + MSRV CI) | **Unmet — blocked externally.** See unresolved finding 1. |

Re-running qualification on `2c32ffef0dfe0d9931a679723a24cdc17f1fccb6` and
recording the result is the only work remaining to close this milestone. No
corrective plan is required: no defect was found.

## Registry updates

- M003a row left at `closing` with this record, explicitly noting the blocked
  hosted qualification.

## Unresolved findings

1. **Blocked — hosted native/MSRV qualification was not observed.** Run
   [37365396150](https://github.com/eggstack/eggplan/actions/runs/37365396150) on
   `528f75f` reached `msrv` success (`111949172887`) and
   `native (windows-latest)` success (`111949173179`), but
   `native (ubuntu-latest)` was **cancelled** (`111949173133`) and
   `native (macos-latest)` never left `queued`. That run was cancelled to release
   queue capacity. The combined commit `2c32ffe` has run
   [37368489637](https://github.com/eggstack/eggplan/actions/runs/37368489637)
   queued with all four jobs unscheduled; observed still unscheduled 13.5 minutes
   after creation (`20:14:24Z` to `20:27:52Z`) with none started. The two
   documentation-only pushes that followed (`559e57b`, `6521613`) queued runs
   `37369527303` and `37369775115`, which are also unscheduled, so the condition
   is not specific to one commit. Prior runs on this repository scheduled and
   completed within about 3 minutes, so this is a queue delay, not a failure. No
   job reported a failing step. This is a GitHub Actions runner-availability
   condition outside the repository, recorded as blocked rather than substituted,
   and the milestone is not marked closed.
2. **Informational — the characterization harness self-reports timings on one
   machine.** Wall-time figures are indicative only. The gated matrix asserts
   counts; the ignored harness replays the previous shape over the same fixture
   so the comparison is like-for-like rather than against a remembered number.
3. **Informational — `registry render` remains unbounded by retention.** It
   retains every plan, as it did before this milestone. The snapshot does not
   change that, and the memory behavior is documented rather than silently
   altered.
4. **Informational — the first attempt at the `Subset` selection hit a debug
   assertion.** `debug_assert!(selection.is_repository_wide())` did not cover the
   subset form. It was corrected to cover all multi-plan selections before the
   milestone was pushed, and the bug is recorded here because it is the kind of
   gap a future selection variant could reintroduce.

## Residual findings for M003b

Projection/CLI M003b consumed this read model as planned: repository-wide batch
execution uses `After` for keyset pagination and `Subset` for explicit ID sets, so
neither reintroduces per-Plan subject capture. M003b is implemented at `2c32ffe`
and shares the same blocked hosted qualification recorded above.

## Hosted qualification

**Not observed.** See unresolved finding 1. No hosted result is claimed for this
milestone. Re-run qualification on `2c32ffe` when runner capacity is available and
replace this section with the concrete run and job IDs before marking the
milestone closed.