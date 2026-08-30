"""Sperrgrund-Klartext im Browser: der Backend-Satz muss die Anzeige auch erreichen.

Anlass 2026-08-28, zwei Vault-Funde zum selben Defekt aus zwei Winkeln:
  - backlog/taxgraph/sperrgruende-erreichen-den-nutzer-nicht.md: 23 von 39 Sperrgruenden
    fallen im Browser auf eine rohe Feldliste durch ("Noch offen: " + r.offen.join(", ")),
    weil app.js::zeigeErgebnis() das vom Backend gelieferte `klartext`-Feld (GET
    /fall/{id}/ergebnis, ueber SPERRGRUND_KLARTEXT befuellt) nirgends liest.
  - backlog/taxgraph/guard-sperrgruende-leerer-satz-im-browser.md: 22 davon zeigen sogar
    NUR "Noch offen: " ohne jeden Inhalt danach, weil `r.offen` im Guard-Zweig
    (produkt/haut/api.py, _ergebnis_roh) fuer alle 36 _an_gesamt_sperrgrund-Gruende
    hartkodiert `[]` ist.

Beide Funde beschreiben DIESELBE rohe-Feldliste-Zeile in app.js (Zeile 2326) und denselben
Fix ("Fertig ist es, wenn" in beiden Eintraegen): zeigeErgebnis() liest `r.klartext`, wenn
vorhanden, als PRIMAERE Quelle -- nicht nachrangig zu GUARD. GUARD und die beiden
else-if-Sonderfaelle (kein_scheiben_gesamtbescheid, engine_unavailable) "koennen danach
entfallen" (Vault-Wortlaut) -- MUESSEN es aber nicht. Das entscheidet, WIE dieses Gate
pruefen darf:

Warum keine Namensliste (GUARD-Keys vs. SPERRGRUND_KLARTEXT) als Pass/Fail-Kriterium taugt
------------------------------------------------------------------------------------------
Beide Vault-Wegwerf-Skripte zaehlen "durchfallende Gruende" ueber eine Mengendifferenz:
SPERRGRUND_KLARTEXT-Schluessel minus (GUARD-Schluessel vereinigt mit den beiden
else-if-Literalen). Das ist eine gute MESSUNG des heutigen Zustands (s. `_heutige_durchfaller`
unten, als Diagnose wiederverwendet) -- aber ein SCHLECHTES Dauerkriterium: der Zielzustand
(klartext als primaere Quelle) macht die Frage "ist dieser Grund-Name in GUARD gelistet?"
irrelevant, WEIL jeder SPERRGRUND_KLARTEXT-Grund per Definition einen `klartext` mitbekommt
(s. tests/test_sperrgrund_klartext.py::test_jeder_grund_hat_klartext). Ein Test, der weiter
auf die Namensliste prueft, bliebe nach dem beschriebenen Fix fuer immer ROT, ausser jemand
pflegt GUARD auf alle 39 Eintraege nach -- also genau die Doppelpflege, die dieser ganze
Fund beseitigen soll. Eine vierte, hand-gepflegte Liste waere zudem genau die Bauart
("zwei Repraesentationen, ungetestete Uebergabe"), die in diesem Repo schon mehrfach
auseinandergelaufen ist (s. Moduldoc test_sperrgrund_klartext.py).

Stattdessen: strukturelle Pruefung
-----------------------------------
Dieses Gate prueft nicht NAMEN, sondern die REIHENFOLGE der if/else-if/else-Kette in
zeigeErgebnis()'s Guard-Zweig: erst muss geklaert sein, DASS die Kette ueberhaupt aus
mehreren Zweigen besteht (Blindheits-Waechter, s. test_kette_wird_ueberhaupt_gefunden),
dann: steht VOR dem Zweig, der die rohe Feldliste ("Noch offen: ...") zuweist, irgendeine
Bedingung, die `klartext` prueft? Wenn ja, ist die rohe Feldliste fuer jeden Grund mit einem
echten `klartext` (das sind laut Definition ALLE 39 aus SPERRGRUND_KLARTEXT) unerreichbar --
unabhaengig davon, ob GUARD daneben als totes Erbe stehen bleibt oder geloescht wird. Das ist
robust gegen die im Auftrag genannte plausible Falsch-Reparatur ("klartext irgendwo
hinschreiben, ohne ihn anzuzeigen"): eine Zeile wie "const x = r.klartext;" ausserhalb dieser
konkreten if/else-if-Kette aendert an der Kettenstruktur nichts und haelt den Test ROT.

Zahl-Falle (Auftrag des Instructors: nicht auf 22/23/25 verlassen, selbst zaehlen)
-----------------------------------------------------------------------------------
Die beiden Vault-Eintraege nennen 23 bzw. 22 durchfallende Gruende, ein Kollege kam laut dem
ersten Eintrag auf 25. Dieser Test verlaesst sich auf KEINE der drei Zahlen. `_heutige_durchfaller`
zaehlt bei JEDEM Lauf frisch nach derselben Methodik wie die beiden Wegwerf-Skripte (GUARD-
Schluessel per Zeilenanfang-vor-Doppelpunkt-Regex aus dem `const GUARD = {...};`-Block, plus
die beiden else-if-Literale ueber ein `r.grund === "..."`-Muster, beides gegen
SPERRGRUND_KLARTEXT verglichen) und meldet die eigene Zahl in der Fehlermeldung von
test_kein_sperrgrund_faellt_auf_die_rohe_feldliste_durch, WENN diese (strukturelle) Pruefung
rot ist. Eigene Zaehlung am 2026-08-28 (HEAD b7c65b1): 23 Durchfaller unter 39
SPERRGRUND_KLARTEXT-Eintraegen -- deckt sich mit dem ERSTEN Vault-Eintrag, nicht mit der
25 des Kollegen oder der 22 des zweiten Eintrags (der zaehlt eine ANDERE, engere Frage: von
den 23 zeigen 22 zusaetzlich eine LEERE Liste statt einer nicht-leeren -- s. dortiger Eintrag).
Wenn diese Zahl bei einem spaeteren Lauf abweicht, ist das ein eigener Befund, kein Test-Fehler.
"""

