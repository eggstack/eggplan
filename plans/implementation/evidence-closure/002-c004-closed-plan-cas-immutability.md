# Evidence and Closure M002 C004 — Closed-Plan CAS Immutability

Status: closed (see plans/closure/evidence-closure/002-c004-closed.md)

Repository baseline: 60a5f9212d3e6feab8acb45fb6acbfdbd56264aa

Source roadmap:

- plans/subsystems/evidence-closure-roadmap.md

Predecessor plan/closure:

- plans/implementation/evidence-closure/002-closure-records-integrity-and-recovery.md
- plans/closure/evidence-closure/002-closed.md
- plans/closure/evidence-closure/002-c001-closed.md
- plans/closure/evidence-closure/002-c002-closed.md
- plans/closure/evidence-closure/002-c003-closed.md

Primary class: correctness / persistence / recovery corrective

## 1. Objective

Close a durable availability defect in guarded-closure authority. Ordinary
repository compare-and-swap can rewrite a Plan whose status is already
`Closed`, which desynchronizes it from its `ClosureRecord` and leaves the whole
state root permanently unreadable.

This pass restores the invariant that a canonical `Closed` Plan is immutable
under ordinary CAS, and that only `RepositoryStore::finalize_closure` may
produce a `Closed` Plan. It does not reopen C001 subject revalidation, C002
authority containment, or C003 guard hygiene, and it preserves all historical
closure evidence.

This is a production correctness defect, not hygiene. It is registered ahead of
further capability work in this subsystem.

## 2. Finding

### E-M002-C004-01 — ordinary CAS accepts a Closed→Closed rewrite, poisoning the state root

`RepositoryStore::compare_and_swap` in
`crates/eggplan-repo/src/store.rs:928-937` applies two independent guards:

    if next.status == PlanStatus::Closed && current.status != PlanStatus::Closed {
        return Err(RepoError::GuardedClosureRequired);
    }
    if next.status != current.status
        && !eggplan_core::plan_transition_allowed(&current.status, &next.status)
    {
        return Err(RepoError::InvalidTransition);
    }

For a `Closed` → `Closed` update both guards are bypassed:

- The first guard requires `current.status != Closed`, so it does not fire.
- The second guard requires `next.status != current.status`, so
  `plan_transition_allowed` is never consulted. It would return `false`
  regardless: `plan_transition_allowed` (`crates/eggplan-core/src/model.rs:391-403`)
  has no `Closed` row in its `matches!` arm, making `Closed` terminal.
- `Plan::validate` (`crates/eggplan-core/src/model.rs:288-347`) constrains only
  structure, identifiers, and bounds. It never inspects plan status, so nothing
  constrains the content of a `Closed` rewrite.
- The per-item guard (`store.rs:938-948`) is likewise bypassed for items that
  stay `Completed`, for the same short-circuit reason.

`atomic_write` then commits the new bytes at `revision + 1`
(`store.rs:955`). The trailing re-read (`store.rs:956`) loads the plan through
`load_unlocked`, which for a `Closed` plan with a `closure.json` calls
`ClosureRecord::validate(&plan)` (`store.rs:740-755`). That validation pins both
the target revision and the plan content digest, so it now fails and returns
`RepoError::Corrupt`.

The failure is detected only *after* the bad bytes are durable. The result:

- the stored `Closed` Plan at `revision + 1` no longer matches its
  `ClosureRecord`, which is the exact corruption state
  `architecture/evidence.md:85-86` already declares to be corruption;
- `RepositoryStore::open` calls `store.list()` (`store.rs:614`), which loads
  every plan, so the poisoned plan makes the entire state root fail to open;
- no recovery path exists, because recovery only handles pending closures
  (`store.rs:262-305`) and never repairs a mismatched `ClosureRecord`.

**Severity.** Durable loss of availability for the whole state root. It is
explicitly *not* a closure bypass and not an authority gain: no
`ClosureRecord` is forged, no evidence is fabricated, no trust is enrolled, and
the guarded finalizer still owns the only legitimate path to `Closed`. The
attacker-visible effect is a self-inflicted denial of service on a repository
the caller already has write access to. Severity: high for durability, none for
evidence integrity.

