"""Land-/Forstwirtschaft-Gewinn verschwindet aus der Deklaration (Instructor-Auftrag, 2026-08-31):
`gewinn_betriebsart=land_forst` läuft durch, aber `einkuenfte_gewinn` steht in KEINER Zeile der
Deklaration. Root Cause: Anlage L hat im ESt-XSD KEINE einzelne Betrags-Kennzahl für den
laufenden Gewinn — welche von vier Kandidaten-Kz (E0900202/E0900301 fuer §4, E0900405/E0900502
fuer §13a, alle im Container `L/Gewinn/Einz_Unt`) zutrifft, haengt an zwei Feldern, die es im
Projekt nicht gibt: Gewinnermittlungsart (E0900407) und Wirtschaftsjahr-Lage (E0900101). Ohne die
ist jede feste Kz-Wahl geraten — deshalb fehlt der Eintrag in produkt/mapping/est_mapping.py
bewusst, nicht versehentlich (XSD-Befund, Recherche-Worker 2026-08-31).

Kontrollzeile (gewerbe) MUSS grün bleiben — sonst misst dieser Test nicht die Gewinnart, sondern
etwas anderes. Beide Fälle in einer Datei, damit die Kontrolle nicht ohne den Fall verschwindet.

xfail(strict=True, raises=AssertionError) statt "einfach rot": ein dauerhaft roter Test im Baum
zerstoert das Signal "rot heisst, etwas ist kaputtgegangen" — nach einer Woche schaut niemand mehr
hin. `raises=` ist Pflicht, nicht Zierde: ein xfail(strict) ohne `raises=` nagelt nur fest, DASS der
Test scheitert, nicht WORAN — das hat in diesem Projekt schon zweimal einen Defektwechsel maskiert.
Sobald jemand die zwei fehlenden Felder baut, kippt der Test auf XPASS und erzwingt Aufmerksamkeit.

Bezeichnungs-Assertion ("Testhof" muss auftauchen) wurde GESTRICHEN, nicht auskommentiert: sie
pinnte keinen Defekt, sondern eine Unmoeglichkeit. `Einz_Unt` (der zu `veranlagung=einzel` passende
Container fuer den Einzelbetrieb) hat im XSD KEIN Bezeichnungsfeld. Ein Bezeichnungs-Kz existiert in
Anlage L nur in `Ges_Fest` (E0901005/E0901101) und `MU` (E0900711/E0900611) — beides Container fuer
gesondert festgestellte Anteile bzw. Mitunternehmerschaften, die eigene Finanzamt-/Steuernummer-
Angaben verlangen. Dort "Testhof" einzutragen hiesse, eine Mitunternehmerschaft zu behaupten, die
im Testfall nicht existiert — dieselbe Fehlform (falscher Container: Ges_Fest/MU statt Einz_U/
Freiber_T), die fuer Anlage G/S bereits am 2026-08-20 verworfen wurde (Container-Korrektur in
produkt/mapping/est_mapping.py). Eine Bezeichnungs-Kz fuer den land_forst-Einzelbetrieb zu bauen
hiesse also, denselben Fehler ein zweites Mal zu machen.

Update (Folgeauftrag, gleicher Tag): `vollstaendig`/`eingaben_konsistent` sperren inzwischen korrekt
(tests/test_luf_gewinn_sperrt_vollstaendig.py) — die dortige Pruefung auf True wurde entfernt, sie
war die Zusicherung, die der Fix absichtlich umdreht. ACHTUNG Widerspruch, nicht zwei Haelften:
beide Dateien speisen dieselbe Eingabe (land_forst, 30.000 EUR) in denselben Endpunkt und verlangen
entgegengesetzte Endzustaende — dieser Test hier will den Betrag irgendwann UNTER einem Kz sehen,
der sperrt_vollstaendig-Test will `vollstaendig=False` erhalten bleiben. Baut jemand den Kz-Zweig,
faellt zwangslaeufig der land_forst-Block in tests/test_luf_gewinn_sperrt_vollstaendig.py — das ist
dann Folgerichtigkeit, keine Regression; jener Test muss im selben Schritt mit umgeschrieben werden."""
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

# Betrag: 3000000 Cent Eingabe (Feldtyp `cent`) -> 30000 EUR im Schreiber-Output (Kz E0800302 etc.).
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
    """Echter Nutzerpfad auf der Scheibe `gesamt` (die einzige Kachel mit Gewinnfeldern —
    `an_gesamt` hat keine Startseiten-Kachel, ist nicht nutzer-erreichbar)."""
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


def test_gewerbe_kontrolle_betrag_steht_im_formular(base):
    """Kontrolle: gewerbe MUSS im Formular stehen. Bricht dieser Test, misst der land_forst-Test
    unten nichts (Kontrolllinien-Disziplin)."""
    dekl = _erklaere_gewinn(base, "kontrolle_gewerbe", "gewerbe", "Testbetrieb")
    assert dekl["vollstaendig"] is True
    assert dekl["eingaben_konsistent"] is True
    werte = dekl["deklaration"]
    betrag_kz = [k for k, v in werte.items() if v == 30000]
    assert betrag_kz, f"kein Kz mit 30000 EUR in der Deklaration: {werte}"
    assert "Testbetrieb" in werte.values(), f"gewinn_bezeichnung fehlt in der Deklaration: {werte}"


@pytest.mark.xfail(
    strict=True, raises=AssertionError,
    reason="Anlage-L-Kz fuer laufenden Gewinn fehlt bewusst: die Kz-Wahl haengt an zwei Feldern, "
           "die es im Projekt nicht gibt (Gewinnermittlungsart E0900407, Wirtschaftsjahr-Lage "
           "E0900101) — siehe Moduldocstring.")
def test_land_forst_gewinn_fehlt_in_deklaration(base):
    """Fall: der Gewinn steht in KEINER Zeile der Deklaration. Bewusst xfail, keine Regression —
    der Kz-Eintrag fehlt in produkt/mapping/est_mapping.py fuer den Zweig `land_forst`, weil die
    Kz-Wahl ohne Gewinnermittlungsart/Wirtschaftsjahr nicht eindeutig ist (Anlage-L-Kz noch offen)."""
    dekl = _erklaere_gewinn(base, "fall_land_forst", "land_forst", "Testhof")
    werte = dekl["deklaration"]
    betrag_kz = [k for k, v in werte.items() if v == 30000]
    assert betrag_kz, f"kein Kz mit 30000 EUR in der Deklaration: {werte}"
