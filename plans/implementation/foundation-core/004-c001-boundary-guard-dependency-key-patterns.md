# Foundation Core M004 C001 — Boundary Guard Dependency-Key Patterns Must Model the Manifest Syntax the Repository Uses

Status: closed

Repository baseline: ee54a0b8e2cb0d843565c54930ad2c21111320db

Source roadmap:

- plans/subsystems/foundation-core-roadmap.md

Long-term requirements:

- plans/000-long-term-specification.md (core is pure; no fs, Git, process, or net)

Related ADRs: ADR-0002.

Predecessor plan/closure:

- plans/implementation/foundation-core/003-c005-core-boundary-guard-and-tooling-governance-reconciliation.md
- plans/closure/foundation-core/003-c005-closed.md
- plans/implementation/codegg-integration/003-c002-boundary-guard-completeness.md
- plans/closure/codegg-integration/003-c002-closed.md

Primary class: tooling governance / boundary enforcement

## 1. Objective

Make the boundary guards' dependency rules fire on every way this repository
actually declares a dependency.

The guards are the repository's mechanism for keeping pure crates pure and
adapters authority-free. Seven of their manifest rules match a dependency
name only when the name is immediately followed by whitespace, `=`, or the end
of the line. This repository declares **every** shared dependency in the
workspace-inheritance form `name.workspace = true`, where a `.` follows the
name. A dependency declared that way is invisible to those seven rules.

The result is that the guards protecting the sibling-runtime, transport, and
credential boundaries — the rules that exist for the express purpose of
keeping an adapter from acquiring authority it must not have — report success
against a manifest that declares exactly what they forbid.

This pass changes only the guard scripts and the documentation that describes
them. No production Rust source is edited, no dependency set changes, and no
crate's public API, schema version, digest, or golden byte changes.

## 2. Findings

### E-FC-C001-01 — the workspace-inheritance spelling bypasses every anchored manifest rule

The seven anchored rules and the declaration form they miss:

| Guard | Rule | Location |
|---|---|---|
| core | prohibited dependency families | `scripts/check-core-boundary.sh:62` |
| codegg-compat | production `eggplan-repo` dependency (awk, section-scoped) | `scripts/check-codegg-compat-boundary.sh:49` |
| codegg-compat | `codegg` / `codegg-core` dependency | `scripts/check-codegg-compat-boundary.sh:57` |
| codegg-compat | async, database, network, and filesystem client crates | `scripts/check-codegg-compat-boundary.sh:63` |
| integrations | sibling runtime dependency | `scripts/check-integrations-boundary.sh:45` |
| integrations | async runtime, HTTP client, or Git transport | `scripts/check-integrations-boundary.sh:50` |
| integrations | credential or secret-store helper | `scripts/check-integrations-boundary.sh:56` |

Every dependency this repository shares is declared in the inheritance form.
`crates/eggplan-integrations/Cargo.toml:10-12` reads `serde.workspace = true`,
`serde_json.workspace = true`, `sha2.workspace = true`; the root
`Cargo.toml:18-27` declares the shared versions.

Reproduced against the current tree, in a scratch copy:

    $ printf '[dependencies]\ntokio.workspace = true\n' > m.toml
    $ rg -n '^[[:space:]]*(tokio|reqwest|hyper|ureq|surf|isahc|async-std|axum|octocrab|git2|gix)([[:space:]=]|$)' m.toml
    $ echo $?
    1

The rule that forbids an async runtime, HTTP client, or Git transport in the
integrations crate does not match `tokio.workspace = true`.

The same rule matched the plain spelling, so the gap is the spelling and not
the intent:

    $ printf '[dependencies]\ntokio = "1"\n' > m.toml
    $ rg -n '^[[:space:]]*(tokio|reqwest|hyper|ureq|surf|isahc|async-std|axum|octocrab|git2|gix)([[:space:]=]|$)' m.toml
    2:tokio = "1"

