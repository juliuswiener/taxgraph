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


# ---------------------------------------------------------------- Anlage Energetische Maßnahmen § 35c

def test_p35c_anlage_kommt_im_xml_an(bindung):
    """Die Formalien der Anlage (2026-08-16). Bis dahin trug die Bindung nur die zwei
    Summenfelder — checkESt lehnte mit SIEBEN Beanstandungen ab.

    Zwei Dinge nagelt der Test fest, die man leicht falsch baut:

    1. Der Betrag steht ZWEIMAL im XML: als Summe (E0241901) und in der Zeile der gewählten
       Maßnahmenart (hier Heizung, E0241501). Das Formular verlangt beides; wer nur die Summe
       schreibt, bekommt "keine Angaben zu den einzelnen Aufwendungen".
    2. E0240902 fragt UMGEKEHRT zu unserem Gate ("habe ich in Anspruch genommen" gegen
       "keine Doppelförderung"). Bei keine_doppelfoerderung=True muss dort "2" (Nein) stehen.
       Vorher hätte der Writer das Element bei False ganz weggelassen — JaNein12 ist aber kein
       Ankreuzfeld, sondern zweiwertig.
    """
    xml = _xml({"p35c_sanierungsaufwendungen": 2000000,        # 20.000 EUR
                "p35c_massnahme_art": "heizung",
                "p35c_massnahme_einzelbetrag": 2000000,
                "p35c_foerderung_in_anspruch": False,          # = keine Doppelförderung
                "p35c_objekt_strasse": "Musterweg 3",
                "p35c_objekt_plz_ort": "12345 Musterstadt",
                "p35c_gebaeude_herstellungsbeginn": "01.06.1995",
                "p35c_baubeginn_massnahme": "15.03.2025",
                "p35c_gesamtflaeche_qm": 120,
                "p35c_eigene_wohnflaeche_qm": 120,
                "p35c_bereits_ermaessigung_frueher": False},
               bindung)

    allg = ("EM_35c", "Obj", "Allg")
    assert _pfad_im_xml(xml, allg + ("E0240401",), "Musterweg 3")
    assert _pfad_im_xml(xml, allg + ("E0240501",), "12345 Musterstadt")
    assert _pfad_im_xml(xml, allg + ("E0240402",), "01.06.1995")
    assert _pfad_im_xml(xml, allg + ("E0240801",), "120")
    assert _pfad_im_xml(xml, allg + ("E0240802",), "120")
    assert _pfad_im_xml(xml, allg + ("E0240803",), "2")        # JaNein12: 2 = Nein
    assert _pfad_im_xml(xml, ("EM_35c", "Obj", "Aufw", "E0240902"), "2")
    assert _pfad_im_xml(xml, ("EM_35c", "Obj", "Aufw", "Massn", "E0240901"), "15.03.2025")
    # Summe UND Einzelzeile — der Betrag steht zweimal, das ist gewollt.
    assert _pfad_im_xml(xml, ("EM_35c", "Obj", "Aufw", "Massn", "Sum", "E0241901"), "20000")
    assert _pfad_im_xml(xml, ("EM_35c", "Obj", "Aufw", "Massn", "Heizung", "E0241501"), "20000")


def test_janein12_nein_wird_geschrieben_nicht_weggelassen(bindung):
    """Der Writer-Fix von 2026-08-16, isoliert.

    Ja-Typen sind zwei verschiedene Dinge: Ja1/JaX sind ANKREUZFELDER (Nein = weglassen),
    JaNein12 hat ZWEI echte Werte. Vorher ließ der Writer bei False in beiden Fällen das Element
    weg — bei JaNein12 verschwand damit eine gegebene Antwort, und checkESt beanstandete "Bitte
    geben Sie an, ob …". Betroffen war auch E0161607 (Wohnsitz der gepflegten Person).
    """
    import re
    xml = _xml({"p35c_bereits_ermaessigung_frueher": False,
                "rentner_gepflegter_wohnsitz_inland": False,
                "rentner_gepflegter_hilflos": False}, bindung)
    flach = re.sub(r"<(/?)[a-zA-Z0-9]+:", r"<\1", xml)
    assert "<E0240803>2</E0240803>" in flach, "JaNein12-Nein fehlt (Element weggelassen?)"
    assert "<E0161607>2</E0161607>" in flach, "JaNein12-Nein fehlt beim Pflege-Wohnsitz"
    # Ja1 bleibt ein Ankreuzfeld: False -> Element gar nicht erst anlegen.
    assert "E0161808" not in flach, "Ja1-Nein darf NICHT als Element erscheinen"


# Neun Maßnahmenarten -> neun Zeilen der Anlage.
P35C_ARTEN = [
    ("waende", "Waende", "E0241001"),
    ("dach", "Dach", "E0241101"),
    ("geschossdecken", "Geschossd", "E0241201"),
    ("fenster_tueren", "Fenst_Tuer", "E0241301"),
    ("sommerlicher_waermeschutz", "Somm_Waerm", "E0241302"),
    ("lueftung", "Lueftung", "E0241401"),
    ("heizung", "Heizung", "E0241501"),
    ("digital", "Digital", "E0241601"),
    ("heizung_optimierung", "Heizung_alt", "E0241701"),
]


