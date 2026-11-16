#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

MODE="all"
if [[ "${1:-}" == "--quick" ]]; then
  MODE="quick"
elif [[ "${1:-}" == "--visual" ]]; then
  MODE="visual"
fi

echo "==> [EqnTide Test Runner] mode=$MODE"

if [[ "$MODE" == "visual" ]]; then
  cargo test --test export_verification -- --nocapture
  echo ""
  echo "Visual PNGs ready for AI inspection under: target/test_exports/visual/"
  ls -1 target/test_exports/visual/*.png
  echo "Index: target/test_exports/visual/INDEX.md"
  exit 0
fi

echo "==> 1. Workspace unit & library tests"
cargo test --workspace --all-targets

echo "==> 2. Conversion matrix & cross-conversion integration tests"
cargo test --test conversion_matrix --test cross_conversion

if [[ "$MODE" == "all" ]]; then
  echo "==> 3. Format export verification & AI visual layout sheets"
  cargo test --test export_verification -- --nocapture
fi

echo "==> 4. Python packaging & Word DOTM/VBA tests"
PY_ENV=""
if ! python3 -c "import olefile, oletools" >/dev/null 2>&1; then
  for candidate in "$HOME"/.local/lib/python*/site-packages; do
    if [[ -d "$candidate/oletools" ]]; then
      PY_ENV="$candidate${PYTHONPATH:+:$PYTHONPATH}"
      break
    fi
  done
fi
if [[ -n "$PY_ENV" ]]; then
  PYTHONPATH="$PY_ENV" python3 -m unittest discover -s tests/python -p "test_*.py" -v
else
  python3 -m unittest discover -s tests/python -p "test_*.py" -v
fi

echo "==> 5. WPS Office add-in syntax & Node.js regression tests"
node --check wps-addin/main.js
node --check wps-addin/taskpane.js
if [[ ! -d tests/wps/node_modules ]]; then
  npm ci --prefix tests/wps
fi
npm test --prefix tests/wps

echo ""
echo "All selected EqnTide test suites PASSED."
