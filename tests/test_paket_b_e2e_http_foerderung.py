"""Tests für § 35a Abs. 3 S. 2 Förderung-Gate (hh_handwerker_keine_foerderung).

Deterministisch, NULL LLM. Prüft:
  (a) Geförderte Maßnahmen (keine_foerderung=false) → Abs. 3 (Handwerker) = 0, Abs. 1/2 unberührt.
  (b) Nicht gefördert (keine_foerderung=true) → voller Abzug alle Töpfe.
  (c) Unbeantwortet (keine_foerderung=absent) + Handwerker > 0 → handwerker_foerderung_offen.
  (d) Mutationsprobe: Gate-Logik invertiert → false green wird rot.
"""
from __future__ import annotations

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
import runner as R       # noqa: F401

jsonschema = pytest.importorskip("jsonschema")
SCHEMA_DIR = os.path.join(ROOT, "produkt", "haut", "api_schema")


def _schema(name: str) -> dict:
    with open(os.path.join(SCHEMA_DIR, f"{name}.json"), encoding="utf-8") as f:
        return json.load(f)


def _val(name: str, obj: dict) -> None:
    jsonschema.Draft202012Validator(_schema(name)).validate(obj)


def _catala_da() -> bool:
    try:
        import runner  # noqa: F401
        return True
    except Exception:
        return False


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


VZ = 2025


# ---- Minimal-Kegel für § 35a Tests (gesamt, keine V+V/Gewinn) ----

def _minimal_gesamt_kegel():
    return [
        ("veranlagung", "einzel"),
        ("bruttoarbeitslohn", 0),
        ("vv_einnahmen", 0), ("vv_gebaeude_afa", 0), ("vv_schuldzinsen", 0),
        ("vv_erhaltungsaufwand", 0), ("vv_sonstige_wk", 0), ("vv_entgelt_quote_prozent", 100),
        ("ep_arbeitstage", 0), ("ep_entfernung_km", 0), ("ep_oepnv_kosten", 0), ("ep_eigenes_kfz", False),
        ("vor_an_anteil_rv", 0), ("vor_ag_anteil_rv", 0), ("vor_rv_ausserhalb_lstb", 0),
        ("basis_kv", 0), ("basis_pv", 0),
        ("versicherungsart", "gesetzlich_an"), ("vorsorge_arbeitslosenversicherung", 0), ("vorsorge_erwerbsunfaehigkeit", 0), ("vorsorge_unfall_haftpflicht", 0), ("vorsorge_rv_alt_mit_ueberschuss", 0), ("vorsorge_rv_alt_ohne_ueberschuss", 0), ("mit_anspruch_auf_zuschuss", False),
        ("kein_gewinn", False), ("kein_kap", True), ("kein_vuv", True), ("kein_sonstige", True),
        ("kap_kapitalertraege", 0), ("kap_gewinn_aktien", 0), ("kap_gewinn_sonstige", 0),
        ("kap_verlust_aktien", 0), ("kap_verlust_sonstige", 0),
        # Deutlich über dem Grundfreibetrag: bei 5.000 EUR ist die ESt schon vor jeder
        # § 35a-Ermäßigung 0, und jede Differenzmessung wäre ein Bodeneffekt.
        ("einkuenfte_gewinn", 5000000),  # 50.000 EUR Gewinn
        ("gewinn_betriebsart", "gewerbe"),
    ]


def _gesamt_anlegen(base, fid, kegel):
    st, _ = _req(base, "POST", "/fall", {"scheibe": "gesamt", "veranlagungszeitraum": VZ, "fall_id": fid})
    assert st == 201
    for feld, wert in kegel:
        st, _ = _req(base, "POST", f"/fall/{fid}/event", _laie(feld, wert))
        assert st == 201, f"POST event {feld}={wert} failed: {st}"


def _basis_zahl_cent(base, fid):
    """Steuer desselben Kegels OHNE jede § 35a-Angabe — die Bezugsgröße, gegen die die
    Ermäßigung gemessen wird. Ohne sie prüft ein Test nur, DASS gerechnet wurde, nicht WAS."""
    _gesamt_anlegen(base, fid, _minimal_gesamt_kegel())
    st, erg = _req(base, "GET", f"/fall/{fid}/ergebnis")
    assert erg["grund"] == "bestaetigt", f"Basisfall gesperrt: {erg.get('grund')}"
    assert erg["zahl_cent"] is not None, "Basisfall ohne Zahl"
    return erg["zahl_cent"]


