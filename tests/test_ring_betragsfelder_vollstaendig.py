"""RING_BETRAGSFELDER (api_constants.py) darf nicht still veralten.

Die Menge beantwortet "welche flachen Betragsfelder kann der Ring ueberhaupt lesen" und traegt
seit dem Klasse-C-Fix (Vault backlog/taxgraph/klasse-c-vorlaeufiges-einkommen-faellt-still-aus.md)
die Sperre gegen ein stilles Herausfallen vorlaeufiger Betraege. Sie ist handgepflegt -- ein
Laufzeit-AST-Scan gehoert nicht in den Produktcode.

Dieser Test leitet sie deshalb bei JEDEM Lauf neu aus dem Quelltext ab und vergleicht. Bauart wie
tests/test_slot_fn_reader_existiert.py::GELESENE_SLOT_NAMEN_JE_QUANTITAET: die Handliste bleibt,
aber sie kann nicht mehr unbemerkt von der Wirklichkeit abweichen.

Herleitung (identisch zum Kommentar an der Konstante):
  (a) String-Literal in einem der drei Ring-Module, ODER
  (b) ueber einen Namen erreichbar, den das Ring-Modul aus api_constants zieht und der an einen
      String oder ein Tupel von Strings gebunden ist.

Beide Richtungen werden geprueft: fehlt ein Feld (Ring liest etwas, das nicht in der Menge steht)
ist die Sperre lueckenhaft; steht eines zu viel drin, sperrt sie Faelle, die gar nicht betroffen
sind -- und genau das war die Auflage "Was ist verschwunden?".

NULL LLM.
"""
from __future__ import annotations

import ast
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/haut", "produkt/traverser"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import api_constants as AC              # noqa: E402
import traverser as TR                  # noqa: E402
from api_constants import RING_BETRAGSFELDER, SCHEIBEN  # noqa: E402

RING_MODULE = ("bescheid_zweige.py", "bescheid_einkuenfte.py", "bescheid_abzuege.py")


def _ring_namen_und_literale() -> tuple[set, set]:
    """(String-Literale ohne Docstrings, Namen) aus den drei Ring-Modulen."""
    literale, namen = set(), set()
    for datei in RING_MODULE:
        pfad = os.path.join(ROOT, "produkt", "bescheid", datei)
        baum = ast.parse(open(pfad, encoding="utf-8").read(), filename=pfad)
        docstrings = set()
        for n in ast.walk(baum):
            if isinstance(n, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef, ast.Module)) \
                    and n.body and isinstance(n.body[0], ast.Expr) \
                    and isinstance(n.body[0].value, ast.Constant):
                docstrings.add(id(n.body[0].value))
        for n in ast.walk(baum):
            if isinstance(n, ast.Constant) and isinstance(n.value, str) and id(n) not in docstrings:
                literale.add(n.value)
            elif isinstance(n, ast.Name):
                namen.add(n.id)
    return literale, namen


def _ueber_konstanten(namen: set) -> set:
    """Die Feldnamen, die diese Namen in api_constants erreichen -- String ODER Tupel von Strings."""
    treffer = set()
    for nm in namen:
        v = getattr(AC, nm, None)
        if isinstance(v, str):
            treffer.add(v)
        elif isinstance(v, (tuple, list, frozenset, set)) and all(isinstance(x, str) for x in v):
            treffer |= set(v)
    return treffer


def _numerisch_und_flach(fid: str, bindung: dict) -> bool:
    b = bindung.get(fid) or {}
    return b.get("typ") in ("cent", "int") and not b.get("instanz_gruppe")


def _abgeleitet() -> set:
    literale, namen = _ring_namen_und_literale()
    erreichbar = literale | _ueber_konstanten(namen)
    felder = set()
    for cfg in SCHEIBEN.values():
        if cfg.get("guard"):
            felder |= set(cfg["felder"] or ())
    bindung = TR.lade_bindung()
    return {f for f in felder if _numerisch_und_flach(f, bindung) and f in erreichbar}


def test_konstante_ist_vollstaendig():
    """Kein Feld fehlt: sonst faellt genau dort ein vorlaeufiger Betrag weiter still heraus."""
    fehlend = sorted(_abgeleitet() - set(RING_BETRAGSFELDER))
    assert not fehlend, (
        f"Der Ring liest {fehlend}, aber RING_BETRAGSFELDER kennt sie nicht -- ein vorlaeufiger "
        "Wert in einem dieser Felder faellt weiter still aus der Zahl. Konstante in "
        "produkt/haut/api_constants.py nachziehen.")


def test_konstante_hat_keine_karteileichen():
    """Kein Feld zu viel: sonst sperrt die Sperre Faelle, die der Ring gar nicht liest."""
    zuviel = sorted(set(RING_BETRAGSFELDER) - _abgeleitet())
    assert not zuviel, (
        f"RING_BETRAGSFELDER nennt {zuviel}, aber der Ring erreicht sie nicht -- diese Felder "
        "wuerden einen Fall sperren, der nie betroffen war.")


def test_abgeleitete_menge_ist_nicht_leer():
    """Blindheitswaechter: eine kaputte Ableitung (Import verschoben, AST-Muster geaendert) waere
    sonst still gruen in beiden Vergleichen oben -- zwei leere Mengen sind gleich."""
    abgeleitet = _abgeleitet()
    assert len(abgeleitet) > 50, f"Nur {len(abgeleitet)} Felder abgeleitet -- die Extraktion ist kaputt."


def test_das_geprobte_feld_ist_enthalten():
    """Der Anlass der Sperre muss in der Menge stehen. Ohne diesen Fall koennte die Ableitung
    vollstaendig sein und das Ticket-Feld trotzdem fehlen (falsche Scheiben-Auswahl o.ae.)."""
    assert "rentner_veraeusserungsgewinn" in RING_BETRAGSFELDER
    assert "einkuenfte_gewinn" in RING_BETRAGSFELDER
