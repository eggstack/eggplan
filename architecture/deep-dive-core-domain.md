# Deep dive: `eggplan-core` domain

Index: [overview](overview.md) · Normative companions: [core](core.md), [evidence](evidence.md).
Status context: foundation milestones M001–M003 are closed; evidence-closure M001–M002 plus
correctives C001–C003 are closed; policy extensions (M003) are deferred.

## 1. Role and boundary

`eggplan-core` owns deterministic domain data and pure validation: typed IDs, plan/item/
criterion/requirement schemas, bounds, transitions, graph readiness, canonical serialization
and digests, pure assessment, closure-candidate/record shapes, and supersession lineage.
Crate docs state the boundary up front (`src/lib.rs:1-6`): no filesystem, process, network,
database, scheduler, or model-runtime responsibilities. The dependency list enforces it
(`crates/eggplan-core/Cargo.toml:9-14`): only `serde`, `serde_json`, `sha2`, `thiserror`,
`uuid`. Persistence, Git subject capture, and closure finalization belong to `eggplan-repo`;
observation acquisition belongs to provider adapters. Notably, core's `ProviderRegistry`
(`src/evidence.rs:140-160`) only records host-supplied trust — observation text can never
enroll its own provider.

## 2. Module walkthrough

- `lib.rs` — re-exports, `is_execution_evidence()` (`src/lib.rs:33-42`, the five bound
  kinds: Command, Test, StaticAnalysis, DelegatedRun, Benchmark), and the `bounds` module
  (`src/lib.rs:46-67`): Unicode-scalar text limits, `MAX_ITEMS = 512`,
  `MAX_DEPENDENCIES = 64`, `MAX_CRITERIA = 128`, `MAX_REQUIREMENTS = 32`, plus observation
  metadata/artifact caps. `#![forbid(unsafe_code)]` (`src/lib.rs:1`).
- `identity.rs` — `TypedId` trait (`src/identity.rs:76-81`) and `define_id!` macro
  (`src/identity.rs:83-148`): `ep_` / `epi_` / `epc_` / `epp_` / `epe_` / `epcl_` / `eps_`
  prefixes, ≤96 chars, ASCII alphanumerics plus `-`/`_`. `VerificationDigest`
  (`src/identity.rs:10-31`) enforces `sha256:` + 64 lowercase hex chars.
- `model.rs` — `PlanStatus` / `PlanItemStatus` (`src/model.rs:9-26`), `EvidenceKind`
  (`src/model.rs:30-41`), `SubjectRevision` / `ArtifactRef` (`src/model.rs:52-69`),
  `EvidenceRequirement` / `AcceptanceCriterion` / `PlanItem` / `Plan`
  (`src/model.rs:86-137`). All schema structs use `#[serde(deny_unknown_fields)]`.
  `Plan::new` (`src/model.rs:270-287`) stamps `SCHEMA_VERSION` and validates.
  `plan_transition_allowed` / `item_transition_allowed` (`src/model.rs:391-422`) leave
  Closed/Cancelled (plan) and Completed/Cancelled (item) terminal with no outgoing edges.
- `graph.rs` — `validate_graph` (`src/graph.rs:25-43`) rejects dependency and parent
  cycles separately (`GraphError`, `src/graph.rs:6-11`) via iterative DFS that avoids
  attacker-controlled recursion depth (`src/graph.rs:44-81`). `readiness`
  (`src/graph.rs:83-120`) is position-then-ID ordered, `Ready` only for Active plans with
  Pending/Actionable items whose dependencies are all Completed — a derived statement,
  not execution authority.
- `schema.rs` — `SCHEMA_VERSION = 2` (`src/schema.rs:5`); `canonical_json`
  (`src/schema.rs:9-11`), `digest_json` (`src/schema.rs:13-17`); `verification_digest`
  (`src/schema.rs:22-90`) builds the domain-separated
  `eggplan.provider-verification.v1` envelope with depth/node/byte bounds;
  `parse_plan` (`src/schema.rs:92-96`) deserializes then validates.