from __future__ import annotations

import os
import pathlib
import re
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/haut", "produkt/store", "produkt/traverser", "produkt/unsicherheit",
             "produkt/mapping", "produkt/konsistenz", "produkt/import", "produkt/engine",
             "produkt/bescheid", "golden", "elster"):
    _p = os.path.join(ROOT, _sub)
    if _p not in sys.path:
        sys.path.insert(0, _p)

import bescheid_deklaration as BD  # noqa: E402

APP_JS_PFAD = pathlib.Path(ROOT) / "produkt" / "haut" / "static" / "app.js"


def _app_js_text() -> str:
    return APP_JS_PFAD.read_text(encoding="utf-8")


def _guard_block(text: str) -> str:
    """Der Zweig in zeigeErgebnis(), der den Guard-Fall behandelt (r.zahl_cent === null):
    vom Setzen der Guard-CSS-Klasse bis zum Beginn des Erfolgs-Zweigs ("} else {"). Beides
    stabile, inhaltliche Anker statt Zeilennummern -- die verschieben sich mit jeder
    Nachbar-Aenderung, ohne dass dieser Test etwas davon wissen muss."""
    anker = 'el.className = "ergebnis ergebnis-guard";'
    try:
        start = text.index(anker)
        ende = text.index("} else {", start)
    except ValueError as e:
        raise AssertionError(
            "zeigeErgebnis() wurde umgebaut -- der Anker "
            f"{anker!r} bzw. der anschliessende Erfolgs-Zweig ist nicht mehr auffindbar. "
            "Dieses Gate prueft dann nichts mehr und muss nachgezogen werden."
        ) from e
    return text[start:ende]


def _branch_kette(block: str) -> list[tuple[str, str | None, str]]:
    """Die if/else-if/else-Kette, die el.textContent setzt, in Quelltext-Reihenfolge --
    (Zweigart, Bedingungstext oder None bei 'else', zugewiesener Ausdruck)."""
    muster = re.compile(r'(if|else if|else)\s*(?:\(([^)]*)\))?\s*el\.textContent\s*=\s*([^;]*);')
    return [(m.group(1), m.group(2), m.group(3)) for m in muster.finditer(block)]


def _guard_keys(text: str) -> set[str]:
    """Schluessel im `const GUARD = {...};`-Objekt -- dieselbe Regex wie im
    Vault-Wegwerf-Skript (backlog/taxgraph/sperrgruende-erreichen-den-nutzer-nicht.md)."""
    m = re.search(r'const GUARD = \{(.*?)\n\};', text, re.S)
    if not m:
        raise AssertionError("const GUARD = {...}; nicht mehr in app.js gefunden.")
    return set(re.findall(r'^\s*(\w+):', m.group(1), re.M))


def _sonderfall_gruende(text: str) -> set[str]:
    """Die beiden fest verdrahteten else-if-Literale (kein_scheiben_gesamtbescheid,
    engine_unavailable) -- per Regex, nicht von Hand kopiert."""
    return set(re.findall(r'r\.grund === "(\w+)"', text))