# Die Kz stehen ABSICHTLICH auch im parametrize-Aufruf und nicht nur in der Konstante darueber:
# test_m prueft per AST, ob ein gebundenes Kz in einem assert-gesteuerten Pfad LIEGT, und eine
# Liste daneben zaehlt dort zu Recht nicht — sonst genuegte jede Erwaehnung im Dateitext.
@pytest.mark.parametrize("art,container,kz", [
    ("waende", "Waende", "E0241001"),
    ("dach", "Dach", "E0241101"),
    ("geschossdecken", "Geschossd", "E0241201"),
    ("fenster_tueren", "Fenst_Tuer", "E0241301"),
    ("sommerlicher_waermeschutz", "Somm_Waerm", "E0241302"),
    ("lueftung", "Lueftung", "E0241401"),
    ("heizung", "Heizung", "E0241501"),
    ("digital", "Digital", "E0241601"),
    ("heizung_optimierung", "Heizung_alt", "E0241701"),
])
def test_p35c_massnahmenart_trifft_ihre_zeile(bindung, art, container, kz):
    """Jede Maßnahmenart landet in IHRER Zeile — und in keiner anderen.

    Die Tabelle steht in est_mapping.VERZWEIGUNG; ein Zahlendreher darin waere XSD-valide und
    stuende in der falschen Zeile. Deshalb wird JEDE Art gefahren, nicht nur die aus dem
    Block-Test."""
    xml = _xml({"p35c_massnahme_art": art, "p35c_massnahme_einzelbetrag": 2000000}, bindung)
    assert _pfad_im_xml(xml, ("EM_35c", "Obj", "Aufw", "Massn", container, kz), "20000"), (
        f"{art} landet nicht in {container}/{kz}")
    fremde = [k for _, _, k in P35C_ARTEN if k != kz]
    getroffen = [k for k in fremde if k in xml]
    assert not getroffen, f"{art} schreibt auch in fremde Zeilen: {getroffen}"


# ---------------------------------------------------------------- Anlage V § 21

def test_anlage_v_kommt_im_xml_an(bindung):
    """Die Formalien der Anlage V (2026-08-16). Bis dahin trug die Bindung nur die fünf
    Rechen-Slots (Einnahmen, AfA, Zinsen, Erhaltung, sonstige WK) — checkESt lehnte in fünf
    aufeinander folgenden Schichten ab.

    Der Test nagelt die DREI Summenzeilen fest, die leicht verwechselt werden:
      E0700206  Summe der Wohnungs-Mieteinnahmen
      E0701401  Summe ALLER Einnahmen des Objekts (Mieten + Umlagen + Sonstiges)
      E0705701  Summe der Werbungskosten
    und die Ergebniszeile E0701601 samt ihrer Zurechnung E0701801. Eine Verwechslung wäre
    XSD-valide und stünde in der falschen Zeile.
    """
    xml = _xml({"vv_einnahmen": 1200000,               # 12.000 EUR
                "vv_mieteinnahmen_summe": 1200000,
                "vv_einnahmen_summe_gesamt": 1200000,
                "vv_summe_werbungskosten": 200000,     #  2.000 EUR
                "vv_ueberschuss": 1000000,             # 10.000 EUR
                "vv_ueberschuss_person_a": 1000000,
                "vv_objekt_strasse": "Mietweg 7",
                "vv_objekt_plz": "12345",
                "vv_objekt_ort": "Musterstadt",
                "vv_wohneinheit_bezeichnung": "1. OG links",
                "vv_nebenkosten_nicht_vereinbart": True,
                "vv_nutzung_ferienwohnung": False,
                "vv_nutzung_an_angehoerige": False,
                "vv_nutzung_kurzfristig": False},
               bindung)

    lage = ("V", "Allg", "Lage")
    assert _pfad_im_xml(xml, lage + ("E0700407",), "Mietweg 7")
    assert _pfad_im_xml(xml, lage + ("E0700503",), "12345")
    assert _pfad_im_xml(xml, lage + ("E0700504",), "Musterstadt")
    nutzung = ("V", "Allg", "Nutzung")
    assert _pfad_im_xml(xml, nutzung + ("E0700703",), "2")     # JaNein12: 2 = Nein
    assert _pfad_im_xml(xml, nutzung + ("E0700704",), "2")
    assert _pfad_im_xml(xml, nutzung + ("E0700705",), "2")
    einz = ("V", "Einn", "Mieteinn", "Whg", "Einz")
    assert _pfad_im_xml(xml, einz + ("E0701202",), "1. OG links")
    assert _pfad_im_xml(xml, einz + ("E0700201",), "12000")
    # drei verschiedene Summen, drei verschiedene Bedeutungen
    assert _pfad_im_xml(xml, ("V", "Einn", "Mieteinn", "Whg", "Sum", "E0700206"), "12000")
    assert _pfad_im_xml(xml, ("V", "Einn", "Sum", "E0701401"), "12000")
    assert _pfad_im_xml(xml, ("V", "Wk", "Se_WK", "E0705701"), "2000")
    assert _pfad_im_xml(xml, ("V", "Erm_Zuord_Ek", "E0701601"), "10000")
    assert _pfad_im_xml(xml, ("V", "Erm_Zuord_Ek", "E0701801"), "10000")
    assert _pfad_im_xml(xml, ("V", "Einn", "Uml", "E0702404"), "1")   # Ja1: nicht vereinbart

    # Gegenprobe mit gesondert vereinbarten Nebenkosten: dann steht der BETRAG in E0700501, und
    # die Gesamt-Einnahmensumme enthaelt ihn (12.000 Miete + 600 Umlagen).
    xml2 = _xml({"vv_einnahmen": 1200000, "vv_nebenkosten_umgelegt": 60000,
                 "vv_nebenkosten_nicht_vereinbart": False,
                 "vv_einnahmen_summe_gesamt": 1260000}, bindung)
    assert _pfad_im_xml(xml2, ("V", "Einn", "Uml", "E0700501"), "600")
    assert _pfad_im_xml(xml2, ("V", "Einn", "Sum", "E0701401"), "12600")
    # E0702404 ist Ja1, KEIN JaNein12 — im selben Formular stehen beide Familien nebeneinander:
    # die drei Nutzungs-Flags oben sind zweiwertig ("2" = Nein), diese Zeile ist ein Ankreuzfeld
    # (Nein = Element weglassen). Genau diese Unterscheidung trifft der Writer-Fix.
    import re as _re
    assert "E0702404" not in _re.sub(r"<(/?)[a-zA-Z0-9]+:", r"<\1", xml2), (
        "Ja1-Nein darf NICHT als Element erscheinen")


