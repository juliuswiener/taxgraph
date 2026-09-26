"""Erreichbarkeits-Gate: Sperrgrund-Tests für _an_gesamt_sperrgrund().

Teste die Guards, die verhindern, dass der Ring Mitveranlagung berechnet:
- uebernachtung_tatbestand_offen (Kosten > 0, aber Ort/Bedingungen nicht bestätigt)
- uebernachtung_zeitraum_offen (monate/monate_bisher nicht bestätigte int)

Wichtig: beide Richtungen testen (sperrt vs. sperrt nicht). Ausland sperrt NICHT: die ersten
48 Monate unterscheiden nach § 9 Abs. 1 S. 3 Nr. 5a Sätze 1-3 nicht nach dem Ort. Die frühere
Sperre ausland_uebernachtung_nicht_ring_faehig ist entfernt.
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
import audit                # noqa: E402
from bescheid_deklaration import sperrgrund_klartext  # noqa: E402

jsonschema = pytest.importorskip("jsonschema")
SCHEMA_DIR = os.path.join(ROOT, "produkt", "haut", "api_schema")


def _schema(name: str) -> dict:
    with open(os.path.join(SCHEMA_DIR, f"{name}.json"), encoding="utf-8") as f:
        return json.load(f)


def _val(name: str, obj: dict) -> None:
    jsonschema.Draft202012Validator(_schema(name)).validate(obj)


def _req(base: str, method: str, path: str, body: dict | None = None,
         erwarte: int | None = None):
    """HTTP-Request mit optionalem Status-Check.

    Prüft selbst:
    - 5xx → AssertionError (nie unterdrückbar)
    - 4xx → AssertionError, es sei denn `erwarte=<code>` ist gesetzt
    - 2xx → durch
    - erwarte=N → assert status == N
    """
    data = json.dumps(body).encode("utf-8") if body is not None else None
    req = urllib.request.Request(base + path, data=data, method=method,
                                 headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=10) as r:
            status = r.status
            content = json.loads(r.read())
    except urllib.error.HTTPError as e:
        status = e.code
        content = json.loads(e.read())
    if erwarte is not None:
        assert status == erwarte, (
            f"erwarte={erwarte}, erhalten={status} {method} {path} {body}")
    elif status >= 500:
        raise AssertionError(
            f"Serverfehler {status} {method} {path} {body}: {content}")
    elif status >= 400:
        raise AssertionError(
            f"Fehler {status} {method} {path} {body}: {content}")
    return status, content


def _laie(fld, w):
    return {"feld_id": fld, "wert": w, "zustand": "bestaetigt",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "ui:laie", "signal": {"signal_1": None, "signal_2": f"ok@{fld}"}}


@pytest.fixture
def base(tmp_path, monkeypatch):
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
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
    assert resp["ring_gesperrt_klartext"] == sperrgrund_klartext(
        "uebernachtung_tatbestand_offen")


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
    assert resp["ring_gesperrt_klartext"] is None


def test_ausland_uebernachtung_bleibt_im_dialog_und_der_tatbestand_sperrt(base):
    """Ein bestätigtes „Ausland" darf die Regel NICHT abschalten — der Sperrgrund bleibt sichtbar.

    Der Defekt, den dieser Test festhält (gemessen 2026-09-26): `uebernachtung_im_inland` hing als
    Gate an `p9_1_3_nr5a_uebernachtung_nach_48`. Ein bestätigtes „Ausland" setzte die Regel auf
    „ausgeschlossen" — ALLE sieben Fragen fielen aus der Queue, auch die Kostenfrage. Der Guard
    prüft aber Kosten > 0: ohne Kostenfrage sperrt er nie, es gibt also WEDER Zahl NOCH Sperrgrund.
    Der Abzug verschwand still. Die Backlog-Messung sah eine Sperre, weil sie die Felder direkt per
    POST setzte und den Dialog übersprang.

    Zwei Hälften, beide im Nutzerpfad (/fragen bzw. /stand):
    (1) Der Ort schaltet nichts ab — die Regel-Felder bleiben in der Queue.
    (2) Ein UNVOLLSTÄNDIGER Auslandsfall sperrt weiter sichtbar (Tatbestand offen), statt still
        durchzurechnen. Der Ort selbst ist kein Sperrgrund mehr: mit allen drei Bedingungen
        bestätigt rechnet der Ring (siehe test_ring_regression_kampagne, Ausland-Tests).
    """
    from tests.test_ring_regression_kampagne import _ueb, GESAMT_AN_KEGEL, _an_anlegen

    # (1) Nur die Eingangsfrage + Ort beantwortet: die Kostenfrage MUSS weiter gefragt werden.
    status, resp = _req(base, "POST", "/fall", {"scheibe": "an_gesamt", "veranlagungszeitraum": "2025", "fall_id": "ueb-3"})
    assert status == 201
    fall_id = resp["fall_id"]
    for feld, wert in (("uebernachtung_auswaerts", True), ("uebernachtung_im_inland", False)):
        status, resp = _req(base, "POST", f"/fall/{fall_id}/event", _laie(feld, wert))
        assert status == 201
    status, resp = _req(base, "GET", f"/fall/{fall_id}/fragen", None)
    assert status == 200
    offen = [q["feld_id"] for q in resp["fragen"]]
    assert "uebernachtung_kosten_monat" in offen, (
        'Die Kostenfrage fehlt in der Queue, nachdem der Ort mit "Ausland" beantwortet wurde — '
        f"das Gate schaltet die Regel ab, der Abzug fällt still weg. Queue: {offen}")

    # (2) Kosten eingetragen, Tatbestand unvollständig (Alleinnutzung/Unterbrechung fehlen):
    # der Sperrgrund steht sichtbar auf /stand.
    status, resp = _req(base, "POST", f"/fall/{fall_id}/event", _laie("uebernachtung_kosten_monat", 100000))
    assert status == 201
    status, resp = _req(base, "GET", f"/fall/{fall_id}/stand", None)
    assert status == 200
    _val("stand", resp)
    assert resp["ring_gesperrt"] == "uebernachtung_tatbestand_offen", (
        f"ring_gesperrt={resp.get('ring_gesperrt')!r}")
    assert resp["ring_gesperrt_klartext"] == sperrgrund_klartext("uebernachtung_tatbestand_offen")


def test_verpflegung_dreimonats_felder_erreichbar(base):
    """Regressions-Wächter: vpf_tage_*_nach_drei_monaten müssen über POST 201 erreichbar sein.
    Tote Bindung ohne Scheibe (4.Vorkommen heute).
    """
    from tests.test_paket_b_e2e_http import _gesamt_kegel, _gesamt_anlegen, _req, _laie
    
    kegel = _gesamt_kegel(0, bruttolohn=5000000)
    _gesamt_anlegen(base, "vpf-drei", kegel)
    
    # Die drei Dreimonatsfrist-Felder
    felder_zu_pruefen = [
        "vpf_tage_24h_nach_drei_monaten",
        "vpf_tage_an_abreise_nach_drei_monaten",
        "vpf_tage_ueber_8h_nach_drei_monaten"
    ]
    
    for feld_id in felder_zu_pruefen:
        st, resp = _req(base, "POST", "/fall/vpf-drei/event", _laie(feld_id, 0))
        assert st == 201, f"{feld_id}: POST {st} statt 201 — tote Bindung?"
