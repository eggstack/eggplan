# Projection/CLI M004 C002 — Windows Fixture Byte Preservation — Closed

Status: closed

Source plan: plans/implementation/projection-cli/004-c002-windows-fixture-byte-preservation.md

Source roadmap: plans/subsystems/projection-cli-roadmap.md

Reviewed baseline: `ee54a0b8e2cb0d843565c54930ad2c21111320db`.

Implementation commit: `bccebb9083ec0ac6622bfb74b452508bf8628bf2`.

Hosted CI: [run 43513363952](https://github.com/eggstack/eggplan/actions/runs/43513363952) — success on MSRV, Ubuntu, macOS, and Windows.

## Executive finding

Projection/CLI M004 C002 closes. The Windows job now preserves the Markdown
fixture bytes that the digest sidecars and render anchors pin. The fix is a
single `.gitattributes` rule, scoped to
`crates/eggplan-markdown/tests/fixtures/**`, setting `text eol=lf`.

The failure in hosted run `40206104047` was caused by Windows Git checkout
converting fixture text to CRLF. Two tests then failed: digest verification for
`imported-identifiers.json` and the byte-level rendered-document anchor. The
fixtures and sidecars were not regenerated or edited; LF is now enforced at
checkout.

## Requirement-to-evidence matrix

| Plan requirement | Evidence | Status |
|---|---|---|
| The fixture-tree Git attribute forces LF on Windows checkouts | `.gitattributes` contains `crates/eggplan-markdown/tests/fixtures/** text eol=lf`; `git check-attr` reports `text: set`, `eol: lf` for the corpus, rendered golden, and `.sha256` sidecar; hosted Windows `actions/checkout@v4` followed by the full test suite passed | met |
| No fixture or digest-sidecar bytes change | `git diff --exit-code ee54a0b8e2cb0d843565c54930ad2c21111320db bccebb9083ec0ac6622bfb74b452508bf8628bf2 -- crates/eggplan-markdown/tests/fixtures/` is empty | met |
| Focused Markdown byte-level tests and workspace suite pass locally | `cargo test -p eggplan-markdown --locked --test bound_hygiene` — 7 passed; `cargo test --workspace --locked` — 348 passed, 3 ignored | met |
| Hosted Windows CI passes on the pushed commit | Run `43513363952`, head `bccebb9083ec0ac6622bfb74b452508bf8628bf2`; Windows check, clippy, and workspace test steps passed. The complete matrix finished green on MSRV, Ubuntu, macOS, and Windows. | met |

## Implementation evidence

`.gitattributes` adds:

```gitattributes
crates/eggplan-markdown/tests/fixtures/** text eol=lf
```

The rule applies to source Markdown fixtures, JSON golden files, rendered
Markdown anchors, and digest sidecars in the fixture subtree. No production
Rust code or golden content changed. It leaves line-ending policy elsewhere in
the repository untouched.

## Verification executed

| Command or evidence | Result |
|---|---|
| `git check-attr text eol --` on corpus, render golden, and sidecar | `text: set`, `eol: lf` for each path |
| `cargo test -p eggplan-markdown --locked --test bound_hygiene` | pass — 7 passed, 0 failed |
| `cargo test --workspace --locked` | pass — 348 passed, 3 ignored |
| `git diff --check` | pass |
| Hosted CI run `43513363952` | pass — MSRV and native Ubuntu/macOS/Windows jobs all green; Windows workspace tests passed |

## Invariant and failure review

The byte-level golden contract remains unchanged: sidecars still hash the
existing fixture bytes, and rendered output continues to be compared byte for
byte. Git now makes the fixture checkout deterministic on Windows. The failed
run remains historical failure evidence; run `43513363952` verifies the
correction.

No runtime, repository-state, trust, evidence, security, or path behavior
changed. No migration or compatibility handling is needed. The attributes rule
is limited to Markdown interchange test fixtures. The user-facing CLI and
documentation are unchanged.

## Unresolved findings

None for this corrective line. No other fixture trees are claimed by this
change or included in its closure scope.

## Roadmap and registry disposition

Projection/CLI M004 C002 is closed. Its registered status and the roadmap
closure pointer were updated with this record. Existing M004 C001 and all
earlier milestones remain unchanged.
