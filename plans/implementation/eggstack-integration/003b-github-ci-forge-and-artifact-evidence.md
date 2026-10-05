# Eggstack Integrations M003b — GitHub CI, Forge, and Artifact Evidence

Status: closed

Repository baseline: 71904570e908a15c099f9b2804cabfbccf9ae51a

Source roadmap:

- plans/subsystems/eggstack-integration-roadmap.md
- plans/002-long-term-roadmap.md Eggstack integrations M003

Predecessors:

- Eggstack integrations M001 provider SPI — closed
- Eggstack integrations M002 Eggwork/Eggsearch — closed

Parallel sibling plan:

- plans/implementation/eggstack-integration/003a-eggbench-verified-bundle-and-comparison-evidence.md

Primary class: integration / forge evidence / authority preservation

## 1. Objective

Add a bounded GitHub forge evidence adapter that normalizes host-acquired
Actions, Checks/status, revision, and artifact facts into Eggplan observations.

The adapter is an interpretation boundary only. It does not contain a GitHub
client, credential handling, polling, webhook handling, workflow dispatch, log
download, or artifact download.

Initial implementation targets GitHub because it is already the hosted
qualification authority used by Eggplan and sibling Eggstack repositories.
The DTO design should remain forge-neutral enough that a later GitLab/other
adapter can be added without changing eggplan-core.

## 2. Ownership and trust boundary

The host owns:

- GitHub API authentication;
- repository access;
- API version selection;
- pagination;
- workflow/check discovery;
- artifact retrieval;
- retry/rate-limit handling;
- attestation retrieval or verification.

`eggplan-integrations` owns only strict bounded DTO decoding and normalized
observation construction.

No adapter may call `ProviderRegistry::register_trusted`. Provider enrollment
remains an explicit host policy action.

Update `scripts/check-integrations-boundary.sh` to cover every new forge
module and reject HTTP/runtime/process/credential dependencies.

## 3. Provider split

Do not use one excessively broad provider identity for every GitHub fact.

Recommended fixed providers:

- `epp_github_actions`
  - class: `github-actions-evidence-v1`
  - kinds: Test, Command, DelegatedRun, Benchmark as explicitly requested by
    the host context
- `epp_github_forge`
  - class: `github-forge-evidence-v1`
  - kinds: Revision, Artifact

A host may enroll either, both, or neither.

Do not permit payload text to choose provider ID or widen the descriptor's
allowed kinds.

## 4. Workflow run DTO

Define a strict `GitHubActionsRunV1` containing only bounded stable facts:

- schema version;
- GitHub repository numeric/node identity and stable repository name;
- workflow ID and bounded workflow path/name;
- run ID;
- run attempt;
- event;
- head SHA;
- run status;
- optional conclusion;
- bounded job summaries;
- created/started/completed timestamps when supplied;
- optional canonical run handle for operator navigation.

Each job summary may retain job ID, bounded job name, head SHA when supplied,
status, conclusion, and bounded step conclusion counts. Do not retain logs,
annotations, environment dumps, secrets, token-bearing URLs, or arbitrary
failure prose.

`run_attempt` is part of execution identity. A rerun must not be silently
collapsed into the prior attempt.

## 5. Check/status DTOs

Define explicit bounded DTOs for forge-native commit checks rather than
reinterpreting arbitrary JSON.

Support at minimum:

- check-run/check-suite style status + conclusion facts;
- classic combined/individual commit status context + state.

Retain exact commit SHA and stable check/status identity.

Unknown status/conclusion values fail closed.

Do not infer that a commit is tested merely because a branch name matches.

## 6. Status mapping

Normalize conservatively.

### Actions/check execution state

- queued/requested/waiting/pending/in_progress -> `InProgress`
- success -> `Passed`
- failure/timed_out/startup_failure -> `Failed`
- cancelled/skipped -> `Skipped`
- action_required -> `Blocked`
- neutral/stale -> `Inconclusive`

### Classic commit status

- pending -> `InProgress`
- success -> `Passed`
- failure/error -> `Failed`

If GitHub introduces an unknown interpretation-changing value, reject the DTO
until the adapter contract is updated.

## 7. Exact subject binding

Execution-derived CI evidence must be revision exact.

For a Git Eggplan subject:

- GitHub `head_sha` must equal the observation context's Git revision for
  Test/Command/DelegatedRun/Benchmark evidence;
- mismatch is an adapter error, not stale-but-passing evidence;
- repository identity in the host DTO must match the repository the host
  associates with the Eggplan subject.

Pull-request workflows frequently test a synthetic merge commit. M003b must not
translate that run into proof for a different source commit by guessing PR
semantics. Record it only for the exact tested SHA unless a future explicit
subject-translation contract is designed.

Revision/Artifact evidence may use the same exact-SHA provenance but does not
gain execution authority merely because it originated from GitHub.

## 8. Verification binding

All execution-derived kinds remain governed by Eggplan evidence schema v2.

The host supplies the expected `VerificationDigest`; the GitHub adapter must
not derive it from workflow ID/path, run ID, job ID/name, commit SHA,
command-looking text, or branch protection context.

This keeps "the workflow ran" distinct from "the workflow ran the verification
spec required by this criterion."

## 9. Artifact DTO and references

Normalize GitHub Actions artifact metadata without downloading artifact bodies.

Retain:

- artifact ID;
- stable artifact name;
- repository/run/run-attempt identity;
- producing head SHA;
- byte size;
- expiry state/time when supplied;
- exact SHA-256 digest when GitHub supplies one.

Recommended durable handle:

    github-actions:artifact:<repository-id>:<artifact-id>

Attach `sha256:<64 lowercase hex>` to `ArtifactRef.digest`.

Do not persist signed/temporary download URLs.

Artifact expiry does not retroactively turn the historical workflow result into
failure. For a current `Artifact` availability claim, an already-expired
artifact maps to `Unavailable`; the CI execution observation remains whatever
its native terminal result was.

## 10. Revision evidence

Support a narrow forge revision observation for facts such as an exact GitHub
commit identity associated with a repository.

This is provenance/identity evidence, not proof that the source is reviewed,
safe, merged, or release-worthy.

Avoid importing mutable branch/tag names as revision identity.

## 11. Explicit attestation exclusion

GitHub artifact attestations, Sigstore verification, in-toto predicates, and
SLSA provenance belong to Interoperability/Distribution M001.

M003b may preserve the artifact SHA-256 and producer/run identity required by
that future work. It must not:

- emit `EvidenceKind::Attestation`;
- treat "an attestation record exists" as verified authenticity;
- implement Sigstore verification;
- equate valid provenance with criterion satisfaction.

## 12. Bounded compatibility fixtures

Freeze minimal host DTO fixtures for:

1. successful workflow;
2. failed workflow;
3. in-progress workflow;
4. cancelled workflow;
5. neutral/stale check;
6. action-required check;
7. classic success/failure/pending statuses;
8. successful rerun with `run_attempt > 1`;
9. artifact with SHA-256;
10. expired artifact;
11. exact-subject mismatch;
12. PR merge-SHA mismatch.

Fixture provenance should cite the API contract/version reviewed, but tests
must use local serialized DTOs and perform no GitHub network access.

## 13. Negative/security tests

At minimum:

- caller cannot select provider identity;
- unknown DTO version rejected;
- unknown GitHub enum rejected;
- malformed SHA or digest rejected;
- wrong head SHA rejected for execution evidence;
- rerun attempts remain distinct;
- expired artifact cannot report current Artifact Passed;
- artifact digest does not become a verification digest;
- workflow success without verification binding cannot satisfy execution
  evidence schema v2;
- temporary URLs/log text are not serialized;
- GitHub attestation-looking payload is ignored/rejected by this adapter;
- no adapter code enrolls trust or performs network/process access.

## 14. Documentation

Add `architecture/github-forge-adapter.md` with the host acquisition boundary,
fixed providers and allowed kinds, Actions/check/status mapping, exact-SHA
rules, rerun identity, artifact handle/digest semantics, expiry semantics, and
attestation non-goal.

## 15. Ordered work packages

### WP1 — Strict forge DTOs and provider descriptors

Freeze Actions/check/status/artifact DTOs, bounds, and fixed provider IDs.

### WP2 — Actions and check normalization

Implement exact-subject status normalization and verification-binding handoff.

### WP3 — Artifact and revision normalization

Add opaque artifact references/digests, expiry semantics, and narrow revision
facts.

### WP4 — Authority hardening

Add rerun, merge-SHA, unknown-enum, no-attestation, and no-trust-enrollment
regressions plus boundary guard updates.

### WP5 — Documentation and hosted qualification

Document contracts and close with native/MSRV hosted evidence.

## 16. Required verification

    cargo fmt --all -- --check
    cargo check --workspace --all-targets --locked
    cargo clippy --workspace --all-targets --locked -- -D warnings
    cargo test --workspace --locked
    cargo +1.89.0 check --workspace --all-targets --locked
    cargo +1.89.0 test --workspace --locked
    bash scripts/check-integrations-boundary.sh
    bash scripts/check-core-boundary.sh
    git diff --check

No qualification test may require live GitHub API credentials.

## 17. Acceptance criteria

M003b closes when:

1. GitHub Actions and forge facts normalize through strict bounded local DTOs.
2. no GitHub client/runtime/credential ownership enters Eggplan.
3. successful CI is exact-SHA and verification-bound before it can satisfy an
   execution criterion.
4. rerun attempts cannot alias prior execution.
5. native failure/cancellation/in-progress/blocking states remain observable.
6. artifact SHA-256 is preserved without embedding or downloading artifact
   bodies.
7. artifact expiry is represented without rewriting historical CI outcomes.
8. attestation verification remains deferred and cannot be forged through the
   adapter.
9. provider trust remains host-controlled.
10. native/MSRV hosted qualification is green.

## 18. Stop conditions

Stop and record a corrective/design decision if exact tested revision cannot be
determined from host facts; GitHub semantics require branch-name inference;
verification authority would need to be inferred from workflow names; artifact
identity requires credential-bearing URLs; implementation needs
HTTP/async/process dependencies in `eggplan-integrations`; or attestation
verification becomes necessary to complete ordinary CI evidence.

## 19. Closure evidence

Record the DTO schema/enum compatibility matrix; provider descriptors;
exact-SHA/verification negative tests; rerun/PR-merge regressions; artifact
digest/expiry tests; boundary guard proof; implementation SHA; native/MSRV
hosted workflow IDs; and residual findings/Interoperability M001 handoff notes.