- `evidence.rs` — `EVIDENCE_SCHEMA_VERSION = 2` (`src/evidence.rs:9`); closed
  `EvidenceStatus` vocabulary (`src/evidence.rs:13-22`); `EvidenceObservation` keeps
  fields private behind accessors (`src/evidence.rs:162-235`); `finalize`
  (`src/evidence.rs:163-182`) validates then hashes `ObservationContent`
  (`src/evidence.rs:59-73`), which excludes `content_digest` itself.
  `ProviderDescriptor::new` / `register_trusted` (`src/evidence.rs:98-160`) reject empty
  kind sets and duplicate IDs.
- `assessment.rs` — `assess_plan` (`src/assessment.rs:84-154`) is pure over
  (plan, current subject, observations, registry): sorts observations by ID, flags
  duplicates and invalid plans, then folds item assessments through `highest()`.
  Detail in §4.
- `closure.rs` — `EvidenceSupersessionRecord::new` / `validate`
  (`src/closure.rs:30-86`); `effective_observations` (`src/closure.rs:89-132`);
  `ClosureCandidate::build` (`src/closure.rs:150-204`); `ClosureRecord::finalize` /
  `validate` (`src/closure.rs:220-281`). Core only *shapes* closure; the repository
  finalizer owns subject recapture and the Closed-plan write.

## 3. Canonical-JSON / digest contract and v1-vs-v2

Canonical bytes are compact `serde_json::to_vec` of structs with fixed declaration order
(`BTreeMap` keys sorted); digests are `sha256:<64 lowercase hex>` over those exact bytes.
Plans accept schema 1 or 2 (`src/model.rs:289`); observations likewise
(`src/evidence.rs:254`). The version rules are mirrored in both places: v1 structs must
not carry a verification binding (`src/model.rs:308-314`, `src/evidence.rs:257-261`),
while v2 *requires* one for every execution kind (`src/model.rs:315-323`,
`src/evidence.rs:262-269`). Unknown fields fail closed at every level, including nested
criteria, requirements, subjects, and artifacts — pinned by
`nested_unknown_plan_fields_fail_closed` (`src/schema.rs:186-214`) and
`legacy_evidence_rejects_unknown_nested_subject_and_artifact_fields`
(`src/evidence.rs:438-454`). Valid v1 bytes/digests stay frozen (see §5).

## 4. Assessment precedence and closure-candidate semantics

Per-requirement gating (`src/assessment.rs:235-319`) runs in order: legacy-unbound
execution requirement → fail closed (`LegacyUnboundExecutionRequirement`,
`src/assessment.rs:243-252`); untrusted provider / disallowed kind (both set `invalid`);
provider mismatch (skipped, not `invalid`); human-judgment policy — kind requires
`allow_human_judgment` at requirement *and* criterion level *and* provider class
`"human"` (`src/assessment.rs:281-289`); corrupt digest; subject inequality
(`StaleSubject`); verification-digest presence/mismatch. Only survivors count as
`eligible`; `Any` needs `min_count` Passed, `All` needs all eligible Passed
(`src/assessment.rs:321-334`). Unsatisfied status within a requirement prefers failed →
blocked → in-flight → inconclusive → missing/unavailable → stale → missing
(`src/assessment.rs:335-374`); across requirements/criteria/items/plans, `highest()`
ranks InvalidOrStale 9 > Failed 8 > Blocked 7 > InFlight 6 > Missing 5 >
AwaitingHumanJudgment 4 > Inconclusive 3 > ActionableWorkRemaining 2 > Complete 1
(`src/assessment.rs:394-409`). Item labels never self-certify: evidence-complete but
non-Completed items yield InFlight/ActionableWorkRemaining, and only Completed +
criteria-complete yields Complete (`src/assessment.rs:208-223`); criteria-less items and
empty plans stay incomplete (`src/assessment.rs:208-211`, `src/assessment.rs:134-137`).

