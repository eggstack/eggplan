# eggplan user documentation

User-facing documentation for adopting and operating Eggplan. This directory answers
"how do I use this?" — it does not describe internal design decisions.

| Guide | Read it when |
|---|---|
| [Getting started](getting-started.md) | Installing, first run, creating your first plan |
| [CLI reference](cli-reference.md) | Every command, flag, and the JSON envelope |
| [Evidence and closure](evidence-and-closure.md) | How observations become proof, and how a plan actually closes |
| [Provider policy](provider-policy.md) | Writing the trust policy file that `assess` and `close` require |

## What Eggplan is

A repository-local planning and evidence mechanism. It models bounded plans, derives
dependency readiness, records observations through provider adapters, and assesses closure
from structured state.

It **does not** schedule or execute work, run verification commands, fetch evidence,
authenticate producers, or store model reasoning. Readiness is a derived statement about
dependencies — it is never authority to act.

## Where design detail lives

If you are changing Eggplan rather than using it, the design record is elsewhere:

- `architecture/` — normative component contracts and code-verified deep dives.
- `plans/` — the planning system: specifications, ADRs, roadmaps, and closure evidence.
- `AGENTS.md` — how an agent or maintainer works in this repository.
- `.skills/` — reusable procedures for working in this repository.

`plans/registry.md` is the only place current milestone status is recorded.
