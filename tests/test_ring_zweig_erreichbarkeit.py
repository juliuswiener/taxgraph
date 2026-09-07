"""Erreichbarkeits-Gate — Backlog ring-eintrag-ohne-zweig-ungegatet.md, Entscheidung
zwei-ratschen-bei-null-angefangen.md (Julius 2026-08-26).

Jeder gesamt_ring/teil_ringe-Eintrag in api_constants.SCHEIBEN benennt eine "quantitaet",
die _bescheid_fn (produkt/bescheid/bescheid_zweige.py) per String-Vergleich an einen
_zweig_*-Rechenweg weiterreicht. Ohne passenden Zweig fällt der Aufruf lautlos auf das
abschließende `return None` ("kein exponierter Accessor") — genau die Fehlerklasse, die
den toten Teil-Ring am AfA-Betrag erzeugt hat (s. api_constants.py, n_vor_gwg-Kommentar),
nur von Hand gefunden.

MIT Ausnahmeliste (anders als das Marker-Gate, s. test_mutation_marker_gate.py): der
dokumentierte AfA-Fall zeigt, dass ein Ring-Eintrag ohne Zweig ein bewusster
ENTSCHEIDUNGSPUNKT ist (entfernen oder Zweig bauen), kein automatischer Fehler — die
naheliegende Reparatur (einen Zweig bauen) war dort die falsche, weil sie eine zweite
Rechenquelle für dieselbe AfA geschaffen hätte. Die Liste ist für das, was jemand bewusst
geparkt hat, und darf NUR schrumpfen.
"""
from __future__ import annotations

import inspect
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/haut", "produkt/store", "produkt/traverser", "produkt/unsicherheit",
             "produkt/mapping", "produkt/konsistenz", "produkt/eingang", "produkt/bescheid",
             "produkt/engine", "golden", "elster"):
    _p = os.path.join(ROOT, _sub)
    if _p not in sys.path:
        sys.path.insert(0, _p)

import api_constants as AC   # noqa: E402
import bescheid_zweige as BZ  # noqa: E402

# Bewusst geparkte Ring-Eintraege ohne Zweig, je (scheibe, bezeichner). Heute LEER —
# nachgemessen 2026-08-26/2026-09-07: alle fuenf ring-tragenden SCHEIBEN-Eintraege
# (vier gesamt_ring + der eine teil_ringe-Eintrag) haben einen Zweig in _bescheid_fn.
# Darf NUR schrumpfen. Ein neuer Eintrag hier braucht dieselbe Sorgfalt wie beim
# dokumentierten AfA-Fall: erst pruefen, ob Entfernen statt Parken die richtige Antwort
# ist, dann erst als Ausnahme eintragen.
RING_OHNE_ZWEIG_AUSNAHMEN: set[tuple[str, str]] = set()


def _erkannte_zweig_namen() -> set[str]:
    """Alle `quantitaet == "..."`-Literale, die _bescheid_fn an einen Zweig weiterreicht."""
    quelle = inspect.getsource(BZ._bescheid_fn)
    return set(re.findall(r'quantitaet == "([^"]+)"', quelle))


def _ring_eintraege() -> list[tuple[str, str, str]]:
    """(scheibe, bezeichner, ring_name) fuer jeden benannten Ring in SCHEIBEN."""
    eintraege = []
    for scheibe, cfg in AC.SCHEIBEN.items():
        ring = cfg.get("gesamt_ring")
        if ring is not None:
            eintraege.append((scheibe, "gesamt_ring", ring))
        for label, teil_ring, _felder in cfg.get("teil_ringe", []):
            eintraege.append((scheibe, f"teil_ring:{label}", teil_ring))
    return eintraege


def test_jeder_ring_eintrag_hat_einen_zweig():
    """Jeder Ring-Eintrag ohne Zweig muss entweder einen _zweig_*-Pfad in _bescheid_fn
    haben oder bewusst in RING_OHNE_ZWEIG_AUSNAHMEN stehen."""
    erkannt = _erkannte_zweig_namen()
    fehlend = [(s, b, r) for s, b, r in _ring_eintraege()
               if r not in erkannt and (s, b) not in RING_OHNE_ZWEIG_AUSNAHMEN]
    assert not fehlend, (
        "Ring-Eintrag ohne Zweig und ohne Ausnahme — Entscheidungspunkt wie beim "
        "dokumentierten AfA-Fall (entfernen oder Zweig bauen), nicht stillschweigend "
        "lassen:\n  " + "\n  ".join(f"{s}.{b} -> {r!r}" for s, b, r in fehlend))


def test_ausnahmeliste_darf_nicht_veraltet_sein():
    """Jede Ausnahme muss (a) noch zu einem echten SCHEIBEN-Eintrag gehören und (b)
    tatsächlich weiterhin keinen Zweig haben — sonst prüft die Liste nichts mehr und
    MUSS schrumpfen (Julius-Entscheid: nur schrumpfen, nie als Dauerzustand stehen bleiben)."""
    erkannt = _erkannte_zweig_namen()
    vorhanden = {(s, b): r for s, b, r in _ring_eintraege()}
    for schluessel in RING_OHNE_ZWEIG_AUSNAHMEN:
        assert schluessel in vorhanden, (
            f"Ausnahme {schluessel} zeigt auf keinen SCHEIBEN-Eintrag mehr — streichen.")
        assert vorhanden[schluessel] not in erkannt, (
            f"Ausnahme {schluessel} hat inzwischen einen Zweig ({vorhanden[schluessel]!r}) "
            "— aus RING_OHNE_ZWEIG_AUSNAHMEN streichen, sonst prüft die Liste nichts.")
