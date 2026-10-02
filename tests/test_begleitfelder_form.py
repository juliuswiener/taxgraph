"""`ts`, `herkunft` und `signal` eines Events tragen beim Schreiben genau die Form, die der Rust-Leser kennt.

Vault decisions/tuer-und-speicher-weisen-ab-was-die-fallakte-nicht-exakt-halten-kann, Punkt 2; Backlog
python-schreibt-akte-die-der-rust-leser-sperrt. Gemessen (Worker haertung, 2026-10-02): `ts = 5` und
`herkunft` mit Zusatzschlüssel gaben 201 und machten die Akte für Rust unlesbar (jede Route 500);
`signal` als 5/"x"/[1]/true gab 500 (AttributeError), als 0/""/[]/false still das Standard-Signal.

Die Tabelle der Formen liegt in rust/fixtures/begleitfelder_formen.json und gilt für beide Sprachen:
dieser Test prüft, was `append_event` tut (und dass jeder angenommene Aufruf als gespeichertes Event
die JSON-Runde besteht); rust/store/src/persistenz.rs prüft, dass der Rust-Leser jedes angenommene Event
lädt. Laden prüft nie: eine Akte mit alter Form lädt weiter.

NULL LLM."""
from __future__ import annotations

import copy
import json
import os
import sys

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "produkt", "store"))
sys.path.insert(0, os.path.join(ROOT, "produkt", "traverser"))
import store as ST  # noqa: E402
import traverser as TR  # noqa: E402
from test_paket_b_e2e_http import _laie, _req, base  # noqa: E402,F401 — Fixture

with open(os.path.join(ROOT, "rust", "fixtures", "begleitfelder_formen.json"), encoding="utf-8") as _f:
    TABELLE = json.load(_f)
FAELLE = TABELLE["faelle"]
BINDUNG = TR.lade_bindung()


def _schreibe(fall: dict) -> dict:
    kw = {}
    if "ts" in fall:
        kw["ts"] = fall["ts"]
    if "signal" in fall:
        kw["signal"] = fall["signal"]
    st = ST.leerer_store(2025, fall_id="form")
    return ST.append_event(st, feld_id="ep_arbeitstage", wert=1, zustand="vorlaeufig",
                           herkunft=copy.deepcopy(fall["herkunft"]), schreiber="ui:laie",
                           bindung=BINDUNG, **kw)


def test_tabelle_hat_beide_seiten():
    urteile = {f["python"] for f in FAELLE}
    assert urteile == {"angenommen", "abgewiesen"}
    assert len({f["name"] for f in FAELLE}) == len(FAELLE)


@pytest.mark.parametrize("fall", [f for f in FAELLE if f["python"] == "angenommen"], ids=lambda f: f["name"])
def test_angenommene_form_wird_gespeichert_und_besteht_die_json_runde(fall):
    ev = _schreibe(fall)
    assert json.loads(json.dumps(ev, allow_nan=False)) == ev
    assert isinstance(ev["ts"], str) and isinstance(ev["signal"], dict)
    assert ev["herkunft"] == fall["herkunft"]


@pytest.mark.parametrize("fall", [f for f in FAELLE if f["python"] == "abgewiesen"], ids=lambda f: f["name"])
def test_falsche_form_wird_abgewiesen_ohne_event(fall):
    st = ST.leerer_store(2025, fall_id="form-abgewiesen")
    kw = {k: fall[k] for k in ("ts", "signal") if k in fall}
    with pytest.raises(ValueError, match=r"^fail-closed \(Form\): "):
        ST.append_event(st, feld_id="ep_arbeitstage", wert=1, zustand="vorlaeufig",
                        herkunft=copy.deepcopy(fall["herkunft"]), schreiber="ui:laie", bindung=BINDUNG, **kw)
    assert st["events"] == []


def test_meldung_nennt_feld_und_typ_nie_den_wert():
    st = ST.leerer_store(2025, fall_id="form-meldung")
    with pytest.raises(ValueError, match=r"ts muss Text sein oder fehlen, nicht int"):
        ST.append_event(st, feld_id="ep_arbeitstage", wert=1, zustand="vorlaeufig", ts=987654321,
                        herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                        schreiber="ui:laie", bindung=BINDUNG)
    with pytest.raises(ValueError) as e:
        ST.append_event(st, feld_id="ep_arbeitstage", wert=1, zustand="vorlaeufig",
                        herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer",
                                  "geheim-schluessel": 1}, schreiber="ui:laie", bindung=BINDUNG)
    assert "geheim-schluessel" not in str(e.value)


def test_form_wird_vor_dem_falsy_signal_geprueft():
    """`signal or {...}` machte 0, "", [] und false still zum Standard-Signal — der Grund, warum die Prüfung
    VOR dieser Zeile stehen muss (Gegenprobe: sie dahinter, und diese vier Fälle gehen durch)."""
    for falsy in (0, "", [], False):
        with pytest.raises(ValueError, match=r"signal muss ein Objekt sein"):
            _schreibe({"herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                       "signal": falsy})


def test_http_ts_zahl_und_signal_liste_sind_422(base):
    """Der Weg des Nutzers: POST /event. Vorher 201 (ts), 500 (signal) bzw. still das Standard-Signal."""
    _req(base, "POST", "/fall", {"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": "form-http"},
         erwarte=201)
    ev = _laie("ep_arbeitstage", 200)
    _req(base, "POST", "/fall/form-http/event", {**ev, "ts": 5}, erwarte=422)
    for signal in (5, "x", [1], True, 0, "", [], False):
        _req(base, "POST", "/fall/form-http/event", {**ev, "signal": signal}, erwarte=422)
    _req(base, "POST", "/fall/form-http/event", {**ev, "herkunft": {**ev["herkunft"], "x": 1}}, erwarte=422)
    _req(base, "POST", "/fall/form-http/event", ev, erwarte=201)
    assert _req(base, "GET", "/fall/form-http/ergebnis", erwarte=200)[0] == 200
