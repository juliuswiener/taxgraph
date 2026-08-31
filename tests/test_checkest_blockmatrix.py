"""Block-Matrix gegen die amtliche Pruefung: ist ein BLOCK als Ganzes einreichbar?

Nachfolger von test_checkest_feldmatrix.py, das nur Felder abdeckt, die ALLEIN eine sinnvolle
Erklaerung ergeben. Viele Felder ergeben das nicht — sie brauchen ihren Block:

    kind_kv ohne Anlage Kind        -> "Tragen Sie bitte den Vornamen des Kindes ein"
    vv_einnahmen ohne Objekt        -> "'Laufende_Nummer_V': Das Feld muss angegeben werden"
    p35c ohne Gebaeude              -> "muss mindestens der Standort des Wohngebaeudes"
    gewst_hebesatz ohne Messbetrag  -> "der Hebesatz wurde angegeben, die zu zahlende ... jedoch nicht"

Die Einzelfeld-Matrix wertet solche Meldungen als Rauschen und klammert die Felder aus. Damit
bleibt aber unpruefbar, ob der Block als Ganzes ueberhaupt durchgeht — und genau dort saß der
Fund, der diese Datei ausgeloest hat:

    fam_anzahl_kinder=1 allein   rc=0          Kinderfreibetrag laeuft ohne Anlage Kind
    + kind_idnr                  rc=610001002  Vorname fehlt — und `kind_vorname` gibt es
                                               repo-weit gar nicht

Jede kindbezogene Abzugsposition ausser dem Freibetrag ist dadurch heute uneinreichbar
(BACKLOG anlage-kind-unvollstaendig).

AUFBAU: je Block ein Eintrag mit den Feldern, die ihn MINIMAL VOLLSTAENDIG machen. Ein Block,
der durchlaeuft, ist gruen. Ein Block, der es nicht tut, steht mit der gemessenen ERiC-Meldung
in BLOCKIERTE_BLOECKE — als Schuld, nicht als Freibrief: der Test schlaegt fehl, wenn ein
eingetragener Block ploetzlich sauber durchlaeuft, damit eine geschlossene Luecke nicht als
Dauer-Ausnahme stehen bleibt (dieselbe Mechanik wie in der Einzelfeld-Matrix).

WAS SIE NICHT PRUEFT: ob die Zahlen stimmen. checkESt prueft Plausibilitaet, nicht Richtigkeit.
"""

from __future__ import annotations

import os
import re
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("elster", "produkt/haut", "produkt/import", "produkt/mapping",
            "produkt/store", "produkt/traverser"):
    sys.path.insert(0, os.path.join(ROOT, sub))
sys.path.insert(0, HERE)

import api as API                   # noqa: E402
import audit                        # noqa: E402
import checkest_gate as CE          # noqa: E402
import elster_xml as EX             # noqa: E402
import est_mapping                  # noqa: E402
import store as ST                  # noqa: E402

from test_checkest_durchstich import (  # noqa: E402
    _ABSENDER, _HID, _b, _fall_einzel, braucht_eric,
)


@pytest.fixture(autouse=True)
def _isoliert(tmp_path, monkeypatch):
    """Faelle in tmp_path — sonst kollidieren Wiederholungslaeufe mit ihren eigenen Altfaellen."""
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))


def _setz(store, feld, wert):
    """Setzt ein Feld, auch wenn die Ratschen-Fixtur es schon fuehrt (fail-closed gegen
    Ueberschreiben, store.py:232)."""
    aktiv = None
    for e in reversed(store.get("events") or []):
        if e.get("feld_id") == feld and not e.get("ersetzt_durch"):
            aktiv = e["event_id"]
            break
    if aktiv:
        ST.append_event(store, feld_id=feld, wert=wert, zustand="bestaetigt",
                        herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft",
                                  "haftung": "nutzer"},
                        schreiber="laie",
                        signal={"signal_1": {"typ": "laie_eingabe"},
                                "signal_2": "laie_bestaetigt"},
                        ersetzt=aktiv)
    else:
        _b(store, feld, wert)


