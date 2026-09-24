#!/usr/bin/env bash
set -euo pipefail

manifest="crates/eggplan-codegg-compat/Cargo.toml"
source_dir="crates/eggplan-codegg-compat/src"

# Compatibility assessment is pure: persistence and Git authority stay with
# the host application, not in the production bridge dependency graph.
if awk '
    /^\[dependencies\]/ { active = 1; next }
    /^\[/ { active = 0 }
    active && /eggplan-repo[[:space:]]*=/ { found = 1 }
    END { exit !found }
' "$manifest"; then
    echo "compatibility production dependencies must not include eggplan-repo" >&2
    exit 1
fi

if rg -n '^[[:space:]]*(codegg|codegg-core)[[:space:]]*=' "$manifest"; then
    echo "compatibility crate must not depend on CodeGG" >&2
    exit 1
fi

if rg -n '^[[:space:]]*pub[[:space:]]+(struct|enum|trait|type)[[:space:]]+(WorkOrder|Goal|GoalVerification|TodoState|WorkPlanCheckpoint|ContextEpoch|AgentRunExecutor|JobExecutor|WorktreePolicy|SandboxPolicy)\b' "$source_dir"; then
    echo "compatibility crate declares ownership outside the WorkPlan seam" >&2
    exit 1
fi

echo "eggplan-codegg-compat ownership boundary check passed"
