"""Paket-B-Durchstich über HTTP — fährt tests/test_paket_a_e2e.py Schritt für Schritt über die
Haut-Endpunkte nach (gleiche EP-Familie, gleiche Asserts, gleiche 2156). Deterministisch, NULL LLM.

Auflagen (Instructor): (A) POST …/chat -> 501, nie 200-Fake; (B) Server bindet AUSSCHLIESSLICH
127.0.0.1 (asserted); (C) api_schema/*.json wird gegen die echten Responses validiert. Der
numerische Teil (Spanne, 2156) hängt an der Catala-Toolchain -> sauberer Skip wie im A-Test.
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
for sub in ("produkt/haut", "golden"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import api as API        # noqa: E402
import server as SRV     # noqa: E402

jsonschema = pytest.importorskip("jsonschema")

SCHEMA_DIR = os.path.join(ROOT, "produkt", "haut", "api_schema")
EP_FELDER = {"ep_arbeitstage", "ep_entfernung_km", "ep_oepnv_kosten", "ep_eigenes_kfz"}


def _schema(name: str) -> dict:
    with open(os.path.join(SCHEMA_DIR, f"{name}.json"), encoding="utf-8") as f:
        return json.load(f)


def _val(name: str, obj: dict) -> None:
    jsonschema.Draft202012Validator(_schema(name)).validate(obj)


def _catala_da() -> bool:
    try:
        import runner  # noqa: F401
        return True
    except Exception:
        return False


def _req(base: str, method: str, path: str, body: dict | None = None):
    data = json.dumps(body).encode("utf-8") if body is not None else None
    req = urllib.request.Request(base + path, data=data, method=method,
                                 headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=10) as r:
            return r.status, json.loads(r.read())
    except urllib.error.HTTPError as e:
        return e.code, json.loads(e.read())


def _laie(fld, w):
    return {"feld_id": fld, "wert": w, "zustand": "bestaetigt",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "ui:laie", "signal": {"signal_1": None, "signal_2": f"ok@{fld}"}}


def _llm(fld, w):
    return {"feld_id": fld, "wert": w, "zustand": "vorlaeufig",
            "herkunft": {"herkunft": "llm_vorschlag", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "llm:chat", "signal": {"signal_1": None, "signal_2": None}}


@pytest.fixture
def base(tmp_path, monkeypatch):
    # Fall-Daten in ein temporäres Verzeichnis (nie ins Repo, nie in den echten faelle/-Ordner)
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    srv = SRV.make_server(0)                      # port=0 -> freier Port
    assert srv.server_address[0] == "127.0.0.1", "Auflage B: Server muss an 127.0.0.1 binden"
    th = threading.Thread(target=srv.serve_forever, daemon=True)
    th.start()
    try:
        yield f"http://{srv.server_address[0]}:{srv.server_address[1]}"
    finally:
        srv.shutdown()
        th.join(timeout=2)


def test_bindet_nur_localhost():
    """Auflage B, hart: die Bind-Adresse ist 127.0.0.1, niemals 0.0.0.0."""
    srv = SRV.make_server(0)
    try:
        assert srv.server_address[0] == "127.0.0.1"
    finally:
        srv.server_close()


def test_chat_501(base):
    """Auflage A: POST /chat liefert 501 mit erklärendem Vertrag, NIE 200."""
    _req(base, "POST", "/fall", {"scheibe": "ep", "veranlagungszeitraum": 2025, "fall_id": "c1"})
    st, b = _req(base, "POST", "/fall/c1/chat", {"text": "hallo"})
    assert st == 501, f"chat muss 501 sein, war {st}"
    assert "vertrag" in b and "stufe" in b
    assert b.get("fehler") == "not_implemented"


def test_fail_closed_llm_kann_nicht_bestaetigen(base):
    """Der fail-closed-Store weist ein llm:-bestaetigt-Event ab — über HTTP -> 422, nie 201."""
    _req(base, "POST", "/fall", {"scheibe": "ep", "veranlagungszeitraum": 2025, "fall_id": "f1"})
    boese = {"feld_id": "ep_arbeitstage", "wert": 220, "zustand": "bestaetigt",
             "herkunft": {"herkunft": "llm_vorschlag", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
             "schreiber": "llm:chat", "signal": {"signal_1": None, "signal_2": "gefaelscht"}}
    st, b = _req(base, "POST", "/fall/f1/event", boese)
    assert st == 422, f"llm-bestaetigt muss abgewiesen werden, war {st}"
    assert "fail-closed" in b["fehler"]


def test_schema_gate_negativ():
    """Auflage C, Härtung: ein verfälschtes Objekt MUSS das Schema-Gate rot färben."""
    kaputt = {"fall_id": "x", "snapshot_id": "y", "fragen": [{"feld_id": "a", "typ": "UNBEKANNT"}]}
    with pytest.raises(jsonschema.ValidationError):
        _val("fragen", kaputt)


def test_durchstich_http(base):
    catala = _catala_da()
    fid = "e2e-ep"

    # 0) Fall anlegen
    st, b = _req(base, "POST", "/fall",
                 {"scheibe": "ep", "veranlagungszeitraum": 2025, "fall_id": fid})
    assert st == 201 and b["fall_id"] == fid

    # 1) leerer Fall -> fragen == die 4 EP-Felder
    st, b = _req(base, "GET", f"/fall/{fid}/fragen")
    assert st == 200
    _val("fragen", b)                                          # Auflage C
    assert {q["feld_id"] for q in b["fragen"]} == EP_FELDER
    # Fragetexte kommen aus der Bindung (laienverständlich, kein §)
    assert all("§" not in (q["fragetext_laie"] or "") for q in b["fragen"])

    # 2) 3x laie-bestätigt + 1x llm-VORLÄUFIG
    for fld, w in [("ep_entfernung_km", 30), ("ep_eigenes_kfz", True), ("ep_oepnv_kosten", 0)]:
        st, b = _req(base, "POST", f"/fall/{fid}/event", _laie(fld, w))
        assert st == 201
        _val("event", b)
    st, llm = _req(base, "POST", f"/fall/{fid}/event", _llm("ep_arbeitstage", 220))
    assert st == 201
    llm_ev = llm["event_id"]

    # nur das offene Feld bleibt in der Queue
    st, b = _req(base, "GET", f"/fall/{fid}/fragen")
    assert [q["feld_id"] for q in b["fragen"]] == ["ep_arbeitstage"]

    # 3) stand: arbeitstage schimmernd (KI), Spanne offen (nur mit Engine numerisch)
    st, stand_a = _req(base, "GET", f"/fall/{fid}/stand")
    assert st == 200
    _val("stand", stand_a)
    assert stand_a["felder"]["ep_arbeitstage"]["herkunft_badge"] == "schimmernd"
    assert stand_a["felder"]["ep_entfernung_km"]["herkunft_badge"] == "solide"
    spanne_a = None
    if catala:
        iv = stand_a["intervall"]
        assert iv["max_cent"] - iv["min_cent"] > 0, "offener arbeitstage -> Spanne > 0"
        spanne_a = iv["max_cent"] - iv["min_cent"]

    # 4) FAIL-CLOSED vorher: Input-Kegel enthält vorlaeufig -> keine feste Zahl
    st, erg = _req(base, "GET", f"/fall/{fid}/ergebnis")
    assert st == 200
    _val("ergebnis", erg)
    assert erg["zahl_cent"] is None
    assert erg["grund"] == "input_kegel_nicht_bestaetigt"

    # 5) LLM-Wert via ZWEI-SIGNAL bestätigen (ersetzt das llm-Event)
    st, b = _req(base, "POST", f"/fall/{fid}/event", {
        "feld_id": "ep_arbeitstage", "wert": 220, "zustand": "bestaetigt",
        "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        "schreiber": "ui:laie",
        "signal": {"signal_1": llm_ev, "signal_2": "klick@beleg_arbeitstage"}, "ersetzt": llm_ev})
    assert st == 201

    # 6) stand: Spanne schrumpft auf Punkt (monoton)
    st, stand_b = _req(base, "GET", f"/fall/{fid}/stand")
    if catala:
        iv2 = stand_b["intervall"]
        spanne_b = iv2["max_cent"] - iv2["min_cent"]
        assert spanne_b < spanne_a and spanne_b == 0

    # 7) FAIL-CLOSED nachher: Kegel bestätigt -> echte Zahl (Naht-Einheit CENT: 215600 = 2156,00 €,
    #    wie test_paket_a_e2e nach der Einheiten-Konvention ad4e22b)
    st, erg2 = _req(base, "GET", f"/fall/{fid}/ergebnis")
    _val("ergebnis", erg2)
    if catala:
        assert erg2["zahl_cent"] == 215600
        assert erg2["grund"] == "bestaetigt"
    else:
        assert erg2["zahl_cent"] is None and erg2["grund"] == "engine_unavailable"

    # Vorwärts-Trace/Justification bis anker_ref
    st, w = _req(base, "GET", f"/fall/{fid}/feld/ep_arbeitstage/warum")
    assert st == 200
    _val("warum", w)
    j = w["justification"]
    assert j["herkunft"]["herkunft"] == "laie"      # nach Bestätigung: laie-Beleg
    assert j["zustand"] == "bestaetigt" and j["signal"]["signal_2"]
    assert j["anker_ref"]["zitatanker"] == "jeden Arbeitstag"
