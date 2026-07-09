#!/usr/bin/env bash
# Assemble the Catala-generated Python into an importable package for the
# differential-test harness. Run after `clerk build p32a-python`.
#
# Layout produced (under oracle/gettsim/_catala/):
#   rt/   catala_runtime.py, dates.py         -> top-level modules (on sys.path)
#   pkg/  Stdlib_en.py, ..., Einkommensteuertarif.py, __init__.py  -> package
#
# The generated modules import the runtime as a top-level module
# (`from catala_runtime import *`) and their siblings as a package
# (`from . import Stdlib_en`), so both locations must be on sys.path.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

OUT="oracle/gettsim/_catala"
rm -rf "$OUT"
mkdir -p "$OUT/rt" "$OUT/pkg"

cp _build/libcatala/python/*.py "$OUT/pkg/"
cp _build/rules/estg/p32a/python/Einkommensteuertarif.py "$OUT/pkg/"

# Move the runtime out of the package to the top-level module directory.
mv "$OUT/pkg/catala_runtime.py" "$OUT/rt/"
mv "$OUT/pkg/dates.py" "$OUT/rt/"
touch "$OUT/pkg/__init__.py"

echo "assembled Catala python package in $OUT"
