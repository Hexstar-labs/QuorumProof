#!/bin/bash
# Run every linter configured for the repo (Issue #1679).
#
#   ./scripts/lint.sh            # run all linters that are installed
#   ./scripts/lint.sh rust       # only clippy
#   ./scripts/lint.sh python     # only ruff
#   ./scripts/lint.sh js         # frontend + dashboard ESLint
#
# A linter whose tool is missing is reported and skipped rather than failing,
# so the script is usable on a fresh machine. CI runs the JS linters directly
# (see .github/workflows/ci.yml); see docs/linting.md for the full standard.
set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT" || exit 1

target="${1:-all}"
failed=0

run_rust() {
  if ! command -v cargo >/dev/null 2>&1; then
    echo "[skip] cargo not installed"
    return 0
  fi
  echo "==> clippy (workspace)"
  cargo clippy --workspace --lib --target x86_64-unknown-linux-gnu -- -D warnings || failed=1
}

run_python() {
  if ! command -v ruff >/dev/null 2>&1; then
    echo "[skip] ruff not installed (pip install ruff)"
    return 0
  fi
  echo "==> ruff"
  ruff check . || failed=1
  ruff format --check . || failed=1
}

run_js() {
  for dir in frontend dashboard; do
    if [ ! -d "$dir/node_modules" ]; then
      echo "[skip] $dir/node_modules missing (run npm ci in $dir)"
      continue
    fi
    echo "==> eslint ($dir)"
    (cd "$dir" && npm run lint) || failed=1
  done
}

case "$target" in
  all)    run_rust; run_python; run_js ;;
  rust)   run_rust ;;
  python) run_python ;;
  js)     run_js ;;
  *)      echo "usage: $0 [all|rust|python|js]" >&2; exit 2 ;;
esac

exit "$failed"
