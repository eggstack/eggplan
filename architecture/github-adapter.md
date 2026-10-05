# GitHub CI, forge, and artifact evidence adapter

`eggplan_integrations::github` normalizes host-acquired GitHub Actions, check,
classic commit status, artifact, and commit identity facts into Eggplan
observations.

It contains no HTTP client, no async runtime, no process execution, no
credential handling, no polling, no webhook, no workflow dispatch, and no log or
artifact download. The host owns every interaction with GitHub and supplies
bounded local DTOs. GitHub is the first forge target because it is already
Eggplan's hosted qualification authority; nothing here is GitHub-specific at the
core layer, and a GitLab or other forge adapter can be added beside this one.

Reviewed API contract: REST API version `2022-11-28`.

## Providers

| | Actions provider | Forge provider |
|---|---|---|
| Provider ID | `epp_github_actions` | `epp_github_forge` |
| SPI class | `Execution` | `Utility` |
| Core class | `execution` | `utility` |
| Allowed kinds | `Test`, `Command`, `DelegatedRun`, `Benchmark` | `Revision`, `Artifact` |
| `supports_in_progress` | yes | no |
| `artifacts` | yes | yes |
| `verification_binding` | yes | **no** |
| `research_trust_metadata` | no | no |

Provider identity is adapter-fixed. Payload text cannot select a different
provider or widen the allowed kinds. `Attestation` is absent from both
descriptors; attestation verification remains Interoperability M001.

The forge provider's `verification_binding: false` is load-bearing, not
metadata. It means an `Artifact` or `Revision` observation carrying a
`VerificationDigest` is rejected with `VerificationBindingNotSupported` rather
than silently ignoring the field. That is what makes it impossible to replay an
artifact digest as a verification digest.

## Three claims kept apart

1. **Execution outcome** — a run, check, or classic status ran and its native
   terminal result.
2. **Artifact availability** — a produced artifact is currently retrievable.
3. **Commit identity** — an exact commit exists at a repository.

None of them implies review, safety, merge state, or criterion satisfaction. In
particular, an expired artifact does not retroactively turn a successful run
into a failure, and a successful run does not make an expired artifact available.

## Bounded DTOs

Five independent DTOs, each schema version 1, each `deny_unknown_fields`, each
capped at 256 KiB:

| DTO | Upstream source |
|---|---|
| `GitHubActionsRunV1` | `GET /repos/{o}/{r}/actions/runs/{run_id}` |
| `GitHubCheckRunV1` | `GET /repos/{o}/{r}/commits/{ref}/check-runs` (one entry) |
| `GitHubCommitStatusV1` | `GET /repos/{o}/{r}/commits/{ref}/status` |
| `GitHubArtifactV1` | `GET /repos/{o}/{r}/actions/runs/{run_id}/artifacts` (one entry) |
| `GitHubRevisionV1` | host-resolved exact commit identity |

Timestamps are supplied by the host as bounded unix milliseconds rather than
raw RFC 3339 strings, matching how the other adapters bound time.

Retained per run: repository id and full name, workflow id/path/name, run id,
run number, run attempt, event, exact `head_sha`, native status and conclusion,
bounded job summaries, created/started/completed times, and an optional
navigation handle. Per job: job id, name, optional head SHA, status, conclusion,
and per-conclusion **step counts**.

Not retained: log text, annotations, `runner_name`, `runner_id`, step names or
output, `html_url`, `details_url`, `url`, `archive_download_url`, `logs_url`,
commit messages, author identity, `display_title`, branch protection details,
and check `output` prose.

The navigation handle is a handle, not a URL. Signed download links, anything
containing `://`, `@`, `?`, `#`, a backslash, or whitespace are rejected.

## Revision exactness

For a Git Eggplan subject, the forge's tested commit must equal
`context.subject.revision`, and the subject must be `Clean`. A mismatch or a
dirty subject is a hard adapter rejection.

That single rule carries two important guarantees:

- A pull-request merge-commit run is **not** evidence about the underlying
  source commit. The merge SHA simply does not match, and the host must resolve
  and verify subject translation explicitly or not at all. This is the fail-closed
  default; a future reviewed host component may translate, but this adapter
  never does.
- A dirty worktree is never described as tested by a clean CI run. Forge CI
  checks out a commit; it cannot observe uncommitted bytes.

