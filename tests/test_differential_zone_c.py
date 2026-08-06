"""Zone C — Rentner, Versorgung, Person-B-Partnerfelder (18 Felder).
Differential-Invariante: jedes bestätigte Betragsfeld hat Weg ins XML (Kz) oder Grund in
nicht_deklariert/dokumentiert/Ausschluss. Vierter Topf gibt es nicht.

Abdeckung:
  rentner_* (6): alter_bei_rentenbeginn_partner, grad_der_behinderung_partner,
    jahresrente_partner, renten_beginn_jahr_partner, rentenfreibetrag_partner,
    veraeusserungsgewinn
  versorgung_* (4): alter_bei_beginn, beginn_jahr, bemessungsgrundlage, jahresrente
  p22_nr3_einkuenfte
  geburtsjahr, geburtsjahr_partner
  basis_kv_pv_partner, weitere_vorsorgeaufwendungen_partner
  realsplitting_unterhaltsleistungen, realsplitting_empfaenger_kv_pv
  fam_monate_ohne_voraussetzung

Zusammenveranlagung nötig für _partner-Felder (Person-B-Achse, Klasse g).
"""
from __future__ import annotations

import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/import", "produkt/mapping", "produkt/store", "produkt/traverser"):
    sys.path.insert(0, os.path.join(ROOT, sub))

from test_ring_deklaration_differential import _pruefe_differential, _b

import elster_xml as EX        # noqa: E402
import est_mapping             # noqa: E402
import store as ST             # noqa: E402
import traverser as TR         # noqa: E402


@pytest.fixture(scope="module")
def bindung():
    return TR.lade_bindung()


def test_differential_zone_c_rentner_ehepaar(bindung):
    """Rentner-Ehepaar Zusammenveranlagung: deckt 14 Felder.

    Person A/B gesetzliche Rente aa-Erstjahr (§22), Behinderung (§33b),
    Versorgungsbezüge, Realsplitting, Geburtsjahre, Partner-Vorsorge,
    fam_monate_ohne_voraussetzung.

    Verteilung:
    - rentner_grad_der_behinderung_partner → PARTNER_INSTANZ → person_b (E0109708)
    - rentner_jahresrente_partner → PARTNER_VERZWEIGUNG → person_b (E1800301)
    - rentner_renten_beginn_jahr_partner → PARTNER_VERZWEIGUNG → person_b (E1800501)
    - Rest (11) → nicht_deklariert (kein elster_kz)
    """
    s = ST.leerer_store(2025, fall_id="diff_zc_rentner")

    # === Pflichtkegel ===
    _b(s, "veranlagung", "zusammen")
    _b(s, "kein_gewinn", True)
    _b(s, "kein_kap", True)
    _b(s, "kein_vuv", True)
    _b(s, "kein_sonstige", False)     # Renten = sonstige Einkünfte

    # === Person A: Basis-Vorsorge ===
    _b(s, "vor_an_anteil_rv", 200000)
    _b(s, "vor_ag_anteil_rv", 150000)
    _b(s, "vor_rv_ausserhalb_lstb", 100000)
    _b(s, "basis_kv", 450000)
    _b(s, "basis_pv", 0)
    _b(s, "versicherungsart", "gesetzlich_freiwillig")
    _b(s, "vorsorge_arbeitslosenversicherung", 0)
    _b(s, "vorsorge_erwerbsunfaehigkeit", 0)
    _b(s, "vorsorge_unfall_haftpflicht", 0)
    _b(s, "vorsorge_rv_alt_mit_ueberschuss", 0)
    _b(s, "vorsorge_rv_alt_ohne_ueberschuss", 0)
    _b(s, "mit_anspruch_auf_zuschuss", False)

    # === Person A: Rentner (§22, aa-Erstjahr) ===
    _b(s, "rentner_renten_art", "gesetzliche_rente")
    _b(s, "rentner_jahresrente", 20000000)         # → E1800301 (aa)
    _b(s, "rentner_renten_beginn_jahr", 2025)       # → E1800501 (aa)
    _b(s, "rentner_alter_bei_rentenbeginn", 65)
    _b(s, "rentner_rentenfreibetrag", 0)
    _b(s, "rentner_grad_der_behinderung", 50)       # → E0109708

    # === Person B: Rentner-Partner (§22, aa-Erstjahr) ===
    _b(s, "rentner_renten_art_partner", "gesetzliche_rente")
    _b(s, "rentner_jahresrente_partner", 15000000)  # → E1800301 via PARTNER_VERZWEIGUNG → person_b
    _b(s, "rentner_renten_beginn_jahr_partner", 2025)  # → E1800501 via PARTNER_VERZWEIGUNG → person_b
    _b(s, "rentner_alter_bei_rentenbeginn_partner", 67)  # → nicht_deklariert
    _b(s, "rentner_rentenfreibetrag_partner", 500000)   # 5.000 € → nicht_deklariert
    _b(s, "rentner_grad_der_behinderung_partner", 60)  # → E0109708 via PARTNER_INSTANZ → person_b

    # === Person A: Versorgungsbezüge ===
    _b(s, "versorgung_alter_bei_beginn", 65)          # → nicht_deklariert
    _b(s, "versorgung_beginn_jahr", 2025)              # → nicht_deklariert
    _b(s, "versorgung_bemessungsgrundlage", 1200000)   # → nicht_deklariert
    _b(s, "versorgung_jahresrente", 1200000)           # → nicht_deklariert

    # === Realsplitting ===
    _b(s, "realsplitting_unterhaltsleistungen", 2000000)   # → nicht_deklariert
    _b(s, "realsplitting_empfaenger_kv_pv", 200000)     # 2.000 € → nicht_deklariert

    # === Familien-Feld ===
    _b(s, "fam_monate_ohne_voraussetzung", 3)          # → nicht_deklariert

    # === Geburtsjahre (Kohorten-Schlüssel, keine Betragsfelder im XML) ===
    _b(s, "geburtsjahr", 1960)                         # → nicht_deklariert
    _b(s, "geburtsjahr_partner", 1958)                 # → nicht_deklariert

    # === Person B: Vorsorge (Klasse g null-Kz-MVP) ===
    _b(s, "basis_kv_partner", 350000)
    _b(s, "basis_pv_partner", 0)
    _b(s, "versicherungsart_partner", "gesetzlich_freiwillig")
    _b(s, "vorsorge_arbeitslosenversicherung_partner", 50000)
    _b(s, "vorsorge_erwerbsunfaehigkeit_partner", 0)
    _b(s, "vorsorge_unfall_haftpflicht_partner", 0)
    _b(s, "vorsorge_rv_alt_mit_ueberschuss_partner", 0)
    _b(s, "vorsorge_rv_alt_ohne_ueberschuss_partner", 0)  # → nicht_deklariert

    # === Person A: Bruttoarbeitslohn (damit Einkünfte ≠ 0) ===
    _b(s, "bruttoarbeitslohn", 5000000)                # → E0200201

    # === Person B: Bruttoarbeitslohn (Person-Multiplikation, Klasse g) ===
    _b(s, "bruttoarbeitslohn_partner", 4000000)        # → E0200201 via PARTNER_INSTANZ → person_b
    _b(s, "vor_an_anteil_rv_partner", 160000)
    _b(s, "vor_ag_anteil_rv_partner", 120000)
    _b(s, "vor_rv_ausserhalb_lstb_partner", 80000)

    snap, _ = ST.materialisiere(s)
    result = est_mapping.deklariere(snap, bindung)
    xml = EX.erzeuge_xml(result, vz=2025, hersteller_id="74931")

    funde = _pruefe_differential(snap, bindung, result, xml, "ZoneC-Rentner")
    assert not funde, (
        f"Zone C Rentner-Ehepaar: {len(funde)} Betragsfelder ohne Weg ins XML:\n"
        + "\n".join(f"  – {f}" for f in funde))


