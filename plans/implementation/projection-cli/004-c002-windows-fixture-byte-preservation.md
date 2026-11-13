# Projection/CLI M004 C002 — Windows Fixture Byte Preservation

Status: active

Repository baseline: `ee54a0b8e2cb0d843565c54930ad2c21111320db`

Source roadmap: plans/subsystems/projection-cli-roadmap.md

Related plan/closure: M004 C001 —
`plans/implementation/projection-cli/004-c001-cli-help-subcommand-fidelity.md`,
`plans/closure/projection-cli/004-c001-closed.md`.

Primary class: non-blocking cross-platform test infrastructure

## 1. Objective

Preserve the exact LF bytes of Markdown interchange fixtures when Git checks
out the repository on Windows. The recorded fixture digests and byte-level
anchors must remain valid on every supported CI runner.

## 2. Finding

Hosted CI run `40206104047` passed on Ubuntu, macOS, and MSRV. On Windows,
`cargo test --workspace --locked` failed in
`eggplan-markdown/tests/bound_hygiene.rs`:

- `recorded_golden_files_match_their_digest_sidecars` computed a different
  digest for `imported-identifiers.json`;
- `the_byte_level_anchor_still_matches_the_imported_plan_and_rendered_document`
  observed CRLF in the checked-out Markdown golden while its recorded bytes use
  LF.

The fixture content and digests are intentionally byte-level contracts. The
failure is caused by Git's Windows line-ending conversion, not by a semantic
change in Markdown import or rendering.

## 3. Scope

In scope:

- add repository attributes that keep Markdown interchange fixture files at LF
  in Windows worktrees;
- verify the attributes cover the corpus, golden anchors, and digest sidecars.

Out of scope:

- changing fixture bytes, digest values, production behavior, or test semantics;
- changing the CI matrix or disabling byte-level assertions.

## 4. Required changes

1. Add the narrowest `.gitattributes` rule that guarantees LF checkout for all
   files under `crates/eggplan-markdown/tests/fixtures/`.
2. Keep all fixture contents and `.sha256` sidecars byte-identical in Git.
3. Confirm Git reports the LF attribute for representative corpus, golden, and
   sidecar files.

## 5. Verification

- `cargo test -p eggplan-markdown --locked --test bound_hygiene`
- `cargo test --workspace --locked`
- `git diff --check`
- hosted CI on the pushed commit, including the native Windows job.

The hosted run is required to close this corrective line. Record its exact run
ID and result in the closure evidence.

## 6. Acceptance criteria

1. The fixture-tree Git attribute forces LF on Windows checkouts.
2. No fixture or digest-sidecar bytes change in the commit.
3. The focused Markdown byte-level tests and workspace suite pass locally.
4. Hosted Windows CI passes on the pushed commit.
