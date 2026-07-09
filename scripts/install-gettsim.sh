#!/usr/bin/env bash
# Create the Python environment for the differential test: GETTSIM plus the
# Catala-generated Python runtime. The Catala runtime uses PEP-695 syntax and
# needs Python >= 3.12, so this env is 3.12. Uses uv as environment manager.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "=== create venv oracle/.venv312 (Python 3.12) ==="
uv venv oracle/.venv312 --python 3.12

echo "=== install gettsim ==="
uv pip install --python oracle/.venv312/bin/python gettsim

echo "=== versions ==="
oracle/.venv312/bin/python -c "import gettsim; print('gettsim', getattr(gettsim, '__version__', '?'))"
echo "=== done ==="
