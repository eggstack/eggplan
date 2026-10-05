---
name: eggplan-closure-authority
description:
  Preserve the frozen closure and Git-subject identity contracts in eggplan. Use when
  touching RepositoryStore::finalize_closure, Git subject capture, or anything that reads or
  writes closure records. Triggers on "finalize_closure", "subject", "closure record",
  "SubjectCapture", or a closure/dirty-digest test failure.
---

# Closure and subject authority

These contracts are **frozen**. Changing them is not a refactor — it is a break that
invalidates evidence in every downstream host.

## The finalizer owns the subject

`RepositoryStore::finalize_closure` takes **no caller-supplied current subject**. A caller
may build a `ClosureCandidate` and propose a subject, but the finalizer decides whether that
subject is still current at commit time.

Internally it captures the Git subject **twice** under its own lock:

1. `s1` at the start — if it differs from the candidate, abort with `ClosureSubjectStale`.
2. `s2` immediately before the first canonical closure write — if it differs from `s1` or
   from the candidate, abort with `ClosureSubjectDrift`.

Mismatches are typed `RepoError` variants, distinct from invalid-update, lifecycle, and
corrupt errors. Neither abort leaves pending or final closure state behind.

## Never expose the capture seam

`SubjectCapture`, `GitSubjectCapture`, and `ScriptedSubjectCapture` are `pub(crate)`.
`finalize_closure_with_capture` is `pub(crate)`. These are covered by
`scripts/check-closure-authority-boundary.sh` and by `compile_fail` doctests, so making any
of them public fails the build and CI.

Test seams are not authority seams. A deterministic stale/drift/capture-failure regression
belongs as a crate-internal test, not as a public injection point.

## Ordinary CAS can never reach Closed

`plan_transition_allowed` keeps `Active → Closed` legal **only** because the guarded
finalizer performs that transition. Separately, an unconditional guard refuses **any** CAS
against an already-`Closed` Plan before `atomic_write`, with `RepoError::ClosedPlanImmutable`.
The two rejections stay distinct so a caller can tell an illegal entry from an illegal
rewrite. Removing the unconditional guard reintroduces a durability defect: a rewrite gets
committed at `revision + 1` before re-read validation fails, leaving the plan and the whole
state root unopenable with no recovery path.

## Git subject identity is frozen

`capture_git_subject_fingerprint` (repository-ID-free, `SCHEMA_VERSION = 1`) returns exactly
the revision/state/dirty digest `GitSubjectSource::capture` returns and shares one capture
implementation with it. Its digest bytes are pinned by
`crates/eggplan-repo/tests/git_subject_digest_golden.rs`.

Never: add a domain separator, change row ordering / status bits / index-entry treatment,
widen default bounds, or expose repository IDs, paths, contents, or manifest bytes through
that API. A host that derives different bytes fails loudly, which is the intended design.

The fingerprint fails **closed** on an exclusion that does not resolve inside the worktree
(`GitSubjectError::InvalidExclusion`); `GitSubjectSource` keeps its historical lenient
resolution. That difference is intentional — do not "fix" it.

## Subject equality is applicability

A passing observation for any other source tree is stale history, never current proof.
Assessment requires exact `SubjectRevision` equality. Stored policy is historical evidence
and does not auto-trust providers in later assessments.

A `Closed` Plan without a matching `ClosureRecord` is **corruption**, not a stale state.
Already-poisoned state roots still have no repair path — raising that as new scope, not as
an incidental fix.
