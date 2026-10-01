"""ROTES GATE: `pflichtfelder_luecken` wird geschrieben und von keiner Produktstelle gelesen.

Warum es das gibt
-----------------
`EM.deklariere()` liefert 12 Schluessel. Fuenf davon liest keine Produktstelle:
`basis_snapshot`, `vollstaendig`, `pflichtfelder_luecken`, `pflichtfelder_vollstaendig`
und `dokumentiert`. `pflichtfelder_luecken` ist der einzige mit einer eigenen,
unersetzlichen Aussage: er kennt die FELD-ABWESENHEIT gegen die gepflegte
PFLICHTFELDER-Liste, die die Hauptschleife in `deklariere()` per Konstruktion nie
sehen kann (`produkt/mapping/est_mapping.py:546`).

Gemessen am 2026-10-01 (HEAD `1065e25`, Korpus 192 Fallakten):

* 112 Faelle tragen `eingaben_konsistent=True` UND nicht-leere `pflichtfelder_luecken`.
  Das Abgabe-Gate in `api.py:731` (`einreichen()`) prueft NUR `eingaben_konsistent` —
  der eigene Schluessel steht daneben und wird nicht befragt. Die Folge: statt unseres
  Klartexts antwortet ERiC („Es wurde kein Geburtsdatum angegeben ..."), und das
  Frontend zeigt dazu nur „Die Pruefung hat Einwaende gegen die Erklaerung gefunden".
* Auf `rentner_gesamt` ist es teurer: der Kegel kennt `p36_lohnsteuer`, aber weder
  `bruttoarbeitslohn` noch `steuerklasse`. Wer die Lohnsteuer eintraegt, erfuellt die
  Bedingung „mindestens eines der Gruppe" und bekommt zwei Felder als fehlend gemeldet,
  die diese Scheibe nie fragt.

Der Test geht ueber den ECHTEN Weg (POST /fall -> POST /event -> POST /einreichen),
nicht ueber den Direktaufruf von `deklariere()`. Nur so zaehlt, was der Nutzer sieht.
"""
from __future__ import annotations

import json
import os
import sys
import threading
import urllib.error
import urllib.request

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/haut", "produkt/store", "golden"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import api as API        # noqa: E402
import server as SRV     # noqa: E402
import audit                # noqa: E402


def _req(base: str, method: str, path: str, body: dict | None = None):
    data = json.dumps(body).encode("utf-8") if body is not None else None
    req = urllib.request.Request(base + path, data=data, method=method,
                                 headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=20) as r:
            return r.status, json.loads(r.read())
    except urllib.error.HTTPError as e:
        return e.code, json.loads(e.read() or b"{}")


def _laie(feld_id, wert):
    return {"feld_id": feld_id, "wert": wert, "zustand": "bestaetigt",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft",
                         "haftung": "nutzer"},
            "schreiber": "ui:laie",
            "signal": {"signal_1": None, "signal_2": f"ok@{feld_id}"}}


@pytest.fixture
def base(tmp_path, monkeypatch):
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    monkeypatch.setenv("TAXGRAPH_NO_AUTH", "1")
    srv = SRV.make_server(0)
    th = threading.Thread(target=srv.serve_forever, daemon=True)
    th.start()
    try:
        yield f"http://{srv.server_address[0]}:{srv.server_address[1]}"
    finally:
        srv.shutdown()
        th.join(timeout=5)
        srv.server_close()


# Alle 12 PFLICHTFELDER beantwortet, plus die Stammdaten fuer den Absender-Block.
# Auf der Scheibe `gesamt` liegen alle 12 im Fragenkegel (gemessen 2026-10-01).
_VOLL = (("stammdaten_nachname", "Maier"), ("stammdaten_vorname", "Hans"),
         ("stammdaten_geburtsdatum", "05.05.1955"),
         ("stammdaten_strasse", "Musterstr."), ("stammdaten_hausnummer", "55"),
         ("stammdaten_plz", "55555"), ("stammdaten_wohnort", "Musterort"),
         ("stammdaten_steuernummer", "9181081508155"),
         ("kist_konfession", "keine"), ("stammdaten_keine_bankverbindung", True),
         ("stammdaten_art_est_erklaerung", True), ("veranlagung", "einzel"),
         ("bruttoarbeitslohn", 6000000), ("steuerklasse", "1"),
         ("p36_lohnsteuer", 1200000),
         ("vor_an_anteil_rv", 4200000), ("vor_ag_anteil_rv", 1200000))


