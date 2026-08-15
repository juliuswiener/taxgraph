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
                "realsplitting_empfaenger_kv_krankengeld": 150000,  # 1.500 EUR (davon)
                "dba_auslaendische_einkuenfte": 500000,       #  5.000 EUR
                "dba_gezahlte_auslaendische_steuer": 70000},  #    700 EUR
               bindung)

    assert _pfad_im_xml(xml, ("Kind", "KBK", "Art", "Sum", "E0506105"), "3000")
    assert _pfad_im_xml(xml, ("SO", "Unt_Leist", "E0304601"), "12000")
    assert _pfad_im_xml(xml, ("SO", "Unt_Leist", "E0300717"), "1800")
    # Dritte Kz desselben Containers, ergänzt 2026-08-14. Ohne sie beanstandete checkESt
    # "Zeile 7" (Krankengeld-Anspruch), obwohl Zeile 5 gefüllt war — die Anlage U war über die
    # zwei Summenfelder hinaus nicht gebaut. Die drei Beträge sind INEINANDER geschachtelt:
    # 1.500 sind Teil der 1.800, die Teil der 12.000 sind. Keine Summanden.
    assert _pfad_im_xml(xml, ("SO", "Unt_Leist", "E0300829"), "1500")
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


# ---------------------------------------------------------------- Block 6: Kind-KV/PV § 10 Abs. 1 Nr. 3 S. 2

def test_kind_kv_pv_kommt_im_xml_an(bindung):
    """kind_kv (E0503110) und kind_pv (E0503310) an ihrer Schema-Stelle, NICHT vertauscht.
    Anlage Kind ist PRO KIND ein Dokument. Zwei Kinder mit UNTERSCHIEDLICHEN KV- und PV-Werten:
      Kind1 (Basis): kind_kv=100000ct (1000€), kind_pv=50000ct (500€)
      Kind2 (__2):   kind_kv=50000ct (500€), kind_pv=0ct (0€)
    Das erste Kind-Dokument muss E0503110=1000 (KV) und E0503310=500 (PV) tragen.
    Vertauschung in der Bindung (kind_kv→E0503310, kind_pv→E0503110) liefert
    E0503110=500 im ersten Dokument — der KV-Assert schlägt fehl.
    """
    xml = _xml({"kind_idnr": "11111111111", "kind_kv": 100000, "kind_pv": 50000,
                "kind_idnr__2": "22222222222", "kind_kv__2": 50000, "kind_pv__2": 0},
               bindung)

    # Erstes Kind-Dokument (kind_idnr 11111111111): E0503110 = KV 1000€
    assert _pfad_im_xml(xml, ("Kind", "KV_PV", "AW_Stpfl", "E0503110"), "1000"), (
        "E0503110 trägt nicht 1000 (KV Kind1):\n" + xml)
    # Erstes Kind-Dokument: E0503310 = PV 500€
    assert _pfad_im_xml(xml, ("Kind", "KV_PV", "AW_Stpfl", "E0503310"), "500"), (
        "E0503310 trägt nicht 500 (PV Kind1):\n" + xml)
    # Beide Kinder haben ein Kind-Dokument: E0503110 kommt 2× vor (1000 + 500)
    import re
    flach = re.sub(r"<(/?)[a-zA-Z0-9]+:", r"<\1", xml)
    kv_werte = re.findall(r"E0503110>(\d+)", flach)
    assert kv_werte == ["1000", "500"], (
        f"KV-Werte je Kind-Dokument nicht [1000,500]: {kv_werte}\n" + xml)
    pv_werte = re.findall(r"E0503310>(\d+)", flach)
    assert pv_werte == ["500", "0"], (
        f"PV-Werte je Kind-Dokument nicht [500,0]: {pv_werte}\n" + xml)


# ---------------------------------------------------------------- Block 7: §33b Abs.5 Kind-PB-Übertragung

