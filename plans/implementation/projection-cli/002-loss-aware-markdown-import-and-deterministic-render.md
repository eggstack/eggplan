# Projection and CLI M002 — Loss-Aware Markdown Import and Deterministic Render

Status: closing

Repository baseline: bfbec4a77d1e35e0a422af1edc9c3f358f7ad982

Source roadmap:

- plans/subsystems/projection-cli-roadmap.md

Predecessor closure:

- plans/closure/projection-cli/001-closed.md
- plans/closure/evidence-closure/002-c002-closed.md

Current CodeGG planning baseline reviewed:

- dbowm91/codegg @ 5f4532659dbf0df2cd9f2b3bdb024217d2ea7868
- representative implementation-plan shape:
  plans/implementation/long-horizon-work-execution/002-durable-work-plan-foundation.md

The CodeGG head was re-checked at handoff. Its representative document still
uses H1 title, top-level `Status:`/`Repository baseline:` metadata, numbered
Objective, Ordered work packages with `### Work package <label> — <title>`
subsections, and numbered Acceptance criteria. This is an interface review
baseline, not a runtime dependency.

Primary class: capability / compatibility / projection

## 1. Objective

Add a bounded Markdown interchange surface without making Markdown canonical
state.

M002 provides:

1. deterministic Eggplan-native Markdown rendering;
2. lossless re-import of the Eggplan-native intent payload;
3. a strict, documented importer for a useful subset of CodeGG-style
   implementation-plan Markdown;
4. explicit loss/unmapped-field reporting;
5. CLI commands for render/import/inspect.

Markdown import must never manufacture EvidenceObservation, provider trust,
ClosureRecord, or a canonical Closed Plan.

## 2. Ownership and authority

Canonical authority remains:

    eggplan-core typed Plan
      + eggplan-repo canonical state
      + immutable evidence/closure records

Markdown is only:

- a derived human projection; or
- an import source for plan intent.

A Markdown document cannot establish:

- passed evidence;
- trusted provider identity;
- current SubjectRevision;
- closure;
- successful verification;
- a historical observation.

Imported lifecycle state is advisory provenance only. New canonical imports
start as Draft.

## 3. New module/crate boundary

Prefer a dedicated `eggplan-markdown` crate rather than expanding
`eggplan-projection` into a parser.

Recommended dependencies:

- `eggplan-core`;
- `serde` / `serde_json`;
- std only otherwise.

Do not add a general Markdown AST/parser dependency in M002 unless the strict
line-oriented parser proves insufficient for the documented grammar.

`eggplan-projection` remains read-only structured projection.
`eggplan-markdown` owns interchange grammar, rendering, import reports, and
format detection.

The CLI consumes both.

## 4. Eggplan-native Markdown v1

Define an explicit format marker, for example:

    <!-- eggplan-markdown:v1 -->

The document contains two layers:

1. an authoritative-for-import bounded machine payload containing Plan intent;
2. a derived human-readable projection.

Use a fenced canonical JSON payload with an explicit language/tag, for example:

    ```eggplan-plan-json
    {...}
    ```

The exact tag may differ, but it must be unique and versioned.

### Native payload rules

The embedded payload may contain:

- plan ID;
- objective;
- items;
- item ordering/parent/dependencies;
- item descriptions;
- acceptance criteria and evidence requirements;
- blocker/next-action text;
- source revision/status as informational provenance.

It must not contain/import as authority:

- EvidenceObservation objects;
- supersession records;
- provider-policy authority;
- ClosureCandidate/ClosureRecord;
- live Git SubjectRevision authority.

On import:

- validate strict Eggplan Plan schema;
- preserve IDs when valid and non-conflicting;
- force new canonical lifecycle to Draft/revision 0;
- return original revision/status as import provenance;
- report the lifecycle downgrade as an explicit informational loss when the
  source was not Draft.

If the payload contains a Closed Plan, import its intent as Draft only. Never
call guarded closure from Markdown import.

## 5. Deterministic native rendering

Rendering must be stable for identical canonical input.

Human section should include at minimum:

- H1 objective/title;
- plan ID/revision/status;
- item counts;
- per-item status/description;
- dependencies and parent;
- blocker/next action;
- acceptance criteria;
- evidence requirement descriptors;
- closure-present flag as display-only information when rendering repository
  state.

Do not embed observation payloads by default.

If evidence summaries are optionally rendered, they must be bounded,
read-only, and clearly marked as derived display. They are not imported back
as evidence.

No ANSI, timestamps generated at render time, random IDs, or locale-dependent
ordering.

