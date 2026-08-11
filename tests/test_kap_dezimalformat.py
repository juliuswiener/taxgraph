"""Anlage KAP: die Steuerabzugsbetraege brauchen "N,NN", die Ertraege ganze Euro.

Das XSD unterscheidet zwei Zahlentypen im selben Formular, und der Vordruck zeigt es auch —
Seite 3 fuehrt bei den Steuerabzugsbetraegen die Spalten "EUR | Ct", Zeile 7 (Kapitalertraege)
nur "EUR":

    E1904701 KapESt   Zeile 37    DezimalzahlNichtNeg...MinNK2_MaxNK2   ->  "1250,00"
    E1904901 SolZ     Zeile 38    dito                                  ->  "68,75"
    E1904801 KiSt     Zeile 39    dito                                  ->  "112,50"
    E1905101 q        Zeile 41    Dezimalzahl...MinNK2_MaxNK2           ->  "1000,00"

    E1900701 Ertraege Zeile 7     GanzzahlNichtNeg...                   ->  "5000"
    E1901401 SparerPB Zeile 16    dito                                  ->  "1000"

`est_mapping._cent_nach_kz` schreibt per Default den gerundeten Integer; nur Kz mit
E60-Praefix oder in `_KOMMA_OHNE_E60_KZ` bekommen das Komma-Format. Fehlt ein Kz in der Liste,
steht "1000" statt "1000,00" im XML und checkESt lehnt ab:

    Feld '$/KAP[1]/St_Abz_Betr_Inl_u_Inv_Ert[1]/E1905101[1]$':
    Geldbetraege muessen vom Format '0,00' sein

Genau das passierte beim Stufe-3-Bau (9801efc): E1905101 wurde vergessen, waehrend Stufe 2
(ac2989a) die drei Geschwister-Kz korrekt eingetragen hatte. Gefunden 2026-08-11 beim
Block-Matrix-Sweep — die Einzelfeld-Tests des Baus liefen daran vorbei, weil sie die Rechnung
prueften und nicht die Formatierung im XML.

Dieser Test prueft das Format DIREKT am XML-Text, nicht ueber rc: ein Formatfehler kann von
einer vorgelagerten Block-Beanstandung verdeckt werden (bei den p36_*-Feldern war das so), und
dann sagt rc nichts ueber das Format aus.
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
    _ABSENDER, _HID, _b, braucht_eric,
)
from test_checkest_feldmatrix import _mit  # noqa: E402

# Kz -> (Feld, Store-CENT, erwarteter XML-Text). Die Cent-Reste sind Absicht: bei glatten
# Betraegen waere "1250" von "1250,00" nur am Komma zu unterscheiden, ein abgeschnittener
# Rest fiele nicht auf.
KOMMA_KZ = [
    ("E1904701", "p36_kapitalertragsteuer", 125000, "1250,00"),
    ("E1904901", "p36_kapitalertragsteuer_solz", 6875, "68,75"),
    ("E1904801", "p36_kapitalertragsteuer_kist", 11250, "112,50"),
    ("E1905101", "kap_q_auslaendische_steuer", 100050, "1000,50"),
]

GANZZAHL_KZ = [
    ("E1900701", "kap_kapitalertraege", 500000, "5000"),
]


def _setz(store, feld, wert):
    """Setzt ein Feld, auch wenn die Fixtur es schon fuehrt.

    Der Store ist fail-closed gegen Ueberschreiben eines aktiven Events (store.py:232), und
    die Ratschen-Fixtur belegt kap_kapitalertraege bereits mit 0 — `_b()` allein kracht dort.
    """
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


def _xml(paare):
    store = _mit(paare[0][0], paare[0][1])
    for feld, wert in paare[1:]:
        _setz(store, feld, wert)
    store = dict(store)
    store["scheibe"] = "gesamt"
    bindung = API._scheibe_bindung(store)
    felder, sid = ST.materialisiere(store)
    felder = API._mit_ring_werten(felder, 2025)
    return EX.erzeuge_xml(est_mapping.deklariere(felder, bindung, snapshot_id=sid),
                          vz=2025, hersteller_id=_HID, abgabefaehig=True, **_ABSENDER)


@pytest.mark.parametrize("kz,feld,cent,erwartet", KOMMA_KZ)
def test_steuerabzugsbetrag_wird_mit_zwei_nachkommastellen_geschrieben(kz, feld, cent, erwartet):
    """Braucht kein ERiC — geprueft wird der XML-Text, nicht die Plausibilitaet.

    Absichtlich am Text und nicht ueber rc: eine vorgelagerte Block-Beanstandung kann den
    Formatfehler verdecken. Bei den p36_*-Feldern war genau das der Fall — sie fielen im
    Matrix-Sweep mit "keine zugehoerigen Kapitalertraege erklaert" durch, und ob ihr Format
    stimmte, sagte dieses rc nicht.
    """
    xml = _xml([(feld, cent), ("kap_kapitalertraege", 500000)])
    treffer = re.search(f"<{kz}>([^<]*)</{kz}>", xml)
    assert treffer, f"{kz} ({feld}) fehlt im XML — Bindung oder Scheibe pruefen"
    assert treffer.group(1) == erwartet, (
        f"{kz} ({feld}) steht als {treffer.group(1)!r} im XML, erwartet {erwartet!r}. "
        f"Das Kz gehoert in est_mapping._KOMMA_OHNE_E60_KZ; ohne den Eintrag schreibt "
        f"_cent_nach_kz() den rohen Integer und checkESt lehnt ab: 'Geldbetraege muessen "
        f"vom Format 0,00 sein'.")


@pytest.mark.parametrize("kz,feld,cent,erwartet", GANZZAHL_KZ)
def test_ertragsfeld_bleibt_ganzzahlig(kz, feld, cent, erwartet):
    """Gegenprobe: nicht jedes KAP-Kz will Nachkommastellen.

    Ohne diese Haelfte waere ein Bau gruen, der ALLE Kz ins Komma-Format schiebt — das XSD
    verlangt fuer die Ertragszeilen aber GanzzahlNichtNegOhneFuehrNull.
    """
    xml = _xml([(feld, cent)])
    treffer = re.search(f"<{kz}>([^<]*)</{kz}>", xml)
    assert treffer, f"{kz} ({feld}) fehlt im XML"
    assert treffer.group(1) == erwartet, (
        f"{kz} ({feld}) steht als {treffer.group(1)!r} im XML, erwartet {erwartet!r} "
        f"(GanzzahlNichtNegOhneFuehrNull laut XSD — kein Komma-Format).")


@braucht_eric
def test_q_mit_kapitalertraegen_ist_einreichbar():
    """Scharf: q im vollstaendigen Block erreicht rc=0.

    Vor dem Format-Fix stand hier 'Geldbetraege muessen vom Format 0,00 sein' — eine
    Erklaerung mit angerechneter auslaendischer Steuer war nicht einreichbar.
    """
    xml = _xml([("kap_q_auslaendische_steuer", 100000), ("kap_kapitalertraege", 500000)])
    rc, antwort = CE.validate(xml, "ESt_2025")
    texte = [" ".join(t.split())
             for t in re.findall(r"<Text>(.*?)</Text>", antwort or "", re.S)]
    assert rc == CE.RC_OK, f"rc={rc}: {texte[:2]}"
