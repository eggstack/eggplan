# Foundation Core M003 C005 — Core Boundary Guard Completeness and Tooling-Governance Reconciliation

Status: closed (see plans/closure/foundation-core/003-c005-closed.md)

Repository baseline: 37ad290ca6b971a68d2871aedb39f4882f4eb2a8

Source roadmap:

- plans/subsystems/foundation-core-roadmap.md

Long-term requirements:

- plans/000-long-term-specification.md (core is pure; no fs, Git, process, or net)

Related ADRs: ADR-0002.

Predecessor plan/closure:

- plans/implementation/foundation-core/003-subject-scope-strict-schema-and-platform-hardening.md
- plans/closure/foundation-core/003-closed.md
- plans/implementation/codegg-integration/003-c002-boundary-guard-completeness.md
- plans/closure/codegg-integration/003-c002-closed.md

Primary class: tooling governance / boundary enforcement

## 1. Objective

Make `scripts/check-core-boundary.sh` enforce the boundary it documents, and
correct the tooling-governance deep dive so that it describes the guards that
actually exist.

The core guard is the last boundary script without a tool precondition, and
the only one that does not scan core's production source for impure access.
The core crate's source is clean today — verified: no `std::fs`,
`std::process`, `std::net`, `std::path`, `File::`, `git2`, or `Command::new`
occurrence anywhere under `crates/eggplan-core/src`. The defect is that
nothing in CI would notice if that stopped being true.

This pass changes only the guard script and the documentation that describes
it. No production Rust source is edited.

## 2. Findings

### E-FC-C005-01 — the core guard reports success when ripgrep is absent

`scripts/check-core-boundary.sh:7` and `:12` invoke `rg` inside `if`
statements. When `rg` is not installed, each invocation exits 127 inside a
conditional, the `if` takes the else branch, and the guard reports a pass
without scanning anything.

Reproduced against the current tree at the recorded baseline:

    $ PATH=<without ripgrep> bash scripts/check-core-boundary.sh
    scripts/check-core-boundary.sh: line 7: rg: command not found
    scripts/check-core-boundary.sh: line 12: rg: command not found
    eggplan-core static boundary check passed
    exit=0

The sibling `rg`-based guards already refuse this condition. Reproduced:

    $ PATH=<without ripgrep> bash scripts/check-codegg-compat-boundary.sh
    required boundary-guard tool 'rg' is not installed; refusing to report a pass
    exit=1

    $ PATH=<without ripgrep> bash scripts/check-integrations-boundary.sh
    required boundary-guard tool 'rg' is not installed; refusing to report a pass
    exit=1

This condition is already recorded in
`architecture/deep-dive-tooling-governance.md:126-135`, which cites the
observed hosted-run output at `scripts/check-core-boundary.sh: line 7: rg:
command not found` and identifies the core guard as "the one residual gap;
see finding 6". This finding independently reproduces the recorded
observation at the current baseline.

### E-FC-C005-02 — the core guard does not scan core source for impure access

`scripts/check-core-boundary.sh` scans two things:

- the **manifest** (`:7-10`) for a list of prohibited dependency families;
- the **source** (`:12-15`) only for reasoning/transcript field names.

It never scans `crates/eggplan-core/src` for filesystem, process, network,
path, database, or Git access. The `[dependencies]`-scoped manifest check
cannot catch this, because `std::fs`, `std::process`, `std::net`, and
`std::path` are in the standard library and need no dependency at all.

Reproduced against a synthetic tree at the recorded baseline: a core source
file whose body is

    pub fn steal() -> String {
        std::fs::read_to_string("/etc/passwd").unwrap()
    }

together with a manifest containing only `serde`, passes the guard:

    $ bash scripts/check-core-boundary.sh
    eggplan-core static boundary check passed
    exit=0

This is a direct contradiction of the crate's stated boundary. `AGENTS.md`
states that `eggplan-core` is a "pure deterministic domain, no fs/Git/process/net",
and `architecture/overview.md` states the same. The sibling guards already
police this class of access:

- `scripts/check-codegg-compat-boundary.sh` fails on filesystem access in the
  bridge source, a hardening added by CodeGG M003 C002;
- `scripts/check-integrations-boundary.sh:65-69` fails on
  `std::fs|std::path|tempfile::|…|std::net|TcpStream|UdpSocket` across the
  whole adapter source tree.

The core guard is the outlier among the five.

### E-FC-C005-03 — the tooling-governance deep dive misdescribes the integrations guard

