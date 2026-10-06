# Projection and CLI M003a — Repository Inspection Snapshot and Performance Qualification

Status: closed

Repository baseline: 71904570e908a15c099f9b2804cabfbccf9ae51a

Source roadmap:

- plans/subsystems/projection-cli-roadmap.md
- plans/002-long-term-roadmap.md Projection/CLI M003

Predecessors:

- plans/closure/projection-cli/001-closed.md
- plans/closure/projection-cli/002-closed.md

Primary class: performance / read-model architecture / correctness

## 1. Objective

Eliminate avoidable repeated repository and Git-subject work from
repository-wide CLI reads while preserving canonical authority and existing
machine output semantics.

Current code paths repeatedly perform expensive work:

- `status` calls `summary_context` per Plan and captures the Git subject per
  Plan;
- `registry render` captures the Git subject independently per Plan;
- `check` captures the Git subject independently per Plan;
- `RepositoryStore::get` deeply validates closed-plan/evidence/closure state,
  after which higher layers may list or assess the same material again.

Dirty subject capture may enumerate up to 10,000 paths and hash up to 64 MiB of
content. Repository-wide reads must not multiply that work by Plan count.

M003a introduces a bounded command-local repository inspection read model and
characterizes its complexity/performance. It is not a persistent cache and
does not become canonical state.

## 2. Non-goals

Do not add:

- daemon/background caching;
- SQLite/database indexing;
- async runtime;
- watcher;
- network service;
- global repository server;
- execution/evidence acquisition;
- a second canonical state format.

Do not weaken existing deep integrity verification merely to improve latency.

## 3. Read snapshot ownership

Add a repository-level read API, recommended shape
`RepositoryStore::inspection_snapshot(...)`, owned by `eggplan-repo`.

The repository layer owns filesystem access and subject capture.
`eggplan-projection` remains repository-free and receives already-loaded
canonical/domain facts.

The snapshot/read model should contain only what read commands need, for
example:

- repository ID;
- one captured current `SubjectRevision`;
- selected Plan values + revisions;
- validated observations;
- validated supersession records;
- optional closure records;
- integrity/recovery markers required by check;
- explicit counts/truncation where the caller requested bounded selection.

Do not serialize this snapshot under `.eggplan`. It is ephemeral process
state.

## 4. Cooperative consistency model

Repository state and Git worktree state have different authorities.

For one bounded repository-wide inspection:

1. acquire a cooperative shared/read lock using the same lock root used by
   writers;
2. capture Git subject S1 once after the lock is established;
3. enumerate/select Plan IDs deterministically;
4. load and validate each selected Plan's canonical state once;
5. derive all command projections/assessments from those loaded facts;
6. recapture Git subject S2 immediately before releasing the read lock;
7. if S1 != S2, fail/report `subject_drifted_during_inspection`.

Existing sanctioned writers use the exclusive repository lock, so the shared
lock prevents ordinary Eggplan mutations from interleaving with the read.
External worktree writers remain outside the lock; S1/S2 detects source drift.

If portable shared-lock semantics cannot be made reliable with the existing
fs2 contract on all supported platforms, stop and document a revision-fence
alternative rather than pretending the snapshot is atomic.

The read API must not expose a public injectable subject-authority seam.

## 5. Single-pass repository loading

Avoid calling public high-level read methods in a way that reopens the same
files repeatedly.

Refactor internal validation so one snapshot load can:

- decode/validate a Plan once;
- decode/validate its observations once;
- decode/validate supersessions once;
- decode/validate closure once;
- reproduce closure assessment when required;
- retain those validated values for subsequent CLI projection.

Public `get`, `list_observations`, and other existing APIs keep their
current safety semantics.

Do not share mutable references into internal store state.

## 6. CLI adoption

Migrate repository-wide read commands first:

- `status` with no single-plan target;
- `registry render`;
- `check` after any explicit pending-recovery operation.

Single-plan commands may use the same snapshot API when it simplifies code, but
must not regress latency or diagnostics.

`check --recover-pending` remains explicit mutation: perform recovery through
the existing mutating store path first, then open a read snapshot for the
reported state.

## 7. Assessment/provider-policy behavior

M003a must not introduce implicit provider trust.

The snapshot contains raw validated evidence, not an authority decision.
Assessment still receives a caller/host-supplied `ProviderRegistry`.

M003b may expose provider-policy flags on additional read commands. M003a
should shape the snapshot so one policy can be applied consistently to all
selected Plans without re-reading state.

## 8. Projection boundary

Keep `eggplan-projection` pure and reusable.

Projection functions consume canonical `Plan`, already captured subject,
optional already-computed assessment, closure presence/summary, and bounded
counts.

They must not open repositories, acquire locks, capture Git state, or cache.

Update `scripts/check-projection-cli-boundary.sh` if new modules create an
enforcement gap.

## 9. Algorithmic qualification

Add deterministic regressions proving the important complexity properties.

At minimum verify, with crate-private/test-only instrumentation:

- one Git subject capture at snapshot start and one at end, independent of Plan
  count;
- each selected Plan canonical file is decoded once per snapshot;
- each selected observation/supersession/closure is decoded at most once per
  snapshot;