def test_laufende_nummer_v_wird_geschrieben(bindung):
    """`Laufende_Nummer_V` ist im Schema OPTIONAL (minOccurs=0), von ERiC aber verlangt:
    "'$/V[1]/Laufende_Nummer_V[1]$': Das Feld muss angegeben werden."

    Dieselbe Klasse wie der Kontoinhaber, der trotz minOccurs=0 Pflicht ist — Schema und
    Plausibilität fallen in beide Richtungen auseinander, und nur der scharfe Lauf sagt, in
    welche. Der Writer führt die Ausnahme deshalb als GEMESSENE Liste, nicht als Ableitung."""
    import re
    xml = _xml({"vv_einnahmen": 1200000}, bindung)
    flach = re.sub(r"<(/?)[a-zA-Z0-9]+:", r"<\1", xml)
    assert "<Laufende_Nummer_V>1</Laufende_Nummer_V>" in flach, (
        "Laufende_Nummer_V fehlt — ERiC verlangt es trotz minOccurs=0")


# ---------------------------------------------------------------- doppelte Haushaltsführung § 9 Abs. 1 S. 3 Nr. 5

def test_dhf_formalien_kommen_im_xml_an(bindung):
    """Die Formalien der doppelten Haushaltsführung (2026-08-19). Bis dahin trug die Bindung
    nur die Beträge und vier Tatbestands-Bools — checkESt lehnte ab, egal wie richtig die
    Zahlen waren.

    VIER SCHICHTEN, jede erst sichtbar, nachdem die vorige beantwortet war:

        nur Beträge                rc=610001002  Beschäftigungsort, Grund, Datum der
                                                 Begründung, Aussage zum eigenen Hausstand
        + E0206504 (Rewiring)      rc=610001002  NEU: PLZ/Ort des eigenen Hausstandes und
                                                 der Zeitpunkt, seit dem er besteht
        + die fünf Formalien       rc=610001002  Datum der Begründung und des ununter-
                                                 brochenen Bestehens GEMEINSAM
        + E0206304                 rc=0

    Zwei Dinge nagelt der Test fest, die man leicht falsch baut:

    1. E0206504 ist JaNein12, kein Ankreuzfeld. `dhf_eigener_hausstand` trug bis zu diesem
       Bau `elster_kz: null` mit dem Grund "Tatbestands-Voraussetzung (Ja/Nein), kein
       Betragsfeld". Der erste Halbsatz stimmt, der Schluss daraus war falsch: kein
       Betragsfeld zu sein heißt nicht, keine Deklarationsseite zu haben. Das Feld war
       askable und ring-wirksam (DHF_BEDINGUNGEN gated in bescheid_zweige.py den ganzen
       Werbungskostenabzug), aber die Antwort erreichte das Finanzamt nie.
    2. E0206304 ist DatumTTpMMp — Tag und Monat MIT abschließendem Punkt, OHNE Jahr, weil das
       Jahr der Veranlagungszeitraum ist. Deshalb trägt das Feld `typ: text` und nicht
       `typ: datum`: Letzteres prüft fail-closed auf ^\\d{2}\\.\\d{2}\\.\\d{4}$ (store.py:190)
       und wiese "31.12." ab.
    """
    xml = _xml({"dhf_unterkunftskosten_monat": 80000,          # 800 EUR/Monat
                "dhf_monate": 12,
                "dhf_eigener_hausstand": True,
                "dhf_beschaeftigungsort": "80331 München",
                "dhf_grund": "Versetzung an den Beschäftigungsort",
                "dhf_begruendet_am": "01.03.2025",
                "dhf_bestanden_bis": "31.12.",
                "dhf_hausstand_plz_ort": "20095 Hamburg",
                "dhf_hausstand_seit": "01.01.2015"},
               bindung)

    allg = ("N_DHH", "DHHF", "Allg")
    assert _pfad_im_xml(xml, allg + ("E0206404",), "80331 München")
    assert _pfad_im_xml(xml, allg + ("E0206205",), "Versetzung an den Beschäftigungsort")
    assert _pfad_im_xml(xml, allg + ("E0206103",), "01.03.2025")
    assert _pfad_im_xml(xml, allg + ("E0206304",), "31.12.")
    assert _pfad_im_xml(xml, allg + ("E0206504",), "1")        # JaNein12: 1 = Ja
    assert _pfad_im_xml(xml, allg + ("E0206505",), "20095 Hamburg")
    assert _pfad_im_xml(xml, allg + ("E0206506",), "01.01.2015")


def test_dhf_eigener_hausstand_nein_schreibt_die_zwei(bindung):
    """Gegenprobe zu Punkt 1 oben: bei "Nein" muss "2" im XML stehen, nicht nichts.

    Das ist der Unterschied, an dem die JaNein12-Klasse hängt (E0240803/E0240902/E0161607
    haben ihn schon einmal gekostet): bei einem Ankreuzfeld IST das Weglassen die Antwort
    "Nein", bei JaNein12 ist "Nein" ein eigener Wert. Ließe der Writer das Element weg,
    verschwände eine gegebene Antwort lautlos — und checkESt beanstandet genau das
    ("Bitte treffen Sie eine Aussage, ob …").
    """
    xml = _xml({"dhf_unterkunftskosten_monat": 80000,
                "dhf_monate": 12,
                "dhf_eigener_hausstand": False},
               bindung)
    assert _pfad_im_xml(xml, ("N_DHH", "DHHF", "Allg", "E0206504"), "2")


