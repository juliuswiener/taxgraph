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
    # Die Zahl als ASSERTION, nicht `grund != "bestaetigt"` als Sprungbedingung: die frühere Toleranz
    # für rechnung_unbar_offen liess genau die Inversion des Nachbar-Gates grün durch —
    # hh_rechnung_unbar ist oben bestätigt, dieser Sperrgrund darf hier nie feuern.
    if catala:
        assert erg["zahl_cent"] is not None, f"Sperrgrund statt Zahl: {erg['grund']}"
        assert erg["grund"] == "bestaetigt", f"Unexpected grund={erg['grund']}"


# ---- § 35a Abs. 4 S. 1: der Haushalt muss in der EU oder im EWR liegen --------------------
#
# Das ZWEITE Voraussetzungsfeld derselben Regel, und bis 2026-09-26 das einzige ohne Sperrgrund.
# `hh_in_eu_ewr` wird gefragt (askable, eigener Laientext, in beiden /fragen-Queues) — bleibt die
# Antwort aus, las der Ring `is True` als False und nullte damit ALLE DREI Toepfe, ohne Sperre und
# ohne Hinweis.
#
# Wovon die 1.000 EUR die Differenz sind (gemessen, nicht erschlossen — die Einheit allein macht
# den Satz mehrdeutig: die `kette` fuehrt EUR, `zahl_cent` fuehrt Cent). Fall 5.000 EUR Handwerker,
# vor dem Fix:
#   Δ zahl_cent            1067800 -> 967800  = 100000 ct = 1.000 EUR
#   Δ zu_versteuerndes_einkommen  49964 -> 49964 = 0        (die Ermäßigung beruehrt das zvE nicht)
#   Δ tarifliche_est              10678 -> 10678 = 0
#   Δ festzusetzende_est          10678 ->  9678 = 1.000 EUR
# Die 1.000 EUR sind also der § 35a-ABZUG SELBST (20 % von 5.000 EUR), und weil § 35a eine
# Steuerermaessigung ist, wirkt er 1:1 auf die festzusetzende Steuer — nicht die Haelfte davon.
# Kalibriert an einem zweiten Betrag (3.000 EUR Handwerker): Δ zahl_cent 60000 ct = 600 EUR,
# Δ festzusetzende_est 600 EUR, zvE und tarifliche_est wieder unveraendert. Das Verhaeltnis
# haelt bei 20 %, es ist also der Abzugsmechanismus und kein Einzelartefakt.
# Der Fall trug in bau-zweiges Kegel zusaetzlich 90 EUR KiSt-Differenz (dort ist KiSt konfiguriert);
# die Kegel hier fuehren keine KiSt (kist_cent=None), gemessen wird hier nur die ESt.
#
# Der Schaden ist die Anzeige, nicht die Zahl: "bestaetigt" ueber einer Rechnung, die den Abzug
# nicht enthaelt. Bei WERTEN ist fail-closed richtig — dasselbe Muster wie beim Geschwisterfeld
# handwerker_foerderung_offen daneben (Abs. 3 S. 2, gate-naht-guard-liest-zustand).
#
# Was hier NICHT gemessen wird: ob das Feld ueberhaupt gefragt wird. Das ist eine andere Frage
# (Traverser-Queue, tests/test_paket_b_e2e_http_foerderung.py::test_..._queue) und waere hier
# eine zweite Behauptung in einem Test, der eine messen soll.

_HHEU_HANDWERKER = [("hh_handwerker_betrag", 500000), ("hh_rechnung_unbar", True),
                    ("hh_handwerker_keine_foerderung", True)]


