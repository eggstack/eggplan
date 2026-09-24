# Markdown plan interchange

`eggplan-markdown` renders canonical Plans as Eggplan Markdown v1 and imports a
strict, bounded subset of CodeGG implementation-plan Markdown. Markdown is
intent input or a human projection; it is never canonical repository state.

## Eggplan Markdown v1

A native document has exactly one version marker and one
`eggplan-plan-json` fenced payload:

````markdown
<!-- eggplan-markdown:v1 -->

```eggplan-plan-json
{ ... strict versioned plan-intent object ... }
```
````

The payload carries the source Plan schema version, plan/item IDs, objective, ordering, parent/dependency
links, descriptions, acceptance criteria, evidence requirement descriptors,
blocker and next-action intent, plus source revision/status as provenance. It
does not carry observations, provider-policy authority, a live subject, or a
closure record. The human section is derived and ignored on import.

Import validates the complete payload against Eggplan domain bounds,
then creates a revision-zero Draft Plan. Plan lifecycle is advisory provenance
only; item lifecycle is reset to Pending, or Blocked when blocker intent is
present. A closure-present line is display only. No evidence or closure API is
called.

Rendering is deterministic for the same Plan and closure-present display flag.
It sorts items by position then ID, uses stable JSON field order, and adds no
timestamps, random values, terminal escapes, or locale-dependent sorting.

## Supported CodeGG subset

| Markdown section | Import behavior |
|---|---|
| H1 title | Fallback objective when no Objective section exists |
| `Status:` / `Repository baseline:` | Import report and provenance only |
| `## N. Objective` | Objective text |
| `## N. Ordered work packages` (or `Work packages`) with `### Work package A — Title` | One deterministically identified Plan item per work package, preserving source order |
| `Dependencies: A, B` inside a work package | Exact references to labeled work packages; unknown, self, duplicate, or cyclic references fail validation |
| `## N. Acceptance criteria` (including names such as `Binary acceptance criteria`) | Plan-level criteria become a final intent item depending on work packages; `Work package A: ...` criteria attach to that named item |
| Other sections, including Scope, Findings, Verification, Stop conditions, and Handoff notes | Dropped from canonical Plan intent and named in the loss report |
| Fenced code blocks | Ignored as semantics and reported as unmapped input |

Order alone never creates dependency edges. IDs are stable hashes of source
identity and normalized work-package headings. The caller-provided source name
is the identity when available; otherwise the document digest is used.

## Bounds and loss report

Input is limited to 16 MiB, 20,000 lines, 64 KiB per line, 512 source-name
characters, 256 sections, and
Eggplan core's item/text/graph limits. NUL, invalid UTF-8, duplicate native
payloads, malformed JSON, unsupported versions, ambiguous Objective or
Acceptance Criteria sections, malformed references, and invalid canonical
Plans are rejected. No links, includes, URLs, local files, or code fences are
fetched or executed.

Every successful import returns an `ImportReport` schema version 1 with source
format/name, source lifecycle/revision provenance, generated IDs, preserved
and dropped fields, lossy mappings, stable warning codes, and truncation state.
Codes include `source_lifecycle_not_authoritative`,
`source_revision_provenance_only`, `markdown_evidence_not_imported`,
`markdown_closure_not_imported`, `ordered_sections_not_dependency_edges`,
`unmapped_section`, `text_truncated`, and `generated_item_id`.

## CLI examples

```sh
eggplan markdown render ep_example --output plan.md
eggplan markdown inspect plan.md --format auto --json
eggplan --state-root .eggplan markdown import plan.md --format auto --json
```

`inspect` is read-only. `import` requires an explicit state root, parses before
opening the repository, creates one new Draft Plan, and rejects ID collisions.
Import failure leaves repository state unchanged. Evidence, provider policy,
SubjectRevision, and closure are never imported.
