#!/bin/bash
set -e

if [ "$#" -ne 2 ]; then
    echo "Usage: $0 <baseline_gas> <new_gas>"
    exit 1
fi

BASELINE=$1
NEW=$2

echo "Comparing baseline gas ($BASELINE) with new gas ($NEW)..."

awk -v base="$BASELINE" -v new="$NEW" 'BEGIN {
    if (base == 0) {
        print "Baseline gas is 0. Cannot compute regression."
        exit 1
    }
    diff = new - base
    pct = (diff / base) * 100
    printf "Gas change: %.2f%%\n", pct
    if (pct > 5.0) {
        print "Error: Gas regression exceeds 5% CPU/instruction budget!"
        exit 1
    }
    print "Gas budget check passed."
    exit 0
}'
