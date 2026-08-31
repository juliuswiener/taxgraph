"""POST /fall darf einen veranlagungszeitraum ohne params/<jahr>/ nicht annehmen (Nachbar-Befund,
2026-08-31): vz=2099 wurde bisher mit 201 angelegt, das spaetere Laden von params/2099/... schlug
still fehl und die Deklaration zeigte E1901401=None bei eingaben_konsistent=true. Die Menge der
gueltigen Jahre wird aus params/ GELESEN (nicht hier hartkodiert), sonst muesste dieser Test bei
jedem neuen Jahresordner nachgezogen werden."""
from __future__ import annotations

import os
import sys
import threading

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/haut", "golden"):
    sys.path.insert(0, os.path.join(ROOT, sub))
sys.path.insert(0, os.path.join(ROOT, "produkt", "store"))

import api as API        # noqa: E402
import server as SRV     # noqa: E402
import audit              # noqa: E402

from test_paket_b_e2e_http import _req  # noqa: E402 — gleicher HTTP-Helfer wie der Rest der Suite


@pytest.fixture
def base(tmp_path, monkeypatch):
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


def _verfuegbare_jahre() -> set[int]:
    params_dir = os.path.join(ROOT, "params")
    return {int(n) for n in os.listdir(params_dir) if n.isdigit()}


def test_vz_ohne_params_wird_abgelehnt(base):
    """vz=2099 hat keinen params/2099/-Ordner -> 400, kein Fall entsteht."""
    assert 2099 not in _verfuegbare_jahre(), "Testannahme verletzt: params/2099/ existiert bereits"
    st, body = _req(base, "POST", "/fall",
                     {"scheibe": "ep", "veranlagungszeitraum": 2099, "fall_id": "vzguard_2099"},
                     erwarte=400)
    assert "veranlagungszeitraum" in body["fehler"]
    st2, _ = _req(base, "GET", "/fall/vzguard_2099/stand", erwarte=404)


def test_vz_mit_params_bleibt_erlaubt(base):
    """Kontrollzeile: ein Jahr MIT params/-Ordner wird weiter angenommen."""
    jahre = _verfuegbare_jahre()
    assert jahre, "Testannahme verletzt: params/ ist leer"
    vz = sorted(jahre)[0]
    st, body = _req(base, "POST", "/fall",
                     {"scheibe": "ep", "veranlagungszeitraum": vz, "fall_id": "vzguard_gueltig"},
                     erwarte=201)
    assert body["veranlagungszeitraum"] == vz


def test_vz_negativ_wird_abgelehnt(base):
    _req(base, "POST", "/fall",
         {"scheibe": "ep", "veranlagungszeitraum": -5, "fall_id": "vzguard_neg"}, erwarte=400)


def test_vz_leerer_string_400_nicht_500(base):
    """Vorher: nackter ValueError -> 500. Derselbe int()-Aufruf wird jetzt gefangen."""
    _req(base, "POST", "/fall",
         {"scheibe": "ep", "veranlagungszeitraum": "", "fall_id": "vzguard_leer"}, erwarte=400)


def test_vz_null_400_nicht_500(base):
    _req(base, "POST", "/fall",
         {"scheibe": "ep", "veranlagungszeitraum": None, "fall_id": "vzguard_null"}, erwarte=400)


def test_vz_als_string_zahl_bleibt_erlaubt(base):
    """'2025' (Zeichenkette) wird weiter akzeptiert -- int() konvertiert, die Jahr-Pruefung greift danach."""
    jahre = _verfuegbare_jahre()
    vz = sorted(jahre)[0]
    st, body = _req(base, "POST", "/fall",
                     {"scheibe": "ep", "veranlagungszeitraum": str(vz), "fall_id": "vzguard_strzahl"},
                     erwarte=201)
    assert body["veranlagungszeitraum"] == vz
