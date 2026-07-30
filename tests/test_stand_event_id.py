"""stand-Endpunkt liefert event_id je Feld — erlaubt Überschreiben via ersetzt-Parameter.

Fail-Closed-Garantie: ein aktives Event kann nur mit event_id als ersetzt überschrieben werden.
GET /fall/{id}/stand war blind (kein event_id) — Korrektur erlaubt UI, Feld zu korrigieren.
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


def test_stand_liefert_event_id(base):
    """stand-Endpunkt enthält event_id für jedes belegte Feld."""
    # Fall anlegen
    status, resp = _req(base, "POST", "/fall", {"scheibe": "gesamt", "veranlagungszeitraum": "2025", "fall_id": "test-1"})
    assert status == 201, f"fall_anlegen fehlgeschlagen: {resp}"
    fall_id = resp["fall_id"]

    # Feld setzen
    ev_resp = _laie("einkuenfte_gewinn", 10000)
    status, resp = _req(base, "POST", f"/fall/{fall_id}/event", ev_resp)
    assert status == 201, f"event-POST fehlgeschlagen: {resp}"
    event_id = resp["event_id"]
    assert event_id is not None

    # /stand abrufen
    status, resp = _req(base, "GET", f"/fall/{fall_id}/stand", None)
    assert status == 200
    _val("stand", resp)

    # event_id im Feld vorhanden
    assert "einkuenfte_gewinn" in resp["felder"]
    feld = resp["felder"]["einkuenfte_gewinn"]
    assert "event_id" in feld, "event_id fehlt in /stand-Feld"
    assert feld["event_id"] == event_id, f"event_id stimmt nicht überein: {feld['event_id']} != {event_id}"


def test_stand_event_id_gleich_warum(base):
    """event_id aus /stand ist DIESELBE wie in /warum/justification."""
    # Fall anlegen
    status, resp = _req(base, "POST", "/fall", {"scheibe": "gesamt", "veranlagungszeitraum": "2025", "fall_id": "test-2"})
    assert status == 201
    fall_id = resp["fall_id"]

    # Feld setzen
    status, resp = _req(base, "POST", f"/fall/{fall_id}/event", _laie("einkuenfte_gewinn", 50000))
    assert status == 201
    event_id_event = resp["event_id"]

    # /stand event_id
    status, resp_stand = _req(base, "GET", f"/fall/{fall_id}/stand", None)
    assert status == 200
    event_id_stand = resp_stand["felder"]["einkuenfte_gewinn"]["event_id"]

    # /warum justification.event_id
    status, resp_warum = _req(base, "GET", f"/fall/{fall_id}/feld/einkuenfte_gewinn/warum", None)
    assert status == 200
    event_id_warum = resp_warum["justification"]["event_id"]

    # Alle drei sind identisch
    assert event_id_event == event_id_stand == event_id_warum


def test_stand_event_id_null_fuer_unbelegte_felder(base):
    """Unbelegte Felder haben event_id = null."""
    # Fall anlegen (leer)
    status, resp = _req(base, "POST", "/fall", {"scheibe": "gesamt", "veranlagungszeitraum": "2025", "fall_id": "test-3"})
    assert status == 201
    fall_id = resp["fall_id"]

    # /stand abrufen
    status, resp = _req(base, "GET", f"/fall/{fall_id}/stand", None)
    assert status == 200
    _val("stand", resp)

    # Unbelegte Felder müssen event_id=null haben
    if resp["felder"]:  # Falls überhaupt Felder vorhanden sind
        for fid, feld in resp["felder"].items():
            # event_id muss in jedem Feld vorhanden sein, ist aber null falls unbelegte
            assert "event_id" in feld
            if feld.get("zustand") is None or feld["zustand"] != "bestaetigt":
                # Unbelegtes Feld → event_id sollte null sein
                assert feld["event_id"] is None, f"Unbelegtes Feld {fid} hat event_id={feld['event_id']}"


def test_fehler_ohne_ersetzt_bleibt_fail_closed(base):
    """Ohne ersetzt-Parameter schlägt Überschreiben fehl (fail-closed)."""
    # Fall anlegen
    status, resp = _req(base, "POST", "/fall", {"scheibe": "gesamt", "veranlagungszeitraum": "2025", "fall_id": "test-4"})
    assert status == 201
    fall_id = resp["fall_id"]

    # Erstes Event
    status, resp = _req(base, "POST", f"/fall/{fall_id}/event", _laie("einkuenfte_gewinn", 65000))
    assert status == 201

    # Zweites Event OHNE ersetzt → sollte 422 sein
    status, resp2 = _req(base, "POST", f"/fall/{fall_id}/event", _laie("einkuenfte_gewinn", 70000))
    assert status == 422, f"Sollte 422 sein, aber war {status}: {resp2}"


def test_ersetzt_mit_event_id_erlaubt_ueberschreiben(base):
    """Mit ersetzt=event_id gelingt Überschreiben (E2E fail-closed-Mechanik)."""
    # Fall anlegen
    status, resp = _req(base, "POST", "/fall", {"scheibe": "gesamt", "veranlagungszeitraum": "2025", "fall_id": "test-5"})
    assert status == 201
    fall_id = resp["fall_id"]

    # Erstes Event
    status, resp1 = _req(base, "POST", f"/fall/{fall_id}/event", _laie("einkuenfte_gewinn", 65000))
    assert status == 201
    event_id_1 = resp1["event_id"]

    # /stand → event_id auslesen
    status, resp_stand = _req(base, "GET", f"/fall/{fall_id}/stand", None)
    assert status == 200
    event_id_from_stand = resp_stand["felder"]["einkuenfte_gewinn"]["event_id"]
    assert event_id_from_stand == event_id_1

    # Zweites Event MIT ersetzt (aus /stand!) → sollte 201 sein
    ev_body = _laie("einkuenfte_gewinn", 70000)
    ev_body["ersetzt"] = event_id_from_stand
    status, resp2 = _req(base, "POST", f"/fall/{fall_id}/event", ev_body)
    assert status == 201, f"Sollte 201 sein, aber war {status}: {resp2}"
    event_id_2 = resp2["event_id"]

    # /stand erneut → NEUE event_id
    status, resp_stand2 = _req(base, "GET", f"/fall/{fall_id}/stand", None)
    assert status == 200
    event_id_from_stand2 = resp_stand2["felder"]["einkuenfte_gewinn"]["event_id"]
    assert event_id_from_stand2 == event_id_2, "Nach Überschreiben sollte /stand neue event_id zeigen"
    assert event_id_from_stand2 != event_id_1, "Neue event_id sollte sich vom alten unterscheiden"

    # Wert aktualisiert
    assert resp_stand2["felder"]["einkuenfte_gewinn"]["wert"] == 70000


def test_stand_event_id_aequivalent_zu_aktives(base):
    """Äquivalenz: /stand.event_id == ST._aktives()[fid]["event_id"] bei mehreren Korrektionen.

    Nagelt fest: die Logik in api.py::stand() muss exakt mit ST._aktives() synchron sein.
    Unterschiede bedeuten Korrektheit-Drift (wie die DBA-Enum-Naht).
    """
    import sys, json
    sys.path.insert(0, os.path.join(ROOT, "produkt", "store"))
    import store as ST_module

    # Fall anlegen
    status, resp = _req(base, "POST", "/fall", {"scheibe": "gesamt", "veranlagungszeitraum": "2025", "fall_id": "test-6"})
    assert status == 201
    fall_id = resp["fall_id"]

    # Event 1
    status, resp1 = _req(base, "POST", f"/fall/{fall_id}/event", _laie("einkuenfte_gewinn", 10000))
    assert status == 201
    event_id_1 = resp1["event_id"]

    # Event 2 (überschreibt Event 1)
    ev_body2 = _laie("einkuenfte_gewinn", 20000)
    ev_body2["ersetzt"] = event_id_1
    status, resp2 = _req(base, "POST", f"/fall/{fall_id}/event", ev_body2)
    assert status == 201
    event_id_2 = resp2["event_id"]

    # Event 3 (überschreibt Event 2)
    ev_body3 = _laie("einkuenfte_gewinn", 30000)
    ev_body3["ersetzt"] = event_id_2
    status, resp3 = _req(base, "POST", f"/fall/{fall_id}/event", ev_body3)
    assert status == 201
    event_id_3 = resp3["event_id"]

    # /stand abrufen
    status, resp_stand = _req(base, "GET", f"/fall/{fall_id}/stand", None)
    assert status == 200
    event_id_from_stand = resp_stand["felder"]["einkuenfte_gewinn"]["event_id"]

    # Store über API.lade_fall laden + _aktives() aufrufen
    store = API.lade_fall(fall_id)
    aktiv = ST_module._aktives(store)
    event_id_from_aktives = aktiv["einkuenfte_gewinn"]["event_id"]

    # Äquivalenz nageln fest
    assert event_id_from_stand == event_id_from_aktives, \
        f"Drift: /stand={event_id_from_stand} != _aktives()={event_id_from_aktives}"
    assert event_id_from_stand == event_id_3, "Nach 3 Korrektionen sollte neueste event_id vorhanden sein"