def test_differential_zone_c_veraeusserung(bindung):
    """Rentner-Veräußerungsgewinn (§16 Abs.4): Betriebsveräußerung gewerblich →
    E0801301 (Anlage G) via VERZWEIGUNG.

    Erfasst: rentner_veraeusserungsgewinn.
    """
    s = ST.leerer_store(2025, fall_id="diff_zc_veraeuss")
    _b(s, "veranlagung", "einzel")
    _b(s, "kein_gewinn", False)
    _b(s, "kein_kap", True)
    _b(s, "kein_vuv", True)
    _b(s, "kein_sonstige", True)
    _b(s, "bruttoarbeitslohn", 5000000)
    _b(s, "vor_an_anteil_rv", 200000)
    _b(s, "vor_ag_anteil_rv", 150000)
    _b(s, "vor_rv_ausserhalb_lstb", 100000)
    _b(s, "kap_kapitalertraege", 0)

    # Veräußerungsgewinn mit Betriebsart = gewerbe → E0801301
    _b(s, "rentner_veraeusserungs_betriebsart", "gewerbe")
    _b(s, "rentner_veraeusserungsgewinn", 15000000)  # 150.000 € → E0801301 via VERZWEIGUNG

    snap, _ = ST.materialisiere(s)
    result = est_mapping.deklariere(snap, bindung)
    xml = EX.erzeuge_xml(result, vz=2025, hersteller_id="74931")

    funde = _pruefe_differential(snap, bindung, result, xml, "ZoneC-Veraeuss")
    assert not funde, (
        f"Veräußerungsgewinn: {len(funde)} Betragsfelder ohne Weg ins XML:\n"
        + "\n".join(f"  – {f}" for f in funde))


def test_differential_zone_c_p22_nr3(bindung):
    """§22 Nr.3 sonstige Einkünfte: kein Kz-Mapping (fail-closed nicht_deklariert).

    Erfasst: p22_nr3_einkuenfte.
    """
    s = ST.leerer_store(2025, fall_id="diff_zc_p22nr3")
    _b(s, "veranlagung", "einzel")
    _b(s, "kein_gewinn", True)
    _b(s, "kein_kap", True)
    _b(s, "kein_vuv", True)
    _b(s, "kein_sonstige", False)   # p22 = sonstige Einkünfte
    _b(s, "bruttoarbeitslohn", 5000000)
    _b(s, "vor_an_anteil_rv", 200000)
    _b(s, "vor_ag_anteil_rv", 150000)
    _b(s, "vor_rv_ausserhalb_lstb", 100000)

    # §22 Nr.3
    _b(s, "p22_nr3_einkuenfte", 10000000)  # → nicht_deklariert (kein Kz)

    # Rentner-Basis (sonstige Einkünfte §22 Nr.1, damit kein leerer sonstige-Pfad)
    _b(s, "rentner_renten_art", "gesetzliche_rente")
    _b(s, "rentner_jahresrente", 20000000)
    _b(s, "rentner_renten_beginn_jahr", 2025)
    _b(s, "rentner_alter_bei_rentenbeginn", 65)
    _b(s, "rentner_rentenfreibetrag", 0)

    snap, _ = ST.materialisiere(s)
    result = est_mapping.deklariere(snap, bindung)
    xml = EX.erzeuge_xml(result, vz=2025, hersteller_id="74931")

    funde = _pruefe_differential(snap, bindung, result, xml, "ZoneC-p22nr3")
    assert not funde, (
        f"§22 Nr.3: {len(funde)} Betragsfelder ohne Weg ins XML:\n"
        + "\n".join(f"  – {f}" for f in funde))