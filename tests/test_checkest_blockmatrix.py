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
import checkest_gate as CE          # noqa: E402
import elster_xml as EX             # noqa: E402
import est_mapping                  # noqa: E402
import store as ST                  # noqa: E402

from test_checkest_durchstich import (  # noqa: E402
    _ABSENDER, _HID, _b, _fall_einzel, braucht_eric,
)


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
    ],
    "kinderbetreuung": [
        ("fam_anzahl_kinder", 1),
        ("kind_idnr", "12345678911"),
        ("kinderbetreuungskosten", 200000),
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
}

# Bloecke, die HEUTE nicht durchgehen — jeder mit der gemessenen ERiC-Meldung und einem
# BACKLOG-Verweis. Ein Eintrag hier ist eine Schuld, kein Freibrief.
BLOCKIERTE_BLOECKE = {
    "anlage_kind_instanz": (
        "'Tragen Sie bitte den Vornamen des Kindes ein (1. Anlage Kind).' — kind_vorname "
        "existiert repo-weit nicht, und fuenf weitere Kind-Felder (Geburtsjahr, "
        "Kindschaftsverhaeltnis) sind gebunden, aber nicht im gesamt-Kegel. "
        "BACKLOG anlage-kind-unvollstaendig."),
    "kinderbetreuung": (
        "dito, plus drei weitere Beanstandungen (u. a. Aufteilung der Betreuungskosten bei "
        "nicht zusammen veranlagten Eltern). Alle vier vor dem Bau einzeln aufnehmen."),
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
        assert klasse != "io_gate_nicht_geprueft", (
            f"{block}: rc={rc} — XML bricht VOR der Pruefung ab, ein leerer Fehlerpuffer "
            f"heisst hier NICHT fehlerfrei.")

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
