#!/usr/bin/env bash
set -euo pipefail

manifest="crates/eggplan-core/Cargo.toml"
source_dir="crates/eggplan-core/src"

# Precondition: the guards below run `rg`, and every one of them treats "no
# match" as success. If `rg` is missing, each invocation exits 127 inside a
# conditional, the conditional takes the else branch, and the guard reports
# success without scanning anything. That was not hypothetical: ripgrep is not
# in the stock GitHub runner images, so this script silently no-opped in CI
# until the workflow began installing it. Fail loudly instead of reporting an
# unscanned pass.
#
# The precondition is a function so the self-proofs can exercise the refusal
# path against a deliberately absent tool name.
require_tools() {
    local tool
    for tool in "$@"; do
        if ! command -v "$tool" >/dev/null 2>&1; then
            echo "required boundary-guard tool '$tool' is not installed; refusing to report a pass" >&2
            return 1
        fi
    done
    return 0
}

require_tools rg mktemp || exit 1

# ---------------------------------------------------------------------------
# Pure-core boundary. Three guards, each stated with the exact scope it scans,
# so a failure message never claims more than was checked.
#
#   1. no_prohibited_dependency  manifest, every section
#   2. no_impure_source          src tree, all files
#   3. no_reasoning_field        src tree, all files
#
# Guard 1 scans the whole manifest, including [dev-dependencies] and
# [build-dependencies]: a pure domain crate must not pull in an async runtime,
# transport, or database crate even for its own tests.
#
# Guard 2 scans the whole src tree, including inline `#[cfg(test)]` modules.
# The manifest guard cannot express this boundary: std::fs, std::process,
# std::net, and std::path live in the standard library and need no dependency,
# so only a source scan can detect them.
#
# Guard 3 scans the whole src tree for public reasoning/transcript fields.
#
# Every guard maps ripgrep's exit status explicitly: 0 is a violation, 1 is
# clean, and anything else means the scan could not be performed. A scan that
# could not be performed fails closed; it is never reported as a pass.
#
# The tracked tree is checked before the proofs run, so a violation in tracked
# source is reported with the guard's own message, which states the exact
# scope that was scanned.
#
# Self-proofs at the bottom run the same functions against synthetic fixtures
# in a temporary directory. They never write tracked source.
# ---------------------------------------------------------------------------

# Guard 1 pattern, unchanged from the pre-hardening script.
PROHIBITED_DEPENDENCY_PATTERN='(^|[[:space:]])(tokio|async-std|reqwest|hyper|sqlx|rusqlite|diesel|subprocess|bollard|rmcp|openai|ollama)([[:space:]]|=|$)'

# Guard 2 pattern: filesystem, process, network, path, environment, database,
# and Git access. One alternative per capability, anchored so that a fully
# qualified call and a `use`-imported call form are both caught without
# matching unrelated core vocabulary (`EvidenceKind::Command`, a `path`
# variable, a "git" subject kind, `SourcePath::`, `InputFile::`).
IMPURE_SOURCE_PATTERN="std::fs|\
\bfs::(read|read_to_string|read_dir|write|create_dir|create_dir_all|remove_file|remove_dir|rename|copy|metadata|symlink_metadata|canonicalize|read_link|File|OpenOptions)|\
\bFile::|\
std::process|\
\bprocess::Command|\
\bCommand::new|\
std::env|\
\benv::(var|vars|args|args_os|current_dir|home_dir|temp_dir)|\
std::net|\
\bnet::(TcpStream|TcpListener|UdpSocket|SocketAddr|IpAddr|Ipv4Addr|Ipv6Addr)|\
\b(TcpStream|TcpListener|UdpSocket)\b|\
std::path|\
\bPathBuf\b|\
\bPath::|\
\b(sqlx|rusqlite|diesel|sqlite|postgres|sea_orm|tokio_postgres)\b|\
\b(git2|gix)\b|\
\bgit::"

# Guard 3 pattern, unchanged from the pre-hardening script.
REASONING_FIELD_PATTERN='pub[[:space:]]+[^;]*(thinking|scratchpad|chain_of_thought|hidden_reasoning)'