**Detection gap.** `crates/eggplan-repo/tests/repository.rs:291-311` exercises
only the `Draft` → `Closed` rejection. No test covers `Closed` → `Closed`, which
is why the defect survived the C002/C003 authority work.

**Contributing decision.** The two guards encode different intents — "entering
`Closed` requires the finalizer" and "lifecycle transitions must be legal" — and
neither expresses "a `Closed` Plan is immutable". The missing invariant is
stated in `plans/registry.md` design gate 10 and in
`architecture/deep-dive-repository.md` finding 6.1.

## 3. Controlling semantics

The controlling invariant, from design gate 10 in `plans/registry.md:187-188`:
"A canonical Closed Plan must have a valid guarded ClosureRecord; ordinary CAS
must never be a bypass around closure assessment."

From `architecture/evidence.md:85-86`: "A Closed Plan without a matching record
is corruption." Combined, these require that a `Closed` Plan be *unreachable by
ordinary CAS in any direction*, not merely guarded on entry.

Required behavior after this pass:

- ordinary CAS against a Plan whose current status is `Closed` fails, for every
  proposed target status including `Closed` itself;
- the failure is a typed, machine-stable `RepoError`, not a late `Corrupt`;
- `RepositoryStore::finalize_closure` is unaffected and remains the only path
  that writes a `Closed` Plan;
- `plan_transition_allowed` keeps `Closed` terminal, and core gains a regression
  test pinning that no `(Closed, _)` or `(_, Closed)` pair is legal.

## 4. Scope

### In scope

- the `compare_and_swap` guard in `crates/eggplan-repo/src/store.rs`;
- a typed error for attempts to modify an already-`Closed` Plan;
- core regression coverage for `Closed` terminality in `plan_transition_allowed`;
- regression tests covering `Closed` → `Closed` and `Closed` → non-`Closed`;
- normative documentation updates in `architecture/repository.md`,
  `architecture/evidence.md`, and `architecture/deep-dive-repository.md` to
  state Closed-Plan CAS immutability and its consequence;
- registry and roadmap lineage updates.

### Out of scope

- changing C001 subject revalidation, the S1/S2 double-capture algorithm, or
  C002 authority containment;
- changing the `ClosureRecord` schema, digest rules, or pending-closure
  protocol;
- exposing any new public API, including a repair or migration entry point;
- adding a store-repair command for already-poisoned repositories;
- Projection/CLI, CodeGG, or Eggstack integration milestones;
- changing `item_transition_allowed` terminality, which is already correct.

## 5. Required production changes

Ownership is the repository. `eggplan-repo` is the authority for
compare-and-swap, so the invariant belongs there and not in
`eggplan-core`; core keeps only the regression test that pins existing
terminality.

Preferred shape:

- add a guard that rejects any `compare_and_swap` whose `current.status` is
  `Closed`, before the existing entry guard and before `atomic_write`;
- reuse `RepoError::GuardedClosureRequired` if its documented meaning already
  covers "this Plan may only change through the guarded finalizer"; otherwise add
  one precisely named variant. Either way the CLI maps it to a stable
  diagnostic code consistent with existing `closure_subject_*` naming, and the
  choice is recorded in the closure record;
- keep the existing `next.status == Closed && current.status != Closed` guard so
  the `Draft`/`Active`/`Blocked` → `Closed` rejection reason stays distinct.

Explicitly rejected alternative: adding a `Closed` row to
`plan_transition_allowed`. It cannot fix the defect on its own, because the
`next.status != current.status` short-circuit prevents that function from being
called at all for `Closed` → `Closed`, and it would weaken core's terminal-state
matrix rather than fix the repository's authority rule.

## 6. Failure, restart, and contention semantics

- The guard must run before any write, so a rejected update leaves repository
  state byte-identical.
- The defect's current behavior — write, then fail with `Corrupt` — must not be
  reachable again. `Corrupt` must never be the first signal of an attempted
  illegal transition.
- No new locking is introduced. The existing store lock and the pending-closure
  protocol are unchanged; this pass adds a precondition check inside the
  already-locked section.
