# Deep dive: `eggplan-core` domain

Index: [overview](overview.md) · Normative companions: [core](core.md), [evidence](evidence.md).
Status context (per `plans/registry.md`, the authority for milestone status): foundation
M001 and M003 are closed and M002 is recorded *conditionally closed* (its platform caveat
was resolved by M003); evidence-closure M001–M002 plus correctives C001–C003 are closed —
note there are two distinct C001s, the evidence-closure one and the CodeGG M003 one; the
interoperability/distribution roadmap is deferred.

## 1. Role and boundary

`eggplan-core` owns deterministic domain data and pure validation: typed IDs, plan/item/
criterion/requirement schemas, bounds, transitions, graph readiness, canonical serialization
and digests, pure assessment, closure-candidate/record shapes, and supersession lineage.
Crate docs state the boundary up front (`src/lib.rs:3-6`, under the `forbid(unsafe_code)`
attribute on line 1): no filesystem, process, network, database, scheduler, or
model-runtime responsibilities. The dependency list enforces it
(`crates/eggplan-core/Cargo.toml:10-14`): only `serde`, `serde_json`, `sha2`, `thiserror`,
`uuid`. Persistence, Git subject capture, and closure finalization belong to `eggplan-repo`;
observation acquisition belongs to provider adapters. Notably, core's `ProviderRegistry`
(`src/evidence.rs:140-160`) only records host-supplied trust — observation text can never
enroll its own provider.

## 2. Module walkthrough

- `lib.rs` — re-exports, `is_execution_evidence()` (`src/lib.rs:33-42`, the five bound
  kinds: Command, Test, StaticAnalysis, DelegatedRun, Benchmark), and the `bounds` module
  (`src/lib.rs:46-68`): Unicode-scalar text limits, `MAX_ITEMS = 512`,
  `MAX_DEPENDENCIES = 64`, `MAX_CRITERIA = 128`, `MAX_REQUIREMENTS = 32`, plus the
  observation-side caps `MAX_ARTIFACT_REFS = 64`, `MAX_OBSERVATION_METADATA = 32`,
  `OBSERVATION_METADATA_VALUE_CHARS = 2_000`, `INVOCATION_REF_CHARS = 2_000`, the closure-side
  `SUPERSESSION_REASON_CHARS = 2_000` (`src/lib.rs:56`), and
  `MAX_OBSERVATIONS_PER_PLAN = 10_000` (`src/lib.rs:67`). `MAX_OBSERVATIONS_PER_PLAN`
  is *declared* here but never enforced in this crate — it is applied only by the
  repository (`eggplan-repo/src/store.rs:974,1083`), so `assess_plan` itself is
  observation-count-unbounded. `#![forbid(unsafe_code)]` (`src/lib.rs:1`).
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
  (`src/schema.rs:27-113`) builds the domain-separated
  `eggplan.provider-verification.v1` envelope with depth/node/byte bounds — the byte
  bound is now consulted from the same structural walk that already counts nodes and
  string lengths, so a payload whose own serialized lower bound cannot fit is rejected
  before the buffer is built;
  `parse_plan` (`src/schema.rs:115-119`) deserializes then validates.
- `evidence.rs` — `EVIDENCE_SCHEMA_VERSION = 2` (`src/evidence.rs:9`); closed
  `EvidenceStatus` vocabulary (`src/evidence.rs:13-22`); `EvidenceObservation` keeps
  fields private behind accessors (`src/evidence.rs:162-235`); `finalize`
  (`src/evidence.rs:163-182`) validates then hashes `ObservationContent`
  (`src/evidence.rs:59-73`), which excludes `content_digest` itself.
  `ProviderDescriptor::new` / `register_trusted` (`src/evidence.rs:98-160`) reject empty
  kind sets and duplicate IDs.
- `assessment.rs` — `assess_plan` (`src/assessment.rs:96-173`) is pure over
  (plan, current subject, observations, registry): sorts observations by ID, flags
  duplicates and invalid plans, groups the sorted slice by evidence kind once so each
  requirement scans only its own kind, then folds item assessments through `highest()`.
  The four output structs carry `deny_unknown_fields`. Detail in §4.
- `closure.rs` — `EvidenceSupersessionRecord::new` / `validate`
  (`src/closure.rs:29-96`); `effective_observations` (`src/closure.rs:98-141`);
  `ClosureCandidate::build` (`src/closure.rs:158-214`); `ClosureRecord::finalize` /
  `validate` (`src/closure.rs:228-291`). Core only *shapes* closure; the repository
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
`nested_unknown_plan_fields_fail_closed` (`src/schema.rs:209-237`) and
`legacy_evidence_rejects_unknown_nested_subject_and_artifact_fields`
(`src/evidence.rs:438-454`). Valid v1 bytes/digests stay frozen (see §5).

## 4. Assessment precedence and closure-candidate semantics