# ---------------------------------------------------------------- Entfernungspauschale § 9 Abs. 1 S. 3 Nr. 4

def test_ep_zieladresse_kommt_im_xml_an(bindung):
    """Ziel des Weges + Zieladresse (2026-08-19). Ohne sie lehnte checkESt ab: "Bei den Angaben
    zur Entfernungspauschale fehlt die Angabe zum Ziel des Weges und / oder zu PLZ, Ort und
    Straße". EINE Schicht, danach rc=0.

    Die zweite Zusicherung dieses Tests ist die WENIGER offensichtliche: das amtliche Beispiel
    est_e10_2025.xml füllt im selben Container acht Geschwisterfelder, darunter E0203101
    (Zeitraum), E0203508 (Arbeitstage je Woche) und E0203509 (Ausfalltage). Aus dem Beispiel
    allein liest sich das wie eine Folgeanforderung — gemessen wurde sie NICHT nachgefordert.
    Hätte man dem Beispiel statt der Messung geglaubt, wären drei überflüssige Pflichtfragen
    im Dialog gelandet. Ein amtliches Beispiel zeigt, was erlaubt ist, nicht was verlangt wird.
    """
    xml = _xml({"ep_arbeitstage": 220,
                "ep_entfernung_km": 42,
                "ep_ziel_des_weges": "1",
                "ep_ziel_adresse": "80331 München, Marienplatz 1"},
               bindung)

    erste = ("N", "Wk", "EP", "Erste_Taetig")
    assert _pfad_im_xml(xml, erste + ("E0203003",), "1")
    assert _pfad_im_xml(xml, erste + ("E0203501",), "80331 München, Marienplatz 1")
    assert _pfad_im_xml(xml, erste + ("E0203503",), "220")     # Arbeitstage, schon vorher gebunden
    assert _pfad_im_xml(xml, erste + ("E0203504",), "42")      # Entfernung, schon vorher gebunden


# ---------------------------------------------------------------- eigene Berufsausbildung § 10 Abs. 1 Nr. 7

def test_berufsausbildung_einzelaufstellung_kommt_im_xml_an(bindung):
    """Bezeichnung + Einzelbetrag neben der Summe (2026-08-19). Mit der Summe allein lehnte
    checkESt ab: "Es wurde die Summe der Aufwendungen für die eigene Berufsausbildung
    angegeben, bitte geben Sie auch die Bezeichnung der Ausbildung und die Art und Höhe der
    einzelnen Aufwendungen an".

    Der Betrag steht ZWEIMAL im XML — als Summe (E0108202, vom Nutzer erfragt) und in der
    Einzelzeile (E0108002, von bescheid_deklaration._mit_ring_werten aus der Summe berechnet).
    Bei EINEM Posten sind beide gleich; das ist die ausdrückliche MVP-Grenze, das Schema
    erlaubt bis zu zehn Einz-Einträge.

    Hier wird der Einzelbetrag von Hand gesetzt, weil dieser Test den WRITER prüft, nicht die
    Ring-Injektion — der berechnete Zwilling entsteht eine Schicht früher.
    """
    xml = _xml({"berufsausbildung_aufwendungen": 200000,        # 2.000 EUR
                "berufsausbildung_einzelbetrag": 200000,
                "berufsausbildung_bezeichnung": "Studium Betriebswirtschaft, Semesterbeiträge"},
               bindung)

    assert _pfad_im_xml(xml, ("SA", "AW_eig_BAusb", "Sum", "E0108202"), "2000")
    assert _pfad_im_xml(xml, ("SA", "AW_eig_BAusb", "Einz", "E0108002"), "2000")
    assert _pfad_im_xml(xml, ("SA", "AW_eig_BAusb", "Einz", "E0108201"),
                        "Studium Betriebswirtschaft, Semesterbeiträge")


def test_berufsausbildung_einzelbetrag_wird_aus_der_summe_berechnet():
    """Die andere Hälfte: der Zwilling entsteht in _mit_ring_werten, nicht im Writer.

    Ohne diesen Schritt bliebe E0108002 leer und die Beanstandung stünde wieder da — der
    Writer-Test oben würde das NICHT bemerken, weil er den Wert selbst setzt. Genau diese Naht
    (Ring ODER Writer geprüft, nie die Übergabe) war die Person-B-Lücke.
    """
    import os
    import sys
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    sys.path.insert(0, os.path.join(root, "produkt", "bescheid"))
    import bescheid_deklaration as BD

    felder = {"berufsausbildung_aufwendungen": {"wert": 200000, "zustand": "bestaetigt"}}
    aus = BD._mit_ring_werten(dict(felder), 2025)
    assert aus["berufsausbildung_einzelbetrag"]["wert"] == 200000
    assert aus["berufsausbildung_einzelbetrag"]["zustand"] == "bestaetigt"

    # Gegenprobe: ohne bestätigte Summe entsteht kein Zwilling — sonst schriebe die Erklärung
    # eine Einzelzeile für einen Betrag, den der Nutzer nie bestätigt hat.
    offen = {"berufsausbildung_aufwendungen": {"wert": 200000, "zustand": "vorlaeufig"}}
    assert "berufsausbildung_einzelbetrag" not in BD._mit_ring_werten(dict(offen), 2025)


# ---------------------------------------------------------------- § 22 Nr. 3 sonstige Leistungen

