#!/bin/sh
# Stop — refuse to finish on an audit finding.
#
# The counterpart to graphyn-stop-check.sh. That one enforces the rules a
# repository wrote down; this one reports changes that look like they were made
# to pass a check rather than to work — which is the failure mode a rule file
# cannot describe in advance.
#
# Only exit 1 from `audit` blocks: a finding at the configured severity. Exit 2
# means the audit could not run at all, and holding an agent over a missing
# snapshot would trap it in a loop it has no way to fix.
#
# Findings are advisory by default here. The severity floor is `error`, so the
# two high-confidence error detectors block and dead-on-arrival does not; set
# GRAPHYN_AUDIT_SEVERITY=warn to hold on everything.

_graphyn_lib=""
for _candidate in \
  "$(dirname "$0")/graphyn-hook-lib.sh" \
  "$(dirname "$0")/../lib/graphyn-hook-lib.sh" \
  "$(git rev-parse --show-toplevel 2>/dev/null)/agent-configs/hooks/lib/graphyn-hook-lib.sh"
do
  if [ -r "$_candidate" ]; then
    _graphyn_lib="$_candidate"
    break
  fi
done
[ -n "$_graphyn_lib" ] || exit 0
# shellcheck source=/dev/null
. "$_graphyn_lib"

TIMEOUT="${GRAPHYN_HOOK_TIMEOUT:-30}"
SEVERITY="${GRAPHYN_AUDIT_SEVERITY:-error}"

bin=$(graphyn_bin)
[ -n "$bin" ] || exit 0
graphyn_has_graph || exit 0

root=$(graphyn_root)

output=$(graphyn_with_timeout "$TIMEOUT" "$bin" audit "$root" --base HEAD --head worktree --severity "$SEVERITY" 2>&1)
status=$?

if [ $status -eq 1 ]; then
  printf 'Graphyn audit found changes that look like they were made to pass a check rather than to work:\n\n%s\n\nFix the change, or record a deliberate exception in .graphyn/audit-ignore with a reason.\n' "$output" >&2
  exit 2
fi

# 0 = nothing found. 2 = the audit could not run. 124 = timed out. None of
# these is a finding, and reporting one for any of them would be the same
# unearned claim the audit exists to catch.
exit 0
