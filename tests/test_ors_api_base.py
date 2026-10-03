"""`$ORS_API_BASE` lenkt den Karten-Dienst um — nur dafür da, dass der Vergleichslauf (rust/parity)
einen lokalen Stub statt `api.openrouteservice.org` anspricht.

Vault `decisions/rust-9c-generator-je-route-und-flow-portieren` Punkt 4, Bericht
`berichte/haertung8.md` Folge 6. Vorher war die Basis eine Modulkonstante ohne Umgebungsvariable.

Der Test läuft gegen einen echten lokalen HTTP-Server und liest, was Python dorthin sendet. Der
Schlüssel ist synthetisch. Er darf im Query des Geocodings und im Header des Routings stehen (so
sendet ihn ors_client) — nirgends sonst.
"""
from __future__ import annotations

import json
import os
import sys
import threading
from http.server import BaseHTTPRequestHandler, HTTPServer

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "produkt", "haut"))

import ors_client as OC  # noqa: E402

SCHLUESSEL = "SYNTHETISCH-ORS-SCHLUESSEL"


class _Stub:
    """Antwortet der Reihe nach mit `skript` [(status, body)]; merkt sich jede Anfrage."""

    def __init__(self, skript):
        self.skript, self.gesehen = list(skript), []
        stub = self

        class H(BaseHTTPRequestHandler):
            def _antworte(self):
                laenge = int(self.headers.get("Content-Length") or 0)
                roh = self.rfile.read(laenge).decode("utf-8") if laenge else ""
                stub.gesehen.append({"methode": self.command, "pfad": self.path,
                                     "authorization": self.headers.get("Authorization"),
                                     "content_type": self.headers.get("Content-Type"), "body": roh})
                status, body = stub.skript.pop(0)
                daten = body.encode("utf-8")
                self.send_response(status)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(daten)))
                self.end_headers()
                self.wfile.write(daten)

            do_GET = do_POST = _antworte

            def log_message(self, *a):
                pass

        self.srv = HTTPServer(("127.0.0.1", 0), H)
        self.url = f"http://127.0.0.1:{self.srv.server_address[1]}"
        threading.Thread(target=self.srv.serve_forever, daemon=True).start()

    def schliessen(self):
        self.srv.shutdown()
        self.srv.server_close()


@pytest.fixture
def stub():
    gebaut = []

    def bauen(skript):
        s = _Stub(skript)
        gebaut.append(s)
        return s

    yield bauen
    for s in gebaut:
        s.schliessen()


_GEO = json.dumps({"features": [{"geometry": {"coordinates": [11.5, 48.1]}}]})
_ROUTE = json.dumps({"routes": [{"summary": {"distance": 30400.0}}]})


def test_ohne_variable_geht_es_an_den_standard(monkeypatch):
    monkeypatch.delenv("ORS_API_BASE", raising=False)
    assert OC._basis() == "https://api.openrouteservice.org"
    for leer in ("", "   ", "/"):
        monkeypatch.setenv("ORS_API_BASE", leer)
        assert OC._basis() == "https://api.openrouteservice.org", repr(leer)


def test_die_variable_lenkt_alle_drei_aufrufe_um(stub, monkeypatch):
    s = stub([(200, _GEO), (200, _GEO), (200, _ROUTE)])
    monkeypatch.setenv("ORS_API_KEY", SCHLUESSEL)
    monkeypatch.setenv("ORS_API_BASE", s.url + "/")        # Schrägstrich am Ende ist egal

    assert OC.entfernung_km("Musterstr. 1, 80331 München", "Beispielweg 2, 80333 München") == 30

    assert [(g["methode"], g["pfad"].split("?")[0]) for g in s.gesehen] == [
        ("GET", "/geocode/search"), ("GET", "/geocode/search"), ("POST", "/v2/directions/driving-car")]
    assert s.gesehen[0]["pfad"] == (
        f"/geocode/search?api_key={SCHLUESSEL}&text=Musterstr.+1%2C+80331+M%C3%BCnchen"
        "&size=1&boundary.country=DE")
    route = s.gesehen[2]
    assert route["authorization"] == SCHLUESSEL and route["content_type"] == "application/json"
    assert route["body"] == ('{"coordinates": [[11.5, 48.1], [11.5, 48.1]], '
                             '"preference": "shortest", "units": "m"}')


def test_ein_fehler_des_stubs_nennt_weder_schluessel_noch_url(stub, monkeypatch):
    s = stub([(500, "{}")])
    monkeypatch.setenv("ORS_API_KEY", SCHLUESSEL)
    monkeypatch.setenv("ORS_API_BASE", s.url)

    with pytest.raises(OC.OrsNichtVerfuegbar) as e:
        OC.geocode("irgendwo")
    text = f"{e.value} {e.value!r} {e.value.__cause__!r} {e.value.__context__!r}"
    assert SCHLUESSEL not in text and "127.0.0.1" not in text, text
    assert str(e.value) == "ORS-Aufruf fehlgeschlagen: HTTPError"
