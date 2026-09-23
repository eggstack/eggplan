# Evidence M001 C001 Closure — Verification Binding and End-to-End Evidence

Status: closed

Source plan: plans/implementation/evidence-closure/001-c001-verification-binding-and-end-to-end-evidence-corrective.md

Source roadmap: plans/subsystems/evidence-closure-roadmap.md

Reviewed baseline: `ff5f60c4355177985fcc7bb3ec043d1ed0f684d5`; implementation commit `eeb5d99515d44b5842cc867190fe3257c28c966f`.

Predecessor closure: plans/closure/evidence-closure/001-closed.md. Foundation
gate: plans/closure/foundation-core/003-closed.md.

## Executive finding

C001 closes. Schema-v2 requirements and execution-derived observations are
bound to an opaque validated SHA-256 verification-spec digest. Exact binding
is required before an observation can contribute to satisfaction. Schema-v1
plans and observations remain readable with their historical canonical bytes
and digests; unbound v1 execution requirements fail closed and v1 observations
do not satisfy bound requirements. The integrated repository regression proves
persisted evidence does not stale its own subject, a mismatched verification
does not count, and later source changes make prior observations stale.

Hosted workflow [35864604300](https://github.com/eggstack/eggplan/actions/runs/35864604300)
passed on native Linux, macOS, Windows, and Rust 1.89 Linux.

## Finding-to-evidence matrix

| Finding | Correction | Evidence |
|---|---|---|
| E-C001-01 kind/provider/subject match was broad | Typed expected/observed verification digest and exact comparison before evidence eligibility | `execution_evidence_requires_exact_verification_binding`; `mismatched_observations_are_excluded_from_any_and_all_cardinality` |
| E-C001-02 invocation_ref was being considered as identity | Kept invocation_ref as provenance; dedicated VerificationDigest is the only verification match identity | `architecture/evidence.md`; v2 wire fixtures |
| E-C001-03 v1 migration could invent proof | Explicit v1/v2 validation; no digest inference; legacy unbound requirement reason; v1 observations remain immutable/readable | `legacy_unbound_execution_requirement_fails_closed`; `legacy_unbound_observation_cannot_satisfy_a_v2_requirement`; unchanged v1 fixtures |
| E-C001-04 persist/recapture path untested | Integrated Git + RepositoryStore + append + recapture + assess + source mutation + reopen regression | `bound_evidence_persists_recaptures_and_assesses_end_to_end` |
| E-C001-05 evidence architecture file absent | Added architecture/evidence.md and linked it from README/core docs | File exists and describes implemented contracts |

## Schema and compatibility evidence

- Plan schema v1 canonical JSON and digest fixtures remain unchanged and parse
  successfully.
- Evidence v1 status/kind digest fixture remains unchanged; a compatibility
  reader validates and roundtrips legacy observations without rewriting them.
- Plan schema-v2 fixture includes a bound Test requirement and freezes its
  canonical bytes and digest.
- Evidence schema-v2 fixture freezes a bound Test observation's canonical
  bytes and content digest.
- Unknown plan/evidence versions fail closed. Unknown nested plan, subject,
  observation, and artifact fields fail during direct parse; repository reopen
  rejects unknown plan and artifact fields.
- All five execution-derived kinds (Command, Test, StaticAnalysis,
  DelegatedRun, Benchmark) reject missing v2 bindings. Malformed uppercase,
  short, or non-hex digests are rejected.

## Exact verification executed

Local commands passed against implementation commit `eeb5d99`:

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked                 # 52 passed
cargo +1.89.0 check --workspace --all-targets --locked
cargo +1.89.0 test --workspace --locked
bash scripts/check-core-boundary.sh
git diff --check
```

Hosted workflow 35864604300 passed:

| Runner | Evidence |
|---|---|
| Linux stable | format, workspace check, clippy, tests, core boundary guard |
| macOS stable | workspace check, clippy, tests, core boundary guard; format step is Linux-only |
| Windows stable | workspace check, clippy, tests; boundary script is skipped by workflow configuration |
| Linux Rust 1.89 | workspace check and tests |

## Invariant, failure, and compatibility review

- Provider identity and allowed-kind authority remain host-constructed.
- Assessment does not run commands, access the network, or infer a digest from
  requirement text or invocation_ref.
- Verification digest participates in observation content digests. Reusing an
  observation ID with a changed binding conflicts in repository append.
- Provider trust/kind, provider match, integrity, and subject checks precede
  binding equality and passing status.
- `Any` counts only bound eligible observations. `All` ignores observations
  for other verification bindings; matching failed observations retain their
  prior failure semantics.
- Exact SubjectRevision remains the default. Persisted state is excluded from
  dirty identity by Foundation M003; normal source changes still stale prior
  observations.
- v1 finalized observations are never rewritten during load or assessment.

## Unresolved findings

None within C001 scope. The hosted workflow skips Windows execution of the
shell boundary guard; Windows native compilation, clippy, and repository tests
passed. This does not limit the cross-platform repository qualification.

## Roadmap disposition

Foundation M003 and Evidence M001 C001 corrective gates are closed. The
following downstream milestones' named corrective blockers are cleared, so
they are ready for implementation planning:

- Evidence M002 — closure records and integrity/recovery.
- Projection/CLI M001 — CLI control surface and derived registry.
- CodeGG Integration M001 — golden parity and adapter seam.
- Eggstack Integrations M001 — provider SPI.

No downstream implementation plan was registered as part of this closure.
Those handoffs still need their own bounded plan and any sibling-interface
recheck required by the roadmap. Interop/distribution remains deferred.

## Registry updates

- Evidence M001 C001: closed.
- Evidence M002, Projection/CLI M001, CodeGG Integration M001, and Eggstack
  Provider SPI M001: ready for implementation planning.
