#!/usr/bin/env python3
"""Vollsweep: jedes Kz-tragende askable Feld EINZELN gegen die amtliche checkESt-Pruefung.

Rekonstruiert den "ersten Wurf" aus tests/test_checkest_feldmatrix.py (Docstring dort,
Zeile 116-118): "Der erste Wurf zog alle 55 Kz-tragenden askable Felder automatisch aus
der Bindung. 29 davon wurden rot." Dieser Lauf wurde nie als Artefakt committet -- nur die
kuratierte Ueberlebende-Liste (MATRIX) und Prosa-Kommentare blieben zurueck. BACKLOG-Eintrag
feldmatrix-21-unklassifiziert: 21 der urspruenglichen 29 roten Felder sind bis heute nicht
klassifiziert (BEFUND / BLOCK-ARTEFAKT / PROBEWERT-ARTEFAKT).

Nur MESSEN. Schreibt NICHT in tests/test_checkest_feldmatrix.py oder
tests/test_checkest_blockmatrix.py -- Team-Lead-Vorgabe, ein anderer Worker editiert
Letztere gerade parallel.

Wiederverwendet _scharf()/_mit()/_fall_einzel() 1:1 aus tests/test_checkest_feldmatrix.py --
NICHT neu gebaut, damit garantiert derselbe Pfad (inkl. _mit_ring_werten) gemessen wird wie
im offiziellen Gate. s. tests/test_checkest_feldmatrix.py:63-74: eine erste Fassung ging
direkt store -> est_mapping.deklariere und uebersprang die Ring-Injektionen (E0205508,
KAP-Antragsfelder) -- genau dieser Fehler soll hier nicht wiederholt werden.

Feld-Pool: glob produkt/bindung/bindung_*.yaml, gemerged wie
tests/test_deklarations_abdeckung.py::merged -- askable UND elster_kz gesetzt.
beispielwert je Feld aus derselben Bindungstabelle (dieselbe Quelle wie dort ::_snapshot).

Aufruf: set -a; . ./.env; set +a; python3 scripts/measure_feldmatrix_vollsweep.py
Gibt NIE Hersteller-ID/Steuernummer/IdNr aus.
"""
from __future__ import annotations

import glob
import json
import os
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "tests"))

import test_checkest_feldmatrix as TCF   # noqa: E402 -- liefert _scharf/_mit/_fall_einzel 1:1
import yaml                              # noqa: E402

CE = TCF.CE
HID = TCF._HID
if not HID:
    sys.exit("ELSTER_HERSTELLER_ID fehlt -- set -a; . ./.env; set +a")
if not os.path.isdir(os.environ.get("ERIC_DIR", os.path.expanduser("~/02_Software/eric"))):
    sys.exit("$ERIC_DIR fehlt -- ERiC nicht entpackt/gesetzt")

BINDUNG_GLOB = os.path.join(ROOT, "produkt", "bindung", "bindung_*.yaml")


def _merged() -> dict:
    d: dict = {}
    for f in sorted(glob.glob(BINDUNG_GLOB)):
        for b in (yaml.safe_load(open(f, encoding="utf-8")).get("bindungen") or []):
            d[b["feld_id"]] = b
    return d


def _pool(merged: dict) -> list:
    """Kz-tragende askable Felder, alphabetisch -- der Feld-Pool des 'ersten Wurfs'."""
    return sorted(fid for fid, b in merged.items() if b.get("askable") and b.get("elster_kz"))


def main() -> None:
    merged = _merged()
    pool = _pool(merged)
    print(f"Pool: {len(pool)} Kz-tragende askable Felder", flush=True)

    basis_rc, basis_texte = TCF._scharf(TCF._fall_einzel())
    if basis_rc != CE.RC_OK:
        sys.exit(f"Basisfall selbst nicht plausibel (rc={basis_rc}) -- Messung sinnlos.\n"
                 + "\n".join(basis_texte))
    print(f"Basisfall rc={basis_rc} (RC_OK) -- Messung startet.\n", flush=True)

    ergebnisse = []
    for i, fid in enumerate(pool, 1):
        wert = merged[fid]["beispielwert"]
        try:
            rc, texte = TCF._scharf(TCF._mit(fid, wert))
            klasse = CE.klassifiziere_rc(rc)
        except Exception as exc:  # Feld verweigert schon den Fallbau (z.B. Enum-Konflikt)
            rc, texte, klasse = -1, [f"EXCEPTION: {exc!r}"], "exception"
        status = "GRUEN" if rc == CE.RC_OK else "ROT"
        print(f"[{i}/{len(pool)}] {fid}={wert!r} rc={rc} klasse={klasse} {status}", flush=True)
        ergebnisse.append(dict(feld_id=fid, wert=wert, elster_kz=merged[fid].get("elster_kz"),
                               rc=rc, klasse=klasse, texte=texte))

    rot = [e for e in ergebnisse if e["rc"] != CE.RC_OK]
    print(f"\n{len(rot)}/{len(pool)} rot.\n", flush=True)
    for e in rot:
        print(f"ROT {e['feld_id']} (kz={e['elster_kz']}) rc={e['rc']} klasse={e['klasse']}")
        for t in e["texte"][:3]:
            print(f"   - {t}")

    out_path = os.path.join(ROOT, "scripts", "_vollsweep_rohdaten.json")
    with open(out_path, "w", encoding="utf-8") as fh:
        json.dump(dict(pool_groesse=len(pool), rot_anzahl=len(rot), ergebnisse=ergebnisse),
                  fh, indent=2, ensure_ascii=False)
    print(f"\nRohdaten: {out_path}")


if __name__ == "__main__":
    main()
