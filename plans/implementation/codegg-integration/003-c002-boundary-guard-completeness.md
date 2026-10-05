# CodeGG Integration M003 C002 — Compatibility Boundary Guard Completeness

Status: closed (see plans/closure/codegg-integration/003-c002-closed.md)

Repository baseline: 60a5f9212d3e6feab8acb45fb6acbfdbd56264aa

Source roadmap:

- plans/subsystems/codegg-integration-roadmap.md

Predecessor plan/closure:

- plans/implementation/codegg-integration/003-repository-plan-binding-contract.md
- plans/closure/codegg-integration/003-conditionally-closed.md
- plans/closure/codegg-integration/003-c001-closed.md

Reviewed CodeGG baseline (research baseline, not a pin):

- CodeGG `3623f65e` + `b470865a` on `main`, which consumed the Eggplan
  fingerprint contract at commit `0dd33b7`

Primary class: tooling governance / boundary enforcement / defense in depth

## 1. Objective

Close two defects in `scripts/check-codegg-compat-boundary.sh` so the guard
actually enforces the boundary it documents, and so its failure messages stop
overstating what was checked.

The production bridge itself is pure and correct: `crates/eggplan-codegg-compat/src`
contains no `std::fs`, no `File::`, no `read_to_string`, no `std::process`, and
no network or database access, and it performs no repository I/O. This pass
changes only the guard, so that the CI-enforced boundary matches the verified
reality instead of trailing behind it.

This does not reopen M003 or C001 and preserves all historical closure evidence.

## 2. Findings

### C-CODEGG-C002-01 — the source guard omits filesystem access

`scripts/check-codegg-compat-boundary.sh:34` currently fails on:

    std::process|Command::new|tokio::|reqwest::|hyper::|sqlx::|std::net|TcpStream|UdpSocket

Filesystem access is absent from that alternation. A `std::fs`, `File::open`,
`File::create`, or `fs::read_to_string` in `crates/eggplan-codegg-compat/src`
would pass CI today.

This is a direct contradiction of the crate's own stated boundary.
`architecture/codegg-compat.md:11` says the production crate depends on
`eggplan-core`, serde, and serde_json only and that "Repository persistence
remains outside this bridge." The manifest guard at `:24` covers the
*dependency graph* for async/database/network clients, but nothing covers
filesystem access in the source, and the `eggplan-repo` guard at `:9-17` is
`[dependencies]`-scoped so a `std::fs` use needs no new dependency at all.

The sibling guards are inconsistent on this point:
`scripts/check-integrations-boundary.sh` and
`scripts/check-projection-cli-boundary.sh` both police process and transport
access, and `check-closure-authority-boundary.sh` scopes its own scans
explicitly. The compatibility guard is the outlier.

### C-CODEGG-C002-02 — three guards are not section-scoped while their messages claim "production"

Only the `awk` guard at `:9-17` tracks the `[dependencies]` section. The `rg`
guards at `:19`, `:24`, and `:34` scan the whole manifest or source tree, yet
their failure messages say:

- `:20` "compatibility crate must not depend on CodeGG" while matching any
  section, including `[dev-dependencies]`;
- `:25` "compatibility production dependencies must not include async, database,
  or network clients" while matching any section;
- `:35` "compatibility production source must remain free of process, network,
  and database access" while matching the full source tree.

The practical effect is asymmetric and currently load-bearing:
`crates/eggplan-codegg-compat/Cargo.toml` legitimately carries `eggplan-repo`
and `tempfile` as **dev**-dependencies, used by `tests/parity.rs` and
`tests/repository_projection.rs` to exercise the real store. The `[dependencies]`
guard permits that. A dev-dependency on `codegg` or `tokio` would fail even
though a production one is the real concern.

This is a real enforcement/semantics mismatch. It is deliberately **not**
corrected by simply making every guard section-scoped, because that would
loosen enforcement. The correct resolution is chosen in §4.

### C-CODEGG-C002-03 — guard scope is under-documented