def test_p22_nr3_einnahmen_und_werbungskosten_kommen_im_xml_an(bindung):
    """Vier Zahlen, von denen der Nutzer zwei kennt (2026-08-19). Vier Messrunden:

        nur Einkünfte                rc=610001002  "es fehlt eine Angabe zu den Einnahmen"
        + Einnahmen                  rc=610001002  Einzelbeträge fehlen; UND: "Einkünfte
                                                   entspricht nicht Einnahmen abzüglich
                                                   Werbungskosten"
        + Einzelbetrag + WK          rc=610001002  "Art und Höhe der Einnahmen gemeinsam"
        + Art                        rc=0

    Die zweite Runde ist der Grund, warum hier gerechnet statt behauptet wird: ERiC prüft
    Einnahmen − Werbungskosten == Einkünfte nach. Ein leeres Werbungskosten-Feld erfüllt das
    nicht, und ein Zwilling "Einnahmen = Einkünfte" hätte Werbungskosten von null behauptet,
    obwohl das ältere Feld ausdrücklich netto fragt.
    """
    xml = _xml({"p22_nr3_einkuenfte": 100000,          # 1.000 EUR netto
                "p22_nr3_einnahmen": 130000,           # 1.300 EUR brutto
                "p22_nr3_einnahmen_art": "Gelegentliche Vermittlung",
                "p22_nr3_einnahmen_einzelbetrag": 130000,
                "p22_nr3_werbungskosten": 30000},      # 300 EUR Differenz
               bindung)

    assert _pfad_im_xml(xml, ("SO", "Leist", "Einz", "E0305103"), "Gelegentliche Vermittlung")
    assert _pfad_im_xml(xml, ("SO", "Leist", "Einz", "E0305104"), "1300")
    assert _pfad_im_xml(xml, ("SO", "Leist", "Sum", "E0305101"), "1300")
    assert _pfad_im_xml(xml, ("SO", "Leist", "Sum", "E0305201"), "300")
    assert _pfad_im_xml(xml, ("SO", "Leist", "Sum", "E0305301"), "1000")


def test_p22_nr3_ohne_werbungskosten_bleibt_das_element_weg():
    """E0305201 ist GanzzahlPos — eine 0 ist dort UNZULÄSSIG.

    Wer keine Werbungskosten hatte, gibt Einnahmen gleich Einkünfte an. Ein Zwilling, der stur
    die Differenz schreibt, setzte dann 0 und das Schema wiese es ab. Die Konsistenzprobe geht
    ohne das Element trotzdem auf: Einnahmen minus nichts ist gleich Einkünfte (gemessen, rc=0).

    Die Gegenrichtung wird gleich mitgeprüft: bei widersprüchlicher Eingabe (Einnahmen KLEINER
    als Einkünfte) entsteht ebenfalls kein Feld — der Widerspruch wird nicht auf 0 geklemmt,
    sondern bleibt sichtbar, damit checkESt ihn beanstandet statt ihn zu schlucken.
    """
    import os
    import sys
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    sys.path.insert(0, os.path.join(root, "produkt", "bescheid"))
    import bescheid_deklaration as BD

    def _lauf(einnahmen, einkuenfte):
        f = {"p22_nr3_einnahmen": {"wert": einnahmen, "zustand": "bestaetigt"},
             "p22_nr3_einkuenfte": {"wert": einkuenfte, "zustand": "bestaetigt"}}
        return BD._mit_ring_werten(f, 2025)

    gleich = _lauf(100000, 100000)
    assert "p22_nr3_werbungskosten" not in gleich, "0 EUR Werbungskosten dürfen kein Element werden"
    assert gleich["p22_nr3_einnahmen_einzelbetrag"]["wert"] == 100000

    widerspruch = _lauf(80000, 100000)          # Einnahmen < Einkünfte: unmöglich
    assert "p22_nr3_werbungskosten" not in widerspruch

    mit_kosten = _lauf(130000, 100000)
    assert mit_kosten["p22_nr3_werbungskosten"]["wert"] == 30000


# ---------------------------------------------------------------- § 35 Gewerbesteuer-Anrechnung

def test_gewst_zu_zahlen_kommt_im_xml_an(bindung):
    """Die dritte Zahl neben Messbetrag und Hebesatz (2026-08-19). Ohne sie zwei Beanstandungen:
    "... sind gemeinsam anzugeben" und "Der Hebesatz wurde angegeben, die zu zahlende
    Gewerbesteuer jedoch nicht". Mit ihr rc=0.

    Anders als bei § 22 Nr. 3, wo der fehlende Wert erfragt werden musste, wird hier gerechnet:
    § 16 Abs. 1 GewStG sagt Messbetrag mal Hebesatz, und beide Faktoren hat der Nutzer aus
    seinen Bescheiden eingetragen. 5.000 EUR Messbetrag bei 400 % ergeben 20.000 EUR.

    NICHT der 4-fache Messbetrag aus § 35 Abs. 1 S. 2 EStG — das ist die Obergrenze der
    Anrechnung auf die Einkommensteuer (runner.py), eine andere Norm und eine andere Zahl. Die
    beiden liegen im Code nebeneinander und sind leicht zu verwechseln.
    """
    xml = _xml({"gewst_messbetrag": 500000,      # 5.000 EUR
                "gewst_hebesatz": 400,           # 400 %
                "gewst_zu_zahlen": 2000000},     # 20.000 EUR
               bindung)

    zu_zahlen = ("G", "St_Erm_P35", "Ang_GSt_GMB", "Einz_Betr", "Zu_zah_GSt_VZ")
    assert _pfad_im_xml(xml, zu_zahlen + ("E0801704",), "20000")
    assert _pfad_im_xml(xml, zu_zahlen + ("E0801705",), "400")


