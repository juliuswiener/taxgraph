"""`pflichtfelder_luecken` wird geschrieben und von keiner Produktstelle gelesen.

Warum es das gibt
-----------------
`EM.deklariere()` liefert 12 Schluessel. Vier davon liest keine Produktstelle:
`basis_snapshot`, `vollstaendig`, `pflichtfelder_luecken`,
`pflichtfelder_vollstaendig`. `dokumentiert` liest nur `zuruecklesen()`, und das ruft
kein Produktpfad auf — fuenf ohne Produktleser.

`pflichtfelder_luecken` ist der einzige unter ihnen mit einer eigenen, unersetzlichen
Aussage: er kennt die FELD-ABWESENHEIT gegen die gepflegte PFLICHTFELDER-Liste, die die
Hauptschleife in `deklariere()` per Konstruktion nie sehen kann
(`produkt/mapping/est_mapping.py:546`).

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

Die Tests gehen ueber den ECHTEN Weg (POST /fall -> POST /event -> POST /einreichen),
nicht ueber den Direktaufruf von `deklariere()`. Nur so zaehlt, was der Nutzer sieht.

xfail(strict=True, raises=AssertionError) statt „einfach rot" — wie in
tests/test_luf_gewinn_kz_fehlt.py: ein dauerhaft roter Test im Baum zerstoert das Signal
„rot heisst, etwas ist kaputtgegangen", und unter `make unit` bricht er die volle Suite
ab. `raises=` ist Pflicht, nicht Zierde: es nagelt fest, WORAN der Test scheitert, nicht
nur DASS er scheitert. Sobald ein Leser gebaut wird, kippt der Test auf XPASS und
erzwingt Aufmerksamkeit.

Die Vorbedingungen in beiden xfail-Tests stehen VOR der roten Assertion: sie belegen,
dass der Fall ueberhaupt so gebaut ist, wie der Befund ihn beschreibt. Sie stehen als
`pytest.fail(...)` da, NICHT als `assert`: unter `raises=AssertionError` zaehlt eine
gebrochene Vorbedingung sonst als xfailed statt als Fehler — der Test meldete dann
"Defekt bestaetigt", waehrend in Wahrheit der Fall nicht mehr so gebaut wird. `Failed`
ist keine `AssertionError`, faellt also durch das `raises=` hindurch und erscheint als
FAILED. Belegt am 2026-10-01 mit absichtlich gebrochener Vorbedingung.

Die dritte Funktion hier ist KEIN xfail: die Kontrollzeile muss gruen bleiben, sonst
misst die Datei „irgendetwas ist kaputt" statt der fehlenden Leser.
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


def test_kontrollzeile_vollstaendiger_fall_hat_keine_luecke(base):
    """GRUEN, kein xfail. Die Kontrolle fuer die beiden Tests darunter.

    Ohne diese Zeile misst die Datei nicht die fehlenden Leser, sondern nur, dass
    irgendetwas nicht geht: `pflichtfelder_luecken` muss auf `gesamt` leer sein, wenn
    alle 12 Felder beantwortet sind. Faellt das hier um, ist die Messmechanik hin.
    """
    _fall(base, "pl0", "gesamt", list(_VOLL))
    st, b = _req(base, "GET", "/fall/pl0/deklaration")
    assert st == 200, f"GET /deklaration -> {st} {b}"
    assert [e["feld_id"] for e in (b.get("pflichtfelder_luecken") or [])] == [], (
        "Vollstaendiger Fall darf keine Pflichtfeldluecke melden.")
    assert b.get("eingaben_konsistent") is True, (
        "Vollstaendiger Fall muss stimmig sein.")


@pytest.mark.xfail(strict=True, raises=AssertionError, reason=(
    "`einreichen()` befragt nur `eingaben_konsistent`, nicht `pflichtfelder_luecken`. "
    "Der Fall faellt bis ERiC durch und der Nutzer liest eine Fremdmeldung statt des "
    "Feldnamens. Erwartet 409 'deklaration_unvollstaendig'."))
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
    _luecken = [e["feld_id"] for e in (b.get("pflichtfelder_luecken") or [])]
    if _luecken != ["bruttoarbeitslohn"]:
        pytest.fail(
            "Vorbedingung des Tests: die Gruppe 'alle_oder_keins' greift, weil steuerklasse "
            f"und p36_lohnsteuer beantwortet sind — es fehlt genau bruttoarbeitslohn. "
            f"Gemeldet wurde: {_luecken}.")
    if b.get("eingaben_konsistent") is not True:
        pytest.fail(
            "Vorbedingung: die vorhandenen Angaben sind stimmig, das Gate 'unvollstaendig' "
            "greift also nicht. Sonst scheitert der Test am Gate statt am fehlenden Leser.")

    st, b = _req(base, "POST", "/fall/pl1/einreichen", {})
    # Beide beobachteten Ausgaenge sind falsch, und keiner ist unserer:
    #   mit Hersteller-ID  -> 422 plausibilitaet_verletzt, rc=610001002 (ERiC urteilt)
    #   ohne Hersteller-ID -> 422 xml_nicht_baubar (der Absender-Block bricht ab)
    # Erwartet ist 409 mit `deklaration_unvollstaendig` — dem Grund, den nur
    # `pflichtfelder_luecken` kennt. Damit xfail in beiden Umgebungen.
    assert st == 409 and b.get("grund") == "deklaration_unvollstaendig", (
        f"Erwartet 409 'deklaration_unvollstaendig', erhalten {st} ({b.get('grund')}, "
        f"rc={b.get('rc')}). Ohne Leser von `pflichtfelder_luecken` faellt der Fall bis "
        f"ERiC durch, und der Nutzer liest eine Fremdmeldung statt des Feldnamens.")
    assert [e["feld_id"] for e in (b.get("unvollstaendig") or [])] == \
        ["bruttoarbeitslohn"], (
        "Der Grund muss die Luecke nennen, die `pflichtfelder_luecken` kennt.")


@pytest.mark.xfail(strict=True, raises=AssertionError, reason=(
    "Der Kegel von `rentner_gesamt` kennt aus der Gruppe `alle_oder_keins` nur "
    "`p36_lohnsteuer` — nicht `bruttoarbeitslohn` und nicht `steuerklasse`. Wer die "
    "Lohnsteuer eintraegt, bekommt die zwei als fehlend gemeldet und kann sie auf "
    "dieser Scheibe nicht beantworten. ERiC verlangt sie trotzdem (rc=610001002, "
    "gemessen 2026-10-01). Erwartet: jede gemeldete Luecke ist im Kegel beantwortbar."))
def test_rentner_gesamt_meldet_keine_felder_die_sein_kegel_nie_fragt(base):
    """Eine Luecke, die die laufende Scheibe nie fragt, ist keine Hilfe, sondern eine Sackgasse.

    Gemessen am 2026-10-01 (HEAD `1065e25`, echtes ERiC 44.2.4.0) an genau diesem Fall:

    * `rentner_gesamt` mit beantworteter `p36_lohnsteuer`, ohne `bruttoarbeitslohn` und
      `steuerklasse` -> ERiC lehnt ab, rc=610001002, Text „Lohnsteuer ... erklaert, aber
      kein Bruttoarbeitslohn angegeben".
    * Dieselbe Scheibe ohne jede Lohnangabe -> ERiC nimmt an (rc=0).

    Die Gruppe ist also NICHT zu streng: ERiC verlangt beide Felder wirklich. Der Defekt
    sitzt im Fragenkegel, der `p36_lohnsteuer` stellt und die zwei zugehoerigen nie.
    Deshalb prueft dieser Test nicht „die Meldung soll weg" (das waere stilles
    Wegfiltern), sondern die Invariante: jedes gemeldete Feld muss der Kegel der
    laufenden Scheibe stellen koennen. Ein Kegel-Fix macht ihn gruen, ein Filter im
    Melder nicht.
    """
    _fall(base, "pl2", "rentner_gesamt",
          [x for x in _VOLL if x[0] not in ("bruttoarbeitslohn", "steuerklasse")])
    st, b = _req(base, "GET", "/fall/pl2/deklaration")
    if st != 200:
        pytest.fail(f"Vorbedingung: GET /deklaration -> {st} {b}")
    luecken = [e["feld_id"] for e in (b.get("pflichtfelder_luecken") or [])]
    if "p36_lohnsteuer" in luecken:
        pytest.fail(
            "Vorbedingung: p36_lohnsteuer ist beantwortet und liegt im Kegel von "
            "'rentner_gesamt' — sie darf nicht als fehlend gemeldet werden.")
    if not luecken:
        pytest.fail(
            "Vorbedingung: die Gruppe 'alle_oder_keins' greift, sobald eines ihrer Felder "
            "beantwortet ist (est_mapping.PFLICHTFELDER). Ohne bruttoarbeitslohn und "
            "steuerklasse muss sie also etwas melden.")
    kegel = set(API.SCHEIBEN["rentner_gesamt"]["felder"])
    unerreichbar = sorted(set(luecken) - kegel)
    assert not unerreichbar, (
        f"Als fehlend gemeldet, obwohl auf 'rentner_gesamt' nicht beantwortbar: "
        f"{unerreichbar}. Dieser Kegel kennt aus der Gruppe 'alle_oder_keins' nur "
        f"p36_lohnsteuer. Ein Hinweis auf ein Feld, das die Oberflaeche nicht stellt, "
        f"ist keine Hilfe, sondern eine Sackgasse.")
