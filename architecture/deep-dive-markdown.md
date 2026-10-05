# Deep dive: `eggplan-markdown` interchange

Index: [overview](overview.md) · Normative companion: [markdown-interchange](markdown-interchange.md).
Status context: projection/CLI M002 (loss-aware Markdown import + deterministic render) is closed
(`plans/subsystems/projection-cli-roadmap.md:41-56`, registered closed with closure evidence at
`plans/registry.md:77`); fixtures re-pinned to CodeGG head
`5f4532659dbf0df2cd9f2b3bdb024217d2ea7868` (`crates/eggplan-markdown/tests/fixtures.rs:10-17`).
Roadmap M003 (ergonomics/performance, `plans/subsystems/projection-cli-roadmap.md:58`) has **no**
registered implementation plan and no closure evidence — the registry records it as waiting for real
repository use (`plans/registry.md:57`, `plans/registry.md:180`). Nothing in this crate should be
read as M003 evidence.

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

The authority boundary is structural, not merely documented. The crate's only output type is
`ImportedPlan { plan: Plan, report: ImportReport }` (`src/lib.rs:46-50`), and `Plan`
(`eggplan-core/src/model.rs:127-137`) has no observation, provider, or closure field at all — a
Markdown document has no representation to land in. A grep of `src/lib.rs` for
`std::fs|File|reqwest|TcpStream|Command|process::|env::` returns nothing, so nothing is fetched,
opened, or executed: links, includes, URLs, and paths are only ever text. Both importers
construct their plan once — native at `src/lib.rs:275-284`, CodeGG via `Plan::new` at
`src/lib.rs:612-617`, which core fixes at revision 0, `PlanStatus::Draft`, `subject: None`
(`eggplan-core/src/model.rs:270-287`) — and the CLI writes exactly that one plan
(`crates/eggplan-cli/src/lib.rs:805-809`), where the store re-asserts revision 0 + Draft and
rejects an existing ID (`crates/eggplan-repo/src/store.rs:851-862`).

## 2. Eggplan Markdown v1: marker, fenced payload, carried vs dropped

A native document is one version marker plus one fenced canonical payload. Render emits
`NATIVE_MARKER` (`src/lib.rs:13`) and a ` ```eggplan-plan-json ` fence (`src/lib.rs:14`)
containing pretty-printed `NativePayload` (`src/lib.rs:62-72`, `deny_unknown_fields` at
`src/lib.rs:63`; items at `src/lib.rs:74-85`), then a derived human section
(`src/lib.rs:150-203`, shape enumerated below). Import requires exactly one payload start
(`src/lib.rs:213-219`)
and exactly one marker (`src/lib.rs:220-223`), the fence on its own line
(`src/lib.rs:226-229`), a closing fence (`src/lib.rs:230-233`), parseable JSON
(`src/lib.rs:235-236`), byte-equal canonical pretty form (`src/lib.rs:237-243`), and
`format_version == 1` (`src/lib.rs:244-249`).

Carried: plan schema version, plan/item IDs, objective, ordering, parent/dependency links,
descriptions, acceptance criteria with evidence-requirement descriptors, blocker and
next-action intent, plus source revision/status as provenance (`src/lib.rs:128-149` for
render; `src/lib.rs:275-290` for import reset + provenance keys
`markdown_source_status`/`markdown_source_revision`). Absent from the native payload entirely:
observations, provider authority, live subject (`subject: None`, `src/lib.rs:283`), and any closure
record. Those dimensions are not silently discarded on the native path — `NativePayload`/`NativeItem`
have no field for them and carry `deny_unknown_fields` (`src/lib.rs:63`, `src/lib.rs:75`), so a
payload carrying `"observations"`, `"subject"`, or `"closure"` is a hard import error. The
`Closure record present: … (display only)` line is render-only (`src/lib.rs:154-159`), its
`yes`/`no` value comes from the caller's `has_closure: bool` parameter (`src/lib.rs:120`) and is
never parsed from the document, and the human section is never re-parsed. Lifecycle is reset: plan
becomes revision-zero Draft, items become Pending — or Blocked when blocker intent is present
(`src/lib.rs:257-284`).

The derived human section is fixed-shape, not free prose: `# <objective>`, then
`Plan: \`<id>\` · revision <n> · status \`<status>\``, `Items: <n>`, the closure display line, then per
item `## <id> — <description>` / `Status: \`<status>\`` / the description body again, optional
`Parent:`, sorted `Dependencies:`, `Blocker:`, `Next action:`, and `### Acceptance <id>` per
criterion with `Evidence requirement (display only): …` lines (`src/lib.rs:152-203`). Note the
description is emitted twice per item (heading and body, `src/lib.rs:165-169`) — cosmetic, but it
means a rendered item is visibly self-duplicating.

