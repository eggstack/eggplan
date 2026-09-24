#!/usr/bin/env bash
set -euo pipefail

crate="crates/eggplan-integrations"
cargo_toml="$crate/Cargo.toml"

if rg -n -i 'eggwork|eggsearch|eggbench|eggsact|tokio|reqwest|hyper|mcp|async.runtime' "$cargo_toml" "$crate/src"; then
  echo "provider SPI must not depend on sibling runtimes, transports, or acquisition" >&2
  exit 1
fi

if rg -n '\b(async\s+fn|tokio::|reqwest::|std::process|Command::new)\b' "$crate/src"; then
  echo "provider SPI must only normalize facts and must not acquire or execute evidence" >&2
  exit 1
fi

if ! rg -q 'eggplan-core' "$cargo_toml"; then
  echo "provider SPI must use the Eggplan core evidence contract" >&2
  exit 1
fi

echo "integration provider SPI boundary check passed"
