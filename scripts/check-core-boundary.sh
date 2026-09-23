#!/usr/bin/env bash
set -euo pipefail

manifest="crates/eggplan-core/Cargo.toml"
source_dir="crates/eggplan-core/src"

if rg -ni '(^|[[:space:]])(tokio|async-std|reqwest|hyper|sqlx|rusqlite|diesel|subprocess|bollard|rmcp|openai|ollama)([[:space:]]|=|$)' "$manifest"; then
  echo "prohibited dependency family found in eggplan-core" >&2
  exit 1
fi

if rg -n 'pub[[:space:]]+[^;]*(thinking|scratchpad|chain_of_thought|hidden_reasoning)' "$source_dir"; then
  echo "reasoning/transcript field found in eggplan-core public source" >&2
  exit 1
fi

echo "eggplan-core static boundary check passed"
