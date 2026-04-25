#!/bin/bash
# Run cargo-mutants against the three QuorumProof contract crates.
# Usage: ./scripts/mutation_test.sh [--in-place] [extra cargo-mutants flags]
#
# Outputs:
#   mutants.out/         — full results directory
#   mutants.out/missed   — mutants not caught by any test (test gaps)
#
# Exit codes:
#   0  all mutants caught (or unviable)
#   1  missed mutants detected — improve tests for the listed functions
#   2  cargo-mutants not installed

set -euo pipefail

if ! command -v cargo-mutants &>/dev/null; then
    echo "cargo-mutants not found. Install with:"
    echo "  cargo install cargo-mutants"
    exit 2
fi

echo "==> Running mutation tests on quorum_proof, sbt_registry, zk_verifier..."

cargo mutants \
    --package quorum_proof \
    --package sbt_registry \
    --package zk_verifier \
    "$@"

# Summarise results
MISSED=$(grep -c '^MISSED' mutants.out/outcomes.json 2>/dev/null || true)
CAUGHT=$(grep -c '^CAUGHT' mutants.out/outcomes.json 2>/dev/null || true)

echo ""
echo "==> Mutation summary"
echo "    Caught : ${CAUGHT:-see mutants.out/}"
echo "    Missed : ${MISSED:-see mutants.out/}"

if [[ -s mutants.out/missed ]]; then
    echo ""
    echo "==> Missed mutants (add tests to cover these):"
    cat mutants.out/missed
    exit 1
fi

echo "==> All mutants caught."