- no extra subject capture occurs during per-Plan assessment/projection;
- selected Plan ordering is deterministic;
- output truncation remains bounded.

Instrumentation must be private/test-only. Do not expose a public alternate
subject-capture authority seam.

## 10. Performance characterization

Create a repeatable non-network benchmark/characterization harness using
temporary Git repositories.

Matrix at minimum:

- Plans: 1, 10, 100;
- observations per Plan: 0, 10, 100 where within existing repository bounds;
- Git subject: clean and dirty;
- dirty fixture includes enough content to make repeated subject hashing
  measurable;
- commands/read models: status, registry, check, and direct snapshot load.

Record wall time, subject captures, canonical files decoded, total selected
records, and peak retained projection/snapshot counts where practical.

Do not use shared-runner wall-clock thresholds as hard correctness gates.
Hosted performance noise has already proven capable of producing false
regressions in sibling qualification work.

Hard CI gates should assert algorithmic counts/bounds. Wall-time tables are
characterization evidence.

## 11. Memory and bounds

A snapshot must remain bounded by existing repository/domain maxima and
explicit command selection.

Do not load arbitrary unbounded artifact bodies.

For repository-wide commands that already cap projected Plans at 100, do not
silently make the in-memory projection unbounded merely because more Plan
directories exist. The snapshot may validate/count additional entries when
required by `check`, but must document memory behavior explicitly.

## 12. Error semantics

Add stable typed errors/reason codes for at least:

- subject drift during inspection;
- read-lock acquisition timeout/failure;
- corrupt selected Plan state;
- recovery required;
- unsafe path;
- repository changed in a way the chosen consistency mechanism cannot safely
  represent.

Existing JSON failure envelope remains schema v1 unless a separate additive
schema change is justified. Do not replace stable existing error codes without
compatibility handling.

## 13. Required tests

At minimum:

- clean 1/10/100 Plan snapshots;
- dirty repository snapshots;
- subject changes between S1/S2 fail closed;
- concurrent sanctioned writer cannot interleave with shared snapshot;
- lock timeout remains bounded;
- closed-plan deep validation still detects corrupt closure/evidence;
- abandoned/pending closure reporting unchanged;
- status/registry/check JSON golden output unchanged for equivalent logical
  state;
- no public subject-capture injection API appears;
- Linux/macOS/Windows lock/path behavior;
- Rust 1.89.

## 14. Documentation

Add `architecture/repository-inspection-snapshot.md` documenting why the read
model exists, lock/S1/S2 consistency, what it does and does not guarantee,
memory/bounds, the no-persistence/no-cache rule, and command adoption.

Add a performance characterization section/report under docs or closure
evidence with the exact fixture matrix and machine/environment metadata.

## 15. Ordered work packages

### WP1 — Baseline characterization and instrumentation

Measure current repeated subject/file work and add private algorithmic counters.

### WP2 — Repository inspection snapshot

Implement shared read locking, one bounded load, S1/S2 recapture, and typed
errors.

### WP3 — CLI migration

Move status/registry/check repository-wide reads to the snapshot and preserve
golden output.

### WP4 — Performance qualification and hardening

Run the matrix, add concurrency/drift regressions, verify bounds and platform
behavior.

### WP5 — Documentation and closure evidence

Document semantics and record hosted qualification plus before/after
characterization.

## 16. Required verification

    cargo fmt --all -- --check
    cargo check --workspace --all-targets --locked
    cargo clippy --workspace --all-targets --locked -- -D warnings
    cargo test --workspace --locked
    cargo +1.89.0 check --workspace --all-targets --locked
    cargo +1.89.0 test --workspace --locked
    bash scripts/check-core-boundary.sh
    bash scripts/check-projection-cli-boundary.sh
    bash scripts/check-closure-authority-boundary.sh
    git diff --check

Run the characterization harness locally and on at least one hosted Linux
runner as evidence, but do not make noisy wall-clock numbers closure gates.

## 17. Acceptance criteria

M003a closes when:

1. repository-wide CLI reads no longer capture the Git subject once per Plan;
2. selected canonical state is loaded once into an ephemeral validated read
   model;
3. sanctioned Eggplan writers cannot interleave with a claimed snapshot;
4. external source drift across the scan is detected by S1/S2;
5. existing integrity/closure checks remain at least as strict;
6. existing stable JSON projection meaning is unchanged;
7. no persistent cache/daemon/database is introduced;
8. algorithmic complexity regressions are deterministic in CI;
9. before/after performance characterization is recorded;
10. native/MSRV CI passes.

## 18. Stop conditions

Stop and write a corrective/design note if shared locking cannot provide the
stated portable cooperative semantics; avoiding duplicate reads would require
weakening deep validation; the design needs a persistent cache/background
daemon; projection would need filesystem authority; a public subject-injection
seam would be required; or wall-clock gating is the only available proof.

## 19. Closure evidence

Record the snapshot API/consistency contract; algorithmic counter results;
before/after characterization matrix; output golden diff/no-diff;
concurrency/drift tests; platform lock evidence; implementation SHA;
native/MSRV hosted workflow IDs; and residual findings/M003b dependency
disposition.
