"""PII-Filter vor ausgehendem LLM-Call.

Filtert personenbezogene Daten aus Freitext, bevor er an einen externen
LLM-Provider geht. Gibt (gefilterter_text, getroffene_kategorien) zurück.

HINWEIS: FREIE PERSONENNAMEN werden bewusst NICHT erkannt. Ein Regex auf
Eigennamen trifft Steuerbegriffe ("Riester", "Rürup", "Ehegatte") und
zerstört den Freitext, aus dem das LLM Werte extrahieren soll. Die enge
Regel "Herr/Frau + folgendes Wort" ist der max. vertretbare Eingriff.
Geldbeträge und Paragraphen-Nennungen bleiben unangetastet — sonst kann
das LLM keine Cent-Werte mehr vorschlagen.

Reihenfolge: IBAN vor steuer_id prüfen (sonst frisst die 11-Ziffern-Regel
Teile der IBAN).
"""

from __future__ import annotations

import re

# IBAN: DE + 2 Ziffern + 10-30 alphanum, optionale Leerzeichen
_IBAN = re.compile(r"\b[A-Z]{2}\d{2}(?:[ ]?\d){10,30}\b")

# Steuer-ID: 11 Ziffern, optional mit Leerzeichen/Schrägstrich gruppiert
# Ein Regex: \b\d + (?:[ /]?\d){10} = 11 Digits total, Trennzeichen erlaubt.
# IBAN läuft VORHER (steht zuerst in _KATEGORIEN) — kein Konflikt mit DE...-IBAN.
_STEUER_ID = re.compile(r"\b\d(?:[ /]?\d){10}\b")

# Datum TT.MM.JJJJ
_DATUM = re.compile(r"\b(0[1-9]|[12]\d|3[01])\.(0[1-9]|1[0-2])\.\d{4}\b")

# PLZ + großgeschriebenes Wort: 5 Ziffern + Leerzeichen + Großbuchstabe.
# Negative Lookahead für Wörter, die nach 5 Ziffern KEIN Ort sind:
# Währungen (Euro, EUR, €), Einheiten (Euro, Kilometer, Meter, Stück, …),
# und häufige Steuerkontext-Wörter.
# Der Set ist klein und endlich — im Steuer-Freitext folgt auf 5 Ziffern
# fast immer "Euro" oder eine Einheit, kein echter Ortsname.
_PLZ_ORT = re.compile(
    r"\b\d{5}\s+"
    r"(?!(?:Euro|EUR|€|Kilometer|km|Meter|m|Stück|Tonnen|kg|g|Liter|"
    r"Jahre|Tage|Stunden|Mitglieder|Mitarbeiter|Einwohner|Jahr|Tag|Stunde)\b)"
    r"[A-ZÄÖÜ][a-zäöüß]+\b"
)

# Straße + Hausnummer: Wort auf -straße/-str./-weg/-allee/-platz + HAUSNUMMER PFLICHT.
# Ohne Hausnummer erzeugen "Arbeitsplatz", "Ehering", "Studienplatz" Fehlalarme.
# -straße/-strasse/-str. bleiben ohne Hausnummer erlaubt (sehr spezifisch, kaum
# Fehlalarme im Steuerkontext — "Arbeitsstraße" gibt es praktisch nicht).
_STRASSE = re.compile(
    r"\b[A-ZÄÖÜ][a-zäöüß]+(?:straße|strasse|str\.)"
    r"(?:\s+\d{1,4}[a-z]?)?"
    r"|"
    r"\b[A-ZÄÖÜ][a-zäöüß]+(?:weg|allee|platz|gasse|damm|ring|chaussee)"
    r"(?:\s+\d{1,4}[a-z]?)"
)

# Anrede (enge Regel): "Herr" oder "Frau" + folgendes Wort
_ANREDE = re.compile(r"\b(?:Herr|Frau)\s+[A-ZÄÖÜ][a-zäöüß]+\b")

_PLATZHALTER = "[PII]"

_KATEGORIEN: list[tuple[str, re.Pattern]] = [
    ("iban", _IBAN),
    ("steuer_id", _STEUER_ID),
    ("datum", _DATUM),
    ("plz_ort", _PLZ_ORT),
    ("strasse", _STRASSE),
    ("anrede_name", _ANREDE),
]


def filtere(text: str) -> tuple[str, list[str]]:
    """Ersetzt PII im Text durch Platzhalter.

    Args:
        text: Roher Freitext (Nutzereingabe).

    Returns:
        (gefilterter Text, sortierte Liste der getroffenen Kategorien).
    """
    if not text:
        return text, []

    getroffen: list[str] = []
    for kategorie, pattern in _KATEGORIEN:
        neuer_text, n = pattern.subn(_PLATZHALTER, text)
        if n > 0:
            getroffen.append(kategorie)
            text = neuer_text

    return text, sorted(getroffen)