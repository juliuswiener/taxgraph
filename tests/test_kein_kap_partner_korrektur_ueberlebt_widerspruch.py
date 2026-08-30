"""HTTP-Messung (server.py, echter Prozess): Person A wird bei einem Flag-Wert-Widerspruch
(kein_kap=True + kap_kapitalertraege>0, beide bestaetigt) korrekt gesperrt --
bescheid_deklaration.py:822 (`_an_gesamt_sperrgrund`, Guard der gesamt-Scheibe) ruft
`flag_check.flag_widersprueche()` auf und liefert `grund=flag_konsistenz_offen`, keinen Bescheid.
`FLAG_NEGIERT` (flag_check.py) kennt diesen Widerspruch nur fuer Person A -- kein einziger
`_partner`-Eintrag (produkt/konsistenz/partner_check.py prueft eine andere Achse: Partnerfeld
gesetzt UND veranlagung != "zusammen", feuert bei "zusammen" nie).

Fuer den Partner entsteht derselbe Widerspruch auf dem voelligen Normalweg, keine Manipulation:
der Nutzer traegt kap_kapitalertraege_partner=5000 EUR normal ein (regulaerer Dialogweg), aendert
spaeter seine Meinung und korrigiert (ersetzt=<event_id>, Auflage B in store.py::append_event)
kein_kap_partner auf True -- "mein Partner hat doch keine Kapitalertraege". Gemessen (2026-08-30,
HEAD 8c47a7b): der Fall bleibt bestaetigt, 898800 Cent (statt eines Sperrgrunds) zaehlen weiter --
750 EUR Steuer auf zurueckgezogene Einkuenfte, ohne Warnung, ohne Sperre.

FLAG_NEGIERT ist eine reine Datentabelle (flag, feld) -> flag_widersprueche() iteriert generisch
darueber; ein Eintrag "kein_kap_partner": ["kap_kapitalertraege_partner", ...] traefe denselben
Code-Pfad wie Person A, OHNE bescheid_deklaration.py selbst zu aendern (reserviert, hier nicht
angefasst).

Drei Tests:
  test_person_a_flag_widerspruch_wird_gesperrt                     -- Kontrollzweig, bereits
      richtig, KEIN xfail. Der Praezedenzfall, den der Fix nachbauen soll.
  test_kap_partner_korrektur_widerspruch_bleibt_unentdeckt          -- der Defekt, xfail(strict=True).
  test_kap_partner_korrektur_richtung_offen_zu_gesetzt_bleibt_korrekt -- Gegenprobe, MUSS gruen
      bleiben (ein Fix fuer den Defekt darf diese schon richtige Richtung nicht kaputt machen).

NULL LLM.
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

import api as API              # noqa: E402
import audit                   # noqa: E402
import server as SRV           # noqa: E402

MAX_FRAGEN = 600

KAP_PARTNER_FAMILIE = ("kap_kapitalertraege_partner", "kap_gewinn_aktien_partner",
                        "kap_verlust_aktien_partner", "kap_gewinn_sonstige_partner",
                        "kap_verlust_sonstige_partner")

EXPLIZIT_A = {
    "veranlagung": "einzel",
    "bruttoarbeitslohn": 6000000,
    "kein_gewinn": True, "kein_vuv": True, "kein_sonstige": True,
    "kein_kap": False,
    "kap_kapitalertraege": 500000,
    "kap_gewinn_aktien": 0,
}

EXPLIZIT_AB = {
    "veranlagung": "zusammen",
    "bruttoarbeitslohn": 6000000,
    "kein_gewinn": True, "kein_kap": True, "kein_vuv": True, "kein_sonstige": True,
    "kein_kap_partner": False,
    "kap_kapitalertraege_partner": 500000,
    "kap_gewinn_aktien_partner": 0, "kap_verlust_aktien_partner": 0,
    "kap_gewinn_sonstige_partner": 0, "kap_verlust_sonstige_partner": 0,
}


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


def _req(base_url, method, path, body=None):
    data = json.dumps(body).encode("utf-8") if body is not None else None
    req = urllib.request.Request(base_url + path, data=data, method=method,
                                  headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=10) as r:
            return r.status, json.loads(r.read())
    except urllib.error.HTTPError as e:
        return e.code, json.loads(e.read())


def _laie(fld, wert, ersetzt=None):
    d = {"feld_id": fld, "wert": wert, "zustand": "bestaetigt",
         "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
         "schreiber": "ui:laie", "signal": {"signal_1": None, "signal_2": f"ok@{fld}"}}
    if ersetzt:
        d["ersetzt"] = ersetzt
    return d


# -- Antwort-Generator, identisch zu tests/test_kein_kap_partner_vorab_sperre_und_ausweg.py (s.
# dort fuer die Begruendung der festen Muster-Tabelle statt eines Regex-Sample-Generators).
_MUSTER_BEISPIELWERT = {
    r"^\d{2}\.\d{2}\.\d{4}$": "01.01.2000",
    r"^\d{2}\.\d{2}-\d{2}\.\d{2}$": "01.01-31.12",
}


def _wert_zu_muster(muster: str) -> str:
    wert = _MUSTER_BEISPIELWERT.get(muster)
    if wert is None:
        raise AssertionError(f"Kein Beispielwert fuer unbekanntes Muster {muster!r} hinterlegt.")
    return wert


def _antwort_fuer(frage: dict, explizit: dict):
    fid = frage["feld_id"]
    if fid in explizit:
        return explizit[fid]
    typ = frage.get("typ")
    if typ == "bool":
        return False
    if typ == "enum":
        werte = frage.get("enum_werte") or []
        return werte[0] if werte else None
    if typ == "text":
        muster = frage.get("muster")
        if muster:
            return _wert_zu_muster(muster)
        return "x"
    if typ == "datum":
        return "01.01.2000"
    return 0   # cent/int


def _durchklicken(base_url, fall_id, explizit):
    """Beantwortet den Dialog ab dem aktuellen Fall-Stand bis /fragen leer ist. Gibt die
    gestellten Feld-IDs und ihre event_ids zurueck (fuer spaetere ersetzt=-Korrekturen)."""
    gestellt = []
    event_ids = {}
    for _ in range(MAX_FRAGEN):
        st, b = _req(base_url, "GET", f"/fall/{fall_id}/fragen")
        assert st == 200, (st, b)
        fragen = b["fragen"]
        if not fragen:
            return gestellt, event_ids
        frage = fragen[0]
        fid = frage["feld_id"]
        wert = _antwort_fuer(frage, explizit)
        assert wert is not None, f"keine Antwort fuer {fid} (typ={frage.get('typ')!r})"
        st, r = _req(base_url, "POST", f"/fall/{fall_id}/event", _laie(fid, wert))
        assert st == 201, (fid, st, r)
        gestellt.append(fid)
        event_ids[fid] = r["event_id"]
    raise AssertionError(f"Dialog endet nach {MAX_FRAGEN} Fragen nicht.")


# ---------------------------------------------------------------- Kontrollzweig (bereits richtig)

def test_person_a_flag_widerspruch_wird_gesperrt(base):
    """Praezedenzfall: Person A traegt Kapitalertraege ein, korrigiert danach das Abwesenheits-
    Kreuz auf True -- der Guard sperrt sofort, statt den zurueckgezogenen Wert weiterzurechnen.
    Das ist der Zustand, den der Partner-Fix nachbauen soll."""
    fall_id = "person_a_kontrolle"
    st, b = _req(base, "POST", "/fall",
                 {"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": fall_id})
    assert st == 201, (st, b)

    gestellt, event_ids = _durchklicken(base, fall_id, EXPLIZIT_A)
    assert "kap_kapitalertraege" in gestellt, "Positivkontrolle: Feld nicht erreicht"

    st, erg1 = _req(base, "GET", f"/fall/{fall_id}/ergebnis")
    assert st == 200 and erg1.get("grund") == "bestaetigt" and erg1.get("zahl_cent"), erg1

    st, r = _req(base, "POST", f"/fall/{fall_id}/event",
                 _laie("kein_kap", True, ersetzt=event_ids["kein_kap"]))
    assert st == 201, ("Korrektur abgelehnt", st, r)

    st, erg2 = _req(base, "GET", f"/fall/{fall_id}/ergebnis")
    assert st == 200 and erg2.get("grund") == "flag_konsistenz_offen" and erg2.get("zahl_cent") is None, (
        f"erwartet grund=flag_konsistenz_offen nach dem Widerspruch, tatsaechlich "
        f"grund={erg2.get('grund')!r} zahl_cent={erg2.get('zahl_cent')} -- wenn dieser "
        f"Kontrollzweig nicht mehr sperrt, hat sich die Person-A-Pruefung selbst geaendert und "
        f"die beiden Tests unten vergleichen nicht mehr gegen den echten Praezedenzfall.")


# ---------------------------------------------------------------- der Defekt

@pytest.mark.xfail(
    strict=True,
    reason="flag_check.py:FLAG_NEGIERT kennt kein_kap<->kap_kapitalertraege nur fuer Person A, "
           "kein '_partner'-Eintrag. bescheid_deklaration.py:822 (_an_gesamt_sperrgrund) ruft "
           "flag_widersprueche() bereits fuer die gesamt-Scheibe auf -- ein Partner-Eintrag in "
           "FLAG_NEGIERT traefe denselben Code-Pfad wie Person A, ohne bescheid_deklaration.py "
           "selbst zu aendern. Marker faellt am Tag des Fixes (XPASS) und zwingt dazu, ihn zu "
           "entfernen.")
def test_kap_partner_korrektur_widerspruch_bleibt_unentdeckt(base):
    fall_id = "partner_defekt"
    st, b = _req(base, "POST", "/fall",
                 {"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": fall_id})
    assert st == 201, (st, b)

    gestellt, event_ids = _durchklicken(base, fall_id, EXPLIZIT_AB)
    assert "kap_kapitalertraege_partner" in gestellt, "Positivkontrolle: Feld nicht erreicht"

    st, erg1 = _req(base, "GET", f"/fall/{fall_id}/ergebnis")
    assert st == 200 and erg1.get("grund") == "bestaetigt", erg1
    assert erg1.get("zahl_cent") == 898800, (
        f"Ausgangswert hat sich veraendert ({erg1.get('zahl_cent')} statt 898800) -- die "
        f"Vergleichszahl unten misst dann nicht mehr denselben Fall.")

    st, r = _req(base, "POST", f"/fall/{fall_id}/event",
                 _laie("kein_kap_partner", True, ersetzt=event_ids["kein_kap_partner"]))
    assert st == 201, ("Korrektur abgelehnt", st, r)

    st, erg2 = _req(base, "GET", f"/fall/{fall_id}/ergebnis")
    assert st == 200, erg2
    assert erg2.get("grund") == "flag_konsistenz_offen" and erg2.get("zahl_cent") is None, (
        f"Der Widerspruch (kein_kap_partner=True + kap_kapitalertraege_partner=5000 EUR, beide "
        f"bestaetigt) bleibt unentdeckt: zahl_cent={erg2.get('zahl_cent')} Cent, "
        f"grund={erg2.get('grund')!r} -- die 750 EUR auf zurueckgezogene Partner-Kapitalertraege "
        f"zaehlen weiter, statt wie bei Person A zu sperren.")


# ---------------------------------------------------------------- Gegenprobe (muss gruen bleiben)

def test_kap_partner_korrektur_richtung_offen_zu_gesetzt_bleibt_korrekt(base):
    """Die andere Korrekturrichtung -- erst verneint (Sackgasse), dann korrigiert, dann
    eingetragen -- funktioniert schon richtig (s. test_kein_kap_partner_vorab_sperre_und_ausweg.py).
    Ein Fix fuer den Defekt oben darf DIESE Richtung nicht kaputt machen; dieser Test bleibt gruen,
    ein neuer roter Fehlschlag hier zeigt genau das an."""
    fall_id = "partner_kontrolle_umgekehrt"
    st, b = _req(base, "POST", "/fall",
                 {"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": fall_id})
    assert st == 201, (st, b)

    basis_ohne_familie = {k: v for k, v in EXPLIZIT_AB.items()
                           if k not in ("kein_kap_partner",) + KAP_PARTNER_FAMILIE}
    for fld, w in basis_ohne_familie.items():
        st, r = _req(base, "POST", f"/fall/{fall_id}/event", _laie(fld, w))
        assert st == 201, (fld, st, r)
    st, r = _req(base, "POST", f"/fall/{fall_id}/event", _laie("kein_kap_partner", True))
    assert st == 201, ("kein_kap_partner", st, r)
    kkp_event_id = r["event_id"]

    gestellt, _ = _durchklicken(base, fall_id, {})
    assert not any(f in gestellt for f in KAP_PARTNER_FAMILIE), (
        "Die Familie wurde schon gefragt, bevor die Sackgasse ueberhaupt entstehen konnte -- "
        "dann prueft dieser Test die falsche Ausgangslage.")

    st, erg1 = _req(base, "GET", f"/fall/{fall_id}/ergebnis")
    assert st == 200 and erg1.get("grund") == "partner_kegel_offen", erg1

    st, r = _req(base, "POST", f"/fall/{fall_id}/event",
                 _laie("kein_kap_partner", False, ersetzt=kkp_event_id))
    assert st == 201, ("Korrektur abgelehnt", st, r)

    st, stand = _req(base, "GET", f"/fall/{fall_id}/fragen")
    assert st == 200
    offen_ids = [f["feld_id"] for f in stand["fragen"]]
    assert "kap_kapitalertraege_partner" in offen_ids, (
        "Nach der Korrektur ist kap_kapitalertraege_partner nicht in der Warteschlange -- "
        "dann waere der gemessene Ausweg keiner.")

    st, r = _req(base, "POST", f"/fall/{fall_id}/event", _laie("kap_kapitalertraege_partner", 500000))
    assert st == 201, (st, r)
    for fld in ("kap_gewinn_aktien_partner", "kap_verlust_aktien_partner",
                "kap_gewinn_sonstige_partner", "kap_verlust_sonstige_partner"):
        st, r = _req(base, "POST", f"/fall/{fall_id}/event", _laie(fld, 0))
        assert st == 201, (fld, st, r)

    st, erg2 = _req(base, "GET", f"/fall/{fall_id}/ergebnis")
    assert st == 200 and erg2.get("grund") == "bestaetigt" and erg2.get("zahl_cent") == 898800, (
        f"erwartet grund=bestaetigt, zahl_cent=898800 (der eingetragene Wert zaehlt korrekt), "
        f"tatsaechlich grund={erg2.get('grund')!r} zahl_cent={erg2.get('zahl_cent')}")