`architecture/deep-dive-codegg-compat.md` previously described this script as
covering `:9-27` and omitted two of the five guards. The deep dive has been
corrected as part of the review that produced this plan. `scripts/` itself has
no in-file statement of the five guards it enforces.

## 3. Controlling semantics

From `plans/registry.md` design gates 6 and 17, and ADR-0004: the compatibility
bridge generalizes CodeGG WorkPlan semantics incrementally while
`WorkOrder`, scheduler, runtime, and storage ownership stay CodeGG-owned, and
staged adoption must use a pure Eggplan assessment bridge in production.

The guard's job is to make that purity mechanically true, not to describe it.
Required behavior after this pass:

- the source guard fails on filesystem access, not only process/network/database;
- guard failure messages state exactly what was scanned;
- the `eggplan-repo` dev-dependency allowance stays legal;
- a production dependency on `codegg`, `tokio`, or an equivalent client fails;
- the owned-identity declaration guard keeps matching declarations while not
  false-positives on incidental mentions of the same words.

## 4. Scope

### In scope

- `scripts/check-codegg-compat-boundary.sh` guard coverage and messages;
- deterministic synthetic positive/negative self-proofs for the modified
  guards, following the `prove(...)` pattern already established in
  `scripts/check-closure-authority-boundary.sh:44-49`;
- an in-file header comment listing all five guards and their scope;
- reconciliation of the C-CODEGG-C002-02 asymmetry;
- documentation alignment in `architecture/codegg-compat.md` and
  `architecture/deep-dive-tooling-governance.md`;
- the registry lineage citation clarification in §7;
- normal native/MSRV qualification.

### Out of scope

- any change to `crates/eggplan-codegg-compat` production source (the crate is
  already correct);
- changing the `eggplan-codegg-compat` dependency set, including its
  dev-dependencies;
- changing WorkPlan mapping, the projection contract, digests, or IDs;
- changing M003 or C001 behavior or their closure evidence;
- modifying `scripts/check-closure-authority-boundary.sh` or the other four
  guards, except to reuse their proven self-test pattern;
- the `plans/registry.md:79` closure-filename/status wording, which is tracked
  separately as a traceability nit.

No production Rust source is edited in this pass. If implementing the guard
turns out to require a source change, that is a stop condition, not extra scope.

## 5. Required changes

- extend the source guard's alternation with filesystem access patterns
  (`std::fs`, `File::open`, `File::create`, `fs::read`, `fs::read_to_string`,
  `fs::write`, and equivalents), keeping it readable rather than a sprawling
  regex;
- decide the C-CODEGG-C002-02 asymmetry explicitly and record the decision in
  the closure record. Two defensible resolutions:
  1. keep the `rg` guards unscoped as deliberate strictness, and change the
     messages to say what is actually scanned — for example "any section",
     including dev-dependencies; or
  2. make the manifest guards `[dependencies]`-scoped to match the stated
     intent, and add a separate, explicit guard for dev-dependencies on
     CodeGG specifically, since importing the very crate whose runtime ownership
     is prohibited is a different concern from an async client.
  Option 1 is preferred: it does not weaken any check, and the current
  `eggplan-repo` dev-dependency is already legal under the section-scoped guard,
  so no relaxation is needed to keep the tree green.
- add synthetic self-proofs that demonstrate a `std::fs` use fails, a dev-
  dependency on `codegg` fails, the current manifest and source pass, and the
  legal dev-dependency on `eggplan-repo` passes;
- self-proofs must not modify tracked source.

## 6. Required tests and verification

At minimum, the self-proof must show:

- current `crates/eggplan-codegg-compat` manifest and source pass;
- a synthetic `std::fs::read_to_string` in the bridge source fails;
- a synthetic `[dev-dependencies]` entry on `codegg` fails;
- a synthetic `[dev-dependencies]` entry on `eggplan-repo` passes, matching the
  real current state;
- a synthetic process/network/database use still fails;
- a synthetic `pub struct WorkOrder` still fails;
- a bare mention of `WorkOrder` in a comment or a non-`pub` position does not
  fail.

