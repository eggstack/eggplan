#!/usr/bin/env bash
set -euo pipefail

manifest="crates/eggplan-codegg-compat/Cargo.toml"
source_dir="crates/eggplan-codegg-compat/src"

# ---------------------------------------------------------------------------
# Compatibility ownership boundary. Five guards, each stated with the exact
# scope it scans, so a failure message never claims more than was checked.
#
#   1. no_production_repo_dep   manifest, [dependencies] section only
#   2. no_codegg_dependency     manifest, every section
#   3. no_client_dependency     manifest, every section
#   4. no_owned_identity        production source tree
#   5. no_impure_source         production source tree
#
# Only guard 1 is section-scoped. Guards 2 and 3 are deliberately stricter and
# match any section, including [dev-dependencies] and [build-dependencies]:
# the bridge must not pull in CodeGG or an impure capability crate at all, even
# for tests. That strictness is intentional and is not relaxed to accommodate
# the legal dev-dependencies the test suite needs.
#
# Guard 1 is section-scoped because the test suite must be able to exercise the
# real store: crates/eggplan-codegg-compat/Cargo.toml legitimately declares
# eggplan-repo and tempfile as [dev-dependencies].
#
# Self-proofs at the bottom run the same functions against synthetic fixtures in
# a temporary directory. They never read or write tracked source.
# ---------------------------------------------------------------------------

# Guard 1: the production dependency graph must not reach the persistence layer.
no_production_repo_dep() {
    awk '
        /^\[dependencies\]/ { active = 1; next }
        /^\[/ { active = 0 }
        active && /eggplan-repo[[:space:]]*=/ { found = 1 }
        END { exit !found }
    ' "$1"
}

# Guard 2: importing the very crate whose runtime ownership is prohibited fails
# in any section, not only in production dependencies.
no_codegg_dependency() {
    rg -n '^[[:space:]]*(codegg|codegg-core)[[:space:]]*=' "$1"
}

# Guard 3: async, database, network, and filesystem client crates fail in any
# section.
no_client_dependency() {
    rg -n '^[[:space:]]*(tokio|sqlx|reqwest|hyper|ureq|surf|isahc|async-std)[[:space:]]*=' "$1"
}

# Guard 4: a public declaration of a CodeGG-owned identity is an ownership
# claim. Incidental mentions -- comments, doc text, non-pub positions -- are
# deliberately not matched.
no_owned_identity() {
    rg -n '^[[:space:]]*pub[[:space:]]+(struct|enum|trait|type)[[:space:]]+(WorkOrder|Goal|GoalVerification|TodoState|WorkPlanCheckpoint|ContextEpoch|AgentRunExecutor|JobExecutor|WorktreePolicy|SandboxPolicy)\b' "$1"
}

# Guard 5: the production source performs no process, filesystem, network, or
# database access. Both the fully-qualified and the `use`-imported call forms
# are matched, so `std::fs::read_to_string` and a bare `fs::read_to_string`
# behind `use std::fs` are both caught.
no_impure_source() {
    rg -n 'std::process|Command::new|tokio::|reqwest::|hyper::|sqlx::|async_std::|std::net|TcpStream|UdpSocket|std::fs|std::path|tempfile::|fs::(read|read_to_string|write|create_dir|remove_file|rename|copy|File|metadata|canonicalize)|File::(open|create|create_new|open_options)' "$1"
}

run_guards() {
    local m="$1"
    local s="$2"

    if no_production_repo_dep "$m"; then
        echo "compatibility [dependencies] must not include eggplan-repo (scanned: $m, [dependencies] section only)" >&2
        return 1
    fi

    if no_codegg_dependency "$m"; then
        echo "compatibility manifest must not depend on CodeGG in any section, including [dev-dependencies] (scanned: $m, all sections)" >&2
        return 1
    fi

    if no_client_dependency "$m"; then
        echo "compatibility manifest must not include async, database, or network clients in any section, including [dev-dependencies] (scanned: $m, all sections)" >&2
        return 1
    fi

    if no_owned_identity "$s"; then
        echo "compatibility source declares ownership outside the WorkPlan seam (scanned: $s, all files)" >&2
        return 1
    fi

    if no_impure_source "$s"; then
        echo "compatibility production source must remain free of process, filesystem, network, and database access (scanned: $s, all files)" >&2
        return 1
    fi
}

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
    local label="$1" expect="$2" fn="$3" target="$4"
    if "$fn" "$target" >/dev/null 2>&1; then
        got=1
    else
        got=0
    fi
    if [ "$got" != "$expect" ]; then
        echo "codegg-compat guard self-proof failed: $label (expected fail=$expect, got fail=$got)" >&2
        exit 1
    fi
}

# --- guard 1: production vs dev dependency on eggplan-repo -----------------
manifest_fixture '[package]
name = "x"
[dependencies]
eggplan-core = { path = "../eggplan-core" }
'
prove "guard1 clean production deps" 0 no_production_repo_dep "$fixtures/Cargo.toml"

