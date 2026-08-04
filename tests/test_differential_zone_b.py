"""Zone B: Kapital §20 sonstige, V+V §21 WK, PV §3 Nr.72, §23 Veräußerung,
Mitunternehmer §15, AfA/Arbeitsmittel, GWG, betriebseinnahmen/sonstige_betriebsausgaben,
verlustvortrag_bestand.

Importiert _pruefe_differential und _b aus der Mutterschiff-Datei — EINE Wahrheit.
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

from test_ring_deklaration_differential import _pruefe_differential, _b, _alle_kz_im_xml  # noqa: E402

HID = "74931"
TS = "2026-08-05T00:00:00+00:00"
H = {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}


def _pruefe(snap, bindung, result, xml, label):
    """Wrapper: prueft die Invariante UND stellt sicher, dass geprueft > 0.
    _pruefe_differential wird nie veraendert — der Count wird hier nachgeprueft."""
    funde = _pruefe_differential(snap, bindung, result, xml, label)
    geprueft = 0
    for feld_id, sfeld in snap.items():
        b = bindung.get(feld_id)
        if b is None:
            continue
        if sfeld.get("zustand") != "bestaetigt":
            continue
        if b.get("typ") not in ("cent", "int", "euro"):
            continue
        if not sfeld.get("wert"):
            continue
        geprueft += 1
    assert geprueft > 0, (
        f"LEERER TEST: '{label}' erreicht 0 Betragsfelder — Szenario ist wirkungslos")
    return funde


@pytest.fixture(scope="module")
def bindung():
    return TR.lade_bindung()


# ---------------------------------------------------------------- Szenario 1: Vermieter + PV

def test_zone_b_vermieter_pv(bindung):
    """Vermieter §21 + Photovoltaik §3 Nr.72 — 4 Ziel-Felder, alle nicht_deklariert."""
    s = ST.leerer_store(2025, fall_id="z_b_vm_pv")
    _b(s, "veranlagung", "einzel")
    _b(s, "kein_gewinn", True)
    _b(s, "kein_kap", True)
    _b(s, "kein_vuv", False)
    _b(s, "kein_sonstige", True)

    # V+V §21 — bekannte Felder (schon in Mutterschiff getestet)
    _b(s, "vv_einnahmen", 1200000)          # E0700201 (1:1)
    _b(s, "vv_gebaeude_afa", 300000)        # Aggregat
    _b(s, "vv_schuldzinsen", 200000)        # Aggregat
    _b(s, "vv_erhaltungsaufwand", 100000)   # Aggregat
    _b(s, "vv_sonstige_wk", 50000)          # Aggregat
    _b(s, "vv_entgelt_quote_prozent", 100)  # int, nicht deklariert

    # === ZIEL-FELDER ===
    _b(s, "vv_werbungskosten", 850000)       # elster_kz=null → nicht_deklariert
    _b(s, "pv_einnahmen", 120000)            # elster_kz=null → nicht_deklariert
    _b(s, "pv_bruttoleistung_kwp", 12)       # int, elster_kz=null → nicht_deklariert
    _b(s, "pv_anzahl_einheiten", 1)          # int, elster_kz=null → nicht_deklariert

    snap, _ = ST.materialisiere(s)
    result = est_mapping.deklariere(snap, bindung)
    xml = EX.erzeuge_xml(result, vz=2025, hersteller_id=HID)

    funde = _pruefe(snap, bindung, result, xml, "ZoneB-VermieterPV")
    assert not funde, (
        f"ZoneB Vermieter+PV: {len(funde)} Betragsfelder ohne Weg ins XML:\n"
        + "\n".join(f"  – {f}" for f in funde))


# ---------------------------------------------------------------- Szenario 2: Kapital + Mitunternehmer + AfA/Betriebseinnahmen

def test_zone_b_kapital_mitunternehmer_afa(bindung):
    """Kapital §20 sonstige + Mitunternehmer §15 + AfA + betriebseinnahmen + verlustvortrag.
    Zusammenveranlagung — auch kap_gewinn_sonstige_partner."""
    s = ST.leerer_store(2025, fall_id="z_b_kap_mu_afa")
    _b(s, "veranlagung", "zusammen")
    _b(s, "kein_gewinn", False)
    _b(s, "kein_kap", False)
    _b(s, "kein_vuv", True)
    _b(s, "kein_sonstige", True)

    # Context: reale Kz für nicht-leeres XML
    _b(s, "bruttoarbeitslohn", 5000000)        # E0200201
    _b(s, "vor_an_anteil_rv", 200000)          # E2000401
    _b(s, "vor_ag_anteil_rv", 150000)          # E2000801
    _b(s, "vor_rv_ausserhalb_lstb", 100000)    # E2000601
    _b(s, "kap_kapitalertraege", 500000)       # E1900701
    _b(s, "kap_gewinn_aktien", 0)
    _b(s, "kap_verlust_aktien", 0)
    _b(s, "kap_verlust_sonstige", 0)

    # Gewinn-Kontext (einkuenfte_gewinn braucht gewinn_betriebsart)
    _b(s, "einkuenfte_gewinn", 5000000)         # VERZWEIGUNG mit leerem kz-dict → nicht_deklariert
    _b(s, "gewinn_betriebsart", "gewerbe")      # Art-Weiche

    # === ZIEL-FELDER ===
    _b(s, "kap_gewinn_sonstige", 250000)             # elster_kz=null → nicht_deklariert
    _b(s, "kap_gewinn_sonstige_partner", 150000)     # elster_kz=null → nicht_deklariert
    _b(s, "gewinnanteil", 4000000)                   # elster_kz=null → nicht_deklariert
    _b(s, "verguetung_taetigkeit", 200000)           # elster_kz=null → nicht_deklariert
    _b(s, "verguetung_darlehen", 100000)             # elster_kz=null → nicht_deklariert
    _b(s, "verguetung_ueberlassung", 50000)          # elster_kz=null → nicht_deklariert
    _b(s, "afa_jahresbetrag", 500000)                # elster_kz=null → nicht_deklariert
    _b(s, "betriebseinnahmen", 8000000)              # elster_kz=null → nicht_deklariert
    _b(s, "verlustvortrag_bestand", 6000000)         # elster_kz=null → nicht_deklariert

    # Partner-Kontext (Zusammenveranlagung: PARTNER_INSTANZ)
    _b(s, "bruttoarbeitslohn_partner", 4000000)
    _b(s, "vor_an_anteil_rv_partner", 160000)
    _b(s, "vor_ag_anteil_rv_partner", 120000)
    _b(s, "vor_rv_ausserhalb_lstb_partner", 80000)
    _b(s, "kap_kapitalertraege_partner", 300000)
    _b(s, "kap_gewinn_aktien_partner", 0)
    _b(s, "kap_verlust_aktien_partner", 0)
    _b(s, "kap_verlust_sonstige_partner", 0)

    snap, _ = ST.materialisiere(s)
    result = est_mapping.deklariere(snap, bindung)
    xml = EX.erzeuge_xml(result, vz=2025, hersteller_id=HID)

    funde = _pruefe(snap, bindung, result, xml, "ZoneB-KapMUAfA")
    assert not funde, (
        f"ZoneB Kapital+Mitunternehmer+AfA: {len(funde)} Betragsfelder ohne Weg ins XML:\n"
        + "\n".join(f"  – {f}" for f in funde))


# ---------------------------------------------------------------- Szenario 3: Arbeitsmittel + §23

def test_zone_b_am_p23(bindung):
    """Arbeitsmittel (am_* + nutzungsdauer) + §23 Veräußerung (instanz_gruppe).
    Alle elster_kz=null → nicht_deklariert."""
    s = ST.leerer_store(2025, fall_id="z_b_am_p23")
    _b(s, "veranlagung", "einzel")
    _b(s, "kein_gewinn", True)
    _b(s, "kein_kap", True)
    _b(s, "kein_vuv", True)
    _b(s, "kein_sonstige", True)

    # Context: reale Kz für nicht-leeres XML
    _b(s, "bruttoarbeitslohn", 5000000)       # E0200201
    _b(s, "vor_an_anteil_rv", 200000)         # E2000401
    _b(s, "vor_ag_anteil_rv", 150000)         # E2000801
    _b(s, "vor_rv_ausserhalb_lstb", 100000)   # E2000601

    # === ZIEL-FELDER: Arbeitsmittel/AfA ===
    _b(s, "am_anschaffungskosten", 120000)              # cent, elster_kz=null → nicht_deklariert
    _b(s, "am_anschaffung_monat", 10)                   # int, elster_kz=null → nicht_deklariert
    _b(s, "arbeitsmittel_nutzungsdauer", 3)              # int, elster_kz=null → nicht_deklariert

    # === ZIEL-FELDER: §23 Veräußerung (instanz_gruppe) ===
    _b(s, "p23_veraeusserungspreis", 20000000)           # cent, elster_kz=null → nicht_deklariert
    _b(s, "p23_anschaffung_herstellungskosten", 15000000)  # cent, elster_kz=null → nicht_deklariert
    _b(s, "p23_werbungskosten", 500000)                  # cent, elster_kz=null → nicht_deklariert

    snap, _ = ST.materialisiere(s)
    result = est_mapping.deklariere(snap, bindung)
    xml = EX.erzeuge_xml(result, vz=2025, hersteller_id=HID)

    funde = _pruefe(snap, bindung, result, xml, "ZoneB-AM-P23")
    assert not funde, (
        f"ZoneB AM+P23: {len(funde)} Betragsfelder ohne Weg ins XML:\n"
        + "\n".join(f"  – {f}" for f in funde))


# ---------------------------------------------------------------- Szenario 4: GWG (Topf d)

def test_zone_b_gwg_topfd(bindung):
    """gwg_anschaffungskosten_netto = E6002301 (E77-Datenart), in E10_AUSSCHLUSS_DATENART.
    Instanz-Pfad (base__2 -> anlage_instanzen) -> E10_AUSSCHLUSS_DATENART -> Topf (d), kein
    XmlFehler, kein stilles Weglassen.

    Basis-Feld (base=Instanz 1) crasht als bestehendes Verhalten (test_person_b_xml_luecke
    test_e10_ausschluss_gwg_dokumentiert Zeile 413 dokumentiert den Crash) — Instanz 2..N
    heben den Wert in den anlage_instanzen-Bucket, wo der Ausschluss greift.
    _pruefe_differential zaehlt __-Felder nicht (bindung-lookup per exakter feld_id) —
    hier geht es um Topf-d-Verhalten, nicht um einen Differential-Count.
    """
    s = ST.leerer_store(2025, fall_id="z_b_gwg_topfd")
    _b(s, "veranlagung", "einzel")
    _b(s, "kein_gewinn", True)
    _b(s, "kein_kap", True)
    _b(s, "kein_vuv", True)
    _b(s, "kein_sonstige", True)
    _b(s, "bruttoarbeitslohn", 5000000)
    _b(s, "vor_an_anteil_rv", 200000)
    _b(s, "vor_ag_anteil_rv", 150000)
    _b(s, "vor_rv_ausserhalb_lstb", 100000)
    _b(s, "gwg_anschaffungskosten_netto__2", 60000)  # Instanz 2 -> anlage_instanzen -> Topf d

    snap, _ = ST.materialisiere(s)
    result = est_mapping.deklariere(snap, bindung)
    xml = EX.erzeuge_xml(result, vz=2025, hersteller_id=HID)

    # E6002301 darf NICHT im E10-XML landen (E77-Datenart, Topf d)
    assert "E6002301" not in _alle_kz_im_xml(xml), (
        "E6002301 darf nicht im E10-XML stehen (E77-Datenart, dokumentierter Ausschluss)")
    funde = _pruefe(snap, bindung, result, xml, "ZoneB-GWG-TopfD")
    assert not funde, (
        f"ZoneB GWG: {len(funde)} Betragsfelder ohne Weg ins XML:\n"
        + "\n".join(f"  – {f}" for f in funde))


# ---------------------------------------------------------------- Szenario 6: sonstige_betriebsausgaben (FUND)

def test_zone_b_fund_sonstige_ba(bindung):
    """FUND: sonstige_betriebsausgaben (E6004901) — echtes Betragsfeld mit elster_kz.
    Bestätigt mit Wert != 0 -> est_mapping-Klasse-1 -> deklaration[E6004901] -> XmlFehler
    (kein E10-Pfad, fehlt in E10_AUSSCHLUSS_DATENART). ROT = der Fund.

    Loesung: E6004901 in E10_AUSSCHLUSS_DATENART aufnehmen ODER den Writer-
    deklaration-Pfad um den Ausschluss-Check erweitern (wie gwg E6002301).
    """
    s = ST.leerer_store(2025, fall_id="z_b_so_ba")
    _b(s, "veranlagung", "einzel")
    _b(s, "kein_gewinn", True)
    _b(s, "kein_kap", True)
    _b(s, "kein_vuv", True)
    _b(s, "kein_sonstige", True)
    _b(s, "bruttoarbeitslohn", 5000000)
    _b(s, "vor_an_anteil_rv", 200000)
    _b(s, "vor_ag_anteil_rv", 150000)
    _b(s, "vor_rv_ausserhalb_lstb", 100000)
    _b(s, "sonstige_betriebsausgaben", 3000000)  # E6004901 -> deklaration, crasht

    snap, _ = ST.materialisiere(s)
    result = est_mapping.deklariere(snap, bindung)
    xml = EX.erzeuge_xml(result, vz=2025, hersteller_id=HID)
    funde = _pruefe(snap, bindung, result, xml, "ZoneB-SonstigeBA")
    assert not funde, (
        f"ZoneB sonstige_betriebsausgaben: {len(funde)} Betragsfelder ohne Weg ins XML:\n"
        + "\n".join(f"  – {f}" for f in funde))