"""Zone D: Sonderausgaben, agB, Steuerermäßigungen, Anrechnung (21 Felder).

Prüf-Logik aus test_ring_deklaration_differential importieren — eine Wahrheit.
"""
from __future__ import annotations

import os
import sys

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
for sub in ("produkt/import", "produkt/mapping", "produkt/store", "produkt/traverser"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import elster_xml as EX        # noqa: E402
import est_mapping             # noqa: E402
import store as ST             # noqa: E402
import traverser as TR         # noqa: E402
from test_ring_deklaration_differential import (  # noqa: E402
    _b, _pruefe_differential)

HID = "74931"
TS = "2026-08-04T14:00:00+00:00"
H = {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}


@pytest.fixture(scope="module")
def bindung():
    return TR.lade_bindung()


# ----------------------------------------------------------------
# Gruppe 1: §33a Unterhalt + Berufsausbildung + Kinderbetreuung (7 Felder)
# ----------------------------------------------------------------

def test_zone_d_unterhalt_agb_keine_luecken(bindung):
    """§33a Unterhalt/Ausbildung + §10 Abs.1 Nr.7 + §10 Abs.1 Nr.5 — 7 Betragsfelder.

    p33a_* alle null-kz -> nicht_deklariert (Topf b).
    berufsausbildung_aufwendungen null-kz -> nicht_deklariert (Topf b).
    kinderbetreuung_* null-kz -> nicht_deklariert (Topf b).
    """
    s = ST.leerer_store(2025, fall_id="zone_d_unterhalt")
    _b(s, "veranlagung", "einzel")
    _b(s, "bruttoarbeitslohn", 5000000)
    _b(s, "kein_gewinn", True)
    _b(s, "kein_kap", True)
    _b(s, "kein_vuv", True)
    _b(s, "kein_sonstige", True)

    # §33a Unterhalt (4)
    _b(s, "p33a_unterhalt_aufwendungen", 1000000)      # cent, null-kz -> nicht_deklariert
    _b(s, "p33a_unterhalt_kv_pv", 50000)                # cent, null-kz -> nicht_deklariert
    _b(s, "p33a_andere_einkuenfte_bezuege", 200000)     # cent, null-kz -> nicht_deklariert
    _b(s, "p33a_ausbildung_anzahl_kinder", 1)            # int, null-kz -> nicht_deklariert

    # Berufsausbildung §10 Abs.1 Nr.7 (1)
    _b(s, "berufsausbildung_aufwendungen", 300000)       # cent, null-kz -> nicht_deklariert

    # Kinderbetreuung §10 Abs.1 Nr.5 (2)
    _b(s, "kinderbetreuungskosten", 600000)              # cent, null-kz -> nicht_deklariert
    _b(s, "kinderbetreuung_anzahl_kinder", 2)            # int, null-kz -> nicht_deklariert

    snap, _ = ST.materialisiere(s)
    result = est_mapping.deklariere(snap, bindung)
    xml = EX.erzeuge_xml(result, vz=2025, hersteller_id=HID)

    funde = _pruefe_differential(snap, bindung, result, xml, "Zone D Unterhalt/agB")
    assert not funde, (
        f"Zone-D-Unterhalt: {len(funde)} Betragsfelder ohne Weg ins XML:\n"
        + "\n".join(f"  – {f}" for f in funde))


# ----------------------------------------------------------------
# Gruppe 2: Steuerermäßigungen §35a/§35c (4 Felder)
# ----------------------------------------------------------------

def test_zone_d_steuerermaeßigungen_keine_luecken(bindung):
    """§35a Haushaltsnahe + §35c Sanierung — 4 Betragsfelder.

    hh_dienstleistungen (E0107208), hh_minijob_aufwendungen (E0104109) -> Topf a.
    p35c_* null-kz -> nicht_deklariert (Topf b).
    """
    s = ST.leerer_store(2025, fall_id="zone_d_erm")
    _b(s, "veranlagung", "einzel")
    _b(s, "bruttoarbeitslohn", 5000000)
    _b(s, "kein_gewinn", True)
    _b(s, "kein_kap", True)
    _b(s, "kein_vuv", True)
    _b(s, "kein_sonstige", True)

    # §35a (2 mit Kz)
    _b(s, "hh_dienstleistungen", 300000)                # cent, E0107208 -> 1:1 (Topf a)
    _b(s, "hh_minijob_aufwendungen", 280000)            # cent, E0104109 -> 1:1 (Topf a)

    # §35c (2 ohne Kz)
    _b(s, "p35c_sanierungsaufwendungen", 2000000)       # cent, null-kz -> nicht_deklariert
    _b(s, "p35c_energieberater_aufwendungen", 80000)    # cent, null-kz -> nicht_deklariert

    snap, _ = ST.materialisiere(s)
    result = est_mapping.deklariere(snap, bindung)
    xml = EX.erzeuge_xml(result, vz=2025, hersteller_id=HID)

    funde = _pruefe_differential(snap, bindung, result, xml, "Zone D Steuererm.")
    assert not funde, (
        f"Zone-D-Steuererm.: {len(funde)} Betragsfelder ohne Weg ins XML:\n"
        + "\n".join(f"  – {f}" for f in funde))


# ----------------------------------------------------------------
# Gruppe 3: KiSt + §36 Anrechnung + Spenden (5 Felder)
# ----------------------------------------------------------------

def test_zone_d_sonderausgaben_anrechnung_keine_luecken(bindung):
    """KiSt + §36 Anrechnung + §10b Spenden — 5 Betragsfelder.

    kist_*, p36_* null-kz -> nicht_deklariert (Topf b).
    spenden_betrag (E0108405) -> Topf a.
    """
    s = ST.leerer_store(2025, fall_id="zone_d_sa_anr")
    _b(s, "veranlagung", "einzel")
    _b(s, "bruttoarbeitslohn", 5000000)
    _b(s, "kein_gewinn", True)
    _b(s, "kein_kap", True)
    _b(s, "kein_vuv", True)
    _b(s, "kein_sonstige", True)

    # KiSt (2)
    _b(s, "kist_gezahlt", 60000)                        # cent, null-kz -> nicht_deklariert
    _b(s, "kist_erstattet", 5000)                        # cent, null-kz -> nicht_deklariert

    # §36 Anrechnung (2)
    _b(s, "p36_lohnsteuer", 800000)                      # cent, null-kz -> nicht_deklariert
    _b(s, "p36_vorauszahlungen", 200000)                 # cent, null-kz -> nicht_deklariert

    # §10b Spende (1 mit Kz)
    _b(s, "spenden_betrag", 30000)                       # cent, E0108405 -> 1:1 (Topf a)

    snap, _ = ST.materialisiere(s)
    result = est_mapping.deklariere(snap, bindung)
    xml = EX.erzeuge_xml(result, vz=2025, hersteller_id=HID)

    funde = _pruefe_differential(snap, bindung, result, xml, "Zone D SA/Anr.")
    assert not funde, (
        f"Zone-D-SA/Anr.: {len(funde)} Betragsfelder ohne Weg ins XML:\n"
        + "\n".join(f"  – {f}" for f in funde))


# ----------------------------------------------------------------
# Gruppe 4: DBA-Anrechnung + GewSt §35 + §32b (5 Felder)
# ----------------------------------------------------------------

def test_zone_d_ausland_gewst_keine_luecken(bindung):
    """§34c DBA-Anrechnung + §35 GewSt + §32b Progressionsvorbehalt — 5 Felder.

    Alle null-kz -> nicht_deklariert (Topf b).
    GewSt hat KEIN E10-Analog — erwartet Topf (b) oder (d).
    """
    s = ST.leerer_store(2025, fall_id="zone_d_ausl")
    _b(s, "veranlagung", "einzel")
    _b(s, "bruttoarbeitslohn", 5000000)
    _b(s, "kein_gewinn", True)
    _b(s, "kein_kap", True)
    _b(s, "kein_vuv", True)
    _b(s, "kein_sonstige", True)

    # DBA (2): Screening-Felder für Vollständigkeit (keine Beträge, aber notwendiger Kontext)
    _b(s, "dba_staat", "Frankreich")
    _b(s, "dba_methode", "kein_dba")
    _b(s, "dba_einkunftsart", "unselbstaendige_arbeit")
    _b(s, "dba_mehrere_staaten", False)
    _b(s, "dba_auslaendische_einkuenfte", 400000)        # cent, null-kz -> nicht_deklariert
    _b(s, "dba_gezahlte_auslaendische_steuer", 30000)    # cent, null-kz -> nicht_deklariert

    # §32b Progressionsvorbehalt (1)
    _b(s, "p32b_progressionseinkuenfte", 500000)          # cent, null-kz -> nicht_deklariert

    # GewSt §35 (2): KEIN E10-Analog, erwartet Topf (b) via nicht_deklariert
    _b(s, "gewst_hebesatz", 450)                          # int, null-kz -> nicht_deklariert
    _b(s, "gewst_messbetrag", 2024750)                    # cent, null-kz -> nicht_deklariert

    snap, _ = ST.materialisiere(s)
    result = est_mapping.deklariere(snap, bindung)
    xml = EX.erzeuge_xml(result, vz=2025, hersteller_id=HID)

    funde = _pruefe_differential(snap, bindung, result, xml, "Zone D Ausland/GewSt")
    assert not funde, (
        f"Zone-D-Ausland/GewSt: {len(funde)} Betragsfelder ohne Weg ins XML:\n"
        + "\n".join(f"  – {f}" for f in funde))


# ----------------------------------------------------------------
# Zusammenveranlagung + person_b (Auswahl Zone-D)
# ----------------------------------------------------------------

def test_zone_d_zusammenveranlagung_keine_luecken(bindung):
    """Zusammenveranlagung: Zone-D-Felder + person_b — Abdeckung für person_b.

    Testet, dass shared Zone-D-Felder auch im Zusammen-Fall sauber laufen.
    """
    s = ST.leerer_store(2025, fall_id="zone_d_zusammen")
    _b(s, "veranlagung", "zusammen")
    _b(s, "kein_gewinn", True)
    _b(s, "kein_kap", True)
    _b(s, "kein_vuv", True)
    _b(s, "kein_sonstige", True)

    # Person A
    _b(s, "bruttoarbeitslohn", 5000000)
    _b(s, "vor_an_anteil_rv", 200000)

    # Person B
    _b(s, "bruttoarbeitslohn_partner", 4000000)
    _b(s, "vor_an_anteil_rv_partner", 160000)

    # Zone-D-Felder (Auswahl über alle Gruppen)
    _b(s, "p33a_unterhalt_aufwendungen", 1000000)        # cent, null-kz -> nicht_deklariert
    _b(s, "p33a_ausbildung_anzahl_kinder", 1)             # int, null-kz -> nicht_deklariert
    _b(s, "hh_dienstleistungen", 300000)                  # cent, E0107208 -> 1:1
    _b(s, "hh_minijob_aufwendungen", 280000)              # cent, E0104109 -> 1:1
    _b(s, "kist_gezahlt", 60000)                          # cent, null-kz -> nicht_deklariert
    _b(s, "kist_erstattet", 5000)                         # cent, null-kz -> nicht_deklariert
    _b(s, "spenden_betrag", 30000)                        # cent, E0108405 -> 1:1
    _b(s, "p36_lohnsteuer", 800000)                       # cent, null-kz -> nicht_deklariert
    _b(s, "p36_vorauszahlungen", 200000)                  # cent, null-kz -> nicht_deklariert
    _b(s, "kinderbetreuungskosten", 600000)               # cent, null-kz -> nicht_deklariert
    _b(s, "kinderbetreuung_anzahl_kinder", 2)             # int, null-kz -> nicht_deklariert

    snap, _ = ST.materialisiere(s)
    result = est_mapping.deklariere(snap, bindung)
    xml = EX.erzeuge_xml(result, vz=2025, hersteller_id=HID)

    funde = _pruefe_differential(snap, bindung, result, xml, "Zone D Zusammen")
    assert not funde, (
        f"Zone-D-Zusammen: {len(funde)} Betragsfelder ohne Weg ins XML:\n"
        + "\n".join(f"  – {f}" for f in funde))