def _hheu_fall(base, fid, eu_ewr):
    """Legt einen gesamt-Fall mit Handwerkerkosten an. `eu_ewr` = True/False/None;
    None heisst: das Feld bleibt UNBEANTWORTET (der zu messende Zustand)."""
    _gesamt_anlegen(base, fid, _minimal_gesamt_kegel())
    for feld, wert in _HHEU_HANDWERKER:
        st, _ = _req(base, "POST", f"/fall/{fid}/event", _laie(feld, wert))
        assert st == 201, f"{feld}={wert} abgelehnt: {st}"
    if eu_ewr is not None:
        st, _ = _req(base, "POST", f"/fall/{fid}/event", _laie("hh_in_eu_ewr", eu_ewr))
        assert st == 201, f"hh_in_eu_ewr={eu_ewr} abgelehnt: {st}"
    st, erg = _req(base, "GET", f"/fall/{fid}/ergebnis")
    _val("ergebnis", erg)
    return erg


def test_p35a_eu_ewr_unbeantwortet_sperrt(base):
    """§ 35a Abs. 4 S. 1: unbeantwortetes hh_in_eu_ewr darf den Abzug nicht still entfallen lassen.

    Zwei Faelle, EINE Behauptung: A ist die Kontrolle (beantwortet True), B der Prueefall (offen).
    Ohne A zeigte ein roter Lauf nur, dass IRGENDETWAS fehlt — mit A steht daneben, dass der
    Abzug bei beantworteter Frage sehr wohl ankommt.
    """
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    basis = _basis_zahl_cent(base, "hheu-basis")
    mit = _hheu_fall(base, "hheu-beantwortet", True)
    offen = _hheu_fall(base, "hheu-offen", None)

    # (A) Kontrolle: beantwortet True -> der volle Abs.-3-Abzug. 20 % von 5.000 = 1.000 EUR.
    assert mit["grund"] == "bestaetigt", (
        f"Kontrolle gesperrt (grund={mit['grund']}) — dann misst dieser Test nichts")
    assert basis - mit["zahl_cent"] == 100000, (
        f"Kontrolle: {basis - mit['zahl_cent']} Cent statt 100000 (20 % von 5.000 EUR) — "
        f"Basis {basis}, beantwortet {mit['zahl_cent']}")

    # WORAUF die 100000 ct sitzen, nicht nur DASS sie da sind: § 35a ist eine Steuerermaessigung,
    # sie senkt die festzusetzende Steuer 1:1 und laesst das zvE unberuehrt. Die `kette` fuehrt EUR,
    # `zahl_cent` Cent — ohne diese drei Zeilen laesst sich "1.000 EUR" als Abzug oder als halbe
    # Steuerwirkung lesen. bau-zweige hat hier 90 EUR KiSt GEMESSEN (sein Kegel hat KiSt).
    assert basis == 1067800, f"Bezugszahl {basis} statt 1067800"
    k = mit["kette"]
    assert k["tarifliche_est"] == 10678, f"tarifliche_est {k['tarifliche_est']} statt 10678"
    assert k["festzusetzende_est"] == 9678, (
        f"festzusetzende_est {k['festzusetzende_est']} statt 9678 — die 1.000 EUR sind der "
        f"Abzug selbst, 1:1 in der festzusetzenden Steuer")
    assert k["zu_versteuerndes_einkommen"] == 49964, (
        f"zvE {k['zu_versteuerndes_einkommen']} statt 49964 — eine Steuerermaessigung darf das "
        f"zvE nicht bewegen")
    assert k["tarifliche_est"] - k["festzusetzende_est"] == 1000, (
        f"tarifliche_est {k['tarifliche_est']} - festzusetzende_est "
        f"{k['festzusetzende_est']} != 1000 EUR — der Abzug wirkt nicht 1:1 auf die Steuer")

    # (B) Prueefall: offen -> Sperre mit Klartext, KEINE stille Null.
    assert offen["grund"] == "haushalt_eu_ewr_offen", (
        f"Offenes hh_in_eu_ewr liefert grund={offen['grund']!r} (zahl_cent={offen['zahl_cent']}) "
        f"statt der Sperre — die Voraussetzung aus § 35a Abs. 4 S. 1 verschwindet still, "
        f"waehrend der Nutzer 'bestaetigt' ueber einer Rechnung ohne Abzug liest")
    assert offen["zahl_cent"] is None, (
        f"Gesperrter Fall darf keine Zahl tragen, hat aber {offen['zahl_cent']}")


