"""Person-B fehlende XML-Deklaration — Struktur-Test.

Gemessen: Zusammenveranlagungs-Ehepaar mit je 50.000 EUR Lohn.
Ring rechnet 20.490 EUR ESt, Erklärung hätte 5.508 EUR (Person A ohne B).
Differenz: 14.982 EUR, weil die 10 Partner-Felder nicht ins XML kommen.

Dieser Test ist xfail (rot erwartet) und dient als Regression-Gate:
sobald der Person-B-Writer gebaut ist, fällt das xfail weg und zeigt Rückbau an.

Struktur-Test, kein Rechnungstest — prüft nur, dass ein Partner-Wert
im XML unter Person=PersonB auftaucht, nicht die Steuer selbst.
"""
from __future__ import annotations

import os
import sys

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "produkt", "import"))
sys.path.insert(0, os.path.join(ROOT, "produkt", "mapping"))
sys.path.insert(0, os.path.join(ROOT, "elster", "submission"))

import elster_xml as EX        # noqa: E402
import est_mapping            # noqa: E402

HID = "74931"  # Test-Hersteller-ID


def _dekl(**kz) -> dict:
    """Deklaration mit vollstaendig=True."""
    return {"vollstaendig": True, "deklaration": dict(kz)}


@pytest.mark.xfail(
    reason="Person B fehlt im XML-Writer: PFLICHT_DEFAULT={'Person': 'PersonA'} "
           "in elster_xml.py Z. 39. Kz werden nur für PersonA geschrieben. "
           "Gemessen: 10 Partner-Felder (PARTNER_INSTANZ) fehlen. "
           "Differenz Ehepaar je 50k EUR: 14.982 EUR ESt nicht deklariert. "
           "Zwei Felder (basis_kv_pv_partner, weitere_vorsorgeaufwendungen_partner) "
           "haben keine Kz in est_mapping — fallen raus. "
           "Person-B-Writer: ein Muster (maxOccurs=2 auf Pfad), Weg B (Person-Zuordnung in est_mapping)."
)
def test_person_b_partner_felder_im_xml():
    """Zusammenveranlagung: Partner-Felder unter Person=PersonB im XML.

    Unterschiedliche Werte für A und B, danach Blöcke zerlegen und prüfen:
    – PersonB-Block enthält Partner-Wert
    – PersonA-Block enthält Partner-Wert NICHT

    Iteriert über die 10 Partner-Felder aus est_mapping.PARTNER_INSTANZ.
    Zwei Felder (basis_kv_pv_partner, weitere_vorsorgeaufwendungen_partner)
    haben keinen Kz-Eintrag und fallen raus — bleiben Ring-Felder.
    """
    # 10 Partner-Felder mit Kz (aus PARTNER_INSTANZ in est_mapping.py Z. 116-131)
    # Person A Werte (erste Spalte) vs Person B Werte (zweite Spalte)
    partner_felder = {
        "E0200201": (5000000, 4000000),        # Bruttolohn: 50k vs 40k
        "E2000401": (100000, 80000),           # VOR AN: 1k vs 0.8k
        "E2000801": (50000, 40000),            # VOR AG: 500 vs 400
        "E2000601": (30000, 20000),            # VOR RV: 300 vs 200
        "E1900701": (200000, 150000),          # Kapital Erträge: 2k vs 1.5k
        "E1900901": (150000, 100000),          # Kapital Gewinn Aktien: 1.5k vs 1k
        "E1901301": (0, 0),                    # Kapital Verlust Aktien
        "E1901201": (0, 0),                    # Kapital Verlust Sonstiges
        "E0109708": (0, 0),                    # GdB (N/A)
        "E0109706": (0, 0),                    # Hilflos (N/A)
    }

    # Baue Deklaration mit UNTERSCHIEDLICHEN Person-A + B Werten
    kz_decl = {}
    for kz, (wert_a, wert_b) in partner_felder.items():
        if wert_a > 0:
            kz_decl[kz] = wert_a

    # Person-B Werte (die aktuell NICHT ins XML kommen) mit Suffix als separate Kz
    # (simuliert, als würden sie ins XML gehen — dient nur der Prüfung)
    for kz, (wert_a, wert_b) in partner_felder.items():
        if wert_b > 0:
            # Marker: diese Werte sollen unter PersonB sein
            kz_decl[f"__person_b_{kz}"] = wert_b

    # Minimal Person-A Basis
    kz_decl["E0100201"] = "Maier"
    kz_decl["E0100401"] = "01.01.1960"

    # Erzeuge XML
    xml = EX.erzeuge_xml(_dekl(**kz_decl), vz=2025, hersteller_id=HID)
    xml_clean = xml.replace("ns0:", "").replace("ns1:", "")

    # Zerlege XML in Person-Blöcke
    # Muster: <Person>PersonA</Person> ... <Person>PersonB</Person>
    # Finde die zwei Person-Container
    person_a_start = xml_clean.find("<Person>PersonA</Person>")
    person_b_start = xml_clean.find("<Person>PersonB</Person>")

    assert person_a_start >= 0, "Person=PersonA nicht im XML"
    assert person_b_start >= 0, (
        "Person=PersonB nicht im XML. "
        "PFLICHT_DEFAULT='PersonA' erzeugt keine zweite Person — alle Kz gehen nach A."
    )

    # Extrahiere Person-A-Block (von PersonA bis zum nächsten Person-Tag oder Ende)
    person_a_end = xml_clean.find("<Person>", person_a_start + 1)
    if person_a_end == -1:
        person_a_end = len(xml_clean)
    person_a_block = xml_clean[person_a_start:person_a_end]

    # Extrahiere Person-B-Block (von PersonB bis zum nächsten Person-Tag oder Ende)
    person_b_end = xml_clean.find("<Person>", person_b_start + 1)
    if person_b_end == -1:
        person_b_end = len(xml_clean)
    person_b_block = xml_clean[person_b_start:person_b_end]

    # Prüfe für jedes Partner-Feld: Wert im PersonB-Block, nicht in PersonA
    failures = []
    for kz, (wert_a, wert_b) in partner_felder.items():
        if wert_b > 0:
            # PersonB sollte diesen Wert haben
            if f">{wert_b}<" not in person_b_block:
                failures.append(f"{kz} Wert {wert_b} fehlt in PersonB-Block")
            # PersonA sollte diesen Wert NICHT haben (unterschiedlich)
            if f">{wert_b}<" in person_a_block:
                failures.append(f"{kz} Wert {wert_b} falsch in PersonA-Block (sollte nur in B sein)")

    assert not failures, (
        f"Person-B-Werte nicht korrekt zugeordnet:\n" +
        "\n".join(f"  – {f}" for f in failures) +
        "\nPerson-B-Writer nicht gebaut oder Werte fehlen."
    )


@pytest.mark.xfail(
    reason="Person B fehlt im XML-Writer (siehe test_person_b_partner_felder_im_xml)"
)
def test_person_b_minimalbeispiel_bruttolohn():
    """Minimalprüfung: nur Bruttolohn Person B im XML.

    Einfacher als test_person_b_partner_felder_im_xml,
    zeigt das Kernproblem deutlich.
    """
    # Nur Partner-Lohn, nichts sonst
    kz_decl = {
        "E0100201": "Maier",  # Name Person A
        "E0100401": "01.01.1960",
        "E0200201": 5000000,  # Bruttolohn Person B (50.000 EUR)
    }

    xml = EX.erzeuge_xml(_dekl(**kz_decl), vz=2025, hersteller_id=HID)
    xml_clean = xml.replace("ns0:", "").replace("ns1:", "")

    # Muss PersonB Container geben, sonst ist E0200201 falsch zugeordnet
    assert "<Person>PersonB</Person>" in xml_clean, (
        "E0200201 (Bruttolohn) sollte unter Person=PersonB stehen, "
        "nicht unter PersonA. PFLICHT_DEFAULT='PersonA' macht alle Kz zu Person A."
    )
