"""Umfangs-Pruefung am Writer: ein abgabefaehiges XML entsteht nur mit allen Pflichtfeldern.

`erzeuge_xml` las bis hierher nur `eingaben_konsistent` — ob sich vorhandene Angaben
widersprechen. Ein leerer Store widerspricht sich nicht und galt deshalb als konsistent. Ob das
Noetige DA ist, sagt ein eigener Schluessel, `pflichtfelder_vollstaendig` (est_mapping.PFLICHTFELDER,
Julius-Entscheidung 2026-08-30, s. test_vollstaendig_pflichtfelder_voll.py). Den schrieb
`deklariere()`, gelesen hat ihn keine Produktstelle.

Gemessen 2026-10-01 (ERiC 44.2.4.0, checkESt ESt_2025), jeweils VOR dem Fix:

- Leerer Store: rc=610001002 mit 14 Texten, darunter
    "Es wurde kein Geburtsdatum angegeben (steuerpflichtige Person / Ehemann / Person A)."
    "Religion nicht angegeben oder kein gültiger Wert (steuerpflichtige Person / Ehemann / Person A)."
    "Bitte geben Sie den Namen und Vornamen an (steuerpflichtige Person / Ehemann / Person A)."
    "Bitte geben Sie die vollständige derzeitige Adresse an. [...]"
- Name, Anschrift, Bankentscheidung und Steuernummer da, Geburtsdatum und Konfession nicht:
  der Writer schrieb die Datei (1782 Zeichen), ERiC lehnte mit rc=610001002 und genau den
  ersten beiden Texten oben ab. Das Vorsatz-Seitengate deckt diese zwei nicht: es leitet nur
  Name, Strasse, PLZ und Ort aus der Deklaration ab, Geburtsdatum und Konfession haben dort
  keinen Platz.

Der Weg ist der oeffentliche Schreibweg `schreibe_xml` mit einem echten Store, kein
handgebautes Ergebnis.
"""
from __future__ import annotations

import os
import sys

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
for _sub in ("produkt/eingang", "produkt/mapping", "produkt/store", "produkt/traverser"):
    sys.path.insert(0, os.path.join(ROOT, _sub))

import elster_xml as EX      # noqa: E402
import est_mapping as EM     # noqa: E402
import store as ST           # noqa: E402
import traverser as TR       # noqa: E402

HID = "74931"                # ERiC-Test-Hersteller-ID aus dem amtlichen Beispiel-XML

# Was das Vorsatz-Seitengate verlangt: Name, Anschrift, Bankentscheidung, Steuernummer.
SEITENGATE = [("stammdaten_nachname", "Maier"), ("stammdaten_vorname", "Hans"),
              ("stammdaten_strasse", "Musterstr."), ("stammdaten_hausnummer", "5"),
              ("stammdaten_plz", "55555"), ("stammdaten_wohnort", "Musterort"),
              ("stammdaten_keine_bankverbindung", True),
              ("stammdaten_steuernummer", "9181081508155")]
# Die zwei Pflichtfelder, die das Seitengate nicht sieht.
REST = [("stammdaten_geburtsdatum", "05.05.1955"), ("kist_konfession", "keine")]


def _deklariere(paare):
    s = ST.leerer_store(2025, fall_id="pflicht_writer")
    for feld_id, wert in paare:
        ST.append_event(store=s, feld_id=feld_id, wert=wert, zustand="bestaetigt",
                        herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft",
                                  "haftung": "nutzer"},
                        schreiber="ui:laie", signal={"signal_1": None, "signal_2": "x"},
                        ts="2026-10-01T00:00:00Z")
    felder, sid = ST.materialisiere(s)
    return EM.deklariere(felder, TR.lade_bindung(), snapshot_id=sid), felder


def _schreibe(paare, tmp_path):
    result, felder = _deklariere(paare)
    EX.schreibe_xml(result, str(tmp_path / "submission.xml"), vz=2025, hersteller_id=HID,
                    abgabefaehig=True, snapshot=felder)
    return result


def test_leerer_store_schreibt_kein_abgabefaehiges_xml(tmp_path):
    """Leerer Store: konsistent, aber alle sieben Pflichtfelder fehlen."""
    with pytest.raises(EX.XmlFehler, match="pflichtfelder_vollstaendig") as fehler:
        _schreibe([], tmp_path)
    for feld_id in ("stammdaten_nachname", "stammdaten_vorname", "stammdaten_geburtsdatum",
                    "stammdaten_strasse", "stammdaten_plz", "stammdaten_wohnort",
                    "kist_konfession"):
        assert feld_id in str(fehler.value), f"{feld_id} fehlt in der Meldung: {fehler.value}"
    assert not (tmp_path / "submission.xml").exists()


def test_seitengate_voll_aber_geburtsdatum_und_konfession_fehlen(tmp_path):
    """Der Fall, den vorher erst ERiC fing: alles fuer den Vorsatz da, zwei Pflichtfelder nicht."""
    result, _ = _deklariere(SEITENGATE)
    assert result["eingaben_konsistent"] is True, result["unvollstaendig"]
    assert [e["feld_id"] for e in result["pflichtfelder_luecken"]] == [
        "stammdaten_geburtsdatum", "kist_konfession"]
    with pytest.raises(EX.XmlFehler, match="pflichtfelder_vollstaendig") as fehler:
        _schreibe(SEITENGATE, tmp_path)
    assert "['stammdaten_geburtsdatum', 'kist_konfession']" in str(fehler.value)
    assert not (tmp_path / "submission.xml").exists()


def test_gegenprobe_vollstaendiger_store_schreibt_das_xml(tmp_path):
    """Ohne diese Probe waere eine Pruefung, die alles ablehnt, ebenfalls gruen."""
    result = _schreibe(SEITENGATE + REST, tmp_path)
    assert result["pflichtfelder_vollstaendig"] is True, result["pflichtfelder_luecken"]
    assert "<Vorsatz>" in (tmp_path / "submission.xml").read_text(encoding="utf-8")
