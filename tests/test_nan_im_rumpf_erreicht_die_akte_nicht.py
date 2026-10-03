"""NaN, ±Infinity und 1e400 im Anfrage-Rumpf sind kein JSON (RFC 8259). Die Tür in server.py
weist sie ab wie die Rust-Tür (rust/api/src/dispatch.rs, lies_koerper): 400 "ungültiges JSON
im Body", gleicher Status, gleicher Wortlaut.

Ebenso eine Ganzzahl außerhalb von i64 an JEDER Stelle des Rumpfs (Backlog
python-schreibt-ganzzahl-ueber-i64-in-die-fallakte; unten, test_ganzzahl_*): Python liest sie exakt,
Rust (ohne arbitrary_precision) nur gerundet als f64, und der Rust-Lader sperrt sie. Die Rust-Tür
(lies_koerper) kennt diese Abweisung noch nicht, siehe Bericht haertung8.

Der Fund (Backlog falldatei-mit-nan-liest-rust-als-text, AK4, gemessen 2026-10-02): json.loads
nimmt NaN und Infinity als Literal an, 1e400 wird still zu inf. Je Zeile des Berichts an main:

1. ts, herkunft mit Zusatzschlüssel, signal.signal_1 = NaN -> 201, der Wert stand in der Akte.
   Der Rust-Lader sperrt eine solche Akte; DELETE und POST /event antworten danach 500
   (rust/api/tests/http.rs, abgewiesene_akte_bleibt_byte_gleich).
2. wert = NaN -> 422 aus Auflage T statt 400 an der Tür.
5. veranlagungszeitraum = NaN -> 400 mit eigenem Wortlaut; Infinity und 1e400 -> 500
   OverflowError aus int() in api.fall_anlegen. Die Tür ist der einzige Weg dorthin, der keine
   feste Jahreszahl mitbringt (Aufrufer gegrept), deshalb dort kein eigener Fang.

K1 aus derselben Messung, ohne NaN: signal_2 als Zahl warf bei zustand=bestaetigt in
store.append_event AttributeError -> 500. Rust führt signal_2 als Option<String>. Jetzt 422 mit
Feldname, für jeden zustand.

Dazu das Netz hinter der Tür: api.speichere_fall schreibt mit allow_nan=False. Kommt NaN doch
bis dorthin, scheitert das Schreiben, und die Akte bleibt, wie sie war. N2: keine Teil-Datei *.tmp
bleibt dabei liegen.

NULL LLM.
"""
from __future__ import annotations

import json
import os
import sys
import urllib.error
import urllib.request

import pytest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from test_paket_b_e2e_http import base, _req, _vorl  # noqa: F401,E402

import api as API  # noqa: E402 — den Pfad setzt test_paket_b_e2e_http

TUER = (400, {"fehler": "ungültiges JSON im Body"})
NICHT_ENDLICH = ["NaN", "Infinity", "-Infinity", "1e400", "-1e400"]


def _roh(base: str, pfad: str, rumpf: dict, literal: str | bytes) -> tuple[int, dict]:
    """POST mit `literal` wörtlich an der Stelle "@L@". json.dumps schriebe 1e400 als Infinity;
    1e400 läuft aber über parse_float, Infinity über parse_constant. Bytes für Rümpfe ohne UTF-8."""
    text = json.dumps(rumpf, ensure_ascii=False).encode()
    assert text.count(b'"@L@"') == 1, text
    roh = literal if isinstance(literal, bytes) else literal.encode()
    return _sende(base, pfad, text.replace(b'"@L@"', roh))


def _sende(base: str, pfad: str, daten: bytes) -> tuple[int, dict]:
    req = urllib.request.Request(base + pfad, data=daten, method="POST",
                                 headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=10) as r:
            return r.status, json.loads(r.read())
    except urllib.error.HTTPError as e:
        return e.code, json.loads(e.read())


def _akte() -> bytes:
    with open(os.path.join(API.FAELLE, "f1.json"), "rb") as f:
        return f.read()


def _dateien() -> list[str]:
    """Was im Fallverzeichnis liegt: die Akten und alles, was neben ihnen entstünde, auch eine *.tmp.
    server.py schreibt audit.jsonl vor der Antwort (`base` setzt AUDIT_DIR = FAELLE); die Datei steht
    beim ersten Blick also immer da und gehört zum Vergleich."""
    return sorted(os.listdir(API.FAELLE))


