#!/usr/bin/env bash
set -euo pipefail

# Closure subject authority boundary guard.
#
# Static defense-in-depth companion to the compile-fail doctests in
# crates/eggplan-repo/src/lib.rs. The compile boundary is the stronger
# evidence; this script fails if accidental public symbols that would let a
# downstream crate inject closure subject authority reappear.
#
# Forbidden:
#   * public re-exports of SubjectCapture, GitSubjectCapture, or
#     ScriptedSubjectCapture from eggplan-repo;
#   * a public RepositoryStore::finalize_closure_with_capture method;
#   * any other publicly callable alternate finalizer that accepts a caller
#     SubjectCapture implementation.

repo_src="crates/eggplan-repo/src"
repo_lib="$repo_src/lib.rs"
repo_store="$repo_src/store.rs"

if rg -n '^[[:space:]]*pub[[:space:]]+(use|fn[[:space:]]+new)[[:space:]]+(ScriptedSubjectCapture|SubjectCapture|GitSubjectCapture)' "$repo_lib" "$repo_store"; then
  echo "eggplan-repo re-exports a closure subject capture symbol as public" >&2
  exit 1
fi

if rg -n '^[[:space:]]*pub[[:space:]]+(trait|struct)[[:space:]]+(ScriptedSubjectCapture|SubjectCapture|GitSubjectCapture)\b' "$repo_lib" "$repo_store"; then
  echo "eggplan-repo declares a closure subject capture type as public" >&2
  exit 1
fi

if rg -n '^[[:space:]]*pub[[:space:]]+fn[[:space:]]+finalize_closure_with_capture\b' "$repo_store"; then
  echo "RepositoryStore::finalize_closure_with_capture must not be public" >&2
  exit 1
fi

if rg -n '^[[:space:]]*pub[[:space:]]+fn[[:space:]]+finalize_closure_with_[A-Za-z0-9_]+\b' "$repo_store"; then
  echo "an alternate public capture-injected finalizer was reintroduced" >&2
  exit 1
fi

echo "closure-authority boundary check passed"