`architecture/deep-dive-tooling-governance.md:60-71` describes
`check-integrations-boundary.sh` as:

    ### `check-integrations-boundary.sh` (mechanism: `grep -nE` / `grep -RInE` / `grep -q`)

    - Manifest: fails on `eggwork | eggsearch | eggbench | eggsact | tokio |
      reqwest | hyper | mcp | async.runtime` …
    - Source: fails on `^use (eggwork | eggsearch | eggbench | eggsact)::`,
      `async fn`, `tokio::`, `reqwest::`, `std::process`, `Command::new` under
      `src/` …
    - Trust: fails on `register_trusted | ProviderRegistry` in
      `src/eggwork.rs` and `src/eggsearch.rs` …

The actual script (`scripts/check-integrations-boundary.sh:1-80`) is
different in every respect that matters:

| Claim in the deep dive | Actual script |
|---|---|
| mechanism is `grep -nE` / `grep -RInE` / `grep -q` | the script uses `rg` exclusively (`:45`, `:50`, `:56`, `:62`, `:68`, `:74`, `:79`) |
| three guard concerns | six named guards: `no_sibling_runtime` (`:44-46`), `no_transport_dep` (`:49-51`), `no_credential_dep` (`:55-57`), `no_process_source` (`:61-63`), `no_acquisition_source` (`:67-69`), `no_trust_enrollment` (`:73-75`) |
| source scan matches `^use (… )::` and process/transport only | guard 5 (`:67-69`) also matches `std::fs`, `std::path`, `tempfile::`, `read_dir`, `std::net`, `TcpStream`, `UdpSocket`, `which::`, and executable discovery |
| trust guard applies to `src/eggwork.rs` and `src/eggsearch.rs` only | guard 6 scans the **whole source tree** (`:34-35` and `:73-75`), so a newly added adapter cannot escape enforcement by not being named |
| no tool precondition mentioned | `:11-16` requires both `rg` and `awk` and refuses to report an unscanned pass |
| no self-proofs mentioned | the script carries deterministic synthetic self-proofs |

The deep dive understates the guard's coverage and misattributes its
mechanism. A reader of the deep dive would conclude that the Eggbench and
GitHub adapters are not covered by the trust guard, and that the guard cannot
detect filesystem access in an adapter — both conclusions are false.

### E-FC-C005-04 — the deep dive's ripgrep claim is stale

`architecture/deep-dive-tooling-governance.md:133-135`:

    `check-core-boundary.sh` still lacks the precondition and is the one residual
    gap; see finding 6. The three `grep`/`python3` guards
    (`integrations`, `projection-cli`, `closure-authority`) were never affected.

`integrations` is no longer a `grep`/`python3` guard. It is an `rg` guard
with its own `command -v rg` precondition
(`scripts/check-integrations-boundary.sh:11-16`), so it was affected by the
same no-op trap and was hardened the same way. The sentence is correct about
`projection-cli` and `closure-authority` and incorrect about `integrations`.

### E-FC-C005-05 — load-bearing `file:line` citations in the deep dives have drifted

The deep dives carry `file:line` claims that are checked against the code by
readers and by tools. Several no longer resolve. Confirmed examples:

- `architecture/deep-dive-projection-cli.md:54-57` cites `run` and `execute`
  at `crates/eggplan-cli/src/lib.rs:225-241` and `:258-266`; the current
  definitions are at `lib.rs:263` and `:318-326`.
- `architecture/deep-dive-projection-cli.md:162-167` cites `assessment_state`
  at `crates/eggplan-cli/src/lib.rs:1323-1345`; the current definition is at
  `lib.rs:1743-1765`.
- `architecture/deep-dive-repository.md:122` cites "25 variants" at
  `crates/eggplan-repo/src/store.rs:54-110`; the current `RepoError` enum
  spans `store.rs:55-114`.
- `architecture/deep-dive-integrations.md:20` attributes `ProviderRegistry` to
  `crates/eggplan-integrations/src/lib.rs:102-113`; the type is defined in
  `crates/eggplan-core/src/evidence.rs`.

These are documentation-truthfulness defects in the same file family as
E-FC-C005-03 and E-FC-C005-04, and they are corrected in the same pass
because the deep dives are the design record the guards are described
against.

## 3. Controlling semantics

From `architecture/deep-dive-tooling-governance.md` and `.skills/eggplan-verify/SKILL.md`:
a boundary guard must make its boundary mechanically true, and a guard that
reports success without scanning anything is worse than no guard because it
consumes the reader's trust.

