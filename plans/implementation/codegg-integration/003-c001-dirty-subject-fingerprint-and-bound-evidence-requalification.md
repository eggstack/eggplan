# CodeGG Integration M003 C001 — Dirty-Subject Fingerprint Contract and Bound-Evidence Requalification

Status: ready for coordinated handoff

Repository baseline:

- Eggplan current head: `6644725ae540b19873d7c769e3afeab7bce53d09`
- Eggplan M003 implementation: `3f7c603315131bb169bfdd2bb575531d228532b1`
- Eggplan M003 closure reconciliation: `6644725ae540b19873d7c769e3afeab7bce53d09`
- CodeGG M003 implementation: `53dea47f414641c3f9756c3f8f181f6208be8115`
- CodeGG M003 closure: `aa21cfe1763d7ea00d11e582ed4108233e4088a9`
- CodeGG C001 corrective registration: `dc9ae6ccf1cfa8dea51522ddc6bf33976562edb9`
- CodeGG C001 plan:
  `plans/implementation/eggplan-assessment-integration/005-m003-c001-dirty-subject-provenance-and-bound-evidence.md`

Source roadmap:

- `plans/subsystems/codegg-integration-roadmap.md`

Historical M003 plan:

- `plans/implementation/codegg-integration/003-repository-plan-binding-contract.md`

Primary class: post-closure correctness corrective / source-identity contract

## 1. Finding

M003 correctly records that CodeGG and Eggplan currently compute dirty Git
subjects with deliberately different manifest encodings.

The CodeGG binding consumer therefore proves only:

- equal HEAD revision;
- equal clean/dirty state;
- matching durable repository association;
- `.eggplan` excluded on both sides.

That is enough to reject obvious repository/revision mismatch, but it is not
enough for dirty exact-subject evidence.

CodeGG's historical execution provenance persists its own native dirty digest.
The M003 translator currently places that native CodeGG digest into an Eggplan
`SubjectRevision`. Eggplan repository assessment compares the complete
`SubjectRevision`, including `dirty_digest`, against the repository's own
capture. Because the digest algorithms are intentionally non-comparable, a
stable dirty execution cannot reliably satisfy Eggplan exact-subject matching.

The existing dirty test proves only that a dirty repository can bind. It does
not prove:

    dirty execution
      -> durable historical provenance
      -> repository observation writeback
      -> exact Eggplan subject match
      -> item completion
      -> guarded closure

There is also a bounded capture-stability gap at initial binding: if dirty
contents change between the two independent captures while HEAD and
clean/dirty classification stay unchanged, the current proof cannot detect the
change.

Historical M003 closure remains valid for the qualified clean path and recorded
limitations. This corrective does not rewrite that closure.

## 2. Corrective objective

Expose the existing Eggplan Git-subject digest as a small, versioned,
repository-ID-free fingerprint contract so an external host can capture and
persist the exact digest Eggplan would later require for dirty exact-subject
assessment.

Do NOT:

- change the existing Eggplan `SubjectRevision` schema;
- change the current `dirty_digest` algorithm;
- rewrite historical observations/closures;
- expose dirty paths or file contents;
- make CodeGG's native provenance digest equal to Eggplan's;
- move execution capture or scheduling into Eggplan.

The corrective adds a compatibility fingerprint, not a new authority model.

## 3. Public fingerprint API

Add a bounded public type in `eggplan-repo`, recommended:

    pub struct GitSubjectFingerprintV1 {
        pub schema_version: u16,
        pub revision: String,
        pub state: SubjectState,
        pub dirty_digest: Option<String>,
    }

with:

    pub const SCHEMA_VERSION: u16 = 1;

and one public capture operation, recommended:

    pub fn capture_git_subject_fingerprint(
        root: impl AsRef<Path>,
        options: GitSubjectOptions,
        excluded_path: Option<impl AsRef<Path>>,
    ) -> Result<GitSubjectFingerprintV1, GitSubjectError>