# Runs one `rg` scan and maps ripgrep's exit status to a guard result:
#   0 -> a prohibited match was found (violation)
#   1 -> clean (no match)
#   2 -> the scan could not be performed (fail closed; never a pass)
scan() {
    local pattern="$1" target="$2"
    shift 2
    local status=0
    rg "$@" -- "$pattern" "$target" || status=$?
    if [ "$status" -gt 1 ]; then
        echo "core boundary guard could not scan $target (rg exited $status); refusing to report a pass" >&2
        return 2
    fi
    return "$status"
}

# Guard 1: no async runtime, HTTP client, database, or model-runtime
# dependency may enter the core crate's dependency graph.
no_prohibited_dependency() {
    scan "$PROHIBITED_DEPENDENCY_PATTERN" "$1" -ni
}

# Guard 2: the source performs no filesystem, process, network, path,
# environment, database, or Git access. Both the fully qualified and the
# `use`-imported call forms are matched, so `std::fs::read_to_string` and a
# bare `fs::read_to_string` behind `use std::fs` are both caught.
no_impure_source() {
    scan "$IMPURE_SOURCE_PATTERN" "$1" -n
}

# Guard 3: no persisted model-reasoning field in the public surface.
no_reasoning_field() {
    scan "$REASONING_FIELD_PATTERN" "$1" -n
}

# require_clean <guard-fn> <target> <failure-message>
#   guard exit 0 -> violation: print the message and fail
#   guard exit 1 -> clean
#   guard exit 2 -> the scan could not be performed: fail closed
require_clean() {
    local fn="$1" target="$2" message="$3" status=0
    "$fn" "$target" || status=$?
    case "$status" in
        0)
            echo "$message" >&2
            return 1
            ;;
        1)
            return 0
            ;;
        *)
            return 2
            ;;
    esac
}

run_guards() {
    local m="$1" s="$2"

    require_clean no_prohibited_dependency "$m" \
        "prohibited dependency family found in eggplan-core (scanned: $m, all sections)" || return $?

    require_clean no_impure_source "$s" \
        "eggplan-core source must stay pure: no filesystem, process, network, path, environment, database, or Git access (scanned: $s, all files)" || return $?

    require_clean no_reasoning_field "$s" \
        "reasoning/transcript field found in eggplan-core public source (scanned: $s, all files)" || return $?
}

# The tracked tree is checked before the proofs run, so a violation in tracked
# source is reported with the guard's own message, which states the exact
# scope that was scanned.
run_guards "$manifest" "$source_dir"

# --------------------------------------------------------------------------
# Self-proofs. `prove <label> <expect-fail:0|1> -- <guard-fn> <fixture>`
# --------------------------------------------------------------------------
fixtures="$(mktemp -d)"

# Targeted, non-recursive cleanup: this script only ever creates Cargo.toml and
# src/lib.rs below the temporary root.
cleanup() {
    rm -f "$fixtures/Cargo.toml" "$fixtures/src/lib.rs"
    rmdir "$fixtures/src" "$fixtures" 2>/dev/null || true
}
trap cleanup EXIT

manifest_fixture() {
    printf '%s' "$1" >"$fixtures/Cargo.toml"
}

source_fixture() {
    mkdir -p "$fixtures/src"
    printf '%s' "$1" >"$fixtures/src/lib.rs"
}

prove() {
    local label="$1" expect="$2" fn="$3" target="$4" status=0
    "$fn" "$target" >/dev/null 2>&1 || status=$?
    if [ "$status" -gt 1 ]; then
        echo "core boundary guard self-proof could not scan: $label (guard exit $status)" >&2
        exit 1
    fi
    local got=0
    if [ "$status" -eq 0 ]; then
        got=1
    fi
    if [ "$got" != "$expect" ]; then
        echo "core boundary guard self-proof failed: $label (expected fail=$expect, got fail=$got)" >&2
        exit 1
    fi
}

# --- guard 1: prohibited dependency families --------------------------------
manifest_fixture '[package]
name = "eggplan-core"
[dependencies]
serde.workspace = true
serde_json.workspace = true
'
prove "guard1 clean manifest" 0 no_prohibited_dependency "$fixtures/Cargo.toml"