Required behavior after this pass:

- the core guard refuses to report a pass when its required tools are
  unavailable, exactly as the codegg and integrations guards do;
- the core guard fails when core's production source uses filesystem,
  process, network, path, database, or Git access;
- the core guard carries deterministic synthetic self-proofs for the guards it
  enforces, following the pattern established in
  `scripts/check-closure-authority-boundary.sh` and
  `scripts/check-integrations-boundary.sh`;
- the tooling-governance deep dive describes each guard's mechanism, scope,
  guard count, and precondition accurately;
- the deep dive's `file:line` citations resolve against the current code.

## 4. Scope

### In scope

- `scripts/check-core-boundary.sh` — tool precondition, source-purity guard,
  guard count and messages, self-proofs;
- `architecture/deep-dive-tooling-governance.md` — the integrations guard
  description, the ripgrep-history sentence, and the drifted citations;
- the drifted `file:line` citations in
  `architecture/deep-dive-projection-cli.md`,
  `architecture/deep-dive-repository.md`, and
  `architecture/deep-dive-integrations.md` that are confirmed stale;
- registry and roadmap lineage updates.

### Out of scope

- any change to `crates/eggplan-core` production source. The core crate is
  already pure; only the enforcement is incomplete. If implementing the guard
  turns out to require a source change, that is a stop condition, not extra
  scope;
- changing the other four guard scripts' behavior, except to reuse their
  proven self-test pattern;
- `scripts/check-integrations-boundary.sh` itself — the script is correct;
  only the description of it is wrong;
- the CI workflow's ripgrep installation steps, which are already correct;
- the `docs/` user-facing guides, which are owned by the Projection/CLI
  documentation corrective;
- any production Rust, dependency, schema, digest, or public API change;
- the deferred Interop/distribution subsystem.

## 5. Required production changes

Preferred shape:

- add a tool precondition to `scripts/check-core-boundary.sh` that requires
  `rg` — and any other tool the script uses — and refuses to report a pass
  when a required tool is unavailable, using the same message shape as
  `scripts/check-integrations-boundary.sh:11-16`;
- add a source-purity guard over `crates/eggplan-core/src` that fails on
  filesystem, process, network, path, database, and Git access, using an
  alternation in the style of
  `scripts/check-integrations-boundary.sh:67-69` rather than a sprawling
  regular expression;
- keep the existing manifest guard and the existing reasoning-field guard, and
  state the guard count, mechanism, and scan scope in an in-file header
  comment, following `scripts/check-integrations-boundary.sh:18-39`;
- add deterministic synthetic self-proofs that demonstrate: the current core
  manifest and source pass; a synthetic `std::fs` use in core source fails; a
  synthetic `std::process` or `Command::new` use fails; a synthetic
  `std::net` use fails; a synthetic prohibited manifest dependency still
  fails; a synthetic reasoning/transcript field still fails; and the guard
  refuses to report a pass when its tool is unavailable. Self-proofs must not
  modify tracked source;
- correct `architecture/deep-dive-tooling-governance.md:60-71` to describe the
  integrations guard's real mechanism (`rg`), real guard count (six), real
  scan scopes, real precondition, and real self-proofs;
- correct `architecture/deep-dive-tooling-governance.md:133-135` so that the
  ripgrep-history sentence names `integrations` as an `rg` guard that was
  hardened, or restates the sentence so it is true of all three guards;
- correct the confirmed stale `file:line` citations in
  `architecture/deep-dive-projection-cli.md`,
  `architecture/deep-dive-repository.md`, and
  `architecture/deep-dive-integrations.md`.

Explicitly rejected alternatives:

- adding a shell dependency on `awk` to the core guard without need. The
  precondition must require only the tools the script actually uses;
- enforcing core purity by adding a dependency to the manifest instead of
  scanning the source. Standard-library access needs no dependency, so a
  manifest check cannot express this boundary;
- deleting the deep dive's guard descriptions instead of correcting them. The
  deep dive is the design record the guards are described against, and the
  guard descriptions are its purpose.

## 6. Failure, cancellation, restart, and contention semantics

- The guard must fail closed. A missing tool, an unreadable source tree, or
  an unscannable path must produce a non-zero exit and a message that says
  what went wrong, never a pass.
- The guard must not modify tracked source, and its self-proofs must clean up
  their temporary fixtures.
- The self-proofs must be deterministic and must not depend on network,
  process execution, or a Git worktree.
- The guard's runtime must remain negligible relative to the Rust build it
  accompanies.
