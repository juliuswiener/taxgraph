"""_ergebnis_roh(): "offen" darf keine fest verdrahtete leere Liste sein.

Anlass 2026-08-28/29, Vault-Funde zum selben Defekt:
  - backlog/taxgraph/guard-sperrgruende-leerer-satz-im-browser.md
  - backlog/taxgraph/sperrgruende-erreichen-den-nutzer-nicht.md

produkt/haut/api.py::_ergebnis_roh() hat vier return-Zweige. In ZWEI davon ist der Wert von
"offen" ein woertliches `[]` -- unabhaengig vom tatsaechlichen Sperrgrund/Zustand:
  - dem Guard-Zweig (K2: `sperr = _an_gesamt_sperrgrund(...)`, dann `if sperr: return ...`)
  - dem "kein_scheiben_gesamtbescheid"-Zweig (Multi-Regel-Scheibe ohne Gesamt-Accessor)
Im Browser (app.js::zeigeErgebnis) wird daraus fuer jeden Grund ohne eigenen GUARD-Eintrag
woertlich "Noch offen: " ohne jeden Inhalt danach -- gemessen: 22 von 36 Faellen.

Die zwei UEBRIGEN Zweige haben dasselbe Feld, aber NICHT hartkodiert:
  - der "engine_unavailable"/"input_kegel_nicht_bestaetigt"-Zweig: `"offen": sorted(offen)`
  - der Erfolgs-Zweig ("grund": "bestaetigt"): `"offen": offen_c`
Diese zwei dienen hier als KONTROLLE (Positivbeleg): waeren sie auch hartkodiert, waere
dieser Test aus Zufall rot, nicht aus Befund (s. test_kontrollzweige_liefern_bereits_eine_echte_liste).

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

Fail-closed, kein Toleranzwert
-------------------------------
Jeder der beiden bekannten Fundorte ist ein eigener parametrisierter Fall
(test_offen_ist_nicht_hartkodiert_leer[guard_sperrgrund] /
[kein_scheiben_gesamtbescheid]) -- eine "reicht mehrheitlich"-Schwelle wuerde genau die
Regression verdecken, die dieser Test fangen soll (wie
tests/test_sechs_abgabe_luecken_blockmatrix.py es fuer ERiC-Rueckweisungen vormacht).
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
    gruen fuer die beiden Bug-Faelle unten -- ein Zweig, der nicht GEFUNDEN wird, kann auch
    nicht als 'hartkodiert leer' auffallen."""
    zweige = _alle_zweige()
    erwartet = {"guard_sperrgrund", "kein_scheiben_gesamtbescheid",
                "engine_oder_kegel_offen", "bestaetigt_erfolg"}
    fehlend = erwartet - set(zweige)
    assert not fehlend, (
        f"_ergebnis_roh in api.py wurde umgebaut -- Zweige {sorted(fehlend)} sind ueber das "
        f"Muster nicht mehr auffindbar (gefunden: {sorted(zweige)}). Dieses Gate muss "
        "nachgezogen werden.")


# ---------------------------------------------------------------- die zwei Fundstellen

@pytest.mark.xfail(
    strict=True,
    reason="backlog/taxgraph/guard-sperrgruende-leerer-satz-im-browser.md: 'offen' ist in "
           "beiden Backend-Zweigen fest verdrahtet leer -- Marker faellt am Tag des Fixes "
           "(XPASS) und zwingt dazu, ihn zu entfernen.")
@pytest.mark.parametrize("zweig", ["guard_sperrgrund", "kein_scheiben_gesamtbescheid"])
def test_offen_ist_nicht_hartkodiert_leer(zweig):
    """Fail-closed, je Fundstelle ein eigener Fall -- keine Quote. In BEIDEN Zweigen liefert
    _ergebnis_roh() heute "offen": [] woertlich, unabhaengig vom tatsaechlichen Sperrgrund
    oder Zustand. Kontrolle dazu: test_kontrollzweige_liefern_bereits_eine_echte_liste."""
    zweige = _alle_zweige()
    offen_node, lineno = zweige[zweig]
    assert not _ist_leere_listen_konstante(offen_node), (
        f"[{zweig}] api.py Zeile {lineno}: 'offen' ist dort ein woertliches [] -- fest "
        "verdrahtet, unabhaengig vom tatsaechlichen Sperrgrund. Im Browser wird daraus fuer "
        "jeden Grund ohne eigenen GUARD-Eintrag 'Noch offen: ' ohne jeden Inhalt danach.")


@pytest.mark.parametrize("zweig", ["engine_oder_kegel_offen", "bestaetigt_erfolg"])
def test_kontrollzweige_liefern_bereits_eine_echte_liste(zweig):
    """Positivbeleg: diese zwei Zweige haben den Fehler NICHT. Waeren sie es auch, waere
    test_offen_ist_nicht_hartkodiert_leer oben zufaellig rot, nicht aus Befund -- dieser Test
    macht sichtbar, dass das Muster echte Unterscheidungskraft hat (s. Mutationsprobe)."""
    zweige = _alle_zweige()
    offen_node, lineno = zweige[zweig]
    assert not _ist_leere_listen_konstante(offen_node), (
        f"[{zweig}] api.py Zeile {lineno}: unerwartet ein woertliches [] -- dieser Zweig galt "
        "bislang als Kontrolle (nicht hartkodiert). Entweder eine echte Regression, oder "
        "dieser Test muss neu bewertet werden.")