Two boundaries are bypassed by one declaration. `eggbench-core` is what the
integrations guard's `no_sibling_runtime` rule exists to forbid, and `tokio`
is what its `no_transport_dep` rule exists to forbid. Declaring both in the
inheritance form passes both.

`scripts/check-projection-cli-boundary.sh` is not affected: it uses an
unanchored `grep -RInE` over the whole manifest, so it matches `tokio` in
`tokio.workspace = true` as a substring. That guard is also weaker in other
ways, but it does not have this defect.

### E-FC-C001-02 — a renamed dependency bypasses the same rules

A dependency may also be declared under a different key with the real package
named in a `package` field:

    [dependencies]
    my-http = { package = "reqwest", version = "0.12" }

No anchored rule matches this: the key is `my-http`, and the forbidden name
appears after `package = "`. This is a second spelling of the same defect
class — the rule models a dependency *key*, not a dependency.

### E-FC-C001-03 — a hyphenated sub-crate bypasses the same rules

`tokio-util = "1"` pulls in `tokio`. The core rule's trailing alternation is
`([[:space:]]|=|$)`, so `-` matches none of it and the rule does not fire. The
integrations sibling-runtime rule already handles this form — its pattern
carries an optional `(-[A-Za-z0-9_-]+)?` suffix — but the core, codegg-compat
transport, and integrations transport and credential rules do not.

A guard fix that leaves an equally trivial bypass in place is not a fix, so
this instance of the same defect class is in scope.

### E-FC-C001-04 — the core guard's source rule does not match `tempfile::`

`scripts/check-core-boundary.sh:69-85` defines `IMPURE_SOURCE_PATTERN` as one
alternative per impure capability: `std::fs`, `fs::(read|...)`, `File::`,
`std::process`, `process::Command`, `Command::new`, and the rest. It has **no
`tempfile::` alternative**, and `grep -n tempfile scripts/check-core-boundary.sh`
returns nothing.

The two sibling guards do match it:
`scripts/check-codegg-compat-boundary.sh:78` and
`scripts/check-integrations-boundary.sh:68` both include `tempfile::`.

So a production dependency on `tempfile` in `eggplan-core`, used as
`tempfile::NamedTempFile::new()`, passes the core guard — the filesystem
acquisition the core guard exists to forbid.

`eggplan-core` declares no `tempfile` dependency at all today
(`crates/eggplan-core/Cargo.toml`), and its source contains no `tempfile::`
token, so the tree is clean. The defect is that nothing in CI would notice if
that stopped being true.

### E-FC-C001-05 — `tempfile` as a production dependency is not rejected by any guard

`plans/closure/codegg-integration/003-c002-closed.md:272` records this as open
and states that the source-side exposure is closed because `tempfile::` is
matched by `no_impure_source`. That statement is true for the codegg-compat
and integrations guards and false for the core guard (E-FC-C001-04).

The recorded finding also says a production-only `tempfile` rule "cannot be
added to the unscoped guard 3 without failing the tree". That premise is also
wrong: the rule as written does not match this repository's own spelling of
the dependency, so adding it would not have failed the tree — it would have
been inert.

`tempfile` is a legitimate dev-dependency of `eggplan-cli`
(`crates/eggplan-cli/Cargo.toml:22`) and `eggplan-codegg-compat`
(`crates/eggplan-codegg-compat/Cargo.toml:15`), and a legitimate *production*
dependency of `eggplan-repo`
(`crates/eggplan-repo/Cargo.toml:16`), which uses it in `atomic_write`
(`crates/eggplan-repo/src/store.rs:1496-1518`). Any production-only `tempfile`
rule must therefore exempt `eggplan-repo` and apply to the pure crates only.

## 3. Controlling semantics

- A boundary guard must fail on every spelling of the dependency it forbids.
  A rule that matches one spelling is not a boundary.
- The manifest is a table of *dependency keys*, and a key may be followed by
  `= value`, `.workspace = true`, or a renamed `= { package = "..." }` form.
  A rule that models only the first is incomplete.
