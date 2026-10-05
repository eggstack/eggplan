#!/usr/bin/env bash
set -euo pipefail

manifest="crates/eggplan-integrations/Cargo.toml"
source_dir="crates/eggplan-integrations/src"

# Precondition: every guard below runs `rg` inside an `if`. If `rg` is missing,
# each invocation exits 127 inside a conditional, so the `if` takes the else
# branch and the guard reports success without scanning anything. Fail loudly
# instead of reporting an unscanned pass.
for tool in rg awk; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "required boundary-guard tool '$tool' is not installed; refusing to report a pass" >&2
        exit 1
    fi
done

# ---------------------------------------------------------------------------
# Provider integration boundary. Six guards, each stated with the exact scope it
# scans, so a failure message never claims more than was checked.
#
#   1. no_sibling_runtime   manifest, every section
#   2. no_transport_dep      manifest, every section
#   3. no_credential_dep     manifest, every section
#   4. no_process_source     src tree, all files
#   5. no_acquisition_source src tree, all files
#   6. no_trust_enrollment   src tree, all files
#
# Guards 1-3 are deliberately unscoped by section, including
# [dev-dependencies]: an adapter must not pull in a sibling runtime, a
# transport, or a credential helper even for tests. The adapter test suite uses
# local serialized DTO fixtures and needs none of them.
#
# Guards 4-6 scan the whole src tree rather than a hand-listed set of modules, so
# a newly added adapter cannot escape the enforcement by not being named.
#
# Self-proofs at the bottom run the same functions against synthetic fixtures in
# a temporary directory. They never read or write tracked source.
# ---------------------------------------------------------------------------

# Guard 1: no sibling Eggstack runtime may be linked in. Matched on the
# dependency key position, not as a free substring, so Eggplan's own
# `eggplan-codegg-compat` is not a false positive while `eggbench-core` is.
no_sibling_runtime() {
    rg -n '^[[:space:]]*(eggwork|eggsearch|eggbench|eggsact|eggsec|codegg|eggpool)(-[A-Za-z0-9_-]+)?[[:space:]]*=' "$1"
}

# Guard 2: no async runtime, HTTP client, or Git transport.
no_transport_dep() {
    rg -n '^[[:space:]]*(tokio|reqwest|hyper|ureq|surf|isahc|async-std|axum|octocrab|git2|gix)([[:space:]=]|$)' "$1"
}

# Guard 3: no credential or secret-store helper. An adapter is handed
# already-verified facts and must never hold or request a secret.
no_credential_dep() {
    rg -n '^[[:space:]]*(secrecy|keyring|jsonwebtoken|oauth2|wiremock)([[:space:]=]|$)' "$1"
}

# Guard 4: the adapter never executes a sibling process. Both the fully
# qualified and the `use`-imported call forms are matched.
no_process_source() {
    rg -n 'std::process|process::Command|Command::new|std::env::var|\bexec\(|spawn\(' "$1"
}

# Guard 5: no filesystem walk, no network, no executable discovery. Adapters
# normalize facts a host already verified; they never acquire evidence.
no_acquisition_source() {
    rg -n 'std::fs|std::path|tempfile::|fs::(read|read_to_string|write|create_dir|remove_file|rename|copy|File|metadata|canonicalize|read_dir)|File::(open|create|create_new|open_options)|read_dir|std::net|TcpStream|UdpSocket|reqwest::|ureq::|octocrab::|which::|which\(' "$1"
}

# Guard 6: adapter source may not enroll provider trust. Enrolling stays an
# explicit host policy action through ProviderRegistry::register_trusted.
no_trust_enrollment() {
    rg -n 'register_trusted|ProviderRegistry' "$1"
}

# The generic SPI must still be built on the Eggplan core evidence contract.
requires_core_contract() {
    ! rg -q 'eggplan-core' "$1"
}

run_guards() {
    local m="$1" s="$2"

    if no_sibling_runtime "$m"; then
        echo "provider adapters must not depend on a sibling runtime in any section, including [dev-dependencies] (scanned: $m, all sections)" >&2
        return 1
    fi

    if no_transport_dep "$m"; then
        echo "provider adapters must not depend on an async runtime, HTTP client, or Git transport in any section, including [dev-dependencies] (scanned: $m, all sections)" >&2
        return 1
    fi

    if no_credential_dep "$m"; then
        echo "provider adapters must not depend on a credential or secret-store helper in any section, including [dev-dependencies] (scanned: $m, all sections)" >&2
        return 1
    fi

    if no_process_source "$s"; then
        echo "provider SPI must only normalize facts and must not execute a process or read host environment (scanned: $s, all files)" >&2
        return 1
    fi

    if no_acquisition_source "$s"; then
        echo "provider SPI must not acquire evidence: no filesystem walk, network, or executable discovery (scanned: $s, all files)" >&2
        return 1
    fi

    if no_trust_enrollment "$s"; then
        echo "provider adapters cannot enroll trust in a host registry (scanned: $s, all files)" >&2
        return 1
    fi

    if requires_core_contract "$m"; then
        echo "provider SPI must use the Eggplan core evidence contract (scanned: $m, all sections)" >&2
        return 1
    fi
}

