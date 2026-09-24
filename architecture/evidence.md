# Evidence and assessment architecture

## Requirements and observations

An `EvidenceRequirement` is part of a plan and describes evidence needed for
an acceptance criterion. An `EvidenceObservation` is a finalized immutable
record from a provider. Requirements do not prove execution; observations do
not enroll their own provider as trusted.

The host builds a provider registry that maps provider IDs to provider class
and allowed evidence kinds. Assessment accepts observations only from that
registry. A content digest detects changes to recorded semantics, but provides
integrity only and does not authenticate the producer.

## Verification identity

For schema v2, Command, Test, StaticAnalysis, DelegatedRun, and Benchmark
requirements and observations carry a `VerificationDigest` in the format
`sha256:<64 lowercase hexadecimal characters>`. The plan author and trusted
provider adapter must hash the same bounded canonical verification
specification. Eggplan treats the digest as opaque: it does not interpret or
run a command, and `invocation_ref` remains descriptive provenance rather than
a matching key.

Research observations may optionally carry a binding. Revision, Artifact,
Attestation, and HumanJudgment retain their existing semantic identity and
policy. The observation content digest includes the verification digest, so
reusing an observation ID with changed verification identity conflicts in the
repository ledger.

## Compatibility and subject applicability

Schema-v1 plans and observations remain readable with their original canonical
bytes and content digests. A v1 execution requirement without a binding is
legacy evidence policy and assessment reports
`legacy_unbound_execution_requirement`; it cannot satisfy a criterion. A
caller can explicitly upgrade a plan to v2 by supplying the expected digest.
Eggplan never guesses one from prose. V1 observations remain unchanged and
cannot satisfy a bound v2 execution requirement.

Assessment uses exact `SubjectRevision` equality by default. A passing
observation for an older or differently dirty source tree is stale historical
evidence, not current proof.

## Status and cardinality

The normalized statuses are Passed, Failed, InProgress, NotRun, Skipped,
Blocked, Unavailable, and Inconclusive. They remain distinct during
assessment. `Any` counts only eligible observations with the exact required
verification binding. `All` considers only observations eligible for that
binding; a mismatched observation belongs to another verification and is
irrelevant to this requirement. The minimum count applies to the eligible set.

Assessment checks provider trust and kind, optional provider match, human
judgment policy, observation integrity, subject equality, and verification
binding before status can contribute to satisfaction. Reason ordering and
observation ordering are deterministic. Missing, stale, untrusted, mismatched,
legacy-unbound, and ordinary status outcomes have separate reason codes.

Observations are append-only in repository storage. Replaying identical
canonical content is idempotent; reusing an ID for different content conflicts.
Assessment is pure and performs no network or process side effects.

## Supersession and closure

Evidence corrections are represented by append-only, digest-protected
`EvidenceSupersessionRecord` links. Assessment's effective view contains only
terminal observations that have not been superseded; old observation files
remain available for audit. Self-links, dangling references, cycles, duplicate
successors, and corrupt digests fail closed.

A `ClosureCandidate` snapshots one exact Plan revision, subject, assessment,
provider-policy digest, satisfying observation digests, and supersession
lineage. `RepositoryStore::finalize_closure` reloads and reassesses these
inputs under the repository lock, recaptures the Git `SubjectRevision`
internally, requires equality with the candidate subject, and recaptures the
subject a second time immediately before writing the pending closure record.
The finalizer, not the caller, owns current-subject authority. A stale
candidate subject before the first capture aborts with a typed failure; a
drift between the two captures aborts with a typed failure; neither produces
pending or final closure state. On the second successful capture the finalizer
writes the pending closure record, replaces the Plan with its next Closed
revision, and promotes the record. Repository open discards a pending
candidate when the source Plan is still current and promotes it when the
exact target Plan is present. A Closed Plan without a matching record is
corruption. Ordinary Plan CAS rejects transitions to Closed.

Eggplan does not lock arbitrary external Git/worktree writers. After the
finalizer's pre-write recapture, any subsequent source-tree change leaves the
already-finalized closure record structurally valid; current-state surfaces
may report that closure as stale relative to the later worktree, but that is
distinct from closure-record corruption.

## Non-goals

Eggplan does not execute verification, schedule work, fetch evidence, define a
general policy language, authenticate signatures, or persist model reasoning.