Render is deterministic: `plan.validate()` first (`src/lib.rs:121-122`), pre-render size
estimate (`src/lib.rs:123`, `820-862`), stable pretty JSON field order, items sorted by
`(position, id)` (`src/lib.rs:161-162`; a total order, because `plan.validate()` rejects duplicate
item IDs at `eggplan-core/src/model.rs:324-326`), dependencies sorted (`src/lib.rs:174-178`),
`markdown_text` escaping (`src/lib.rs:1011-1018`: `&`/`<`/`>` escaped, `\r` stripped,
`\n` → `<br>`), no timestamps/randomness, trailing byte-cap check (`src/lib.rs:204-208`).
The render output is a pure function of `(plan, has_closure)`: `src/lib.rs` contains no clock,
random source, `HashMap`, environment read, or locale-sensitive sort, and imports only
`BTreeMap`/`BTreeSet` for ordered collection (`src/lib.rs:11`). The size pre-check deliberately
over-estimates (6 bytes per Unicode scalar, `src/lib.rs:820-825`) so the bound is checked before
encoding, with the authoritative post-render check at `src/lib.rs:204-208`.
CRLF round-trips (`src/lib.rs:234` normalizes; covered at `src/lib.rs:1062-1069`).

## 3. CodeGG subset import: section mapping, IDs, dependencies

Parsing is line-oriented with backtick/tilde fence tracking (`src/lib.rs:345-373`,
`fence_run` at `src/lib.rs:737-745`): fenced regions contribute no semantics.
`## ` headings delimit sections with a 256-section cap (`src/lib.rs:374-384`); `Status:`
and `Repository baseline:` are read **only before the first `##` section**
(`src/lib.rs:386-404`), with conflicting duplicates rejected (`src/lib.rs:389-401`).
H1 (fence-aware, `# `-only, `src/lib.rs:798-818`) is the objective fallback when no
Objective section exists (`src/lib.rs:426-438`); an empty Objective body falls back to
`"Imported plan objective"` (`src/lib.rs:924-931`) and a document with no H1 at all to
`"Imported implementation plan"` (`src/lib.rs:347`); the resolved objective is finally bounded to
`bounds::OBJECTIVE_CHARS` (`src/lib.rs:614`).

Work packages come from `## … Ordered work packages` / `Work packages`
(`src/lib.rs:912-917`) with `### Work package <label> — <title>` headings parsed at
`src/lib.rs:887-903` (em-dash or hyphen split, label bounded at 64 **bytes** — `label.len() > 64`,
`src/lib.rs:899`; a shorter multi-byte label can therefore be rejected) and capped at
`bounds::MAX_ITEMS` = 512 packages (`src/lib.rs:407-411`, re-checked at `src/lib.rs:445-449`).
IDs are
`epi_md_<24 hex>` over `identity + NUL + normalized heading` (`src/lib.rs:459-463`,
`normalize_heading` at `src/lib.rs:963-969`); plan ID is `ep_md_<24 hex>` over the
caller-provided source name or else the document digest (`src/lib.rs:439-443`,
`short_hash`/`sha256` at `src/lib.rs:971-976`). Duplicate labels are rejected
(`src/lib.rs:456-458`); generated hash collisions are hard errors (`src/lib.rs:464-466`,
acceptance-item collision at `src/lib.rs:593-597`).

