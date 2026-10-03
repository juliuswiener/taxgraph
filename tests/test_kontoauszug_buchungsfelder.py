"""POST /kontoauszug, format json: `datum` und `verwendungszweck` einer Buchung, die die Fallakte nicht halten kann.

Backlog falldatei-mit-nan-liest-rust-als-text, AK4 (k9-akte, 2026-10-03). Die Rumpf-Tür (server.py) weist NaN,
±Infinity, 1e400 und eine Ganzzahl ausserhalb von i64 an jeder Stelle des Rumpfs mit 400 ab. `inhalt` ist aber ein
JSON-TEXT; `api.kontoauszug` liest ihn mit einem eigenen `json.loads` ohne diese Haken.

Gemessen vor der Änderung (Temp-Wurzel, `inhalt` als Text, Buchung mit Kategorie und Zielfeld):
- `datum` = NaN, ±Infinity, ±1e400 (auch in Liste oder Objekt) -> 500 `ValueError` aus `speichere_fall`
  (`allow_nan=False`); die Akte blieb leer, keine *.tmp.
- `datum` = Ganzzahl ausserhalb von i64 (2^63, 2^64, -2^63-1, auch in einer Liste) -> 200, der Wert stand in der
  Akte, und der Rust-Lader sperrt eine solche Akte (`Sperrform`): der echte "Python schreibt, Rust sperrt"-Weg.
- `verwendungszweck` als Zahl, Liste, Objekt, true oder NaN auf einer Ausgabe -> 500 `AttributeError` ('lower').

Jetzt 422 mit "Kontoauszug nicht lesbar: ..."; der Text nennt Feld und Typ, nie den Wert. `betrag` bleibt bei
`verworfen` (Entscheidung kontoauszug-betrag-cent-genau-oder-verworfen): ein Betrag ist eine Zahl, die das Programm
nicht tragen kann; ein Datum ist es nicht.

NULL LLM (der Klassifikator ist ohne $LLM_API_KEY aus)."""
from __future__ import annotations

import os

import pytest

from test_schreibwege_pruefungen import API, _akte, _neu, _roh, base  # noqa: F401 — Fixture und Helfer

RUERUP = "Ruerup-Rente Jahresbeitrag Basisrente"      # -> vor_rv_ausserhalb_lstb
I64_MAX, I64_MIN = 2**63 - 1, -(2**63)


def _auszug(datum: str = '"2025-03-15"', zweck: str = f'"{RUERUP}"', betrag: str = "-120000") -> str:
    """JSON-Text mit EINER Buchung; Datum, Zweck und Betrag stehen wörtlich (auch als NaN)."""
    return '[{"datum": %s, "betrag": %s, "verwendungszweck": %s}]' % (datum, betrag, zweck)


def _hochladen(base, fid: str, inhalt: str):
    _neu(base, fid)
    return _roh(base, "POST", f"/fall/{fid}/kontoauszug", {"format": "json", "inhalt": inhalt})


def _akte_blieb_leer(fid: str) -> None:
    assert _akte(fid) == []
    assert [f for f in os.listdir(API.FAELLE) if f.endswith(".tmp")] == []


def datum_meldung(typ: str) -> str:
    return f"Kontoauszug nicht lesbar: datum einer Buchung enthält eine Zahl ({typ}), die die Akte nicht halten kann."


def zweck_meldung(typ: str) -> str:
    return f"Kontoauszug nicht lesbar: verwendungszweck einer Ausgabe muss Text sein, nicht {typ}."


@pytest.mark.parametrize("name,datum,typ", [
    ("nan", "NaN", "float"),
    ("unendlich", "Infinity", "float"),
    ("minus_unendlich", "-Infinity", "float"),
    ("ueberlauf", "1e400", "float"),
    ("minus_ueberlauf", "-1e400", "float"),
    ("in_liste", "[1, NaN]", "float"),
    ("in_objekt", '{"a": {"b": [Infinity]}}', "float"),
    ("zwei_hoch_63", str(I64_MAX + 1), "int"),
    ("zwei_hoch_64", str(2**64), "int"),
    ("minus_ueber_i64", str(I64_MIN - 1), "int"),
    ("int_in_liste", f"[{2**64}]", "int"),
    ("int_in_objekt", '{"a": %d}' % (I64_MIN - 1), "int"),
    ("beides_nennt_float", f"[{2**64}, NaN]", "float"),      # Reihenfolge im Text zählt nicht, float geht vor
])
def test_datum_mit_nicht_tragbarer_zahl_ist_422_und_die_akte_bleibt_leer(base, name, datum, typ):
    fid = f"bf-{name}"
    status, b = _hochladen(base, fid, _auszug(datum=datum))
    assert (status, b) == (422, {"fehler": datum_meldung(typ)}), (status, b)
    _akte_blieb_leer(fid)