def test_gewst_zu_zahlen_wird_berechnet_beide_personen():
    """Die Rechnung selbst, für Person A und den Partnerbetrieb.

    Der Hebesatz ist eine Prozentzahl, der Messbetrag steht in Cent — also
    messbetrag * hebesatz // 100, Ergebnis wieder Cent. Ganzzahlig ABGERUNDET: die Gemeinde
    setzt volle Euro fest, und Aufrunden behauptete eine höhere Steuerschuld als die
    tatsächliche, was über § 35 zu einer zu hohen Anrechnung führte.
    """
    import os
    import sys
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    sys.path.insert(0, os.path.join(root, "produkt", "bescheid"))
    import bescheid_deklaration as BD

    aus = BD._mit_ring_werten({
        "gewst_messbetrag": {"wert": 500000, "zustand": "bestaetigt"},
        "gewst_hebesatz": {"wert": 400, "zustand": "bestaetigt"},
        "gewst_messbetrag_partner": {"wert": 300000, "zustand": "bestaetigt"},
        "gewst_hebesatz_partner": {"wert": 450, "zustand": "bestaetigt"},
    }, 2025)
    assert aus["gewst_zu_zahlen"]["wert"] == 2000000            # 5.000 * 400 %
    assert aus["gewst_zu_zahlen_partner"]["wert"] == 1350000    # 3.000 * 450 %

    # Ein Betrieb allein lässt die andere Seite unberührt — sonst entstünde eine
    # Gewerbesteuerschuld für einen Partner, der gar keinen Betrieb hat.
    nur_a = BD._mit_ring_werten({
        "gewst_messbetrag": {"wert": 500000, "zustand": "bestaetigt"},
        "gewst_hebesatz": {"wert": 400, "zustand": "bestaetigt"},
    }, 2025)
    assert "gewst_zu_zahlen_partner" not in nur_a

    # Unbestätigt bleibt unberechnet: ein vorläufiger Messbetrag darf keine feste Steuerschuld
    # in die Erklärung schreiben.
    vorlaeufig = BD._mit_ring_werten({
        "gewst_messbetrag": {"wert": 500000, "zustand": "vorlaeufig"},
        "gewst_hebesatz": {"wert": 400, "zustand": "bestaetigt"},
    }, 2025)
    assert "gewst_zu_zahlen" not in vorlaeufig


# ---------------------------------------------------------------- § 10b Abs. 1a Vermögensstock

def test_spenden_vermoegensstock_kommt_im_xml_an(bindung):
    """Ohne dieses Feld lehnte checkESt ab: "Bitte geben Sie an, in welcher Höhe die 2025
    geleisteten Spenden in den Vermögensstock einer Stiftung ... berücksichtigt werden sollen."

    E0108509, NICHT E0108607 — Letzteres ist der Vorjahres-Rest (Spenden aus Vorjahren, die
    bisher nicht berücksichtigt wurden). Der Backlog-Eintrag nannte die falsche Kennzahl; das
    fiel erst beim Lesen des Containers auf.
    """
    xml = _xml({"spenden_betrag": 50000,                # 500 EUR
                "spenden_vermoegensstock": 20000},      # davon 200 EUR in den Vermögensstock
               bindung)
    stift = ("SA", "Zuw", "Sp_erh_Verm_Stift")
    assert _pfad_im_xml(xml, stift + ("E0108405",), "500")
    assert _pfad_im_xml(xml, stift + ("E0108509",), "200")


def test_vermoegensstock_hinweis_nur_bei_echtem_betrag():
    """Der Nutzer erfährt, dass diese Angabe deklariert, aber nicht gerechnet wird.

    Das ist die Hälfte der Entscheidung, die dieses Feld überhaupt askable macht. Eine still
    gesetzte 0 hätte für jeden behauptet, er wolle nichts als Vermögensstock-Spende
    berücksichtigt sehen — für den, der eine Stiftung mit aufbaut, wären das bis zu eine
    Million Abzugsvolumen, die niemand je erfragt hat. Der Rechenkern kennt § 10b Abs. 1a
    nicht (rules.yaml nennt ihn selbst "einen eigenen Zuschnitt"), also fällt die angezeigte
    Steuer zu hoch aus — die ungefährliche Richtung, aber keine, die man verschweigt.

    Die Antwort 0 ist der Normalfall und sagt gerade, dass der Sonderfall NICHT vorliegt —
    dafür darf kein Hinweis erscheinen, sonst ist er Rauschen und wird überlesen.
    """
    import os
    import sys
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    sys.path.insert(0, os.path.join(root, "produkt", "konsistenz"))
    import check_nicht_gerechnet as CNG

    mit = CNG.nicht_gerechnete_angaben(
        {"spenden_vermoegensstock": {"wert": 20000, "zustand": "bestaetigt"}})
    assert len(mit) == 1 and mit[0]["feld_id"] == "spenden_vermoegensstock"
    assert "günstiger" in mit[0]["hinweis"]

    for still in ({"spenden_vermoegensstock": {"wert": 0, "zustand": "bestaetigt"}},
                  {"spenden_vermoegensstock": {"wert": 20000, "zustand": "vorlaeufig"}},
                  {}):
        assert CNG.nicht_gerechnete_angaben(still) == []


def test_preflight_meldet_nicht_gerechnete_angabe_als_amber():
    """Der Hinweis muss den Status anheben — GREEN hieße "hier gibt es nichts zu wissen"."""
    import os
    import sys
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    sys.path.insert(0, os.path.join(root, "produkt", "konsistenz"))
    import preflight as PF

    erg = PF.preflight({"spenden_vermoegensstock": {"wert": 20000, "zustand": "bestaetigt"}})
    assert erg["status"] == "AMBER"
    assert len(erg["hinweise_nicht_gerechnet"]) == 1

    sauber = PF.preflight({"spenden_vermoegensstock": {"wert": 0, "zustand": "bestaetigt"}})
    assert sauber["hinweise_nicht_gerechnet"] == []


