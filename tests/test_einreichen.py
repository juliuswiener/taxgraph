"""P3.2b POST /fall/{id}/einreichen — Deklaration → XML → checkESt. KEIN Versand.

Der Endpunkt validiert nur (ERIC_VALIDIERE, offline). Diese Tests decken vor allem die
fail-closed-Kanten ab, die ohne ERiC-Installation prüfbar sind: unvollständige Deklaration
sperrt, fehlende ERiC-Bibliothek meldet 503 statt zu crashen, und der Erfolgspfad behauptet
NIE, etwas eingereicht zu haben.
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
for sub in ("produkt/haut", "produkt/import", "golden"):
    sys.path.insert(0, os.path.join(ROOT, sub))
sys.path.insert(0, os.path.join(ROOT, "produkt", "store"))

import api as API        # noqa: E402
import server as SRV     # noqa: E402


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
        with urllib.request.urlopen(req, timeout=30) as r:
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
            "schreiber": "ui:laie",
            "signal": {"signal_1": {"typ": "laie_eingabe"}, "signal_2": "laie_bestaetigt"}}


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


def _fall(base, scheibe="an_gesamt", vz=2025):
    st, r = _req(base, "POST", "/fall", {"fall_id": "tg1", "scheibe": scheibe,
                                        "veranlagungszeitraum": vz})
    assert st == 201, r
    return r["fall_id"]


# ----------------------------------------------------------------- fail-closed

def test_leerer_fall_wird_nicht_eingereicht(base):
    """Ohne bestätigte Pflichtfelder ist die Deklaration unvollständig → 409, kein XML."""
    fid = _fall(base)
    st, r = _req(base, "POST", f"/fall/{fid}/einreichen", {}, erwarte=422)
    assert r["eingereicht"] is False


def test_unbekannter_fall_404(base):
    st, r = _req(base, "POST", "/fall/gibtsnicht/einreichen", {}, erwarte=404)


def test_antwort_behauptet_nie_versand(base):
    """Egal welcher Ausgang — 'eingereicht' ist immer False, solange Versand nicht verdrahtet ist."""
    fid = _fall(base)
    st, r = _req(base, "POST", f"/fall/{fid}/einreichen", {}, erwarte=422)
    assert r.get("eingereicht") is False
    assert "versendet" not in json.dumps(r).lower()


def test_grund_ist_maschinenlesbar(base):
    """Fehlerantworten tragen einen stabilen grund-Code, nicht nur Prosa."""
    fid = _fall(base)
    _st, r = _req(base, "POST", f"/fall/{fid}/einreichen", {}, erwarte=422)
    assert r["grund"] in {"deklaration_unvollstaendig", "xml_nicht_baubar",
                          "eric_nicht_verfuegbar", "plausibilitaet_verletzt"}


def test_unvollstaendig_nennt_die_offenen_felder(base):
    """409 muss sagen WELCHES Feld fehlt — sonst ist die Antwort für den Nutzer wertlos.

    Der Wert wird VORLAEUFIG gesetzt (Zwei-Signal unvollständig): nur dann ist die
    Deklaration unvollständig. Ein bestätigter bruttoarbeitslohn macht sie vollständig,
    der Lauf käme bis zum XML-Bau und stürbe dort an der fehlenden Hersteller-ID — der
    Test hätte seine eigene Prämisse nie hergestellt und 409 nie gesehen.
    """
    fid = _fall(base)
    vorlaeufig = dict(_laie("bruttoarbeitslohn", 4500000),
                      zustand="vorlaeufig",
                      signal={"signal_1": {"typ": "laie_eingabe"}, "signal_2": None})
    _req(base, "POST", f"/fall/{fid}/event", vorlaeufig)

    st, r = _req(base, "POST", f"/fall/{fid}/einreichen", {}, erwarte=409)
    assert r["grund"] == "deklaration_unvollstaendig", r
    offen = r["unvollstaendig"]
    assert isinstance(offen, list) and offen, "409 ohne die offenen Felder ist wertlos"
    assert any(e.get("feld_id") == "bruttoarbeitslohn" for e in offen), offen


# ----------------------------------------------------------------- Einheit (ohne HTTP)

def test_vorlaeufiges_feld_blockiert_einreichen(tmp_path, monkeypatch):
    """Ein vorläufiger Wert macht die Deklaration unvollständig — fail-closed vor dem XML."""
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    _st, r = API.fall_anlegen({"fall_id": "tg1", "scheibe": "an_gesamt", "veranlagungszeitraum": 2025})
    fid = r["fall_id"]
    store = API.lade_fall(fid)
    import store as ST
    ST.append_event(store, feld_id="bruttoarbeitslohn", wert=4500000, zustand="vorlaeufig",
                    herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                    schreiber="laie", signal={"signal_1": {"typ": "laie_eingabe"}, "signal_2": None})
    API.speichere_fall(fid, store)
    st, resp = API.einreichen(fid, {})
    assert st in (409, 422)
    assert resp["eingereicht"] is False


def test_eric_fehlt_gibt_503_statt_crash(tmp_path, monkeypatch):
    """Ohne ERiC-Bibliothek: sauberes 503, kein Traceback nach aussen."""
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    import elster_xml as EX
    monkeypatch.setattr(EX, "erzeuge_xml",
                        lambda *a, **k: '<?xml version="1.0"?><Elster/>')
    monkeypatch.setattr(API.EM, "deklariere",
                        lambda *a, **k: {"vollstaendig": True, "deklaration": {"E0100201": "M"},
                                         "unvollstaendig": []})
    import checkest_gate as CE
    monkeypatch.setattr(CE, "validate",
                        lambda *a, **k: (_ for _ in ()).throw(RuntimeError("libericapi.so nicht gefunden")))
    _st, r = API.fall_anlegen({"fall_id": "tg1", "scheibe": "an_gesamt", "veranlagungszeitraum": 2025})
    st, resp = API.einreichen(r["fall_id"], {})
    assert st == 503
    assert resp["grund"] == "eric_nicht_verfuegbar"
    assert resp["eingereicht"] is False


def test_plausibilitaetsfehler_reicht_ericantwort_durch(tmp_path, monkeypatch):
    """rc!=0 → 422 mit der ERiC-Antwort, damit der Nutzer die verletzte Regel sieht."""
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    import elster_xml as EX
    monkeypatch.setattr(EX, "erzeuge_xml", lambda *a, **k: '<?xml version="1.0"?><Elster/>')
    monkeypatch.setattr(API.EM, "deklariere",
                        lambda *a, **k: {"vollstaendig": True, "deklaration": {"E0100201": "M"},
                                         "unvollstaendig": []})
    import checkest_gate as CE
    monkeypatch.setattr(CE, "validate",
                        lambda *a, **k: (CE.RC_PLAUSIBILITAET, "<FehlerRegelpruefung>E0100201</FehlerRegelpruefung>"))
    _st, r = API.fall_anlegen({"fall_id": "tg1", "scheibe": "an_gesamt", "veranlagungszeitraum": 2025})
    st, resp = API.einreichen(r["fall_id"], {})
    assert st == 422
    assert resp["grund"] == "plausibilitaet_verletzt"
    assert "E0100201" in resp["ericantwort"]
    assert resp["klasse"] == "plausibilitaet_fehler"


def test_io_gate_rc_ist_nicht_gruen(tmp_path, monkeypatch):
    """rc=610301200 liefert 0 Fehler im Puffer, ist aber NICHT geprüft — nie als grün werten."""
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    import elster_xml as EX
    monkeypatch.setattr(EX, "erzeuge_xml", lambda *a, **k: '<?xml version="1.0"?><Elster/>')
    monkeypatch.setattr(API.EM, "deklariere",
                        lambda *a, **k: {"vollstaendig": True, "deklaration": {"E0100201": "M"},
                                         "unvollstaendig": []})
    import checkest_gate as CE
    monkeypatch.setattr(CE, "validate", lambda *a, **k: (CE.RC_IO_KEIN_TICKET, ""))
    _st, r = API.fall_anlegen({"fall_id": "tg1", "scheibe": "an_gesamt", "veranlagungszeitraum": 2025})
    st, resp = API.einreichen(r["fall_id"], {})
    assert st == 422, "leerer Fehlerpuffer bei rc!=0 darf nicht als plausibel durchgehen"
    assert resp["klasse"] == "io_gate_nicht_geprueft"


def test_erfolg_meldet_plausibel_aber_nicht_eingereicht(tmp_path, monkeypatch):
    """rc==0 ist der Erfolgsfall — und selbst der sagt eingereicht=False."""
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    import elster_xml as EX
    monkeypatch.setattr(EX, "erzeuge_xml", lambda *a, **k: '<?xml version="1.0"?><Elster/>')
    monkeypatch.setattr(API.EM, "deklariere",
                        lambda *a, **k: {"vollstaendig": True, "deklaration": {"E0100201": "M"},
                                         "unvollstaendig": []})
    import checkest_gate as CE
    monkeypatch.setattr(CE, "validate", lambda *a, **k: (CE.RC_OK, ""))
    _st, r = API.fall_anlegen({"fall_id": "tg1", "scheibe": "an_gesamt", "veranlagungszeitraum": 2025})
    st, resp = API.einreichen(r["fall_id"], {})
    assert st == 200
    assert resp["plausibel"] is True
    assert resp["eingereicht"] is False
    assert "Versand ist nicht verdrahtet" in resp["hinweis"]


def test_xml_nicht_baubar_gibt_422(tmp_path, monkeypatch):
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    import elster_xml as EX
    monkeypatch.setattr(API.EM, "deklariere",
                        lambda *a, **k: {"vollstaendig": True, "deklaration": {"E9999999": "x"},
                                         "unvollstaendig": []})
    monkeypatch.setattr(EX, "erzeuge_xml",
                        lambda *a, **k: (_ for _ in ()).throw(EX.XmlFehler("Kz ohne Pfad")))
    _st, r = API.fall_anlegen({"fall_id": "tg1", "scheibe": "an_gesamt", "veranlagungszeitraum": 2025})
    st, resp = API.einreichen(r["fall_id"], {})
    assert st == 422
    assert resp["grund"] == "xml_nicht_baubar"