def _heutige_durchfaller(text: str) -> set[str]:
    """Diagnose, NICHT das Pass/Fail-Kriterium (s. Moduldoc, 'Zahl-Falle'): welche
    SPERRGRUND_KLARTEXT-Gruende erkennt weder GUARD noch einer der beiden else-if-Sonderfaelle
    -- also welche fallen HEUTE, ohne den beschriebenen Fix, auf die rohe Feldliste durch."""
    behandelt = _guard_keys(text) | _sonderfall_gruende(text)
    return set(BD.SPERRGRUND_KLARTEXT) - behandelt


# ---------------------------------------------------------------- Blindheits-Waechter

def test_sperrgrund_klartext_ist_nicht_trivial_klein():
    """Wie test_sperrgrund_klartext.py::test_ast_extraktion_findet_wirklich_gruende: eine
    leere oder winzige Quelle wuerde jede Pruefung unten sang- und klanglos gruen machen."""
    assert len(BD.SPERRGRUND_KLARTEXT) >= 30, (
        f"nur {len(BD.SPERRGRUND_KLARTEXT)} Eintraege in SPERRGRUND_KLARTEXT -- "
        "ist das noch dieselbe Konstante?")


def test_kette_wird_ueberhaupt_gefunden():
    """Ohne diesen Waechter waere eine kaputte Regex (0 Treffer) still gruen fuer
    test_kein_sperrgrund_faellt_auf_die_rohe_feldliste_durch, weil `any([])` False ist --
    also strukturell dasselbe Ergebnis wie eine ECHTE fehlende klartext-Pruefung."""
    kette = _branch_kette(_guard_block(_app_js_text()))
    assert len(kette) >= 3, (
        f"nur {len(kette)} Zweige in der zeigeErgebnis()-Guard-Kette gefunden -- "
        "die Regex zur Zweig-Extraktion greift nicht mehr (umgebaut?).")


# ---------------------------------------------------------------- (a) + (b)

@pytest.mark.xfail(
    strict=True,
    reason="backlog/taxgraph/sperrgruende-erreichen-den-nutzer-nicht.md: app.js liest "
           "'klartext' nirgends -- Marker faellt am Tag des Fixes (XPASS) und zwingt dazu, "
           "ihn zu entfernen.")
def test_klartext_wird_ueberhaupt_gelesen():
    """(a) aus dem Auftrag: kommt 'klartext' in app.js ueberhaupt vor.

    Schwach fuer sich allein (ein toter Verweis wuerde reichen) -- deshalb nur die halbe
    Bedingung, ergaenzt durch die strukturelle Pruefung unten."""
    text = _app_js_text()
    assert "klartext" in text, (
        "app.js liest 'klartext' nirgends -- jeder der 39 Backend-Erklaertexte "
        "(SPERRGRUND_KLARTEXT) bleibt wirkungslos, egal wie gut formuliert.")


@pytest.mark.xfail(
    strict=True,
    reason="backlog/taxgraph/sperrgruende-erreichen-den-nutzer-nicht.md: die rohe Feldliste "
           "ist nicht hinter einer klartext-Pruefung versteckt -- Marker faellt am Tag des "
           "Fixes (XPASS) und zwingt dazu, ihn zu entfernen.")
def test_kein_sperrgrund_faellt_auf_die_rohe_feldliste_durch():
    """(b) aus dem Auftrag: kein Grund aus SPERRGRUND_KLARTEXT faellt auf den rohen
    Feldlisten-Zweig durch -- geprueft ueber die Zweig-REIHENFOLGE, nicht ueber eine
    Namensliste (Begruendung: Moduldoc, Abschnitt 'Warum keine Namensliste...')."""
    text = _app_js_text()
    kette = _branch_kette(_guard_block(text))
    feldlisten_index = next(
        (i for i, (_, _, ausdruck) in enumerate(kette) if "Noch offen" in ausdruck), None)
    assert feldlisten_index is not None, (
        "die rohe Feldliste (\"Noch offen: \" + ...) ist aus der Guard-Kette verschwunden -- "
        "dieses Gate hat dann nichts mehr zu pruefen und muss nachgezogen werden.")

    klartext_prueft_davor = any(
        "klartext" in (bedingung or "") for _, bedingung, _ in kette[:feldlisten_index])

    if not klartext_prueft_davor:
        durchfaller = sorted(_heutige_durchfaller(text))
        raise AssertionError(
            "Die rohe Feldliste ist nicht hinter einer klartext-Pruefung versteckt -- sie "
            "wird erreicht, sobald ein Grund weder in GUARD noch einer der beiden "
            "else-if-Sonderfaelle steht. Heutige Zaehlung nach der Vault-Methodik (Diagnose, "
            f"nicht das Kriterium selbst): {len(durchfaller)} von {len(BD.SPERRGRUND_KLARTEXT)} "
            "Sperrgruenden fallen durch: " + ", ".join(durchfaller))
