"""Sechs Abgabe-Luecken-Blockmatrix: fuer sechs Bereiche ohne eigenen BLOECKE-Eintrag in
tests/test_checkest_blockmatrix.py wird hier je EIN "nacktes Kernfeld" (nur das jeweilige
Kernfeld gesetzt, alle schliessenden Zusatzangaben fehlen) gegen ECHTES ERiC gehalten.

Warum es das gibt
------------------
Vault backlog/taxgraph/sechs-abgabe-luecken-blockmatrix.md, Nachmessung 2026-08-28: fuer
berufsausbildung, dhf, ep_zieladresse, gewst_zu_zahlen, p22_nr3, p33a gab es bislang nur
Wegwerf-Sonden ausserhalb des Repos (/tmp/fuenf_mehrfachvarianten.py,
/tmp/tg_gewst_clean/tests/test_gewst_hebesatz_offen_sonde.py). Neun echte ERiC-Laeufe plus
ein legitimer Schreibabbruch bestaetigten: das amtliche Programm weist alle sechs "nackten"
Faelle zurueck. Aber die Sonden sind Wegwerf-Dateien ausserhalb des Repos -- bricht die
Bindung morgen, meldet sich kein Test. Dieser Test macht die Messung stehend.

Bauart: HARTES Gate, keine Ratschen-Obergrenze
------------------------------------------------
Fail-closed, keine Toleranz: es geht um sechs BENANNTE Faelle, nicht um eine Quote. Jeder
Fall ist ein eigener Parameter und muss einzeln zurueckgewiesen werden -- eine "5 von 6
reichen"-Schwelle wuerde genau die Regression verdecken, die dieses Gate fangen soll.

Ausgangsfall: _fall_einzel() aus test_checkest_durchstich.py (Veranlagung "einzel",
Stammdaten Person A vollstaendig, kein Gewinn/Kap/VuV/Sonstige -- derselbe Boden wie der
Durchstich-Test und test_vollstaendig_pflichtfelder_voll.py). Je Fall wird GENAU das in
FAELLE unten genannte Kernfeld ergaenzt, keine weiteren Zusatzangaben -- Feldauswahl exakt
wie in der Nachmessung 2026-08-28 gemessen, dort auch die Fehlertexte im Wortlaut.

Ein rc, der NICHT GEPRUEFT bedeutet (io_gate_nicht_geprueft, hersteller_id_gesperrt,
datenartversion_unbekannt, io_reader_unerwartete_elemente -- s.
checkest_gate.klassifiziere_rc) zaehlt hier NICHT als "zurueckgewiesen": das waere die
610301200-Falle, ein leerer Fehlerpuffer, der wie "kein Befund" aussieht, aber "nicht
geprueft" heisst.

Ein Schreibabbruch (XmlFehler, der Writer blockt VOR jeder ERiC-Pruefung) ist ein DRITTER,
eigener Ausgang -- weder Zustimmung noch Ablehnung. Er zaehlt hier als "gefangen", genau wie
in test_vollstaendig_pflichtfelder_voll.py::_ohne_feld.

Ueberspringt sauber, wenn ERiC oder die Hersteller-ID fehlen (credential-freies CI, gleiches
Muster wie test_checkest_durchstich.braucht_eric). Die ID wird nie geloggt.
"""
from __future__ import annotations

import os
import re
import sys

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
for _sub in ("produkt/import", "produkt/mapping", "produkt/store",
             "produkt/traverser", "elster"):
    sys.path.insert(0, os.path.join(ROOT, _sub))

import checkest_gate as CE   # noqa: E402
import elster_xml as EX      # noqa: E402
import est_mapping           # noqa: E402
import store as ST           # noqa: E402
import traverser as TR       # noqa: E402

from test_checkest_durchstich import _ABSENDER, _HID, _b, _fall_einzel, braucht_eric  # noqa: E402