# --------------------------------------------------------------------------
# Self-proofs. `prove <label> <expect-fail:0|1> -- <guard-fn> <fixture>`
# --------------------------------------------------------------------------
fixtures="$(mktemp -d)"

# Targeted, non-recursive cleanup: this script only ever creates Cargo.toml and
# src/*.rs below the temporary root.
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
        echo "integrations boundary guard self-proof failed: $label (expected fail=$expect, got fail=$got)" >&2
        exit 1
    fi
}

# --- guard 1: sibling runtimes ----------------------------------------------
manifest_fixture '[package]
name = "x"
[dependencies]
eggplan-core = { path = "../eggplan-core" }
serde.workspace = true
'
prove "guard1 clean manifest" 0 no_sibling_runtime "$fixtures/Cargo.toml"

for case in \
    'eggbench-core = "1"' \
    'eggwork-core = "1"' \
    'eggsact = "1"' \
    'codegg = "1"' \
    '[dev-dependencies]
eggbench-core = "1"'
do
    manifest_fixture "$case
"
    prove "guard1 rejects: $case" 1 no_sibling_runtime "$fixtures/Cargo.toml"
done

# The guard must not fire on Eggplan's own crate names or on a substring.
manifest_fixture '[dependencies]
eggplan-codegg-compat = { path = "../eggplan-codegg-compat" }
'
prove "guard1 allows eggplan-codegg-compat" 0 no_sibling_runtime "$fixtures/Cargo.toml"

# --- guard 2: transport dependencies ---------------------------------------
for case in 'tokio = { version = "1" }' 'reqwest = "0.12"' 'octocrab = "0.4"' 'git2 = "0.19"'; do
    manifest_fixture "[dependencies]
$case
"
    prove "guard2 rejects: $case" 1 no_transport_dep "$fixtures/Cargo.toml"
done
manifest_fixture '[dependencies]
serde.workspace = true
'
prove "guard2 clean manifest" 0 no_transport_dep "$fixtures/Cargo.toml"

# --- guard 3: credential dependencies --------------------------------------
for case in 'secrecy = "0.10"' 'keyring = "3"' 'oauth2 = "5"' 'jsonwebtoken = "9"'; do
    manifest_fixture "[dependencies]
$case
"
    prove "guard3 rejects: $case" 1 no_credential_dep "$fixtures/Cargo.toml"
done
manifest_fixture '[dependencies]
serde.workspace = true
'
prove "guard3 clean manifest" 0 no_credential_dep "$fixtures/Cargo.toml"

# --- guard 4: process execution in source ----------------------------------
source_fixture 'pub fn normalize(bytes: &[u8]) -> usize { bytes.len() }
'
prove "guard4 allows pure normalization" 0 no_process_source "$fixtures/src/lib.rs"

for case in \
    'std::process::Command::new("eggbench").arg("--json");' \
    'use std::process::Command; Command::new("eggbench");' \
    'std::env::var("GITHUB_TOKEN").ok();'
do
    source_fixture "$case
"
    prove "guard4 rejects: $case" 1 no_process_source "$fixtures/src/lib.rs"
done

# --- guard 5: evidence acquisition in source -------------------------------
source_fixture 'use std::collections::BTreeMap;
pub fn f() -> BTreeMap<String, String> { BTreeMap::new() }
'
prove "guard5 allows pure std use" 0 no_acquisition_source "$fixtures/src/lib.rs"

for case in \
    'std::fs::read_to_string("manifest.json").unwrap();' \
    'use std::fs; let _ = fs::read_dir(".");' \
    'use std::fs::File; let _ = File::open("x");' \
    'use std::path::PathBuf; let _ = PathBuf::from("x");' \
    'use tempfile::NamedTempFile;' \
    'let _ = std::net::TcpStream::connect("api.github.com");' \
    'reqwest::get("x");' \
    'octocrab::Octocrab::builder().build();' \
    'which::which("gitbench").unwrap();'
do
    source_fixture "$case
"
    prove "guard5 rejects: $case" 1 no_acquisition_source "$fixtures/src/lib.rs"
done

# --- guard 6: trust enrollment in source -----------------------------------
source_fixture '// Adapters hand a descriptor to the host; they never enroll it.
pub fn descriptor() -> &'"'"'static str { "epp_example" }
'
prove "guard6 allows descriptor-only source" 0 no_trust_enrollment "$fixtures/src/lib.rs"

for case in \
    'registry.register_trusted(descriptor);' \
    'let _r: ProviderRegistry = ProviderRegistry::default();' \
    'fn f(r: &mut ProviderRegistry) { let _ = r; }'
do
    source_fixture "$case
"
    prove "guard6 rejects: $case" 1 no_trust_enrollment "$fixtures/src/lib.rs"
done

# --- guard 7: core contract presence ---------------------------------------
manifest_fixture '[package]
name = "x"
[dependencies]
serde = "1"
'
prove "guard7 rejects a manifest without eggplan-core" 1 requires_core_contract "$fixtures/Cargo.toml"

manifest_fixture '[dependencies]
eggplan-core = { path = "../eggplan-core" }
'
prove "guard7 allows a manifest with eggplan-core" 0 requires_core_contract "$fixtures/Cargo.toml"

echo "integrations boundary self-proofs passed"

run_guards "$manifest" "$source_dir"

echo "integration provider SPI boundary check passed"