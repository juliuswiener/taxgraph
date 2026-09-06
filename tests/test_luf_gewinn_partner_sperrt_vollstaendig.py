"""Symmetrie-Nachzug zu tests/test_luf_gewinn_sperrt_vollstaendig.py (Instructor-Auftrag,
2026-08-31, auf Befund "Reichweite des zweiten Diffs"): der Klasse-f-Fix dort (VERZWEIGUNG,
est_mapping.py) betraf nur Person A. Der Klasse-g×f-Zweig (PARTNER_VERZWEIGUNG) hatte dieselbe
Kz-Lücke (`gewinn_betriebsart_partner="land_forst"`), kippte aber `vollstaendig`/
`eingaben_konsistent` nicht — der Betrag des PARTNERS verschwand still aus der abgegebenen
Erklärung. Entscheidung (Instructor, 2026-08-31): eine Sperre ist unangenehm, aber ehrlich;
die stille Auslassung eines Einkommens des Partners in einer abgegebenen Erklärung ist es nicht.

Kontrollzeile (gewerbe) MUSS `vollstaendig=True` bleiben, sonst misst der land_forst-Fall
unten nichts (Kontrolllinien-Disziplin, wie im Person-A-Pendant).

Fehlt eine Angabe, die eine Zahl in der Erklärung bewegen kann, wird gesperrt statt still
weggelassen — eine Sperre kostet den Nutzer einen Umweg, eine verschwiegene Einkunft kostet
ihn das Verfahren. Der Satz gilt nur für Angaben, deren Zahlwirkung GEMESSEN ist (hier belegt:
die 3.000.000 Cent aus einkuenfte_gewinn_partner verschwinden aus person_b, siehe
/tmp/probe_partner_workspace.py); für Felder ohne Zahlwirkung ist Sperren falsch.
(Instructor-Entscheidung, 2026-08-31, Fassung NACHTRAG 2)

NACHTRAG 3 (Korrektur der Gegenlese, 2026-08-31): der ursprüngliche Grund-Text nannte
einkuenfte_gewinn_partner/gewinn_bezeichnung_partner, als wären sie unbeantwortet. Live gemessen
(test_einkuenfte_gewinn_partner_ist_im_dialog_erreichbar_und_beantwortbar unten): beide Felder
SIND im Dialog erreichbar (feld_bedingung kein_gewinn_partner=false, unabhängig vom
Betriebsart-Wert), der Nutzer KANN sie beantworten und sie verlassen danach die /fragen-Queue.
Kein Wert dieser beiden Felder kann die Kz-Lücke schließen — das entscheidende Feld ist
gewinn_betriebsart_partner, dessen Wert zu ändern eine falsche Angabe wäre, keine Reparatur.
"Fehlt" ist deshalb die falsche Beschriftung; der Grund-Text muss vorne sagen, dass nichts fehlt.

KEIN xfail: beide Fälle sollen so aussehen, wie sie sind."""
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


def _erklaere_gewinn_partner(base, fall_id, betriebsart, bezeichnung):
    """Echter Nutzerpfad auf der Scheibe `gesamt`, Zusammenveranlagung mit Partner-Gewinn
    (dasselbe Muster wie _erklaere_gewinn in test_luf_gewinn_sperrt_vollstaendig.py, gespiegelt
    auf Person B)."""
    _req(base, "POST", "/fall",
         {"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": fall_id}, erwarte=201)
    for fld, w in (
        ("veranlagung", "zusammen"),
        ("gewinn_betriebsart_partner", betriebsart),
        ("einkuenfte_gewinn_partner", GEWINN_CENT),
        ("gewinn_bezeichnung_partner", bezeichnung),
        ("kein_vuv", True), ("kein_sonstige", True), ("kein_kap", True),
        ("kein_p23_verkauf", True), ("kein_gewinn", True),
        ("kein_kap_partner", True), ("kein_sonstige_partner", True), ("kein_gewinn_partner", False),
    ):
        _req(base, "POST", f"/fall/{fall_id}/event", _bestaetigt(fld, w), erwarte=201)
    st, dekl = _req(base, "GET", f"/fall/{fall_id}/deklaration", erwarte=200)
    return dekl


def test_gewerbe_kontrolle_partner_bleibt_vollstaendig(base):
    """Kontrolle: ein Partner mit gewerbe (Kz vorhanden) MUSS weiter durchkommen — sonst ist
    das keine Sperre mehr, sondern eine Blockade."""
    dekl = _erklaere_gewinn_partner(base, "kontrolle_gewerbe_partner", "gewerbe", "Partnerbetrieb")
    assert dekl["vollstaendig"] is True, dekl["unvollstaendig"]
    assert dekl["eingaben_konsistent"] is True, dekl["unvollstaendig"]
    assert dekl["unvollstaendig"] == []
    assert dekl["person_b"].get("E0800302") == 30000, dekl["person_b"]
    assert dekl["person_b"].get("E0800301") == "Partnerbetrieb", dekl["person_b"]