def test_p35a_foerderung_mit_foerderung(base):
    """§ 35a Abs. 3 S. 2: Maßnahme mit öffentlicher Förderung (keine_foerderung=false).
    Erwartung: Abs. 3 (Handwerker) = 0, Abs. 1 + 2 unberührt.
    Testfall: Minijob 400€ + Dienstleistung 2000€ + Handwerker 3000€ (gefördert).
    Sollwert Abs. 1/2 nur: 400×20% + 2000×20% = 80 + 400 = 480 EUR.
    """
    catala = _catala_da()
    basis = _basis_zahl_cent(base, "p35a-foerd-ja-basis")
    kegel = _minimal_gesamt_kegel()
    _gesamt_anlegen(base, "p35a-foerd-ja", kegel)

    # Abs. 1/2/3 eingeben, aber hh_handwerker_keine_foerderung=false (ist gefördert)
    _req(base, "POST", "/fall/p35a-foerd-ja/event", _laie("hh_minijob_betrag", 40000))  # 400 EUR
    _req(base, "POST", "/fall/p35a-foerd-ja/event", _laie("hh_dienstleistung_betrag", 200000))  # 2000 EUR
    _req(base, "POST", "/fall/p35a-foerd-ja/event", _laie("hh_handwerker_betrag", 300000))  # 3000 EUR
    _req(base, "POST", "/fall/p35a-foerd-ja/event", _laie("hh_in_eu_ewr", True))
    _req(base, "POST", "/fall/p35a-foerd-ja/event", _laie("hh_rechnung_unbar", True))
    _req(base, "POST", "/fall/p35a-foerd-ja/event", _laie("hh_handwerker_keine_foerderung", False))  # GEFÖRDERT

    st, erg = _req(base, "GET", "/fall/p35a-foerd-ja/ergebnis")
    _val("ergebnis", erg)

    if catala:
        # `grund == "bestaetigt"` als ASSERTION, nicht als Sprungbedingung: sonst ist die
        # Bedingung identisch mit der Behauptung darunter und der Test kann nie rot werden.
        assert erg["grund"] == "bestaetigt", f"Expected confirmed, got grund={erg['grund']}"
        assert erg["offen"] == [], f"No open gates expected, got offen={erg['offen']}"
        # Die Euro-Zahl selbst, nicht nur „es wurde gerechnet": § 35a mindert die Steuer
        # 1:1, die Differenz zur Basis IST die Ermäßigung.
        # Nur Abs. 1 + 2: (400 + 2000) × 20% = 480 EUR. Der Handwerker (600 EUR) ist
        # gefördert und damit nach Abs. 3 S. 2 gesperrt.
        assert basis - erg["zahl_cent"] == 48000, (
            f"Ermaessigung {basis - erg['zahl_cent']} Cent statt 48000 "
            f"(Basis {basis}, mit Foerderung {erg['zahl_cent']}) — "
            f"Differenz 60000 = der geförderte Handwerker wird trotz Abs. 3 S. 2 abgezogen")


def test_p35a_foerderung_ohne_foerderung(base):
    """§ 35a Abs. 3 S. 2: Maßnahme OHNE öffentliche Förderung (keine_foerderung=true).
    Erwartung: voller Abzug Abs. 1 + 2 + 3.
    Testfall: wie oben, aber keine_foerderung=true.
    Sollwert: (400 + 2000 + 3000) × 20% = 80 + 400 + 600 = 1080 EUR.
    Differenzial zu mit_foerderung: die 600 EUR des Handwerkers.
    """
    catala = _catala_da()
    basis = _basis_zahl_cent(base, "p35a-foerd-nein-basis")
    kegel = _minimal_gesamt_kegel()
    _gesamt_anlegen(base, "p35a-foerd-nein", kegel)

    # Gleiche Aufwendungen, aber keine_foerderung=true (nicht gefördert)
    _req(base, "POST", "/fall/p35a-foerd-nein/event", _laie("hh_minijob_betrag", 40000))
    _req(base, "POST", "/fall/p35a-foerd-nein/event", _laie("hh_dienstleistung_betrag", 200000))
    _req(base, "POST", "/fall/p35a-foerd-nein/event", _laie("hh_handwerker_betrag", 300000))
    _req(base, "POST", "/fall/p35a-foerd-nein/event", _laie("hh_in_eu_ewr", True))
    _req(base, "POST", "/fall/p35a-foerd-nein/event", _laie("hh_rechnung_unbar", True))
    _req(base, "POST", "/fall/p35a-foerd-nein/event", _laie("hh_handwerker_keine_foerderung", True))  # NICHT GEFÖRDERT

    st, erg = _req(base, "GET", "/fall/p35a-foerd-nein/ergebnis")
    _val("ergebnis", erg)

    if catala:
        # `grund == "bestaetigt"` als ASSERTION, nicht als Sprungbedingung: sonst ist die
        # Bedingung identisch mit der Behauptung darunter und der Test kann nie rot werden.
        assert erg["grund"] == "bestaetigt"
        assert erg["offen"] == []
        # Der Kontrastfall zur Förderung: alle drei Töpfe, 80 + 400 + 600 = 1080 EUR.
        # Er hält den Fix ehrlich — eine Sperre, die IMMER nullt, wäre hier rot.
        assert basis - erg["zahl_cent"] == 108000, (
            f"Ermaessigung {basis - erg['zahl_cent']} Cent statt 108000 "
            f"(Basis {basis}, ohne Foerderung {erg['zahl_cent']})")


