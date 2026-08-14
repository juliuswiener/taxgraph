"""Ein Vorschlag ohne wörtlichen Beleg im Nutzertext wird verworfen.

Julius 2026-08-14: "wir müssen das modell zwingen den beleg für die behauptung (als quote des
users zb) mit zu schicken."

Der Zwang allein wäre nur eine Prompt-Bitte. Der Wert entsteht erst durch die GEGENPROBE: das
Modell kann eine Begründung frei erfinden, aber kein Zitat, das im Text nicht vorkommt. Deshalb
prüft _beleg_geprueft() jedes `beleg` gegen den Freitext und wirft raus, was sich nicht belegen
lässt — deterministisch, ohne darauf zu vertrauen, dass sich das Modell an Regeln hält.

Anlass war ein gemessener Fall: aus "Ich bin Arbeitnehmer, verheiratet, fahre an 220 Tagen 15 km
zur Arbeit und habe 62000 Euro brutto verdient" schlug das Modell zusätzlich kein_gewinn,
kein_kap, kein_vuv und kein_sonstige = true vor. Für keines dieser vier gibt es eine Textstelle.
Mit dem Gate fallen sie, auch wenn die Prompt-Regel einmal versagt.

Geprüft wird gegen den PII-GEFILTERTEN Text — genau den hat das Modell gesehen. Gegen das Original
zu prüfen würde legitime Belege verwerfen, sobald der Filter etwas maskiert hat.

NULL LLM.
"""
from __future__ import annotations

import os
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
for sub in ("produkt/haut", "produkt/store", "produkt/traverser", "produkt/mapping",
            "produkt/unsicherheit", "golden", "produkt/import", "produkt/auth"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import api_llm  # noqa: E402

TEXT = ("Ich bin Arbeitnehmer, verheiratet, fahre an 220 Tagen 15 km zur Arbeit "
        "und habe 62000 Euro brutto verdient.")


def _v(feld, wert, beleg):
    return {"feld_id": feld, "wert": wert, "beleg": beleg, "begruendung": "egal"}


def test_belegter_vorschlag_bleibt():
    behalten, verworfen = api_llm._beleg_geprueft(
        [_v("bruttoarbeitslohn", 6200000, "62000 Euro brutto verdient")], TEXT)
    assert [x["feld_id"] for x in behalten] == ["bruttoarbeitslohn"]
    assert not verworfen


def test_erfundene_abwesenheit_faellt_raus():
    """Der gemessene Fall: vier kein_-Felder, für die es keine Textstelle gibt."""
    vorschlaege = [_v("bruttoarbeitslohn", 6200000, "62000 Euro brutto"),
                   _v("kein_kap", True, "Ich bin Arbeitnehmer, also keine Kapitalerträge"),
                   _v("kein_vuv", True, "keine Vermietung"),
                   _v("kein_sonstige", True, "nichts weiter")]
    behalten, verworfen = api_llm._beleg_geprueft(vorschlaege, TEXT)
    assert [x["feld_id"] for x in behalten] == ["bruttoarbeitslohn"]
    assert {x["feld_id"] for x in verworfen} == {"kein_kap", "kein_vuv", "kein_sonstige"}


def test_paraphrase_zaehlt_nicht_als_beleg():
    """"sechzigtausend" statt "62000" — inhaltlich nah, aber nicht im Text. Genau solche
    Umschreibungen sind das Einfallstor, durch das ein erfundener Wert plausibel wirkt."""
    behalten, verworfen = api_llm._beleg_geprueft(
        [_v("bruttoarbeitslohn", 6000000, "sechzigtausend Euro brutto")], TEXT)
    assert not behalten and len(verworfen) == 1


def test_fehlender_oder_leerer_beleg_faellt_raus():
    behalten, _ = api_llm._beleg_geprueft(
        [{"feld_id": "bruttoarbeitslohn", "wert": 1, "begruendung": "ohne beleg"},
         _v("ep_arbeitstage", 220, "")], TEXT)
    assert not behalten


def test_zu_kurzer_beleg_zaehlt_nicht():
    """Ein einzelnes Zeichen steht in fast jedem Text und belegt nichts — sonst könnte sich das
    Modell mit "5" oder "e" jeden beliebigen Wert genehmigen."""
    behalten, _ = api_llm._beleg_geprueft([_v("ep_entfernung_km", 999, "1")], TEXT)
    assert not behalten


def test_grossschreibung_und_leerzeichen_schaden_nicht():
    """Ein inhaltlich richtiger Beleg darf nicht an Formatierung scheitern — Modelle zitieren
    gern mit veränderter Groß-/Kleinschreibung oder normalisierten Leerzeichen."""
    behalten, _ = api_llm._beleg_geprueft(
        [_v("ep_arbeitstage", 220, "AN   220 TAGEN")], TEXT)
    assert [x["feld_id"] for x in behalten] == ["ep_arbeitstage"]


def test_schema_erzwingt_den_beleg():
    """Die zweite Hälfte: das Schema, das der Provider durchsetzt. Ohne `beleg` in `required`
    dürfte das Modell es weglassen, und das Gate oben verwürfe dann einfach alles."""
    s = api_llm.CHAT_SCHEMA
    assert s.get("strict") is True, "strict fehlt — der Provider erzwingt das Schema dann nicht"
    item = s["schema"]["properties"]["vorschlaege"]["items"]
    assert "beleg" in item["required"], "beleg ist nicht verpflichtend"
    assert item["additionalProperties"] is False
    assert "wörtlich" in item["properties"]["beleg"]["description"].lower(), (
        "Die Schema-Beschreibung sagt dem Modell nicht, dass ein WÖRTLICHES Zitat verlangt ist")
