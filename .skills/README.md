# Repository skills

Reusable procedures for working in this repository. Each skill is a focused, repeatable task
with the rules that are easy to get wrong, plus the architecture section that holds the
design rationale. Read the skill, then follow its link when you need the "why".

Skills are **execution procedures, not plans.** The planning system is `plans/`, and
`plans/registry.md` remains the only place current milestone status lives.

| Skill | Use when | Design authority |
|---|---|---|
| [eggplan-verify](eggplan-verify/SKILL.md) | Running the verification suite, or triaging a boundary-guard failure | [deep-dive-tooling-governance](../architecture/deep-dive-tooling-governance.md) |
| [eggplan-milestone](eggplan-milestone/SKILL.md) | Planning a change, closing a milestone, updating the registry | [plans/README.md](../plans/README.md), [closure rules](../plans/closure/README.md) |
| [eggplan-boundaries](eggplan-boundaries/SKILL.md) | Adding a dependency, I/O call, or cross-crate reference | [overview](../architecture/overview.md), [provider SPI](../architecture/provider-spi.md) |
| [eggplan-closure-authority](eggplan-closure-authority/SKILL.md) | Touching `finalize_closure`, Git subject capture, or closure records | [repository](../architecture/repository.md), [evidence](../architecture/evidence.md) |

## When to add a skill

Add one only when a procedure is **repeated across milestones** and the rules are easy to
get subtly wrong — a rule whose violation is silent (a guard that no-ops without ripgrep, a
schema version that drifted) is a good candidate. A one-off task belongs in its plan
document, not here.

Keep a skill short and checkable. If it cannot be verified against the code, it is a
liability: prefer pointing at the architecture deep dive instead of restating it.
