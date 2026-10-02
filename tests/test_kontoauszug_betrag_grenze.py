"""POST /kontoauszug: eine Buchung, deren Betrag das Programm nicht tragen kann, geht allein in `verworfen`.

Folge 2, Punkt 2 (haertung8, 2026-10-02) zur Entscheidung kontoauszug-betrag-cent-genau-oder-verworfen: Eine Zeile
mit unlesbarem Betrag zählt in `verworfen`, der `hinweis` nennt den Betrag als Grund, die übrigen Zeilen bleiben,
den Upload ganz abzulehnen ist ausdrücklich nicht gewollt. Das gilt hier auch für einen Betrag ab 10^10 Cent
(Auflage F2 des Stores) und für NaN/Infinity im JSON.

Gemessen vor der Änderung (2026-10-02, format json): Betrag -10^10 → 500 `ValueError: fail-closed
(F2/Magnitude)`; JSON-Text mit `NaN` → 500 `ValueError: cannot convert float NaN to integer`; `Infinity` → 500
`OverflowError`. Gespeichert war nichts (speichere_fall läuft erst nach der Übernahme), aber der Nutzer verlor
den ganzen Auszug samt der lesbaren Zeilen.

NULL LLM: der Klassifikator ist ohne $LLM_API_KEY aus (die Zeilen unten sind deterministisch einzuordnen)."""
from __future__ import annotations

import base64
import json
import urllib.error
import urllib.request

import pytest

from test_schreibwege_pruefungen import _akte, _neu, _roh, base, KW  # noqa: F401 — Fixture und Helfer

RUERUP = "Ruerup-Rente Jahresbeitrag Basisrente"      # -> vor_rv_ausserhalb_lstb
MALER = "Rechnung Maler Mustermann"                    # -> hh_handwerker_betrag
GRENZE = 10 ** 10                                      # Cent; ab hier weist der Store einen Vorschlag ab (F2)


def _auszug(betrag_text: str) -> str:
    """JSON-Text: gute Buchung, die Buchung mit dem Betrag unter Test, gute Buchung gleicher Kategorie danach."""
    return ('[{"betrag": -120000, "verwendungszweck": "%s"},'
            ' {"betrag": %s, "verwendungszweck": "%s"},'
            ' {"betrag": -50000, "verwendungszweck": "%s"}]' % (RUERUP, betrag_text, MALER, MALER))


def _hochladen(base, fid, fmt, inhalt):
    _neu(base, fid)
    return _roh(base, "POST", f"/fall/{fid}/kontoauszug", {"format": fmt, "inhalt": inhalt})


def test_kontrolle_alle_betraege_im_bereich_werden_uebernommen(base):
    status, b = _hochladen(base, "bg-gut", "json", _auszug("-50000"))
    assert status == 200 and (b["uebernommen"], b["transaktionen"], b["verworfen"]) == (2, 3, 0), b
    assert "hinweis" not in b
    assert _akte("bg-gut") == [("vor_rv_ausserhalb_lstb", 120000), ("hh_handwerker_betrag", 50000)]


def test_kontrolle_der_betrag_knapp_unter_der_grenze_bleibt(base):
    status, b = _hochladen(base, "bg-knapp", "json", _auszug(str(-(GRENZE - 1))))
    assert status == 200 and (b["uebernommen"], b["verworfen"]) == (2, 0), b


# Der Rumpf-Leser (server.py) weist NaN, ±Infinity, 1e400 und eine Ganzzahl ausserhalb von i64 schon mit 400 ab. Als
# JSON-TEXT in `inhalt` kommen sie am Rumpf vorbei und erreichen die Übernahme: dort liegt der Fall.
NUR_ALS_TEXT = [("nan", "NaN"), ("unendlich", "Infinity"), ("minus_unendlich", "-Infinity"),
                ("float_ueberlauf", "-1e400"), ("ueber_i64", "-100000000000000000000")]
AUCH_ALS_LISTE = [("minus_grenze", str(-GRENZE)), ("plus_grenze", str(GRENZE)),   # Einnahme: schreibt nichts, ist
                  ("float_gross", "-1e30"), ("text", '"abc"'), ("null", "null"),  # aber ebenso nicht tragbar
                  ("liste", "[1]")]


