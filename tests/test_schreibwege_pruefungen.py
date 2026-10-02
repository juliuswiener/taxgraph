"""Jeder Python-Schreibweg gegen die drei Wert-Fälle der Speicherprüfung: Zahl ausserhalb von `bereich`,
Minus an einem `nicht_negativ`-Feld, falsche Form von `ts`/`herkunft`/`signal`.

Folge 1, Punkt 1 (haertung8, 2026-10-02) zu den Entscheidungen zahl-ausserhalb-des-bereichs-wird-beim-speichern-
abgewiesen-die-null-nicht, geldfeld-ohne-minus-im-schema-lehnt-minus-bei-eingabe-ab und tuer-und-speicher-weisen-
ab-was-die-fallakte-nicht-exakt-halten-kann. Die Prüfung sitzt in `append_event`; diese Datei misst, ob jeder
Weg dorthin sie auch erreicht und mit einer Antwort statt mit einem 500 endet.

    Weg                | ausserhalb bereich   | Minus an nicht_negativ | falsche Form
    POST /event        | 422                  | 422                    | 422
    POST /chat         | abgelehnt (200)      | abgelehnt (200)        | nicht erreichbar (Weg baut sie selbst)
    POST /kontoauszug  | nicht erreichbar     | nicht erreichbar       | nicht erreichbar
    POST /vorjahr      | übersprungen (200)   | übersprungen (200)     | nicht erreichbar
    POST /entfernung   | nicht erreichbar     | 422                    | nicht erreichbar
    eDaten, Beleg      | ValueError           | ValueError             | ValueError   (Funktionen, ohne Aufrufer)

"Nicht erreichbar" ist hier jeweils eine Eigenschaft des Wegs, kein Zufall der Testdaten; die Tests unten
halten sie fest (kein Kontoauszug-Ziel trägt `bereich`; der Wert ist `abs(betrag)`; ts/herkunft/signal baut der
Weg selbst, und jedes gespeicherte Event besteht die Formprüfung). Nicht Teil dieser Datei, weil sie die Akte
heute erreichen: die Ableitung (`_rechne_ab`, Ticket speichern-prueft-den-bereich-nicht-fuer-ableitungen,
Entscheidung "ausdrücklich nicht") und `uebernehme_edaten` ohne `bindung` (Gate
tests/test_append_event_bindung_gate.py).

NULL LLM (der Dialog und der Karten-Dienst sind ersetzt)."""
from __future__ import annotations

import copy
import json
import os
import sys
import urllib.error
import urllib.request

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
for _sub in ("produkt/haut", "produkt/store", "produkt/traverser", "produkt/mapping", "produkt/eingang", "golden"):
    sys.path.insert(0, os.path.join(ROOT, _sub))

import api as API  # noqa: E402
import api_llm  # noqa: E402
import beleg_writer as BW  # noqa: E402
import elster_writer as EW  # noqa: E402
import kontoauszug_writer as KW  # noqa: E402
import store as ST  # noqa: E402
import traverser as TR  # noqa: E402
from test_paket_b_e2e_http import base  # noqa: E402,F401 — Fixture

BINDUNG = TR.lade_bindung()
BEREICH = ("fam_anzahl_kinder", 99)          # bereich 0..20
MINUS = ("hh_handwerker_betrag", -5000)      # nicht_negativ, ohne bereich
GUT = {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}


def _roh(base, method, path, body=None):
    """Status und Antwort, ohne einen 4xx/5xx zu verschlucken oder zu verbieten (anders als `_req`)."""
    data = json.dumps(body).encode("utf-8") if body is not None else None
    req = urllib.request.Request(base + path, data=data, method=method,
                                 headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=30) as r:
            return r.status, json.loads(r.read())
    except urllib.error.HTTPError as e:
        return e.code, json.loads(e.read())


def _neu(base, fid, vz=2025):
    status, _ = _roh(base, "POST", "/fall", {"scheibe": "gesamt", "veranlagungszeitraum": vz, "fall_id": fid})
    assert status == 201, status


def _akte(fid) -> list:
    return [(e["feld_id"], e["wert"]) for e in API.lade_fall(fid)["events"]]


def _form_ok(fid) -> None:
    """Jedes gespeicherte Event besteht die Formprüfung — auch die Events, die der Weg selbst gebaut hat."""
    for e in API.lade_fall(fid)["events"]:
        ST._pruefe_begleitfelder(e["ts"], e["herkunft"], e["signal"])


