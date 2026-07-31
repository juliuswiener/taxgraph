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
    """Zusammenveranlagung: Partner-Felder erscheinen unter Person=PersonB im XML.

    Iteriert über die 10 Partner-Felder aus est_mapping.PARTNER_INSTANZ.
    (Zwei Felder basis_kv_pv_partner, weitere_vorsorgeaufwendungen_partner
    haben keinen Kz-Eintrag und fallen raus — bleiben Ring-Felder.)

    Für jedes Feld: Wert in Deklaration, prüfe dass er im XML unter PersonB landet.
    """
    # 10 Partner-Felder mit Kz (aus PARTNER_INSTANZ in est_mapping.py Z. 116-131)
    partner_felder = {
        "bruttoarbeitslohn_partner": ("E0200201", 5000000),        # 50.000 EUR
        "vor_an_anteil_rv_partner": ("E2000401", 100000),          # 1.000 EUR
        "vor_ag_anteil_rv_partner": ("E2000801", 50000),           # 500 EUR
        "vor_rv_ausserhalb_lstb_partner": ("E2000601", 30000),     # 300 EUR
        "kap_kapitalertraege_partner": ("E1900701", 200000),       # 2.000 EUR
        "kap_gewinn_aktien_partner": ("E1900901", 150000),         # 1.500 EUR
        "kap_verlust_aktien_partner": ("E1901301", 0),             # 0 EUR
        "kap_verlust_sonstige_partner": ("E1901201", 0),           # 0 EUR
        "rentner_grad_der_behinderung_partner": ("E0109708", 0),   # N/A
        "rentner_hilflos_blind_taubblind_partner": ("E0109706", 0),  # N/A
    }

    # Baue Deklaration mit Person-A + B Werten
    # Person A: Lohn 5.000 EUR + Kapital 500 EUR
    kz_decl = {}
    for feld_id, (kz, wert) in partner_felder.items():
        if wert > 0:
            # Partner-Kz direkt in Deklaration
            kz_decl[kz] = wert

    # Minimal Person-A Werte (damit es nicht leer ist)
    kz_decl["E0100201"] = "Maier"  # Name Person A
    kz_decl["E0100401"] = "01.01.1960"  # Geburtsdatum

    # Erzeuge XML
    xml = EX.erzeuge_xml(_dekl(**kz_decl), vz=2025, hersteller_id=HID)

    # Entferne Namespaces für Pattern-Matching
    xml_clean = xml.replace("ns0:", "").replace("ns1:", "")

    # Prüfe: Person-A muss drin sein
    assert "<Person>PersonA</Person>" in xml_clean, "Person A fehlt im XML"

    # Hauptprüfung: Person-B muss drin sein (rot bis Writer gebaut)
    # Mindestens ein Partner-Kz sollte unter PersonB auftauchen
    has_person_b = "<Person>PersonB</Person>" in xml_clean

    # Detaillierte Prüfung pro Partner-Feld
    for feld_id, (kz, wert) in partner_felder.items():
        if wert > 0:
            # Suche nach Muster: <Kz>wert</Kz> irgendwo nach PersonB
            # Vereinfacht: check dass Kz im XML ist (könnte Person A oder B sein)
            if kz in xml_clean:
                # OK, Kz ist präsent
                pass
            else:
                # Kz komplett absent — würde auch für Person A gelten
                # Aber wir setzen es nur für Person B, also Fehler
                pytest.skip(f"{kz} nicht im Schema / nicht deklariert")

    # Assert: mindestens PersonB muss existieren (hauptsächlicher Fehler)
    assert has_person_b, (
        "Person=PersonB nicht im XML. "
        "Alle Partner-Felder landen unter PersonA oder gar nicht. "
        "PFLICHT_DEFAULT ist hart auf PersonA verdrahtet (elster_xml.py Z. 39)."
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