`ClosureCandidate::build` demands a Complete assessment bound to the exact plan revision
and subject (`src/closure.rs:159-167`), then snapshots satisfying-observation digests,
sorted supersession digests, and the sorted provider policy plus its digest
(`src/closure.rs:168-203`). `ClosureRecord::finalize` only targets revision
`source + 1` with status Closed (`src/closure.rs:226-231`); `validate` rechecks plan
digest, policy digest, and record digest (`src/closure.rs:253-281`). Stored policy is
history, not future trust.

## 5. Test strategy

There is no `tests/` integration directory — only `tests/fixtures/` plus inline
`#[cfg(test)]` unit tests per module. Golden fixtures freeze bytes, not pretty JSON:
`schema-v1-plan.json` + `.sha256`, `schema-v2-plan.json` + `.sha256`,
`evidence-v2-observation.json` + `.sha256`, and `evidence-v1-digests.json` (8 status +
10 kind content digests). Tests assert byte-for-byte equality and digest equality
(`src/schema.rs:143-170`, `src/evidence.rs:371-435`), digest sensitivity and tamper
rejection (`src/evidence.rs:476-497`), strict-schema rejection (§3 refs), per-status
determinism, stale/dirty/untrusted rejection, cardinality and human-judgment policy,
supersession cycles, and v2-binding enforcement for all five execution kinds
(`src/model.rs:512-559`, `src/assessment.rs:511-653`).

## 6. Review findings

Strengths: tiny dependency surface; `forbid(unsafe_code)`; Unicode-scalar (not byte)
bounds with NUL/empty rejection; terminal-state matrices with tests; exact-subject
equality including dirty-digest sensitivity; deterministic ID-sorted assessment;
append-only supersession with self-link/dangling/cycle/duplicate/conflict fail-closed
(`src/closure.rs:89-132`); production code is essentially panic-free — the audit found
`unwrap`/`expect` only inside `#[cfg(test)]` with one exception below.

Gaps / risks / surprises:

1. **`model.rs:216` is the sole production `expect`** — `AcceptanceCriterion::validate`
   serializes each requirement to detect exact duplicates via
   `serde_json::to_string(req).expect(...)`. Infallible in practice but turns a
   `Result`-returning validator into a theoretical panic path; returning
   `ValidationError::Invalid` would keep validation total.
2. **Closure shapes use `String` errors** (`src/closure.rs:30-281`: `Result<_, String>`
   throughout) while the rest of the crate has typed `IdError` / `ValidationError` /
   `EvidenceError` / `GraphError` — repository callers cannot match on failure kinds.
   Related: `ClosureCandidate` carries no self `content_digest` (supersession records
   and `ClosureRecord` do); candidate integrity rests on the record digest covering it.
3. **Precedence asymmetry is subtle**: within one requirement Inconclusive outranks
   NotRun/Skipped/Unavailable (`src/assessment.rs:355-368`), but across requirements
   `highest()` ranks Missing (5) above Inconclusive (3). Deterministic and probably
   intended, but no comment or test pins the intent — a future edit could flip either
   side without failing.
4. **Magic `"human"` provider class** (`src/assessment.rs:284`) is the only
   stringly-typed policy gate; SPI consumers must discover it from code rather than a
   shared constant. Minor.
5. **Closure/supersession coverage in-crate is thin** — one unit test
   (`src/closure.rs:313-343`); candidate/record round-trips are presumably exercised
   from `eggplan-repo`, which is correct layering but means this crate alone does not
   prove its closure shapes. I did not verify the repo-side coverage (out of scope).

No inconsistencies with [core](core.md) or [evidence](evidence.md) were found; the
code matches both documents, including the v1/v2 binding rules and the "candidate
builds, repository finalizes" authority split.

## 7. Verification pointers

```sh
cargo test -p eggplan-core
cargo test -p eggplan-core assessment
bash scripts/check-core-boundary.sh
```
