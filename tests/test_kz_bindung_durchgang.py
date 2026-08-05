"""Kz-Bindungen: Wert kommt WIRKLICH im XML an, an der richtigen Schema-Stelle.

Wächst mit jedem gebundenen Block (2026-08-05 ff.). Ergänzt das
Ring↔Deklaration-Differential: das prüft, ob ein Feld irgendwo verbucht ist,
dieser Test prüft die konkrete Schema-Stelle und den konkreten Wert.

Warum beides nötig ist: ein Kz kann in `deklaration` stehen (Differential grün) und
trotzdem im falschen Container landen oder in der Cent→Euro-Wandlung kippen. Genau
diese Naht war die Person-B-Lücke — Ring ODER Writer geprüft, nie die Übergabe.

Zusätzlich XSD-Validierung gegen `elster11_E10_<vz>_extern.xsd`: xs:sequence ist
ordnungsempfindlich, ein Kz an der falschen Stelle fällt nur dort auf.
"""
from __future__ import annotations

import os
import sys

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
for sub in ("produkt/import", "produkt/mapping", "produkt/traverser", "elster/submission"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import elster_xml as EX        # noqa: E402
import est_mapping             # noqa: E402
import traverser as TR         # noqa: E402

HID = "74931"


@pytest.fixture(scope="module")
def bindung():
    return TR.lade_bindung()


def _xml(felder: dict, bindung: dict) -> str:
    snap = {fid: {"wert": w, "zustand": "bestaetigt"} for fid, w in felder.items()}
    return EX.erzeuge_xml(est_mapping.deklariere(snap, bindung), vz=2025, hersteller_id=HID)


def _pfad_im_xml(xml: str, pfad: tuple[str, ...], wert: str) -> bool:
    """Steht `wert` unter genau diesem Element-Pfad? Namespace-Präfixe werden entfernt,
    damit der Test nicht an der ns0/ns1-Vergabe von ElementTree hängt (spröde)."""
    import re
    flach = re.sub(r"<(/?)[a-zA-Z0-9]+:", r"<\1", xml)
    rest = flach
    for name in pfad:
        i = rest.find(f"<{name}>")
        if i < 0:
            return False
        rest = rest[i:]
    return rest.lstrip().startswith(f"<{pfad[-1]}>{wert}<")


# ---------------------------------------------------------------- Block 1: KiSt § 10 Abs. 1 Nr. 4

def test_kist_kommt_im_xml_an(bindung):
    """kist_gezahlt -> E0107601, kist_erstattet -> E0107602, je an ihrer Schema-Stelle.

    Unterschiedliche Werte, damit eine Verwechslung der beiden Kz auffiele.
    600 EUR gezahlt / 50 EUR erstattet, Eingabe in Cent (Bindung typ=cent).
    """
    xml = _xml({"kist_gezahlt": 60000, "kist_erstattet": 5000}, bindung)

    assert _pfad_im_xml(xml, ("SA", "KiSt", "Gezahlt", "Sum", "E0107601"), "600"), (
        "E0107601 nicht unter SA/KiSt/Gezahlt/Sum mit Wert 600:\n" + xml)
    assert _pfad_im_xml(xml, ("SA", "KiSt", "Erstattet", "E0107602"), "50"), (
        "E0107602 nicht unter SA/KiSt/Erstattet mit Wert 50:\n" + xml)

    # Kein Übersprechen: der Gezahlt-Wert darf nicht im Erstattet-Zweig auftauchen
    erstattet = xml[xml.find("Erstattet"):]
    assert ">600<" not in erstattet, "gezahlter Betrag steht im Erstattet-Zweig"


# ---------------------------------------------------------------- Block 2

def test_block2_kommt_im_xml_an(bindung):
    """Berufsausbildung, § 22 Nr. 3, GewSt-Messbetrag + Hebesatz an ihrer Schema-Stelle.

    § 22 Nr. 3 geht bewusst auf E0305301 ("Einkünfte"), NICHT auf E0305101
    ("Summe der Einnahmen"): der Regel-Slot heißt einkuenfte_vor_freigrenze, ist also
    die Netto-Größe nach Werbungskosten (die separat in E0305201 stehen).

    Der Hebesatz ist kein Cent-Feld (typ:int, "Hebesatz in Prozent") — 400 muss 400
    bleiben und darf nicht durch die Cent→Euro-Wandlung laufen.
    """
    xml = _xml({"berufsausbildung_aufwendungen": 120000,   # 1.200 EUR
                "p22_nr3_einkuenfte": 80000,               #   800 EUR
                "gewst_messbetrag": 350000,                # 3.500 EUR
                "gewst_hebesatz": 400}, bindung)           #   400 % (kein Cent!)

    assert _pfad_im_xml(xml, ("SA", "AW_eig_BAusb", "Sum", "E0108202"), "1200")
    assert _pfad_im_xml(xml, ("SO", "Leist", "Sum", "E0305301"), "800")
    assert _pfad_im_xml(xml, ("G", "St_Erm_P35", "Ang_GSt_GMB", "Einz_Betr",
                              "Festzus_GMB_VZ", "E0801606"), "3500")
    assert _pfad_im_xml(xml, ("G", "St_Erm_P35", "Ang_GSt_GMB", "Einz_Betr",
                              "Zu_zah_GSt_VZ", "E0801705"), "400")

    # § 22 Nr. 3 darf NICHT im Einnahmen-Kz landen (das waere die Brutto-Groesse)
    assert "E0305101" not in xml, "p22_nr3 im Einnahmen-Kz statt im Einkuenfte-Kz"


# ---------------------------------------------------------------- Block 3

def test_block3_kommt_im_xml_an(bindung):
    """§ 33a Unterhalt und § 35c energetische Maßnahmen.

    Zwei Verwechslungen, die der Test festnagelt:
    - § 35c: Sanierung (E0241901 Summe Maßnahmen) und Energieberater (E0242001) haben
      GETRENNTE Kz. § 35c Abs. 1 S. 4 behandelt die Beratungskosten anders (50 % statt
      Staffel), das Schema trennt sie ebenso.
    - § 33a: E0124401 ist die in E0120103 ENTHALTENE KV/PV-Teilmenge, kein additiver
      Posten — die Summe darf nicht doppelt gezählt werden.
    """
    xml = _xml({"p33a_unterhalt_aufwendungen": 600000,        # 6.000 EUR
                "p33a_unterhalt_kv_pv": 90000,                #   900 EUR (darin enthalten)
                "p35c_sanierungsaufwendungen": 2000000,       # 20.000 EUR
                "p35c_energieberater_aufwendungen": 150000},  #  1.500 EUR
               bindung)

    assert _pfad_im_xml(xml, ("ESt1A_U", "Ang_HH_unt_P_Unt_Leist", "AW_U", "U_Ztr",
                              "E0120103"), "6000")
    assert _pfad_im_xml(xml, ("ESt1A_U", "Ang_HH_unt_P_Unt_Leist", "Ang_Unt_Pers",
                              "KV_PV", "E0124401"), "900")
    assert _pfad_im_xml(xml, ("EM_35c", "Obj", "Aufw", "Massn", "Sum", "E0241901"), "20000")
    assert _pfad_im_xml(xml, ("EM_35c", "Obj", "Aufw", "Massn", "Energieberat",
                              "E0242001"), "1500")

    # Energieberater darf NICHT auf der Massnahmen-Summe landen (frueherer Fehlbefund)
    massn_sum = xml[xml.find("E0241901"):xml.find("E0241901") + 60]
    assert ">1500<" not in massn_sum, "Energieberater-Betrag steht in der Massnahmen-Summe"


# ---------------------------------------------------------------- Block 4

def test_block4_kommt_im_xml_an(bindung):
    """Kinderbetreuung, Realsplitting § 10 Abs. 1a, DBA § 34c.

    Der wichtigste Teil ist die Trennung Realsplitting ↔ § 33a: beide heißen
    "Unterhalt", liegen aber in verschiedenen Sektionen.
      Realsplitting (Geberseite, Anlage U)  -> SO/Unt_Leist       E0304601 / E0300717
      § 33a (bedürftige Person)             -> ESt1A_U/…          E0120103 / E0124401
    Das Kürzel "ESt1A_U" legt "Anlage U" nahe, meint aber § 33a — in der Bindung waren
    dem Realsplitting zunächst die § 33a-Kz zugewiesen.
    """
    xml = _xml({"kinderbetreuungskosten": 300000,             #  3.000 EUR
                "realsplitting_unterhaltsleistungen": 1200000,  # 12.000 EUR
                "realsplitting_empfaenger_kv_pv": 180000,     #  1.800 EUR (darin enthalten)
                "dba_auslaendische_einkuenfte": 500000,       #  5.000 EUR
                "dba_gezahlte_auslaendische_steuer": 70000},  #    700 EUR
               bindung)

    assert _pfad_im_xml(xml, ("Kind", "KBK", "Art", "Sum", "E0506105"), "3000")
    assert _pfad_im_xml(xml, ("SO", "Unt_Leist", "E0304601"), "12000")
    assert _pfad_im_xml(xml, ("SO", "Unt_Leist", "E0300717"), "1800")
    assert _pfad_im_xml(xml, ("AUS", "Staat_Spez_InvFonds", "Ek", "E0601401"), "5000")
    assert _pfad_im_xml(xml, ("AUS", "Staat_Spez_InvFonds", "Anzur_ausl_St",
                              "E0601901"), "700")

    # Realsplitting darf NICHT in der § 33a-Sektion landen (und umgekehrt)
    assert "E0120103" not in xml, "Realsplitting im § 33a-Kz (ESt1A_U) statt SO/Unt_Leist"
    assert "E0124401" not in xml, "Realsplitting-KV/PV im § 33a-Kz statt SO/Unt_Leist"


def test_kist_xml_ist_schema_valide(bindung, tmp_path):
    """Das erzeugte XML validiert gegen das amtliche E10-2025-Schema.

    Fängt Ordnungsfehler (xs:sequence), die ein reiner Stringvergleich nie sieht.
    Ohne ERiC-Doku (kein $ERIC_DIR) wird übersprungen — wie in test_xsd_verify.
    """
    import validate_xsd as VX
    if not VX.find_schema("2025"):
        pytest.skip("elster11_E10_2025_extern.xsd nicht gefunden — ERIC_DIR setzen")

    ziel = tmp_path / "bloecke.xml"
    ziel.write_text(_xml({"kist_gezahlt": 60000, "kist_erstattet": 5000,
                          "berufsausbildung_aufwendungen": 120000,
                          "p22_nr3_einkuenfte": 80000,
                          "gewst_messbetrag": 350000, "gewst_hebesatz": 400,
                          "p33a_unterhalt_aufwendungen": 600000,
                          "p33a_unterhalt_kv_pv": 90000,
                          "p35c_sanierungsaufwendungen": 2000000,
                          "p35c_energieberater_aufwendungen": 150000}, bindung),
                    encoding="utf-8")
    ok, meldung = VX.validate(str(ziel), "2025")
    assert ok, f"XML nicht schema-valide: {meldung}"