@pytest.mark.parametrize("form,name,betrag", [("text", n, b) for n, b in NUR_ALS_TEXT + AUCH_ALS_LISTE]
                         + [("liste", n, b) for n, b in AUCH_ALS_LISTE])
def test_ein_nicht_tragbarer_betrag_verwirft_nur_diese_buchung(base, form, name, betrag):
    inhalt = _auszug(betrag) if form == "text" else json.loads(_auszug(betrag))
    fid = f"bg-{name}-{form}"
    status, b = _hochladen(base, fid, "json", inhalt)
    assert status == 200, (status, b)
    assert (b["uebernommen"], b["transaktionen"], b["verworfen"]) == (2, 2, 1), b
    assert "Betrag" in b["hinweis"], b
    assert _akte(fid) == [("vor_rv_ausserhalb_lstb", 120000), ("hh_handwerker_betrag", 50000)]


@pytest.mark.parametrize("betrag", ["NaN", "-Infinity", "-1e400", "-100000000000000000000"])
def test_im_rumpf_als_liste_weist_schon_die_tuer_ab(base, betrag):
    """Kontrolle zur Zeile oben: diese Werte erreichen die Übernahme nur als Text. Als Liste im Rumpf bleibt es
    beim 400 des Rumpf-Lesers (Folge 1) — kein 500, und kein Upload mit stiller Lücke."""
    _neu(base, "bg-tuer")
    roh = ('{"format": "json", "inhalt": ' + _auszug(betrag) + '}').encode()
    req = urllib.request.Request(base + "/fall/bg-tuer/kontoauszug", data=roh, method="POST",
                                 headers={"Content-Type": "application/json"})
    with pytest.raises(urllib.error.HTTPError) as e:
        urllib.request.urlopen(req, timeout=30)
    assert e.value.code == 400


def test_ein_element_ohne_objekt_verwirft_nur_diese_buchung(base):
    status, b = _hochladen(base, "bg-kein-objekt", "json", '[{"betrag": -120000, "verwendungszweck": "%s"}, "x", 5]' % RUERUP)
    assert status == 200 and (b["uebernommen"], b["verworfen"]) == (1, 2), (status, b)


def test_csv_betrag_ab_der_grenze_wird_verworfen(base):
    csv = ("datum;betrag;verwendungszweck\n"
           f"15.03.2025;-1200,00;{RUERUP}\n"
           f"16.03.2025;-{GRENZE // 100},00;{MALER}\n"          # genau 10^10 Cent
           f"17.03.2025;-{GRENZE // 100 - 1},99;{MALER}\n")     # 1 Cent darunter
    status, b = _hochladen(base, "bg-csv", "csv", csv)
    assert status == 200 and (b["uebernommen"], b["verworfen"]) == (2, 1), (status, b)
    assert "Betrag" in b["hinweis"], b


def test_pdf_betrag_ab_der_grenze_wird_verworfen(base, monkeypatch):
    monkeypatch.setattr(KW, "lies_kontoauszug_pdf", lambda pfad: ("text", {}))
    monkeypatch.setattr(KW, "parse_pdf_zeilen", lambda text, conf_map, schwelle=0.6: (
        [{"datum": "2025-03-15", "betrag": -120000, "verwendungszweck": RUERUP},
         {"datum": "2025-03-16", "betrag": -GRENZE, "verwendungszweck": MALER}], 0))
    status, b = _hochladen(base, "bg-pdf", "pdf", base64.b64encode(b"%PDF-1.4 fake").decode("ascii"))
    assert status == 200 and (b["uebernommen"], b["verworfen"]) == (1, 1), (status, b)
    assert "Betrag" in b["hinweis"], b


@pytest.mark.parametrize("inhalt,meldung", [
    ('[{"betrag": -1', "json-Inhalt nicht parsebar"),         # als Ganzes kein JSON: bleibt 400, wie vor der Änderung
    ('{"betrag": -1}', "json muss eine Liste von Transaktionen sein"),
])
def test_json_das_als_ganzes_nicht_taugt_bleibt_400(base, inhalt, meldung):
    status, b = _hochladen(base, "bg-400", "json", inhalt)
    assert status == 400 and meldung in b["fehler"], (status, b)
