# Eggwork evidence adapter

`eggplan-integrations::eggwork` normalizes host-supplied Eggwork core JSON. It
does not depend on eggwork-core, connect to eggworkd, acquire credentials, or
execute work. DTOs are qualified against Eggwork
`faaa0b905fa6bc43e46825fdd98530b5533a970f`.

The adapter accepts schema-version 1 snapshots and artifact records within
fixed byte/count bounds. It requires snapshot/result state agreement, positive
generation, matching artifact execution/generation, unique artifact IDs,
SHA-256 digests, confined logical paths, and matching artifact counts. Unknown
snapshot/result/artifact fields and enum variants fail closed.

| Eggwork fact | Eggplan status or metadata |
|---|---|
| Accepted, Preparing, Running, Cancelling | InProgress |
| Succeeded | Passed |
| Failed, TimedOut | Failed |
| Cancelled | Skipped |
| Interrupted | Inconclusive |
| Terminal snapshot without result | Inconclusive |
| Finalization failure | Inconclusive; artifact capture may be incomplete |
| Sandbox failure or resource limit exceeded | Failed |

Command, Test, DelegatedRun, and Benchmark observations require the host's
`ObservationContext.verification_digest`; no execution ID or returned command
text is used to derive it. Metadata retains IDs, generation, terminal state,
exit/failure classes, byte counts, cleanup-warning presence, and bounded
artifact references. It does not retain stdout/stderr, environment, credentials,
leases, timestamps, or arbitrary sandbox/resource messages.
