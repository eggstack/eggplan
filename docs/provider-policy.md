# Provider policy

`assess` and `close` both require an explicit provider-policy file. This is deliberate:
**trust is host-conferred, never self-asserted.** A provider ID inside an observation is
only a label, so the host must state which providers it actually trusts and for what.

There is **no default trusted provider.** Without a policy file, an empty registry cannot
promote any passing observation to trusted proof.

## Format

```json
{
  "schema_version": 1,
  "providers": [
    {
      "provider_id": "epp_local_tests",
      "class": "test_runner",
      "allowed_kinds": ["test"]
    },
    {
      "provider_id": "epp_ci_bench",
      "class": "benchmark_harness",
      "allowed_kinds": ["benchmark", "static_analysis"]
    }
  ]
}
```

| Field | Rule |
|---|---|
| `schema_version` | Must be exactly `1` |
| `providers` | At most 128 entries; duplicate `provider_id` values are rejected |
| `provider_id` | Typed ID, `epp_` prefix, prefix-validated and length-bounded |
| `class` | Free text, at most 64 characters, non-empty, no NUL |
| `allowed_kinds` | 1–10 values, deduplicated; must not be empty |

The file is at most 64 KiB and rejects unknown fields.

## Allowed evidence kinds

`command`, `test`, `static_analysis`, `revision`, `artifact`, `delegated_run`, `benchmark`,
`research`, `human_judgment`, `attestation`.

A provider may only contribute evidence whose kind appears in its `allowed_kinds`.
Grant the narrowest set that works — a test runner trusted for `benchmark` evidence is a
policy mistake, not a convenience.

## Using it

```sh
cargo run -p eggplan-cli -- assess ep_example --state-root .eggplan --provider-policy policy.json
cargo run -p eggplan-cli -- close  ep_example --state-root .eggplan \
  --expected-revision 7 --provider-policy policy.json
```

The policy is scoped to **one invocation**. It is not persisted as a standing grant, and it
is not carried forward to later assessments. A closure record stores the policy as
**historical evidence** describing what was trusted at closure time — it does not
auto-trust those providers again later.

## Why the CLI cannot accept evidence directly

The CLI never accepts user-authored passing evidence. Trust has to be established by a host
that actually observed the run, and the host is a provider adapter, not a human typing JSON
into a terminal. This is the boundary that keeps a plausible-looking hand-written observation
from closing a plan.

## Failure modes

All of these fail closed:

| Diagnostic | Cause |
|---|---|
| `invalid_policy` | Malformed descriptor, bad class, empty or oversized `allowed_kinds` |
| `unknown_schema` | `schema_version` is not 1 |
| `duplicate_provider` | The same `provider_id` appears twice |
| `policy_bound` | More than 128 providers, or the file exceeds 64 KiB |

## Related

- [Evidence and closure](evidence-and-closure.md) — how the registry affects assessment
- [CLI reference](cli-reference.md) — full command and flag surface
