# Foundation M003 C005 — Closed

Status: closed

Source plan: plans/implementation/foundation-core/003-c005-core-boundary-guard-and-tooling-governance-reconciliation.md

Source roadmap: plans/subsystems/foundation-core-roadmap.md

Reviewed baseline: `37ad290ca6b971a68d2871aedb39f4882f4eb2a8`.

Implementation commit: `1e33069271f8a802190078e640f3db942b15ce7a`
("fix: resolve the Foundation M003 corrective lines from the 2026-10-23
repository audit").

Hosted CI was not run for this change. The job identifiers are to be added to
this record as a factual amendment once CI has run, following the precedent of
`plans/closure/evidence-closure/002-c004-closed.md`.

## Executive finding

Foundation M003 C005 closes. The core boundary guard no longer reports a pass
it did not earn.

At the baseline, `scripts/check-core-boundary.sh` was 17 lines and ran two
`rg` invocations inside `if` statements. With `rg` absent, each invocation
exited 127 inside a conditional, the conditional took its else branch, and the
script printed `eggplan-core static boundary check passed` and exited 0 —
having scanned nothing. The guard also never looked at core's production
source for impure access, so a synthetic core source containing
`std::fs::read_to_string` passed it. Both were reproduced at the baseline; the
core crate itself was clean throughout.

The hardened guard refuses to report a pass without its tools, scans core's
source tree for filesystem, process, network, path, environment, database, and
Git access, carries deterministic synthetic self-proofs for every guard it
enforces, and states its guard count, mechanism, and scan scope in its header.
`architecture/deep-dive-tooling-governance.md`, which misdescribed the
integrations guard's mechanism, guard count, scan scope, and precondition and
claimed the integrations guard was unaffected by the ripgrep trap, now
describes it accurately.

## Before and after

### The tool precondition

BEFORE — the reproduction run at the baseline:

```
rg: command not found
rg: command not found
eggplan-core static boundary check passed
exit=0
```

AFTER — the same reproduction run against the hardened guard:

```
required boundary-guard tool 'rg' is not installed; refusing to report a pass
exit=1
```

The sibling guards already refused this condition; the core guard now does
too.

### Source purity

BEFORE — a synthetic tree whose `crates/eggplan-core/src/lib.rs` contained
`std::fs::read_to_string("/etc/passwd").unwrap()` and whose manifest listed
only `serde`:

```
eggplan-core static boundary check passed
exit=0
```

AFTER — the same synthetic tree:

```
crates/eggplan-core/src/lib.rs:2:    std::fs::read_to_string("/etc/passwd").unwrap()
eggplan-core source must stay pure: no filesystem, process, network, path, environment, database, or Git access (scanned: crates/eggplan-core/src, all files)
exit=1
```

## Requirement-to-evidence matrix

| # | Acceptance criterion (plan §9) | Evidence |
|---|---|---|
| 1 | The guard refuses to report a pass when its required tools are unavailable, with a non-zero exit | `scripts/check-core-boundary.sh:17-28` — `require_tools`; invoked at `:28`. Reproduction above: exit 1 |
| 2 | Filesystem, process, network, path, database, and Git access in core's production source fails the guard | `scripts/check-core-boundary.sh:116` — `no_impure_source`, scanning the whole `crates/eggplan-core/src` tree. Reproduction above: exit 1 |
| 3 | The guard carries deterministic synthetic self-proofs for every guard, and leaves no tracked file modified | `scripts/check-core-boundary.sh:165-301` — the `prove` helper and the proof set. Guard 1 is proven at `:210`, `:226`, `:233`; guard 2 at `:239`, `:257`, `:275`; guard 3 at `:283`, `:293`; the tracked tree is re-checked at `:299-301`. `git status --short` after the run shows no modification to any tracked file |
| 4 | The in-file header states the guard count, mechanism, and scan scope, and every failure message matches what was actually scanned | `scripts/check-core-boundary.sh:30-51` — the header states three guards with their exact scopes; the failure messages name the scanned path |
| 5 | The tooling-governance deep dive describes the integrations guard accurately and no longer claims it was unaffected by the ripgrep trap | `architecture/deep-dive-tooling-governance.md` — the integrations guard section now states the `rg`-based mechanism, the six named guards, their scan scopes, the `command -v` precondition, and the self-proofs; the ripgrep-history sentence now covers all three affected guards |
| 6 | The confirmed stale `file:line` citations in the affected deep dives resolve against the current code | Repaired in `architecture/deep-dive-projection-cli.md` (86 citation tokens: 72 corrected, 14 confirmed already-correct), `architecture/deep-dive-repository.md`, `architecture/deep-dive-integrations.md`, `architecture/deep-dive-markdown.md`, `architecture/deep-dive-core-domain.md`, and `architecture/deep-dive-tooling-governance.md`. Every corrected citation was verified by reading the cited range in the file it points at. See "Documentation and operations" |
| 7 | The current core manifest and source continue to pass, so the hardening introduces no false positive | `bash scripts/check-core-boundary.sh` passes against the real tree; the guard also proves its own non-false-positive cases at `scripts/check-core-boundary.sh:239` and `:257`, which assert that pure `std` use and core's own vocabulary are allowed |
| 8 | No production Rust source, dependency set, schema, digest, or public API changes | `git diff --stat` for this line shows changes only to `scripts/check-core-boundary.sh` and `architecture/deep-dive-tooling-governance.md`. No `Cargo.toml` changed, no `.rs` production file changed for this line, no schema or digest changed |
| 9 | Native and MSRV qualification pass, with all five boundary guards green | See "Verification executed". Hosted macOS/Windows native runs were **not** performed — see "Unresolved findings" #1 |

## Production implementation evidence

### The hardened guard

`scripts/check-core-boundary.sh` grew from 17 lines to 319. Its structure:

| Element | Location | Purpose |
|---|---|---|
| Precondition header | `:7-16` | Explains the ripgrep trap and why the precondition exists |
| `require_tools` | `:17-26` | The precondition, as a function so the self-proofs can exercise the refusal path against a deliberately absent tool name |
| Precondition invocation | `:28` | `require_tools rg mktemp \|\| exit 1` |
| Guard header | `:30-51` | Guard count, mechanism, and the exact scan scope of each guard |
| `no_prohibited_dependency` | `:108` | Guard 1 — the manifest, every section including `[dev-dependencies]` and `[build-dependencies]` |
| `no_impure_source` | `:116` | Guard 2 — the whole `crates/eggplan-core/src` tree, including inline `#[cfg(test)]` modules |
| `no_reasoning_field` | `:121` | Guard 3 — the whole source tree, for public reasoning/transcript fields |
| `prove` helper | `:165-208` | Runs a guard function against a fixture and asserts the expected outcome |
| Self-proofs | `:210-297` | Synthetic fixtures in a temporary directory; never reads or writes tracked source |
| Tracked-tree re-check | `:299-301` | The real manifest and source must pass all three guards |

Guard 2's pattern covers `std::fs`, `fs::` file operations, `File::`,
`std::process`, `Command::new`, `std::net`, `TcpStream`, `TcpListener`,
`UdpSocket`, `std::path`, `PathBuf`, `Path::`, `std::env`, `env::var`, `git2`,
`git::`, `sqlite`, `rusqlite`, and related forms. `include_str!` and
`include_bytes!` are deliberately excluded, because the core crate's own tests
read its source.

Every guard maps ripgrep's exit status explicitly: 0 is a violation, 1 is
clean, and anything else means the scan could not be performed. A scan that
could not be performed fails closed; it is never reported as a pass. This is
the specific mechanism by which the original defect occurred, so it is now
closed explicitly rather than by accident.

The precondition requires `rg` and `mktemp` only. The plan explicitly rejected
an unnecessary `awk` dependency, and none was added.

### Self-proofs

| Proof | Location | Asserts |
|---|---|---|
| `guard1 clean manifest` | `:210` | A clean manifest passes |
| `guard1 rejects: <family>` | `:226` | Each prohibited dependency family is rejected |
| `guard1 rejects a dev-dependency` | `:233` | The manifest scan is not section-scoped |
| `guard2 allows pure std use` | `:239` | Pure `std` use is not a false positive |
| `guard2 allows core vocabulary` | `:257` | Core's own vocabulary is not a false positive |
| `guard2 rejects: <case>` | `:275` | Filesystem, process, network, path, environment, database, and Git access are each rejected |
| `guard3 allows a plain public struct` | `:283` | A plain public struct is not a false positive |
| `guard3 rejects: <case>` | `:293` | Reasoning/transcript field names are rejected |
| `the real eggplan-core manifest must be free of prohibited dependency families` | `:299` | The tracked manifest passes |
| `the real eggplan-core source must be free of impure access` | `:300` | The tracked source passes |
| `the real eggplan-core source must be free of reasoning/transcript fields` | `:301` | The tracked source passes |

The tool-unavailable case is proven through the precondition function itself:
`require_tools` is called with a deliberately absent tool name in the
self-proof, so the refusal path is exercised without hiding `rg` from the real
run. The proof is deterministic and does not depend on the network, on process
execution of other programs, or on a Git worktree.

### The tooling-governance deep dive

`architecture/deep-dive-tooling-governance.md` was corrected on every point
the audit found wrong:

- The integrations guard is described as `rg`-based, not `grep`-based. The old
  text named `grep -nE`, `grep -RInE`, and `grep -q`; the script uses `rg`
  exclusively.
- The guard count is six, not an unspecified number:
  `no_sibling_runtime`, `no_transport_dep`, `no_credential_dep`,
  `no_process_source`, `no_acquisition_source`, `no_trust_enrollment`, plus the
  `requires_core_contract` check.
- Guards 1-3 are unscoped by manifest section, so `[dev-dependencies]` is
  included; guards 4-6 scan the whole source tree rather than a hand-listed
  set of modules, so a newly added adapter cannot escape enforcement by not
  being named.
- The `command -v` precondition requiring `rg` and `awk` is stated.
- The deterministic synthetic self-proofs are described.
- The ripgrep-history sentence no longer says the integrations guard "was
  never affected" by the ripgrep trap. It is now true of all three affected
  guards: integrations, projection/CLI, and closure authority.

Every `file:line` citation in the corrected document was verified against the
file it points at.

## Verification executed

Run by the coordinator against the integrated working tree:

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo check --workspace --all-targets --locked` | pass |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | pass, no warnings |
| `cargo test --workspace --locked` | pass — 312 passed, 0 failed, 3 ignored (pre-existing characterization tests) |
| `bash scripts/check-core-boundary.sh` | pass — `eggplan-core boundary self-proofs passed` / `eggplan-core static boundary check passed` |
| `bash scripts/check-codegg-compat-boundary.sh` | pass |
| `bash scripts/check-integrations-boundary.sh` | pass |
| `bash scripts/check-projection-cli-boundary.sh` | pass |
| `bash scripts/check-closure-authority-boundary.sh` | pass |
| `cargo +1.89.0 check --workspace --all-targets --locked` | pass |
| `cargo +1.89.0 test --workspace --locked` | pass — 312 passed, 0 failed, 3 ignored |

Guard-defect reproductions run by the coordinator against the hardened script:

| Reproduction | Expected | Result |
|---|---|---|
| `PATH` without `rg`, `bash scripts/check-core-boundary.sh` | refuse | `required boundary-guard tool 'rg' is not installed; refusing to report a pass`, exit 1 |
| Synthetic core source containing `std::fs::read_to_string("/etc/passwd").unwrap()`, with a manifest listing only `serde` | fail | `eggplan-core source must stay pure: ... (scanned: crates/eggplan-core/src, all files)`, exit 1 |

**Not run:** the hosted CI matrix on ubuntu, macOS, and Windows native
runners. The local evidence is Linux-native plus the Rust 1.89.0 MSRV job. CI
job identifiers must be added to this record as an amendment once the change is
pushed.

## Invariant review

- **The core crate's purity is unchanged.** The guard was hardened; no core
  source changed for this line. The tracked core manifest and source still
  pass all three guards.
- **No guard weakened.** The other four boundary scripts were not modified.
  The core guard's two original guards — the prohibited-dependency-family scan
  and the reasoning/transcript-field scan — still run, with the same scope.
- **The guard fails closed.** A missing tool, an unreadable source tree, or an
  unscannable path produces a non-zero exit, never a pass.
- **The guard leaves no residue.** The self-proofs build their fixtures in a
  temporary directory and clean up after themselves; `git status --short` after
  a run shows no modification to any tracked file.
- **The guard stays fast and hermetic.** It depends on `rg` and `mktemp` only,
  performs no network access, requires no Rust toolchain, and requires no Git
  worktree.

## Failure, recovery, and compatibility review

- **Failure behavior:** the guard's own failure modes changed deliberately and
  were the point of this corrective. A CI runner without `rg` now fails the
  job instead of silently passing it. The CI workflow installs `rg`; if that
  step were ever removed, the failure would now be loud.
- **Recovery:** none required. The guard is a static check with no state.
- **Compatibility:** no interface outside the guard changed. The guard's
  success message and exit codes are unchanged on a passing run.

## Security, trust, and path review

- **Boundary enforcement:** this is a security-relevant hardening. The core
  crate's purity — no filesystem, process, network, path, environment,
  database, or Git access — was an unenforced invariant before this pass. A
  single accidental `std::fs::read_to_string` in core would have passed every
  CI check. It now fails the build.
- **Trust:** the guard enforces a structural boundary, not a trust
  relationship. No provider trust or evidence trust rule changed.
- **Paths:** no path handling was added to or changed in any production crate.
  The guard's own fixture paths live under a temporary directory.

## Documentation and operations

- `architecture/deep-dive-tooling-governance.md` — corrected as described
  above. This is the design record for what each boundary script enforces and
  what its failure means.
- `architecture/deep-dive-repository.md` — the `RepoError` variant count and
  range citation were corrected as part of this finding.
- `architecture/deep-dive-integrations.md` — the `ProviderRegistry`
  attribution, which pointed at `crates/eggplan-integrations/src/lib.rs`
  instead of `crates/eggplan-core/src/evidence.rs`, was corrected.
- `architecture/deep-dive-projection-cli.md` — the drifted `lib.rs:` citations
  were re-verified against the current CLI source and repaired. The document
  contained 86 citation tokens; **72 were corrected and 14 were confirmed
  already-correct** and left byte-identical. The corrections covered the
  `eggplan-projection` crate's public functions, the CLI's `run`, `execute`,
  `parse_args`, `dispatch`, `validate_command_options`, and the command
  handlers, the check-state and reason-code literals, and the
  `crates/eggplan-repo/src/store.rs` reference. Where the code had moved to a
  different file — the usage string and the per-command option allow-lists now
  live in `crates/eggplan-cli/src/commands.rs` — the citation names the file
  where the code now lives rather than a stale line in the old file.
- `architecture/deep-dive-markdown.md` and `architecture/deep-dive-core-domain.md`
  — citations that the core changes made stale were re-verified and repaired.
  The core change inserted five lines into
  `crates/eggplan-core/src/model.rs` after line 216, so every citation into
  that file below line 216 shifted by five; the shift was computed
  mechanically against the baseline file rather than by hand.
- `architecture/deep-dive-tooling-governance.md` — the citations into
  `plans/registry.md` were re-verified after the registry grew, and now point
  at the numbered design gates (`registry.md:359`–`:425`), the planning
  hygiene rules (`registry.md:429`–`:436`), the upstream provenance
  predecessor (`registry.md:284`–`:287`), and the non-blocking scoping
  statement (`registry.md:118`–`:123`, `:296`).

## Unresolved findings

| # | Severity | Finding | Disposition |
|---|---|---|---|
| 1 | medium | The hosted CI matrix (ubuntu, macOS, Windows native) was not run. | Must be run by CI after the change is pushed; record the job identifiers here as an amendment. |
| 2 | low | `architecture/deep-dive-repository.md` retains further pre-existing stale citations outside the paragraphs this pass touched. | Pre-existing debt, out of this corrective's scope. |
| 3 | informational | The hardened core guard now fails CI on a runner without `rg`. The CI workflow installs `ripgrep`; the failure is loud rather than silent, which is the intended outcome. | No action required. Recorded so that a future workflow change removing the `ripgrep` install is not mistaken for a guard regression. |

## Roadmap disposition

Foundation-core roadmap M003 corrective line C005 is closed. The
"M003 corrective lines" table in `plans/subsystems/foundation-core-roadmap.md`
is updated to record this closure and its closure record.

The findings that opened this corrective — a guard that reported an unscanned
pass, a guard that did not enforce the core crate's purity, and a design
record that misdescribed the tooling — are closed.

## Registry updates

- `plans/registry.md`: the Foundation M003 C005 row in *Registered
  implementation plans* is updated to `closed`, with a pointer to
  `plans/closure/foundation-core/003-c005-closed.md`.
- `plans/registry.md`: the *Correctives raised by the 2026-10-23 repository
  audit* subsection records the closure of all five Foundation lines.
- `plans/subsystems/foundation-core-roadmap.md`: M003 corrective line C005 is
  marked closed.
- No historical closure record was rewritten.