- Restart behavior is unchanged: a repository that was never poisoned opens
  normally. This pass deliberately adds no migration for already-poisoned
  state; see stop conditions.

## 7. Required tests

At minimum:

- `Closed` → `Closed` ordinary CAS is rejected with the typed error, and the
  stored plan bytes are unchanged afterward;
- `Closed` → `Active` (and every other target status) is rejected the same way;
- `Draft`/`Active`/`Blocked` → `Closed` still returns
  `RepoError::GuardedClosureRequired`, preserving the existing regression;
- `finalize_closure` still succeeds on an eligible plan and still produces a
  `Closed` Plan plus a matching `ClosureRecord`;
- a full reopen after a rejected CAS succeeds, proving no poisoning occurred;
- `plan_transition_allowed` has no legal pair with `Closed` in either position;
- all existing closure, supersession, and recovery regressions remain green;
- the six `compile_fail` doctests still pass.

## 8. Verification

Record actual results for:

    cargo fmt --all -- --check
    cargo check --workspace --all-targets --locked
    cargo clippy --workspace --all-targets --locked -- -D warnings
    cargo test --workspace --locked
    cargo test -p eggplan-repo --locked
    cargo test --workspace --doc --locked
    cargo +1.89.0 check --workspace --all-targets --locked
    cargo +1.89.0 test --workspace --locked
    bash scripts/check-closure-authority-boundary.sh
    bash scripts/check-core-boundary.sh
    git diff --check

Hosted Linux/macOS/Windows and Rust 1.89 qualification must remain green. The
`cargo +1.89.0` rows are the MSRV gate and must be recorded truthfully, including
if the toolchain is unavailable in the implementing environment.

## 9. Acceptance criteria

C004 closes when:

1. ordinary CAS cannot modify a `Closed` Plan in any direction, including
   `Closed` → `Closed`;
2. the rejection is a typed `RepoError` raised before any write, and stored bytes
   are provably unchanged;
3. no reachable path can desynchronize a `Closed` Plan from its
   `ClosureRecord`;
4. `RepositoryStore::finalize_closure` remains the only writer of a `Closed`
   Plan, and its behavior is unchanged;
5. `plan_transition_allowed` keeps `Closed` terminal, pinned by a core test;
6. the CLI maps the new rejection to a stable diagnostic code;
7. `architecture/repository.md` and `architecture/evidence.md` state
   Closed-Plan CAS immutability and the corruption consequence;
8. no public API or persisted schema changes;
9. native and MSRV qualification pass, with all five boundary guards green.

## 10. Stop conditions

Stop and report rather than improvise if:

- a fix would require exposing a new public API or a subject-capture injection
  seam (C002 containment must not regress);
- a fix would require changing the `ClosureRecord` schema or digest rules;
- the guard cannot be added before `atomic_write` without restructuring the
  locking discipline;
- any already-poisoned repository is found in the wild, because recovery or
  migration is explicitly out of scope and needs its own decision;
- required verification cannot be obtained honestly.

## 11. Closure evidence required

Per `plans/003-planning-process.md` §7, a closure record must distinguish
planned commands from commands actually run, and must cover:

- implementation commit(s) and PR reference;
- the requirement-to-evidence matrix for every acceptance criterion in §9;
- the regression test that would have detected E-M002-C004-01 before the fix;
- exact verification output, including the MSRV rows;
- hosted native/MSRV run and job identifiers where available;
- an explicit statement that evidence integrity and closure authority were
  reviewed and that this defect did not affect either;
- unresolved findings with severity, and roadmap disposition;
- registry updates.

Do not convert a planned command into a passing result because it appears in
§8. If a row was not run, record it as not run or unavailable.

## 12. Handoff notes

- Do not absorb Projection/CLI, CodeGG, or Eggstack milestones into this pass.
- The defect is availability-scoped. Do not describe it as a closure bypass in
  any status update, plan, or closure record; §2 states the precise blast
  radius, and overstating it would misdirect the next reader.
- `architecture/deep-dive-repository.md` finding 6.1 already records this defect
  and must be updated to reference the closing corrective rather than left
  implying an open finding.
