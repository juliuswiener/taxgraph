"""ors_client (OpenRouteService) — Parse-Logik gegen HAND-KONSTRUIERTE synthetische Antworten.

KEIN Live-Aufruf: urllib.request.urlopen wird gemockt (Fixture-Replay, $0, kein PII). Prüft, dass der
Client die dokumentierte ORS-Antwortform (geocode features + directions routes.summary.distance) korrekt
zu km faltet, und dass fehlender Key / leere Antwort SAUBER auf OrsNichtVerfuegbar fällt (nie Crash).
"""
import io
import json
import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, os.path.join(ROOT, "produkt", "haut"))

import ors_client   # noqa: E402


class _FakeResp(io.BytesIO):
    def __enter__(self): return self
    def __exit__(self, *a): return False


def _fake_urlopen_factory(geocode_coords, distanz_m):
    """Gibt eine urlopen-Attrappe zurück, die je nach URL geocode- bzw. directions-JSON liefert."""
    def _fake(req, timeout=None):
        url = req.full_url
        if "/geocode/search" in url:
            body = {"features": [{"geometry": {"coordinates": geocode_coords}}]}
        elif "/v2/directions/" in url:
            body = {"routes": [{"summary": {"distance": distanz_m}}]}
        else:
            body = {}
        return _FakeResp(json.dumps(body).encode("utf-8"))
    return _fake


def test_entfernung_km_faltet_synthetische_antwort(monkeypatch):
    monkeypatch.setenv("ORS_API_KEY", "SYNTHETISCHER_TEST_KEY")
    monkeypatch.setattr(ors_client.urllib.request, "urlopen",
                        _fake_urlopen_factory([13.40, 52.52], 30000.0))
    # 30.000 m -> 30 km (kürzeste Straßenverbindung, ganzzahlig gerundet)
    assert ors_client.entfernung_km("Musterstraße 1, Berlin", "Beispielweg 2, Berlin") == 30


def test_rundung_auf_ganze_km(monkeypatch):
    monkeypatch.setenv("ORS_API_KEY", "K")
    monkeypatch.setattr(ors_client.urllib.request, "urlopen",
                        _fake_urlopen_factory([13.0, 52.0], 30499.0))
    assert ors_client.entfernung_km("a", "b") == 30       # 30,499 km -> 30
    monkeypatch.setattr(ors_client.urllib.request, "urlopen",
                        _fake_urlopen_factory([13.0, 52.0], 30500.0))
    assert ors_client.entfernung_km("a", "b") == 30       # bankers rounding (30,5 -> 30)


def test_kein_key_faellt_sauber(monkeypatch):
    monkeypatch.delenv("ORS_API_KEY", raising=False)
    with pytest.raises(ors_client.OrsNichtVerfuegbar):
        ors_client.entfernung_km("a", "b")


def test_leere_geocode_antwort_faellt_sauber(monkeypatch):
    monkeypatch.setenv("ORS_API_KEY", "K")
    def _leer(req, timeout=None):
        return _FakeResp(json.dumps({"features": []}).encode("utf-8"))
    monkeypatch.setattr(ors_client.urllib.request, "urlopen", _leer)
    with pytest.raises(ors_client.OrsNichtVerfuegbar):
        ors_client.entfernung_km("a", "b")


def test_malformte_geometrie_faellt_sauber_nicht_keyerror(monkeypatch):
    """Regression: features vorhanden, aber geometry/coordinates fehlt oder ist unbrauchbar (z. B. ORS
    liefert ein Feature ohne Punktgeometrie) — MUSS OrsNichtVerfuegbar werfen, nie KeyError/TypeError.
    Voraussetzung dafür, dass api.py den except-Block auf (OrsNichtVerfuegbar, ImportError) verengen darf."""
    monkeypatch.setenv("ORS_API_KEY", "K")
    def _malformt(req, timeout=None):
        return _FakeResp(json.dumps({"features": [{"geometry": {}}]}).encode("utf-8"))
    monkeypatch.setattr(ors_client.urllib.request, "urlopen", _malformt)
    with pytest.raises(ors_client.OrsNichtVerfuegbar):
        ors_client.geocode("a")
