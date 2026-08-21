"""Eine Frage darf dem Nutzer keinen Sachverhalt unterstellen, den er nie angegeben hat.

GEMESSEN 2026-08-21 im echten Nutzerlauf (Julius, Fall demo-1787301323980). Der Fragetext von
`vpf_frist_nicht_unterbrochen` lautete:

    "Du warst mehr als 3 Monate am selben Ort, hast aber keine Reisetage nach Ablauf der
     3 Monate angegeben. Lief die Tätigkeit dort durchgehend, ohne eine Pause von 4 Wochen
     oder mehr?"

Julius hatte zu Orten NICHTS angegeben. Er antwortete mit „ja". Seither liegt in seinem Fall:

    08:39:25 | ui:laie | vpf_frist_nicht_unterbrochen = True | zustand: bestaetigt

Eine bestätigte Angabe über einen Sachverhalt, den es nicht gibt. Das ist der Schaden, gegen den
dieser Test steht — nicht die Irritation, sondern die falsche Bestätigung.

WARUM DER TEXT ÜBERHAUPT SO WAR, und warum das kein Schlamperei-Fund ist: für den RECHENPFAD
stimmten beide Behauptungen. bescheid_deklaration.py:388-405 stellt die Rückfrage nur, wenn
`vpf_monate_am_ort > 3` UND Tage gesamt > 0 UND alle Tage nach Frist == 0 — dort ist die
Einleitung belegt. Der DIALOG kennt diese Bedingung nicht: `traverser.naechste_fragen` filtert
allein nach askable / unbeantwortet / Regel-nicht-ausgeschlossen, eine feldbezogene
Sichtbarkeitsbedingung gibt es nicht. Wer den Text für den einen Pfad schreibt, schreibt ihn
zwangsläufig falsch für den anderen. Dieselbe Klasse wie [[naht-blindstelle-zwei-repraesentationen]].

WAS DIESER TEST NICHT LEISTET: er prüft die FORM des Textes, nicht die Sichtbarkeit der Frage. Die
eigentliche Lösung wäre, dass der Dialog die Frage nur unter denselben Bedingungen zeigt wie der
Rechenpfad. Das ist ein Eingriff in den Traverser, der alle askable Felder beträfe, und bewusst
nicht Teil dieses Fixes. Solange es ihn nicht gibt, ist die ehrliche Formulierung die Absicherung.

NULL LLM.
"""
from __future__ import annotations

import os
import re
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, os.path.join(ROOT, "produkt/traverser"))

import traverser as TR  # noqa: E402

BINDUNG = TR.lade_bindung()


# Ein Fragetext, der so BEGINNT, erzählt dem Nutzer, was er getan hat, statt ihn zu fragen.
# Bewusst auf den Satzanfang beschränkt: mitten im Satz ist „du hast" oft eine legitime
# Rückbindung an eine schon gegebene Antwort („… den du hast eintragen lassen"), am Anfang
# fast nie.
_BEHAUPTUNG = re.compile(
    r"^\s*Du\s+(hast|hattest|warst|bist|wurdest|hast\s+dir|zahlst|zahltest|nutzt|nutztest"
    r"|gibst|gabst|besitzt|erhieltst|bekamst)\b",
    re.I)

# Ausnahmen brauchen einen Grund und einen Beleg, dass die Behauptung im Dialog IMMER zutrifft —
# also aus einer Antwort folgt, die zwingend vorher kam. Leer, und das soll so bleiben.
BEHAUPTUNG_ERLAUBT: dict[str, str] = {}


def _askable_texte() -> list[tuple[str, str]]:
    return [(fid, (b.get("fragetext_laie") or "").strip())
            for fid, b in sorted(BINDUNG.items()) if b.get("askable")]


def test_kein_fragetext_unterstellt_dem_nutzer_etwas():
    """Der Sweep. Deckt jedes künftige Feld mit ab, nicht nur das eine gefundene."""
    schaden = []
    for fid, text in _askable_texte():
        if not text or fid in BEHAUPTUNG_ERLAUBT:
            continue
        if _BEHAUPTUNG.match(text):
            schaden.append(f"{fid}: {text[:110]}")
    assert not schaden, (
        "Fragetexte behaupten einen Sachverhalt, statt ihn zu erfragen:\n"
        + "\n".join(f"   - {z}" for z in schaden)
        + "\n\nDer Dialog stellt JEDE fragbare Frage — auch dem Nutzer, auf den die Behauptung "
          "nicht zutrifft (traverser.naechste_fragen kennt keine feldbezogene "
          "Sichtbarkeitsbedingung). Wer sie bejaht, bestätigt etwas, das es nicht gibt.\n"
          "Umformulieren, ohne die Polarität zu ändern: die Bedingung gehört in hilfe_kurz, "
          "die Frage selbst muss für jeden beantwortbar sein. Echte Ausnahme? Dann nach "
          "BEHAUPTUNG_ERLAUBT mit Begründung UND Beleg, dass die Behauptung im Dialog immer gilt.")


def test_das_muster_greift_ueberhaupt():
    """Ohne diese Probe wäre der Test oben vakuum-grün, sobald jemand die Regex zerschiesst."""
    assert _BEHAUPTUNG.match("Du warst mehr als 3 Monate am selben Ort, hast aber …")
    assert _BEHAUPTUNG.match("Du hast Kinder unter 14 Jahren?")
    # Gegenprobe: echte Fragen und legitime Rückbindungen mitten im Satz bleiben unbehelligt.
    assert not _BEHAUPTUNG.match("Lief deine Tätigkeit am selben Ort durchgehend?")
    assert not _BEHAUPTUNG.match("Wie viele Kilometer sind es zur Arbeit?")
    assert not _BEHAUPTUNG.match("Wurde dir ein Frühstück gestellt?")
    assert not _BEHAUPTUNG.match("Welchen Betrag hast du gezahlt?")


def test_der_gefundene_fall_ist_repariert():
    """Namentlich, damit ein Rückbau auffällt und nicht bloss der Sweep gruen bleibt."""
    b = BINDUNG.get("vpf_frist_nicht_unterbrochen")
    assert b is not None, "Feld verschwunden — dann gehört dieser Test angepasst, nicht gelöscht."
    text = (b.get("fragetext_laie") or "")
    assert not _BEHAUPTUNG.match(text), f"der Fragetext behauptet wieder etwas: {text[:110]!r}"
    hilfe = (b.get("hilfe_kurz") or "")
    assert "3 Monate" in hilfe or "drei Monate" in hilfe, (
        "Die Bedingung, unter der die Frage überhaupt zählt, muss in hilfe_kurz stehen — sonst "
        "ist die Frage zwar ehrlich, aber unbeantwortbar.")


@pytest.mark.parametrize("feld", sorted(BEHAUPTUNG_ERLAUBT))
def test_ausnahmen_sind_noch_noetig(feld):
    """Eine Ausnahme ist eine Schuld: läuft das Feld sauber, muss der Eintrag raus."""
    b = BINDUNG.get(feld)
    assert b is not None, f"{feld} gibt es nicht mehr — Eintrag aus BEHAUPTUNG_ERLAUBT entfernen."
    assert _BEHAUPTUNG.match((b.get("fragetext_laie") or "")), (
        f"{feld} steht als Ausnahme, behauptet aber nichts mehr — Eintrag entfernen.")
