"""Ehepaar-Gegenprobe: der eigentliche Beweis des Instanz-Achsen-Umbaus.

Fall: Zusammenveranlagung, beide Personen je 50.000 € Bruttoarbeitslohn, VZ 2025.

Messung vom 2026-07-31:
  Ring rechnet:        20.490 € ESt
  Erklärung VORHER:     5.508 € ESt   (nur Person A)
  Differenz:           14.982 €

Nach dem Umbau muss die Differenz WEG sein — beide Löhne deklariert.

Kein Auswendiglernen einer Steuerzahl: der Test nagelt die STRUKTUR fest
(beide E0200201-Werte im XML, Summe = 100.000 €). Der Ring-Wert ist
transparente Dokumentation, kein Gate.
"""
from __future__ import annotations

import os
import re
import sys

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
for sub in ("produkt/import", "produkt/mapping", "produkt/store", "produkt/traverser", "golden"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import elster_xml as EX       # noqa: E402
import est_mapping            # noqa: E402
import store as ST            # noqa: E402
import traverser as TR        # noqa: E402
import runner as R            # noqa: E402

HID = "74931"
TS = "2026-08-04T14:00:00+00:00"
H = {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}

# § 9a S. 1 Nr. 1a EStG Arbeitnehmer-Pauschbetrag VZ 2025
_AN_PB_2025 = 1230


@pytest.fixture(scope="module")
def bindung():
    return TR.lade_bindung()


def _b(s, feld_id, wert, zustand="bestaetigt"):
    sig = {"signal_1": None, "signal_2": f"ok@{feld_id}"} if zustand == "bestaetigt" else {
        "signal_1": None, "signal_2": None}
    ST.append_event(store=s, feld_id=feld_id, wert=wert, zustand=zustand,
                    herkunft=H, schreiber="ui:laie", signal=sig, ts=TS)


def _extrahiere_e0200201(xml_str: str) -> list[int]:
    """Extrahiert alle E0200201-Werte aus dem XML (beide N-Container)."""
    clean = xml_str.replace("ns0:", "").replace("ns1:", "")
    return [int(m) for m in re.findall(r"<E0200201>(\d+)</E0200201>", clean)]


def test_ehepaar_gegenprobe_beide_loehne_deklariert(bindung):
    """Zusammenveranlagung je 50k: beide E0200201 (50000) im XML, Summe 100k.

    Der Test baut den Store -> deklariere -> erzeuge_xml auf. Der Ring läuft
    parallel zur Transparenz. Gate ist die Summe der deklarierten Löhne.
    """
    # ===== Store aufbauen =====
    s = ST.leerer_store(2025, fall_id="ehepaar_50k")

    # Person A
    _b(s, "bruttoarbeitslohn", 5000000)             # 50.000 EUR → E0200201
    _b(s, "vor_an_anteil_rv", 3500000)               # 35.000 EUR → E2000401
    _b(s, "vor_ag_anteil_rv", 1000000)               # 10.000 EUR → E2000801
    _b(s, "vor_rv_ausserhalb_lstb", 0)               # 0 EUR → E2000601
    _b(s, "kap_kapitalertraege", 0)
    _b(s, "kap_gewinn_aktien", 0)
    _b(s, "kap_verlust_aktien", 0)
    _b(s, "kap_verlust_sonstige", 0)

    # Person B (Partner)
    _b(s, "bruttoarbeitslohn_partner", 5000000)      # 50.000 EUR → E0200201 in person_b
    _b(s, "vor_an_anteil_rv_partner", 3500000)       # 35.000 EUR → E2000401 in person_b
    _b(s, "vor_ag_anteil_rv_partner", 1000000)       # 10.000 EUR → E2000801 in person_b
    _b(s, "vor_rv_ausserhalb_lstb_partner", 0)       # 0 EUR → E2000601 in person_b
    _b(s, "kap_kapitalertraege_partner", 0)
    _b(s, "kap_gewinn_aktien_partner", 0)
    _b(s, "kap_verlust_aktien_partner", 0)
    _b(s, "kap_verlust_sonstige_partner", 0)

    # Pflichtkegel
    _b(s, "veranlagung", "zusammen")
    _b(s, "kein_gewinn", True)
    _b(s, "kein_kap", True)
    _b(s, "kein_vuv", True)
    _b(s, "kein_sonstige", True)

    # ===== Ring-Lauf (Transparenz) =====
    # Einkünfte nichtselbstständig: 50.000 − 1.230 (AN-PB) = 48.770 pro Person
    # Zusammenveranlagung: 48.770 + 48.770 = 97.540
    eink_ns = 2 * (50000 - _AN_PB_2025)
    ring_sachverhalt = {
        "veranlagungszeitraum": 2025,
        "einkuenfte_nichtselbststaendig": eink_ns,
        "veranlagung": "zusammen",
    }
    kette = R.catala_gesamt_kette(ring_sachverhalt)
    ring_est = kette["festzusetzende_est"]

    # ===== Deklaration → XML =====
    snap, _ = ST.materialisiere(s)
    result = est_mapping.deklariere(snap, bindung)
    xml = EX.erzeuge_xml(result, vz=2025, hersteller_id=HID)

    # ===== Extraktion E0200201 (Bruttolohn) =====
    werte = _extrahiere_e0200201(xml)

    print(f"\n  Ring ESt (gesamt): {ring_est} €")
    print(f"  Ring GdE:          {kette['gesamtbetrag_der_einkuenfte']} €")
    print(f"  Ring zvE:          {kette['zu_versteuerndes_einkommen']} €")
    print(f"  E0200201 Werte:    {werte}")
    print(f"  Summe Löhne:       {sum(werte)} €")

    # ===== Gate: beide 50.000 Löhne deklariert =====
    assert len(werte) == 2, (
        f"Erwarte 2× E0200201 (Person A + Person B), gefunden {len(werte)}: {werte}. "
        "Person B wird nicht deklariert → Instanz-Achse defekt."
    )
    assert werte == [50000, 50000], (
        f"Beide Löhne müssen 50.000 sein, habe {werte}. "
        "Werte vertauscht oder falsch indiziert."
    )
    assert sum(werte) == 100000, (
        f"Lohnsumme muss 100.000 sein, habe {sum(werte)}. "
        "Person-B-Lohn fehlt oder doppelt."
    )

    # ===== Dokumentation (kein Gate): Ring-ESt =====
    print(f"\n  ✓ BEIDE 50k-Löhne deklariert. Lohnsumme {sum(werte)} €.")
    print(f"  ✓ Ring rechnet {ring_est} € ESt (zusammen, je 50k).")

    # Prüfe auch: Deklaration enthält genau die Einträge
    assert result["deklaration"].get("E0200201") == 50000, "Person A E0200201 in deklaration"
    assert result["person_b"].get("E0200201") == 50000, "Person B E0200201 in person_b"
    assert len(result["person_b"]) > 0, "person_b-Bucket leer → Person B nicht deklariert"


def test_ehepaar_gegenprobe_e0200201_haengt_nicht_nur_in_deklaration(bindung):
    """Regression: E0200201 darf NICHT doppelt in deklaration sein (heisst: Person B
    in Person A-Container geschrieben). Stattdessen: einer in deklaration, einer in person_b."""
    s = ST.leerer_store(2025, fall_id="ehepaar_nur_einmal")
    _b(s, "bruttoarbeitslohn", 5000000)
    _b(s, "bruttoarbeitslohn_partner", 4000000)
    _b(s, "veranlagung", "zusammen")
    _b(s, "kein_gewinn", True)
    _b(s, "kein_kap", True)
    _b(s, "kein_vuv", True)
    _b(s, "kein_sonstige", True)

    snap, _ = ST.materialisiere(s)
    result = est_mapping.deklariere(snap, bindung)
    xml = EX.erzeuge_xml(result, vz=2025, hersteller_id=HID)
    werte = _extrahiere_e0200201(xml)

    # Zwei <E0200201>: 50000 (A, in deklaration) + 40000 (B, in person_b)
    assert werte == [50000, 40000], f"Erwarte [50000, 40000], habe {werte}"

    # Person B Wert (40000) DARF NICHT in deklaration sein — nur in person_b
    assert result["deklaration"]["E0200201"] == 50000
    assert result["person_b"]["E0200201"] == 40000