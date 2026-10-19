#!/usr/bin/env bash
# Keep the source-tree ignore budget from growing. Run from any directory.
set -euo pipefail
root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
baseline_file="$root/.ignored-baseline"
baseline="$(tr -d '\r\n' < "$baseline_file")"
if [[ ! "$baseline" =~ ^[0-9]+$ ]]; then
    echo "Invalid ignored-test baseline: $baseline_file" >&2
    exit 2
fi
# Count occurrences, including #[test] #[ignore] on the same line.
# Restrict to attribute lines so documentation comments do not consume the budget.
# grep returns 1 when there are no matches; that is a valid zero count.
count=$( { grep -rEh --include='*.rs' '^[[:space:]]*#' "$root/contracts/predictify-hybrid/src" || [[ $? == 1 ]]; } |
    { grep -oE '#[[:space:]]*\[[[:space:]]*ignore([[:space:]]|=|\])' || [[ $? == 1 ]]; } | wc -l)
count=$((count))
printf 'Ignored test attributes: %s (baseline: %s)\n' "$count" "$baseline"
if (( count > baseline )); then
    echo "Ignored-test budget exceeded. Restore tests before adding an ignore." >&2
    exit 1
fi
