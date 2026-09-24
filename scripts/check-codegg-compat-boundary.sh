#!/usr/bin/env bash
set -euo pipefail

manifest="crates/eggplan-codegg-compat/Cargo.toml"
source_dir="crates/eggplan-codegg-compat/src"

if rg -n '^[[:space:]]*(codegg|codegg-core)[[:space:]]*=' "$manifest"; then
    echo "compatibility crate must not depend on CodeGG" >&2
    exit 1
fi

if rg -n '^[[:space:]]*pub[[:space:]]+(struct|enum|trait|type)[[:space:]]+(WorkOrder|Goal|GoalVerification|TodoState|WorkPlanCheckpoint|ContextEpoch|AgentRunExecutor|JobExecutor|WorktreePolicy|SandboxPolicy)\b' "$source_dir"; then
    echo "compatibility crate declares ownership outside the WorkPlan seam" >&2
    exit 1
fi

echo "eggplan-codegg-compat ownership boundary check passed"
