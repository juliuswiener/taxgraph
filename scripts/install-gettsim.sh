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
# Dieselben zwei Pins wie requirements-oracle.txt. Beide noetig: gettsim 1.2 verlangt nur
# `ttsim-backend>=1.2`, und ein freies gettsim zieht heute 1.3.1 mit ttsim-backend 1.3.2 --
# der Harness kennt die neue Schnittstelle nicht (ValueError: data columns are missing).
# Hebung nur beide zusammen, mit Harness-Anpassung (requirements-oracle.txt, Kommentar oben).
uv pip install --python oracle/.venv312/bin/python gettsim==1.2 ttsim-backend==1.2.1

echo "=== versions ==="
# Metadaten, NICHT `gettsim.__version__`: das Modul meldet 1.2.1, waehrend die Distribution
# 1.2 heisst -- und nur die zweite kennt pip/uv (requirements-oracle.txt:18-24). Hier stand
# vorher `__version__`, also die Zahl, die nicht zum Pin passt.
oracle/.venv312/bin/python -c "import importlib.metadata as m; print('gettsim', m.version('gettsim'), '/ ttsim-backend', m.version('ttsim-backend'))"
echo "=== done ==="