Record actual results for:

    cargo fmt --all -- --check
    cargo check --workspace --all-targets --locked
    cargo clippy --workspace --all-targets --locked -- -D warnings
    cargo test --workspace --locked
    cargo +1.89.0 check --workspace --all-targets --locked
    cargo +1.89.0 test --workspace --locked
    bash scripts/check-codegg-compat-boundary.sh
    bash scripts/check-core-boundary.sh
    bash scripts/check-integrations-boundary.sh
    bash scripts/check-projection-cli-boundary.sh
    bash scripts/check-closure-authority-boundary.sh
    git diff --check

All five boundary guards must pass. Hosted Linux/macOS/Windows and Rust 1.89
qualification must remain green; the guard runs on Linux and macOS and is
skipped on Windows, which is recorded in
`architecture/deep-dive-tooling-governance.md` finding 1.

## 7. Planning hygiene

`plans/registry.md:120` cites
`plans/implementation/eggplan-assessment-integration/001-durable-execution-subject-provenance.md`,
which does not exist in this repository. The surrounding sentence records that
the plan "closed in dbowm91/codegg", so the reference is a cross-repository
provenance note, not a broken in-repo link.

Required correction: make the path visibly external so no reader or tool
attempts to resolve it locally. Qualify it as a CodeGG repository path and
keep the CodeGG commit `418fdc85…` attribution intact. This is a wording
clarification, not a change to any closure conclusion or to the CodeGG lineage.

The path quoted above is intentionally left non-resolving in this plan, because
it reproduces the defective registry text that this section corrects. It refers
to `dbowm91/codegg`, not to this repository.

Do not modify any other historical closure record.

## 8. Acceptance criteria

C002 closes when:

1. filesystem access in `crates/eggplan-codegg-compat/src` fails the guard;
2. process, network, and database access still fail the guard;
3. the owned-identity declaration guard still fails and does not
   false-positive on incidental mentions;
4. the legal `[dev-dependencies]` entry on `eggplan-repo` continues to pass;
5. the C-CODEGG-C002-02 asymmetry is resolved deliberately and the decision is
   recorded in the closure record;
6. all five guards have deterministic synthetic self-proofs, or the modified
   guards do;
7. the script header states all five guards and their scan scope, and every
   failure message matches what was actually scanned;
8. `architecture/codegg-compat.md` and
   `architecture/deep-dive-tooling-governance.md` describe the guard
   accurately;
9. the `plans/registry.md:120` citation is unambiguously marked as a CodeGG
   repository path with its attribution preserved;
10. no production Rust source, dependency set, schema, or public API changes;
11. native and MSRV qualification pass with all five guards green.

## 9. Stop conditions

Stop and report rather than improvise if:

- making the guard correct would require editing the bridge's production source
  to satisfy it, which would mean the purity claim was wrong and needs its own
  assessment;
- the chosen asymmetry resolution would loosen a currently effective check
  without a recorded compensating guard;
- the self-proof cannot be made deterministic without modifying tracked source;
- the boundary guard is found to be depended on by a sibling repository's CI in
  a way that a behavior change would break;
- required verification cannot be obtained honestly.

## 10. Closure evidence required

- implementation commit(s) and PR reference;
- the requirement-to-evidence matrix for every acceptance criterion in §8;
- before/after output of the guard, including each synthetic case;
- the recorded C-CODEGG-C002-02 decision and its rationale;
- exact verification output, including the MSRV rows, distinguishing planned
  from actually run commands;
- hosted native/MSRV run and job identifiers where available;
- explicit confirmation that no production behavior changed;
- unresolved findings with severity, and roadmap disposition;
- registry updates.

## 11. Handoff notes

- This is tooling hardening. The CodeGG integration roadmap remains terminal for
  M001-M003 and C001; this pass does not reopen capability work and does not
  gate Eggplan-side or CodeGG-side capability plans.
- Record truthfully that the guard was previously incomplete. The production
  bridge was always pure; the defect was in the enforcement, not in the crate.
  Do not describe this as a bridge correctness defect.