# ---------------------------------------------------------------- § 33a Unterhalt an Angehörige

def test_p33a_angaben_zur_unterstuetzten_person_kommen_im_xml_an(bindung):
    """Sieben Angaben zur unterstützten Person und ihrem Haushalt (2026-08-19).

    DREI SCHICHTEN, die größte der acht Lücken:

        nur der Betrag        rc=610001002, FÜNF Beanstandungen — unterstützte Person, Adresse
                              des Haushalts, Personenzahl, Unterstützungszeitraum, Zahlungs-
                              zeitraum gemeinsam mit der Höhe
        + diese sieben        rc=610001002, SIEBEN NEUE — Einkünfte und Vermögen der Person,
                              Beiträge Dritter, Haushaltszugehörigkeit, Kindergeld-Anspruch,
                              Verwandtschaftsverhältnis, Identifikationsnummer
        + die zweite Schicht  rc=0 (s. test_p33a_zweite_schicht_kommt_im_xml_an)

    Vierzehn neue Fragen für eine Anlage, die vorher nur einen Betrag trug. Keine davon war
    ableitbar: wen jemand unterstützt, wo diese Person lebt und wovon sie lebt, weiß nur der
    Nutzer.
    """
    xml = _xml({"p33a_unterhalt_aufwendungen": 600000,
                "p33a_person_name": "Maier, Erika",
                "p33a_person_beruf_familienstand": "Rentnerin, verwitwet",
                "p33a_person_geburtsdatum": "05.05.1955",
                "p33a_haushalt_anschrift": "Musterstr. 5, 12345 Musterstadt",
                "p33a_haushalt_personenzahl": 1,
                "p33a_unterstuetzungszeitraum": "01.01-31.12",
                "p33a_zahlungszeitraum": "01.01-31.12"},
               bindung)

    wurzel = ("ESt1A_U", "Ang_HH_unt_P_Unt_Leist")
    person = wurzel + ("Ang_Unt_Pers", "Allg", "Persoenl")
    assert _pfad_im_xml(xml, person + ("E0120201",), "Maier, Erika")
    assert _pfad_im_xml(xml, person + ("E0120202",), "Rentnerin, verwitwet")
    assert _pfad_im_xml(xml, person + ("E0120203",), "05.05.1955")
    assert _pfad_im_xml(xml, wurzel + ("HH_unt_P", "E0120101"), "Musterstr. 5, 12345 Musterstadt")
    assert _pfad_im_xml(xml, wurzel + ("HH_unt_P", "E0120108"), "1")
    assert _pfad_im_xml(xml, wurzel + ("AW_U", "U_Ztr", "E0120109"), "01.01-31.12")
    assert _pfad_im_xml(xml, wurzel + ("AW_U", "U_Ztr", "E0120104"), "01.01-31.12")
    # Der Betrag daneben, schon vorher gebunden — ERiC verlangt ihn GEMEINSAM mit dem Zeitraum.
    assert _pfad_im_xml(xml, wurzel + ("AW_U", "U_Ztr", "E0120103"), "6000")


def test_p33a_zweite_schicht_kommt_im_xml_an(bindung):
    """Die sieben Angaben, die erst nach der ersten Schicht sichtbar wurden — danach rc=0.

    Fünf davon sind JaNein12: "Nein" ist eine ANTWORT und wird als "2" geschrieben, nicht
    weggelassen. Genau das prüft der Test unten, denn bei einem Ankreuzfeld wäre das Weglassen
    richtig — und diese Verwechslung hat im Repo schon dreimal Angaben verschluckt.

    DIE IDENTIFIKATIONSNUMMER war die letzte Hürde, und sie kostete eine eigene Messrunde: eine
    ausgedachte Nummer MIT korrekter Prüfziffer nach § 139b AO (03165413965) wurde von ERiC
    abgelehnt. Die IdNr fordert zusätzlich, dass die erste Ziffer nicht 0 ist und unter den
    ersten zehn Ziffern genau eine doppelt vorkommt. Das steht jetzt in
    scripts/idnr_pruefziffer.py::strukturell_gueltig — sonst läuft der nächste in dieselbe Falle,
    denn eine gültige Prüfziffer sieht aus wie ein fertiger Beweis.
    """
    xml = _xml({"p33a_unterhalt_aufwendungen": 600000,
                "p33a_person_name": "Maier, Erika",
                "p33a_person_beruf_familienstand": "Rentnerin, verwitwet",
                "p33a_person_geburtsdatum": "05.05.1955",
                "p33a_haushalt_anschrift": "Musterstr. 5, 12345 Musterstadt",
                "p33a_haushalt_personenzahl": 1,
                "p33a_unterstuetzungszeitraum": "01.01-31.12",
                "p33a_zahlungszeitraum": "01.01-31.12",
                "p33a_person_hat_einkuenfte": False,
                "p33a_person_hat_vermoegen": False,
                "p33a_weitere_person_beteiligt": False,
                "p33a_person_im_inlaendischen_haushalt": False,
                "p33a_kindergeld_anspruch": False,
                "p33a_verwandtschaftsverhaeltnis": "Mutter",
                "p33a_person_idnr": "86095742719"},
               bindung)

    person = ("ESt1A_U", "Ang_HH_unt_P_Unt_Leist", "Ang_Unt_Pers")
    assert _pfad_im_xml(xml, person + ("Allg", "Persoenl", "E0120211"), "86095742719")
    assert _pfad_im_xml(xml, person + ("Allg", "Persoenl", "E0120701"), "Mutter")
    # JaNein12: "Nein" steht als "2" im XML, es fehlt nicht.
    assert _pfad_im_xml(xml, person + ("Ek_Bez_u_P", "Allg", "E0123313"), "2")
    assert _pfad_im_xml(xml, person + ("Allg", "Verm_u_P", "E0123105"), "2")
    assert _pfad_im_xml(xml, person + ("Weit_beitr_P", "E0124801"), "2")
    assert _pfad_im_xml(xml, person + ("Allg", "U_Berecht", "E0122505"), "2")
    assert _pfad_im_xml(xml, person + ("Allg", "U_Berecht", "E0122613"), "2")