def _ev(feld, wert, **kw):
    b = {"feld_id": feld, "wert": wert, "zustand": "vorlaeufig", "herkunft": copy.deepcopy(GUT),
         "schreiber": "ui:laie", "signal": {"signal_1": None, "signal_2": None}}
    b.update(kw)
    return b


# ------------------------------------------------------------------ POST /event

@pytest.mark.parametrize("fall,body,meldung", [
    ("bereich", _ev(*BEREICH), "fail-closed (Bereich): fam_anzahl_kinder=99"),
    ("minus", _ev(*MINUS), "fail-closed (Vorzeichen): hh_handwerker_betrag=-5000"),
    ("ts_zahl", _ev("ep_arbeitstage", 1, ts=5), "fail-closed (Form): ts"),
    ("herkunft_zusatz", _ev("ep_arbeitstage", 1, herkunft={**GUT, "x": 1}), "fail-closed (Form): herkunft"),
    ("signal_zahl", _ev("ep_arbeitstage", 1, signal=5), "fail-closed (Form): signal"),
])
def test_event_weist_ab_mit_422_und_ohne_event(base, fall, body, meldung):
    fid = f"ev-{fall}".replace("_", "-")
    _neu(base, fid)
    status, antwort = _roh(base, "POST", f"/fall/{fid}/event", body)
    assert status == 422, (status, antwort)
    assert meldung in antwort["fehler"]
    assert _akte(fid) == []


def test_event_kontrolle_derselbe_aufruf_ohne_fehler_wird_gespeichert(base):
    _neu(base, "ev-gut")
    status, _ = _roh(base, "POST", "/fall/ev-gut/event", _ev("ep_arbeitstage", 1))
    assert status == 201
    assert _akte("ev-gut") == [("ep_arbeitstage", 1)]
    _form_ok("ev-gut")


# ------------------------------------------------------------------ POST /chat

def _stub_dialog(monkeypatch, feld, wert):
    monkeypatch.setattr(api_llm, "_llm_dialog", lambda freitext, katalog, kontext="", user_id=None: {
        "vorschlaege": [{"feld_id": feld, "wert": wert, "beleg": "x", "begruendung": "y"}], "antwort": "",
        "unsicher": False, "aussagen": [], "rueckfragen": []})


@pytest.mark.parametrize("fall,vorschlag", [("bereich", BEREICH), ("minus", MINUS)])
def test_chat_lehnt_den_vorschlag_ab_und_antwortet_200(base, monkeypatch, fall, vorschlag):
    feld, wert = vorschlag
    _stub_dialog(monkeypatch, feld, wert)
    fid = f"chat-{fall}"
    _neu(base, fid)
    status, antwort = _roh(base, "POST", f"/fall/{fid}/chat", {"text": "irgendwas"})
    assert status == 200, (status, antwort)
    assert antwort["abgelehnt"] == [feld] and antwort["vorschlaege"] == []
    assert _akte(fid) == []


def test_chat_kontrolle_ein_guter_vorschlag_wird_gespeichert_und_hat_die_form(base, monkeypatch):
    _stub_dialog(monkeypatch, "fam_anzahl_kinder", 2)
    _neu(base, "chat-gut")
    status, antwort = _roh(base, "POST", "/fall/chat-gut/chat", {"text": "zwei kinder"})
    assert status == 200 and antwort["abgelehnt"] == []
    assert _akte("chat-gut") == [("fam_anzahl_kinder", 2)]
    _form_ok("chat-gut")


# ------------------------------------------------------------------ POST /kontoauszug

def test_kontoauszug_ziele_tragen_keinen_bereich_und_der_wert_ist_nie_negativ():
    """Beide Fälle sind hier nicht erreichbar. Bekäme ein Ziel einen `bereich`, endete ein Betrag darüber als 500
    (der Aufruf in api.kontoauszug fängt keine Abweisung des Stores) — dieser Test wird dann zuerst rot."""
    assert [f for f in KW.KATEGORIE_FELD.values() if BINDUNG[f].get("bereich")] == []
    for betrag, erwartet in [(-50000, [50000]), (50000, []), (0, [])]:   # Einnahmen und 0 schreiben nichts
        s = ST.leerer_store(2025, fall_id="ka")
        KW.uebernehme_kontoauszug(s, [{"betrag": betrag, "verwendungszweck": "Rechnung Maler"}], BINDUNG)
        assert [e["wert"] for e in s["events"]] == erwartet, betrag


