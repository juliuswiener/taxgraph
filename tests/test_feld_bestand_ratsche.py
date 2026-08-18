"""Ein Feld darf nicht stillschweigend aus der Bindung verschwinden — Nutzerangaben hängen daran.

Phase 5, Punkt 5 des Refactor-Plans: "Künftige Bindungs-Feld-Entfernung erzwingt paired
Migration-Eintrag."

WAS PASSIERT, WENN EIN FELD VERSCHWINDET: die Angaben, die Nutzer dazu gemacht haben, sind aus
der Oberfläche weg, aus der Deklaration weg, aus dem Ergebnis weg. Die Events bleiben im Store
liegen und materialisieren weiter zu einem Wert, den niemand mehr ansieht — `materialisiere()`
übernimmt jede feld_id, ohne gegen die Bindung zu prüfen. Kein Absturz, keine Fehlermeldung:
eine Angabe ist einfach nicht mehr da.

(Der Audit-Bericht beschrieb das als "verwaiste Events werden still gedroppt". Nachgemessen ist
es umgekehrt — sie werden übernommen und leben weiter. Für den Nutzer ist die Folge dieselbe:
seine Angabe wirkt nicht mehr.)

BELEGT, NICHT AUSGEDACHT: bb76dea hat Partner-Felder umgehängt und dabei 15+ vorbestehende Tests
umgerissen. Aufgefallen ist es nur, weil Tests auf die feld_ids zeigten — hätte es damals echte
Fälle gegeben, wären deren Angaben still verschwunden.

DIE RATSCHE KENNT NUR EINE RICHTUNG: neue Felder brauchen keinen Eintrag (sie können nichts
verlieren). Nur das Verschwinden muss begründet werden, und das ist der seltene, gefährliche
Fall. Deshalb ist der Pflegeaufwand fast null, anders als bei einer Liste, die jedes neue Feld
mitführen müsste — solche Listen verrotten, und eine verrottete Liste ist schlimmer als keine.

NULL LLM.
"""
from __future__ import annotations

import os
import sys

import pytest

yaml = pytest.importorskip("yaml")

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, os.path.join(ROOT, "produkt", "traverser"))

import traverser as TR  # noqa: E402

BESTAND = os.path.join(ROOT, "produkt", "bindung", "FELD_BESTAND.yaml")


def _bestand() -> dict:
    with open(BESTAND, encoding="utf-8") as f:
        return yaml.safe_load(f) or {}


def _heutige_felder() -> set[str]:
    return set(TR.lade_bindung())


def test_kein_feld_ist_unbegruendet_verschwunden():
    """Der Kern. Was einmal in der Bindung stand, steht entweder noch da oder in `entfernt:`
    mit einem Grund."""
    doc = _bestand()
    erfasst = set(doc.get("felder") or [])
    entfernt = doc.get("entfernt") or {}
    heute = _heutige_felder()

    verschwunden = sorted(erfasst - heute - set(entfernt))
    assert not verschwunden, (
        f"{len(verschwunden)} feld_id(s) sind aus der Bindung verschwunden, ohne dass jemand "
        f"gesagt hat, was aus vorhandenen Nutzerangaben werden soll:\n  "
        + "\n  ".join(verschwunden[:15])
        + f"\n\nEntweder das Feld wiederherstellen, oder es in {os.path.basename(BESTAND)} "
        f"unter `entfernt:` eintragen — mit Grund UND mit dem, was mit bereits erfassten "
        f"Angaben geschieht. Ein Feld zu streichen ist erlaubt; es kommentarlos zu streichen "
        f"nicht.")


def test_entfernte_felder_sind_begruendet():
    """Kein stilles Ausklammern: ein leerer Grund ist kein Grund. Verlangt wird ausserdem eine
    Aussage über die BESTANDSDATEN — 'wird nicht mehr gebraucht' erklärt, warum das Feld weg
    ist, aber nicht, was mit den Angaben passiert, die schon jemand gemacht hat."""
    entfernt = _bestand().get("entfernt") or {}
    for feld, grund in entfernt.items():
        assert isinstance(grund, str) and len(grund) > 30, (
            f"{feld}: Begründung fehlt oder ist zu knapp ({grund!r}) — sie muss auch sagen, "
            f"was mit bereits erfassten Angaben geschieht")


def test_entfernte_felder_sind_wirklich_weg():
    """Tote Einträge: ein Feld, das wieder in der Bindung steht, gehört nicht mehr in
    `entfernt:`. Sonst deckt der Eintrag beim nächsten Mal ein echtes Verschwinden mit ab."""
    doc = _bestand()
    heute = _heutige_felder()
    zombies = sorted(f for f in (doc.get("entfernt") or {}) if f in heute)
    assert not zombies, (
        f"{zombies} stehen unter `entfernt:`, sind aber wieder in der Bindung — Eintrag "
        f"streichen, sonst deckt er das nächste echte Verschwinden mit ab.")


def test_die_erfassung_ist_nicht_leer():
    """Gegenprobe gegen die bequemste Art, diese Ratsche loszuwerden: die Liste leeren. Dann
    wäre `erfasst - heute` immer leer und der Test grün, ohne irgendetwas zu prüfen — dieselbe
    Falle wie ein Fixture, das seine eigene Voraussetzung mitbringt."""
    erfasst = set(_bestand().get("felder") or [])
    assert len(erfasst) >= 250, (
        f"nur {len(erfasst)} Felder erfasst (erwartet ≥ 250, bei der Erfassung waren es 307) — "
        f"eine geleerte Liste macht diese Ratsche wirkungslos, ohne dass ein Test rot wird")


def test_die_ratsche_erkennt_ihren_eigenen_fehlerfall():
    """Ohne diese Probe wäre nicht belegt, dass der Vergleich überhaupt anschlägt: die Ratsche
    ist im Normalfall grün, man sieht sie also nie arbeiten."""
    erfasst = {"a", "b", "c"}
    heute = {"a", "b"}
    entfernt = {}
    assert sorted(erfasst - heute - set(entfernt)) == ["c"]
    entfernt = {"c": "x" * 40}
    assert not (erfasst - heute - set(entfernt))


def test_neue_felder_brauchen_keinen_eintrag():
    """Die Richtung, die bewusst NICHT geprüft wird — festgehalten, damit niemand sie
    'vervollständigt'. Eine Liste, die jedes neue Feld mitführen muss, wird bei 307 Feldern und
    laufender Entwicklung vergessen; dann steht sie irgendwann falsch da und deckt ein echtes
    Verschwinden mit ab. Ein neues Feld kann ausserdem nichts verlieren."""
    erfasst = set(_bestand().get("felder") or [])
    neu = sorted(_heutige_felder() - erfasst)
    assert True, f"{len(neu)} neue Felder seit der Erfassung — bewusst ohne Eintrag: {neu[:5]}"