@pytest.fixture
def fall(base):
    _req(base, "POST", "/fall", {"fall_id": "f1", "scheibe": "ep", "veranlagungszeitraum": 2025})
    return base


# Jede Stelle und jedes Literal je einmal, kein Kreuzprodukt: die Tür kennt keine Felder, und
# jede Abweisung an der Tür kostet 0,5 s Abbau (serve_forever schläft dann bis zum Poll-Intervall).
@pytest.mark.parametrize("stelle, literal", [("ts", "NaN"), ("herkunft", "Infinity"),
                                             ("signal_1", "-Infinity"), ("ts", "1e400"),
                                             ("herkunft", "-1e400")])
def test_zeile1_ungepruefte_stelle_erreicht_die_akte_nicht(fall, stelle, literal):
    ev = _vorl("ep_arbeitstage", 200)
    rumpf = {"ts": {**ev, "ts": "@L@"},
             "herkunft": {**ev, "herkunft": {**ev["herkunft"], "x": "@L@"}},
             "signal_1": {**ev, "signal": {"signal_1": "@L@", "signal_2": None}}}[stelle]
    vorher = _akte()
    assert _roh(fall, "/fall/f1/event", rumpf, literal) == TUER
    assert _akte() == vorher


@pytest.mark.parametrize("literal", NICHT_ENDLICH)
def test_zeile2_wert_scheitert_an_der_tuer(fall, literal):
    vorher = _akte()
    assert _roh(fall, "/fall/f1/event", {**_vorl("ep_arbeitstage", 0), "wert": "@L@"},
                literal) == TUER
    assert _akte() == vorher


def test_endliche_zahl_passiert_die_tuer(fall):
    """Kontrolle: 2.5 ist JSON. Die Tür lässt die Zahl durch, Auflage T weist sie für ein
    int-Feld ab. Ohne diese Zeile bestünden die Tests oben auch mit einer Tür, die jede
    Kommazahl abweist."""
    status, antwort = _roh(fall, "/fall/f1/event", {**_vorl("ep_arbeitstage", 0), "wert": "@L@"},
                           "2.5")
    assert status == 422 and "fail-closed (Typ)" in antwort["fehler"], antwort


@pytest.mark.parametrize("literal", NICHT_ENDLICH)
def test_zeile5_veranlagungszeitraum_scheitert_an_der_tuer(base, literal):
    rumpf = {"fall_id": "f2", "scheibe": "ep", "veranlagungszeitraum": "@L@"}
    assert _roh(base, "/fall", rumpf, literal) == TUER
    assert not os.path.exists(os.path.join(API.FAELLE, "f2.json"))


def test_n1_rumpf_ohne_utf8_scheitert_an_der_tuer(fall):
    """N1: UnicodeDecodeError ist eine ValueError, `except ValueError` fängt sie mit. Vorher schloss
    der Dienst die Verbindung ohne Antwort. Die Rust-Tür (serde_json::from_slice) antwortet auf
    0xff im Rumpf ebenso 400 mit demselben Wortlaut (gemessen 2026-10-02 an POST /fall)."""
    vorher = _akte()
    assert _roh(fall, "/fall/f1/event", {**_vorl("ep_arbeitstage", 200), "ts": "@L@"},
                b'"2026-\xff"') == TUER
    assert _akte() == vorher


@pytest.mark.parametrize("vorsatz, kodierung", [(b"\xef\xbb\xbf", "utf-8"),
                                                (b"\xff\xfe", "utf-16-le"), (b"", "utf-16-le")],
                         ids=["utf-8-mit-bom", "utf-16-le-mit-bom", "utf-16-le-ohne-bom"])
def test_kodierung_nur_utf8_ohne_bom(fall, vorsatz, kodierung):
    """Die Tür nimmt nur UTF-8 ohne BOM an, wie die Rust-Tür. Vorher erkannte json.loads auf
    Bytes diese drei selbst und legte den Fall an: 201 (gemessen 2026-10-02). UTF-16-LE mit BOM
    scheitert an roh.decode mit UnicodeDecodeError, die anderen zwei an json.loads mit
    JSONDecodeError. Beide sind ValueError, also 400 und nicht 500."""
    vorher = _dateien(), _akte()
    rumpf = json.dumps({"fall_id": "f2", "scheibe": "ep", "veranlagungszeitraum": 2025})
    assert _sende(fall, "/fall", vorsatz + rumpf.encode(kodierung)) == TUER
    assert (_dateien(), _akte()) == vorher