def _block_scharf(felder):
    """Ratschen-Basisfall + alle Felder des Blocks -> Abgabe-XML -> amtliches Plugin.

    Ein Writer-Abbruch wird als eigenes Ergebnis zurueckgegeben, nicht als Exception: "das XML
    entsteht gar nicht" ist ein anderer Befund als "ERiC beanstandet", und beide sollen in der
    Assertion unterscheidbar sein.
    """
    store = _fall_einzel()
    for feld, wert in felder:
        _setz(store, feld, wert)
    store = dict(store)
    store["scheibe"] = "gesamt"
    bindung = API._scheibe_bindung(store)
    snap, sid = ST.materialisiere(store)
    snap = API._mit_ring_werten(snap, 2025)
    try:
        xml = EX.erzeuge_xml(est_mapping.deklariere(snap, bindung, snapshot_id=sid),
                             vz=2025, hersteller_id=_HID, abgabefaehig=True, **_ABSENDER)
    except Exception as ex:
        return None, [f"WRITER-ABBRUCH: {type(ex).__name__}: {str(ex)[:200]}"]
    rc, antwort = CE.validate(xml, "ESt_2025")
    return rc, [" ".join(t.split())
                for t in re.findall(r"<Text>(.*?)</Text>", antwort or "", re.S)]


