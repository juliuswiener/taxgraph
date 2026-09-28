#!/usr/bin/env python3
"""Parity-Orakel: langlaufender Prozess, JSON-Zeilen auf stdin/stdout, ruft
`produkt/engine/runner.py`s Catala-Aufrufe fuer VZ 2024..2026 (REWRITE_PLAN.md §5).

Ein Request pro Zeile: {"fn": "grundtarif", "zve_cent": <int>, "vz": <2024|2025|2026>}
Eine Antwort pro Zeile: {"ok": true, "cent": <int>} oder {"ok": false, "error": "<str>"}

`cent` ist in beiden Richtungen CENT (nicht Euro) -- direkt vergleichbar mit der
Rust-Seite (`catala-sys::grundtarif` liefert Cent).

Start: python3 tools/parity/oracle.py   (von der Repo-Wurzel; setzt sys.path selbst)
"""

from __future__ import annotations

import json
import os
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
sys.path.insert(0, ROOT)

_CAT = os.path.join(ROOT, "oracle", "gettsim", "_catala")
sys.path.insert(0, os.path.join(_CAT, "rt"))
sys.path.insert(0, _CAT)

from pkg import Einkommensteuertarif as E  # noqa: E402  (Catala-generated)
from catala_runtime import Money  # noqa: E402

VZ_ENUM = {
    2024: E.Veranlagungszeitraum(E.Veranlagungszeitraum.Code.VZ2024, None),
    2025: E.Veranlagungszeitraum(E.Veranlagungszeitraum.Code.VZ2025, None),
    2026: E.Veranlagungszeitraum(E.Veranlagungszeitraum.Code.VZ2026, None),
}


def _grundtarif(req: dict) -> int:
    m = Money(f"{int(req['zve_cent']) // 100}.{int(req['zve_cent']) % 100:02d}")
    out = E.grundtarif(E.GrundtarifIn(
        zu_versteuerndes_einkommen_in=m, veranlagungszeitraum_in=VZ_ENUM[int(req["vz"])]))
    return int(out.tarifliche_steuer)


def _splittingtarif(req: dict) -> int:
    m = Money(f"{int(req['zve_cent']) // 100}.{int(req['zve_cent']) % 100:02d}")
    out = E.splittingtarif(E.SplittingtarifIn(
        zu_versteuerndes_einkommen_gemeinsam_in=m, veranlagungszeitraum_in=VZ_ENUM[int(req["vz"])]))
    return int(out.tarifliche_steuer)


DISPATCH = {
    "grundtarif": _grundtarif,
    "splittingtarif": _splittingtarif,
}


def main() -> None:
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        req = json.loads(line)
        try:
            fn = DISPATCH[req["fn"]]
            cent = fn(req)
            resp = {"ok": True, "cent": cent}
        except Exception as exc:  # noqa: BLE001 -- an Fehler weiterreichen, nicht sterben
            resp = {"ok": False, "error": f"{type(exc).__name__}: {exc}"}
        sys.stdout.write(json.dumps(resp) + "\n")
        sys.stdout.flush()


if __name__ == "__main__":
    main()
