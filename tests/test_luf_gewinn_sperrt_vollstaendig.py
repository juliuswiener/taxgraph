"""Reparatur (Instructor-Auftrag, 2026-08-31): ein bestätigtes Geldfeld, das im Klasse-f-Zweig
(VERZWEIGUNG, est_mapping.py) keinen Kz-Ast findet, ist UNSERE Zuordnungslücke, kein Widerspruch
in den Nutzereingaben. Es muss `vollstaendig`/`eingaben_konsistent` kippen (fail-closed), statt
nur in `nicht_deklariert` zu verschwinden (siehe tests/test_luf_gewinn_kz_fehlt.py — der Test dort
ist bewusst xfail(strict, raises=AssertionError), er prüft die fehlende Kennzahl selbst, nicht
diese Sperre).

Kontrollzeile (gewerbe) MUSS `vollstaendig=True` bleiben: die Sperre darf NUR den unbekannten
Kz-Zweig treffen, nicht die sieben strukturell harmlosen `nicht_deklariert`-Formen (Anwendbarkeits-
Flags, Weichen), die bei JEDER Betriebsart in der Liste stehen. Eine Sperre, die jeden trifft, ist
keine Reparatur.

KEIN xfail HIER: beide Fälle sollen so aussehen, wie sie sind — dieser Test ist heute korrekt grün,
nicht rot, das Xfail sitzt in der Schwesterdatei.

ACHTUNG Widerspruch, nicht zwei Haelften einer Absicht (Recherche-Worker 2026-08-31): dieser Test
und tests/test_luf_gewinn_kz_fehlt.py speisen dieselbe Eingabe (land_forst, 30.000 EUR,
"Testhof") in denselben Endpunkt und verlangen ENTGEGENGESETZTE Endzustaende. Dieser Test hier
verlangt `vollstaendig=False` — der Betrag darf NIRGENDS auftauchen. Der xfail-Test in der
Schwesterdatei verlangt, dass der Betrag UNTER EINEM Kz auftaucht. Kein kuenftiger Produktzustand
erfuellt beide zugleich: sobald jemand den fehlenden Anlage-L-Kz-Zweig baut (Gewinnermittlungsart
+ Wirtschaftsjahr-Feld, s. dortiger Moduldocstring), MUSS der land_forst-Testfall hier in DIESER
Datei mit umgeschrieben/entfernt werden — `vollstaendig` kippt dann planmaessig auf True, und der
land_forst-Block unten wird planmaessig rot. Das ist dann Folgerichtigkeit, keine Regression."""
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

GEWINN_CENT = 3000000


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


def _bestaetigt(fld, w):
    return {"feld_id": fld, "wert": w, "zustand": "bestaetigt",
            "herkunft": {"herkunft": "laie"}, "schreiber": "ui:test",
            "signal": {"signal_2": "ui:bestaetigt"}}


def _erklaere_gewinn(base, fall_id, betriebsart, bezeichnung):
    """Echter Nutzerpfad auf der Scheibe `gesamt` (die einzige Kachel mit Gewinnfeldern)."""
    _req(base, "POST", "/fall",
         {"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": fall_id}, erwarte=201)
    for fld, w in (
        ("veranlagung", "einzel"),
        ("gewinn_betriebsart", betriebsart),
        ("einkuenfte_gewinn", GEWINN_CENT),
        ("gewinn_bezeichnung", bezeichnung),
        ("kein_vuv", True), ("kein_sonstige", True), ("kein_kap", True),
        ("kein_p23_verkauf", True), ("kein_gewinn", False),
    ):
        _req(base, "POST", f"/fall/{fall_id}/event", _bestaetigt(fld, w), erwarte=201)
    st, dekl = _req(base, "GET", f"/fall/{fall_id}/deklaration", erwarte=200)
    return dekl


def test_gewerbe_kontrolle_bleibt_vollstaendig(base):
    """Kontrolle: sieben harmlose nicht_deklariert-Formen (Flags/Weichen) duerfen NICHT sperren."""
    dekl = _erklaere_gewinn(base, "kontrolle_gewerbe", "gewerbe", "Testbetrieb")
    assert dekl["vollstaendig"] is True, dekl["unvollstaendig"]
    assert dekl["eingaben_konsistent"] is True, dekl["unvollstaendig"]
    assert dekl["unvollstaendig"] == []
    assert len(dekl["deklaration"]) == 3
    assert len(dekl["nicht_deklariert"]) == 7


def test_land_forst_sperrt_vollstaendig_mit_nutzerfreundlichem_grund(base):
    """Fall: land_forst hat keinen Kz-Zweig -> vollstaendig/eingaben_konsistent kippen auf False,
    mit einem Grund, der dem Nutzer keinen Fehler in SEINEN Eingaben unterstellt. Faellt planmaessig
    weg, sobald tests/test_luf_gewinn_kz_fehlt.py::test_land_forst_gewinn_fehlt_in_deklaration von
    xfail auf XPASS kippt (s. Moduldocstring)."""
    dekl = _erklaere_gewinn(base, "fall_land_forst", "land_forst", "Testhof")
    assert dekl["vollstaendig"] is False
    assert dekl["eingaben_konsistent"] is False
    gruende = {e["feld_id"]: e["grund"] for e in dekl["unvollstaendig"]}
    assert set(gruende) == {"einkuenfte_gewinn", "gewinn_bezeichnung"}
    for grund in gruende.values():
        assert "noch nicht abgebbar" in grund
        assert "keine Eingabe von dir fehlt oder widerspricht sich" in grund
        # darf dem Nutzer keinen Fehler in seinen eigenen Eingaben unterstellen
        assert "ungültig" not in grund.lower()
