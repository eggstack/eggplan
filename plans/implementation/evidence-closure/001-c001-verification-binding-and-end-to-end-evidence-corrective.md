# Evidence M001 C001 — Verification Binding and End-to-End Evidence Corrective

Status: active

Repository baseline: ff5f60c (Foundation M003 closure and C001 handoff).

Source roadmap:

- plans/subsystems/evidence-closure-roadmap.md

Predecessor implementation and closure:

- plans/implementation/evidence-closure/001-evidence-ledger-and-assessment.md
- plans/closure/evidence-closure/001-closed.md
- plans/closure/foundation-core/002-conditionally-closed.md
- Foundation M003 closure when available

Long-term requirements:

- plans/000-long-term-specification.md sections 4.1-4.3, 8-12, 18-20
- plans/001-terminology-and-domain-model.md

Applicable ADRs:

- ADR-0001-planning-evidence-mechanism-not-execution
- ADR-0003-immutable-revision-scoped-evidence

Primary class: corrective invariant / compatibility hardening

## 1. Objective

Correct Evidence M001's overly broad requirement matching.

Today an EvidenceRequirement can match by evidence kind, optional provider,
subject, cardinality, and count. An observation for one Test invocation can
therefore satisfy another Test requirement for the same provider and subject
even when the executed verification was different.

Introduce an explicit verification-spec binding between requirements and
observations, fail closed for unbound execution-derived legacy requirements,
and add the end-to-end regression that proves persisted Eggplan evidence does
not invalidate its own SubjectRevision after Foundation M003.

Also add the evidence architecture document already referenced by the M001
closure record. Historical M001 closure remains unchanged.

## 2. Why this corrective is blocked

Foundation M003 must land first because this corrective depends on:

- Eggplan state not participating in the source dirty fingerprint;
- strict schema-version decoding/migration dispatch foundations;
- current native platform qualification.

Do not implement a new evidence schema against the known self-staleness and
loose-v1 parsing behavior.

## 3. Findings this plan must close

### E-C001-01 — evidence kind/provider/subject is not sufficient identity

A criterion requiring a specific verification must not be satisfiable by an
unrelated observation merely because both are Test, Command, StaticAnalysis,
DelegatedRun, or Benchmark evidence from the same trusted provider.

### E-C001-02 — invocation_ref is provenance text, not a match key

The current optional invocation_ref is bounded descriptive provenance. It is
not canonical enough to compare as verification identity.

The correction must add a dedicated SHA-256 verification-spec digest whose
semantics are explicit.

### E-C001-03 — schema compatibility must not invent proof

Existing schema-v1 requirements and observations have no verification-spec
digest. A compatibility reader may load them, but migration MUST NOT fabricate
a digest or make an old broad requirement appear specifically verified.

### E-C001-04 — end-to-end self-staleness was untested

The repository needs one integrated test spanning Git subject capture,
EvidenceObservation finalization, repository append, subject recapture, and
assessment.

### E-C001-05 — documented evidence architecture file is missing

The M001 closure references architecture/evidence.md, but the file is absent on
the reviewed baseline. Add it rather than editing the historical closure claim.

## 4. Invariants that must not regress

- Provider authority remains host-constructed and cannot be widened by
  serialized observation text.
- EvidenceObservation remains immutable after finalization.
- Observation content digest continues to cover all semantics used for
  satisfaction, including the new verification binding.
- Exact SubjectRevision remains the default applicability rule.
- passed status alone never satisfies a mismatched requirement.
- not_run, skipped, unavailable, failed, blocked, in_progress, and
  inconclusive remain distinct.
- Models may propose verification requirements but cannot manufacture a
  trusted passing observation.
- No executor, scheduler, command runner, or provider-specific SDK enters
  eggplan-core.
- Historical v1 bytes remain readable through an explicit compatibility path;
  they are not silently rewritten.

## 5. Required domain correction

### Verification-spec digest

