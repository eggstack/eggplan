# Evidence and Closure M001 Closure — Evidence Ledger and Deterministic Assessment

Status: closed

Source plan: plans/implementation/evidence-closure/001-evidence-ledger-and-assessment.md

Source roadmap: plans/subsystems/evidence-closure-roadmap.md

Reviewed baseline: 79dcf41 (`plans: unblock evidence M001`)

Implementation commit: e711355 (`feat(evidence): add immutable ledger and assessment`)

Closure record and status transition: recorded in Git history.

## Executive finding

Evidence M001 is implemented and its local Linux contract is verified. Eggplan
now has immutable schema-v1 observations, a host-owned provider trust registry,
an append-only repository ledger, exact-subject matching, and pure deterministic
criterion/item/plan assessment. Missing, stale, failed, unavailable, and
in-flight evidence cannot be promoted to success by prose or item status.
Explicit human judgment remains policy-gated. ClosureRecord persistence is
still Evidence M002. Foundation M002's Windows/macOS qualification gap is
inherited and remains open; this closure makes no platform claim beyond Linux.

## Requirement-to-evidence matrix

| Requirement | Evidence | Result |
|---|---|---|
| Schema-v1 immutable observation | `crates/eggplan-core/src/evidence.rs`; schema and digest fixture | Pass; finalized fields are private and digest covers canonical content, excluding the digest field itself |
| Status and kind compatibility | `crates/eggplan-core/tests/fixtures/evidence-v1-digests.json` | Pass; fixtures freeze all eight statuses and ten evidence kinds |
| Bounded observation/provider data | Core evidence bounds and repository byte/count bounds | Pass; provider labels, metadata, subject paths, observation size, ledger count, and plan envelope size are bounded |
| Provider authority | Explicit host-constructed `ProviderRegistry` and `ProviderDescriptor` | Pass; serialized observation content cannot register or self-authorize a trusted provider |
| Append-only persistence and replay | `crates/eggplan-repo/src/store.rs`; repository integration tests | Pass; observations are digest/schema/filename checked, exact replay is idempotent, conflicting ID reuse is rejected, reopen/list/get preserve content |
| Corruption and path safety | Corrupt digest/schema and symlink evidence directory tests | Pass on Linux; invalid stored content and symlink-backed evidence directory fail closed |
| Exact subject applicability | Core assessment and repository subject tests | Pass; stale subject observations remain historical and do not satisfy current criteria |
| Requirement matching | Core assessment tests | Pass; kind/provider filters, any/all and minimum-count matching are deterministic and bounded |
| Assessment precedence | Core assessment tests | Pass; criterion, item, and plan results use fixed precedence and bounded reason codes |
| Completion requires proof | Completed-item-without-evidence fixture | Pass; lifecycle prose alone cannot make a plan complete |
| Human judgment | Criterion policy and assessment fixtures | Pass; human judgment can satisfy only when explicitly allowed and represented as such |
| CodeGG semantic reference | Handoff re-check at CodeGG SHA `28b46956`; fixture cases in core tests | Pass; missing evidence remains unavailable, owner/run/job provenance alone is not success, and user judgment is distinct; no CodeGG dependency or ownership concepts were added |
| Foundation M002 platform boundary | `plans/closure/foundation-core/002-conditionally-closed.md`; `architecture/repository.md` | Preserved; Linux is qualified, Windows/macOS runtime and cross-target evidence remain outstanding |

The schema-v1 fixture digests are the compatibility baseline. No migration or
rewrite of finalized observations is performed during reads. Explicit
observation references are not required in this milestone; requirement
assessment resolves the bounded repository ledger by policy and exact subject.

## Exact verification executed

Commands below ran from the repository root after the final production-code
change, including the final Evidence M001 additions. All listed commands passed:

| Command | Result |
|---|---|
| `rtk cargo fmt --all -- --check` | Pass |
| `rtk cargo check --workspace --all-targets --locked` | Pass |
| `rtk cargo clippy --workspace --all-targets --locked -- -D warnings` | Pass |
| `rtk cargo test --workspace --locked` | Pass, 37 tests across 5 suites |
| `rtk cargo +1.89.0 check --workspace --all-targets --locked` | Pass |
| `rtk bash scripts/check-core-boundary.sh` | Pass |
| `rtk git diff --check` | Pass |

These results establish the Linux workspace and Rust 1.89 build/test contract.
Windows/macOS runtime tests were not run. Cross-target attempts and their
environmental failures are recorded in the Foundation M002 closure; they are
not passing evidence and remain outstanding.

## Invariant, recovery, and security review

- Assessment is pure and deterministic; it does not run commands, access the
  network, or schedule work.
- A stale subject is inapplicable by default. Historical observations remain
  readable but cannot satisfy a current subject's requirement.
- The provider registry represents explicit host policy, not cryptographic
  authentication. A host must keep registry construction and observation
  finalization behind its trusted adapter boundary.
- Digests detect content changes but do not establish provider authenticity.
- Corrupt schema/digest and conflicting ID reuse fail explicitly; identical
  replay is idempotent. Finalized observations are never rewritten.
- Fields are bounded and no hidden model-reasoning field is persisted. Hosts
  remain responsible for excluding secrets from optional adapter metadata.
- No migration was needed because no earlier runtime ledger existed.
- The capability adds no scheduler, executor, process runner, issue tracker, or
  model reasoning store.

## Documentation and operations

`architecture/evidence.md` documents the observation schema, provider trust
boundary, subject matching, assessment semantics, and persistence limits.
`architecture/repository.md` documents the filesystem ledger layout and
inherited durability qualification. The README points to the workspace and
architecture documentation.

## Roadmap and registry disposition

Evidence M001 is closed. The following roadmap milestones are now eligible for
separate implementation planning; this closure does not implement or hand off
those milestones:

- Projection/CLI M001 — CLI control surface and derived registry.
- CodeGG Integration M001 — golden parity and adapter seam; preserve the
  Foundation M002 platform caveat.
- Eggstack Integrations M001 — provider SPI.
- Foundation M003 — hardening and migration guards, prioritizing Windows/macOS
  qualification.
- Evidence M002 — closure records and integrity/recovery.

Projection/CLI M002, CodeGG Integration M002, and Eggstack Integrations M002
remain blocked on their own M001 work. Interop/distribution remains deferred.
The subsystem roadmaps and `plans/registry.md` were updated to reflect these
statuses and the next eligible planning work.

## Unresolved findings

No Evidence M001 implementation defect remains open. The inherited Foundation
M002 operational gap remains: Windows and macOS runtime/cross-target
qualification is unavailable in the current evidence set. Track that work in
Foundation M003 and preserve the conditional status of Foundation M002 until
the required evidence is obtained.
