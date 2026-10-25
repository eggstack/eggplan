# Deep dive: `eggplan-markdown` interchange

Index: [overview](overview.md) · Normative companion: [markdown-interchange](markdown-interchange.md).
Status context: projection/CLI M002 (loss-aware Markdown import + deterministic render) is closed
(`plans/subsystems/projection-cli-roadmap.md:42-57`, registered closed with closure evidence at
`plans/registry.md:78`); fixtures re-pinned to CodeGG head
`5f4532659dbf0df2cd9f2b3bdb024217d2ea7868` (`crates/eggplan-markdown/tests/fixtures.rs:10-17`).
Roadmap M003 (ergonomics/performance, `plans/subsystems/projection-cli-roadmap.md:59`) is closed
(`plans/registry.md:58`), as is its corrective line C003 — "Remove the
only production `unreachable!()` in the workspace and name the repeated Markdown bound"
(`plans/subsystems/projection-cli-roadmap.md:104`,
`plans/implementation/projection-cli/003b-c003-markdown-interchange-panic-path-and-bound-hygiene.md`,
closed at `plans/closure/projection-cli/003b-c003-closed.md`)
— whose work is recorded in §4 and §6 below. §4 and §6 describe the state of the crate after
that line closed; the crate's own content is still not closure evidence.

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
`ImportedPlan { plan: Plan, report: ImportReport }` (`src/lib.rs:63-67`), and `Plan`
(`eggplan-core/src/model.rs:127-137`) has no observation, provider, or closure field at all — a
Markdown document has no representation to land in. A grep of `src/lib.rs` for
`std::fs|File|reqwest|TcpStream|Command|process::|env::` returns nothing, so nothing is fetched,
opened, or executed: links, includes, URLs, and paths are only ever text. Both importers
construct their plan once — native at `src/lib.rs:319-328`, CodeGG via `Plan::new` at
`src/lib.rs:664-667`, which core fixes at revision 0, `PlanStatus::Draft`, `subject: None`
(`eggplan-core/src/model.rs:275-292`) — and the CLI writes exactly that one plan
(`crates/eggplan-cli/src/lib.rs:1249-1253`), where the store re-asserts revision 0 + Draft and
rejects an existing ID (`crates/eggplan-repo/src/store.rs:930-936`).

## 2. Eggplan Markdown v1: marker, fenced payload, carried vs dropped

A native document is one version marker plus one fenced canonical payload. Render emits
`NATIVE_MARKER` (`src/lib.rs:13`) and a ` ```eggplan-plan-json ` fence (`src/lib.rs:14`)
containing pretty-printed `NativePayload` (`src/lib.rs:79-89`, `deny_unknown_fields` at
`src/lib.rs:80`; items at `src/lib.rs:91-102`), then a derived human section
(`src/lib.rs:194-247`, shape enumerated below). Import requires exactly one payload start
(`src/lib.rs:257-263`)
and exactly one marker (`src/lib.rs:264-267`), the fence on its own line
(`src/lib.rs:270-273`), a closing fence (`src/lib.rs:274-277`), parseable JSON
(`src/lib.rs:279-280`), byte-equal canonical pretty form (`src/lib.rs:281-287`), and
`format_version == 1` (`src/lib.rs:288-293`).

Carried: plan schema version, plan/item IDs, objective, ordering, parent/dependency links,
descriptions, acceptance criteria with evidence-requirement descriptors, blocker and
next-action intent, plus source revision/status as provenance (`src/lib.rs:172-193` for
render; `src/lib.rs:319-334` for import reset + provenance keys
`markdown_source_status`/`markdown_source_revision`). Absent from the native payload entirely:
observations, provider authority, live subject (`subject: None`, `src/lib.rs:327`), and any closure
record. Those dimensions are not silently discarded on the native path — `NativePayload`/`NativeItem`
have no field for them and carry `deny_unknown_fields` (`src/lib.rs:80`, `src/lib.rs:92`), so a
payload carrying `"observations"`, `"subject"`, or `"closure"` is a hard import error. The
`Closure record present: … (display only)` line is render-only (`src/lib.rs:198-203`), its
`yes`/`no` value comes from the caller's `has_closure: bool` parameter (`src/lib.rs:164`) and is
never parsed from the document, and the human section is never re-parsed. Lifecycle is reset: plan
becomes revision-zero Draft, items become Pending — or Blocked when blocker intent is present
(`src/lib.rs:301-328`).

The derived human section is fixed-shape, not free prose: `# <objective>`, then
`Plan: \`<id>\` · revision <n> · status \`<status>\``, `Items: <n>`, the closure display line, then per
item `## <id> — <description>` / `Status: \`<status>\`` / the description body again, optional
`Parent:`, sorted `Dependencies:`, `Blocker:`, `Next action:`, and `### Acceptance <id>` per
criterion with `Evidence requirement (display only): …` lines (`src/lib.rs:196-247`). Note the
description is emitted twice per item (heading and body, `src/lib.rs:209-213`) — cosmetic, but it
means a rendered item is visibly self-duplicating.

