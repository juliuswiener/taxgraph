#!/usr/bin/env python3
"""Einmalige Messung (2026-08-10, Bankverbindungs-Baustein) gegen die amtliche checkESt-Pruefung.

Beantwortet zwei offene Fragen aus dem Team-Lead-Briefing:
  1. Braucht Person B unter Zusammenveranlagung ein EIGENES Bankverbindungs-Feld, oder reicht
     Person As IBAN (eine Erstattung, eine Bankverbindung)?
  2. Verlangt checkESt den BIC (E0102201) bei einer Auslands-IBAN (E0102603) wirklich, oder
     bietet/akzeptiert sie ihn nur optional?

Fall 3b umgeht bewusst unser eigenes fail-closed-Gate (deklariere() blockt eine Auslands-IBAN
ohne BIC laengst VOR jeder XML-Erzeugung) -- das ist hier Absicht: um zu sehen, was ELSTER SELBST
ohne unsere Gate-Logik dazu sagt, wird der BIC-Kz nach deklariere() manuell aus der (sonst
identischen) Deklaration aus Fall 3a entfernt und `vollstaendig` erzwungen.

Aufruf: set -a; . ./.env; set +a; python3 scripts/measure_bankverbindung.py
Gibt NIE die Hersteller-ID aus.
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


def _basis_ohne_keine_bankverbindung(basis: tuple) -> tuple:
    return tuple((f, w) for f, w in basis if f != "stammdaten_keine_bankverbindung")


def _bauen(felder: tuple, fall_id: str):
    s = ST.leerer_store(2025, fall_id=fall_id)
    for f, w in felder:
        T._b(s, f, w)
    snap, _ = ST.materialisiere(s)
    dekl = est_mapping.deklariere(snap, BINDUNG)
    if not dekl["eingaben_konsistent"]:
        sys.exit(f"[{fall_id}] unerwartet unvollstaendig: {dekl['unvollstaendig']}")
    return dekl


# ---- 1) IBAN-Variante, Einzel (Ersatz fuer stammdaten_keine_bankverbindung) ----
felder = _basis_ohne_keine_bankverbindung(T._BASIS_A) + (
    ("stammdaten_iban", "DE02120300000000202051"), ("veranlagung", "einzel"))
dekl_1 = _bauen(felder, "messung_iban_einzel")
_pruefe(dekl_1, "1) IBAN-Variante, Einzel")

# ---- 2) Zusammenveranlagung: A traegt IBAN, B OHNE eigenes Bankverbindungs-Feld ----
felder = (_basis_ohne_keine_bankverbindung(T._BASIS_A) + T._BASIS_B
          + (("stammdaten_iban", "DE02120300000000202051"), ("veranlagung", "zusammen")))
dekl_2 = _bauen(felder, "messung_iban_zusammen_ohne_b")
_pruefe(dekl_2, "2) IBAN-Variante, Zusammen (Person B ohne eigenes Bankverbindungs-Feld)")

# ---- 3a) Auslands-IBAN MIT BIC (durch unser eigenes Gate) ----
felder = _basis_ohne_keine_bankverbindung(T._BASIS_A) + (
    ("stammdaten_iban", "AT611904300234573201"), ("stammdaten_bic", "PBNKDEFF370"),
    ("veranlagung", "einzel"))
dekl_3a = _bauen(felder, "messung_ausland_mit_bic")
_pruefe(dekl_3a, "3a) Auslands-IBAN MIT BIC")

# ---- 3b) Auslands-IBAN OHNE BIC (Tamper -- unser Gate umgangen, s. Docstring oben) ----
dekl_3b = dict(dekl_3a)
dekl_3b["deklaration"] = {k: v for k, v in dekl_3a["deklaration"].items() if k != "E0102201"}
dekl_3b["eingaben_konsistent"] = True
_pruefe(dekl_3b, "3b) Auslands-IBAN OHNE BIC (Tamper -- unser Gate umgangen)")

print("Fertig.")
