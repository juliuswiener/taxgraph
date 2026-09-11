"""Leere ``offen``-Listen duerfen im Browser keinen leeren Satz erzeugen.

Anlass 2026-08-28/29, Vault-Funde zum selben Defekt:
  - backlog/taxgraph/guard-sperrgruende-leerer-satz-im-browser.md
  - backlog/taxgraph/sperrgruende-erreichen-den-nutzer-nicht.md

produkt/haut/api.py::_ergebnis_roh() hat vier return-Zweige. In zwei davon ist der Wert von
"offen" bewusst ein woertliches `[]` -- dort gibt es keine fehlende Eingabe aufzulisten:
  - dem Guard-Zweig (K2: `sperr = _an_gesamt_sperrgrund(...)`, dann `if sperr: return ...`)
  - dem "kein_scheiben_gesamtbescheid"-Zweig (Multi-Regel-Scheibe ohne Gesamt-Accessor)
Der Browser muss deshalb den vom Backend gelieferten ``klartext`` vor dem generischen
"Noch offen"-Fallback anzeigen. Genau diese Reihenfolge prueft
``test_sperrgrund_klartext_im_browser.py``; diese Datei haelt nur noch die vier
Rueckgabeformen als Blindheitswaechter fest.

Die zwei uebrigen Zweige haben dasselbe Feld, aber nicht hartkodiert:
  - der "engine_unavailable"/"input_kegel_nicht_bestaetigt"-Zweig: `"offen": sorted(offen)`
  - der Erfolgs-Zweig ("grund": "bestaetigt"): `"offen": offen_c`
Diese zwei dienen als Positivbeleg, dass die AST-Auswertung zwischen echten und leeren Listen
unterscheidet.

Bauart: Muster, nicht Zeilennummer
-----------------------------------
Instruktion: "Zeilennummern koennen gewandert sein -- such nach dem Muster, nicht nach der
Zeile." Deshalb AST-Extraktion von _ergebnis_roh() aus dem Quelltext (wie
tests/test_sperrgrund_klartext.py es fuer SPERRGRUND_KLARTEXT tut) statt Regex auf feste
Zeilen: jeder der vier return-Zweige wird ueber den INHALT seines "grund"-Werts identifiziert
(Konstante "bestaetigt"/"kein_scheiben_gesamtbescheid", oder eine Variable, deren Zuweisung
per Musterabgleich zurueckverfolgt wird -- ein Aufruf von `_an_gesamt_sperrgrund` bzw. ein
ternaerer Ausdruck). Verschiebt sich die Zeile, findet dieser Test den Zweig trotzdem.
`_ergebnis_roh` selbst wird nie IMPORTIERT/ausgefuehrt (kein Catala-Laufzeit-Bedarf) --
nur ihr Quelltext geparst.

Die Anzeige-Reihenfolge ist der scharfe Vertrag
-----------------------------------------------
Eine leere Liste ist fuer eine nicht unterstuetzte Konstellation korrekt; falsch war nur,
daraus im Browser einen leeren Satz zu bilden. Der scharfe Vertrag lebt deshalb beim
Renderer, nicht als Verbot einer leeren Backend-Liste.
"""

from __future__ import annotations

import ast
import os
import pathlib

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
API_PFAD = pathlib.Path(ROOT) / "produkt" / "haut" / "api.py"


def _api_source() -> str:
    return API_PFAD.read_text(encoding="utf-8")


def _ergebnis_roh_fn(baum: ast.Module) -> ast.FunctionDef:
    for n in ast.walk(baum):
        if isinstance(n, ast.FunctionDef) and n.name == "_ergebnis_roh":
            return n
    raise AssertionError(
        "_ergebnis_roh ist aus produkt/haut/api.py verschwunden (umbenannt/entfernt?) -- "
        "der Anker fuer dieses Gate fehlt, es muss nachgezogen werden.")


def _dict_literal(ret: ast.Return) -> ast.Dict:
    wert = ret.value
    kandidaten = wert.elts if isinstance(wert, ast.Tuple) else [wert]
    for el in kandidaten:
        if isinstance(el, ast.Dict):
            return el
    raise AssertionError(
        f"return in api.py Zeile {ret.lineno} liefert kein Dict-Literal mehr -- "
        "Rueckgabeform von _ergebnis_roh hat sich geaendert.")


def _wert_fuer(dict_node: ast.Dict, schluessel: str) -> ast.AST | None:
    for k, v in zip(dict_node.keys, dict_node.values):
        if isinstance(k, ast.Constant) and k.value == schluessel:
            return v
    return None