Exact naming may vary.

The result MUST contain only:

- schema version;
- HEAD revision;
- clean/dirty state;
- Eggplan-native dirty digest.

It MUST NOT expose:

- path lists;
- file contents;
- index entries;
- symlink targets;
- raw manifest bytes;
- repository ID;
- provider/evidence/closure state.

## 4. Single implementation source

Refactor `GitSubjectSource::capture()` so both APIs use the same internal
capture result.

Required invariant:

    let fingerprint = capture_git_subject_fingerprint(...);
    let subject = GitSubjectSource(...).capture();

    fingerprint.revision == subject.revision
    fingerprint.state == subject.state
    fingerprint.dirty_digest == subject.dirty_digest

for the same root/options/exclusion and stable filesystem state.

Do not duplicate the dirty-manifest algorithm.

The existing private `dirty_manifest` implementation remains the one
canonical algorithm unless implementation needs a small internal extraction.

## 5. Historical compatibility

The existing Eggplan Git subject algorithm and digest bytes are frozen.

C001 MUST NOT:

- add a domain separator that changes existing digest output;
- change row ordering;
- change status-bit encoding;
- change index-entry treatment;
- change regular-file/symlink/deleted/submodule encoding;
- alter current exclusion semantics;
- alter existing default bounds.

Add golden regression fixtures/digests before refactoring and prove byte
identity after the refactor.

Existing repository Plans, observations, closure records, and stale/current
assessment behavior must remain unchanged.

## 6. Exclusion semantics

The fingerprint API must support the same single administrative-root exclusion
as `GitSubjectSource::excluding_path`.

Required behavior:

- exclusion must resolve inside the discovered worktree;
- symlink/canonical-path handling stays equivalent to existing behavior;
- the excluded root and descendants contribute neither dirtiness nor digest;
- an invalid/outside exclusion fails closed;
- no generic glob/multiple-exclusion policy is introduced.

CodeGG will use this specifically for `.eggplan`.

## 7. Bounds and failure behavior

Preserve current `GitSubjectOptions` limits:

- max paths;
- max content bytes;
- max submodule depth.

Fingerprint capture must return the same typed `GitSubjectError` classes as
normal subject capture.

No partial digest may be returned after:

- bound overflow;
- unsafe/symlink escape;
- non-Unicode path;
- Git error;
- missing/unborn HEAD;
- filesystem read failure.

## 8. Cross-implementation role

This API intentionally returns Eggplan's digest; it is not a new universal Git
digest standard.

CodeGG may continue persisting its native `egggit` digest for CodeGG's own
execution-provenance semantics.

The coordinated CodeGG corrective will persist both:

- CodeGG-native dirty digest;
- Eggplan-compatible dirty digest captured through this API.

Only the Eggplan-compatible digest may be placed into a bound repository
`SubjectRevision`.

## 9. Binding-time stability contract

The CodeGG consumer must use the new API in a sandwich capture:

    E1 = Eggplan fingerprint
    C  = governed CodeGG native capture
    E2 = Eggplan fingerprint

Binding is valid only when:

- E1 == E2 exactly;
- C.revision == E1.revision;
- CodeGG clean/dirty state == E1.state.

This closes the identified "same HEAD, still dirty, contents changed between
captures" gap without requiring native digest equality.

Eggplan itself does not implement CodeGG's sandwich; it only supplies the exact
fingerprint primitive.

## 10. Execution-time persistence contract

The CodeGG consumer must capture the Eggplan-compatible fingerprint at the same
attempt start/seal authority boundary where native execution provenance is
captured.

For a stable dirty attempt, CodeGG must durably retain the exact
`GitSubjectFingerprintV1.dirty_digest`.

A historical CodeGG dirty attempt that predates this field remains historical
but MUST NOT be promoted to bound exact-subject Eggplan evidence by guessing,
re-capturing the current worktree, or converting the CodeGG-native digest.