manifest_fixture '[dependencies]
eggplan-repo = { path = "../eggplan-repo" }
'
prove "guard1 rejects production eggplan-repo" 1 no_production_repo_dep "$fixtures/Cargo.toml"

manifest_fixture '[dependencies]
eggplan-core = { path = "../eggplan-core" }
[dev-dependencies]
eggplan-repo = { path = "../eggplan-repo" }
tempfile.workspace = true
'
prove "guard1 allows dev eggplan-repo" 0 no_production_repo_dep "$fixtures/Cargo.toml"

# --- guard 2: CodeGG dependency in any section ------------------------------
manifest_fixture '[dependencies]
eggplan-core = { path = "../eggplan-core" }
[dev-dependencies]
eggplan-repo = { path = "../eggplan-repo" }
'
prove "guard2 clean manifest" 0 no_codegg_dependency "$fixtures/Cargo.toml"

manifest_fixture '[dev-dependencies]
codegg = "0.1"
'
prove "guard2 rejects dev-dependency codegg" 1 no_codegg_dependency "$fixtures/Cargo.toml"

manifest_fixture '[dependencies]
codegg-core = "0.1"
'
prove "guard2 rejects production codegg-core" 1 no_codegg_dependency "$fixtures/Cargo.toml"

# --- guard 3: impure client crates in any section --------------------------
manifest_fixture '[dev-dependencies]
eggplan-repo = { path = "../eggplan-repo" }
tempfile.workspace = true
'
prove "guard3 allows legal dev-dependencies" 0 no_client_dependency "$fixtures/Cargo.toml"

for client in tokio sqlx reqwest hyper ureq surf isahc async-std; do
    manifest_fixture "[dependencies]
$client = \"1\"
"
    prove "guard3 rejects $client" 1 no_client_dependency "$fixtures/Cargo.toml"
done

manifest_fixture '[dev-dependencies]
tokio = "1"
'
prove "guard3 rejects dev-dependency tokio" 1 no_client_dependency "$fixtures/Cargo.toml"

# --- guard 4: owned-identity declarations ----------------------------------
source_fixture 'pub struct Plain;
'
prove "guard4 clean source" 0 no_owned_identity "$fixtures/src"

for ident in WorkOrder Goal GoalVerification TodoState WorkPlanCheckpoint ContextEpoch AgentRunExecutor JobExecutor WorktreePolicy SandboxPolicy; do
    source_fixture "pub struct $ident;
"
    prove "guard4 rejects pub struct $ident" 1 no_owned_identity "$fixtures/src"
    source_fixture "pub enum $ident { A }
"
    prove "guard4 rejects pub enum $ident" 1 no_owned_identity "$fixtures/src"
    source_fixture "pub trait $ident {}
"
    prove "guard4 rejects pub trait $ident" 1 no_owned_identity "$fixtures/src"
done

source_fixture '// WorkOrder is owned by CodeGG and only mentioned here.
struct WorkOrder;
fn f() -> WorkOrder { WorkOrder }
pub const NOTE: &str = "WorkOrder Goal TodoState";
'
prove "guard4 allows incidental mentions" 0 no_owned_identity "$fixtures/src"

# --- guard 5: impure source access -----------------------------------------
source_fixture 'use std::collections::BTreeMap;
pub fn f() -> BTreeMap<String, String> { BTreeMap::new() }
'
prove "guard5 allows pure std use" 0 no_impure_source "$fixtures/src"

declare -a impure_cases=(
    'std::fs::read_to_string("x").unwrap();'
    'use std::fs; let _ = fs::read_to_string("x");'
    'use std::fs; let _ = fs::read("x");'
    'use std::fs; let _ = fs::write("x", b"y");'
    'use std::fs::File; let _ = File::open("x");'
    'use std::fs::File; let _ = File::create("x");'
    'use std::fs; let _ = fs::create_dir("x");'
    'use std::fs; let _ = fs::remove_file("x");'
    'use std::path::PathBuf; let _ = PathBuf::from("x");'
    'use tempfile::NamedTempFile;'
    'std::process::Command::new("x");'
    'use std::process::Command; Command::new("x");'
    'std::net::TcpStream::connect("x");'
    'let _ = TcpStream::connect("x");'
    'let _ = UdpSocket::bind("x");'
    'reqwest::get("x");'
    'sqlx::query("x");'
    'tokio::spawn(async {});'
)
for case in "${impure_cases[@]}"; do
    source_fixture "$case
"
    prove "guard5 rejects: $case" 1 no_impure_source "$fixtures/src"
done

echo "codegg-compat boundary self-proofs passed"

run_guards "$manifest" "$source_dir"

echo "eggplan-codegg-compat ownership boundary check passed"
