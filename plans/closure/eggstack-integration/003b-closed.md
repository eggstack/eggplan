# Eggstack Integrations M003b — GitHub CI, Forge, and Artifact Evidence — Closed

Status: closed

Source implementation plan:

- plans/implementation/eggstack-integration/003b-github-ci-forge-and-artifact-evidence.md

Source roadmap:

- plans/subsystems/eggstack-integration-roadmap.md

Reviewed Eggplan baseline: `71904570e908a15c099f9b2804cabfbccf9ae51a`

Reviewed upstream interface: GitHub REST API version `2022-11-28`.

Implementation commit:

- `93d65b0329487555edc4ccb6449c567e45ad5a1a` — two bounded GitHub forge
  providers, compatibility fixtures, architecture documentation.

Hosted qualification: GitHub Actions run
[37351818356](https://github.com/eggstack/eggplan/actions/runs/37351818356)
passed on the exact implementation commit.

- Linux job `111904256558`: format, check, clippy, tests, and all five boundary
  guards.
- macOS job `111904256725`: check, clippy, tests, and all five boundary guards.
- Windows job `111904256481`: check, clippy, tests, and all five boundary guards.
- Rust 1.89 job `111904256324`: workspace check and tests.

## Executive finding

Eggplan now normalizes host-acquired GitHub Actions, check, classic commit
status, artifact, and commit-identity facts through two bounded providers with
no client, transport, credential handling, or acquisition logic anywhere in the
crate. GitHub is the first forge target because it is already Eggplan's hosted
qualification authority; the core layer is untouched and another forge adapter
can be added beside this one.

Three claims stay separate and are each tested as such: execution outcome,
artifact availability, and commit identity. The most important consequence is
that revision exactness is enforced rather than assumed — a pull-request
merge-commit run is not evidence about the underlying source commit, and a
dirty worktree is never described as tested by a clean CI run.

## Contract qualification method

There is no upstream source repository to pin here; the interface is GitHub's own
published REST contract. Two independent sources were used and cross-checked:

1. Live API responses for this repository, via authenticated `gh api` against
   run `37350383159` and its commit `674264db7dda3f063cbbedd93f6f4938c1959a11`.
2. GitHub's official OpenAPI description
   (`github/rest-api-description`, `api.github.com.json`), which yields the
   authoritative field list, nullability, and enum spellings.

This mattered: the plan assumed GitHub might not supply a per-artifact digest.
It does. `artifact.digest` is documented as "The SHA256 digest of the artifact.
This field will only be populated on artifacts uploaded with upload-artifact v4
or newer. For older versions, this field will be null." The adapter therefore
treats it as an optional exact `sha256:<64 hex>` field, rejects malformed
values, and records absence explicitly without ever inferring one. This was
resolved by checking rather than assuming, and it removes any need for the host
to download a payload.

The five DTO shapes, the enum vocabularies, and the observation shapes are
recorded in `architecture/github-adapter.md`.

## Requirement-to-evidence matrix

| Plan requirement | Evidence |
|---|---|
| §1 narrow, provider-agnostic seam | `provider_identities_and_allowed_kinds_are_fixed` fixes `epp_github_actions` and `epp_github_forge` with disjoint kinds. Five independent DTOs in `src/github.rs`. |
| §2 CI/check/status/artifact/revision DTOs | `GitHubActionsRunV1`, `GitHubCheckRunV1`, `GitHubCommitStatusV1`, `GitHubArtifactV1`, `GitHubRevisionV1`; each is exercised across 32 fixture cases recorded per family in `tests/fixtures/manifest.json`. |
| §3 provider identity and kind allowlists | `provider_identities_and_allowed_kinds_are_fixed` asserts classes, exact allowed-kind sets, and that neither descriptor allows `Attestation`. `unsupported_kinds_are_rejected_in_both_directions` asserts the cross-product is rejected. |
| §4 one observation per Actions run or check | `normalize_run` and `normalize_check_run` each produce exactly one observation; the fact that an artifact is not folded into the run observation is asserted in `artifact_availability_is_a_separate_claim_from_execution`. |
| §5 run number, attempt, event, run URL, status, conclusion, timestamps | Retained as `run_number`, `run_attempt`, `event`, bounded `run_handle`, `native_status`, `native_conclusion`, and unix-ms timestamps. `run_attempt_is_part_of_execution_identity` proves attempt is execution identity and never collapses a rerun into the prior attempt. |
| §6 conservative native status mapping | `workflow_run_conclusions_map_without_inventing_authority` covers all nine run conclusions; `check_run_conclusions_map_without_inventing_authority` covers all six check conclusions; `classic_commit_status_aggregates_conservatively` covers all four classic states plus the empty list. `neutral_and_stale_are_never_success` and `invalid_and_inconclusive`-style assertions pin the non-passing cases. |
| §7 exact tested commit and clean subject | `ci_evidence_for_a_different_commit_is_rejected` rejects a merge-SHA run, a merge-SHA check, a merge-SHA status, and a dirty subject; `a_job_cannot_disagree_with_its_run_about_the_tested_commit` rejects a per-job disagreement. A non-Git subject is deliberately not compared. |
| §8 append-only fact handling | No mutation path exists. The module has no write API, and `provider_identities_and_allowed_kinds_are_fixed` shows the forge provider cannot carry verification binding. |
| §9 artifact naming, digest, expiry, retention | `artifact_availability_is_a_separate_claim_from_execution` covers `Passed` and `Unavailable`; `an_absent_artifact_digest_is_recorded_as_absent_never_inferred` covers present, absent, and five malformed digest shapes. |
| §10 narrow forge revision observation | `revision_provenance_is_narrow_and_not_a_verdict` and `revision_identity_rejects_mutable_branch_or_tag_shapes`. |
| §11 host-supplied provider policy | No policy is read or implied. `execution_evidence_requires_host_verification_binding` proves the host digest is required and never re-derived. |
| §12 documented native failure classes | Each native conclusion retains its exact value in `native_conclusion`, and each maps to a distinct normalized status asserted by the mapping tests, so no native failure class is silently downgraded. |
| §13 typed errors, bounded URLs, trust | `unknown_schemas_versions_and_fields_fail_closed`, `identity_fields_are_validated`, `workflow_paths_and_navigation_handles_are_confined`, `duplicate_identities_are_rejected`, `the_adapter_never_enrolls_trust_or_acquires_evidence`, `a_github_attestation_payload_is_rejected_not_accepted_as_provenance`. |
| §14 docs | `architecture/github-adapter.md`. |
| §15 characterization | §16-closure below. |
| §16 required verification | Section below; all commands run locally and hosted. |
| §17 acceptance criteria | Criteria 1-10 covered by the rows above and the residual findings section. |
| §19 deferred | `architecture/github-adapter.md` "Attestation exclusion". |

## Status-mapping matrix recorded

| Native conclusion | Eggplan status |
|---|---|
| no conclusion while non-terminal | `InProgress` |
| `success` | `Passed` |
| `failure`, `timed_out`, `startup_failure` | `Failed` |
| `cancelled`, `skipped` | `Skipped` |
| `action_required` | `Blocked` |
| `neutral`, `stale` | `Inconclusive` |

| Classic state | Eggplan status |
|---|---|
| `pending` | `InProgress` |
| `success` | `Passed` |
| `failure`, `error` | `Failed` |
| empty status list | `Inconclusive` |

| Forge availability | Eggplan status |
|---|---|
| artifact not expired | `Passed` |
| artifact expired | `Unavailable` |
| exact commit exists at repository | `Passed` (identity only) |

## Exact local verification

All commands ran on Linux at implementation commit
`93d65b0329487555edc4ccb6449c567e45ad5a1a` and passed:

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked                      # 212 passed
cargo +1.89.0 check --workspace --all-targets --locked
cargo +1.89.0 test --workspace --locked              # 212 passed
bash scripts/check-core-boundary.sh
bash scripts/check-codegg-compat-boundary.sh
bash scripts/check-integrations-boundary.sh
bash scripts/check-projection-cli-boundary.sh
bash scripts/check-closure-authority-boundary.sh
git diff --check
```

The GitHub adapter contributes 25 conformance tests in
`crates/eggplan-integrations/tests/github_conformance.rs`.

## Invariant review

- Eggplan remains a planning/evidence mechanism. Authentication, API version
  selection, pagination, rate limits, and every other forge interaction stay
  host-owned.
- Execution outcome, artifact availability, and commit identity are three
  separate observations with separate kinds. None implies review, safety, merge
  state, or criterion satisfaction.
- An expired artifact does not retroactively turn a successful run into a
  failure, and a successful run does not make an expired artifact available.
- `neutral` and `stale` are never success.
- A pull-request merge-commit run is not evidence about the source commit; a
  dirty worktree is never described as tested.
- `run_attempt` is part of execution identity, so a rerun is never collapsed
  into the prior attempt.
- Artifact availability is forge authority, not run authority. An artifact
  assertion is about availability of that artifact reference only.
- Revision provenance asserts commit existence and nothing else.
- Attestation remains deferred to Interoperability M001; no `Attestation` kind is
  emitted and a real Sigstore payload is rejected by every parser.
- The forge provider's `verification_binding: false` is enforced, so an artifact
  digest cannot be replayed as a verification digest.
- Execution-derived evidence requires the host `VerificationDigest`; the adapter
  never derives one.

## Security, trust, and identity review

- No URL is persisted. `html_url`, `details_url`, `url`, `archive_download_url`,
  and `logs_url` are structurally absent from the DTOs. The navigation handle
  rejects `://`, `@`, `?`, `#`, backslash, whitespace, and control characters,
  so a signed or temporary link cannot enter it.
- `archive_download_url` and `Authorization`/`Bearer` are additionally asserted
  absent from the module source by test.
- Commit SHAs must be 40 lowercase hex characters, so mutable branch and tag
  names cannot be substituted for revision identity.
- Digest syntax is strict: `sha256:` plus exactly 64 lowercase hex characters.
- Workflow paths must be confined relative paths with no absolute root,
  backslash, or empty/`.`/`..` segment.
- Job ids, status ids, and status contexts must be unique and non-zero, so a
  duplicated ledger entry cannot inflate a count.
- Provider enrollment remains an explicit host `ProviderRegistry` action. The
  adapter returns descriptors and never registers anything.

## Migration and compatibility review

No schema migration was needed. Plan `SCHEMA_VERSION` and
`EVIDENCE_SCHEMA_VERSION` remain 2 and no existing envelope changed. This
milestone adds one module to an existing crate and no new dependency.

The rewritten `scripts/check-integrations-boundary.sh` needed no per-adapter
edit: it scans the whole crate source tree, so the GitHub adapter is covered by
the same six guards that covered Eggbench, including the filesystem, network,
process, credential, and trust-enrollment checks.

## Documentation and operations

- `architecture/github-adapter.md` — new, the complete adapter contract.
- `architecture/deep-dive-integrations.md` — the M003b delta appended to the
  M003a update section.
- `architecture/provider-spi.md` — GitHub fixture-family provenance.
- `architecture/overview.md` — crate role row now lists the GitHub forge adapter.

## Roadmap disposition

Eggstack M003b is closed. With M003a, the Eggstack M003 line is complete: the
subsystem now has four closed adapter-backed milestones (M001, M002, M003a,
M003b). The remaining Eggstack candidates in the roadmap — Eggsact as an
optional deterministic preflight seam, and Eggup for eventual distribution — are
unchanged and remain deferred.

## Registry updates

- M003b row set to `closed` with this closure record.
- Subsystem status line notes M001/M002/M003a/M003b closed.

## Unresolved findings

1. **Informational — artifact fixtures are modelled, not sampled.** The sampled
   repository produced `total_count: 0` artifacts for the referenced run, so no
   real artifact object could be captured. The artifact DTO, its `digest`
   semantics, and its status mapping were qualified from GitHub's published
   OpenAPI schema and REST documentation rather than a live payload. The
   fixture manifest records this explicitly. A future milestone that targets a
   repository with `upload-artifact` v4 artifacts should resample before
   treating the digest path as production-proven.
2. **Informational — repository identity matching is host-owned.** Eggplan's
   `epr_*` repository id is not a GitHub numeric id, so the adapter can verify
   that the tested commit matches the subject revision but cannot verify that the
   repository matches the host's repository mapping. `repository_id` is retained
   so a host can assert the mapping in its own layer. Recorded rather than
   papered over with a second identifier scheme.
3. **Informational — `stale` and `startup_failure` are Actions-surface values
   outside the REST check-run enum.** They are accepted so a host need not
   special-case them, and both map to non-passing statuses. No live sample of
   either was available.
4. **Informational — branch protection, required checks, and mergeability are
   intentionally out of scope.** The adapter does not interpret them. That is
   recorded here so a later milestone does not assume the absence was an
   oversight.

## Residual findings for Interoperability M001

- The bounded forge provenance this milestone preserves — repository identity,
  run/attempt provenance, exact tested commit, artifact digest when GitHub
  supplies one, and artifact expiry — is the input a future attestation verifier
  will need. M003b deliberately performed no Sigstore, in-toto, or SLSA work.
- Host-side forge components (authenticated client, subject translation,
  repository identity mapping, evidence-chain handling, trusted-root policy) are
  registered separately and remain unimplemented.
- The Eggwork-style subject-translation rule recorded in `plans/registry.md`
  applies here unchanged: Eggplan repository state is canonical, and any forge
  identity translation must be proven by the host rather than inferred here.