# Block -> Felder, die ihn minimal vollstaendig machen sollen. Werte bewusst klein und
# unauffaellig: geprueft wird die STRUKTUR des Blocks, nicht ob ein Grenzwert kippt.
BLOECKE = {
    "kinderfreibetrag_ohne_anlage": [
        ("fam_anzahl_kinder", 1),
    ],
    "anlage_kind_instanz": [
        ("fam_anzahl_kinder", 1),
        ("kind_idnr", "12345678911"),
        ("kind_vorname", "Anna"),
        ("kind_geburtsdatum", "15.03.2015"),
        ("kind_familienkasse", "Familienkasse Bayern Nord"),
        ("kind_wohnsitz_inland_zeitraum", "01.01-31.12"),
        ("kind_kindschaftsverhaeltnis_a", "1"),
        ("kind_kindschaftsverh_zeitraum_a", "01.01-31.12"),
        ("kind_anderer_elternteil_name", "Michael Beispiel"),
        ("kind_anderer_elternteil_geburtsdatum", "01.01.1985"),
        ("kind_anderer_elternteil_kindschaftsverhaeltnis", "1"),
        ("kind_anderer_elternteil_zeitraum", "01.01-31.12"),
    ],
    "kinderbetreuung": [
        ("fam_anzahl_kinder", 1),
        ("kind_idnr", "12345678911"),
        ("kind_vorname", "Anna"),
        ("kind_geburtsdatum", "15.03.2015"),
        ("kind_familienkasse", "Familienkasse Bayern Nord"),
        ("kind_wohnsitz_inland_zeitraum", "01.01-31.12"),
        ("kind_kindschaftsverhaeltnis_a", "1"),
        ("kind_kindschaftsverh_zeitraum_a", "01.01-31.12"),
        ("kind_anderer_elternteil_name", "Michael Beispiel"),
        ("kind_anderer_elternteil_geburtsdatum", "01.01.1985"),
        ("kind_anderer_elternteil_kindschaftsverhaeltnis", "1"),
        ("kind_anderer_elternteil_zeitraum", "01.01-31.12"),
        ("kind_unter_14_haushaltszugehoerig", True),
        ("kinderbetreuungskosten", 200000),
        ("kind_betreuung_dienstleister", "Kindertagesstätte Sonnenschein, Musterstr. 1, 12345 Musterstadt"),
        ("kind_betreuung_zeitraum", "01.01-31.12"),
        ("kind_betreuung_eigenanteil", 200000),
        ("kind_betreuung_kein_gemeinsamer_haushalt_zeitraum", "01.01-31.12"),
        ("kind_betreuung_haushaltszugehoerigkeit_zeitraum", "01.01-31.12"),
        ("kind_betreuung_einzelbetrag", 200000),
        ("kind_betreuung_eigenanteil_betrag", 200000),
        ("kind_betreuung_eigenanteil_zeitraum", "01.01-31.12"),
    ],
    "p35a_handwerker": [
        ("hh_handwerker_betrag", 300000),
        ("hh_handwerker_art", "Malerarbeiten"),
    ],
    "kap_mit_steuerabzug": [
        ("kap_kapitalertraege", 500000),
        ("p36_kapitalertragsteuer", 125000),
        ("p36_kapitalertragsteuer_solz", 6875),
    ],
    "kap_mit_auslandssteuer": [
        ("kap_kapitalertraege", 500000),
        ("kap_q_auslaendische_steuer", 100000),
    ],
    "verpflegung": [
        ("tage_24h", 10),
        ("vpf_fruehstuecke_gestellt_anzahl", 5),
    ],
    # Anlage U, Geberseite (§ 10 Abs. 1a Nr. 1). Die drei Kz des Containers SO/Unt_Leist sind
    # INEINANDER geschachtelt, keine Summanden: 1.500 sind Teil der 1.800, die Teil der 12.000
    # sind. Bis 2026-08-14 fehlte die dritte (E0300829, Krankengeld-Anspruch) in der Bindung —
    # checkESt beanstandete "Zeile 7", obwohl Zeile 5 gefuellt war.
    "realsplitting": [
        ("realsplitting_unterhaltsleistungen", 1200000),
        ("realsplitting_empfaenger_kv_pv", 180000),
        ("realsplitting_empfaenger_kv_krankengeld", 150000),
    ],
    # Pflege-Pauschbetrag (§ 33b Abs. 6). ERiC gab seine Anforderungen in DREI Schichten preis:
    # erst Wohnsitz+Helferzahl, nach deren Ergaenzung IdNr+Personenangaben+"durch wen". Der
    # Pflegegrad allein — so stand es bis 2026-08-15 in der Bindung — war nie einreichbar.
    # Beide Ausloeser werden gefahren, weil sie getrennt in die Anlage fuehren (Staffel bzw.
    # Merkzeichen H) und je fuer sich den Block ausloesen koennen.
    # Anlage V (§ 21). Gemessen 2026-08-16 in FUENF Schichten: Laufende_Nummer_V ->
    # Lage+Wohneinheit+Umlagen -> drei Nutzungs-Flags+Einnahmensumme -> WK-Summe+Ueberschuss ->
    # Zurechnung. Jede Schicht kam erst zum Vorschein, nachdem die vorige beantwortet war.
    "anlage_v_vermietung": [
        ("vv_einnahmen", 1200000),
        ("vv_wohnzwecke", True), ("vv_auf_dauer", True),
        ("vv_objekt_strasse", "Mietweg 7"), ("vv_objekt_plz", "12345"),
        ("vv_objekt_ort", "Musterstadt"),
        ("vv_wohneinheit_bezeichnung", "1. OG links"),
        ("vv_nebenkosten_nicht_vereinbart", True),
        ("vv_nutzung_ferienwohnung", False), ("vv_nutzung_an_angehoerige", False),
        ("vv_nutzung_kurzfristig", False),
        ("vv_gebaeude_afa", 200000), ("vv_schuldzinsen", 0),
        ("vv_erhaltungsaufwand", 0), ("vv_sonstige_wk", 0),
    ],
    "pflege_pauschbetrag": [
        ("rentner_pflegegrad", 4),
        ("rentner_gepflegter_wohnsitz_inland", True),
        ("rentner_pflege_weitere_personen", 0),
        ("rentner_gepflegter_idnr", "12345678911"),
        ("rentner_gepflegter_angaben",
         "Maria Muster, Musterweg 3, 12345 Musterstadt, meine Mutter"),
        ("rentner_pflege_durch", "1"),
    ],
    # Anlage Energetische Maßnahmen (§ 35c). Gemessen 2026-08-16 in DREI Schichten: sieben
    # Beanstandungen -> nach den Formalien noch eine (fehlende Einzelaufwendungen) -> rc=0.
    # Der Betrag steht zweimal im XML: als Summe (E0241901) und in der Zeile der Maßnahmenart
    # (hier Heizung, E0241501) — das Formular verlangt beides.
    "p35c_anlage_energetische_massnahmen": [
        ("p35c_sanierungsaufwendungen", 2000000),
        ("p35c_keine_doppelfoerderung", True),
        ("p35c_objekt_strasse", "Musterweg 3"),
        ("p35c_objekt_plz_ort", "12345 Musterstadt"),
        ("p35c_gebaeude_herstellungsbeginn", "01.06.1995"),
        ("p35c_baubeginn_massnahme", "15.03.2025"),
        ("p35c_gesamtflaeche_qm", 120),
        ("p35c_eigene_wohnflaeche_qm", 120),
        ("p35c_bereits_ermaessigung_frueher", False),
        ("p35c_massnahme_art", "heizung"),
    ],
    "pflege_merkzeichen_h": [
        ("rentner_gepflegter_hilflos", True),
        ("rentner_gepflegter_wohnsitz_inland", True),
        ("rentner_pflege_weitere_personen", 0),
        ("rentner_gepflegter_idnr", "12345678911"),
        ("rentner_gepflegter_angaben",
         "Maria Muster, Musterweg 3, 12345 Musterstadt, meine Mutter"),
        ("rentner_pflege_durch", "1"),
    ],
}