@pytest.mark.parametrize("datum", [str(I64_MAX), str(I64_MIN), "1.5", "1e22", "-0", "null", '"NaN"', "[1, 2.5]", "true"])
def test_kontrolle_tragbares_datum_wird_wie_vorher_uebernommen(base, datum):
    fid = "bf-gut"
    status, b = _hochladen(base, fid, _auszug(datum=datum))
    assert status == 200 and (b["uebernommen"], b["verworfen"]) == (1, 0), (status, b)
    assert _akte(fid) == [("vor_rv_ausserhalb_lstb", 120000)]


@pytest.mark.parametrize("zahl", ["0123456789012345678901234", "-0123456789012345678901", "-", "--5", "+5"])
def test_eine_zahl_die_kein_json_ist_bleibt_400(base, zahl):
    """Kontrolle: nur gültiges JSON kann eine Zahl ausserhalb von i64 sein; führende Null, `-` und `+5` sind es nicht."""
    status, b = _hochladen(base, "bf-kein-json", _auszug(datum=zahl))
    assert (status, b) == (400, {"fehler": "json-Inhalt nicht parsebar"}), (status, b)


def test_die_meldung_traegt_nie_den_wert(base):
    _, b = _hochladen(base, "bf-wert", _auszug(datum="123456789012345678901234567890"))
    assert "12345" not in b["fehler"] and "NaN" not in b["fehler"], b


def test_das_datum_jeder_buchung_zaehlt_auch_bei_einer_einnahme(base):
    """Das Datum geht nur bei einer gebuchten Ausgabe in die Akte; die Prüfung gilt trotzdem für jede Buchung,
    damit sie nicht an Kategorie und Zielfeld hängt."""
    status, b = _hochladen(base, "bf-einnahme", _auszug(datum="NaN", betrag="5000"))
    assert (status, b) == (422, {"fehler": datum_meldung("float")}), (status, b)
    _akte_blieb_leer("bf-einnahme")


def test_eine_verworfene_buchung_wird_nicht_mehr_geprueft(base):
    """`verworfen` kommt zuerst: eine Buchung mit unlesbarem Betrag fliegt allein raus, auch mit NaN im Datum."""
    inhalt = ('[{"datum": NaN, "betrag": NaN, "verwendungszweck": "%s"}, '
              '{"datum": "2025-03-15", "betrag": -120000, "verwendungszweck": "%s"}]' % (RUERUP, RUERUP))
    status, b = _hochladen(base, "bf-verworfen", inhalt)
    assert status == 200 and (b["uebernommen"], b["transaktionen"], b["verworfen"]) == (1, 1, 1), (status, b)
    assert _akte("bf-verworfen") == [("vor_rv_ausserhalb_lstb", 120000)]


def test_kontrolle_nan_im_betrag_bleibt_verworfen(base):
    """Kontrolle, nicht Ersatz für tests/test_kontoauszug_betrag_grenze.py: NaN im Betrag ist kein 422."""
    status, b = _hochladen(base, "bf-betrag", _auszug(betrag="NaN"))
    assert status == 200 and (b["uebernommen"], b["verworfen"]) == (0, 1), (status, b)


@pytest.mark.parametrize("name,zweck,typ", [
    ("zahl", "5", "int"),
    ("kommazahl", "1.5", "float"),
    ("liste", '["maler"]', "list"),
    ("objekt", '{"a": 1}', "dict"),
    ("wahr", "true", "bool"),
    ("nan", "NaN", "float"),
    ("unendlich", "-Infinity", "float"),
    ("grosse_ganzzahl", str(2**64), "int"),
])
def test_zweck_einer_ausgabe_der_kein_text_ist_ist_422(base, name, zweck, typ):
    fid = f"bz-{name}"
    status, b = _hochladen(base, fid, _auszug(zweck=zweck))
    assert (status, b) == (422, {"fehler": zweck_meldung(typ)}), (status, b)
    _akte_blieb_leer(fid)


@pytest.mark.parametrize("name,zweck", [("null", "null"), ("null_zahl", "0"), ("leere_liste", "[]"), ("falsch", "false"),
                                        ("leer", '""')])
def test_kontrolle_falscher_zweck_heisst_leer_und_bleibt_200(base, name, zweck):
    status, b = _hochladen(base, f"bz-leer-{name}", _auszug(zweck=zweck))
    assert status == 200 and (b["uebernommen"], b["transaktionen"], b["verworfen"]) == (0, 1, 0), (status, b)


def test_kontrolle_zweck_einer_einnahme_wird_nie_gelesen(base):
    status, b = _hochladen(base, "bz-einnahme", _auszug(zweck="5", betrag="5000"))
    assert status == 200 and (b["uebernommen"], b["transaktionen"]) == (0, 1), (status, b)


def test_datum_wird_vor_dem_zweck_gemeldet_und_beides_vor_dem_schreiben(base):
    inhalt = ('[{"datum": "ok", "betrag": -100, "verwendungszweck": 5}, '
              '{"datum": NaN, "betrag": -100, "verwendungszweck": "Maler"}]')
    status, b = _hochladen(base, "bf-reihenfolge", inhalt)
    assert (status, b) == (422, {"fehler": datum_meldung("float")}), (status, b)
    _akte_blieb_leer("bf-reihenfolge")