def test_p35a_eu_ewr_false_nullt_den_abzug_und_sperrt_nicht(base):
    """§ 35a Abs. 4 S. 1, beantwortete Verneinung: Haushalt AUSSERHALB der EU/dem EWR.

    Gewaehlt: Abzug 0 und grund="bestaetigt" — NICHT gesperrt. Begruendung, zwei Gruende:
    (1) Das Gesetz sagt hier nichts Unbekanntes: Abs. 4 S. 1 SCHLIESST die Ermäßigung aus, sie
        ist damit rechenbar 0, nicht unentschieden. Ein bestaetigtes "nein" ist eine ANTWORT.
    (2) Der Nachbar derselben Regel macht es genauso: keine_foerderung=false (gefoerdert) nullt
        Abs. 3 und der Fall bleibt bestaetigt (test_p35a_foerderung_mit_foerderung oben, 480 EUR).
        Nur UNSET sperrt. Beides anders zu behandeln waere ein zweiter Rechenweg fuer dieselbe
        Bauart.
    Die Sperre greift also genau dort, wo die Antwort FEHLT — nicht dort, wo sie "nein" lautet.
    """
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    basis = _basis_zahl_cent(base, "hheu-false-basis")
    nein = _hheu_fall(base, "hheu-nein", False)

    assert nein["grund"] == "bestaetigt", (
        f"Beantwortetes NEIN darf nicht sperren (grund={nein['grund']}) — die Antwort ist da, "
        f"das Gesetz entscheidet: kein Abzug")
    assert basis == nein["zahl_cent"], (
        f"{basis - nein['zahl_cent']} Cent Ermäßigung trotz Haushalt außerhalb EU/EWR "
        f"(Basis {basis}, Fall {nein['zahl_cent']}) — § 35a Abs. 4 S. 1 schließt sie aus")


def test_p35a_eu_ewr_offen_sperrt_auch_ohne_handwerker(base):
    """§ 35a Abs. 4 S. 1 gatet ALLE DREI Toepfe (Abs. 1-3), nicht nur den Handwerker-Topf.

    Anders als das Geschwisterfeld daneben (Abs. 3 S. 2 gilt nur fuer Handwerker): ein
    Minijob-Fall ohne jede Handwerkerleistung haengt an derselben unbeantworteten Voraussetzung.
    Ohne diesen Fall waere eine Sperre, die nur `_hh_instanz_positiv("hh_handwerker", ...)`
    prueft, gruen — und liesse den Minijob still auf 0 fallen.
    """
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    _gesamt_anlegen(base, "hheu-minijob", _minimal_gesamt_kegel())
    _req(base, "POST", "/fall/hheu-minijob/event", _laie("hh_minijob_betrag", 40000))  # 400 EUR
    # hh_in_eu_ewr NICHT gesetzt, kein Handwerker, kein hh_rechnung_unbar
    st, erg = _req(base, "GET", "/fall/hheu-minijob/ergebnis")
    _val("ergebnis", erg)
    assert erg["grund"] == "haushalt_eu_ewr_offen", (
        f"Minijob ohne Handwerker: grund={erg['grund']!r} — Abs. 4 gatet auch Abs. 1")


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


def _rentner_erg(base, fid, extra):
    """rentner_gesamt bis /ergebnis, OHNE grund-Vorbedingung — der Sperrgrund ist hier
    gerade die zu messende Groesse. `_zahl` daneben asserted grund=="bestaetigt" und
    waere fuer den offenen Fall unbrauchbar."""
    st, _ = _req(base, "POST", "/fall", {"scheibe": "rentner_gesamt",
                                         "veranlagungszeitraum": VZ, "fall_id": fid})
    assert st == 201
    for feld, wert in RENTNER_KEGEL + extra:
        st, _ = _req(base, "POST", f"/fall/{fid}/event", _laie(feld, wert))
        assert st == 201, f"{feld}={wert} abgelehnt: {st}"
    st, erg = _req(base, "GET", f"/fall/{fid}/ergebnis")
    _val("ergebnis", erg)
    return erg