## 6. CodeGG-style implementation-plan subset

Support an explicitly bounded subset of the planning-document style present at
the reviewed CodeGG baseline.

Recognize:

- H1 title;
- top-level `Status:` line as provenance;
- `Repository baseline:` as provenance;
- `## N. Objective` body;
- `## N. Ordered work packages` section;
- `### Work package ...` subsections;
- `## N. Acceptance criteria`;
- optional explicit `Hard dependency:` / dependency declarations when their
  target can be resolved unambiguously;
- `## ... Scope`, `Explicitly out of scope`, `Stop conditions`,
  `Verification`, and `Handoff notes` as source metadata/loss-report
  material, not executable semantics.

Do not attempt to parse arbitrary Markdown prose into hidden semantics.

### Mapping

Recommended mapping:

- Objective section -> Eggplan objective.
- Each Work package subsection -> one PlanItem.
- Stable item IDs -> deterministic hash of source identity + normalized
  work-package heading, with collision detection.
- Work-package body -> bounded description/next-action projection.
- Explicit dependency declarations -> PlanItem dependencies only when exact.
- Acceptance criteria section -> a synthetic final verification/closure-intent
  item depending on all imported work packages, unless criteria are explicitly
  scoped to a named work package.
- Source `Status:` -> import provenance only; canonical Plan remains Draft.

Do not infer dependencies merely because work packages are listed in order.
Report `ordered_sections_not_dependency_edges` when order is preserved but
not promoted to dependency semantics.

## 7. Import loss report

Return a versioned `ImportReport` with at least:

- source format/version;
- source path/name when supplied;
- imported plan ID;
- original source status/revision if present;
- generated IDs;
- preserved fields;
- dropped/unmapped fields;
- lossy mappings;
- warning codes;
- truncation flags.

Stable loss codes should include at minimum:

- `source_lifecycle_not_authoritative`;
- `source_revision_provenance_only`;
- `markdown_evidence_not_imported`;
- `markdown_closure_not_imported`;
- `ordered_sections_not_dependency_edges`;
- `unmapped_section`;
- `text_truncated`;
- `generated_item_id`.

Loss reporting must be deterministic and bounded.

## 8. Parser safety and bounds

Apply explicit byte/line/section/text/item bounds before allocation-heavy work.

Reject:

- NUL;
- oversized input;
- duplicate native machine payload blocks;
- multiple conflicting Objective sections;
- duplicate generated item IDs;
- malformed dependency references;
- unsupported native format versions;
- malformed embedded canonical JSON;
- hidden/ambiguous second authoritative payload.

Do not fetch includes, URLs, local files, or execute code fences.

Markdown is data only.

## 9. CLI surface

Add a narrow command family, naming may be adjusted for consistency:

    eggplan markdown render PLAN_ID [--output FILE] [--json]
    eggplan markdown import FILE [--format eggplan|codegg|auto] [--json]
    eggplan markdown inspect FILE [--format eggplan|codegg|auto] [--json]

### render

- read-only except optional requested output file;
- repository state remains unchanged;
- output stable native v1 Markdown.

### inspect

- parse and return proposed Plan intent + ImportReport;
- never mutates repository.

### import

- parse first;
- require explicit target state root;
- create a new Draft Plan through repository APIs;
- reject ID collision rather than overwrite;
- no evidence writes;
- no closure writes;
- no provider policy.

JSON output uses existing versioned CLI envelope.

## 10. Roundtrip contract

Native render -> inspect/import must preserve plan intent fields exactly except
the deliberate lifecycle reset:

- ID;
- objective;
- item IDs/order;
- parent/dependencies;
- descriptions;
- acceptance criteria/evidence requirements;
- blocker/next action.

Revision/status/subject/closure are not roundtripped as canonical authority.

Render -> render after importing into a fresh repository may differ only in
declared lifecycle/provenance fields that are intentionally reset.

## 11. CodeGG fixture qualification

Pin fixture provenance to current reviewed CodeGG head:

- `a3c87fc18ee55aaf630401a562c11bb83112fd82`.

Include at least three planning documents:

1. ordinary implementation plan with ordered work packages;
2. corrective plan with explicit findings/acceptance criteria;
3. plan containing sections intentionally outside the supported subset.

Golden tests must assert:

- deterministic imported Plan intent;
- exact loss codes;
- no evidence/closure creation;
- no invented dependency edges;
- stable re-render.

## 12. Repository mutation semantics

Import is ordinary plan creation only.

- revision starts at 0;
- status starts Draft;
- no automatic activation;
- no merge/overwrite in M002;
- collisions fail explicitly;
- import failure leaves no partial Plan.

