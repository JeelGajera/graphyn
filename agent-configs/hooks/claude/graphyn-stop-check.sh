#!/bin/sh
# Stop — refuse to finish on a rule violation.
#
# The last gate before the agent hands back. `graphyn check --diff-only` judges
# the uncommitted change against the last commit, and a violation exits 2,
# which is what tells Claude Code to keep going and feeds stderr back as the
# reason. The agent then has the specific rule failure, not "something is
# wrong".
#
# Only exit code 1 blocks — a rule broken on resolved evidence. Exit 2 from
# `check` means the check could not run at all (no graph, no snapshot,
# unreadable rules), and refusing to stop over a misconfigured tool would trap
# the agent in a loop it cannot fix.

# Locate the shared library.
#
# Guarded with a readability test rather than `. lib || fallback`: in POSIX sh,
# sourcing a file that does not exist is a special-builtin failure that
# terminates the shell on the spot, so the fallback never runs and the hook
# exits non-zero with no output. For a pre-commit hook that silently rejects
# every commit; for a Stop hook it traps the agent. Both were real.
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
# No library means no hook. Exiting 0 leaves the agent, and the commit, alone.
[ -n "$_graphyn_lib" ] || exit 0
# shellcheck source=/dev/null
. "$_graphyn_lib"

TIMEOUT="${GRAPHYN_HOOK_TIMEOUT:-30}"

bin=$(graphyn_bin)
[ -n "$bin" ] || exit 0
graphyn_has_graph || exit 0

root=$(graphyn_root)
[ -f "$root/.graphyn/rules.toml" ] || exit 0

output=$(graphyn_with_timeout "$TIMEOUT" "$bin" check "$root" --diff-only 2>&1)
status=$?

if [ $status -eq 1 ]; then
  printf 'Graphyn rules were violated by this change:\n\n%s\n\nFix the violations above, or amend the rule in .graphyn/rules.toml if it is wrong.\n' "$output" >&2
  exit 2
fi

# 0 = clean. 2 = the check could not run. 124 = timed out. None of these is a
# reason to hold the agent, and reporting a rule failure for any of them would
# be claiming a verdict that was never reached.
exit 0
