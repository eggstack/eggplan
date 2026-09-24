# Eggsearch evidence adapter

`eggplan-integrations::eggsearch` normalizes host-supplied EvidenceBundle JSON
from Eggsearch `dfa90e050c5434f3346902aeb4074901c58e90d1`. It uses a strict
bounded DTO subset, ignores unselected fields including goals, URLs, snippets,
fetched text, and arbitrary warning prose, and makes no search/MCP/network
calls.

The adapter preserves bounded bundle identity, deterministic digests of
source/fetch/provider IDs, link-kind counts, line-range counts, trust counts,
truncation, and stable gap-kind counts. Unknown trust/gap/link enum values,
invalid links, duplicates, and limit violations fail. Eggsearch `LocalTrusted`
is recorded only as content provenance; the normalized `source_trust` remains
`external_untrusted`, and the adapter never enrolls its provider in a host
registry.

| Bundle condition | Eggplan status |
|---|---|
| No sources | Unavailable |
| Valid nonempty bundle, no gaps/truncation/fetch failures | Passed (operation only) |
| Any gap, truncation, or unfetched item | Inconclusive |
| All fetched items failed with FetchFailed gap | Inconclusive |

A Passed status says only that the requested bundle operation produced a
structurally valid result under the adapter profile. It makes no claim that
external research claims are true. The artifact reference is
`eggsearch:bundle:<bundle_id>`; the bundle body remains in the host's native
storage.
