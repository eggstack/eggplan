# Evidence/Closure M003 — Repair or Quarantine for Desynchronized Plans

Status: proposed

Repository baseline: ee54a0b8e2cb0d843565c54930ad2c21111320db

Source roadmap:

- plans/subsystems/evidence-closure-roadmap.md

Long-term requirements:

- plans/000-long-term-specification.md (a canonical Closed Plan must have a
  valid guarded ClosureRecord)

Related ADRs: ADR-0003, ADR-0004.

Predecessor plan/closure:

- plans/implementation/evidence-closure/002-c004-closed-plan-cas-immutability.md
- plans/closure/evidence-closure/002-c004-closed.md

Primary class: capability / invariant / recovery

## 1. Objective

Give a repository whose plan and closure records have desynchronized a
supported way out that does not require hand-editing canonical state.

`plans/closure/evidence-closure/002-c004-closed.md` records, as an open
finding by design, that "already-poisoned state roots have no repair path".
The C004 plan excluded a repair explicitly
(`plans/implementation/evidence-closure/002-c004-closed-plan-cas-immutability.md:140-141`:
"exposing any new public API, including a repair or migration entry point;
adding a store-repair command for already-poisoned repositories"), and the
Evidence/closure roadmap requires that the gap "needs its own plan before it
can be attempted"
(`plans/subsystems/evidence-closure-roadmap.md:207-208`).

This is that plan. It is registered **proposed**, not ready, because the
semantic question it exists to answer has not been decided.

## 2. Findings

### E-EV-C003-01 — prevention is closed, remediation does not exist

The C004 guard works: `crates/eggplan-repo/src/store.rs:1010-1012` refuses
any ordinary mutation of a `Closed` plan. No known path can produce a
desynchronized repository today.

But every detection site for a desynchronized repository is terminal. All of
them surface as `RepoError::Corrupt`, which the CLI maps to the
`corrupt_state` code (`crates/eggplan-cli/src/lib.rs:2308`):

| Desynchronization | Detection site |
|---|---|
| plan bytes differ from the stored digest | `crates/eggplan-repo/src/store.rs:767-772` |
| plan ID differs from the directory ID | `crates/eggplan-repo/src/store.rs:773-778` |
| closure record unparseable | `crates/eggplan-repo/src/store.rs:794-798` |
| closure record fails validation against the plan | `crates/eggplan-repo/src/store.rs:799-801` |
| closure assessment not reproducible | `crates/eggplan-repo/src/store.rs:876-879` |
| closed plan with no valid closure record | `crates/eggplan-repo/src/store.rs:883-888` |
| closure record for a non-closed plan | `crates/eggplan-repo/src/store.rs:889-894` |

What exists today:

- `eggplan check --recover-pending` only acts on `RepoError::RecoveryRequired`,
  which is raised only when `closure.pending.json` exists
  (`crates/eggplan-repo/src/store.rs:784-786`). A desynchronized repository
  has no pending file, so the flag does nothing for it.
- `recover_pending_closures`
  (`crates/eggplan-repo/src/store.rs:273-368`) skips any plan directory
  without a pending file and never compares `closure.json` against
  `plan.json`.
- `atomic_write` (`crates/eggplan-repo/src/store.rs:1496-1522`) is a private
  durability helper, not an entry point.
- The CLI has no `repair`, `migrate`, `restore`, `quarantine`, or `doctor`
  command. `crates/eggplan-cli/src/commands.rs` holds the complete command
  inventory.

The practical consequence is that a desynchronized repository is a **durable
availability failure**: the whole state root stops opening, and the only
remedy is restoring bytes from a backup taken before the desynchronization.

### E-EV-C003-02 — the semantics of a repair are undecided

There is no obvious correct answer, and the options are not equivalent:

1. **delete** the plan directory — discards the plan's history, and for a
   `Closed` plan discards the closure record too;
2. **demote** the plan to a non-closed status — contradicts the stored
   closure record's claim that the plan was closed;
3. **quarantine** the plan directory — moves it aside, preserving the evidence
   while unblocking the repository;
4. **re-derive** the closure from the assessment ledger — reconstructs a
   closure record, which is exactly the authority the guarded finalizer is
   supposed to hold alone.

Options 1 and 2 destroy or contradict evidence. Option 3 preserves it and
does not grant authority. Option 4 risks forking closure authority, which
`plans/registry.md:139-146` and the closure-authority boundary both treat as
non-negotiable.

## 3. Controlling semantics

- The C004 invariant survives: ordinary mutation of a `Closed` plan stays
  refused. A repair path must be a **separate, explicitly authorized
  surface**, not a relaxation of the compare-and-swap guard.
- A repair must not grant closure authority. Only the guarded finalizer may
  write a `ClosureRecord`.
- A repair must not destroy evidence. Quarantining preserves it; deleting
  does not.
- A repair must be explicit. There is no automatic repair on open, because
  automatic repair would silently choose one of the semantics above.
- A repair must be idempotent and must report exactly what it did.

## 4. Scope

To be decided by this plan. At minimum it would cover:

- a new `eggplan-repo` public API, scoped to repair;
- a new CLI command, or an explicit flag on an existing one;
- the semantic choice among options 1-4 in §2 above;
- the closure-authority boundary's treatment of the new surface.

## 5. Required production changes

None yet. This plan is **proposed** and must not be implemented until the
semantic question in §2 is decided and recorded.

## 6. Failure, cancellation, restart, and contention semantics

To be decided with the semantic choice. Whatever is chosen must be
interruptible without leaving the repository in a worse state than it was
found in.

## 7. Required tests

To be decided. A repair surface would need, at minimum, a test that a
desynchronized repository can be repaired, that the repair is refused when
the requested semantics do not apply, and that the repair does not grant
closure authority.

## 8. Verification

Not applicable until the plan is ready.

## 9. Acceptance criteria

Not applicable until the plan is ready.

## 10. Stop conditions

- The semantic choice has not been made and recorded.
- The implementation would relax the C004 `ClosedPlanImmutable` invariant.
- The implementation would allow a repair to write a `ClosureRecord`.

## 11. Closure evidence required

Not applicable until the plan is ready.

## 12. Handoff notes

- This plan is **proposed**, not ready. It is registered because the Evidence/
  closure roadmap requires the gap to have its own plan before it can be
  attempted, and because the gap is recorded as an open finding.
- **This is a capability, not a corrective.** The repository's rule is that
  unbuilt capability gets a milestone plan. The C004 corrective is closed; its
  invariant holds; what is missing is a surface that does not exist yet.
- The decision this plan needs is a product decision: what should a
  desynchronized record become? It belongs to the Evidence/closure roadmap
  and the closure-authority boundary, not to an implementation agent.
- Registering this plan does not close the finding in
  `plans/closure/evidence-closure/002-c004-closed.md`. That record is history
  and its finding stands until a milestone closes it.