Do not add Markdown as a second persistence layer under `.eggplan`.

## 13. Documentation

Add:

- `architecture/markdown-interchange.md`;
- grammar/version documentation;
- CLI examples;
- supported CodeGG subset table;
- explicit non-goals/loss table.

Update `architecture/cli-control-surface.md` and README.

## 14. Required tests

### Native format

- render determinism;
- strict version marker;
- embedded payload strict parsing;
- Draft/Active/Blocked/Closed source lifecycle -> Draft import + provenance;
- plan intent roundtrip;
- collision failure;
- oversized/malformed/duplicate payload rejection.

### CodeGG subset

- representative current fixtures;
- work-package mapping;
- deterministic IDs;
- no implicit dependency inference;
- explicit dependency mapping;
- plan-level acceptance synthetic final item;
- unsupported sections reported, not silently discarded;
- source status never becomes canonical Closed/Completed evidence.

### Security/authority

- Markdown containing "tests passed" creates no observation;
- Markdown containing fake ClosureRecord JSON outside the native intent payload
  creates no closure;
- Markdown cannot enroll provider trust;
- Markdown URLs/includes are not fetched;
- code fences are not executed.

### CLI

- human and JSON output;
- inspect is read-only;
- import creates exactly one Draft Plan;
- failed import leaves repository unchanged;
- Windows/macOS/Linux path handling.

## 15. Verification

At minimum:

    cargo fmt --all -- --check
    cargo check --workspace --all-targets --locked
    cargo clippy --workspace --all-targets --locked -- -D warnings
    cargo test --workspace --locked
    cargo +1.89.0 check --workspace --all-targets --locked
    cargo +1.89.0 test --workspace --locked
    bash scripts/check-core-boundary.sh
    bash scripts/check-projection-cli-boundary.sh
    bash scripts/check-closure-authority-boundary.sh
    git diff --check

Native Linux/macOS/Windows and Rust 1.89 hosted CI must pass.

## 16. Acceptance criteria

M002 closes when:

1. Eggplan-native v1 Markdown renders deterministically;
2. native import preserves intent but never lifecycle/evidence/closure authority;
3. the documented CodeGG planning subset imports with deterministic loss
   reporting;
4. unsupported prose never becomes hidden semantics;
5. CLI render/inspect/import are bounded and machine-readable;
6. no Markdown path can manufacture passing evidence or closure;
7. native roundtrip and CodeGG golden fixtures pass;
8. no new network/process dependency enters the Markdown crate;
9. native/MSRV CI passes.

## 17. Stop conditions

Stop and report if:

- useful import requires arbitrary Markdown interpretation;
- a parser would need to infer evidence from prose;
- import semantics require overwriting existing Plans;
- CodeGG compatibility would require treating source `Status: implemented`
  as Eggplan completion;
- a heavy Markdown runtime becomes necessary without a clear bounded benefit.

## 18. Closure evidence

Record:

- exact format grammar/version;
- supported CodeGG baseline and fixture paths;
- roundtrip matrix;
- loss-code matrix;
- no-evidence/no-closure negative tests;
- CLI fixtures;
- input bounds;
- implementation SHA;
- hosted native/MSRV workflow IDs;
- residual findings and M003 disposition.

## 19. Implementation follow-through

- Added `eggplan-markdown` as a bounded core-facing parser/render crate; the
  CLI remains the repository adapter. No Markdown AST, network, process, or
  persistence dependency was added.
- Added native Markdown v1 with strict canonical JSON intent payloads,
  deterministic rendering, Draft/revision-zero import, and display-only
  lifecycle/closure provenance.
- Added the documented CodeGG subset, deterministic IDs, explicit dependency
  references, plan-level/scoped acceptance mapping, bounded `ImportReport`,
  and fail-closed malformed/ambiguous input handling.
- Added `markdown render|inspect|import`; import requires an explicit state
  root and creates exactly one new Draft Plan through repository APIs.
- CodeGG fixtures are pinned in
  `crates/eggplan-markdown/tests/fixtures/manifest.json` to
  `5f4532659dbf0df2cd9f2b3bdb024217d2ea7868` and cover ordinary, corrective,
  and unmapped-section documents.
- Local Linux verification passed: formatting, workspace check, clippy,
  118 workspace tests, 3 doc compile-fail tests, Rust 1.89 check/tests, core,
  projection/CLI and closure-authority boundary guards, and `git diff --check`.
- Hosted native/MSRV qualification is pending the implementation push and
  will be recorded in the closure record.