def test_beispiel_idnr_sind_eric_tauglich():
    """Jede 11-stellige Beispiel-IdNr in der Bindung muss BEIDE Regeln erfüllen.

    Beispielwerte sind die Probewerte der Feldmatrix-Sweeps. Eine, die ERiC ablehnt, erzeugt
    dort einen Befund über das Produkt, der in Wahrheit einer über den Beispielwert ist — genau
    das ist beim Bau von § 33a passiert.

    person_b_idnr ist ausgenommen und bleibt es: dessen Beispielwert verletzt die Strukturregel,
    das Feld wird aber gar nicht mehr deklariert (elster_kz: null), weil ERiC E0100082 amtlich
    ablehnt. Ein Beispielwert, der nirgends hingeht, muss nicht ERiC-tauglich sein — er wird
    hier nur benannt, damit die Ausnahme sichtbar ist statt stillschweigend.
    """
    import glob
    import os
    import sys
    import yaml
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    sys.path.insert(0, os.path.join(root, "scripts"))
    from idnr_pruefziffer import ist_gueltig, strukturell_gueltig

    schlecht = []
    for pfad in sorted(glob.glob(os.path.join(root, "produkt", "bindung", "bindung_*.yaml"))):
        with open(pfad, encoding="utf-8") as fh:
            daten = yaml.safe_load(fh) or {}
        for b in (daten.get("bindungen") or []):
            wert = b.get("beispielwert")
            if not (isinstance(wert, str) and len(wert) == 11 and wert.isdigit()):
                continue
            if b["feld_id"] == "person_b_idnr":
                continue
            if not (ist_gueltig(wert) and strukturell_gueltig(wert)):
                schlecht.append(f"{b['feld_id']}={wert}")
    assert not schlecht, ("Beispiel-IdNr, die ERiC ablehnen würde: " + ", ".join(schlecht))


# ---------------------------------------------------------------- Rundung: Abzüge zugunsten

def test_abzuege_werden_aufgerundet_einnahmen_abgerundet():
    """Krumme Cent-Beträge: Abzüge auf, Einnahmen ab — "zu Ihren Gunsten".

    Die Regel steht in der amtlichen Anleitung (anl_est1a_2025.txt:269-274) und im Code darüber,
    umgesetzt über die Liste _ABZUGS_KZ. Am 2026-08-19 fehlten dort vierzehn Abzugs-Summen; sie
    liefen auf den Default `wert // 100` und fielen damit um bis zu 99 Cent zu niedrig aus —
    klein, aber systematisch und immer zu Ungunsten der steuerpflichtigen Person.

    DIESER TEST EXISTIERT, WEIL DIE SUITE DEN NACHTRAG NICHT BEMERKT HAT. Nach dem Eintragen
    blieben alle 2246 Tests grün: die Fixturen rechnen mit glatten Beträgen, bei denen auf- und
    abrunden dasselbe Ergebnis liefern. Ein Fix, den kein Test von seinem Gegenteil unterscheiden
    kann, ist nicht belegt — hier ist die Unterscheidung.
    """
    import os
    import sys
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    sys.path.insert(0, os.path.join(root, "produkt", "mapping"))
    import est_mapping as EM

    krumm = 200050          # 2.000,50 EUR — genau zwischen zwei vollen Euro

    # Abzüge: aufrunden. Je einer aus den Gruppen, die 2026-08-19 nachgetragen wurden.
    for kz, was in (("E0108202", "Berufsausbildung"),
                    ("E0107601", "Kirchensteuer"),
                    ("E0108405", "Spenden"),
                    ("E0705701", "V+V-Werbungskosten"),
                    ("E0120103", "Unterhalt § 33a"),
                    ("E0241901", "§ 35c Sanierung")):
        assert EM._cent_nach_kz(krumm, kz) == 2001, f"{was} ({kz}) muss aufgerundet werden"

    # Die EÜR-Kennzahlen sind KEIN Rundungsfall: _cent_nach_kz fängt jedes "E60"-Kz vorher ab
    # und schreibt den Cent-Betrag exakt als Dezimalstring. Sie standen in einer ersten Fassung
    # des Nachtrags fälschlich in _ABZUGS_KZ — dieser Test hat es gefunden.
    assert EM._cent_nach_kz(krumm, "E6004901") == "2000,50"
    assert EM._cent_nach_kz(krumm, "E6002301") == "2000,50"

    # Einnahmen: abrunden. Die Gegenprobe — ohne sie könnte die Liste alles aufrunden und der
    # Test oben bliebe grün.
    for kz, was in (("E0200201", "Bruttoarbeitslohn"),
                    ("E0700201", "Mieteinnahmen"),
                    ("E1900701", "Kapitalerträge")):
        assert EM._cent_nach_kz(krumm, kz) == 2000, f"{was} ({kz}) muss abgerundet werden"

    # Und die Beträge, die einen Abzug MINDERN, gehören auf die Einnahmen-Seite: dort wäre
    # Aufrunden zu Ungunsten. Erstattete Kirchensteuer ist der klarste Fall.
    assert EM._cent_nach_kz(krumm, "E0107602") == 2000, "erstattete KiSt mindert — abrunden"
