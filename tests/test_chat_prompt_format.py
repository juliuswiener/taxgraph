"""Der Chat-Prompt muss zum erzwungenen Antwortformat passen — und darf keine Abwesenheit erfinden.

Zwei Befunde aus der Inbetriebnahme am 2026-08-14, beide erst im echten Betrieb sichtbar:

1. FORMAT-KONFLIKT. llm_client sendet response_format={"type":"json_object"}; dieser Modus verlangt
   ein OBJEKT an der Wurzel. Der Prompt verlangte ein nacktes Array. Das Modell löste den
   Widerspruch, indem es EIN Objekt lieferte — also genau einen Vorschlag, egal wie viele Werte im
   Text standen. Am selben Satz gemessen: einmal 8 Vorschläge (Array, gegen den Modus), zweimal
   nur 1. Das sah aus wie Modell-Laune und war ein Vertragsbruch zwischen Client und Prompt.
   Nach der Umstellung auf {"vorschlaege": [...]}: stabil 4 von 4 erkennbaren Werten.

2. ERFUNDENE ABWESENHEIT. Aus "Ich bin Arbeitnehmer, verheiratet, fahre an 220 Tagen 15 km zur
   Arbeit und habe 62000 Euro brutto verdient" schlug das Modell zusätzlich kein_gewinn, kein_kap,
   kein_vuv und kein_sonstige = true vor. Das hat der Nutzer nicht gesagt — ein Arbeitnehmer kann
   ein Depot haben. Eine erfundene Abwesenheit ist gefährlicher als ein erfundener Betrag: ein zu
   hoher Betrag fällt beim Bestätigen auf, ein "nein, hatte ich nicht" klingt plausibel und wird
   durchgewunken; dann fehlt eine ganze Einkunftsart (Under-Deklaration).

Beide Regeln stehen im System-Prompt. Dieser Test prüft, dass sie dort BLEIBEN — ohne echten Call.

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

KATALOG = [{"feld_id": "bruttoarbeitslohn", "fragetext_laie": "Wie hoch war dein Bruttolohn?",
            "hilfe_kurz": "Steht auf der Lohnsteuerbescheinigung.", "typ": "cent"},
           {"feld_id": "kein_kap", "fragetext_laie": "Hattest du Kapitalerträge?",
            "hilfe_kurz": "Zinsen, Dividenden.", "typ": "bool"}]


def _system() -> str:
    return api_llm._chat_prompt("egal", KATALOG)[0]["content"]


def test_prompt_verlangt_ein_objekt_kein_nacktes_array():
    """Muss zum json_object-Modus des Clients passen, sonst kommt nur ein Vorschlag durch."""
    s = _system()
    assert "vorschlaege" in s, "Der Objekt-Wrapper {'vorschlaege': [...]} fehlt im Prompt"
    assert "JSON-OBJEKT" in s.upper() or "JSON-OBJECT" in s.upper(), (
        "Der Prompt sagt nicht, dass ein Objekt erwartet wird")
    assert "JSON-Array" not in s, (
        "Der Prompt verlangt weiter ein nacktes Array — im json_object-Modus liefert das Modell "
        "dann ein einzelnes Objekt, also genau EINEN Vorschlag.")


def test_prompt_verlangt_ALLE_erkannten_felder():
    """Ohne diesen Hinweis liefert das Modell gern den ersten Treffer und hört auf."""
    s = _system()
    assert "nicht nur den ersten" in s or "alle, die du erkennst" in s, (
        "Der Prompt fordert nicht ausdrücklich alle erkannten Felder")


def test_prompt_verbietet_erfundene_abwesenheit():
    s = _system()
    assert "kein_" in s and "ausdrücklich" in s, (
        "Die Sonderregel für kein_-Felder (Abwesenheit nur bei ausdrücklicher Nennung) fehlt")
    assert "Arbeitnehmer" in s, (
        "Das konkrete Gegenbeispiel fehlt — ohne es hat das Modell genau diesen Fehlschluss gemacht")


def test_die_regel_gilt_ausdruecklich_nur_fuer_kein_felder():
    """Erste Fassung endete mit 'Im Zweifel dieses Feld weglassen'. Das Modell bezog die
    Zurückhaltung auf ALLE Felder und lieferte statt acht nur noch einen Vorschlag — die Regel
    muss ihre eigene Reichweite benennen."""
    s = _system()
    assert "NUR FÜR FELDER MIT PRÄFIX" in s.upper() or "gilt diese Zurückhaltung ausdrücklich NICHT" in s, (
        "Die Reichweite der kein_-Sonderregel ist nicht eingegrenzt — sie schlägt sonst auf alle "
        "Felder durch.")


def test_parser_versteht_den_objekt_wrapper():
    """Gegenprobe auf der Empfangsseite: was der Prompt verlangt, muss der Parser lesen können."""
    roh = ('{"vorschlaege": [{"feld_id": "bruttoarbeitslohn", "wert": 6200000, "begruendung": "x"},'
           ' {"feld_id": "ep_arbeitstage", "wert": 220, "begruendung": "y"}]}')
    out = api_llm._chat_parse(roh)
    assert [v["feld_id"] for v in out] == ["bruttoarbeitslohn", "ep_arbeitstage"]
    assert out[0]["wert"] == 6200000


def test_parser_bleibt_tolerant_gegen_das_alte_format():
    """Ein nacktes Array und ein einzelnes Objekt müssen weiter durchgehen — sonst bricht der
    Chat, sobald ein Modell sich nicht an den Wrapper hält."""
    assert len(api_llm._chat_parse('[{"feld_id": "a", "wert": 1}]')) == 1
    assert len(api_llm._chat_parse('{"feld_id": "a", "wert": 1}')) == 1
    assert api_llm._chat_parse("kein json") == []