# Bloecke, die HEUTE nicht durchgehen — jeder mit der gemessenen ERiC-Meldung und einem
# BACKLOG-Verweis. Ein Eintrag hier ist eine Schuld, kein Freibrief.
BLOCKIERTE_BLOECKE = {
    # anlage_kind_instanz + kinderbetreuung: 2026-08-12 auf rc=0 gemessen, aus BLOCKIERTE_BLOECKE
    # entfernt. Beide brauchten K_Verh_and_P/Ang_Pers (E0501103/104/106/903, "anderer Elternteil"
    # statt Ehefrau-Kindschaftsverhaeltnis — checkESt lehnt K_Verh_B bei Einzelveranlagung ab)
    # sowie bei kinderbetreuung zusaetzlich die KBK_72569777_CType-Einzelposten (Art/Einz,
    # Elt_k_ZV/Kosten/Einz) neben den bereits vorhandenen Summenfeldern. BACKLOG
    # anlage-kind-unvollstaendig.
}


@braucht_eric
@pytest.mark.parametrize("block", sorted(BLOECKE))
def test_block_ist_amtlich_plausibel(block):
    """Ein minimal vollstaendiger Block muss einreichbar sein.

    Faellt ein Block hier NEU durch, fehlt eine Angabe, die das Finanzamt zu diesem Block
    verlangt — dieselbe Klasse wie der KAP-Antragsgrund oder die § 35a-Einzelaufstellung.
    """
    rc, texte = _block_scharf(BLOECKE[block])

    if rc is not None:
        klasse = CE.klassifiziere_rc(rc)
        assert klasse not in CE.NICHT_GEPRUEFT_KLASSEN, (
            f"{block}: rc={rc} [{klasse}] — XML bricht VOR der Pruefung ab, ein leerer "
            f"Fehlerpuffer heisst hier NICHT fehlerfrei.")

    if block in BLOCKIERTE_BLOECKE:
        assert rc != CE.RC_OK, (
            f"{block} ist als blockiert eingetragen, laeuft aber sauber durch (rc={rc}). "
            f"Wenn die Luecke geschlossen wurde: Eintrag aus BLOCKIERTE_BLOECKE entfernen — "
            f"sonst deckt der Test die Regression nicht mehr ab.")
        return

    assert rc == CE.RC_OK, (
        f"Block {block!r} ist nicht einreichbar (rc={rc}).\n"
        + "\n".join(f"   - {t}" for t in texte[:5])
        + f"\nEntweder fehlt eine Angabe des Blocks (dann bauen) oder es ist eine bewusste "
          f"Luecke (dann mit gemessener Meldung in BLOCKIERTE_BLOECKE eintragen).")


def test_blockierte_bloecke_sind_begruendet():
    """Jeder Freistellungs-Eintrag nennt Messung und Grund — kein stilles Ausklammern.

    Braucht kein ERiC.
    """
    for block, grund in BLOCKIERTE_BLOECKE.items():
        assert block in BLOECKE, (
            f"{block} steht in BLOCKIERTE_BLOECKE, wird aber gar nicht gefahren — toter "
            f"Eintrag, der Abdeckung vortaeuscht.")
        assert "BACKLOG" in grund or "dito" in grund, (
            f"{block}: Begruendung ohne BACKLOG-Verweis — dann findet den Punkt spaeter niemand.")


def test_kind_vorname_ist_deklarierbar():
    """E0500107 (Anlage Kind Zeile 1, Vorname) muss auf 'gesamt' gebunden und im Kegel sein.

    Braucht kein ERiC. Das Feld fehlte bis 2026-08-11 repo-weit, wodurch checkESt jede
    Kind-Instanz ablehnte ("Tragen Sie bitte den Vornamen des Kindes ein") und damit jede
    kindbezogene Abzugsposition ausser dem Freibetrag uneinreichbar war.

    Geprueft wird die BINDUNG, nicht das erzeugte XML: ohne den Kegel-Eintrag filtert
    _scheibe_bindung() das Feld aus der Deklaration, egal was der Nutzer eintraegt —
    dieselbe Naht wie bei E0205508 (Verpflegungskuerzung, 3ce178c).
    """
    API.fall_anlegen({"fall_id": "gate_kind_vorname", "scheibe": "gesamt",
                      "veranlagungszeitraum": 2025})
    bindung = API._scheibe_bindung(API.lade_fall("gate_kind_vorname"))
    kz = {v.get("elster_kz") for v in bindung.values() if isinstance(v, dict)}
    assert "E0500107" in kz, (
        "E0500107 (Vorname des Kindes) ist auf 'gesamt' nicht deklarierbar — dann lehnt "
        "checkESt jede Kind-Instanz ab und alle kindbezogenen Abzuege sind uneinreichbar.")


