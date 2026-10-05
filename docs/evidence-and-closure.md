# Evidence and closure

This is the part of Eggplan that most often surprises people. Evidence is **not** something
you assert — it is something a trusted provider observed, bound to an exact source state.

## Requirements versus observations

- An **`EvidenceRequirement`** belongs to a plan and describes what evidence a criterion
  needs. It proves nothing.
- An **`EvidenceObservation`** is a finalized, immutable record from a provider.

Requirements do not prove execution. Observations do not enroll their own provider as
trusted.

## A provider ID is a label, not authority

A provider ID inside an observation is just a label. Only the **host-constructed provider
registry** decides what counts. This is an adapter/host trust boundary, not cryptographic
authentication — a content digest proves recorded content was not altered, and nothing more.

A content digest detects changes to recorded semantics. It does **not** authenticate the
producer.

## Subject equality is applicability

Assessment requires **exact** `SubjectRevision` equality. A passing observation for an older
revision, or for a different dirty worktree, is stale history — never current proof.

This is why a passing test run does not automatically close a plan: if the source changed
after the run, the evidence no longer applies to the code you are closing.

## The verification binding

Requirements and observations of execution-derived kinds (Command, Test, StaticAnalysis,
DelegatedRun, Benchmark) carry a `VerificationDigest` — `sha256:<64 lowercase hex>` over a
bounded canonical verification specification.

The plan author and the trusted adapter must hash the **same** specification. Eggplan
treats the digest as opaque: it does not interpret or run a command, and `invocation_ref`
stays descriptive provenance rather than a matching key.

Legacy v1 execution evidence without a binding is reported as
`legacy_unbound_execution_requirement` and **cannot satisfy a criterion**. Eggplan never
guesses a digest from prose; a plan is upgraded to v2 only by explicitly supplying the
expected digest.

## Cardinality

- **`Any`** counts only eligible observations carrying the exact required binding.
- **`All`** considers only observations eligible for that binding. A mismatched observation
  belongs to a different verification and is irrelevant to this requirement.

The minimum count applies to the eligible set.

## Statuses

Normalized evidence statuses are Passed, Failed, InProgress, NotRun, Skipped, Blocked,
Unavailable, and Inconclusive. They stay distinct during assessment.

Before a status can contribute to satisfaction, Eggplan checks provider trust and kind,
optional provider match, human-judgment policy, observation integrity, subject equality,
and verification binding. Each failure mode has its own reason code.

## Corrections are append-only

Evidence is corrected through **supersession**: an append-only, digest-protected link to a
replacement observation. The effective assessment view contains only terminal observations
that have not been superseded; superseded files remain on disk for audit.

Historical observations are never rewritten to make a later assessment pass.

## Closing a plan

```sh
cargo run -p eggplan-cli -- close ep_example --state-root .eggplan \
  --expected-revision 7 --provider-policy policy.json
```

Closure is not a status flag you set. It goes through the repository's **guarded finalizer**,
which:

1. Re-reads the plan and requires the candidate's exact source revision and digest.
2. Captures the current Git subject and requires it to equal the candidate's.
3. Re-runs assessment itself and requires it to match the candidate exactly.
4. Captures the subject a **second time** immediately before writing, and aborts if it moved.
5. Writes the closure record, advances the plan to `Closed`, and promotes the record
   atomically.

Ordinary compare-and-swap **cannot** enter `Closed`, and cannot modify an already-`Closed`
plan at all. If either check fails, no closure state is written.

Subject mismatch, drift between captures, and capture failure are reported as distinct,
stable diagnostics: `closure_subject_changed`, `closure_subject_drifted`,
`closure_subject_unavailable`.

A crash mid-write leaves a pending record; repository open either discards it (plan still
current) or promotes it (exact target plan present). A `Closed` plan with no matching
closure record is treated as **corruption**.

## What is deliberately out of scope

Eggplan does not execute verification, schedule work, fetch evidence, define a general
policy language, authenticate signatures, or persist model reasoning. It records and
assesses; other systems acquire and run.
