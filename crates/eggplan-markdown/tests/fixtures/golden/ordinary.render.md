<!-- eggplan-markdown:v1 -->

```eggplan-plan-json
{
  "format_version": 1,
  "plan_schema_version": 2,
  "plan_id": "ep_md_eb5f7e10141eb07d06f710df",
  "objective": "Add a bounded, revisioned, restart-durable `WorkPlan`/`WorkItem` domain in `codegg-core` that can represent detailed execution state for large tasks without turning Goal, TodoState, or context checkpoints into a general workflow database.\n\nThis milestone lands storage/domain semantics only; model-facing plan tools and automatic completion arbitration belong to M003.",
  "source_revision": 0,
  "source_status": "draft",
  "items": [
    {
      "id": "epi_md_718540c10274bd870ddd86e7",
      "position": 0,
      "parent": null,
      "dependencies": [],
      "description": "Define IDs, statuses, validation rules, objective provenance, item/criterion/evidence shapes, scope/lifecycle, and serialization. Prove all bounds and invalid transitions with unit tests.",
      "criteria": [],
      "blocker": null,
      "next_action": null
    },
    {
      "id": "epi_md_c0b17b222f900dab546ac129",
      "position": 1,
      "parent": null,
      "dependencies": [],
      "description": "Add migration/tables/indexes and typed store with CAS revision. Include transaction semantics for plan creation/replacement and item/dependency updates.",
      "criteria": [],
      "blocker": null,
      "next_action": null
    },
    {
      "id": "epi_md_6eab1ae650f564dea2b930a4",
      "position": 2,
      "parent": null,
      "dependencies": [],
      "description": "Implement deterministic dependency gating and canonical evidence-reference validation. Do not implement semantic criterion inference.",
      "criteria": [],
      "blocker": null,
      "next_action": null
    },
    {
      "id": "epi_md_5101c5cb4bc398d2bdd80d12",
      "position": 3,
      "parent": null,
      "dependencies": [],
      "description": "Allow optional exact Goal ID/session binding with no change to Goal runtime behavior. Replacement/cancel must not orphan authoritative identities silently.",
      "criteria": [],
      "blocker": null,
      "next_action": null
    },
    {
      "id": "epi_md_d84dfe7803b0869e5d9bdf15",
      "position": 4,
      "parent": null,
      "dependencies": [
        "epi_md_718540c10274bd870ddd86e7",
        "epi_md_c0b17b222f900dab546ac129",
        "epi_md_6eab1ae650f564dea2b930a4",
        "epi_md_5101c5cb4bc398d2bdd80d12"
      ],
      "description": "Imported plan-level acceptance criteria (intent only)",
      "criteria": [
        {
          "id": "epc_md_911ef0b4c556f944a07590e6",
          "statement": "WorkPlan/WorkItems survive restart with stable IDs/revisions.",
          "human_judgment_allowed": false,
          "requirements": []
        },
        {
          "id": "epc_md_196cd6a5bfc66b029f2ad7ce",
          "statement": "A bounded plan larger than TodoState's projection cap is valid and queryable.",
          "human_judgment_allowed": false,
          "requirements": []
        },
        {
          "id": "epc_md_fb2f78fd75bb638121edf5ee",
          "statement": "Dependencies deterministically identify actionable items.",
          "human_judgment_allowed": false,
          "requirements": []
        },
        {
          "id": "epc_md_9bbb1b9592f2d457c06fef54",
          "statement": "Stale writes cannot overwrite newer plan state.",
          "human_judgment_allowed": false,
          "requirements": []
        },
        {
          "id": "epc_md_d1ae1efe8548aa16ea60e981",
          "statement": "Evidence refs point to canonical host-owned identities or are explicitly unavailable.",
          "human_judgment_allowed": false,
          "requirements": []
        },
        {
          "id": "epc_md_0834c9ecc0212a05106456ca",
          "statement": "No model-visible/runtime behavior changes yet.",
          "human_judgment_allowed": false,
          "requirements": []
        }
      ],
      "blocker": null,
      "next_action": null
    }
  ]
}
```

# Add a bounded, revisioned, restart-durable `WorkPlan`/`WorkItem` domain in `codegg-core` that can represent detailed execution state for large tasks without turning Goal, TodoState, or context checkpoints into a general workflow database.<br><br>This milestone lands storage/domain semantics only; model-facing plan tools and automatic completion arbitration belong to M003.

Plan: `ep_md_eb5f7e10141eb07d06f710df` · revision 0 · status `draft`

Items: 5

Closure record present: no (display only)

## epi_md_718540c10274bd870ddd86e7 — Define IDs, statuses, validation rules, objective provenance, item/criterion/evidence shapes, scope/lifecycle, and serialization. Prove all bounds and invalid transitions with unit tests.

Status: `pending`

Define IDs, statuses, validation rules, objective provenance, item/criterion/evidence shapes, scope/lifecycle, and serialization. Prove all bounds and invalid transitions with unit tests.

## epi_md_c0b17b222f900dab546ac129 — Add migration/tables/indexes and typed store with CAS revision. Include transaction semantics for plan creation/replacement and item/dependency updates.

Status: `pending`

Add migration/tables/indexes and typed store with CAS revision. Include transaction semantics for plan creation/replacement and item/dependency updates.

## epi_md_6eab1ae650f564dea2b930a4 — Implement deterministic dependency gating and canonical evidence-reference validation. Do not implement semantic criterion inference.

Status: `pending`

Implement deterministic dependency gating and canonical evidence-reference validation. Do not implement semantic criterion inference.

## epi_md_5101c5cb4bc398d2bdd80d12 — Allow optional exact Goal ID/session binding with no change to Goal runtime behavior. Replacement/cancel must not orphan authoritative identities silently.

Status: `pending`

Allow optional exact Goal ID/session binding with no change to Goal runtime behavior. Replacement/cancel must not orphan authoritative identities silently.

## epi_md_d84dfe7803b0869e5d9bdf15 — Imported plan-level acceptance criteria (intent only)

Status: `pending`

Imported plan-level acceptance criteria (intent only)

Dependencies: epi_md_5101c5cb4bc398d2bdd80d12, epi_md_6eab1ae650f564dea2b930a4, epi_md_718540c10274bd870ddd86e7, epi_md_c0b17b222f900dab546ac129

### Acceptance epc_md_911ef0b4c556f944a07590e6

WorkPlan/WorkItems survive restart with stable IDs/revisions.

### Acceptance epc_md_196cd6a5bfc66b029f2ad7ce

A bounded plan larger than TodoState's projection cap is valid and queryable.

### Acceptance epc_md_fb2f78fd75bb638121edf5ee

Dependencies deterministically identify actionable items.

### Acceptance epc_md_9bbb1b9592f2d457c06fef54

Stale writes cannot overwrite newer plan state.

### Acceptance epc_md_d1ae1efe8548aa16ea60e981

Evidence refs point to canonical host-owned identities or are explicitly unavailable.

### Acceptance epc_md_0834c9ecc0212a05106456ca

No model-visible/runtime behavior changes yet.