- A sub-crate of a forbidden family is the forbidden family. `tokio-util` is
  `tokio`.
- Section-scoped rules stay section-scoped: a dev-dependency is not a
  production dependency, and the repository's legal dev-dependencies must
  keep passing.
- A guard's self-proofs must cover the new spelling. The existing self-proof
  fixtures already write their clean manifests in the inheritance form
  (`scripts/check-core-boundary.sh:207-211`) while every violation fixture
  uses the plain form — which is why the gap survived a hardening pass that
  added self-proofs to every guard.
- The root `Cargo.toml` `[workspace.dependencies]` table is deliberately not
  scanned: it legitimately declares `git2` and `tempfile` for `eggplan-repo`.
  The dependency *edge* that matters is the one in the crate's own manifest,
  so scanning the crate manifests with a complete key matcher is sufficient
  and avoids false positives from the shared table.
- The scan-scope string in each guard's failure message must continue to state
  what was actually scanned.

## 4. Scope

In scope:

- `scripts/check-core-boundary.sh`
- `scripts/check-codegg-compat-boundary.sh`
- `scripts/check-integrations-boundary.sh`
- `architecture/deep-dive-tooling-governance.md`
- the `plans/closure/codegg-integration/003-c002-closed.md` record, by dated
  erratum only

Out of scope:

- any production Rust source;
- any crate manifest, dependency set, or lock file;
- `scripts/check-projection-cli-boundary.sh`, which does not have this defect
  and whose own weaknesses are a separate line;
- `scripts/check-closure-authority-boundary.sh`, which scans source files
  rather than manifests;
- the root `Cargo.toml` `[workspace.dependencies]` table as a scan target;
- rewriting `eggplan-repo`'s production use of `tempfile`, which is
  legitimate and load-bearing for `atomic_write`.

## 5. Required production changes

1. Every anchored manifest rule must accept, as the token following the
   dependency name: `=`, end of line, or `.workspace` followed by optional
   whitespace and `=`.
2. Every anchored manifest rule must additionally match the forbidden name in
   a renamed-dependency position, `package = "forbidden-name"`.
3. The core, codegg-compat transport, and integrations transport and credential
   rules must additionally match a hyphenated sub-crate of a forbidden family.
4. The codegg-compat production-`eggplan-repo` rule must accept the
   inheritance spelling while remaining section-scoped.
5. The core guard's `IMPURE_SOURCE_PATTERN` must gain a `tempfile::`
   alternative, matching the two sibling guards.
6. A production-only `tempfile` dependency rule must be added for the pure
   crates, exempting `eggplan-repo`, using the section-scoping mechanism
   already present in `scripts/check-codegg-compat-boundary.sh:45-52`.
7. Each guard must gain self-proofs for every new spelling it now matches, so
   the rule cannot silently narrow again.

The rules must stay anchored to a dependency-key position. A rule that
matched `tokio` anywhere in the file would also match a comment or a package
name, and the codegg-compat guard's sibling rule is explicitly written to
avoid incidental matches (`scripts/check-codegg-compat-boundary.sh:70-72`
documents that deliberate choice).

## 6. Failure, cancellation, restart, and contention semantics

A guard failure must report the offending file and line and the scope it
scanned, as the existing guards do. A guard that cannot scan — because its
tool is missing — must refuse rather than pass, which the hardened core guard
already does (`scripts/check-core-boundary.sh:17-28`) and which the other two
guards inherit from the same pattern.

The guards are read-only and idempotent. They take no lock and hold no
state.

## 7. Required tests

The guards' self-proofs are the tests. Each new spelling requires:

- a self-proof that the rule **rejects** the new spelling;
- a self-proof that the rule still **allows** the legal dev-dependency
  spelling where the rule is section-scoped;
- a self-proof that the rule still allows a clean manifest written in the
  inheritance form.

