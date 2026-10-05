---
name: eggplan-milestone
description:
  Add or advance a milestone in the eggplan planning system — registering a plan, writing
  closure evidence, and updating plans/registry.md. Use when asked to plan a change, start a
  milestone, close one out, or update planning status. Triggers on "add a milestone",
  "write a plan", "close this out", "update the registry", or before handing work to an
  implementation agent.
---

# Eggplan milestone lifecycle

## Authority order

Never invert this. Each level may only add constraints, never contradict the one above.

```
plans/000-long-term-specification.md
  → plans/001-terminology-and-domain-model.md
    → plans/adrs/ (accepted decisions)
      → plans/002-long-term-roadmap.md
        → plans/subsystems/<subsystem>-roadmap.md
          → plans/implementation/<subsystem>/NNN-short-title.md
            → plans/closure/<subsystem>/NNN-<status>.md
              → plans/registry.md (current status only)
```

## 1. Register before handoff

A plan is not registered until it appears in `plans/registry.md` under **Registered
implementation plans**. Registering before implementation begins is what makes the handoff
to an implementation agent meaningful. Filename conventions are enforced in
`plans/README.md`: ADR `adrs/ADR-NNNN-short-title.md`, roadmap
`subsystems/<subsystem>-roadmap.md`, plan `implementation/<subsystem>/NNN-short-title.md`,
closure `closure/<subsystem>/NNN-<status>.md`.

## 2. Keep the plan header honest

The `Status:` line in a plan file must match `registry.md`. Header drift is the most
common defect in this repo — three plans sat at `Status: ready` long after their closure
records said `closed`. When you close a milestone, update **both** in the same change.

Status vocabulary is defined in `registry.md` and must be used exactly:
`proposed`, `ready`, `active`, `blocked`, `closing`, `closed`, `conditionally closed`,
`corrective required`, `superseded`, `archived`, `deferred`.

## 3. Closing requires evidence, not a green build

A milestone is **not** closed because code exists, compiles, or an agent says it is done.
Closure needs the evidence its source plan names. Per `plans/closure/README.md`, a
milestone must not be marked closed when required verification was unrun and no justified
substitute exists, or when only compilation/formatting was checked for a stronger
correctness claim.

A closure record carries a requirement-to-evidence matrix, roadmap disposition, registry
updates, and residual risk.

## 4. Corrective plans, never silent rewrites

When a later review finds a defect in already-closed work, raise a **corrective plan** with
its own closure record. Do not rewrite the original closure record's conclusions.

`plans/closure/README.md`: "Historical closure remains evidence of what was accepted at the
time." When a refactor makes a historical record's `file:line` citations drift, append a
**factual errata** note mapping old lines to current ones, and state that no conclusion
changes. Do not silently edit the citations.

## 5. Cross-repository paths must be qualified

CodeGG, Eggwork, Eggsearch, and Eggbench are separate repositories. A path from one of
those is not a path in this repo. Qualify it — `dbowm91/codegg:plans/...` or
`` `dbowm91/codegg` `path` `` — and follow whatever convention that file already uses.
An unqualified sibling path is a broken reference that reads as a local one.

## Hygiene

- Mark non-blocking maintenance explicitly so it does not serialize independent handoffs.
- Infrastructure is not user-visible capability until its acceptance criteria are met.
- Never copy Eggwork/Eggsearch/Eggbench responsibilities into `eggplan-core`.
- A plan document is never evidence that a capability exists.
- Current milestone status lives in `registry.md`, never in `architecture/`.