A per-job `head_sha`, when the host supplies one, must agree with its run's
`head_sha`.

## Status mapping

Native status and conclusion are distinct inputs. GitHub sets `conclusion` only
once `status` is `completed`; a DTO that contradicts that invariant is rejected
rather than guessed.

| Native conclusion | Eggplan status |
|---|---|
| no conclusion while non-terminal | `InProgress` |
| `success` | `Passed` |
| `failure`, `timed_out`, `startup_failure` | `Failed` |
| `cancelled`, `skipped` | `Skipped` |
| `action_required` | `Blocked` |
| `neutral`, `stale` | `Inconclusive` |

`neutral` and `stale` are neither success nor failure, and Eggplan will not guess
either. `stale` and `startup_failure` are documented for the Actions surface but
are not part of the REST check-run enum; accepting them costs nothing because
both map to a non-passing status.

Classic commit status has a smaller, total vocabulary:

| Native state | Eggplan status |
|---|---|
| `pending` | `InProgress` |
| `success` | `Passed` |
| `failure`, `error` | `Failed` |

A set of classic statuses aggregates conservatively: any `failure` or `error`
dominates, then any `pending`, then all-success. An empty list is
`Inconclusive`, because an empty list asserts nothing at all.

Artifact availability maps independently: `expired` is `Unavailable`, otherwise
`Passed`. A `Revision` observation is `Passed`, meaning only that this exact
commit exists at this repository.

## Artifact digests

GitHub **does** supply an artifact digest: `artifact.digest`, documented as
"The SHA256 digest of the artifact. This field will only be populated on
artifacts uploaded with upload-artifact v4 or newer. For older versions, this
field will be null."

The DTO therefore treats the digest as an optional exact field in
`sha256:<64 lowercase hex>` form. A malformed digest is rejected outright. An
absent digest is recorded as absent via `digest_present` metadata and is never
inferred from the artifact name, size, or run identity; the artifact reference
then simply carries no digest.

This is the honest state of the upstream interface. Eggplan neither downloads
the payload to verify a digest nor invents one.

## Revision provenance is not a verdict

`GitHubRevisionV1` carries only repository id, repository full name, and an
exact 40-character lowercase commit SHA. No mutable branch or tag name is
accepted as revision identity; `main`, `v1.0.0`, and `refs/heads/main` are all
rejected, as are wrong-length or uppercase SHAs.

A `Passed` revision observation asserts that the commit exists at that
repository. It asserts nothing about review, safety, merge state, tag presence,
or release readiness, and the serialized observation carries no such field.

## Attestation exclusion

This adapter never emits `EvidenceKind::Attestation`. It does not perform
Sigstore, in-toto, or SLSA verification, and it does not treat the presence of
an attestation-shaped payload as verified authenticity. Because all five parsers
are `deny_unknown_fields`, a real Sigstore bundle payload
(`mediaType` / `predicateType` / `bundle.dsseEnvelope`) is rejected by every
entry point rather than partially accepted as provenance. That is covered by a
regression test.

Host-side evidence-chain handling, trusted-root policy, and actual attestation
verification are Interoperability M001.

## Verification binding

Execution-derived evidence (`Test`, `Command`, `DelegatedRun`, `Benchmark`) is
execution-kind in the core evidence contract and therefore requires the host's
`VerificationDigest`. It is rejected with `MissingVerificationBinding` when
absent. The adapter never derives one from a run id, commit SHA, workflow path,
artifact digest, or step count.

## Bounds

256 KiB per DTO; 256 jobs per run; 256 classic statuses per commit; 256
characters for repository full name, workflow name, job name, check name, status
context, and artifact name; 512 characters for a workflow path; 256 characters
for a navigation handle; 40-character lowercase hex commit SHAs; 64-character
lowercase hex digests. Workflow paths must be confined relative paths. Job ids,
status ids, and status contexts must be unique and non-zero. Artifact and
revision identity fields must be non-zero.

## Boundary guard

Run `bash scripts/check-integrations-boundary.sh`. The guards scan the whole
crate source tree, so this adapter is covered without being named: no sibling
runtime, no transport or credential crate, no process or filesystem or network
access, and no `register_trusted`/`ProviderRegistry` enrollment. A source-level
test additionally greps this module for process, filesystem, network, registry,
URL, `Authorization`, and `Bearer` tokens.