Render is deterministic: `plan.validate()` first (`src/lib.rs:165-166`), pre-render size
estimate (`src/lib.rs:167`, `820-862`), stable pretty JSON field order, items sorted by
`(position, id)` (`src/lib.rs:205-206`; a total order, because `plan.validate()` rejects duplicate
item IDs at `eggplan-core/src/model.rs:330-332`), dependencies sorted (`src/lib.rs:218-222`),
`markdown_text` escaping (`src/lib.rs:1094-1102`: `&`/`<`/`>` escaped, `\r` stripped,
`\n` → `<br>`, then a leading fence run neutralized by `escape_leading_fence`
(`src/lib.rs:1104-1114`) so a description carrying a ``` `eggplan-plan-json` fence or the
native marker cannot restate interchange structure on re-import), no
timestamps/randomness, trailing byte-cap check (`src/lib.rs:248-252`).
The render output is a pure function of `(plan, has_closure)`: `src/lib.rs` contains no clock,
random source, `HashMap`, environment read, or locale-sensitive sort, and imports only
`BTreeMap`/`BTreeSet` for ordered collection (`src/lib.rs:11`). The size pre-check deliberately
over-estimates (6 bytes per Unicode scalar, `src/lib.rs:870-875`) so the bound is checked before
encoding, with the authoritative post-render check at `src/lib.rs:248-252`.
CRLF round-trips (`src/lib.rs:278` normalizes; covered at `src/lib.rs:1135-1142`).

## 3. CodeGG subset import: section mapping, IDs, dependencies

Parsing is line-oriented with backtick/tilde fence tracking (`src/lib.rs:389-417`,
`fence_run` at `src/lib.rs:784-792`): fenced regions contribute no semantics.
`## ` headings delimit sections with a 256-section cap (`src/lib.rs:418-428`); `Status:`
and `Repository baseline:` are read **only before the first `##` section**
(`src/lib.rs:433-454`), with conflicting duplicates rejected (`src/lib.rs:437-451`).
H1 (fence-aware, `# `-only, `src/lib.rs:845-865`) is the objective fallback when no
Objective section exists (`src/lib.rs:473-485`); an empty Objective body falls back to
`"Imported plan objective"` (`src/lib.rs:974-981`) and a document with no H1 at all to
`"Imported implementation plan"` (`src/lib.rs:394`); the resolved objective is finally bounded to
`bounds::OBJECTIVE_CHARS` (`src/lib.rs:664`).

Work packages come from `## … Ordered work packages` / `Work packages`
(`src/lib.rs:955-960`) with `### Work package <label> — <title>` headings parsed at
`src/lib.rs:937-953` (em-dash or hyphen split, label bounded at 64 **bytes** — `label.len() > 64`,
`src/lib.rs:949`; a shorter multi-byte label can therefore be rejected) and capped at
`bounds::MAX_ITEMS` = 512 packages (`src/lib.rs:456-461`, re-checked at `src/lib.rs:492-496`).
IDs are
`epi_md_<24 hex>` over `identity + NUL + normalized label + NUL + normalized heading`
(`src/lib.rs:508-514`,
`normalize_heading` at `src/lib.rs:1010-1016`, `normalize_label` shared with the heading
and dependency readers so all three backtick-normalize identically); plan ID is
`ep_md_<24 hex>` over the
caller-provided source name or else the document digest (`src/lib.rs:486-490`,
`short_hash`/`sha256` at `src/lib.rs:1018-1023`). Duplicate labels are rejected
(`src/lib.rs:506-508`); generated hash collisions are hard errors (`src/lib.rs:515-517`,
acceptance-item collision at `src/lib.rs:642-646`). Mixing the label in is what keeps
two differently-labeled packages whose titles normalize identically (case/whitespace
variants) from colliding, since duplicate labels are already rejected.

Acceptance sections match `## N. Acceptance criteria` **and any heading whose normalized name ends
in ` acceptance criteria`** (`src/lib.rs:969-972`), so CodeGG's `## 8. Binary acceptance criteria`
is imported (exercised by `crates/eggplan-markdown/tests/fixtures/corrective-plan.md:351`). Only
`## ` at column 0 opens a section and only `### Work package ` inside a work-package section opens
a package (`src/lib.rs:418`, `src/lib.rs:456`), so deeper headings (`#### …`) and `### …` under any
other section are body text. Every other `## ` section is **dropped by title and named in the loss
report** (`src/lib.rs:731-746`): the implementation is a three-entry whitelist (Objective, work
packages, acceptance criteria), not a blacklist, so Scope, Findings, Verification, Stop conditions,
Handoff notes, Required design, and every other CodeGG section are instances of the same rule.
Their body text is discarded unread.

`Dependencies:` / `Depends on:` lines split on commas, strip backticks, lowercase
(`src/lib.rs:518-534`); empty entries are malformed (`src/lib.rs:525-527`). Unknown
labels fail (`src/lib.rs:547-549`), self-edges fail (`src/lib.rs:550-552`), and
multi-node cycles fail at the final `plan.validate()` via core graph validation
(`src/lib.rs:757-758`; cycle errors in `eggplan-core/src/graph.rs:6-11,36-43`). Duplicate
references (`Dependencies: A, A`) are rejected too, by core's per-item duplicate check
(`eggplan-core/src/model.rs:264-269`), and every edge target must resolve to a known item
(`eggplan-core/src/model.rs:336-346`). Order
alone never creates edges: `dependencies` is populated only from those two line prefixes
(`src/lib.rs:518-534`) plus the synthetic plan-level intent item's edges (`src/lib.rs:651`).
Acceptance criteria attach per `Work package <label>: …`
prefix or become plan-level intent collected into one trailing item depending on all
work packages (`src/lib.rs:588-662`, ID at `src/lib.rs:637-640`, plan-item-limit guard
at `src/lib.rs:632-636`; list markers stripped at `src/lib.rs:999-1011`, capped at
`bounds::MAX_CRITERIA` = 128 in `src/lib.rs:983-997`). Criterion IDs are a third ID family,
`epc_md_<24 hex>` over `identity + NUL + "acceptance" + NUL + statement index + NUL + bounded
statement` (`src/lib.rs:611-615`) — position- and text-sensitive, unlike the work-package IDs, and
unlike them they are not covered by the `used` collision guard (only item IDs enter it,
`src/lib.rs:502`, `src/lib.rs:515`, `src/lib.rs:642`). All items are Pending with empty
requirements (`src/lib.rs:567`, `src/lib.rs:618`); status/revision land in provenance truncated to
`bounds::PROVENANCE_CHARS` = 2,000 chars (`src/lib.rs:669-692`), while the same two values in the
report are capped at the 512-character imported-source text bound
(`src/lib.rs:669-680`) — see finding 6.

## 4. Bounds and `ImportReport` loss codes as implemented

Input bounds: 16 MiB, 20,000 lines, 64 KiB per line, NUL rejection, UTF-8 required
(`src/lib.rs:15-18`, enforced at `src/lib.rs:765-785`). Imported source text is bounded by one
named constant, `MAX_SOURCE_TEXT_CHARS` = 512 characters (`src/lib.rs:20-29`), applied at the
source-name check (`src/lib.rs:144-148`), the report's source status and source revision
(`src/lib.rs:669-680`), the unmapped section titles (`src/lib.rs:739-744`), and the report's
source name and truncation flag (`src/lib.rs:924`, `src/lib.rs:933`); the "one past the limit"
probe is derived from that constant (`src/lib.rs:1051-1053`) rather than written separately.
Core text/graph/item limits are applied through `truncate_chars` (`src/lib.rs:1063-1072`) and
`Plan::validate`. Ambiguous duplicate Objective / Acceptance sections are rejected
(`src/lib.rs:476-484`, `src/lib.rs:579-586`); unterminated fences are rejected
(`src/lib.rs:473-475`).

`ImportReport` (schema version 1 set in `report_base`, `src/lib.rs:921`; struct with
`deny_unknown_fields`, `src/lib.rs:45-61`) carries source
format/version/name, lifecycle/revision provenance, generated IDs, preserved/dropped
fields, lossy mappings, warning codes, and truncation. The complete set is seven warning
codes — `source_lifecycle_not_authoritative`, `source_revision_provenance_only`,
`markdown_evidence_not_imported`, `markdown_closure_not_imported`, `unmapped_section`,
`text_truncated`, `generated_item_id` — and three lossy mappings:
`item_status_reset_to_pending_or_blocked_from_blocker_intent` and, when the source was not
`draft`, `source_plan_status_reset_to_draft` (native only, `src/lib.rs:359-366`), plus
`ordered_sections_not_dependency_edges` (CodeGG only, `src/lib.rs:722-724`). The normative
companion lists that last one among "warning codes"
(`architecture/markdown-interchange.md:74-77`) although it is a lossy mapping. Native reports
(`src/lib.rs:335-369`) preserve IDs/order/links/descriptions/criteria-requirements/
blocker-next-action and always warn `source_lifecycle_not_authoritative`,
`source_revision_provenance_only`, `markdown_evidence_not_imported`,
`markdown_closure_not_imported`. CodeGG reports (`src/lib.rs:668-755`) preserve
objective/order/text (+ criteria/dependencies when present), drop unmapped section
titles (512-char truncated, `src/lib.rs:739-744`) plus a collapsed
`fenced_code_block_contents` entry (`src/lib.rs:747-752`), always map
`ordered_sections_not_dependency_edges`, warn `generated_item_id` /
`markdown_evidence_not_imported` / `markdown_closure_not_imported` (+ lifecycle codes
only when that metadata exists), and sort/dedup warning codes (`src/lib.rs:757-758`).
The exact-code vector is pinned by test (`src/lib.rs:1279-1307`).

Production panic audit: outside `#[cfg(test)]` (`src/lib.rs:1123-1470`) there is no `expect`, no
`panic!`, no `unreachable!()`, no `unwrap`, no indexing panic and no slicing panic reachable
from attacker input. The import-format dispatch is total: `ImportFormat::Auto` is a *request*,
not a resolution, so it is resolved once — before the dispatch — into a private
`ResolvedFormat` (`src/lib.rs:104-114`) whose type has no `Auto` variant. The dispatch match
(`src/lib.rs:153-161`) is therefore exhaustive by construction: adding an `ImportFormat`
variant is a compile error, not a panic arm. `detect_format` is a thin wrapper over the same
private detector (`src/lib.rs:120-136`), so it can never report `Auto` either.

The two remaining indexing sites are provably guarded: `starts[0]` at `src/lib.rs:268`
(guarded by `starts.len() != 1` at `src/lib.rs:259`), and `markers[0]` at `src/lib.rs:265`
(guarded on the same line by `markers.len() != 1`). The section index is no longer unwrapped:
the CodeGG parser binds it with `let Some(current) = current else { … }`
(`src/lib.rs:436-455`), so `sections[current.unwrap()]` is gone from the import path — the
"two-block coupling" fragility this document previously named no longer exists. Every slice in
the crate is a char-boundary-safe offset: `fence_run` slices after a run of ASCII markers
(`src/lib.rs:791`), `native_payload_starts` accumulates whole lines (`src/lib.rs:840`) and adds
a 20-byte ASCII fence to a line whose prefix is whitespace, and `strip_list_marker` slices
after ASCII digits (`src/lib.rs:1000-1002`).

## 5. Test strategy

Unit tests (`src/lib.rs:1123-1470`): native determinism + round-trip + CRLF
(`1151-1174`), lifecycle reset matrix incl. Closed source (`1176-1216`), parent/deps/
criteria-requirement round-trip (`1218-1242`), duplicate-payload / unknown-field /
bad-version rejection (`1244-1257`), CodeGG determinism without order edges
(`1259-1278`), exact warning-code + truncation pin (`1280-1308`), explicit-dep mapping
and unknown/malformed/ambiguous rejection (`1310-1324`), scoped acceptance routing
(`1326-1336`), backticked-label normalisation (`1338-1352`), distinct labels with equal
normalised titles (`1354-1364`), rendered-payload fence and native-marker re-import
(`1366-1391`), inline code left untouched (`1393-1399`), dropped source provenance
(`1401-1416`), NUL/oversize rejection (`1418-1424`), plus the bound-hygiene pins
(`1426-1470`: the 512-character constant, the derived one-past-the-limit probe, the
report-side source-name bound, and the `truncate_chars` return shape). Nothing in the unit tests asserts the CodeGG `subject` is `None` —
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

`tests/bound_hygiene.rs` (`crates/eggplan-markdown/tests/bound_hygiene.rs:125-351`) covers the
panic-path and bound-hygiene pass: every `ImportFormat` value through the public `import`
entry point (`125-171`), `detect_format` never reporting `Auto` (`181-190`), the
512-character bound at every site that applies it, at exactly 512 and 513 characters
(`199-268`), the imported identifiers against the recorded fixture (`277-287`), and the bytes
of every imported plan and rendered document against the recorded golden digests
(`296-308`, `333-351`). The recorded goldens are
`crates/eggplan-markdown/tests/fixtures/golden/`: `imported-identifiers.json` (the imported
plan and criterion IDs per corpus fixture), `import-render-digests.json` (the digest of each
imported plan and rendered document), and the byte-level anchor `ordinary.import.json` /
`ordinary.render.md`, each with a `.sha256` sidecar over its exact bytes. They were captured
from the pre-pass implementation, so any change to an imported plan, a generated identifier,
a loss code, or a rendered document fails the suite.

## 6. Review findings

Strengths: genuinely dependency-free bounded design; fence-aware scanning in every
entry point (`detect_format`, both importers, H1 scan); strict native canonical-JSON
equality plus `deny_unknown_fields` on both payload and report; preamble-conflict
rejection instead of last-wins; ID determinism tests; display-only closure handling;
a single-`Plan` output type that makes evidence/trust/subject/closure structurally
unrepresentable; and no production panic path reachable from hostile input (see §4).

Gaps/risks (all verified against source, not speculation):

1. **Native `ImportReport` never populates `generated_ids`.** `report_base` starts it
   empty (`src/lib.rs:925-927`) and the native path (`src/lib.rs:335-369`) pushes only
   preserved/dropped/warning/lossy entries — unlike CodeGG (`src/lib.rs:668-755`).
   `dropped_fields` is now populated for the one dimension the payload can actually
   carry and discard: non-`markdown_source_*` `plan.provenance`, reported as
   `source_plan_provenance` (`src/lib.rs:349`). The ignored human section is therefore
   still unnamed. The
   observations/policy/subject/closure dimensions cannot be named at all: the payload has no
   such fields and `deny_unknown_fields` (`src/lib.rs:80`) makes an attempt a hard error, so the
   report has nothing to record even though [markdown-interchange](markdown-interchange.md)
   promises dropped fields are named. The human section remains the real residual gap:
   the payload has no field for it and `deny_unknown_fields` makes recording it a hard
   error, so it can be neither carried nor reported. If unsure whether that is intentional,
   say so — but as implemented the asymmetry is real.
2. **CodeGG `Status:`/baseline parse position is narrower than the doc table implies.**
   The interchange doc lists them as imported metadata; the code only reads them
   before the first `##` (`src/lib.rs:433-454`) — the same lines later in the file
   become section body text (CodeGG path) or acceptance-statement text. Metadata
   placed after a heading is silently kept as prose rather than rejected or recorded.
3. **ID-collision handling is fail-closed with no recovery hint.** A genuine hash
   collision between two work packages is a hard `generated work-package ID
   collision` error (`src/lib.rs:515-517`); same for the acceptance-intent item
   (`src/lib.rs:642-646`). The error names no colliding headings. Distinct packages
   whose *titles* normalize identically (case/whitespace, `src/lib.rs:1010-1016`) no
   longer collide, because the normalized label is mixed into the ID
   (`src/lib.rs:508-514`) and duplicate labels are rejected before that. Plan-level IDs
   additionally depend on caller-provided `source_name`, so two unrelated documents
   imported under the same name share IDs — collision rejection then lives outside
   this crate (CLI/repo layer, `crates/eggplan-repo/src/store.rs:931-933`). On the native
   path the situation is stronger: the document supplies `plan_id` verbatim
   (`src/lib.rs:84`, `src/lib.rs:321`) with no hash and no cross-check against the caller, so
   any syntactically valid `PlanId` can be claimed and only the store's `AlreadyExists`
   rejection stands between two documents. That is the intended ownership split, but the
   crate offers no way to detect the impersonation.
4. **Dead branch in `acceptance_lines` (`src/lib.rs:991-993`).** When there is exactly
   one statement longer than 2,000 bytes, `values.truncate(1)` is a no-op; real
   bounding happens later per statement via `CRITERION_CHARS` (`src/lib.rs:609`, 2,000
   Unicode scalars). The dead branch also compares a **byte** length against a limit that is
   applied in **chars**, so a ~900-character non-ASCII statement trips the branch and is yet
   not truncated. Intent is unclear — likely leftover — and deserves a comment or removal.
5. **Former doc drift, now resolved: `Depends on:` alias.** The importer accepts it
   alongside `Dependencies:` (`src/lib.rs:519-523`) and the normative companion's table
   does name the alias (`architecture/markdown-interchange.md:44`). Recorded here only
   because an earlier revision of this deep dive claimed the opposite; no action remains.
6. **The same metadata is bounded differently in the plan and in the report.**
   `Repository baseline:`/`Status:` go into `plan.provenance` through
   `truncate_chars(…, bounds::PROVENANCE_CHARS /* 2,000 */)` (`src/lib.rs:681`, `src/lib.rs:687`)
   but into `report.original_source_status`/`original_source_revision` through the
   512-character imported-source text bound (`src/lib.rs:669-680`). A 600-character baseline is
   therefore preserved in full in the canonical Plan and silently halved in the loss report.
   The 512 figure is now one named constant (`MAX_SOURCE_TEXT_CHARS`, `src/lib.rs:20-29`) shared
   by the source name (`src/lib.rs:924`), the report's source status and revision
   (`src/lib.rs:669-680`), and the unmapped section titles (`src/lib.rs:739-744`), so the
   *number* is no longer a repeated magic literal — but the *asymmetry* between the two bounds
   is unchanged. Loss reporting should not under-report what the Plan kept.
7. **Criterion IDs are position- and text-sensitive and bypass the collision guard.**
   `epc_md_<24 hex>` hashes `identity + NUL + "acceptance" + NUL + index + NUL + bounded
   statement` (`src/lib.rs:611-615`), unlike work-package IDs which hash only the normalized
   heading. Re-wording or re-ordering acceptance lines therefore mints new criterion IDs for
   otherwise unchanged intent, and because the *bounded* statement is hashed, a line that
   differs only past the 2,000-char criterion bound keeps its ID — a stability property worth
   knowing, but an implicit one. The `used` collision set covers only item IDs
   (`src/lib.rs:502`, `src/lib.rs:515`, `src/lib.rs:642`), and core's duplicate-criterion check is
   per item (`eggplan-core/src/model.rs:257-263`), so a 96-bit collision between criteria
   attached to *different* work packages would be silently accepted rather than rejected. Low
   likelihood, but work-package item IDs get the explicit guard and these do not.
8. **Rendered items print their description twice.** The derived human section emits the
   description in the `## <id> — <description>` heading and again as the item body
   (`src/lib.rs:208-214`). Harmless for import (the section is never re-parsed) but it makes
   every rendered item self-duplicating, and a long description is escaped and copied twice
   into the output budget.

## Verification pointers

```sh
cargo test -p eggplan-markdown --locked
cargo test -p eggplan-markdown --locked --test fixtures
bash scripts/check-projection-cli-boundary.sh
```
