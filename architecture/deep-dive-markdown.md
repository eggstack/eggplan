# Deep dive: `eggplan-markdown` interchange

Index: [overview](overview.md) (forward reference) · Normative companion: [markdown-interchange](markdown-interchange.md).
Status context: projection/CLI M002 (loss-aware Markdown import + deterministic render) is closed
(`plans/subsystems/projection-cli-roadmap.md:41-56`); fixtures re-pinned to CodeGG head
`5f4532659dbf0df2cd9f2b3bdb024217d2ea7868` (`crates/eggplan-markdown/tests/fixtures.rs:10-17`).

## 1. Crate role: bounded parser/renderer, never authority

`eggplan-markdown` is a pure intent-interchange library: deterministic render of a canonical
`Plan` to Eggplan Markdown v1, plus strict import of native Markdown and a bounded CodeGG
subset into a revision-zero Draft. It never owns repository state, evidence, provider trust,
subject authority, or closure — the module doc states this up front
(`crates/eggplan-markdown/src/lib.rs:1-4`), and the dependency list enforces it
(`crates/eggplan-markdown/Cargo.toml:8-12`): only `eggplan-core`, `serde`, `serde_json`,
`sha2`. No filesystem, process, network, or async runtime; `#![forbid(unsafe_code)]`
(`src/lib.rs:1`). The projection/CLI boundary script gates exactly this
(`scripts/check-projection-cli-boundary.sh:15-18`): no `eggplan-repo`, CLI, process, or
network symbols in the manifest or sources.

## 2. Eggplan Markdown v1: marker, fenced payload, carried vs dropped

A native document is one version marker plus one fenced canonical payload. Render emits
`NATIVE_MARKER` (`src/lib.rs:13`) and a ` ```eggplan-plan-json ` fence (`src/lib.rs:14`)
containing pretty-printed `NativePayload` (`src/lib.rs:62-72`, `deny_unknown_fields` at
`src/lib.rs:63`; items at `src/lib.rs:74-85`), then a derived human section
(`src/lib.rs:150-160`). Import requires exactly one payload start (`src/lib.rs:213-219`)
and exactly one marker (`src/lib.rs:220-223`), the fence on its own line
(`src/lib.rs:226-229`), a closing fence (`src/lib.rs:230-233`), parseable JSON
(`src/lib.rs:235-236`), byte-equal canonical pretty form (`src/lib.rs:237-243`), and
`format_version == 1` (`src/lib.rs:244-249`).

Carried: plan schema version, plan/item IDs, objective, ordering, parent/dependency links,
descriptions, acceptance criteria with evidence-requirement descriptors, blocker and
next-action intent, plus source revision/status as provenance (`src/lib.rs:128-149` for
render; `src/lib.rs:275-290` for import reset + provenance keys
`markdown_source_status`/`markdown_source_revision`). Dropped: observations, provider
authority, live subject (`subject: None`, `src/lib.rs:283`), closure record — the
`Closure record present: … (display only)` line is render-only (`src/lib.rs:154-159`) and
the human section is never re-parsed. Lifecycle is reset: plan becomes revision-zero
Draft, items become Pending — or Blocked when blocker intent is present
(`src/lib.rs:257-284`).

Render is deterministic: `plan.validate()` first (`src/lib.rs:121-122`), pre-render size
estimate (`src/lib.rs:123`, `820-862`), stable pretty JSON field order, items sorted by
`(position, id)` (`src/lib.rs:161-162`), dependencies sorted (`src/lib.rs:174-178`),
`markdown_text` escaping (`src/lib.rs:1011-1018`: `&`/`<`/`>` escaped, `\r` stripped,
`\n` → `<br>`), no timestamps/randomness, trailing byte-cap check (`src/lib.rs:204-208`).
CRLF round-trips (`src/lib.rs:234` normalizes; covered at `src/lib.rs:1062-1069`).

## 3. CodeGG subset import: section mapping, IDs, dependencies

Parsing is line-oriented with backtick/tilde fence tracking (`src/lib.rs:345-373`,
`fence_run` at `src/lib.rs:737-745`): fenced regions contribute no semantics.
`## ` headings delimit sections with a 256-section cap (`src/lib.rs:374-384`); `Status:`
and `Repository baseline:` are read **only before the first `##` section**
(`src/lib.rs:386-404`), with conflicting duplicates rejected (`src/lib.rs:389-401`).
H1 (fence-aware, `# `-only, `src/lib.rs:798-818`) is the objective fallback when no
Objective section exists (`src/lib.rs:426-438`); empty bodies fall back to fixed strings
(`src/lib.rs:924-931`).

