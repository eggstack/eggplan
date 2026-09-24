# Evidence and Closure M002 Closure Record

Status: closed

Source plan: plans/implementation/evidence-closure/002-closure-records-integrity-and-recovery.md

Source roadmap: plans/subsystems/evidence-closure-roadmap.md

Implementation commit: 24661afe799b745f83ba11a5149af724587b50fe

## Executive finding

Guarded closure is implemented in `eggplan-core` and `eggplan-repo`. Raw Plan
CAS rejects entry to Closed. Finalization reloads and reassesses exact active
state under the repository lock, persists a pending immutable record before
the Closed Plan, then promotes the record. Open recovers either crash point and
deeply replays the recorded assessment and verifies evidence, lineage, policy,
and Plan digests.

## Requirement-to-evidence matrix

| Requirement | Evidence |
|---|---|
| Distinct closure and supersession IDs; versioned digested records | `crates/eggplan-core/src/identity.rs`; `crates/eggplan-core/src/closure.rs`; ID and supersession unit tests |
| Append-only correction lineage, cycle/dangling/conflict rejection, stable effective view | `effective_observations`; `supersession_is_append_only_effective_lineage_and_detects_cycles`; repository append validation |
| Pure complete-only closure candidate and provider-policy snapshot | `ClosureCandidate::build`; guarded closure repository test |
| Ordinary CAS cannot close | `illegal_lifecycle_updates_are_rejected`; `PlanStore::compare_and_swap` guard |
| Current subject, Plan revision, assessment, observation, provider policy, and lineage revalidation | `RepositoryStore::finalize_closure`; `guarded_closure_persists_integrity_and_reopens` |
| Pending before Plan replacement, source-side discard, target-side promotion | `guarded_closure_persists_integrity_and_reopens` simulates pending/source recovery and pending/target recovery using the repository layout |
| Reopen integrity and deterministic matrix replay | `RepositoryStore::load_unlocked`; the same end-to-end test also corrupts a referenced observation and verifies reopen fails |
| Immutable finalized closure and no raw bypass | closure file existence/validation and raw CAS negative test |

## Production and documentation evidence

- Typed ClosureId and EvidenceSupersessionId are separate from Plan and
  observation IDs.
- Supersession correction does not rewrite old observations.
- ClosureRecord stores digests and bounded metadata, not evidence payloads.
- Historical provider-policy facts are validated and never auto-registered for
  future assessments.
- `architecture/core.md`, `architecture/evidence.md`, and
  `architecture/repository.md` document domain, trust, and recovery semantics.
- No scheduler, executor, remote evidence acquisition, policy DSL, or signature
  authority was added.

## Verification executed

Local Linux commands on implementation revision `24661afe799b745f83ba11a5149af724587b50fe`:

| Command | Result |
|---|---|
| `cargo fmt --all` | pass |
| `cargo test --workspace --locked` | pass; 54 tests |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | pass |
| `bash scripts/check-core-boundary.sh` | pass |
| `git diff --check` | pass |
| `cargo +1.89.0 check --workspace --all-targets --locked` | pass |
| `cargo +1.89.0 test --workspace --locked` | pass |

Hosted native and MSRV workflow:

- Run: https://github.com/eggstack/eggplan/actions/runs/35953186805
- Linux job: 107485899223 — pass (format, check, clippy, tests, boundary).
- macOS job: 107485899119 — pass (check, clippy, tests, boundary).
- Windows job: 107485899172 — pass (check, clippy, tests; platform-gated steps
  skipped as specified by workflow).
- Rust 1.89 job: 107485899004 — pass (check and tests).

## Invariants and residual findings

- Plan closure remains evidence-derived, exact-subject, and provider-policy
  controlled.
- Pending state is never reported as finalized closure. Reopening validates
  and recovers before listing canonical state.
- Hashes establish content integrity, not authenticity.
- No unresolved M002 correctness finding remains. Recovery is cooperative and
  uses same-directory atomic replacement and directory sync where supported.

## Roadmap and registry disposition

Evidence M002 is closed. Projection/CLI M001, CodeGG Integration M001, and
Eggstack Provider SPI M001 are ready after their registered baselines are
refreshed to the current Evidence closure and sibling interfaces. Their
respective production work remains independent and outside this closure.
