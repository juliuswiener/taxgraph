"""Die Protokollzeile eines Fall-Aufrufs nennt den Status der ANTWORT und steht VOR ihr.

Vault `decisions/protokollzeile-nach-der-wirkung-vor-der-antwort` (Punkte 1 und 2), Eintrag
`backlog/taxgraph/audit-status-spalte-bleibt-auf-500`.

Vorher stand `status = 500` als Startwert im Dispatch und blieb stehen, sobald ein Handler eine
`ApiError` warf: ein doppeltes `POST /fall` bekam 409, das Protokoll sagte `status=500`. Dazu schrieb
der Dispatch die Zeile NACH `self._json`: wer das Protokoll direkt nach der Antwort las, konnte sie
verpassen.

Der Test läuft am echten Aufrufort: gestarteter Dienst, echte HTTP-Anfragen, Protokoll aus der Datei.
Er schläft nirgends. Ob die Zeile VOR der Antwort steht, misst der letzte Test deterministisch — er
liest das Protokoll in dem Augenblick, in dem der Dienst die Antwort schreibt.
"""
from __future__ import annotations

import json
import os
import sys
import threading
import urllib.error
import urllib.request

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
for _s in ("produkt/haut", "produkt/store", "produkt/auth", "produkt/traverser"):
    sys.path.insert(0, os.path.join(ROOT, _s))

import api as API          # noqa: E402
import audit               # noqa: E402
import server as SRV       # noqa: E402

FALL = {"fall_id": "a1", "scheibe": "ep", "veranlagungszeitraum": 2025}


@pytest.fixture
def base(tmp_path, monkeypatch):
    """Dienst auf freiem Port; Akten und Protokoll liegen in tmp_path."""
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    srv = SRV.make_server(0)
    th = threading.Thread(target=srv.serve_forever, daemon=True)
    th.start()
    try:
        yield f"http://{srv.server_address[0]}:{srv.server_address[1]}"
    finally:
        srv.shutdown()
        th.join(timeout=5)
        srv.server_close()


def _ruf(base: str, methode: str, pfad: str, body: dict | None = None) -> int:
    daten = json.dumps(body).encode("utf-8") if body is not None else None
    req = urllib.request.Request(base + pfad, data=daten, method=methode,
                                 headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=10) as r:
            r.read()
            return r.status
    except urllib.error.HTTPError as e:
        e.read()
        return e.code


def _letzte() -> dict:
    zeilen = audit.lies()
    assert zeilen, "das Protokoll ist leer"
    return zeilen[-1]


# (Methode, Pfad, Rumpf, erwarteter Status, erwartete Aktion)
FAELLE_STATUS = [
    ("POST", "/fall", FALL, 201, "fall_create"),
    ("POST", "/fall", FALL, 409, "fall_create"),                                   # Fall gibt es schon
    ("POST", "/fall", {**FALL, "fall_id": "a2", "scheibe": "gibt_es_nicht"}, 400, "fall_create"),
    ("GET", "/fall/gibt-es-nicht/stand", None, 404, "fall_stand"),                 # Fall unbekannt
    ("GET", "/fall/a1/fragen", None, 200, "fall_fragen"),
]


def test_die_zeile_nennt_den_status_der_antwort(base):
    """201, 409, 400, 404, 200: jede Zeile trägt, was der Client bekam. Gelesen direkt nach der
    Antwort, ohne Wartezeit."""
    ergebnis = []
    for methode, pfad, body, erwartet, aktion in FAELLE_STATUS:
        status = _ruf(base, methode, pfad, body)
        zeile = _letzte()
        ergebnis.append((status, zeile["action"], zeile["detail"]))
        assert (status, zeile["action"], zeile["detail"]) == (erwartet, aktion, f"status={erwartet}"), (
            f"{methode} {pfad}: Antwort {status}, Zeile {zeile}")
    assert [e[0] for e in ergebnis] == [201, 409, 400, 404, 200]


def test_ein_unerwarteter_fehler_steht_als_500_im_protokoll(base, monkeypatch):
    """Gegenprobe zur Gegenrichtung: wo die Antwort 500 ist, bleibt es 500. Der Fix darf den
    Status nicht aus der Antwort „erraten", sondern muss ihn spiegeln."""
    def _platzt(_fall_id):
        raise ValueError("egal")

    monkeypatch.setattr(API, "stand", _platzt)
    assert _ruf(base, "GET", "/fall/abc123/stand") == 500
    zeile = _letzte()
    assert (zeile["action"], zeile["fall_id"], zeile["detail"]) == ("fall_stand", "abc123", "status=500")


def test_die_zeile_steht_im_protokoll_wenn_die_antwort_geschrieben_wird(base, monkeypatch):
    """Reihenfolge, deterministisch: der Dienst schreibt die Antwort mit `Handler._json`. Zu diesem
    Zeitpunkt muss die Zeile schon in der Datei stehen (nach der Wirkung, VOR der Antwort). Wer die
    Zeile hinter `self._json` zurückstellt, macht diesen Test rot — ohne dass ein Zufall helfen muss."""
    gesehen: list[list[str]] = []
    echt = SRV.Handler._json

    def _json_mit_blick(self, status, obj):
        gesehen.append([f"{z['action']}:{z['detail']}" for z in audit.lies()])
        return echt(self, status, obj)

    monkeypatch.setattr(SRV.Handler, "_json", _json_mit_blick)
    assert _ruf(base, "POST", "/fall", FALL) == 201
    assert gesehen == [["fall_angelegt:scheibe=ep", "fall_create:status=201"]], gesehen


def test_scheitert_das_schreiben_geht_die_antwort_trotzdem_raus(base, monkeypatch):
    """Das Protokoll ist ein Nebenkanal: bei einem Schreibfehler antwortet der Dienst wie sonst und
    meldet den Fehler im Fehlerlog (Entscheidung Punkt 1) — nicht als 500 an den Client."""
    import fehler_log

    echt = audit.append

    def _append(user_id, action, fall_id=None, detail=None):
        if action == "fall_create":
            raise OSError("Platte voll")
        return echt(user_id, action, fall_id, detail)

    monkeypatch.setattr(audit, "append", _append)
    assert _ruf(base, "POST", "/fall", FALL) == 201
    eintrag = fehler_log.lies()[-1]
    assert (eintrag["ort"], eintrag["typ"], eintrag["fall_id"]) == ("server.audit", "OSError", None)