for case in \
    'tokio = { version = "1" }' \
    'async-std = "0.5"' \
    'reqwest = "0.12"' \
    'hyper = "1"' \
    'sqlx = "0.8"' \
    'rusqlite = "0.32"' \
    'diesel = "2"' \
    'subprocess = "0.2"' \
    'ollama = "0.4"'
do
    manifest_fixture "[dependencies]
$case
"
    prove "guard1 rejects: $case" 1 no_prohibited_dependency "$fixtures/Cargo.toml"
done

# The manifest guard is not section-scoped: a dev-dependency fails too.
manifest_fixture '[dev-dependencies]
tokio = "1"
'
prove "guard1 rejects a dev-dependency" 1 no_prohibited_dependency "$fixtures/Cargo.toml"

# --- guard 2: impure access in the source tree ------------------------------
source_fixture 'use std::collections::BTreeMap;
pub fn f() -> BTreeMap<String, String> { BTreeMap::new() }
'
prove "guard2 allows pure std use" 0 no_impure_source "$fixtures/src"

# The guard must not fire on core's own vocabulary: an evidence kind named
# Command, a "git" subject kind, a `path` variable, and identifiers that end
# in Path or File but are not the standard-library types.
source_fixture 'pub enum EvidenceKind { Command, Test }
pub fn kind() -> EvidenceKind { EvidenceKind::Command }
pub fn subject_kind() -> &'"'"'static str { "git" }
pub fn segments(path: &[&str]) -> usize { path.len() }
pub struct SourcePath;
impl SourcePath {
    pub fn new() -> Self { SourcePath }
}
pub struct InputFile;
impl InputFile {
    pub fn new() -> Self { InputFile }
}
'
prove "guard2 allows core vocabulary" 0 no_impure_source "$fixtures/src"

for case in \
    'std::fs::read_to_string("/etc/passwd").unwrap();' \
    'use std::fs; let _ = fs::read_to_string("x");' \
    'use std::fs; let _ = fs::read_dir("x");' \
    'use std::fs::File; let _ = File::open("x");' \
    'use std::path::PathBuf; let _ = PathBuf::from("x");' \
    'std::process::Command::new("x");' \
    'use std::process::Command; Command::new("x");' \
    'std::net::TcpStream::connect("x");' \
    'let _ = UdpSocket::bind("x");' \
    'std::env::var("HOME").ok();' \
    'sqlx::query("x");' \
    'let _ = git2::Repository::open("x");'
do
    source_fixture "$case
"
    prove "guard2 rejects: $case" 1 no_impure_source "$fixtures/src"
done

# --- guard 3: reasoning/transcript fields -----------------------------------
source_fixture 'pub struct Plan {
    pub id: String,
}
'
prove "guard3 allows a plain public struct" 0 no_reasoning_field "$fixtures/src"

for case in \
    'pub struct Plan { pub thinking: String }' \
    'pub struct Plan { pub scratchpad: String }' \
    'pub struct Plan { pub chain_of_thought: String }' \
    'pub struct Plan { pub hidden_reasoning: String }'
do
    source_fixture "$case
"
    prove "guard3 rejects: $case" 1 no_reasoning_field "$fixtures/src"
done

# --- the real tree ----------------------------------------------------------
# These three proofs read the tracked manifest and source exactly as the guard
# itself does. They never write them.
prove "the real eggplan-core manifest must be free of prohibited dependency families" 0 no_prohibited_dependency "$manifest"
prove "the real eggplan-core source must be free of impure access" 0 no_impure_source "$source_dir"
prove "the real eggplan-core source must be free of reasoning/transcript fields" 0 no_reasoning_field "$source_dir"

# --- the precondition -------------------------------------------------------
# The precondition must refuse a missing tool rather than let the guards run
# unscanned. Proven against a tool name that cannot exist, so the proof does
# not depend on which tools the host happens to have.
if require_tools rg eggplan-boundary-guard-absent-tool >/dev/null 2>&1; then
    echo "core boundary guard self-proof failed: the precondition accepted an absent tool" >&2
    exit 1
fi

if ! require_tools rg mktemp >/dev/null 2>&1; then
    echo "core boundary guard self-proof failed: the precondition rejected the declared tools" >&2
    exit 1
fi

echo "eggplan-core boundary self-proofs passed"

echo "eggplan-core static boundary check passed"
