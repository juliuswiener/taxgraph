#!/usr/bin/env bash
# Create the Python environment for GETTSIM (make params-check, params-import).
# The differential test against Catala and the Catala-generated Python runtime
# fell with L4. GETTSIM needs Python >= 3.12, so this env is 3.12. Uses uv as
# environment manager.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "=== create venv oracle/.venv312 (Python 3.12) ==="
uv venv oracle/.venv312 --python 3.12

echo "=== install gettsim ==="
# Die Versionen von gettsim und ttsim-backend stehen NUR in requirements-oracle.txt, mit
# Begruendung im Kopf der Datei; rust/bindung/tests/ci_pins.rs haelt die Pins fest.
uv pip install --python oracle/.venv312/bin/python -r requirements-oracle.txt

echo "=== versions ==="
# Metadaten, NICHT `gettsim.__version__`: das Modul meldet 1.2.1, waehrend die Distribution
# 1.2 heisst -- und nur die zweite kennt pip/uv (requirements-oracle.txt:18-24). Hier stand
# vorher `__version__`, also die Zahl, die nicht zum Pin passt.
oracle/.venv312/bin/python -c "import importlib.metadata as m; print('gettsim', m.version('gettsim'), '/ ttsim-backend', m.version('ttsim-backend'))"
echo "=== done ==="
