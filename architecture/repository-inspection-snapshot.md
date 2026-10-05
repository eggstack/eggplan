# Repository inspection snapshot

`RepositoryStore::inspection_snapshot` is Eggplan's bounded, command-local
repository read model. It exists because repository-wide CLI reads used to
capture the Git subject once per Plan and reopen the same canonical files
repeatedly.

Dirty subject capture can enumerate 10,000 paths and hash 64 MiB of content.
Multiplying that by Plan count was the dominant cost of a repository-wide read.
The snapshot collapses it into one cooperatively locked, revision-checked pass.

## What it is not

- Not a cache. Nothing is retained between processes.
- Not canonical state. It is never serialized under `.eggplan`; a test walks
  the whole state root before and after a snapshot and asserts byte-identical
  contents.
- Not a daemon, cache service, database, watcher, or network service.
- Not an authority. It contains raw validated facts, never an assessment or a
  trust decision.

## Consistency model

1. Acquire the **shared** repository lock, using the same `.lock` root writers
   use and the store's configured timeout.
2. Capture the Git subject as **S1**, after the lock is established.
3. Enumerate Plan IDs deterministically (sorted).
4. Load and deep-validate each selected Plan exactly once.
5. Recapture the Git subject as **S2**, immediately before releasing the lock.
6. If `S1 != S2`, return `RepoError::SubjectDrift` and no snapshot.

Sanctioned Eggplan writers take the exclusive repository lock, so the shared
lock prevents ordinary mutations from interleaving with the read. External
worktree writers are outside the lock by design; S1/S2 is what detects that
source drift. This is the honest boundary: the snapshot is consistent against
Eggplan's own writers and *detects* rather than prevents external drift.

Two further cases fail closed rather than guessing:

| S1 | S2 | Result |
|---|---|---|
| available | available, equal | snapshot returned |
| available | available, different | `SubjectDrift` |
| available | unavailable | `InspectionSubject` |
| unavailable | available | `SubjectDrift` (the worktree appeared mid-scan) |
| unavailable | unavailable | snapshot with `subject: None`, `subject_error: Some(..)` |

The last row is why `status` can still report `current_subject_unavailable`
instead of failing outright. Every other row fails closed.

### Locking portability

`File::try_lock_shared` is an inherent `std` method stable at this crate's MSRV
(1.89), and it is what actually runs. It reports contention as a distinct
`std::fs::TryLockError::WouldBlock`, which the crate maps to its existing
`RepoError::LockTimeout` with the same bounded retry loop writers use. Exclusive
locking still goes through the `fs2` trait, which has no stable std equivalent.

## Single-pass loading

`load_snapshot_unlocked` decodes and validates the Plan, its observations, its
supersessions, its closure, and the `effective_observations` projection exactly
once, and returns all of them. `load_unlocked` is that same function with the
extra values dropped, so the two paths cannot drift into different validation.

The snapshot enumerates Plan IDs with a private `plan_ids` helper rather than
`PlanStore::list`. `PlanStore::list` deep-loads every Plan and discards the
result — which is why repository-wide `status` used to decode every Plan once
for enumeration and then decode the retained prefix a second time.

`get`, `list_observations`, `list_supersessions`, and `closure_record` keep their
current safety semantics and cost.

## Selection and memory

Every Plan is deep-validated exactly once regardless of the selection. The only
choice the caller makes is how much record data to retain:

| Selection | Validation | Retention | Used by |
|---|---|---|---|
| `Repository { retain: Some(n) }` | every Plan | first `n` plans' records | `status`, `check` |
| `Repository { retain: None }` | every Plan | all | `registry render` |
| `One(id)` | that Plan only | that Plan | single-Plan commands |

Validating everything is deliberate. `PlanStore::list` has always deep-validated
every Plan directory, so a selection that stopped validating the remainder would
weaken existing integrity verification rather than merely change cost.