Acceptance sections match `## N. Acceptance criteria` **and any heading whose normalized name ends
in ` acceptance criteria`** (`src/lib.rs:919-922`), so CodeGG's `## 8. Binary acceptance criteria`
is imported (exercised by `crates/eggplan-markdown/tests/fixtures/corrective-plan.md:351`). Only
`## ` at column 0 opens a section and only `### Work package ` inside a work-package section opens
a package (`src/lib.rs:374`, `src/lib.rs:406`), so deeper headings (`#### …`) and `### …` under any
other section are body text. Every other `## ` section is **dropped by title and named in the loss
report** (`src/lib.rs:681-696`): the implementation is a three-entry whitelist (Objective, work
packages, acceptance criteria), not a blacklist, so Scope, Findings, Verification, Stop conditions,
Handoff notes, Required design, and every other CodeGG section are instances of the same rule.
Their body text is discarded unread.

`Dependencies:` / `Depends on:` lines split on commas, strip backticks, lowercase
(`src/lib.rs:471-487`); empty entries are malformed (`src/lib.rs:478-480`). Unknown
labels fail (`src/lib.rs:500-502`), self-edges fail (`src/lib.rs:503-505`), and
multi-node cycles fail at the final `plan.validate()` via core graph validation
(`src/lib.rs:710-711`; cycle errors in `eggplan-core/src/graph.rs:6-11,36-43`). Duplicate
references (`Dependencies: A, A`) are rejected too, by core's per-item duplicate check
(`eggplan-core/src/model.rs:259-264`), and every edge target must resolve to a known item
(`eggplan-core/src/model.rs:331-341`). Order
alone never creates edges: `dependencies` is populated only from those two line prefixes
(`src/lib.rs:471-487`) plus the synthetic plan-level intent item's edges (`src/lib.rs:599`).
Acceptance criteria attach per `Work package <label>: …`
prefix or become plan-level intent collected into one trailing item depending on all
work packages (`src/lib.rs:536-611`, ID at `src/lib.rs:588-591`, plan-item-limit guard
at `src/lib.rs:583-587`; list markers stripped at `src/lib.rs:949-961`, capped at
`bounds::MAX_CRITERIA` = 128 in `src/lib.rs:933-947`). Criterion IDs are a third ID family,
`epc_md_<24 hex>` over `identity + NUL + "acceptance" + NUL + statement index + NUL + bounded
statement` (`src/lib.rs:559-563`) — position- and text-sensitive, unlike the work-package IDs, and
unlike them they are not covered by the `used` collision guard (only item IDs enter it,
`src/lib.rs:452`, `src/lib.rs:468`, `src/lib.rs:593`). All items are Pending with empty
requirements (`src/lib.rs:519`, `src/lib.rs:569`); status/revision land in provenance truncated to
`bounds::PROVENANCE_CHARS` = 2,000 chars (`src/lib.rs:619-642`), while the same two values in the
report are capped at a hardcoded 512 chars (`src/lib.rs:619-630`) — see finding 6.

## 4. Bounds and `ImportReport` loss codes as implemented

Input bounds: 16 MiB, 20,000 lines, 64 KiB per line, NUL rejection, UTF-8 required
(`src/lib.rs:15-18`, enforced at `src/lib.rs:715-735`); 512-char source-name limit
(`src/lib.rs:104-106`, truncated to 512 chars in reports at `src/lib.rs:870-884`);
core text/graph/item limits applied through `truncate_chars` (`src/lib.rs:978-989`) and
`Plan::validate`. Ambiguous duplicate Objective / Acceptance sections are rejected
(`src/lib.rs:430-434`, `src/lib.rs:530-534`); unterminated fences are rejected
(`src/lib.rs:423-425`).