# Kernfeld je Luecke, exakt wie in der Nachmessung 2026-08-28 (Vault:
# backlog/taxgraph/sechs-abgabe-luecken-blockmatrix.md, "## Die fuenf anderen Luecken" und
# "## Die eine Luecke, die niemand aufhaelt"). Nur das Kernfeld -- keine schliessenden
# Zusatzangaben.
FAELLE = [
    ("berufsausbildung", {"berufsausbildung_aufwendungen": 200000}),
    ("dhf", {"dhf_unterkunftskosten_monat": 80000, "dhf_monate": 12}),
    ("ep_zieladresse", {"ep_arbeitstage": 220, "ep_entfernung_km": 42}),
    ("gewst_zu_zahlen", {"gewst_messbetrag": 350000}),
    ("p22_nr3", {"p22_nr3_einkuenfte": 100000}),
    ("p33a", {"p33a_unterhalt_aufwendungen": 600000}),
]

_NICHT_GEPRUEFT = {"io_gate_nicht_geprueft", "hersteller_id_gesperrt",
                   "datenartversion_unbekannt", "io_reader_unerwartete_elemente"}


def _pruefe_kernfeld(kernfeld_werte: dict) -> tuple[str, int | None, list[str]]:
    """Baut _fall_einzel() PLUS das/die Kernfeld(er), prueft echt gegen ERiC.

    Rueckgabe (ausgang, rc, texte): ausgang in {"WRITER_ABBRUCH", "GEPRUEFT"}.
    """
    s = _fall_einzel()
    for feld_id, wert in kernfeld_werte.items():
        _b(s, feld_id, wert)
    snap, _sid = ST.materialisiere(s)
    dekl = est_mapping.deklariere(snap, TR.lade_bindung())
    try:
        xml = EX.erzeuge_xml(dekl, vz=2025, hersteller_id=_HID, abgabefaehig=True, **_ABSENDER)
    except EX.XmlFehler as exc:
        return "WRITER_ABBRUCH", None, [str(exc)]
    rc, antwort = CE.validate(xml, "ESt_2025")
    texte = [" ".join(t.split()) for t in re.findall(r"<Text>(.*?)</Text>", antwort or "", re.S)]
    return "GEPRUEFT", rc, texte


@braucht_eric
@pytest.mark.parametrize("name,kernfeld_werte", FAELLE)
def test_nacktes_kernfeld_wird_von_eric_zurueckgewiesen(name, kernfeld_werte):
    """Fail-closed: rot, wenn ein 'nacktes Kernfeld' (keine schliessenden Zusatzangaben) vom
    amtlichen Programm NICHT zurueckgewiesen wird -- das waere genau die Luecke, die
    backlog/taxgraph/sechs-abgabe-luecken-blockmatrix.md fuer diesen Bereich benennt."""
    ausgang, rc, texte = _pruefe_kernfeld(kernfeld_werte)
    if ausgang == "WRITER_ABBRUCH":
        return  # Writer selbst faengt es fail-closed ab -- kein Blindspot, s. Docstring oben

    klasse = CE.klassifiziere_rc(rc)
    assert klasse not in _NICHT_GEPRUEFT, (
        f"[{name}] rc={rc} [{klasse}]: NICHT geprueft, nicht 'kein Befund' -- die "
        f"610301200-Falle. Details in eric.log.")
    assert rc != CE.RC_OK, (
        f"[{name}] rc=0 (keine Beanstandung) fuer ein 'nacktes Kernfeld' ohne die "
        f"schliessenden Zusatzangaben -- die Luecke ist nicht mehr abgesichert. "
        f"Gesetzte Felder: {kernfeld_werte}")
    assert texte, (
        f"[{name}] rc={rc} != RC_OK, aber kein einziger Fehlertext im Rueckgabepuffer -- "
        f"Antwort pruefen, bevor das als 'zurueckgewiesen' zaehlt.")
