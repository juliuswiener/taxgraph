"""Zone A: Reisekosten/Verpflegung (21 Felder) für Ring<->Deklaration-Differential.

Prüf-Logik aus test_ring_deklaration_differential importieren — eine Wahrheit.
"""
from __future__ import annotations

import os
import sys

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
for sub in ("produkt/import", "produkt/mapping", "produkt/store", "produkt/traverser"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import elster_xml as EX           # noqa: E402
import est_mapping                # noqa: E402
import store as ST                # noqa: E402
import traverser as TR            # noqa: E402
from test_ring_deklaration_differential import (  # noqa: E402
    _b, _pruefe_differential)

HID = "74931"
TS = "2026-08-04T14:00:00+00:00"
H = {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}


@pytest.fixture(scope="module")
def bindung():
    return TR.lade_bindung()


def test_zone_a_reisekosten_keine_luecken(bindung):
    """AN mit Reisekosten (EP + Verpflegung + Übernachtung + dHf) — 21 Felder."""
    s = ST.leerer_store(2025, fall_id="zone_a")

    # Basis
    _b(s, "veranlagung", "einzel")
    _b(s, "bruttoarbeitslohn", 5000000)
    _b(s, "kein_gewinn", True)
    _b(s, "kein_kap", True)
    _b(s, "kein_vuv", True)
    _b(s, "kein_sonstige", True)

    # === EP (Entfernungspauschale): 4 Felder ===
    _b(s, "ep_arbeitstage", 220)                    # int, Kz=E0203503
    _b(s, "ep_entfernung_km", 30)                   # int, Kz=E0203504
    _b(s, "ep_oepnv_kosten", 5000)                   # cent, Kz=E0203611 (500€ ÖPNV)
    # ep_eigenes_kfz = True (bool, kein Kz — ignoriert)

    # === Verpflegung (§ 9 Abs. 4a): 10 Felder ===
    _b(s, "tage_24h", 5)                            # int, Kz=E0205409
    _b(s, "tage_an_abreise", 3)                     # int, Kz=E0205302
    _b(s, "tage_ueber_8h_eintaegig", 2)             # int, Kz=E0205201
    _b(s, "vpf_abwesenheit_stunden", 10)            # int, kein Kz -> nicht_deklariert
    _b(s, "vpf_fruehstuecke_gestellt_anzahl", 1)    # int, kein Kz -> nicht_deklariert
    _b(s, "vpf_mittagessen_gestellt_anzahl", 2)     # int, kein Kz -> nicht_deklariert
    _b(s, "vpf_abendessen_gestellt_anzahl", 1)      # int, kein Kz -> nicht_deklariert
    _b(s, "vpf_mahlzeiten_gezahltes_entgelt", 50000)  # cent, kein Kz -> nicht_deklariert
    _b(s, "vpf_steuerfreie_erstattung_betrag", 50000)   # cent, kein Kz -> nicht_deklariert (500€ Erstattung)
    _b(s, "vpf_monate_am_ort", 2)                   # int, kein Kz -> nicht_deklariert
    _b(s, "vpf_tage_24h_nach_drei_monaten", 1)      # int, kein Kz, 1 Tag nach Frist
    _b(s, "vpf_tage_an_abreise_nach_drei_monaten", 1)
    _b(s, "vpf_tage_ueber_8h_nach_drei_monaten", 1)

    # === Übernachtung (§ 9 Abs. 1 S. 3 Nr. 5a): 3 Felder ===
    _b(s, "uebernachtung_kosten_monat", 100000)     # cent, kein Kz -> nicht_deklariert
    _b(s, "uebernachtung_monate", 12)               # int, kein Kz -> nicht_deklariert
    _b(s, "uebernachtung_monate_bisher", 10)        # int, kein Kz -> nicht_deklariert

    # === dHf (§ 9 Abs. 1 S. 3 Nr. 5): 7 Felder, 1 Kz ===
    _b(s, "dhf_unterkunftskosten_monat", 100000)    # cent, Kz=E0207611
    _b(s, "dhf_monate", 6)                          # int, kein Kz -> nicht_deklariert
    # dhf_im_inland, dhf_beruflich_veranlasst, dhf_eigener_hausstand,
    # dhf_finanzielle_beteiligung, dhf_keine_pflicht_dienstwohnung = bool -> ignoriert

    snap, _ = ST.materialisiere(s)
    result = est_mapping.deklariere(snap, bindung)
    xml = EX.erzeuge_xml(result, vz=2025, hersteller_id=HID)

    funde = _pruefe_differential(snap, bindung, result, xml, "Zone A")
    assert not funde, (
        f"Zone-A: {len(funde)} Betragsfelder ohne Weg ins XML:\n"
        + "\n".join(f"  – {f}" for f in funde))