Work packages come from `## … Ordered work packages` / `Work packages`
(`src/lib.rs:912-917`) with `### Work package <label> — <title>` headings parsed at
`src/lib.rs:887-903` (em-dash or hyphen split, label ≤ 64 chars). IDs are
`epi_md_<24 hex>` over `identity + NUL + normalized heading` (`src/lib.rs:459-463`,
`normalize_heading` at `src/lib.rs:963-969`); plan ID is `ep_md_<24 hex>` over the
caller-provided source name or else the document digest (`src/lib.rs:439-443`,
`short_hash`/`sha256` at `src/lib.rs:971-976`). Duplicate labels are rejected
(`src/lib.rs:456-458`); generated hash collisions are hard errors (`src/lib.rs:464-466`,
acceptance-item collision at `src/lib.rs:593-597`).

`Dependencies:` / `Depends on:` lines split on commas, strip backticks, lowercase
(`src/lib.rs:471-487`); empty entries are malformed (`src/lib.rs:478-480`). Unknown
labels fail (`src/lib.rs:500-502`), self-edges fail (`src/lib.rs:503-505`), and
multi-node cycles fail at the final `plan.validate()` via core graph validation
(`src/lib.rs:710-711`; cycle errors in `eggplan-core/src/graph.rs:6-11,36-43`). Order
alone never creates edges. Acceptance criteria attach per `Work package <label>: …`
prefix or become plan-level intent collected into one trailing item depending on all
work packages (`src/lib.rs:536-611`, ID at `src/lib.rs:588-591`, plan-item-limit guard
at `src/lib.rs:583-587`; list markers stripped at `src/lib.rs:949-961`, capped at
`MAX_CRITERIA` in `src/lib.rs:933-947`). All items are Pending with empty requirements;
status/revision land in provenance truncated to `PROVENANCE_CHARS`
(`src/lib.rs:619-642`).

## 4. Bounds and `ImportReport` loss codes as implemented

Input bounds: 16 MiB, 20,000 lines, 64 KiB per line, NUL rejection, UTF-8 required
(`src/lib.rs:15-18`, enforced at `src/lib.rs:715-735`); 512-char source-name limit
(`src/lib.rs:104-106`, truncated to 512 chars in reports at `src/lib.rs:870-884`);
core text/graph/item limits applied through `truncate_chars` (`src/lib.rs:978-989`) and
`Plan::validate`. Ambiguous duplicate Objective / Acceptance sections are rejected
(`src/lib.rs:430-434`, `src/lib.rs:530-534`); unterminated fences are rejected
(`src/lib.rs:423-425`).

`ImportReport` (v1, `deny_unknown_fields`, `src/lib.rs:28-44`) carries source
format/version/name, lifecycle/revision provenance, generated IDs, preserved/dropped
fields, lossy mappings, warning codes, and truncation. Native reports
(`src/lib.rs:291-325`) preserve IDs/order/links/descriptions/criteria-requirements/
blocker-next-action and always warn `source_lifecycle_not_authoritative`,
`source_revision_provenance_only`, `markdown_evidence_not_imported`,
`markdown_closure_not_imported`. CodeGG reports (`src/lib.rs:643-708`) preserve
objective/order/text (+ criteria/dependencies when present), drop unmapped section
titles (512-char truncated, `src/lib.rs:689-694`) plus a collapsed
`fenced_code_block_contents` entry (`src/lib.rs:697-702`), always map
`ordered_sections_not_dependency_edges`, warn `generated_item_id` /
`markdown_evidence_not_imported` / `markdown_closure_not_imported` (+ lifecycle codes
only when that metadata exists), and sort/dedup warning codes (`src/lib.rs:707-708`).
The exact-code vector is pinned by test (`src/lib.rs:1183-1198`).