The three guards already provide a `prove <label> <expect-fail> <fn>
<fixture>` harness with a `manifest_fixture` writer
(`scripts/check-core-boundary.sh:186-208`,
`scripts/check-codegg-compat-boundary.sh:133-146`,
`scripts/check-integrations-boundary.sh:143-156`). The new cases use it.

## 8. Verification

    cargo fmt --all -- --check
    cargo check --workspace --all-targets --locked
    cargo clippy --workspace --all-targets --locked -- -D warnings
    cargo test --workspace --locked
    bash scripts/check-core-boundary.sh
    bash scripts/check-codegg-compat-boundary.sh
    bash scripts/check-integrations-boundary.sh
    bash scripts/check-projection-cli-boundary.sh
    bash scripts/check-closure-authority-boundary.sh
    cargo +1.89.0 check --workspace --all-targets --locked
    cargo +1.89.0 test --workspace --locked
    git diff --check

Each guard must additionally be run against a scratch manifest carrying each
newly-forbidden spelling, and must fail. That negative proof is the closure
evidence; a passing guard against the real tree is not.

## 9. Acceptance criteria

1. `tokio.workspace = true` in a guarded crate's manifest fails the
   integrations transport rule and the core prohibited-dependency rule.
2. `eggbench-core.workspace = true` fails the integrations sibling-runtime
   rule.
3. `codegg.workspace = true` and `codegg-core.workspace = true` fail the
   codegg-compat rule.
4. `eggplan-repo.workspace = true` in `[dependencies]` fails the codegg-compat
   production rule, and the same declaration in `[dev-dependencies]` is still
   allowed.
5. `secrecy.workspace = true` fails the integrations credential rule.
6. `my-http = { package = "reqwest", version = "0.12" }` fails the
   integrations transport rule.
7. `tokio-util = "1"` fails the core prohibited-dependency rule.
8. `tempfile::NamedTempFile::new()` in a core source file fails the core
   source rule.
9. A production `tempfile` dependency fails in `eggplan-core`,
   `eggplan-projection`, `eggplan-markdown`, `eggplan-integrations`,
   `eggplan-codegg-compat`, and `eggplan-cli`, and is still allowed in
   `eggplan-repo`.
10. Every legal dependency declaration in the real tree still passes every
    guard.
11. Every guard's self-proof suite still passes, and the new self-proofs are
    present for each newly-matched spelling.
12. `architecture/deep-dive-tooling-governance.md` states the guards' actual
    behavior, and every `path:line` citation in it is verified against the
    file it names.

## 10. Stop conditions

- A fix would require changing a crate manifest, the dependency set, or the
  lock file.
- A fix would require rewriting `eggplan-repo`'s production use of
  `tempfile`.
- A fix would require changing `scripts/check-projection-cli-boundary.sh`'s
  own weaknesses, which are a separate line.
- A rule cannot be made to match the new spelling without also matching an
  incidental occurrence, such as a comment or an unrelated crate whose name
  merely contains a forbidden name.

## 11. Closure evidence required

- The negative proof for each newly-matched spelling: the command, the
  scratch manifest, and the guard's actual output.
- The positive proof that the real tree still passes every guard.
- The full self-proof output of each guard.
- The coordinator's verification table, with "not run" rows recorded honestly.
- A statement of whether the root `[workspace.dependencies]` table remains
  unscanned, and why that is correct.
- An explicit statement that no crate manifest, dependency set, lock file,
  schema version, digest, or golden byte changed.

## 12. Handoff notes

- This plan is **ready**. The defect is reproduced, the root cause is
  identified in all seven rules, and the required changes are specified
  concretely.
- The defect is recorded as open in
  `plans/closure/codegg-integration/003-c002-closed.md:269-277` in two forms
  that this line corrects: the `tempfile` production-dependency finding, whose
  stated premise is wrong, and the integrations `lib.rs` scan finding, which
  was already fixed by commit `674264d` and needs a dated erratum rather
  than a code change.
- The guard scripts are CI-gated and skipped on Windows runners. A change to
  a guard must keep `.github/workflows/ci.yml` consistent with it.
