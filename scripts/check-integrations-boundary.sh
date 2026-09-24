#!/usr/bin/env bash
set -euo pipefail

crate="crates/eggplan-integrations"
cargo_toml="$crate/Cargo.toml"

if grep -nE 'eggwork|eggsearch|eggbench|eggsact|tokio|reqwest|hyper|mcp|async.runtime' "$cargo_toml"; then
  echo "provider adapters must not depend on sibling runtimes, transports, or acquisition" >&2
  exit 1
fi

if grep -RInE '^use[[:space:]]+(eggwork|eggsearch|eggbench|eggsact)::|\b(async[[:space:]]+fn|tokio::|reqwest::|std::process|Command::new)\b' "$crate/src"; then
  echo "provider SPI must only normalize facts and must not acquire or execute evidence" >&2
  exit 1
fi

if grep -RInE 'register_trusted|ProviderRegistry' "$crate/src/eggwork.rs" "$crate/src/eggsearch.rs"; then
  echo "provider adapters cannot enroll trust in a host registry" >&2
  exit 1
fi

if ! grep -q 'eggplan-core' "$cargo_toml"; then
  echo "provider SPI must use the Eggplan core evidence contract" >&2
  exit 1
fi

echo "integration provider SPI boundary check passed"