Add a typed validated digest representing the exact verification specification
the host/provider says was requested/executed.

The digest is opaque to eggplan-core but MUST be formatted as
sha256:<64 lowercase hex>.

The producer of the requirement and the trusted evidence adapter are
responsible for hashing the same bounded canonical verification specification.
Eggplan compares the digest; it does not execute or interpret the command.

Suggested wire semantics:

- EvidenceRequirement gains expected_verification_digest.
- EvidenceObservation gains verification_digest.
- observation canonical content digest includes verification_digest.
- assessment requires equality whenever the requirement is bound.

Names may change if a clearer type-safe API preserves these semantics.

### Kinds requiring binding

For schema-v2, execution-derived evidence kinds MUST require a verification
binding:

- Command
- Test
- StaticAnalysis
- DelegatedRun
- Benchmark

Research may opt into a binding but is not required in this corrective.
Revision, Artifact, Attestation, and HumanJudgment use their existing semantic
identities/policies.

Do not add an escape hatch that lets a v2 execution requirement silently
downgrade to kind-only matching.

## 6. Schema/version compatibility

This corrective changes persisted semantic fields and therefore must not
silently mutate schema v1.

Implement an explicit next-version compatibility path.

### Plan schema

Newly written plans use a new schema version containing bound execution
requirements.

Known schema-v1 plans remain readable. When a v1 execution-derived requirement
lacks a verification binding, assessment must fail closed with a specific
legacy/unbound reason until a caller explicitly upgrades the plan with the
expected verification digest.

Migration must not guess a digest from description text.

### Evidence schema

Newly finalized execution-derived observations use the new evidence schema
version and include verification_digest.

Known schema-v1 observations remain readable as historical evidence. They
cannot satisfy a newly bound execution requirement because they do not contain
the required verification identity.

Do not rewrite finalized v1 observation files during read or migration.

### Golden fixtures

Retain v1 fixtures unchanged and add v2 fixtures/digests. Tests must prove
both versions are intentionally dispatched and malformed/unknown versions fail
closed.

## 7. Assessment changes

Requirement assessment must distinguish at least:

- missing observation;
- stale subject;
- untrusted provider;
- provider mismatch;
- provider kind not allowed;
- unbound legacy execution requirement;
- observation missing required verification binding;
- verification digest mismatch;
- ordinary evidence status outcomes.

A mismatched digest is not a passing observation and must never appear in
satisfying_observation_ids.

Reason ordering remains deterministic.

For an Any requirement, only correctly bound eligible observations participate
in pass counting. For All, mismatched observations outside the requirement's
binding are irrelevant rather than failures for that requirement; correctly
bound failed observations retain current failure semantics.

## 8. End-to-end integration regression

After Foundation M003:

1. create a temporary Git repository;
2. initialize RepositoryStore in its normal managed state root;
3. create a Plan with one completed item and a bound Test requirement;
4. capture SubjectRevision A;
5. create a trusted provider registry;
6. finalize a passed observation for subject A and verification digest X;
7. append it through RepositoryStore;
8. capture SubjectRevision B without changing ordinary source;
9. assert A == B;
10. assess the Plan with X and assert Complete;
11. create or load an otherwise identical passed Test observation with digest Y
    and prove it does not satisfy the X requirement;
12. modify ordinary source, capture SubjectRevision C, and prove the old X
    observation is stale.

This test is a release gate for the corrective.

## 9. Evidence architecture documentation

Create architecture/evidence.md covering:

- requirement versus observation;
- provider trust boundary;
- verification-spec digest semantics;
- v1/v2 compatibility behavior;
- subject matching;
- status normalization;
- cardinality;
- append-only storage;
- assessment precedence/reason codes;
- integrity versus authenticity;
- explicit non-goals.

Correct the root/architecture links if needed, but do not rewrite the M001
historical closure.

## 10. Ordered work packages

### WP1 — Versioned wire model