def test_kind_pb_uebertragung_kommt_im_xml_an(bindung):
    """kind_grad_der_behinderung (E0505809), kind_hilflos_blind_taubblind (E0505807),
    kind_hinterbliebenen_uebertragung (E0505805) an ihrer Schema-Stelle.

    Zwei Kinder mit UNTERSCHIEDLICHEN GdB-Werten:
      Kind1 (Basis): GdB 50, nicht hilflos, keine Hbl
      Kind2 (__2):   GdB 100, nicht hilflos, keine Hbl

    E0505809 im ersten Kind-Dokument muss "50", im zweiten "100".
    E0505807/E0505805 müssen False (kein >1) sein.
    """
    xml = _xml({"kind_idnr": "11111111111", "kind_grad_der_behinderung": 50,
                "kind_hilflos_blind_taubblind": False, "kind_hinterbliebenen_uebertragung": False,
                "kind_idnr__2": "22222222222", "kind_grad_der_behinderung__2": 100,
                "kind_hilflos_blind_taubblind__2": False, "kind_hinterbliebenen_uebertragung__2": False},
               bindung)

    assert _pfad_im_xml(xml, ("Kind", "Ueb_PB_Beh_Hbl", "Beh", "Ausw_Rentb_Besch", "E0505809"), "50"), (
        "E0505809 trägt nicht 50 (GdB Kind1):\n" + xml)
    import re
    flach = re.sub(r"<(/?)[a-zA-Z0-9]+:", r"<\1", xml)
    gdb_werte = re.findall(r"E0505809>(\d+)", flach)
    assert gdb_werte == ["50", "100"], (
        f"GdB-Werte je Kind-Dokument nicht [50,100]: {gdb_werte}")
    assert "E0505807" not in flach, (
        "E0505807 (blind/hilflos) sollte False sein → Element weggelassen, nicht 1:")
    assert "E0505805" not in flach, (
        "E0505805 (Hbl-Übertragung) sollte False sein → Element weggelassen, nicht 1:")


def test_kind_pb_uebertragung_hinterbliebenen_kommt_im_xml_an(bindung):
    """Hbl-Übertragung (E0505805) = True → "1" im XML."""
    xml = _xml({"kind_idnr": "11111111111", "kind_grad_der_behinderung": 50,
                "kind_hilflos_blind_taubblind": False, "kind_hinterbliebenen_uebertragung": True},
               bindung)
    assert _pfad_im_xml(xml, ("Kind", "Ueb_PB_Beh_Hbl", "Hbl", "E0505805"), "1"), (
        "E0505805 trägt nicht 1 (Hbl-Übertragung, Ja1-Typ):\n" + xml)


def test_kind_pb_uebertragung_xml_ist_schema_valide(bindung, tmp_path):
    """Das erzeugte XML validiert gegen das amtliche E10-2025-Schema.

    Schärft die String-Assertions oben: xs:sequence ist ordnungsempfindlich, ein Kz an
    falscher Stelle oder mit falschem Ja1-Wert ('X' statt '1') fällt nur hier auf. Beide
    Kinder tragen GdB 50/100 + False-Ja-Werte (E0505807/E0505805 werden weggelassen)."""
    import validate_xsd as VX
    if not VX.find_schema("2025"):
        pytest.skip("elster11_E10_2025_extern.xsd nicht gefunden")
    ziel = tmp_path / "kind_pb.xml"
    ziel.write_text(_xml({"kind_idnr": "11111111111", "kind_grad_der_behinderung": 50,
                          "kind_hilflos_blind_taubblind": False, "kind_hinterbliebenen_uebertragung": False,
                          "kind_idnr__2": "22222222222", "kind_grad_der_behinderung__2": 100,
                          "kind_hilflos_blind_taubblind__2": False, "kind_hinterbliebenen_uebertragung__2": False},
                         bindung), encoding="utf-8")
    ok, meldung = VX.validate(str(ziel), "2025")
    assert ok, f"XML nicht schema-valide: {meldung}"


def test_kind_pb_uebertragung_xml_mit_hbl_ist_schema_valide(bindung, tmp_path):
    """Hbl-Übertragung (E0505805=True) validiert gegen das amtliche Schema.

    E0505805 ist Ja1-Typ — ohne Fix rendert er 'X', XSD erwartet '1'."""
    import validate_xsd as VX
    if not VX.find_schema("2025"):
        pytest.skip("elster11_E10_2025_extern.xsd nicht gefunden")
    ziel = tmp_path / "kind_pb_hbl.xml"
    ziel.write_text(_xml({"kind_idnr": "11111111111", "kind_grad_der_behinderung": 50,
                          "kind_hilflos_blind_taubblind": False, "kind_hinterbliebenen_uebertragung": True},
                         bindung), encoding="utf-8")
    ok, meldung = VX.validate(str(ziel), "2025")
    assert ok, f"XML mit Hbl nicht schema-valide: {meldung}"


# ---------------------------------------------------------------- Block 8: §33 Abs.2a Fahrtkostenpauschale (Person A)