def test_p35a_foerderung_unbeantwortet_sperrung(base):
    """§ 35a Abs. 3 S. 2: Maßnahme Förderung-Status UNBEANTWORTET (keine_foerderung absent).
    Erwartung: handwerker_foerderung_offen Sperrung (fail-closed).
    Testfall: Handwerker > 0, aber keine_foerderung nicht gesetzt.
    """
    catala = _catala_da()
    kegel = _minimal_gesamt_kegel()
    _gesamt_anlegen(base, "p35a-foerd-offen", kegel)

    # Handwerker > 0, aber KEINE Antwort auf hh_handwerker_keine_foerderung
    _req(base, "POST", "/fall/p35a-foerd-offen/event", _laie("hh_handwerker_betrag", 300000))
    _req(base, "POST", "/fall/p35a-foerd-offen/event", _laie("hh_in_eu_ewr", True))
    _req(base, "POST", "/fall/p35a-foerd-offen/event", _laie("hh_rechnung_unbar", True))
    # hh_handwerker_keine_foerderung NICHT GESETZT

    st, erg = _req(base, "GET", "/fall/p35a-foerd-offen/ergebnis")
    _val("ergebnis", erg)

    # Sperrung erwartet: grund = sperrgrund, offen leer
    assert erg["grund"] == "handwerker_foerderung_offen", (
        f"Expected sperrgrund handwerker_foerderung_offen, got grund={erg['grund']}"
    )
    assert erg.get("offen", []) == [], (
        f"Expected offen=[], got {erg.get('offen')}"
    )


def test_p35a_foerderung_minijob_bleibt_aktiv(base):
    """§ 35a Abs. 3 S. 2: Minijob (Abs. 1) bleibt AKTIV, auch wenn Handwerker gefördert.
    Testfall: Minijob 400€ + Handwerker 3000€ (gefördert).
    Sollwert: nur Minijob 400 × 20% = 80 EUR, Handwerker = 0.
    """
    catala = _catala_da()
    basis = _basis_zahl_cent(base, "p35a-foerd-minijob-basis")
    kegel = _minimal_gesamt_kegel()
    _gesamt_anlegen(base, "p35a-foerd-minijob", kegel)

    _req(base, "POST", "/fall/p35a-foerd-minijob/event", _laie("hh_minijob_betrag", 40000))  # 400 EUR
    _req(base, "POST", "/fall/p35a-foerd-minijob/event", _laie("hh_handwerker_betrag", 300000))  # 3000 EUR
    _req(base, "POST", "/fall/p35a-foerd-minijob/event", _laie("hh_in_eu_ewr", True))
    # Abs. 5 S. 3 gilt nur für Abs. 2/3 — für den Minijob wäre die Rechnung entbehrlich.
    # Der Handwerkerbetrag steht hier aber im Fall, also verlangt der Riegel sie trotzdem;
    # ohne diese Antwort misst der Test nur `rechnung_unbar_offen` statt die Förderungs-Sperre.
    _req(base, "POST", "/fall/p35a-foerd-minijob/event", _laie("hh_rechnung_unbar", True))
    _req(base, "POST", "/fall/p35a-foerd-minijob/event", _laie("hh_handwerker_keine_foerderung", False))  # GEFÖRDERT

    st, erg = _req(base, "GET", "/fall/p35a-foerd-minijob/ergebnis")
    _val("ergebnis", erg)

    # Minijob sollte trotz Förderung-Sperre der Handwerker aktiv sein
    if catala:
        # `grund == "bestaetigt"` als ASSERTION, nicht als Sprungbedingung: sonst ist die
        # Bedingung identisch mit der Behauptung darunter und der Test kann nie rot werden.
        assert erg["grund"] == "bestaetigt", f"Minijob sollte unabhängig rechnen, got {erg['grund']}"
        assert erg["offen"] == []
        # Nur der Minijob: 400 × 20% = 80 EUR. Der geförderte Handwerker (600 EUR) fällt weg.
        assert basis - erg["zahl_cent"] == 8000, (
            f"Ermaessigung {basis - erg['zahl_cent']} Cent statt 8000 "
            f"(Basis {basis}, Fall {erg['zahl_cent']}) — 68000 hiesse: der geförderte "
            f"Handwerker wird mitgerechnet")