Per-requirement gating (`src/assessment.rs:254-410`) runs in order: legacy-unbound
execution requirement → fail closed (`LegacyUnboundExecutionRequirement`,
`src/assessment.rs:262-271`); untrusted provider / disallowed kind (both set `invalid`);
provider mismatch (skipped, not `invalid`); human-judgment policy — kind requires
`allow_human_judgment` at requirement *and* criterion level *and* provider class
`"human"` (`src/assessment.rs:300-308`); corrupt digest; subject inequality
(`StaleSubject`, which also sets `invalid`); verification-digest
presence/mismatch. Only survivors count as
`eligible`; `Any` needs `min_count` Passed, `All` needs all eligible Passed
(`src/assessment.rs:338-353`). The unsatisfied status is chosen in strict order: an
`invalid` flag short-circuits to InvalidOrStale *before* any ordinary status
(`src/assessment.rs:355-359`), then failed → blocked → in-flight → inconclusive →
missing/unavailable (`src/assessment.rs:360-388`); across
requirements/criteria/items/plans, `highest()`
ranks InvalidOrStale 9 > Failed 8 > Blocked 7 > InFlight 6 > Missing 5 >
AwaitingHumanJudgment 4 > Inconclusive 3 > ActionableWorkRemaining 2 > Complete 1
(`src/assessment.rs:412-427`). Item labels never self-certify: evidence-complete but
non-Completed items yield InFlight/ActionableWorkRemaining, and only Completed +
criteria-complete yields Complete (`src/assessment.rs:227-242`); criteria-less items and
empty plans stay incomplete (`src/assessment.rs:227-230`, `src/assessment.rs:153-156`).

Correction to the original review of this file: the stale branch used to `continue`
without setting `invalid`, so a requirement holding a stale observation *and* an
eligible NotRun/Skipped/Unavailable observation reported
`EvidenceMissingOrUnavailable` — stale ranked *below* missing/unavailable, inverting
the precedence documented in [core](core.md) and contradicting `highest()`. Stale now
sets `invalid` like every other fatal branch, and the trailing `StaleSubject` fallback
that used to sit last in the chain is gone as dead code. `assess_plan` also groups the
ID-sorted observations by evidence kind once per call (`src/assessment.rs:120-130`)
instead of letting each requirement rescan the whole slice; groups are filled in sorted
order, so the per-requirement visit order — and therefore every emitted assessment — is
unchanged.

`ClosureCandidate::build` demands a Complete assessment bound to the exact plan revision
and subject (`src/closure.rs:168-176`), then snapshots satisfying-observation digests,
sorted supersession digests, and the sorted provider policy plus its digest
(`src/closure.rs:177-212`). `ClosureRecord::finalize` only targets revision
`source + 1` with status Closed (`src/closure.rs:235-240`), computed with `checked_add`
so an unbounded plan revision surfaces an error instead of overflowing; `validate`
rechecks plan digest, policy digest, and record digest (`src/closure.rs:262-290`). Stored
policy is history, not future trust.

## 5. Test strategy

There is no `tests/` integration directory — only `tests/fixtures/` plus inline
`#[cfg(test)]` unit tests per module. Golden fixtures freeze bytes, not pretty JSON:
`schema-v1-plan.json` + `.sha256`, `schema-v2-plan.json` + `.sha256`,
`evidence-v2-observation.json` + `.sha256`, and `evidence-v1-digests.json` (8 status +
10 kind content digests). Tests assert byte-for-byte equality and digest equality
(`src/schema.rs:166-193`, `src/evidence.rs:371-435`), digest sensitivity and tamper
rejection (`src/evidence.rs:476-497`), strict-schema rejection (§3 refs), per-status
determinism, stale/dirty/untrusted rejection, cardinality and human-judgment policy,
supersession cycles, and v2-binding enforcement for all five execution kinds on both
the plan side and the observation side (`src/model.rs:551-598`,
`src/evidence.rs:457-472`). The separate `src/assessment.rs:430-1146` range covers
exact-binding matching, the any/all cardinality exclusion of mismatched
observations, both legacy-unbound rejection paths, stale-vs-missing precedence
(`stale_evidence_outranks_missing_or_unavailable_evidence`), and a large-plan timing
guard (`large_plan_assessment_scans_observations_by_kind`), not the five-kind loop.
Added after the review below: `closure.rs` covers supersession reason bounds, the
revision-overflow guard, and closure-shape unknown-field rejection, and `schema.rs`
covers the verification byte bound
(`src/closure.rs:490-623`, `src/schema.rs:240-266`).

## 6. Review findings

Strengths: tiny dependency surface; `forbid(unsafe_code)`; Unicode-scalar (not byte)
bounds with NUL/empty rejection; terminal-state matrices with tests; exact-subject
equality including dirty-digest sensitivity; deterministic ID-sorted assessment;
append-only supersession with self-link/dangling/cycle/duplicate/conflict fail-closed
(`src/closure.rs:98-141`); production code is essentially panic-free — the audit found
`unwrap`/`expect` only inside `#[cfg(test)]` with one exception below.

Gaps / risks / surprises:

1. **`model.rs:216` is the sole production `expect`** — `AcceptanceCriterion::validate`
   serializes each requirement to detect exact duplicates via
   `serde_json::to_string(req).expect(...)`. Infallible in practice but turns a
   `Result`-returning validator into a theoretical panic path; returning
   `ValidationError::Invalid` would keep validation total.
