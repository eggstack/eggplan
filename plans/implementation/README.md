# Eggplan Milestone Implementation Plans

Implementation plans are bounded handoffs tied to a concrete repository
baseline. They may evolve as code changes, but they cannot silently weaken
canonical invariants or accepted ADRs.

## Layout

    implementation/<subsystem>/NNN-short-title.md

## Required sections

Each plan contains:

1. Status and repository baseline.
2. Source roadmap, long-term requirements, and ADRs.
3. Primary class: invariant, capability, infrastructure, or polish.
4. Objective.
5. Why the milestone is ready.
6. Current implementation evidence.
7. Invariants that must not regress.
8. In scope and explicitly out of scope.
9. Required production changes by ownership area.
10. Ordered work packages with acceptance evidence.
11. Failure, cancellation, restart, and contention semantics.
12. Compatibility and migration.
13. Required tests.
14. Exact verification commands.
15. Documentation updates.
16. Acceptance criteria.
17. Stop conditions.
18. Closure evidence required.
19. Handoff notes.

Before handoff, confirm the baseline and external interfaces are current and
register the plan in plans/registry.md.
