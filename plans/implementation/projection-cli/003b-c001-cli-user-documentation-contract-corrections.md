# Projection/CLI M003b C001 — CLI User-Documentation Contract Corrections

Status: closed (see plans/closure/projection-cli/003b-c001-closed.md)

Repository baseline: 37ad290ca6b971a68d2871aedb39f4882f4eb2a8

Source roadmap:

- plans/subsystems/projection-cli-roadmap.md

Long-term requirements:

- plans/000-long-term-specification.md sections 14-15 and 19-20
- plans/001-terminology-and-domain-model.md

Related ADRs: ADR-0002.

Predecessor plan/closure:

- plans/implementation/projection-cli/001-cli-control-surface-and-derived-registry.md
- plans/closure/projection-cli/001-closed.md
- plans/implementation/projection-cli/003b-batch-compact-projections-and-shell-completions.md
- plans/closure/projection-cli/003b-closed.md

Primary class: non-blocking documentation correction

## 1. Objective

Correct four statements in `docs/cli-reference.md` that the implementation
contradicts, and close the documentation gaps that the M003b command surface
introduced.

The user-facing guides are written for adopters. Where a guide states a
guarantee the CLI does not provide — in particular that `--state-root` is
"Never implicit", which is a safety-relevant claim about which state root a
command operates on — the guide is the thing that is wrong, not the code.
This pass corrects the guides and records the one place where the code's own
help text and the guide disagree.

This pass is non-blocking documentation maintenance. It changes no production
behavior, no schema, and no CLI contract.

## 2. Findings

### E-PC-C001-01 — `--state-root` is documented as never implicit, but has a default

`docs/cli-reference.md:12`:

    | `--state-root PATH` | Location of canonical state (conventionally `.eggplan`). Never implicit. |

`docs/getting-started.md:35-36`:

    The path is never implicit — pass `--state-root` explicitly on every invocation.

The implementation defaults it. `crates/eggplan-cli/src/lib.rs:320-324`:

    let state_root = parsed
        .options
        .get("--state-root")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".eggplan"));

The CLI's own usage text agrees with the implementation and disagrees with the
guide: `crates/eggplan-cli/src/commands.rs:131` describes the flag as
"repository state root (default .eggplan)".

Only `markdown import` treats the root as mandatory
(`crates/eggplan-cli/src/lib.rs:1156-1163`).

**Why this is the documentation's error and not the code's.** The default
matches the convention the guide itself states — "conventionally `.eggplan`" —
and matches the CLI's own help. Making the flag mandatory across every
command would be a breaking CLI change to an already-closed milestone, which
is out of scope here. The word "Never" is the defect.

**Why it is safety-relevant.** An operator who omits the flag in a working
directory other than the one they intend operates on a different state root
than they believe. The guide currently tells them this cannot happen.

### E-PC-C001-02 — "All mutating commands require `--expected-revision N`" is false, and the guide says both things

`docs/cli-reference.md:126`:

    All require `--expected-revision N`.

The table immediately below contradicts it: `new --input PLAN.json`
(`:130`) takes no revision, and `markdown import FILE --state-root PATH`
(`:133`) takes neither a revision nor an input plan.

The reason is structural: `new` creates a plan at revision 0, so there is no
prior revision to compare against, and `markdown import` creates one Draft
plan from prose rather than mutating an existing one. Both are creations, not
mutations of an existing revision.

### E-PC-C001-03 — the `unavailable` check state is documented over-broadly

`docs/cli-reference.md:209`:

    | `unavailable` | Evidence is unavailable/not-run/skipped, or the subject is unavailable |

The implementation distinguishes the three evidence statuses the guide lumps
together. `crates/eggplan-core/src/assessment.rs:694-706` maps:

- `EvidenceStatus::NotRun` → `AssessmentStatus::EvidenceMissingOrUnavailable`
- `EvidenceStatus::Skipped` → `AssessmentStatus::EvidenceMissingOrUnavailable`
- `EvidenceStatus::Unavailable` → `AssessmentStatus::EvidenceMissingOrUnavailable`

All three produce the *same* assessment status, so the assessment layer does
not distinguish them. The CLI does. `crates/eggplan-cli/src/lib.rs:1743-1765`:

    AssessmentStatus::EvidenceMissingOrUnavailable
        if observations
            .iter()
            .any(|observation| observation.status() == EvidenceStatus::Unavailable) =>
    {
        "unavailable"
    }
    ...
    AssessmentStatus::EvidenceMissingOrUnavailable
    | AssessmentStatus::ActionableWorkRemaining => "incomplete",

Only an observation whose status is literally `Unavailable` produces the
`unavailable` check state. A `not_run` or `skipped` observation produces
`incomplete`.

The guide's wording therefore promises a distinction the check state does
not make.

### E-PC-C001-04 — `item update` takes at least one field, not exactly one