def _fall(base, fid, scheibe, felder):
    st, b = _req(base, "POST", "/fall",
                 {"scheibe": scheibe, "veranlagungszeitraum": 2025, "fall_id": fid})
    assert st in (200, 201), f"POST /fall -> {st} {b}"
    for f, w in felder:
        st, b = _req(base, "POST", f"/fall/{fid}/event", _laie(f, w))
        assert st in (200, 201), f"POST /event {f} -> {st} {b}"


def test_abgabegate_nennt_die_pflichtfeldluecke_selbst(base):
    """Das Abgabe-Gate muss `pflichtfelder_luecken` befragen — heute tut es das nicht.

    Ein Fall ohne `bruttoarbeitslohn` bleibt `eingaben_konsistent` (die vorhandenen
    Angaben sind stimmig). `pflichtfelder_luecken` kennt die Luecke, `unvollstaendig`
    nicht. Weil `einreichen()` nur `eingaben_konsistent` liest, laeuft der Fall bis zu
    ERiC durch und der Nutzer bekommt eine Fremdmeldung statt unseres Feldnamens.
    """
    _fall(base, "pl1", "gesamt", [x for x in _VOLL if x[0] != "bruttoarbeitslohn"])
    st, b = _req(base, "GET", "/fall/pl1/deklaration")
    assert st == 200, f"GET /deklaration -> {st} {b}"
    assert [e["feld_id"] for e in (b.get("pflichtfelder_luecken") or [])] == \
        ["bruttoarbeitslohn"], (
        "Vorbedingung des Tests: die Gruppe 'alle_oder_keins' greift, weil steuerklasse "
        "und p36_lohnsteuer beantwortet sind — es fehlt genau bruttoarbeitslohn.")
    assert b.get("eingaben_konsistent") is True, (
        "Vorbedingung: die vorhandenen Angaben sind stimmig, das Gate greift also nicht.")

    st, b = _req(base, "POST", "/fall/pl1/einreichen", {})
    # Beide beobachteten Ausgaenge sind falsch, und keiner ist unserer:
    #   mit Hersteller-ID  -> 422 plausibilitaet_verletzt, rc=610001002 (ERiC urteilt)
    #   ohne Hersteller-ID -> 422 xml_nicht_baubar (der Absender-Block bricht ab)
    # Erwartet ist 409 mit `deklaration_unvollstaendig` — dem Grund, den nur
    # `pflichtfelder_luecken` kennt. Der Test ist damit in beiden Umgebungen rot.
    assert st == 409 and b.get("grund") == "deklaration_unvollstaendig", (
        f"Erwartet 409 'deklaration_unvollstaendig', erhalten {st} ({b.get('grund')}, "
        f"rc={b.get('rc')}). Ohne Leser von `pflichtfelder_luecken` faellt der Fall bis "
        f"ERiC durch, und der Nutzer liest eine Fremdmeldung statt des Feldnamens.")
    assert [e["feld_id"] for e in (b.get("unvollstaendig") or [])] == \
        ["bruttoarbeitslohn"], (
        "Der Grund muss die Luecke nennen, die `pflichtfelder_luecken` kennt.")


def test_rentner_gesamt_meldet_keine_felder_die_sein_kegel_nie_fragt(base):
    """FALSCHALARM: `rentner_gesamt` kann zwei der drei Gruppenfelder nie liefern.

    Der Kegel dieser Scheibe kennt `p36_lohnsteuer`, aber nicht `bruttoarbeitslohn` und
    nicht `steuerklasse`. Wer die Lohnsteuer eintraegt, erfuellt „mindestens eines der
    Gruppe" und bekommt die beiden unerreichbaren Felder als fehlend gemeldet.

    Rot, solange `_pflichtfelder_luecken()` gegen die ungefilterte PFLICHTFELDER-Liste
    prueft statt gegen den Kegel der laufenden Scheibe.
    """
    _fall(base, "pl2", "rentner_gesamt",
          [x for x in _VOLL if x[0] not in ("bruttoarbeitslohn", "steuerklasse")])
    st, b = _req(base, "GET", "/fall/pl2/deklaration")
    assert st == 200, f"GET /deklaration -> {st} {b}"
    luecken = [e["feld_id"] for e in (b.get("pflichtfelder_luecken") or [])]
    unerreichbar = sorted(set(luecken) & {"bruttoarbeitslohn", "steuerklasse"})
    assert not unerreichbar, (
        f"Als fehlend gemeldet, obwohl auf 'rentner_gesamt' nicht beantwortbar: "
        f"{unerreichbar}. Dieser Kegel kennt aus der Gruppe 'alle_oder_keins' nur "
        f"p36_lohnsteuer. Ein Hinweis auf ein Feld, das die Oberflaeche nicht stellt, "
        f"ist keine Hilfe, sondern eine Sackgasse.")