Clean historical subjects remain translatable because both representations
have no dirty digest.

## 11. Eggplan tests

Add focused tests for fingerprint/capture equality across at least:

- clean;
- unstaged tracked edit;
- staged tracked edit;
- staged + unstaged same path;
- untracked file;
- deleted tracked file;
- symlink;
- rename/copy status where libgit2 reports it;
- dirty nested submodule;
- excluded `.eggplan` subtree;
- path/content/depth bound failures;
- invalid/outside exclusion.

Add a golden digest fixture set covering representative dirty states before
the refactor.

## 12. CodeGG consuming qualification

Eggplan C001 does not close merely because the fingerprint API compiles.

Cross-repository closure requires CodeGG to pin the exact qualified Eggplan
revision and prove:

1. new execution provenance retains the Eggplan-compatible dirty digest;
2. legacy dirty provenance fails closed for bound exact-subject evidence;
3. dirty stable execution writes a repository observation whose
   `SubjectRevision.dirty_digest` equals Eggplan's repository capture;
4. dirty bound item completion succeeds only with matching evidence;
5. dirty guarded Plan closure succeeds under a stable dirty worktree;
6. dirty content drift causes attempt/binding/closure failure as appropriate;
7. clean M003 behavior remains unchanged.

## 13. Documentation reconciliation

As part of C001 registration/closure:

- correct the historical M003 implementation-plan status header that still
  says the CodeGG consumer is outstanding;
- update the registry's external CodeGG baseline from the old planning commit
  to the landed M003 implementation/closure;
- preserve the original conditional-closure narrative inside the closure
  record as historical evidence;
- record this C001 as a post-closure corrective, not as a rewrite of M003.

## 14. Verification

At minimum:

    cargo fmt --all -- --check
    cargo check --workspace --all-targets --locked
    cargo clippy --workspace --all-targets --locked -- -D warnings
    cargo test --workspace --locked
    cargo +1.89.0 check --workspace --all-targets --locked
    cargo +1.89.0 test --workspace --locked
    bash scripts/check-core-boundary.sh
    bash scripts/check-codegg-compat-boundary.sh
    bash scripts/check-closure-authority-boundary.sh
    git diff --check

Hosted Ubuntu/macOS/Windows and Rust 1.89 qualification is required on the
exact Eggplan implementation revision consumed by CodeGG.

## 15. Acceptance criteria

C001 closes when:

1. Eggplan exposes a bounded repository-ID-free fingerprint API;
2. fingerprint output is exactly the existing Eggplan subject revision/state/
   dirty-digest result for the same stable capture;
3. no existing subject digest changes;
4. no persisted Eggplan schema changes;
5. no path/content manifest leaks through the public API;
6. existing bounds/exclusion/failure behavior remains fail-closed;
7. CodeGG pins the exact C001 Eggplan revision;
8. CodeGG persists the fingerprint for new dirty execution provenance;
9. CodeGG dirty bound evidence/closure E2E passes;
10. legacy dirty provenance fails closed rather than being reinterpreted;
11. both repositories reconcile planning/status docs and hosted evidence.

## 16. Stop conditions

Stop and report if:

- implementing the fingerprint requires changing historical Eggplan digest
  bytes;
- the API must expose raw path/content manifests;
- CodeGG would need current-worktree backfill for historical attempts;
- dirty bound evidence can only work by accepting the CodeGG-native digest as
  an Eggplan digest;
- clean or historical closure semantics regress;
- the corrective would require weakening `SubjectRevision` exact equality.

## 17. Closure evidence

Create:

    plans/closure/codegg-integration/003-c001-closed.md

Record:

- Eggplan implementation SHA;
- before/after digest golden matrix;
- public API shape;
- exclusion/bounds matrix;
- native/MSRV hosted runs;
- exact CodeGG consuming pin/implementation;
- CodeGG dirty evidence + guarded closure test matrix;
- remaining limitations.