def test_kontoauszug_kontrolle_eine_ausgabe_wird_gespeichert_und_hat_die_form(base):
    _neu(base, "ka-gut")
    tx = [{"betrag": -50000, "verwendungszweck": "Rechnung Maler Mustermann", "datum": "2025-03-01"}]
    status, antwort = _roh(base, "POST", "/fall/ka-gut/kontoauszug", {"format": "json", "inhalt": tx})
    assert status == 200 and antwort["uebernommen"] == 1, (status, antwort)
    assert _akte("ka-gut") == [("hh_handwerker_betrag", 50000)]
    _form_ok("ka-gut")


# ------------------------------------------------------------------ POST /vorjahr

def test_vorjahr_ueberspringt_bereich_und_minus_und_uebernimmt_den_rest(base):
    _neu(base, "vj", vz=2024)
    alt = API.lade_fall("vj")
    for feld, wert in [("fam_anzahl_kinder", 99), ("hh_handwerker_betrag", -5000), ("geburtsjahr", 1980)]:
        ST.append_event(alt, feld_id=feld, wert=wert, zustand="bestaetigt", herkunft=copy.deepcopy(GUT),
                        schreiber="ui:laie", signal={"signal_1": None, "signal_2": "ok"})   # Altbestand: ohne bindung
    API.speichere_fall("vj", alt)
    _neu(base, "vj-neu")
    status, antwort = _roh(base, "POST", "/fall/vj-neu/vorjahr", {"vorjahr_fall_id": "vj"})
    assert status == 200, (status, antwort)
    assert antwort["uebersprungen"] == ["fam_anzahl_kinder", "hh_handwerker_betrag"] and antwort["uebernommen"] == 1
    assert _akte("vj-neu") == [("geburtsjahr", 1980)]
    _form_ok("vj-neu")


# ------------------------------------------------------------------ POST /entfernung

@pytest.mark.parametrize("km,status,in_akte", [(-5, 422, []), (30, 200, [("ep_entfernung_km", 30)])])
def test_entfernung_weist_ein_minus_ab_und_speichert_einen_guten_wert(base, monkeypatch, km, status, in_akte):
    import ors_client
    monkeypatch.setattr(ors_client, "entfernung_km", lambda von, nach: km)
    fid = f"ent{km}".replace("-", "m")
    _neu(base, fid)
    st, antwort = _roh(base, "POST", f"/fall/{fid}/entfernung", {"von": "Musterstr 1", "nach": "Beispielweg 2"})
    assert st == status, (st, antwort)
    assert _akte(fid) == in_akte
    _form_ok(fid)


# ------------------------------------------------------------------ eDaten und Beleg (Funktionen)

@pytest.mark.parametrize("fall,feld,wert,kw,vorsilbe", [
    ("bereich", *BEREICH, {}, "fail-closed (Bereich)"),
    ("minus", *MINUS, {}, "fail-closed (Vorzeichen)"),
    ("ts_zahl", "ep_arbeitstage", 1, {"ts": 5}, "fail-closed (Form)"),
])
def test_edaten_mit_bindung_weist_ab_und_schreibt_nichts(fall, feld, wert, kw, vorsilbe):
    s = ST.leerer_store(2025, fall_id="ed")
    with pytest.raises(ValueError) as e:
        EW.uebernehme_edaten(s, [{"feld_id": feld, "wert": wert, "kategorie": "x"}], bindung=BINDUNG, **kw)
    assert str(e.value).startswith(vorsilbe)
    assert s["events"] == []


@pytest.mark.parametrize("fall,feld,wert,kw,vorsilbe", [
    ("minus", "hh_dienstleistung_betrag", -5000, {}, "fail-closed (Vorzeichen)"),
    ("bereich_enum", "kind_grad_der_behinderung", 7, {}, "fail-closed (Typ)"),     # der einzige Bereichs-Treffer im Beleg-Katalog ist ein enum
    ("ts_zahl", "hh_dienstleistung_betrag", 5, {"ts": 5}, "fail-closed (Form)"),
])
def test_beleg_weist_ab_und_schreibt_nichts(fall, feld, wert, kw, vorsilbe):
    s = ST.leerer_store(2025, fall_id="bl")
    kandidat = {"feld_id": feld, "wert": wert, "confidence": 1.0, "roh_text": "r", "beleg_typ": "t", "anker": "a"}
    with pytest.raises(ValueError) as e:
        BW.schreibe_kandidaten(s, [kandidat], beleg_ref="b", bindung=BINDUNG, **kw)
    assert str(e.value).startswith(vorsilbe)
    assert s["events"] == []