`ImportReport` (schema version 1 set in `report_base`, `src/lib.rs:871`; struct with
`deny_unknown_fields`, `src/lib.rs:28-44`) carries source
format/version/name, lifecycle/revision provenance, generated IDs, preserved/dropped
fields, lossy mappings, warning codes, and truncation. The complete set is seven warning
codes — `source_lifecycle_not_authoritative`, `source_revision_provenance_only`,
`markdown_evidence_not_imported`, `markdown_closure_not_imported`, `unmapped_section`,
`text_truncated`, `generated_item_id` — and three lossy mappings:
`item_status_reset_to_pending_or_blocked_from_blocker_intent` and, when the source was not
`draft`, `source_plan_status_reset_to_draft` (native only, `src/lib.rs:315-322`), plus
`ordered_sections_not_dependency_edges` (CodeGG only, `src/lib.rs:672-674`). The normative
companion lists that last one among "warning codes"
(`architecture/markdown-interchange.md:66-69`) although it is a lossy mapping. Native reports
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

Production panic audit: outside `#[cfg(test)]` (`src/lib.rs:1020-1239`) there is no `expect`, no
`panic!`, no indexing panic and no slicing panic reachable from attacker input. The only three
sites are provably guarded: `unreachable!()` at `src/lib.rs:114` (the `explicit` binding at
`src/lib.rs:111` covers every non-`Auto` variant, so `Auto` can never reach it), `starts[0]` at
`src/lib.rs:224` (guarded by `starts.len() != 1` at `src/lib.rs:215`), and
`sections[current.unwrap()]` at `src/lib.rs:405` (reachable only past the
`if current.is_none() { … continue; }` block at `src/lib.rs:386-404`, which is the only way
`current` is set — `src/lib.rs:382`). Every slice in the crate is a char-boundary-safe offset:
`fence_run` slices after a run of ASCII markers (`src/lib.rs:744`), `native_payload_starts`
accumulates whole lines (`src/lib.rs:793`) and adds a 20-byte ASCII fence to a line whose prefix
is whitespace, and `strip_list_marker` slices after ASCII digits (`src/lib.rs:953-955`).
The one fragility worth naming is `src/lib.rs:405`: the invariant is a two-block coupling, so a
future refactor that reorders those blocks would panic on hostile input.

## 5. Test strategy

Unit tests (`src/lib.rs:1020-1239`): native determinism + round-trip + CRLF
(`1048-1070`), lifecycle reset matrix incl. Closed source (`1073-1112`), parent/deps/
criteria-requirement round-trip (`1115-1138`), duplicate-payload / unknown-field /
bad-version rejection (`1141-1153`), CodeGG determinism without order edges
(`1156-1174`), exact warning-code + truncation pin (`1177-1204`), explicit-dep mapping
and unknown/malformed/ambiguous rejection (`1207-1220`), scoped acceptance routing
(`1223-1232`), NUL/oversize rejection (`1235-1238`). Nothing in the unit tests asserts the CodeGG `subject` is `None` —
that invariant is covered only by the corpus tests (`tests/fixtures.rs:71`,
`tests/fixtures.rs:148`). Corpus tests
(`tests/fixtures.rs:19-203`): ordinary-plan determinism with 4 edge-free items + a
final 4-dependency acceptance item (`19-60`, also pinning the reviewed baseline digest at
`45-48` and 6 plan-level criteria at `44`), corrective prose manufacturing no
evidence/closure (`62-112`), unmapped-section reporting (`114-137`), fake-closure +
fenced-code rejection (`140-181`), tilde-fence marker spoofing (`184-190`), nested
short-fence containment (`193-203`). The corpus is pinned to the reviewed CodeGG head and a
3-fixture manifest (`tests/fixtures.rs:9-17`,
`crates/eggplan-markdown/tests/fixtures/manifest.json`), which is what the status context
above cites.

## 6. Review findings

Strengths: genuinely dependency-free bounded design; fence-aware scanning in every
entry point (`detect_format`, both importers, H1 scan); strict native canonical-JSON
equality plus `deny_unknown_fields` on both payload and report; preamble-conflict
rejection instead of last-wins; ID determinism tests; display-only closure handling;
a single-`Plan` output type that makes evidence/trust/subject/closure structurally
unrepresentable; and no production panic path reachable from hostile input (see §4).

Gaps/risks (all verified against source, not speculation):

