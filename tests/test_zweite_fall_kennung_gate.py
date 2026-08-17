"""Jeder geladene Fall muss geprüft worden sein — namentlich, nicht ungefähr.

Zwei Löcher derselben Bauart in einer Woche, beide in Routen, die die Zugriffsnaht nur HALB
benutzten:

  fall_loeschen  hielt eine eigene, unveränderte Kopie der alten fail-open-Prüfung und rief
                 `_fall_owner_check` nie. Anonymer DELETE auf einen fremden Fall: 200, Datei
                 weg — vor wie nach dem Härten der Prüffunktion (behoben 39fcf79).
  vorjahr        prüfte `_fall_owner_check(fall_id)` für das ZIEL und lud danach die aus dem
                 Request-Body stammende Quell-Kennung ungeprüft. Ein eingeloggter Nutzer konnte
                 damit Felder aus einem FREMDEN Fall in seinen eigenen ziehen.

Beide waren einzeilig zu reparieren. Das Problem ist nicht die Zeile, sondern dass sie beim
nächsten Endpunkt wieder fehlen kann — und dass ein Test, der nur die Prüffunktion selbst
prüft, davon nichts merkt: die Funktion war ja korrekt, sie wurde nur nicht gerufen.

Deshalb strukturell statt punktuell. Der Test liest api.py per AST und verlangt: **wer
`lade_fall(X)` aufruft, ruft in derselben Funktion auch `_fall_owner_check(X)` — mit demselben
Ausdruck X.** Der Namensvergleich ist der Kern; `_fall_owner_check(fall_id)` neben
`lade_fall(vj_id)` ist genau der Fehler, den `vorjahr` hatte, und eine Prüfung, die nur
"irgendein owner_check kommt vor" verlangt, hätte ihn durchgewinkt.

Gemessen beim Bau (2026-08-17): 15 Funktionen rufen `lade_fall`, 14 davon mit passender
Prüfung, eine begründete Ausnahme.

NULL LLM.
"""
from __future__ import annotations

import ast
import os

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
API_PFAD = os.path.join(ROOT, "produkt", "haut", "api.py")

# Muster BLOCKIERTE_BLOECKE (tests/test_checkest_blockmatrix.py): kein stilles Ausklammern,
# jede Ausnahme trägt ihren Grund im Code und wird von den Tests unten ehrlich gehalten.
AUSNAHMEN = {
    "_fall_owner_check": (
        "IST die Prüfung. Sie lädt den Fall, um dessen user_id zu lesen — ein Aufruf ihrer "
        "selbst wäre eine Endlosschleife. Die einzige Stelle im Modul, an der lade_fall() "
        "ohne vorausgehende Prüfung stehen DARF."),
}


def _funktionen():
    with open(API_PFAD, encoding="utf-8") as f:
        return [n for n in ast.walk(ast.parse(f.read())) if isinstance(n, ast.FunctionDef)]


def _aufrufe(fn: ast.FunctionDef, name: str) -> list[str]:
    """Die ersten Argumente aller Aufrufe von `name` innerhalb von fn, als Quelltext."""
    return [ast.unparse(c.args[0])
            for c in ast.walk(fn)
            if isinstance(c, ast.Call) and isinstance(c.func, ast.Name)
            and c.func.id == name and c.args]


def test_jeder_geladene_fall_ist_namentlich_geprueft():
    """Der Kern: NICHT "die Funktion prüft irgendetwas", sondern "sie prüft GENAU die Kennung,
    die sie lädt". `vorjahr` erfüllte die schwache Fassung und war trotzdem offen."""
    verstoesse = []
    for fn in _funktionen():
        if fn.name in AUSNAHMEN:
            continue
        geladen = _aufrufe(fn, "lade_fall")
        geprueft = set(_aufrufe(fn, "_fall_owner_check"))
        for kennung in geladen:
            if kennung not in geprueft:
                verstoesse.append(
                    f"{fn.name} (Z.{fn.lineno}): lade_fall({kennung}) ohne "
                    f"_fall_owner_check({kennung}) — geprüft wird nur {sorted(geprueft) or 'nichts'}")
    assert not verstoesse, (
        "Fall geladen, aber nicht namentlich geprüft — dieselbe Lücke wie in vorjahr() und "
        "fall_loeschen():\n  " + "\n  ".join(verstoesse))


def test_zweite_kennung_aus_dem_body_wird_erkannt():
    """Negativprobe des Gates selbst: der eingebaute Fehler MUSS auffallen. Ohne diesen Test
    wäre nicht belegt, dass der Namensvergleich oben wirklich vergleicht — ein Gate, das
    seinen eigenen Fehlerfall nicht kennt, ist eine Behauptung."""
    quelle = (
        "def h(fall_id, body):\n"
        "    _fall_owner_check(fall_id)\n"
        "    a = lade_fall(fall_id)\n"
        "    b = lade_fall(body.get('anderer'))\n"
    )
    fn = [n for n in ast.walk(ast.parse(quelle)) if isinstance(n, ast.FunctionDef)][0]
    geladen = _aufrufe(fn, "lade_fall")
    geprueft = set(_aufrufe(fn, "_fall_owner_check"))
    offen = [k for k in geladen if k not in geprueft]
    assert offen == ["body.get('anderer')"], f"Gate erkennt die zweite Kennung nicht: {offen}"


def test_ausnahmen_sind_begruendet():
    for name, grund in AUSNAHMEN.items():
        assert grund and len(grund) > 20, f"{name}: Ausnahme ohne ausreichende Begründung"


def test_ausnahmen_haben_keine_toten_eintraege():
    """Wird eine ausgenommene Funktion umbenannt oder entfernt, muss der Eintrag mit — sonst
    wächst die Liste zu einem Friedhof, in dem eine echte Lücke nicht mehr auffällt."""
    vorhanden = {fn.name for fn in _funktionen()}
    tot = sorted(set(AUSNAHMEN) - vorhanden)
    assert not tot, f"Ausnahme(n) für nicht mehr existierende Funktion(en): {tot}"


def test_die_pruefung_wird_ueberhaupt_benutzt():
    """Untergrenze gegen den stillen Totalausfall: verschwände `_fall_owner_check` aus allen
    Aufrufern, wären alle Tests oben trivial grün (keine Ladung ohne Prüfung, weil gar nicht
    mehr geprüft wird — und auch nichts mehr gemeldet). Gemessen 2026-08-17: 14 Aufrufer."""
    ruft = sum(1 for fn in _funktionen() if _aufrufe(fn, "_fall_owner_check"))
    assert ruft >= 12, (
        f"Nur {ruft} Funktionen rufen _fall_owner_check (erwartet mindestens 12, gemessen 14) — "
        f"die Zugriffsnaht ist offenbar umgangen worden.")