def test_p35a_foerderung_mutation_gate_inversion(base):
    """Mutationsprobe: Gate-Logik invertieren → false green wird rot.
    Szenario: none_foerderung=true, Handwerker > 0, hh_rechnung_unbar=true.
    Mit korrektem Gate: bestaetigt (keine Handwerker-Sperre).
    Mit invertiertem Gate (Bug): würde sperrung-offen triggern → FALSE GREEN erkannt.
    """
    catala = _catala_da()
    kegel = _minimal_gesamt_kegel()
    _gesamt_anlegen(base, "p35a-mut", kegel)

    _req(base, "POST", "/fall/p35a-mut/event", _laie("hh_handwerker_betrag", 300000))
    _req(base, "POST", "/fall/p35a-mut/event", _laie("hh_in_eu_ewr", True))
    _req(base, "POST", "/fall/p35a-mut/event", _laie("hh_rechnung_unbar", True))
    _req(base, "POST", "/fall/p35a-mut/event", _laie("hh_handwerker_keine_foerderung", True))  # nicht gefördert

    st, erg = _req(base, "GET", "/fall/p35a-mut/ergebnis")
    _val("ergebnis", erg)

    # Mit korrektem Gate: handwerker_foerderung_offen darf NICHT feuern
    assert "handwerker_foerderung_offen" not in erg.get("offen", []), (
        f"Gate-Bug: handwerker_foerderung_offen sollte NICHT feuern bei keine_foerderung=true, "
        f"got offen={erg.get('offen')}"
    )
    # Aber grund sollte bestaetigt sein (oder ein anderer echte Sperrgrund, nicht Förderung)
    if erg["grund"] != "bestaetigt":
        # Okay wenn ein anderer Gate-Grund feuert (z.B. rechnung_unbar für Minijob wenn absent)
        # aber nicht handwerker_foerderung_offen
        assert erg["grund"] in ("rechnung_unbar_offen",), f"Unexpected grund={erg['grund']}"


# rentner_gesamt: derselbe Guard (_shared_steuer_sonder_agb, bescheid_abzuege.py:178), aus
# beiden Zweigen identisch aufgerufen (bescheid_zweige.py:696 / :1168) — hier über die Zahl
# belegt statt über den geteilten Aufrufpfad geglaubt (Geltungsbereich ≠ Verwendung).
# Kegel + Helfer NICHT neu gebaut, sondern von der bereits gruenen rentner_gesamt-Fixtur
# aus test_p33b_abs5_s4_ring.py wiederverwendet.
from test_p33b_abs5_s4_ring import RENTNER_KEGEL, _zahl  # noqa: E402

# 6.000 EUR Handwerker-Arbeitskosten -> voller Abs.-3-Deckel 20% = 1.200 EUR.
_RENTNER_HANDWERKER = [
    ("hh_handwerker_betrag", 600000), ("hh_in_eu_ewr", True), ("hh_rechnung_unbar", True),
]


def test_p35a_foerderung_rentner_zweig_sperrt_ebenfalls(base):
    """§ 35a Abs. 3 S. 2 auf rentner_gesamt: Sollwert VORHER festgelegt (AUFTRAG 45,
    Frage 2, ad-hoc gemessen): Δ = 120000 Cent (1.200 EUR), dieselbe Größe wie auf gesamt.
    """
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    ohne = _zahl(base, "rentner_gesamt", "p35a-rentner-foerd-nein",
                 RENTNER_KEGEL + _RENTNER_HANDWERKER + [("hh_handwerker_keine_foerderung", True)])
    mit = _zahl(base, "rentner_gesamt", "p35a-rentner-foerd-ja",
                RENTNER_KEGEL + _RENTNER_HANDWERKER + [("hh_handwerker_keine_foerderung", False)])
    delta = mit - ohne
    assert delta == 120000, (
        f"ohne_foerderung={ohne} mit_foerderung={mit} Δ={delta} ≠ 120000 (1.200 EUR) — "
        f"die § 35a-Abs.3-S.2-Sperre wirkt auf rentner_gesamt nicht wie auf gesamt")