- The guard's pass condition must not change for the current tree: the
  existing core manifest and source must continue to pass unchanged, so the
  hardening does not introduce a false positive.

## 7. Required tests

The guard's self-proofs are the tests. At minimum they must show:

- the current `crates/eggplan-core` manifest and source pass;
- a synthetic `std::fs::read_to_string` in core source fails;
- a synthetic `std::process` or `Command::new` use in core source fails;
- a synthetic `std::net` use in core source fails;
- a synthetic prohibited dependency in the core manifest still fails;
- a synthetic reasoning/transcript field name in core source still fails;
- the guard refuses to report a pass when `rg` is unavailable;
- the self-proofs leave no tracked file modified, verified with
  `git diff --check` and a clean `git status`.

No Rust test is added by this pass, because no Rust source changes.

## 8. Verification

Record actual results for:

    cargo fmt --all -- --check
    cargo check --workspace --all-targets --locked
    cargo clippy --workspace --all-targets --locked -- -D warnings
    cargo test --workspace --locked
    cargo +1.89.0 check --workspace --all-targets --locked
    cargo +1.89.0 test --workspace --locked
    bash scripts/check-core-boundary.sh
    bash scripts/check-codegg-compat-boundary.sh
    bash scripts/check-integrations-boundary.sh
    bash scripts/check-projection-cli-boundary.sh
    bash scripts/check-closure-authority-boundary.sh
    git diff --check
    git status --short

The `cargo +1.89.0` rows are the MSRV gate and must be recorded truthfully,
including if the toolchain is unavailable in the implementing environment.
Hosted Linux/macOS/Windows and Rust 1.89 qualification must remain green; the
guards run on Linux and macOS and are skipped on Windows, which is recorded
in `architecture/deep-dive-tooling-governance.md`.

## 9. Acceptance criteria

C005 closes when:

1. `scripts/check-core-boundary.sh` refuses to report a pass when its required
   tools are unavailable, with a non-zero exit;
2. filesystem, process, network, path, database, and Git access in core's
   production source fails the guard;
3. the guard carries deterministic synthetic self-proofs for every guard it
   enforces, and they leave no tracked file modified;
4. the guard's in-file header states its guard count, mechanism, and scan
   scope, and every failure message matches what was actually scanned;
5. `architecture/deep-dive-tooling-governance.md` describes the integrations
   guard's mechanism, guard count, scan scopes, precondition, and self-proofs
   accurately, and no longer claims the integrations guard was unaffected by
   the ripgrep trap;
6. the confirmed stale `file:line` citations in the affected deep dives
   resolve against the current code;
7. the current core manifest and source continue to pass, so the hardening
   introduces no false positive;
8. no production Rust source, dependency set, schema, digest, or public API
   changes;
9. native and MSRV qualification pass, with all five boundary guards green.

## 10. Stop conditions

Stop and report rather than improvise if:

- making the core guard correct would require editing core's production source
  to satisfy it, which would mean the purity claim was wrong and needs its
  own assessment;
- the source-purity guard would produce a false positive against the current
  core source, which would mean the scan scope is too broad and must be
  narrowed before it can be adopted;
- the self-proofs cannot be made deterministic without modifying tracked
  source;
- the guard is found to be depended on by a sibling repository's CI in a way
  that a behavior change would break;
- correcting the deep dive's guard descriptions would require changing any
  guard's actual behavior, which is out of scope here;
- required verification cannot be obtained honestly.

## 11. Closure evidence required

- implementation commit(s) and PR reference;
- the requirement-to-evidence matrix for every acceptance criterion in §9;
- before/after output of the core guard, including each synthetic case and the
  tool-unavailable case;
- the confirmed-stale citation list, with each old and new citation recorded;
- exact verification output, including the MSRV rows, distinguishing planned
  from actually run commands;
- hosted native/MSRV run and job identifiers where available;
- explicit confirmation that no production Rust source changed;
- unresolved findings with severity, and roadmap disposition;
- registry updates.

Do not convert a planned command into a passing result because it appears in
§8. If a row was not run, record it as not run or unavailable.

## 12. Handoff notes

- Record truthfully that the core crate was always pure. The defect was in
  the enforcement, not in the crate.
- This pass is tooling hardening plus documentation truthfulness. It does not
  reopen CodeGG M003 C002, and it does not alter the other four guards.
- The drifted `file:line` citations are corrected in the same pass because
  they live in the same design record that this pass already rewrites. Do not
  silently rewrite any other deep dive's conclusions.
