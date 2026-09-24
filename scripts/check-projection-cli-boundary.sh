#!/usr/bin/env bash
set -euo pipefail

projection_manifest="crates/eggplan-projection/Cargo.toml"
cli_manifest="crates/eggplan-cli/Cargo.toml"
cli_src="crates/eggplan-cli/src"

if grep -RInE 'eggplan-cli|eggplan-repo|clap|crossterm|ratatui|std::process|Command::new|tokio::|reqwest::|hyper::|mcp' "$projection_manifest" crates/eggplan-projection/src; then
  echo "projection library must stay reusable and free of CLI, repository, terminal, process, and transport dependencies" >&2
  exit 1
fi

if grep -RInE 'tokio|reqwest|hyper|mcp|async.runtime' "$cli_manifest" "$cli_src"; then
  echo "CLI must not introduce a runtime, network client, or MCP service" >&2
  exit 1
fi

if grep -RInE 'std::process::Command|Command::new|\.output\(\)|\.spawn\(\)' "$cli_src"; then
  echo "CLI must not execute commands to produce evidence" >&2
  exit 1
fi

echo "projection/CLI boundary check passed"