def _ist_leere_listen_konstante(node: ast.AST | None) -> bool:
    """True nur fuer ein woertliches `[]` -- eine berechnete leere Liste (z.B. `sorted([])`
    zur Laufzeit) ist syntaktisch etwas anderes und faellt NICHT hierunter; genau das ist
    der Unterschied zwischen den beiden Bug-Zweigen und den zwei Kontrollzweigen."""
    return isinstance(node, ast.List) and not node.elts


def _zuweisungswert(fn: ast.FunctionDef, name: str) -> ast.AST | None:
    """Der Ausdruck, dem `name` innerhalb der Funktion zugewiesen wird -- Musterabgleich
    statt Zeilenanker: findet, WAS eine Variable enthaelt, unabhaengig davon, wo im
    Funktionskoerper das steht."""
    for n in ast.walk(fn):
        if isinstance(n, ast.Assign) and any(
                isinstance(t, ast.Name) and t.id == name for t in n.targets):
            return n.value
    return None


def _ist_aufruf_von(node: ast.AST | None, funktionsname: str) -> bool:
    return (isinstance(node, ast.Call) and isinstance(node.func, ast.Name)
            and node.func.id == funktionsname)


def _branch_name(fn: ast.FunctionDef, grund_node: ast.AST | None) -> str:
    """Identifiziert einen der vier return-Zweige ueber den INHALT von "grund", nicht ueber
    seine Position im Quelltext."""
    if grund_node is None:
        return "kein_grund_schluessel"
    if isinstance(grund_node, ast.Constant) and grund_node.value == "bestaetigt":
        return "bestaetigt_erfolg"
    if isinstance(grund_node, ast.Constant) and grund_node.value == "kein_scheiben_gesamtbescheid":
        return "kein_scheiben_gesamtbescheid"
    if isinstance(grund_node, ast.Name):
        quelle = _zuweisungswert(fn, grund_node.id)
        if _ist_aufruf_von(quelle, "_an_gesamt_sperrgrund"):
            return "guard_sperrgrund"
        if isinstance(quelle, ast.IfExp):
            return "engine_oder_kegel_offen"
    return "unbekannt"


def _alle_zweige() -> dict[str, tuple[ast.AST | None, int]]:
    """Name -> (offen-Wert-Knoten, Zeilennummer NUR zur Fehlermeldung, nicht als Anker)."""
    baum = ast.parse(_api_source(), filename=str(API_PFAD))
    fn = _ergebnis_roh_fn(baum)
    returns = [n for n in ast.walk(fn) if isinstance(n, ast.Return)]
    ergebnis: dict[str, tuple[ast.AST | None, int]] = {}
    for ret in returns:
        d = _dict_literal(ret)
        grund_node = _wert_fuer(d, "grund")
        offen_node = _wert_fuer(d, "offen")
        name = _branch_name(fn, grund_node)
        ergebnis[name] = (offen_node, ret.lineno)
    return ergebnis


# ---------------------------------------------------------------- Blindheits-Waechter

def test_alle_vier_rueckgabe_zweige_werden_gefunden():
    """Ohne diesen Waechter waere eine kaputte AST-Extraktion (0 oder 1 statt 4 Zweige) still
    gruen fuer die Kontrollen unten -- ein Zweig, der nicht gefunden wird, kann auch nicht
    auf seine Rueckgabeform geprueft werden."""
    zweige = _alle_zweige()
    erwartet = {"guard_sperrgrund", "kein_scheiben_gesamtbescheid",
                "engine_oder_kegel_offen", "bestaetigt_erfolg"}
    fehlend = erwartet - set(zweige)
    assert not fehlend, (
        f"_ergebnis_roh in api.py wurde umgebaut -- Zweige {sorted(fehlend)} sind ueber das "
        f"Muster nicht mehr auffindbar (gefunden: {sorted(zweige)}). Dieses Gate muss "
        "nachgezogen werden.")


@pytest.mark.parametrize("zweig", ["engine_oder_kegel_offen", "bestaetigt_erfolg"])
def test_kontrollzweige_liefern_bereits_eine_echte_liste(zweig):
    """Positivbeleg: diese zwei Zweige haben den Fehler NICHT. Waeren sie es auch, waere
    macht sichtbar, dass das Muster echte Unterscheidungskraft hat."""
    zweige = _alle_zweige()
    offen_node, lineno = zweige[zweig]
    assert not _ist_leere_listen_konstante(offen_node), (
        f"[{zweig}] api.py Zeile {lineno}: unerwartet ein woertliches [] -- dieser Zweig galt "
        "bislang als Kontrolle (nicht hartkodiert). Entweder eine echte Regression, oder "
        "dieser Test muss neu bewertet werden.")
