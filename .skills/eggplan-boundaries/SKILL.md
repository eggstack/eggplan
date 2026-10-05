---
name: eggplan-boundaries
description:
  Keep new eggplan code inside its crate boundary. Use before adding a dependency, a process
  or network call, or a cross-crate reference — and when a boundary guard fails. Triggers on
  "add a dependency", "call the network", "which crate should this go in", or a
  check-*-boundary.sh failure.
---

# Crate boundaries

## Capability matrix

| Crate | May | Must never |
|---|---|---|
| `eggplan-core` | Typed IDs, Plan/evidence schema, bounds, canonical JSON + digests, readiness graph, assessment, closure shapes | Touch fs, Git, process, net, or a scheduler |
| `eggplan-repo` | `.eggplan/` store, revision CAS, append-only evidence + supersession ledger, Git subject capture, guarded closure finalizer | Let ordinary CAS enter `Closed`, or accept a caller-supplied subject |
| `eggplan-integrations` | Provider-normalization SPI + Eggwork/Eggsearch adapters | Acquire evidence, or enroll trust |
| `eggplan-projection` | Reusable derived summaries (registry/status views) | Depend on a CLI, terminal, process, or transport |
| `eggplan-cli` | The thin `eggplan` binary: command surface, JSON envelope, provider policy, read-only `check` | Execute work, accept user-authored evidence, or edit `plans/registry.md` |
| `eggplan-markdown` | Deterministic Markdown v1 render; CodeGG-subset import as Draft intent + loss report | Become canonical state, or import evidence/trust/subject/closure |
| `eggplan-codegg-compat` | Pure two-direction CodeGG bridge (snapshot→assessment, Plan→mirror contract) | Depend on `eggplan-repo` in production, or own CodeGG runtime/storage |

`eggplan-codegg-compat` depends on `eggplan-core`, `serde`, and `serde_json` **only**. It
may use `eggplan-repo` as a `[dev-dependencies]` entry for tests; the guard is
`[dependencies]`-scoped and that asymmetry is deliberate.

## Before you add anything

1. Pure deterministic logic with no I/O belongs in `eggplan-core`.
2. Persistence and Git belong in `eggplan-repo`.
3. A new provider adapter goes in `eggplan-integrations` and **normalizes only**. The host
   acquires evidence; the adapter never does.
4. Trust is host-conferred. Never call `register_trusted` or construct a `ProviderRegistry`
   inside an adapter — the CLI builds it from an explicit per-invocation policy file, and
   there is no default trusted provider.
5. The CLI never executes, never accepts user-authored passing evidence, and never edits
   `plans/registry.md`.

## Watch for

- **`tokio`, `reqwest`, `std::process::Command`** in core, projection, markdown, or the CLI
  is a violation, not a style question.
- **`#[doc(hidden)]` is not an access-control boundary.** It does not satisfy
  `check-closure-authority-boundary.sh`. Use `pub(crate)`.
- `markdown import` creates **one Draft intent** and nothing else — never evidence,
  provider trust, a `SubjectRevision`, or a closure record.
- The only arbitrary-path write in the CLI is `markdown render --output`; keep it that way.

## If a guard fails

The guard message states the file and the scope it actually scanned. Some guards are
section-scoped by design (the manifest guards read `[dependencies]`, not
`[dev-dependencies]`). Read the message before restructuring code — the fix is often
narrower than it looks, and the guard is encoding a real ADR decision, not a lint preference.

If a guard cannot run because `rg` is missing, it exits **green without scanning**. Install
ripgrep; never report a guard as passing on that basis.
