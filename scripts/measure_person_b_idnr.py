#!/usr/bin/env python3
"""Einmalige Messung (2026-08-12, person_b_idnr-Blocker) gegen die amtliche checkESt-Pruefung.

Beantwortet die Frage, die zum Fix (commit 1420bb6, BACKLOG person-b-idnr-wird-abgelehnt)
gefuehrt hat: lehnt ERiC E0100082 (person_b_idnr) amtlich ab, UNABHAENGIG vom gesendeten
Wert? Zwei Faelle auf DERSELBEN sonst identischen Zusammenveranlagungs-Deklaration:

  1) NACHHER (aktueller Stand): person_b_idnr wird nicht mehr deklariert (elster_kz: null
     in bindung_an_gesamt.yaml) -- Kontrollmessung, dass der reguläre Pfad sauber bleibt.
  2) VORHER (Tamper): dieselbe Deklaration, E0100082 nachtraeglich manuell in die
     Kz-Dict eingefuegt (wie es vor dem Fix ueber AN_GESAMT_PARTNER/GESAMT_PARTNER_19
     dekleriert worden waere) -- zeigt, was ELSTER SELBST dazu sagt, unabhaengig von
     unserer eigenen Bindungstabelle. Zwei Werte (eine echte, gueltige IdNr UND eine
     Dummy-Ziffernfolge), um zu pruefen, ob die Ablehnung wert-unabhaengig ist.

Aufruf: set -a; . ./.env; set +a; python3 scripts/measure_person_b_idnr.py
Gibt NIE die Hersteller-ID oder eine echte IdNr aus (nur Dummy-Werte im Code, nichts geloggt).
"""
from __future__ import annotations

import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
for sub in ("produkt/import", "produkt/mapping", "produkt/store", "produkt/traverser",
            "elster", "tests"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import checkest_gate as CE                # noqa: E402
import elster_xml as EX                   # noqa: E402
import est_mapping                        # noqa: E402
import store as ST                        # noqa: E402
import traverser as TR                    # noqa: E402
import test_checkest_durchstich as T      # noqa: E402 -- Fixtur-Wiederverwendung (_BASIS_A/_B, _ABSENDER, _hid, _b)

HID = T._hid()
if not HID:
    sys.exit("ELSTER_HERSTELLER_ID fehlt -- set -a; . ./.env; set +a")
if not os.path.isdir(os.environ.get("ERIC_DIR", os.path.expanduser("~/02_Software/eric"))):
    sys.exit("$ERIC_DIR fehlt -- ERiC nicht entpackt/gesetzt")

BINDUNG = TR.lade_bindung()


def _pruefe(dekl: dict, label: str) -> tuple[int, list[str]]:
    xml = EX.erzeuge_xml(dekl, vz=2025, hersteller_id=HID, abgabefaehig=True, **T._ABSENDER)
    rc, antwort = CE.validate(xml, "ESt_2025")
    texte = [" ".join(t.split()) for t in re.findall(r"<Text>(.*?)</Text>", antwort or "", re.S)]
    klasse = CE.klassifiziere_rc(rc)
    print(f"--- {label} ---")
    print(f"rc={rc} klasse={klasse}")
    for t in texte:
        print(f"  - {t}")
    print()
    return rc, texte


def _bauen(felder: tuple, fall_id: str):
    s = ST.leerer_store(2025, fall_id=fall_id)
    for f, w in felder:
        T._b(s, f, w)
    snap, _ = ST.materialisiere(s)
    dekl = est_mapping.deklariere(snap, BINDUNG)
    if not dekl["vollstaendig"]:
        sys.exit(f"[{fall_id}] unerwartet unvollstaendig: {dekl['unvollstaendig']}")
    return dekl


felder = T._BASIS_A + T._BASIS_B + (("veranlagung", "zusammen"),)

# ---- 1) NACHHER: regulaer, person_b_idnr wird nicht deklariert (aktueller Stand) ----
dekl_1 = _bauen(felder, "messung_zusammen_ohne_idnr")
assert "E0100082" not in dekl_1["deklaration"], (
    "person_b_idnr steht noch in der Deklaration -- Fix nicht (mehr) wirksam?")
_pruefe(dekl_1, "1) NACHHER — person_b_idnr nicht deklariert (aktueller Stand)")

# ---- 2a) VORHER (Tamper): E0100082 mit einer echten, gueltigen Dummy-IdNr nachtraeglich ----
dekl_2a = dict(dekl_1)
dekl_2a["deklaration"] = {**dekl_1["deklaration"], "E0100082": "02476291358"}
_pruefe(dekl_2a, "2a) VORHER (Tamper) — E0100082 mit gueltiger Dummy-IdNr")

# ---- 2b) VORHER (Tamper): E0100082 mit einer Nullen-IdNr — Ablehnung wertunabhaengig? ----
dekl_2b = dict(dekl_1)
dekl_2b["deklaration"] = {**dekl_1["deklaration"], "E0100082": "00000000000"}
_pruefe(dekl_2b, "2b) VORHER (Tamper) — E0100082 mit Nullen-IdNr")

print("Fertig.")
