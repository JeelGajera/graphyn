#!/usr/bin/env bash
# Record a snapshot of each side and build the markdown report.
#
# Both sides are analyzed from the same checkout so that both snapshots land in
# one `.graphyn/db`, which is what lets `report` compare them. The base is
# checked out, analyzed, and the head restored — the working tree is left as it
# was found.
set -euo pipefail

path="${GRAPHYN_PATH:-.}"
cd "$path"

# What to compare against. The pull request base is the useful default; a push
# compares against the commit it replaced.
base_ref="${GRAPHYN_BASE_REF:-}"
if [ -z "$base_ref" ]; then
  if [ "${GITHUB_EVENT_NAME:-}" = "pull_request" ]; then
    base_ref="origin/${GITHUB_BASE_REF}"
  else
    base_ref="${GITHUB_SHA:-HEAD}^"
  fi
fi

head_sha="$(git rev-parse HEAD)"

# A shallow clone has no base commit to compare against. Deepening is cheap and
# the alternative is a report that silently describes the wrong range.
if ! git rev-parse --verify --quiet "${base_ref}^{commit}" >/dev/null; then
  echo "Fetching $base_ref (shallow clone)."
  git fetch --no-tags --depth=50 origin "+refs/heads/*:refs/remotes/origin/*" >/dev/null 2>&1 || true
fi

if ! base_sha="$(git rev-parse --verify --quiet "${base_ref}^{commit}")"; then
  echo "::error::Could not resolve base revision '$base_ref'. Pass base-ref explicitly, or set fetch-depth: 0 on actions/checkout." >&2
  exit 1
fi

if [ "$base_sha" = "$head_sha" ]; then
  echo "Base and head are the same commit; nothing to compare."
  {
    echo "verdict=clean"
    echo "report-path="
  } >> "$GITHUB_OUTPUT"
  exit 0
fi

echo "Comparing ${base_sha} -> ${head_sha}"

# The base tree, snapshotted under its own SHA so the record still means
# something after the branch moves.
git checkout --quiet --force "$base_sha"
graphyn analyze . --snapshot "$base_sha" >/dev/null

git checkout --quiet --force "$head_sha"
graphyn analyze . --snapshot "$head_sha" >/dev/null

report_path="${RUNNER_TEMP:-/tmp}/graphyn-report.md"
rules_arg=()
[ -n "${GRAPHYN_RULES:-}" ] && rules_arg=(--rules "${GRAPHYN_RULES}")

set +e
graphyn report . --base "$base_sha" --head "$head_sha" "${rules_arg[@]}" > "$report_path"
status=$?
set -e

case $status in
  0) verdict=clean ;;
  1) verdict=violated ;;
  *)
    # The report could not be produced. That is not a violation and must not be
    # reported as one — nor as a clean run.
    echo "::warning::Graphyn could not produce a report (exit $status). Nothing was enforced." >&2
    cat "$report_path" >&2 || true
    verdict=undecided
    ;;
esac

# "undecided" also covers a report that was produced but could not decide every
# rule, which the markdown states in its first line.
if [ "$verdict" = "clean" ] && grep -q "could not be decided" "$report_path" 2>/dev/null; then
  verdict=undecided
fi

echo "Verdict: $verdict"
{
  echo "verdict=$verdict"
  echo "report-path=$report_path"
} >> "$GITHUB_OUTPUT"

cat "$report_path" >> "$GITHUB_STEP_SUMMARY"