def test_land_forst_partner_sperrt_vollstaendig_mit_nutzerfreundlichem_grund(base):
    """Fall: der PARTNER hat land_forst-Gewinn ohne Kz-Zweig -> vollstaendig/eingaben_konsistent
    kippen auf False, mit einem Grund, der "des Partners" nennt (sonst weiß das Paar beim Lesen
    nicht, wessen Einkünfte gemeint sind — Auftrag 2026-08-31)."""
    dekl = _erklaere_gewinn_partner(base, "fall_land_forst_partner", "land_forst", "Testhof-Partner")
    assert dekl["vollstaendig"] is False
    assert dekl["eingaben_konsistent"] is False
    gruende = {e["feld_id"]: e["grund"] for e in dekl["unvollstaendig"]}
    assert set(gruende) == {"einkuenfte_gewinn_partner", "gewinn_bezeichnung_partner"}
    for grund in gruende.values():
        assert "noch nicht abgebbar" in grund
        assert "des Partners" in grund
        # darf dem Paar keinen Fehler in den eigenen Eingaben unterstellen
        assert "ungültig" not in grund.lower()
        # NACHTRAG 3 (Korrektur der Gegenlese, 2026-08-31): die Felder SIND beantwortet (Gegenpol
        # unten und /tmp/probe_partner_fragen_queue.py) — der Satz darf nicht klingen wie eine
        # fehlende Eingabe, und die entlastende Aussage muss VORNE stehen, nicht als Nachsatz.
        assert grund.startswith("Eure Angabe"), grund
        assert "vollständig" in grund
        assert "nichts nachtragen" in grund


def test_einkuenfte_gewinn_partner_ist_im_dialog_erreichbar_und_beantwortbar(base):
    """Beleg für NACHTRAG 3: ist die Angabe, die die Sperre einfordert, überhaupt irgendwo im
    Produkt eintragbar? JA — echter HTTP-Weg (/fall/.../fragen, /fall/.../event), keine
    Behauptung. Vor der Antwort steht das Feld in der Fragen-Queue (= 'noch offen'), nach der
    Antwort (HTTP 201) verlässt es sie (= 'beantwortet') — unabhängig vom gewählten
    gewinn_betriebsart_partner-Wert. Dass es TROTZDEM in `unvollstaendig` wieder auftaucht, ist
    also keine unbeantwortete Frage, sondern eine Kz-Lücke — der Grund-Text (Test oben) muss das
    auch so sagen."""
    fall_id = "sonde_partner_erreichbarkeit"
    _req(base, "POST", "/fall",
         {"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": fall_id}, erwarte=201)
    for fld, w in (
        ("veranlagung", "zusammen"),
        ("kein_vuv", True), ("kein_sonstige", True), ("kein_kap", True),
        ("kein_p23_verkauf", True), ("kein_gewinn", True),
        ("kein_kap_partner", True), ("kein_sonstige_partner", True),
    ):
        _req(base, "POST", f"/fall/{fall_id}/event", _bestaetigt(fld, w), erwarte=201)

    def _queue():
        st, body = _req(base, "GET", f"/fall/{fall_id}/fragen", erwarte=200)
        return {f["feld_id"] for f in body["fragen"]}

    _req(base, "POST", f"/fall/{fall_id}/event", _bestaetigt("kein_gewinn_partner", False), erwarte=201)
    vor = _queue()
    assert "einkuenfte_gewinn_partner" in vor, vor
    assert "gewinn_bezeichnung_partner" in vor, vor

    # land_forst waehlen -- die Felder bleiben in der Queue, sind also NICHT an den Betriebsart-Wert gekoppelt
    _req(base, "POST", f"/fall/{fall_id}/event",
         _bestaetigt("gewinn_betriebsart_partner", "land_forst"), erwarte=201)
    assert "einkuenfte_gewinn_partner" in _queue()
    assert "gewinn_bezeichnung_partner" in _queue()

    _req(base, "POST", f"/fall/{fall_id}/event",
         _bestaetigt("einkuenfte_gewinn_partner", 3000000), erwarte=201)
    _req(base, "POST", f"/fall/{fall_id}/event",
         _bestaetigt("gewinn_bezeichnung_partner", "Testhof-Partner"), erwarte=201)

    nach = _queue()
    assert "einkuenfte_gewinn_partner" not in nach, nach
    assert "gewinn_bezeichnung_partner" not in nach, nach
