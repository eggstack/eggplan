---
name: eggplan-verify
description:
  Run the eggplan verification suite in the required order and triage failures. Use before
  claiming any change is complete, when a boundary guard fails, or when asked what "verified"
  means in this repo. Triggers on "verify", "run the checks", "run the guards", "is CI green",
  "did it pass", or before any commit.
---

# Eggplan verification

Run the full suite in this exact order. The order is the contract: `fmt` before `check`,
`check` before `clippy`, `clippy` before `test`, guards last. `README.md` and
`architecture/overview.md` publish the same block; keep all three identical when editing.

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
bash scripts/check-core-boundary.sh
bash scripts/check-codegg-compat-boundary.sh
bash scripts/check-integrations-boundary.sh
bash scripts/check-projection-cli-boundary.sh
bash scripts/check-closure-authority-boundary.sh
```

## What the boundary guards actually enforce

Each guard is a static text scan, CI-gated, and **skipped on Windows runners**. They are
cheap, so run them locally before pushing rather than discovering a CI failure.

| Guard | Enforces |
|---|---|
| `check-core-boundary.sh` | `eggplan-core` stays pure — no fs, Git, process, or net |
| `check-integrations-boundary.sh` | `eggplan-integrations` is a pure normalization SPI; adapters never enroll trust |
| `check-projection-cli-boundary.sh` | `eggplan-projection` / `eggplan-cli` gain no executor, process, or transport |
| `check-codegg-compat-boundary.sh` | 5 named guards: no production `eggplan-repo` dep, no CodeGG dep, no client crate, no owned CodeGG identity, no impure source (fs/path/process/net/db) |
| `check-closure-authority-boundary.sh` | `SubjectCapture` and `finalize_closure_with_*` never become public |

## Prerequisites that silently no-op

The guards shell out to `rg`, `awk`, and `python3`. Every guard invokes `rg` **inside an
`if`**, so a missing `rg` makes the command fail inside a conditional and the guard then
**reports success without scanning anything**. If a guard passes suspiciously fast or
instantly, check `command -v rg` before trusting it. CI installs ripgrep explicitly for
this reason; do not remove those steps.

## Triage

- A guard failure names the file and scan scope in its own message. Read the message before
  guessing — several guards are deliberately section-scoped (e.g. the manifest guards read
  `[dependencies]` but not `[dev-dependencies]`, so a legal `eggplan-repo` dev-dependency
  passes).
- A clippy failure is never acceptable to silence with `#[allow]` unless the lint is
  genuinely inapplicable; CI runs `-D warnings`.
- `cargo test` alone is not proof of a boundary claim. Boundary claims need the guard plus
  the test.
- `git_subject_digest_golden.rs` pins frozen Git-subject digest bytes. If a subject-related
  test fails there, that is a frozen-identity break, not a flaky test.

## Recording evidence

Record what actually ran. Failed, blocked, skipped, unavailable, and unrun checks are all
legitimate closure evidence as long as they are stated as such. Never record a plan as
verified when a step was skipped.
