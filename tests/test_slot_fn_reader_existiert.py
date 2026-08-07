"""Gate gegen fail-open signatur_slot-Umbenennung (BACKLOG slot-fail-open-13568-eur).

Befund (Report reports/adjudikation/p2_festzusetzung_slot_verstoesse_2026-08-08.md, Mutationsprobe
2026-08-08): `bescheid_via_slots()` liest per `slots.get(name, default)` bzw. `slots[name]` — ein
falsch benannter/umbenannter `signatur_slot` in produkt/bindung/*.yaml wirft KEIN KeyError, sondern
liefert still den Default (0 / "einzel"). Gemessen: bindung_an_gesamt.yaml:15 signatur_slot
bruttoarbeitslohn -> bruttoarbeitslohn_x lässt die festgesetzte Steuer OHNE Fehler von 1.356.800 ct
auf 0 ct fallen, `grund` bleibt "bestaetigt". test_bindungstabelle.py sieht das nicht (p2_festzusetzung_*
steht in REGELN_OHNE_GROUND_TRUTH, s. dort) — nur ein zufälliger Cent-Wert-Assert in
test_paket_b_e2e_http.py fängt es.

Dies ist Variante (b) aus dem BACKLOG-Punkt: das Gate, nicht der Produktfix (Variante (a),
.get() auf [] umstellen, braucht erst eine Messung, welche Aufrufer den Slot legitim weglassen
dürfen — nicht hier).

Baumuster wie REGELN_OHNE_GROUND_TRUTH in test_bindungstabelle.py: eine explizite Konstante der
heute bekannten Leser PLUS ein Gegen-Assert, der per AST prüft, dass api.py keinen weiteren Leser
hat, der nicht in der Konstante steht. Ein reiner AST-Leser ohne Gegen-Assert wäre selbst
unverifizierter Code — die Konstante ist die Ground Truth, der AST-Scan hält sie ehrlich.
"""
from __future__ import annotations

import ast
import glob
import os

import pytest

yaml = pytest.importorskip("yaml")

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
BIND_DIR = os.path.join(ROOT, "produkt", "bindung")
API_PY = os.path.join(ROOT, "produkt", "haut", "api.py")

# Heute (2026-08-08) tatsächlich per slots.get(...)/slots[...] gelesene Slot-Namen in
# produkt/haut/api.py — gemessen per AST (s. test_z_ast_scan_ist_vollstaendig unten), nicht geraten.
# Wächst NUR bewusst: ein neuer Leser in api.py muss hier eingetragen werden, sonst schlägt
# test_z_ast_scan_ist_vollstaendig fehl.
GELESENE_SLOT_NAMEN = {
    "arbeitstage",
    "entfernung_km_roh",
    "oepnv_kosten_jahr",
    "eigenes_oder_ueberlassenes_kfz",
    "bruttoarbeitslohn",
    "veranlagung",
}


def _slot_reader_namen_per_ast(path: str) -> set[str]:
    """Sammelt alle String-Literale, mit denen api.py den `slots`-Parameter einer slot_fn liest:
    slots.get("X", ...), slots["X"], und {k: slots[k] for k in ("X", "Y", ...)}."""
    tree = ast.parse(open(path, encoding="utf-8").read())
    namen: set[str] = set()
    for node in ast.walk(tree):
        if (isinstance(node, ast.Call) and isinstance(node.func, ast.Attribute)
                and node.func.attr == "get" and isinstance(node.func.value, ast.Name)
                and node.func.value.id == "slots" and node.args
                and isinstance(node.args[0], ast.Constant) and isinstance(node.args[0].value, str)):
            namen.add(node.args[0].value)
        if (isinstance(node, ast.Subscript) and isinstance(node.value, ast.Name)
                and node.value.id == "slots"):
            sl = node.slice
            if isinstance(sl, ast.Constant) and isinstance(sl.value, str):
                namen.add(sl.value)
        if isinstance(node, ast.DictComp):
            # Muster: {k: slots[k] for k in ("arbeitstage", "entfernung_km_roh", ...) if k in slots}
            for gen in node.generators:
                if isinstance(gen.iter, (ast.Tuple, ast.List)):
                    for elt in gen.iter.elts:
                        if isinstance(elt, ast.Constant) and isinstance(elt.value, str):
                            namen.add(elt.value)
    return namen


def _alle_signatur_slots() -> set[str]:
    """Alle signatur_slot-Werte aus allen produkt/bindung/bindung_*.yaml (bindungen + luecken)."""
    namen: set[str] = set()
    for fp in sorted(glob.glob(os.path.join(BIND_DIR, "bindung_*.yaml"))):
        with open(fp) as fh:
            doc = yaml.safe_load(fh)
        for b in doc.get("bindungen", []):
            slot = b.get("quelle", {}).get("signatur_slot")
            if slot:
                namen.add(slot)
        for l in doc.get("luecken", []):
            slot = l.get("signatur_slot")
            if slot:
                namen.add(slot)
    return namen


def test_z_ast_scan_ist_vollstaendig():
    """Gegen-Assert: GELESENE_SLOT_NAMEN muss exakt decken, was der AST-Scan in api.py heute
    findet. Wird rot, wenn api.py einen neuen slots.get/slots[...]-Leser bekommt, der hier nicht
    nachgetragen wurde — verhindert, dass die Konstante unbemerkt veraltet."""
    gefunden = _slot_reader_namen_per_ast(API_PY)
    assert gefunden == GELESENE_SLOT_NAMEN, (
        f"api.py liest Slots, die nicht in GELESENE_SLOT_NAMEN stehen: "
        f"{sorted(gefunden - GELESENE_SLOT_NAMEN)} — bewusst eintragen (mit Prüfung, ob sie "
        f"in einer Bindung existieren). Nicht mehr gelesen: {sorted(GELESENE_SLOT_NAMEN - gefunden)} "
        f"— hier entfernen."
    )


def test_gelesene_slot_namen_existieren_in_bindung():
    """Kern-Gate: jeder Slot-Name, den api.py per slots.get()/slots[...] TATSÄCHLICH liest, muss
    in mindestens einer produkt/bindung/bindung_*.yaml als signatur_slot existieren. Ein falsch
    benannter signatur_slot in der Bindung fällt hier auf — nicht mehr erst beim stillen
    0-Wert im /ergebnis."""
    vorhanden = _alle_signatur_slots()
    fehlend = GELESENE_SLOT_NAMEN - vorhanden
    assert not fehlend, (
        f"Slot-Name(n) {sorted(fehlend)} werden in produkt/haut/api.py per slots.get()/slots[...] "
        f"gelesen, existieren aber in KEINER produkt/bindung/bindung_*.yaml als signatur_slot. "
        f"Fail-open-Falle: bescheid_via_slots() liefert dann still den Default (0/\"einzel\") "
        f"statt eines Fehlers — siehe reports/adjudikation/"
        f"p2_festzusetzung_slot_verstoesse_2026-08-08.md."
    )