## 5. Test strategy

Unit tests (`src/lib.rs:1020-1239`): native determinism + round-trip + CRLF
(`1048-1070`), lifecycle reset matrix incl. Closed source (`1073-1112`), parent/deps/
criteria-requirement round-trip (`1115-1138`), duplicate-payload / unknown-field /
bad-version rejection (`1141-1153`), CodeGG determinism without order edges
(`1156-1174`), exact warning-code + truncation pin (`1177-1204`), explicit-dep mapping
and unknown/malformed/ambiguous rejection (`1207-1220`), scoped acceptance routing
(`1223-1232`), NUL/oversize rejection (`1235-1238`). Corpus tests
(`tests/fixtures.rs:19-203`): ordinary-plan determinism with 4 edge-free items + a
final 4-dependency acceptance item (`19-60`), corrective prose manufacturing no
evidence/closure (`62-112`), unmapped-section reporting (`114-137`), fake-closure +
fenced-code rejection (`140-181`), tilde-fence marker spoofing (`184-190`), nested
short-fence containment (`193-203`).

## 6. Review findings

Strengths: genuinely dependency-free bounded design; fence-aware scanning in every
entry point (`detect_format`, both importers, H1 scan); strict native canonical-JSON
equality plus `deny_unknown_fields` on both payload and report; preamble-conflict
rejection instead of last-wins; ID determinism tests; display-only closure handling.

Gaps/risks (all verified against source, not speculation):

1. **Native `ImportReport` never populates `generated_ids` or `dropped_fields`.**
   `report_base` starts both empty (`src/lib.rs:878-880`) and the native path
   (`src/lib.rs:291-325`) pushes only preserved/warning/lossy entries — unlike CodeGG
   (`src/lib.rs:643-708`). The ignored human section and the dropped
   observations/policy/subject/closure dimensions are therefore unnamed in native
   reports, although [markdown-interchange](markdown-interchange.md) promises dropped
   fields are named. If unsure whether this is intentional, say so — but as
   implemented the asymmetry is real.
2. **CodeGG `Status:`/baseline parse position is narrower than the doc table implies.**
   The interchange doc lists them as imported metadata; the code only reads them
   before the first `##` (`src/lib.rs:386-404`) — the same lines later in the file
   become section body text (CodeGG path) or acceptance-statement text. Metadata
   placed after a heading is silently kept as prose rather than rejected or recorded.
3. **ID-collision handling is fail-closed with no recovery hint.** Distinct headings
   that normalize identically (case/whitespace, `src/lib.rs:963-969`) collide to a
   hard `generated work-package ID collision` error (`src/lib.rs:464-466`); same for
   the acceptance-intent item (`src/lib.rs:593-597`). There is no disambiguation
   (e.g. label mix-in) and the error names no colliding headings. Plan-level IDs
   additionally depend on caller-provided `source_name`, so two unrelated documents
   imported under the same name share IDs — collision rejection then lives outside
   this crate (CLI/repo layer).
4. **Dead branch in `acceptance_lines` (`src/lib.rs:941-943`).** When there is exactly
   one statement longer than 2,000 bytes, `values.truncate(1)` is a no-op; real
   bounding happens later per statement via `CRITERION_CHARS` (`src/lib.rs:558`).
   Intent is unclear — likely leftover — and deserves a comment or removal.
5. **Minor doc/code drift: `Depends on:` alias.** The importer accepts it alongside
   `Dependencies:` (`src/lib.rs:472-476`) but the interchange doc's table names only
   `Dependencies:`. Either is defensible; the doc should name the alias.

## Verification pointers

```sh
cargo test -p eggplan-markdown --locked
cargo test -p eggplan-markdown --locked --test fixtures
bash scripts/check-projection-cli-boundary.sh
```