1. **Native `ImportReport` never populates `generated_ids` or `dropped_fields`.**
   `report_base` starts both empty (`src/lib.rs:878-880`) and the native path
   (`src/lib.rs:291-325`) pushes only preserved/warning/lossy entries — unlike CodeGG
   (`src/lib.rs:643-708`). The ignored human section is therefore unnamed. The
   observations/policy/subject/closure dimensions cannot be named at all: the payload has no
   such fields and `deny_unknown_fields` (`src/lib.rs:63`) makes an attempt a hard error, so the
   report has nothing to record even though [markdown-interchange](markdown-interchange.md:63-65)
   promises dropped fields are named. If unsure whether this is intentional, say so — but as
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
   this crate (CLI/repo layer, `crates/eggplan-repo/src/store.rs:857-862`). On the native
   path the situation is stronger: the document supplies `plan_id` verbatim
   (`src/lib.rs:67`, `src/lib.rs:277`) with no hash and no cross-check against the caller, so
   any syntactically valid `PlanId` can be claimed and only the store's `AlreadyExists`
   rejection stands between two documents. That is the intended ownership split, but the
   crate offers no way to detect the impersonation.
4. **Dead branch in `acceptance_lines` (`src/lib.rs:941-943`).** When there is exactly
   one statement longer than 2,000 bytes, `values.truncate(1)` is a no-op; real
   bounding happens later per statement via `CRITERION_CHARS` (`src/lib.rs:558`, 2,000
   Unicode scalars). The dead branch also compares a **byte** length against a limit that is
   applied in **chars**, so a ~900-character non-ASCII statement trips the branch and is yet
   not truncated. Intent is unclear — likely leftover — and deserves a comment or removal.
5. **Former doc drift, now resolved: `Depends on:` alias.** The importer accepts it
   alongside `Dependencies:` (`src/lib.rs:472-476`) and the normative companion's table
   does name the alias (`architecture/markdown-interchange.md:44`). Recorded here only
   because an earlier revision of this deep dive claimed the opposite; no action remains.
6. **The same metadata is bounded differently in the plan and in the report.**
   `Repository baseline:`/`Status:` go into `plan.provenance` through
   `truncate_chars(…, bounds::PROVENANCE_CHARS /* 2,000 */)` (`src/lib.rs:634`, `src/lib.rs:640`)
   but into `report.original_source_status`/`original_source_revision` through a hardcoded
   `.chars().take(512)` (`src/lib.rs:619-630`). A 600-character baseline is therefore preserved
   in full in the canonical Plan and silently halved in the loss report, and the 512 figure is
   the same literal used for source names (`src/lib.rs:874`) and section titles
   (`src/lib.rs:689-694`) — one magic number doing three unrelated jobs. Loss reporting should
   not under-report what the Plan kept.
7. **Criterion IDs are position- and text-sensitive and bypass the collision guard.**
   `epc_md_<24 hex>` hashes `identity + NUL + "acceptance" + NUL + index + NUL + bounded
   statement` (`src/lib.rs:559-563`), unlike work-package IDs which hash only the normalized
   heading. Re-wording or re-ordering acceptance lines therefore mints new criterion IDs for
   otherwise unchanged intent, and because the *bounded* statement is hashed, a line that
   differs only past the 2,000-char criterion bound keeps its ID — a stability property worth
   knowing, but an implicit one. The `used` collision set covers only item IDs
   (`src/lib.rs:452`, `src/lib.rs:468`, `src/lib.rs:593`), and core's duplicate-criterion check is
   per item (`eggplan-core/src/model.rs:252-258`), so a 96-bit collision between criteria
   attached to *different* work packages would be silently accepted rather than rejected. Low
   likelihood, but work-package item IDs get the explicit guard and these do not.
8. **Rendered items print their description twice.** The derived human section emits the
   description in the `## <id> — <description>` heading and again as the item body
   (`src/lib.rs:164-170`). Harmless for import (the section is never re-parsed) but it makes
   every rendered item self-duplicating, and a long description is escaped and copied twice
   into the output budget.

## Verification pointers

```sh
cargo test -p eggplan-markdown --locked
cargo test -p eggplan-markdown --locked --test fixtures
bash scripts/check-projection-cli-boundary.sh
```