`check` keeps its existing shape exactly: it counts observations, supersessions,
and closures over *all* plans while projecting the first 100. The snapshot
provides both — `observations_counted()` and friends cover every plan, while
`plans()` holds at most `retain`. `registry render` retains everything, which is
its pre-existing behavior and is not made unbounded by this change; it was
already unbounded before.

## Command adoption

| Command | Path |
|---|---|
| `status` (repository-wide) | one snapshot, one subject, assessment from loaded facts |
| `status <PLAN_ID>` | one-plan snapshot |
| `registry render` | one snapshot retaining every plan |
| `check` | explicit recovery first when requested, then one snapshot |

`check --recover-pending` remains an explicit mutation: recovery runs through
the existing mutating store path, and only then is a read snapshot opened.

`show`, `ready`, `graph`, `markdown render`, `evidence`, `assess`, and `close`
keep their existing single-plan paths. They may adopt the snapshot later without
changing these semantics.

## Assessment and provider policy

The snapshot carries no authority decision. Assessment still receives a
host-supplied `ProviderRegistry`, and no policy file is implied by a read
command. One captured subject and one loaded record set mean the same policy can
be applied consistently to every selected Plan without re-reading state.

## Error semantics

New typed errors, both surfaced as stable CLI reason codes in the existing
schema-v1 failure envelope:

| `RepoError` | CLI code |
|---|---|
| `SubjectDrift` | `subject_drifted_during_inspection` |
| `InspectionSubject(_)` | `subject_unavailable` |

`RepoError::LockTimeout` already existed and maps to `lock_timeout`.
`RecoveryRequired` maps to `recovery_required`, and `Corrupt`/`InvalidPlan`/
`UnsafePath` keep their existing codes. No stable existing code was replaced.

## No public injection seam

There is no public way to supply a subject authority to `inspection_snapshot`,
and none was added. Three `compile_fail` doctests in `lib.rs` pin this:

- `SnapshotCounters` is not reachable from outside the crate;
- `SubjectCapture` cannot be passed to the snapshot;
- `InspectionSnapshot` is not serializable.

The algorithmic counters are crate-private instrumentation used by the in-crate
complexity regressions. They never influence a status, observation, or
projection.

## Algorithmic qualification

`crates/eggplan-repo/src/snapshot.rs` contains in-crate tests asserting, as
deterministic CI gates:

- exactly two subject captures for 1, 10, and 100 Plan repositories;
- each canonical Plan, observation, supersession, and closure file decoded
  exactly once per snapshot;
- no subject capture during per-Plan assessment or projection;
- deterministic Plan selection ordering across repeated snapshots;
- retention bounds and full-validation independence;
- subject drift and mid-scan subject appearance both failing closed;
- concurrent snapshots all served; a sanctioned writer refused while a read
  lock is held; lock timeout bounded and service restored after release;
- closed-plan deep validation still detecting corrupt closure and evidence;
- pending closures reported and never silently recovered;
- nothing written under the state root.

## Performance characterization

Two harnesses, deliberately separated:

- `characterization_counter_matrix_runs_in_ci` runs in CI and asserts only
  deterministic counts across the clean/dirty and observation-density axes.
- `characterization_snapshot_matrix` and
  `characterization_dirty_subject_dominates_and_does_not_scale_with_plans` are
  `#[ignore]`d full before/after runs. They replay the pre-M003a read shape over
  the same fixture and print wall time, captures, and decode counts:

```sh
cargo test -p eggplan-repo --lib characterization_snapshot_matrix -- --ignored --nocapture
```

Wall-clock numbers are recorded, never asserted. Shared-runner noise has
already produced false regressions in sibling qualification work, so the gates
are the counts and the times are evidence.

## Boundary

`eggplan-projection` stays pure and repository-free. Snapshot values are passed
to projection functions as already-canonical facts; projection opens no
repository, takes no lock, captures no Git state, and caches nothing.

`scripts/check-projection-cli-boundary.sh` continues to pass unchanged: the
snapshot lives in `eggplan-repo`, which already owns filesystem access and
subject capture.