2. **Closure shapes use `String` errors** (`src/closure.rs:29-291`: `Result<_, String>`
   throughout) while the rest of the crate has typed `IdError` / `ValidationError` /
   `EvidenceError` / `GraphError` — repository callers cannot match on failure kinds.
   Related: `ClosureCandidate` carries no self `content_digest` (supersession records
   and `ClosureRecord` do); candidate integrity rests on the record digest covering it.
3. **Precedence asymmetry is subtle**: within one requirement Inconclusive outranks
   NotRun/Skipped/Unavailable (`src/assessment.rs:354-365`), but across requirements
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
6. **`MAX_EXTENSION_VALUE_CHARS` (`src/lib.rs:62`) is referenced nowhere in the
   workspace** — a dead constant. Provenance-shaped values are bounded instead by the
   near-identical twin `PROVENANCE_CHARS` (`src/lib.rs:52`), so the intended bound for
   extension values is either unenforced or silently the provenance one.
7. **`MAX_OBSERVATIONS_PER_PLAN` is declared but not enforced here** — core publishes the
   cap (`src/lib.rs:67`) but only `eggplan-repo` applies it
   (`eggplan-repo/src/store.rs:974,1083`). `assess_plan` is therefore
   observation-count-unbounded, so a caller that assembles observations in memory pays
   unbounded sort/fold cost. Declaring a bound in the domain crate that the domain
   function ignores is a live trap for future embedders.
8. **The four assessment output types deserialize without `deny_unknown_fields`**
   (originally `src/assessment.rs:48-80`: `RequirementAssessment`, `CriterionAssessment`,
   `ItemAssessment`, `PlanAssessment`) — inconsistent with the fail-closed posture every
   other persisted type in the crate takes. **This entry's original verdict was wrong and
   is corrected here.** It was recorded as "latent only, because no in-workspace path
   parses an assessment from bytes". That premise does not hold: `PlanAssessment` is
   embedded in `ClosureCandidate`, and `eggplan-repo` deserializes stored records with a
   bare `serde_json::from_slice::<ClosureRecord>` (`eggplan-repo/src/store.rs:257,290,782`),
   so an assessment *is* parsed from persisted bytes on the live closure-reopen path.
   Worse, `ClosureRecord::validate` recomputes the digest over known fields only, so an
   injected unknown key did not perturb it and a tampered `closure.json` still validated —
   contradicting the fail-closed rule in [core](core.md). Fixed: all four structs now carry
   `#[serde(deny_unknown_fields)]`, pinned by
   `closure_shapes_reject_unknown_assessment_fields` (`src/closure.rs:568-623`), which
   injects an unknown key at each nested level of a serialized `ClosureRecord` and asserts
   deserialization fails. Note the reasoning gap behind the original verdict: serde does not
   inherit `deny_unknown_fields` from a parent struct into nested types, so the attribute on
   `ClosureCandidate`/`ClosureRecord` never protected the assessment inside them.
9. **`ClosureCandidate::build` copies supersession digests without validating them**
   (`src/closure.rs:192-196` maps straight to `content_digest` with no `validate()`
   call, unlike the satisfying-observation path above it). The repository compensates by
   re-validating before trusting the candidate
   (`eggplan-repo/src/store.rs:405,451-456,776-784`), so this is defence-in-depth
   resting entirely on the caller rather than on core.

Fixes applied after this review, each with an in-crate regression test: the stale branch
now sets `invalid` so documented precedence holds (§4); assessment is indexed by evidence
kind once per call instead of rescanning per requirement (`src/assessment.rs:120-130`);
supersession `reason` is bounded text that rejects empty and NUL through a shared
`valid_reason` check and the published `bounds::SUPERSESSION_REASON_CHARS`
(`src/closure.rs:34-38,48,77`); `ClosureRecord::finalize`/`validate` compute
`source_revision + 1` with `checked_add` (`src/closure.rs:236,268`), so an unbounded plan
revision yields the caller's error instead of a debug panic or a release wrap to 0; and
`verification_digest` consults its 64 KiB bound from the structural walk before
serializing (`src/schema.rs:87-94`). Its rejection message and the digest bytes for
in-bounds payloads are unchanged; the new bound check is a strict subset of the old
post-serialization rejection, so no input changes verdict.

No substantive inconsistencies with [evidence](evidence.md) were found; the code matches
it, including the v1/v2 binding rules and the "candidate builds, repository finalizes"
authority split. One documentation gap does exist in [core](core.md) itself: its bounds
table omits all five observation-side caps and the dead `MAX_EXTENSION_VALUE_CHARS`, so a
reader sizing a payload from `core.md` alone would miss them. Still reported rather than
edited — those caps are outside this deep dive's ownership. (The supersession-reason bound
was the one exception: the audit's L1 finding required publishing it, so `core.md` now
lists `SUPERSESSION_REASON_CHARS`.)

## 7. Verification pointers

```sh
cargo test -p eggplan-core
cargo test -p eggplan-core assessment
bash scripts/check-core-boundary.sh
```