`docs/cli-reference.md:135`:

    `item update` takes exactly one of `--status`, `--blocker`, or `--next-action`.

The implementation requires *at least one* and applies every field that is
supplied. `crates/eggplan-cli/src/lib.rs:1305-1315` requires one of the three
to be present, and `:1341-1356` applies `--status`, then `--blocker`, then
`--next-action` independently.

### E-PC-C001-05 — `eggplan init` and `eggplan help` are undocumented

`docs/cli-reference.md:19-30` lists the read commands, `:124-133` the mutating
commands, and `:161-194` assessment, closure, projection, and interchange.
Neither `init` nor `help` appears in any table.

Both are implemented:

- `init` — `crates/eggplan-cli/src/commands.rs:188-196`
- `help` — `crates/eggplan-cli/src/commands.rs:351-357`

`init` is mentioned once in `docs/getting-started.md:39` as an invocation, not
as a documented command. `help` is not mentioned in any user-facing guide.

The reverse direction was checked and is clean: every flag named in
`docs/cli-reference.md` exists in the code, and there are no phantom flags
documented.

### E-PC-C001-06 — the value-taking flag list is incomplete

`docs/cli-reference.md:16-17`:

    Value-taking flags: `--state-root`, `--input`, `--expected-revision`,
    `--provider-policy`, `--status`, `--blocker`, `--next-action`, `--output`, `--format`.

`--limit` and `--after` are absent from the list, although both are
documented later in the same file: `--limit` at `:43` and `--after` at `:71`.

## 3. Controlling semantics

From `plans/README.md` and `AGENTS.md`: `docs/` is user-facing documentation
written for adopters, and when a behavior changes the documentation must be
updated to match. From `architecture/cli-control-surface.md`, the CLI is a
thin adapter over libraries whose JSON envelope is the machine contract.

Required behavior after this pass:

- no statement in `docs/` asserts a guarantee the CLI does not provide;
- every command the CLI implements is documented, or explicitly documented as
  intentionally out of the reference's scope;
- the documented check-state vocabulary matches what `check` can emit;
- the documentation and the CLI's own usage text agree;
- no production behavior changes.

## 4. Scope

### In scope

- `docs/cli-reference.md` — the six findings above;
- `docs/getting-started.md` — the `--state-root` statement at `:35-36`;
- `docs/README.md` and `docs/evidence-and-closure.md` — checked for the same
  claims, corrected if they carry them;
- the `architecture/deep-dive-projection-cli.md` statements that describe the
  same behaviors, if they repeat the incorrect claims;
- registry and roadmap lineage updates.

### Out of scope

- any production Rust change, including making `--state-root` mandatory;
- any change to the CLI's command inventory, flag set, or JSON envelope;
- the check-state vocabulary itself, which is owned by
  plans/implementation/foundation-core/003-c002-subject-capture-error-fidelity.md
  where it touches subject diagnostics, and is otherwise a core contract;
- the `show` read path's subject requirement, which M003 C002 changed and which
  is already reflected in `docs/cli-reference.md` and
  `docs/getting-started.md`. **Note added after M003 C002 closed:** `show` now
  exits nonzero with `subject_unavailable` when the Git subject cannot be
  captured, where it previously returned a subjectless summary. Do not
  reintroduce the earlier claim that `show` works without a resolvable
  subject, and do not duplicate the correction this plan already carries for
  the `unavailable` check state;
- the shell-completion depth parity work, owned by
  plans/implementation/projection-cli/003b-c002-shell-completion-depth-parity.md;
- the plan lifecycle and structure-editing capability gap, which is tracked
  as a proposed milestone rather than a corrective;
- the deferred Interop/distribution subsystem;
- `plans/000-long-term-specification.md` and the `architecture/` normative
  documents' product contracts. Where a normative document states a behavior
  the CLI does not implement, that is a capability gap, not a documentation
  error, and is handled by the proposed milestone.

## 5. Required production changes

Preferred shape — correct the guide to describe what the CLI does, and where
the guide's guarantee is genuinely desirable, state the gap rather than
pretending it is met:

- `--state-root`: replace "Never implicit" with an accurate statement of the
  default and its consequence. The recommended wording must state that the
  default is `.eggplan` **relative to the current working directory**, and
  that operators should pass the flag explicitly. Keep the same requirement
  and the same emphasis as today, minus the word "Never".
- `--expected-revision`: distinguish creation commands from revision-checked
  mutations. State which commands require the flag and why a creation does
  not.
- `unavailable`: describe the state as it is emitted — an observation whose
  status is `unavailable`, or a subject that cannot be captured — and state
  that `not_run` and `skipped` observations surface as `incomplete`. Do not
  weaken the check-state vocabulary; correct the description of it.
- `item update`: state that at least one of the three fields is required and
  that supplied fields are applied together.
- `init` and `help`: add both to the reference, in the appropriate table, with
  their real behavior.
- Value-taking flags: add `--limit` and `--after`.

