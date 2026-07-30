"""Erreichbarkeits-Gate: Sperrgrund-Tests für _an_gesamt_sperrgrund().

Teste die Guards, die verhindern, dass der Ring Mitveranlagung berechnet:
- uebernachtung_tatbestand_offen (Kosten > 0, aber Bedingungen nicht alle bestätigt)
- ausland_uebernachtung_nicht_ring_faehig (Kosten > 0, Ausland=True)

Wichtig: beide Richtungen testen (sperrt vs. sperrt nicht).
"""

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
sys.path.insert(0, os.path.join(ROOT, "produkt", "store"))

import api as API        # noqa: E402
import server as SRV     # noqa: E402

jsonschema = pytest.importorskip("jsonschema")
SCHEMA_DIR = os.path.join(ROOT, "produkt", "haut", "api_schema")


def _schema(name: str) -> dict:
    with open(os.path.join(SCHEMA_DIR, f"{name}.json"), encoding="utf-8") as f:
        return json.load(f)


def _val(name: str, obj: dict) -> None:
    jsonschema.Draft202012Validator(_schema(name)).validate(obj)


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


@pytest.fixture
def base(tmp_path, monkeypatch):
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    srv = SRV.make_server(0)
    assert srv.server_address[0] == "127.0.0.1"
    th = threading.Thread(target=srv.serve_forever, daemon=True)
    th.start()
    try:
        yield f"http://{srv.server_address[0]}:{srv.server_address[1]}"
    finally:
        srv.shutdown()
        th.join(timeout=5)
        srv.server_close()


def test_uebernachtung_tatbestand_offen_sperrgrund(base):
    """Guard: Kosten > 0, aber Bedingungen nicht alle bestätigt → Sperrgrund ring_gesperrt."""
    status, resp = _req(base, "POST", "/fall", {"scheibe": "an_gesamt", "veranlagungszeitraum": "2025", "fall_id": "ueb-1"})
    assert status == 201
    fall_id = resp["fall_id"]
    status, resp = _req(base, "POST", f"/fall/{fall_id}/event", _laie("uebernachtung_kosten_monat", 100000))
    assert status == 201
    status, resp = _req(base, "POST", f"/fall/{fall_id}/event", _laie("uebernachtung_auswaerts", True))
    assert status == 201
    status, resp = _req(base, "POST", f"/fall/{fall_id}/event", _laie("uebernachtung_im_inland", True))
    assert status == 201
    status, resp = _req(base, "GET", f"/fall/{fall_id}/stand", None)
    assert status == 200
    _val("stand", resp)
    assert resp["ring_gesperrt"] == "uebernachtung_tatbestand_offen"


def test_uebernachtung_alle_bedingungen_bestaetigt_nicht_gesperrt(base):
    """Gegenprobe: Kosten > 0, ABER alle 3 Bedingungen bestätigt + Inland → NICHT gesperrt."""
    status, resp = _req(base, "POST", "/fall", {"scheibe": "an_gesamt", "veranlagungszeitraum": "2025", "fall_id": "ueb-2"})
    assert status == 201
    fall_id = resp["fall_id"]
    status, resp = _req(base, "POST", f"/fall/{fall_id}/event", _laie("uebernachtung_kosten_monat", 100000))
    assert status == 201
    status, resp = _req(base, "POST", f"/fall/{fall_id}/event", _laie("uebernachtung_auswaerts", True))
    assert status == 201
    status, resp = _req(base, "POST", f"/fall/{fall_id}/event", _laie("uebernachtung_alleinnutzung", True))
    assert status == 201
    status, resp = _req(base, "POST", f"/fall/{fall_id}/event", _laie("uebernachtung_keine_lange_unterbrechung", True))
    assert status == 201
    status, resp = _req(base, "POST", f"/fall/{fall_id}/event", _laie("uebernachtung_im_inland", True))
    assert status == 201
    status, resp = _req(base, "POST", f"/fall/{fall_id}/event", _laie("uebernachtung_monate", 12))
    assert status == 201
    status, resp = _req(base, "POST", f"/fall/{fall_id}/event", _laie("uebernachtung_monate_bisher", 10))
    assert status == 201
    status, resp = _req(base, "GET", f"/fall/{fall_id}/stand", None)
    assert status == 200
    _val("stand", resp)
    assert resp["ring_gesperrt"] is None


def test_ausland_uebernachtung_nicht_ring_faehig_sperrgrund(base):
    """Guard: Kosten > 0 + Ausland (im_inland=False) → Sperrgrund."""
    status, resp = _req(base, "POST", "/fall", {"scheibe": "an_gesamt", "veranlagungszeitraum": "2025", "fall_id": "ueb-3"})
    assert status == 201
    fall_id = resp["fall_id"]
    status, resp = _req(base, "POST", f"/fall/{fall_id}/event", _laie("uebernachtung_kosten_monat", 100000))
    assert status == 201
    status, resp = _req(base, "POST", f"/fall/{fall_id}/event", _laie("uebernachtung_im_inland", False))
    assert status == 201
    status, resp = _req(base, "GET", f"/fall/{fall_id}/stand", None)
    assert status == 200
    _val("stand", resp)
    assert resp["ring_gesperrt"] == "ausland_uebernachtung_nicht_ring_faehig"