def test_p35a_eu_ewr_rentner_zweig_sperrt_und_rechnet(base):
    """§ 35a Abs. 4 S. 1 auf der zweiten Scheibe: Sperre, Abzug und Verneinung.

    Der Nachbarzweig laeuft durch einen ANDEREN Guard-Aufruf (bescheid_zweige.py statt
    bescheid.py), also wird hier ueber die ZAHL gemessen, nicht ueber den geteilten Pfad
    geglaubt. Alle vier Werte sind gemessen (2026-09-26, nach dem Fix, 6.000 EUR Handwerker
    -> Abs.-3-Deckel 20 % = 1.200 EUR):
      ohne_35a 4909500 · eu_ewr=True 4789500 (Δ 120000) · eu_ewr=False 4909500 · offen: None
    """
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    # hh_handwerker_keine_foerderung MUSS mit: ohne die Antwort feuert das GESCHWISTER-Gate
    # (handwerker_foerderung_offen) und der Fall misst den Nachbarn statt Abs. 4. Genau das
    # hat die Kontrolle unten beim ersten Lauf gezeigt (grund=handwerker_foerderung_offen).
    hw = [("hh_handwerker_betrag", 600000), ("hh_rechnung_unbar", True),
          ("hh_handwerker_keine_foerderung", True)]
    ohne = _rentner_erg(base, "p35a-hheu-rentner-ohne", [])
    mit = _rentner_erg(base, "p35a-hheu-rentner-ja", hw + [("hh_in_eu_ewr", True)])
    offen = _rentner_erg(base, "p35a-hheu-rentner-offen", hw)
    nein = _rentner_erg(base, "p35a-hheu-rentner-nein", hw + [("hh_in_eu_ewr", False)])

    # (A) Kontrolle: beantwortet True -> voller Abs.-3-Abzug, 1.200 EUR auf rentner_gesamt.
    assert ohne["grund"] == "bestaetigt", f"Bezugslauf gesperrt: {ohne['grund']}"
    assert ohne["zahl_cent"] == 4909500, f"Bezugszahl {ohne['zahl_cent']} statt 4909500"
    assert mit["grund"] == "bestaetigt", (
        f"Kontrolle gesperrt (grund={mit['grund']}) — dann misst dieser Test nichts")
    assert ohne["zahl_cent"] - mit["zahl_cent"] == 120000, (
        f"Kontrolle: {ohne['zahl_cent'] - mit['zahl_cent']} Cent statt 120000 (20 % von "
        f"6.000 EUR) — ohne {ohne['zahl_cent']}, mit {mit['zahl_cent']}")

    # (B) Prueefall: offen -> Sperre, keine Zahl. Dieselbe Bauart wie auf `gesamt`.
    assert offen["grund"] == "haushalt_eu_ewr_offen", (
        f"Offenes hh_in_eu_ewr auf rentner_gesamt: grund={offen['grund']!r} "
        f"(zahl_cent={offen['zahl_cent']}) statt der Sperre")
    assert offen["zahl_cent"] is None, (
        f"Gesperrter Fall darf keine Zahl tragen, hat aber {offen['zahl_cent']}")

    # (C) Verneinung: ehrliche Zahl ohne Abzug, KEINE Sperre — Polaritaet wie auf `gesamt`.
    assert nein["grund"] == "bestaetigt", (
        f"Beantwortetes NEIN darf nicht sperren (grund={nein['grund']})")
    assert ohne["zahl_cent"] - nein["zahl_cent"] == 0, (
        f"{ohne['zahl_cent'] - nein['zahl_cent']} Cent Ermäßigung trotz Haushalt außerhalb "
        f"EU/EWR (ohne {ohne['zahl_cent']}, nein {nein['zahl_cent']})")
