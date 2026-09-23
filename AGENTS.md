# Agent instructions

Before implementing substantial work, read `plans/README.md`, `plans/registry.md`, the applicable subsystem roadmap, and the registered implementation plan.

The canonical planning hierarchy is:

1. `plans/000-long-term-specification.md`
2. `plans/001-terminology-and-domain-model.md`
3. accepted ADRs under `plans/adrs/`
4. `plans/002-long-term-roadmap.md`
5. subsystem roadmaps under `plans/subsystems/`
6. implementation plans under `plans/implementation/`
7. closure evidence under `plans/closure/`

Do not treat a plan as evidence that a capability exists. Do not mark a milestone closed without the closure evidence required by its implementation plan. Record failed, blocked, skipped, unavailable, and unrun verification truthfully.

Eggplan is a planning/evidence mechanism, not a scheduler, process executor, CI system, issue tracker, or model reasoning store. Preserve that boundary.

Rust MSRV target: 1.89 or newer unless an accepted architecture decision changes it. Prefer small library-first crates, explicit bounds, versioned machine-readable schemas, deterministic behavior, and thin adapters.