def test_kind_formalien_sind_deklarierbar():
    """E0500701/E0500706/E0500703 (Geburtsdatum, Familienkasse, Wohnsitz-Inland-Zeitraum)
    muessen auf 'gesamt' gebunden und im Kegel sein — dieselbe Naht wie kind_vorname oben.

    Braucht kein ERiC. Bis 2026-08-12 fehlte E0500706/E0500703 repo-weit und
    kind_geburtsjahr (int, ohne Kz) konnte E0500701 nicht bedienen, weshalb checkESt
    "Vorname, Geburtsdatum und Familienkasse wurden nicht gemeinsam angegeben" meldete.
    """
    API.fall_anlegen({"fall_id": "gate_kind_formalien", "scheibe": "gesamt",
                      "veranlagungszeitraum": 2025})
    bindung = API._scheibe_bindung(API.lade_fall("gate_kind_formalien"))
    kz = {v.get("elster_kz") for v in bindung.values() if isinstance(v, dict)}
    assert "E0500701" in kz, (
        "E0500701 (Geburtsdatum des Kindes) ist auf 'gesamt' nicht deklarierbar.")
    assert "E0500706" in kz, (
        "E0500706 (Familienkasse) ist auf 'gesamt' nicht deklarierbar.")
    assert "E0500703" in kz, (
        "E0500703 (Zeitraum Wohnsitz Inland) ist auf 'gesamt' nicht deklarierbar.")
    assert "E0501103" in kz, (
        "E0501103 (Name anderer Elternteil) ist auf 'gesamt' nicht deklarierbar.")
    assert "E0501104" in kz, (
        "E0501104 (Geburtsdatum anderer Elternteil) ist auf 'gesamt' nicht deklarierbar.")
    assert "E0501106" in kz, (
        "E0501106 (Kindschaftsverhaeltnis anderer Elternteil) ist auf 'gesamt' nicht deklarierbar.")
    assert "E0501903" in kz, (
        "E0501903 (Zeitraum anderer Elternteil) ist auf 'gesamt' nicht deklarierbar.")
    assert "E0506101" in kz, (
        "E0506101 (Dienstleister Kinderbetreuung) ist auf 'gesamt' nicht deklarierbar.")
    assert "E0506103" in kz, (
        "E0506103 (Zeitraum Kinderbetreuung) ist auf 'gesamt' nicht deklarierbar.")
    assert "E0506604" in kz, (
        "E0506604 (Eigenanteil Kinderbetreuung) ist auf 'gesamt' nicht deklarierbar.")
    assert "E0505201" in kz, (
        "E0505201 (kein gemeinsamer Haushalt Zeitraum) ist auf 'gesamt' nicht deklarierbar.")
    assert "E0505202" in kz, (
        "E0505202 (Haushaltszugehoerigkeit Zeitraum) ist auf 'gesamt' nicht deklarierbar.")
    assert "E0506104" in kz, (
        "E0506104 (Einzelbetrag Dienstleister) ist auf 'gesamt' nicht deklarierbar.")
    assert "E0506605" in kz, (
        "E0506605 (Eigenanteil Einzelbetrag) ist auf 'gesamt' nicht deklarierbar.")
    assert "E0506606" in kz, (
        "E0506606 (Eigenanteil Einzel-Zeitraum) ist auf 'gesamt' nicht deklarierbar.")


@braucht_eric
def test_kinderfreibetrag_laeuft_ohne_anlage_kind():
    """Die Abgrenzung, die den Kind-Befund erst bewertbar macht.

    Ohne diesen Test saehe es so aus, als sei jeder Fall mit Kindern uneinreichbar. Tatsaechlich
    ist es nur jeder mit einer Kind-INSTANZ: der reine Kinderfreibetrag kommt ohne Anlage Kind
    aus. Das ist der Unterschied zwischen "Kinder gehen nicht" und "kindbezogene Abzuege gehen
    nicht".
    """
    rc, texte = _block_scharf(BLOECKE["kinderfreibetrag_ohne_anlage"])
    assert rc == CE.RC_OK, (
        f"Der reine Kinderfreibetrag muss ohne Anlage Kind einreichbar sein, rc={rc}: "
        f"{texte[:3]}")
