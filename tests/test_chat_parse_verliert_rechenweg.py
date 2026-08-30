"""`_chat_parse()` liest den rohen LLM-Text der Stufe 3 — genau die Stelle, die
`test_rechenweg_durchgereicht.py` NICHT erreicht: jene Datei mockt `_llm_dialog` selbst, eine
Ebene OBERHALB von `_chat_parse`, und speist den fertigen Vorschlag (mit `rechenweg`) direkt ein.
Deshalb ist sie grün, obwohl der Verlust real ist.

BEFUND (gemessen gegen HEAD a4da29b0068604398c14de3ff0e81d901fac778e): `_chat_parse`
(`api_llm.py:405-434`) kopiert aus jedem Modell-Vorschlag genau fünf Schlüssel — `feld_id`,
`wert`, `beleg`, `begruendung`, `aussage`. `rechenweg` ist keiner davon, obwohl das Schema
(`DIALOG_SCHEMA`) es als Pflichtfeld (nullable) vom Modell verlangt und `api.py` es an zwei
Stellen bereits korrekt weiterreicht (`v.get("rechenweg")`), sobald es ankommt. Der Verlust
sitzt strukturell zwischen beiden — bei JEDEM Aufruf, nicht stichprobenhaft.

Zwei Tests unten, an zwei verschiedenen Stellen der Kette gemessen:
  1. `_chat_parse()` direkt gefüttert — die kleinste mögliche Nachweisstelle.
  2. der ganze Weg von `llm_client.complete()` (dem Netz-Rand, UNTERHALB von `_chat_parse`) bis
     zur `API.chat()`-Antwort — damit „repariert" nicht heißt „nur `_chat_parse` selbst gibt es
     zurück", sondern „es kommt auch bei der Oberfläche an".

Beide `xfail(strict=True)`: heute rot, ohne die Suite rot zu machen. `strict=True` heißt, sobald
`_chat_parse` `rechenweg` durchreicht, wird aus dem `xfail` ein `XPASS` — und der Testlauf schlägt
GENAU DESHALB fehl, statt still grün zu bleiben, bis jemand den Marker entfernt.

Gegenprobe (kein xfail, muss heute UND nach dem Fix grün bleiben): ein Vorschlag ohne
`rechenweg` darf keins erfinden.

NULL LLM: kein Netz-Call, kein Key, kein Cent — `_chat_parse` ist reine Textverarbeitung, und der
End-to-End-Test ersetzt nur `llm_client.complete` durch eine Fixture-Funktion (derselbe Rand, den
`tests/test_dialog_kanal.py` schon für den Kanal-Test ersetzt)."""
from __future__ import annotations

import json
import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/haut", "produkt/store", "produkt/traverser", "produkt/mapping",
             "produkt/unsicherheit", "produkt/import", "golden"):
    sys.path.insert(0, os.path.join(ROOT, _sub))

import api as API        # noqa: E402
import api_llm           # noqa: E402
import audit             # noqa: E402
import llm_client as LC  # noqa: E402

# Derselbe Fall wie in test_rechenweg_durchgereicht.py: 50.000 EUR im Jahr, sechs von zwölf
# Monaten — der Live-Lauf vom 2026-08-23, an dem der Verlust ursprünglich auffiel.
FELD = "bruttoarbeitslohn"
WERT = 2_500_000
RECHENWEG = {"basis": 5_000_000, "faktor": "6/12",
             "erklaerung": "50.000 € pro Jahr ÷ 12 × 6 Monate (ab Juli arbeitslos)"}


@pytest.mark.xfail(strict=True, reason="_chat_parse kopiert rechenweg nicht (api_llm.py:405-434)")
def test_chat_parse_reicht_rechenweg_durch():
    """Kleinste Nachweisstelle: roher Stufe-3-Text hinein, `rechenweg` muss im geparsten
    Vorschlag wieder herauskommen."""
    roh = json.dumps([{"feld_id": FELD, "wert": WERT, "beleg": "50k pro jahr",
                        "begruendung": "anteilig", "rechenweg": RECHENWEG}])
    out = api_llm._chat_parse(roh)
    assert out and out[0].get("feld_id") == FELD, "der Vorschlag selbst kam nicht durch"
    assert "rechenweg" in out[0], (
        "rechenweg fehlt im geparsten Vorschlag — _chat_parse kopiert es nicht (api_llm.py:405-434)")
    assert out[0]["rechenweg"] == RECHENWEG


def test_chat_parse_ohne_rechenweg_bleibt_sauber():
    """Gegenprobe: ein Vorschlag OHNE rechenweg läuft sauber durch und erzeugt keins — sonst
    bliebe dieser Test grün, obwohl ein künftiger Fix nur die halbe Richtung reicht (ein
    rechenweg, den keiner geschickt hat, wäre genauso falsch wie der fehlende)."""
    roh = json.dumps([{"feld_id": FELD, "wert": 6_200_000, "beleg": "62000 euro brutto",
                        "begruendung": "direkt genannt"}])
    out = api_llm._chat_parse(roh)
    assert out and out[0].get("feld_id") == FELD
    assert not out[0].get("rechenweg"), "rechenweg wurde erfunden, obwohl keins geschickt wurde"


@pytest.fixture
def fall(tmp_path, monkeypatch):
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    API.fall_anlegen({"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": "cpvr1"})
    return "cpvr1"


def _stub_complete(vorschlag: dict):
    """Ersetzt NICHT `_llm_dialog` (das säße oberhalb von `_chat_parse` — die Bauart, die
    test_rechenweg_durchgereicht.py schon grün zeigt, ohne den Verlust zu sehen), sondern den
    Netz-Rand `llm_client.complete` selbst. `_chat_parse` läuft dabei echt mit, über alle drei
    Stufen hinweg — derselbe Text kommt für jeden Aufruf zurück, genau wie in
    tests/test_dialog_kanal.py's `_fixture_complete`."""
    def complete(role, messages, fixture_id=None, schema=None):
        return LC.Completion(text=json.dumps(
            {"vorschlaege": [dict(vorschlag)], "antwort": "", "unsicher": False}))
    return complete


@pytest.mark.xfail(strict=True, reason="Verlust reicht bis in die API-Antwort durch (api_llm.py:405-434)")
def test_rechenweg_kommt_bis_in_die_chat_antwort(fall, monkeypatch):
    """Derselbe Verlust, diesmal end-to-end gemessen statt isoliert: von `llm_client.complete`
    (unterhalb von `_chat_parse`) bis zur echten `API.chat()`-Antwort. Beweist, dass ein Fix in
    `_chat_parse` allein auch beim Nutzer ankommt — und nicht durch eine zweite, ungetestete
    Stelle zwischen `_chat_parse` und der Oberfläche wieder verlorengeht."""
    monkeypatch.setattr(LC, "complete", _stub_complete(
        {"feld_id": FELD, "wert": WERT, "beleg": "50k pro jahr",
         "begruendung": "anteilig", "rechenweg": dict(RECHENWEG)}))
    st, body = API.chat(fall, {"text": "50k im jahr, seit juli arbeitslos"})
    assert st == 200, f"chat() antwortete {st}: {body}"
    v = body["vorschlaege"]
    assert v and v[0]["feld_id"] == FELD, "der Vorschlag kam gar nicht in der Antwort an"
    assert v[0].get("rechenweg") == RECHENWEG, (
        "rechenweg fehlt in der echten chat()-Antwort — derselbe Verlust wie in _chat_parse, "
        "hier end-to-end statt isoliert gemessen")