@pytest.mark.parametrize("zustand", ["bestaetigt", "vorlaeufig"])
def test_k1_signal_2_als_zahl_ist_422_mit_feldname(fall, zustand):
    rumpf = {**_vorl("ep_arbeitstage", 200), "zustand": zustand,
             "signal": {"signal_1": None, "signal_2": 5}}
    vorher = _akte()
    _, antwort = _req(fall, "POST", "/fall/f1/event", rumpf, erwarte=422)
    assert "signal_2" in antwort["fehler"]
    assert _akte() == vorher


def test_netz_speichere_fall_schreibt_kein_nan(tmp_path, monkeypatch):
    monkeypatch.setattr(API, "FAELLE", str(tmp_path))
    API.speichere_fall("f1", {"events": []})
    vorher = (tmp_path / "f1.json").read_bytes()
    with pytest.raises(ValueError):
        API.speichere_fall("f1", {"events": [{"ts": float("nan")}]})
    assert (tmp_path / "f1.json").read_bytes() == vorher


def test_n2_gescheitertes_schreiben_laesst_keine_teil_datei(tmp_path, monkeypatch):
    """N2: scheitert das Schreiben, löscht speichere_fall die eigene *.tmp. Vorher blieb sie
    liegen, mit dem Anfang des Falls darin. Der Auslöser hier ist allow_nan=False; die Löschung
    gilt für jeden Fehler bis einschließlich os.replace."""
    monkeypatch.setattr(API, "FAELLE", str(tmp_path))
    API.speichere_fall("f1", {"events": []})
    with pytest.raises(ValueError):
        API.speichere_fall("f1", {"events": [{"ts": float("nan")}]})
    assert os.listdir(tmp_path) == ["f1.json"]


# ------------------------------------------------------------------ Ganzzahl außerhalb von i64

I64_MAX, I64_MIN = 2**63 - 1, -(2**63)
AUSSERHALB_I64 = ["18446744073709551616", "9223372036854775808", "-9223372036854775809",
                  "-18446744073709551617", "9" * 5000]


@pytest.mark.parametrize("stelle, literal", [("wert", "18446744073709551616"),
                                             ("wert", "-9223372036854775809"),
                                             ("signal_1", "18446744073709551616"),
                                             ("signal_1", "9223372036854775808"),
                                             ("ersetzt", "-18446744073709551617"),
                                             ("wert", "9" * 5000)])
def test_ganzzahl_ausserhalb_i64_scheitert_an_der_tuer(fall, stelle, literal):
    """An jeder Zahlstelle unter `events`, auch dort, wo kein Typ-Check hinsieht. Vorher 201 und die
    Zahl stand exakt in der Akte; Rust sperrte sie dann beim Laden (Sonden p5, p6, p16, haertung)."""
    ev = _vorl("ep_arbeitstage", 200)
    rumpf = {"wert": {**ev, "wert": "@L@"},
             "signal_1": {**ev, "signal": {"signal_1": "@L@", "signal_2": None}},
             "ersetzt": {**ev, "ersetzt": "@L@"}}[stelle]
    vorher = _akte()
    assert _roh(fall, "/fall/f1/event", rumpf, literal) == TUER
    assert _akte() == vorher


@pytest.mark.parametrize("literal", AUSSERHALB_I64[:3])
def test_ganzzahl_ausserhalb_i64_als_veranlagungszeitraum_scheitert_an_der_tuer(base, literal):
    rumpf = {"fall_id": "f2", "scheibe": "ep", "veranlagungszeitraum": "@L@"}
    assert _roh(base, "/fall", rumpf, literal) == TUER
    assert not os.path.exists(os.path.join(API.FAELLE, "f2.json"))


@pytest.mark.parametrize("literal", [str(I64_MAX), str(I64_MIN)])
def test_i64_grenzen_selbst_passieren_die_tuer(fall, literal):
    """Kontrolle: i64::MAX und i64::MIN gehen durch. Ohne diese Zeile bestünden die Tests oben auch
    mit einer Tür, die jede große Zahl abweist. `signal_1` nimmt jeden JSON-Wert, `wert` auf einem
    Feld ohne Vorzeichen- und Bereichsgrenze jede Zahl, die Auflage T zulässt."""
    status, antwort = _roh(fall, "/fall/f1/event",
                           {**_vorl("ep_arbeitstage", 200), "signal": {"signal_1": "@L@", "signal_2": None}},
                           literal)
    assert status == 201, antwort