def test_fahrtkostenpauschale_kommt_im_xml_an(bindung):
    """fahrtkosten_pausch_gdb80_oder_70g (E0161706) und fahrtkosten_pausch_ag_bl_tbl_h (E0161806)
    sind Ja1-Kz unter AgB/Beh_Fk_Pausch → True="1", False = Element weggelassen."""
    xml = _xml({"fahrtkosten_pausch_gdb80_oder_70g": True,
                "fahrtkosten_pausch_ag_bl_tbl_h": False}, bindung)
    assert _pfad_im_xml(xml, ("AgB", "Beh_Fk_Pausch", "E0161706"), "1"), (
        "E0161706 trägt nicht 1 (GdB80/70+G, Ja1-Typ):\n" + xml)
    assert "E0161806" not in xml.replace("ns0:", "").replace("ns1:", ""), (
        "E0161806 (aG/Bl/TBl/H) sollte False sein → Element weggelassen:\n" + xml)


def test_fahrtkostenpauschale_ag_bl_tbl_h_kommt_im_xml_an(bindung):
    """fahrtkosten_pausch_ag_bl_tbl_h (E0161806) = True → "1" (Ja1-Typ)."""
    xml = _xml({"fahrtkosten_pausch_gdb80_oder_70g": False,
                "fahrtkosten_pausch_ag_bl_tbl_h": True}, bindung)
    assert _pfad_im_xml(xml, ("AgB", "Beh_Fk_Pausch", "E0161806"), "1"), (
        "E0161806 trägt nicht 1 (aG/Bl/TBl/H, Ja1-Typ):\n" + xml)


def test_fahrtkostenpauschale_xml_ist_schema_valide(bindung, tmp_path):
    """Beide Fahrtkosten-Kz (True auf E0161706, False auf E0161806) validieren gegen das Schema."""
    import validate_xsd as VX
    if not VX.find_schema("2025"):
        pytest.skip("elster11_E10_2025_extern.xsd nicht gefunden")
    ziel = tmp_path / "fk_pausch.xml"
    ziel.write_text(_xml({"fahrtkosten_pausch_gdb80_oder_70g": True,
                          "fahrtkosten_pausch_ag_bl_tbl_h": False}, bindung), encoding="utf-8")
    ok, meldung = VX.validate(str(ziel), "2025")
    assert ok, f"XML nicht schema-valide: {meldung}"


# ---------------------------------------------------------------- Pflege-Pauschbetrag § 33b Abs. 6

def test_pflege_pauschbetrag_pflichtangaben_im_xml(bindung):
    """Die fünf Begleitangaben neben der Betrags-Staffel (2026-08-15).

    Bis dahin trug die Anlage nur Pflegegrad und Merkzeichen H — also genau die zwei Felder mit
    Rechenwirkung. checkESt lehnte den Pflege-Pauschbetrag deshalb ab (rc=610001002), und zwar in
    DREI Schichten: erst fehlten Wohnsitz und Helferzahl, nach deren Ergänzung IdNr,
    Personenangaben und "durch wen die Pflege erfolgt".

    Der Test prüft die Kz einzeln, weil sie in DREI verschiedenen Containern liegen: die Angaben
    zur gepflegten Person, die zur pflegenden Person und die zu weiteren Beteiligten. Eine
    Verwechslung dazwischen wäre XSD-valide und trotzdem falsch.
    """
    xml = _xml({"rentner_pflegegrad": 4,
                "rentner_gepflegter_hilflos": True,
                "rentner_gepflegter_wohnsitz_inland": True,
                "rentner_pflege_weitere_personen": 0,
                "rentner_gepflegter_idnr": "12345678911",
                "rentner_gepflegter_angaben": "Maria Muster, Musterweg 3, 12345 Musterstadt, meine Mutter",
                "rentner_pflege_durch": "1"},
               bindung)

    pers = ("AgB", "Pflege_PB", "Einz", "Ang_pflegebeduerft_Pers")
    assert _pfad_im_xml(xml, pers + ("E0161506",), "12345678911")
    assert _pfad_im_xml(xml, pers + ("E0110601",),
                        "Maria Muster, Musterweg 3, 12345 Musterstadt, meine Mutter")
    assert _pfad_im_xml(xml, pers + ("E0161607",), "1")          # JaNein12: 1 = Ja
    assert _pfad_im_xml(xml, ("AgB", "Pflege_PB", "Einz", "Ang_pflegende_Pers", "E0106507"), "1")
    assert _pfad_im_xml(xml, ("AgB", "Pflege_PB", "Einz", "Ang_an_Pflege_bet_Pers", "E0106603"), "0")

    # Die 0 bei den weiteren Pflegepersonen ist der Normalfall (Alleinpflege) und MUSS erklärt
    # werden — ERiC verlangt sie ausdrücklich "(gegebenenfalls 0)". Ein Writer, der Nullwerte
    # generell weglässt, würde die Anlage wieder unabgabefähig machen.
    import re
    assert "<E0106603>0</E0106603>" in re.sub(r"<(/?)[a-zA-Z0-9]+:", r"<\1", xml)