For every correction, the implementer must re-read the cited code before
editing the guide, so that the guide is corrected against the current
implementation rather than against this plan's description of it.

Explicitly rejected alternatives:

- changing the CLI to make `--state-root` mandatory in order to make the guide
  true. That is a breaking change to a closed milestone and a separate
  decision;
- deleting the `unavailable` check state or merging it into `incomplete`. The
  ten-state vocabulary is a contract, and the subject-unavailable case is
  deliberately distinct;
- adding a documentation note that says "the CLI does not really do this"
  without fixing the surrounding text. The guide must describe the actual
  behavior.

## 6. Failure, cancellation, restart, and contention semantics

- No production behavior changes. The same command invocations produce the
  same outputs and the same exit codes before and after this pass.
- No schema, digest, envelope, or diagnostic-code changes.
- The documentation must not claim a guarantee the implementation does not
  provide; a weaker but true statement is required, not a stronger one.
- The CLI's own usage text is the tie-breaker for any disagreement between
  the guide and the code, except where the code's text is itself wrong, in
  which case both are corrected together and the change is recorded.

## 7. Required tests

Documentation-only changes require no new Rust test. The required evidence is
instead:

- a citation-by-citation re-check of every statement this pass changes, with
  the code location that supports the corrected wording recorded;
- a forward check that every command in `crates/eggplan-cli/src/commands.rs`
  now appears in `docs/cli-reference.md`, and that every command documented
  there exists in the command table;
- a reverse check that every flag named in `docs/cli-reference.md` exists in
  the CLI's option tables;
- `git diff --check` clean.

If the implementer changes the CLI's usage text to match a corrected guide,
the CLI's own snapshot and completion tests must remain green.

## 8. Verification

Record actual results for:

    cargo fmt --all -- --check
    cargo check --workspace --all-targets --locked
    cargo clippy --workspace --all-targets --locked -- -D warnings
    cargo test --workspace --locked
    cargo test -p eggplan-cli --locked
    cargo +1.89.0 check --workspace --all-targets --locked
    cargo +1.89.0 test --workspace --locked
    bash scripts/check-projection-cli-boundary.sh
    git diff --check
    git status --short

The `cargo +1.89.0` rows are the MSRV gate and must be recorded truthfully,
including if the toolchain is unavailable in the implementing environment.
Hosted Linux/macOS/Windows and Rust 1.89 qualification must remain green.

## 9. Acceptance criteria

C001 closes when:

1. `docs/cli-reference.md` and `docs/getting-started.md` no longer state that
   `--state-root` is never implicit, and instead describe the `.eggplan`
   default accurately;
2. the revision-flag requirement is stated per command, and the guide no
   longer contradicts itself;
3. the `unavailable` check state is described as the CLI emits it, including
   the `not_run`/`skipped` → `incomplete` behavior;
4. the `item update` requirement is stated as at least one field, with
   multiple fields applied together;
5. `init` and `help` are documented, and the value-taking flag list is
   complete;
6. every CLI command appears in the reference, and every documented flag
   exists in the code — verified in both directions;
7. the guide and the CLI's own usage text agree on every point this pass
   touches;
8. no production behavior, schema, envelope, or diagnostic code changed;
9. native and MSRV qualification pass, with the projection/CLI boundary guard
   green.

## 10. Stop conditions

Stop and report rather than improvise if:

- a correction would require changing the CLI's behavior rather than the
  guide's wording, which is a separate decision;
- the implementation is found to differ from this plan's description of it in
  a way that changes the correction, in which case the plan is amended and
  the deviation recorded;
- a normative document in `architecture/` or `plans/000-long-term-specification.md`
  turns out to state the same incorrect behavior, in which case that is a
  capability question and is escalated rather than rewritten;
- required verification cannot be obtained honestly.

## 11. Closure evidence required

- implementation commit(s) and PR reference;
- the requirement-to-evidence matrix for every acceptance criterion in §9;
- the citation-by-citation re-check table, with the supporting code location
  for each corrected statement;
- the forward and reverse command/flag coverage check results;
- exact verification output, distinguishing planned from actually run
  commands;
- hosted native/MSRV run and job identifiers where available;
- unresolved findings with severity, and roadmap disposition;
- registry updates.

Do not convert a planned command into a passing result because it appears in
§8. If a row was not run, record it as not run or unavailable.

## 12. Handoff notes

- This is a documentation-truthfulness pass. Do not describe it as a CLI
  behavior change.
- Record truthfully that the CLI's behavior was already correct and
  self-consistent; the guide was the side that was wrong. The `--state-root`
  item is the one with a safety dimension, because an operator following the
  guide today would believe the flag cannot be omitted.
- The plan-lifecycle and structure-editing gap is deliberately not folded into
  this pass. It is unbuilt capability against the normative specification, not
  a documentation error, and it is tracked as a proposed milestone.