Add explicit version dispatch and v1 compatibility DTOs as needed. Freeze v1;
add v2 plan/evidence golden fixtures.

### WP2 — Verification binding types and validation

Implement typed digest fields and require them for schema-v2 execution-derived
requirements/observations.

### WP3 — Assessment binding

Update matching and deterministic reason codes. Add positive/mismatch/missing
binding matrices.

### WP4 — Repository compatibility

Read v1 and v2 observations/plans according to the Foundation M003 strict
decoder. Finalized v1 evidence stays immutable.

### WP5 — End-to-end regression and docs

Add the self-staleness/binding integration test and architecture/evidence.md.

## 11. Failure, restart, and contention semantics

A malformed verification digest is invalid input.

A v1 unbound execution requirement is not auto-satisfied and is not upgraded
implicitly.

A v1 observation remains readable after restart but cannot satisfy a v2 bound
requirement.

Repository append idempotency remains byte/canonical-content based. The new
verification digest participates in canonical content, so same observation ID
with a different verification digest conflicts.

No network/process side effect is permitted during assessment.

## 12. Required tests

At minimum:

- v1 plan fixture still readable;
- v1 evidence fixtures still readable and unchanged;
- v2 plan/evidence golden bytes and digests;
- unknown future versions fail closed;
- execution kinds require v2 binding;
- malformed binding digest rejected;
- exact binding passes;
- same kind/provider/subject with wrong binding does not pass;
- missing binding does not pass;
- v1 unbound requirement reports explicit fail-closed reason;
- v1 observation cannot satisfy bound v2 requirement;
- Any and All semantics with mixed bound/mismatched observations;
- provider and subject checks still precede success;
- end-to-end subject A == B after persisted evidence;
- source modification makes prior observation stale;
- reopen/reassessment gives identical result;
- observation replay/conflict covers changed verification digest.

## 13. Required verification

Closure must record exact commands and native CI run IDs. Expected gates:

    cargo fmt --all -- --check
    cargo check --workspace --all-targets --locked
    cargo clippy --workspace --all-targets --locked -- -D warnings
    cargo test --workspace --locked
    cargo +1.89.0 check --workspace --all-targets --locked
    bash scripts/check-core-boundary.sh
    git diff --check

Use the hosted cross-platform workflow established by Foundation M003.

## 14. Acceptance criteria

C001 closes when:

1. unrelated execution observations cannot satisfy a specifically bound
   requirement.
2. execution-derived schema-v2 requirements and observations carry matching
   verification-spec digests.
3. legacy v1 state remains readable but cannot gain fabricated verification
   authority.
4. v1 fixtures remain unchanged and v2 fixtures are frozen.
5. the integrated persist/recapture/assess regression proves Eggplan evidence
   does not stale itself at the same source HEAD.
6. architecture/evidence.md exists and matches implementation.
7. all required local and hosted CI gates pass.

## 15. Stop conditions

Stop and report if:

- binding requires eggplan-core to interpret or execute provider commands;
- compatibility would require rewriting historical finalized observations;
- a v1 migration can only work by guessing verification identity;
- Foundation M003 has not closed the subject self-reference issue;
- the schema change needs a broader policy language rather than one bounded
  digest identity.

## 16. Closure evidence required

The corrective closure must include:

- predecessor finding matrix;
- v1/v2 schema and fixture inventory;
- proof v1 fixture bytes/digests are unchanged;
- exact verification-binding semantics;
- mismatch/missing/legacy assessment matrix;
- end-to-end self-staleness regression output;
- hosted CI evidence;
- architecture/evidence.md presence;
- disposition of Evidence M002, Projection/CLI M001, CodeGG Integration M001,
  and Eggstack Provider SPI M001.

## 17. Handoff notes

Do not fold ClosureRecord persistence into this corrective. Close the evidence
identity/trust gap first; Evidence M002 can then build durable closure records
on corrected semantics.
