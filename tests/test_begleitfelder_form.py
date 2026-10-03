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
from datetime import datetime, timezone

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
    with pytest.raises(ValueError, match=r"signal darf nur die Schlüssel signal_1 und signal_2 tragen") as e:
        ST.append_event(st, feld_id="ep_arbeitstage", wert=1, zustand="vorlaeufig",
                        herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                        signal={"signal_1": None, "signal_2": None, "geheim-schluessel": 1},
                        schreiber="ui:laie", bindung=BINDUNG)
    assert "geheim-schluessel" not in str(e.value)


def test_form_wird_vor_dem_falsy_signal_geprueft():
    """`signal or {...}` machte 0, "", [] und false still zum Standard-Signal — der Grund, warum die Prüfung
    VOR dieser Zeile stehen muss (Gegenprobe: sie dahinter, und diese vier Fälle gehen durch)."""
    for falsy in (0, "", [], False):
        with pytest.raises(ValueError, match=r"signal muss ein Objekt sein"):
            _schreibe({"herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                       "signal": falsy})


FEHLT = object()   # Schluesselwort `ts` gar nicht uebergeben
FEST = "2026-01-01T00:00:00+00:00"


def _ist_jetzt_iso(ts) -> bool:
    """Nicht leer, ISO 8601 mit Zone, parsebar, innerhalb einer Minute der Uhr."""
    if not isinstance(ts, str) or not ts:
        return False
    t = datetime.fromisoformat(ts)
    return t.tzinfo is not None and abs((datetime.now(timezone.utc) - t).total_seconds()) < 60


@pytest.mark.parametrize("ts", ["", None, FEHLT], ids=["leer", "None", "fehlt"])
def test_leerer_zeitstempel_heisst_fehlt_und_wird_die_jetzt_zeit(ts):
    """Pin (Vault decisions/leerer-zeitstempel-heisst-fehlt-und-wird-die-jetzt-zeit): `ts or _now()` in
    `append_event` UND `erzeuge_snapshot`. Rust zieht nach (`store::tests`); haelt Python unbemerkt davon
    ab, faellt dieser Test, bevor die zwei Seiten verschiedene Eintraege und Kennungen bilden."""
    kw = {} if ts is FEHLT else {"ts": ts}
    st = ST.leerer_store(2025, fall_id="ts-leer")
    ev = ST.append_event(st, feld_id="ep_arbeitstage", wert=1, zustand="vorlaeufig", schreiber="ui:laie",
                         herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                         bindung=BINDUNG, **kw)
    assert _ist_jetzt_iso(ev["ts"]), repr(ev["ts"])
    snap = ST.erzeuge_snapshot(st, **kw)
    assert _ist_jetzt_iso(snap["ts"]), repr(snap["ts"])


def test_fester_zeitstempel_bleibt_wie_er_ist():
    """Gegenprobe zum Pin oben: ein uebergebener Zeitstempel wird nicht durch die Jetzt-Zeit ersetzt."""
    st = ST.leerer_store(2025, fall_id="ts-fest")
    ev = ST.append_event(st, feld_id="ep_arbeitstage", wert=1, zustand="vorlaeufig", schreiber="ui:laie",
                         herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                         bindung=BINDUNG, ts=FEST)
    assert ev["ts"] == FEST and not _ist_jetzt_iso(ev["ts"])
    assert ST.erzeuge_snapshot(st, ts=FEST)["ts"] == FEST


def test_http_ts_zahl_und_signal_liste_sind_422(base):
    """Der Weg des Nutzers: POST /event. Vorher 201 (ts), 500 (signal) bzw. still das Standard-Signal."""
    _req(base, "POST", "/fall", {"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": "form-http"},
         erwarte=201)
    ev = _laie("ep_arbeitstage", 200)
    _req(base, "POST", "/fall/form-http/event", {**ev, "ts": 5}, erwarte=422)
    for signal in (5, "x", [1], True, 0, "", [], False):
        _req(base, "POST", "/fall/form-http/event", {**ev, "signal": signal}, erwarte=422)
    _req(base, "POST", "/fall/form-http/event", {**ev, "herkunft": {**ev["herkunft"], "x": 1}}, erwarte=422)
    for signal in ({"signal_1": None, "signal_2": "ok", "x": 1}, {"x": 1}):    # Zusatzschlüssel im signal
        _req(base, "POST", "/fall/form-http/event", {**ev, "signal": signal}, erwarte=422)
    _req(base, "POST", "/fall/form-http/event", ev, erwarte=201)
    assert _req(base, "GET", "/fall/form-http/ergebnis", erwarte=200)[0] == 200
