#!/usr/bin/env bash
set -euo pipefail

# Closure subject authority guard. Compile-fail doctests remain the primary
# public API boundary; this check catches accidental source-level exposures.
python3 - <<'PY'
from pathlib import Path
import re
import sys

FORBIDDEN = {"SubjectCapture", "GitSubjectCapture", "ScriptedSubjectCapture"}


def clean(source):
    # These authority declarations are all single-line Rust declarations or
    # `pub use` statements. Strip comments so docs/examples are not authority.
    source = re.sub(r"/\*.*?\*/", "", source, flags=re.S)
    return re.sub(r"//[^\n]*", "", source)


def forbidden(source):
    source = clean(source)
    # Match ordinary direct and grouped public imports, including multiline
    # grouped imports. `pub(crate)` deliberately does not match this prefix.
    for statement in re.findall(r"(?ms)^\s*pub\s+use\b(.*?);", source):
        if FORBIDDEN.intersection(re.findall(r"\b[A-Za-z_][A-Za-z0-9_]*\b", statement)):
            return True
    for name in FORBIDDEN:
        if re.search(rf"(?m)^\s*pub\s+(?:trait|struct)\s+{name}\b", source):
            return True
    if re.search(r"(?m)^\s*pub\s+fn\s+finalize_closure_with_[A-Za-z0-9_]+\b", source):
        return True
    return False


def prove(label, source, should_fail):
    got = forbidden(source)
    if got != should_fail:
        raise SystemExit(f"closure-authority guard self-test failed: {label}")


# Deterministic synthetic proof exercises public direct/grouped imports for all
# hidden authority types, alternate finalizers, and safe crate-private forms.
for name in sorted(FORBIDDEN):
    prove(f"direct {name} re-export", f"pub use store::{name};", True)
    prove(f"grouped {name} re-export", f"pub use store::{{Allowed, {name}}};", True)
prove("alternate finalizer", "pub fn finalize_closure_with_capture() {}", True)
prove("crate-private seams", "pub(crate) use store::SubjectCapture;\npub(crate) fn finalize_closure_with_capture() {}", False)
prove("comments and supported method", "// pub use store::SubjectCapture;\npub fn finalize_closure() {}", False)

for path in (Path("crates/eggplan-repo/src/lib.rs"), Path("crates/eggplan-repo/src/store.rs")):
    if forbidden(path.read_text(encoding="utf-8")):
        print(f"forbidden public closure authority in {path}", file=sys.stderr)
        raise SystemExit(1)

print("closure-authority boundary check and synthetic proofs passed")
PY
