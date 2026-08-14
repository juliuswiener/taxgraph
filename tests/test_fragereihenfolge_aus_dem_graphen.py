"""Die Fragereihenfolge folgt dem Graphen, nicht dem Alphabet.

Befund 2026-08-14 (Julius beim ersten Durchklicken): "die fragen sind in einer random reihenfolge".
Gemessen war sie nicht random, sondern alphabetisch nach feld_id — was aus Nutzersicht dasselbe
ist. Die Folgen in Zahlen, Scheibe gesamt, leerer Fall, 243 offene Fragen:

    veranlagung        Position 203     entscheidet über 38 Partner-Felder
    bruttoarbeitslohn  Position  97
    Screening § 35a    Position  24

Julius' Einwand dazu: "das müsste sich ja aus dem graph ergeben". Genau so — es braucht keine
handkuratierte Liste. Wie viele Fragen eine Antwort erspart, steht bereits in der Bindung:

    (a) bool-Gate seiner Regel      -> alle übrigen askable Felder derselben Regel
    (b) Ob-Bedingung in regel_bedingungen -> alle askable Felder der fremden Regel

traverser.gate_gewicht() rechnet das aus, naechste_fragen() sortiert danach. Der Nebeneffekt ist
der eigentliche Gewinn: jeder neue regel_bedingungen-Eintrag (Screening-Modell) hebt sein Gate
automatisch nach vorn, ohne dass hier jemand etwas nachträgt.

NULL LLM.
"""
from __future__ import annotations

import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/haut", "produkt/store", "produkt/traverser", "produkt/mapping",
            "produkt/unsicherheit", "golden"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import api as API          # noqa: E402
import store as ST         # noqa: E402
import traverser as TR     # noqa: E402


def _leerer_fall(scheibe="gesamt"):
    store = ST.leerer_store(2025, fall_id="reihenfolge")
    store["scheibe"] = scheibe
    return store, API._scheibe_bindung(store)


def test_veranlagung_kommt_zuerst():
    """Die Veranlagungsart entscheidet über ~38 Partner-Felder und stand auf Frage 203 von 243.
    Sie ist der Extremfall, an dem der Fehler zuerst auffiel — und der Grund, warum ein Feld OHNE
    geltungsbedingung trotzdem als Gate behandelt werden muss: veranlagung wirkt über
    regel_bedingungen, nicht über ein eigenes bool-Gate."""
    store, bindung = _leerer_fall()
    fragen = TR.naechste_fragen(store, bindung)
    assert "veranlagung" in fragen, "veranlagung wird gar nicht gefragt."
    pos = fragen.index("veranlagung") + 1
    assert pos <= 3, (
        f"veranlagung steht auf Position {pos} von {len(fragen)} — sie schaltet mehr Felder ab "
        f"als jede andere Frage und gehört an den Anfang.")


def test_gates_stehen_nach_gewicht_absteigend():
    """Kernsortierung: Fragen, die viel abschalten, zuerst. Ohne diesen Test wäre eine Rückkehr
    zur alphabetischen Ordnung unbemerkt möglich — sie sieht in keiner Rechnung anders aus."""
    store, bindung = _leerer_fall()
    fragen = TR.naechste_fragen(store, bindung)
    gw = TR.gate_gewicht(bindung)
    mit_gewicht = [(f, gw.get(f, 0)) for f in fragen if gw.get(f, 0) > 0]
    assert mit_gewicht, "Kein einziges Feld hat Gewicht — gate_gewicht() misst nichts."
    gewichte = [g for _f, g in mit_gewicht]
    assert gewichte == sorted(gewichte, reverse=True), (
        f"Gates stehen nicht absteigend nach Gewicht: {mit_gewicht[:8]}")
    # und sie stehen VOR den gewichtslosen Feldern
    erstes_ohne = next((i for i, f in enumerate(fragen) if gw.get(f, 0) == 0), len(fragen))
    letztes_mit = max(i for i, f in enumerate(fragen) if gw.get(f, 0) > 0)
    assert letztes_mit < erstes_ohne, (
        "Ein Feld ohne Gewicht steht vor einem Feld mit Gewicht — die Queue mischt die Ebenen.")


def test_gewicht_folgt_der_bindung_nicht_einer_liste():
    """Gegen die naheliegende Abkürzung: eine handgepflegte Prioritätsliste im Code. Sie würde
    bei jedem neuen Feld veralten und wäre nicht überprüfbar. Das Gewicht MUSS aus der Bindung
    fallen — hier nachgerechnet für einen Fall aus jeder der beiden Quellen."""
    store, bindung = _leerer_fall()
    gw = TR.gate_gewicht(bindung)

    # (b) regel_bedingungen: veranlagung steuert p2_festzusetzung_zusammen
    bedingungen = TR.lade_regel_bedingungen()
    ziel = [rid for rid, conds in bedingungen.items()
            if any(c["feld"] == "veranlagung" for c in conds)]
    assert ziel, "veranlagung steht in keiner regel_bedingung — der Test misst nichts."
    erwartet = sum(1 for f, b in bindung.items()
                   if b.get("askable") and b["quelle"]["regel_id"] in ziel)
    assert gw["veranlagung"] >= erwartet, (
        f"veranlagung-Gewicht {gw['veranlagung']} < {erwartet} askable Felder in {ziel}")

    # (a) eigenes bool-Gate: hh_hat_aufwendungen streicht die übrigen § 35a-Felder
    mit = [f for f, b in bindung.items()
           if b.get("askable") and b["quelle"]["regel_id"] == "p35a_2_3_haushaltsnahe"]
    assert gw.get("hh_hat_aufwendungen", 0) == len(mit) - 1, (
        f"hh_hat_aufwendungen-Gewicht {gw.get('hh_hat_aufwendungen')} passt nicht zu "
        f"{len(mit)} Feldern der Regel.")


@pytest.mark.parametrize("scheibe", ["gesamt", "an_gesamt", "rentner_gesamt"])
def test_reihenfolge_ist_deterministisch(scheibe):
    """Zwei Aufrufe, dieselbe Liste. Bei Gleichstand entscheidet der Feldname — sonst wanderten
    Fragen zwischen zwei Ladevorgängen, was schlimmer wäre als jede feste Reihenfolge."""
    store, bindung = _leerer_fall(scheibe)
    assert TR.naechste_fragen(store, bindung) == TR.naechste_fragen(store, bindung)
