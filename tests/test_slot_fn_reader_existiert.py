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

KORREKTUR main (2026-08-08, zweiter Durchgang): die erste Fassung prüfte nur "existiert der
Slot-Name IRGENDWO im Repo" (_alle_signatur_slots() über alle bindung_*.yaml). Das ist blind
gegen einen zweiten Träger desselben Namens — Präzedenzfall bereits im Repo: `jahresrente` steht
sowohl in bindung_an_gesamt.yaml:33 als auch bindung_rentner.yaml:13 (s. p19_2-Report). Wird der
Name in der Bindung umbenannt, die der tatsächliche Rechenweg benutzt, bleibt das alte Gate grün,
weil derselbe Name in einer ANDEREN, für diesen Rechenweg irrelevanten Datei noch existiert —
Mutationsprobe unten belegt das (test_alte_pruefung_ist_blind_gegen_zweiten_traeger).

Gemessen (siehe unten): `quantitaet` (der Aufrufparameter von bescheid_via_slots) mappt 1:1 auf
genau eine Scheibe über SCHEIBEN[...]["gesamt_ring"], und jede der vier Scheiben, die
bescheid_via_slots aufrufen (ep, an_gesamt, gesamt, rentner_gesamt), hat ein STATISCHES
`felder`-Tupel (kein felder_datei-Umweg) — die Bindung einer Scheibe ist also zur Testzeit
eindeutig bestimmbar (TR.lade_bindung() gefiltert auf SCHEIBEN[scheibe]["felder"], exakt wie
api.py:_scheibe_bindung() es zur Laufzeit tut). Der Fix prüft deshalb scheibengenau (Variante 2
aus dem Auftrag), nicht nur "irgendwo im Repo".
"""
from __future__ import annotations

import ast
import glob
import os
import sys

import pytest

yaml = pytest.importorskip("yaml")

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
BIND_DIR = os.path.join(ROOT, "produkt", "bindung")
API_PY = os.path.join(ROOT, "produkt", "haut", "api.py")

sys.path.insert(0, os.path.join(ROOT, "produkt", "haut"))
import api_constants as AC  # noqa: E402

# Heute (2026-08-08) tatsächlich per slots.get(...)/slots[...] gelesene Slot-Namen in
# produkt/haut/api.py, JE quantitaet-Zweig von _bescheid_fn — gemessen per AST (s.
# test_z_ast_scan_ist_vollstaendig unten), nicht geraten. Wächst NUR bewusst: ein neuer Leser
# in api.py muss hier eingetragen werden, sonst schlägt test_z_ast_scan_ist_vollstaendig fehl.
GELESENE_SLOT_NAMEN_JE_QUANTITAET = {
    "abziehbarer_betrag": {"arbeitstage", "entfernung_km_roh", "oepnv_kosten_jahr",
                           "eigenes_oder_ueberlassenes_kfz"},
    "festzusetzende_est": {"arbeitstage", "entfernung_km_roh", "oepnv_kosten_jahr",
                           "eigenes_oder_ueberlassenes_kfz", "bruttoarbeitslohn", "veranlagung"},
    "festzusetzende_est_gesamt": {"arbeitstage", "entfernung_km_roh", "oepnv_kosten_jahr",
                                  "eigenes_oder_ueberlassenes_kfz", "bruttoarbeitslohn", "veranlagung"},
    "festzusetzende_est_rentner": set(),
}

# quantitaet -> Scheibe (SCHEIBEN-Key), über die die Bindung für diesen Rechenweg bestimmt wird.
# 1:1, weil jede quantitaet genau einer Scheibe als deren "gesamt_ring" zugeordnet ist.
QUANTITAET_ZU_SCHEIBE = {v["gesamt_ring"]: k for k, v in AC.SCHEIBEN.items() if v.get("gesamt_ring")}


def _slot_reader_namen_je_quantitaet(path: str) -> dict[str, set[str]]:
    """Sammelt String-Literale, mit denen api.py den `slots`-Parameter einer slot_fn liest
    (slots.get("X", ...), slots["X"], {k: slots[k] for k in ("X", "Y", ...)}), GETRENNT je
    `if quantitaet == "..."`-Zweig innerhalb von _bescheid_fn."""
    def sammle(node) -> set[str]:
        namen: set[str] = set()
        for n in ast.walk(node):
            if (isinstance(n, ast.Call) and isinstance(n.func, ast.Attribute)
                    and n.func.attr == "get" and isinstance(n.func.value, ast.Name)
                    and n.func.value.id == "slots" and n.args
                    and isinstance(n.args[0], ast.Constant) and isinstance(n.args[0].value, str)):
                namen.add(n.args[0].value)
            if (isinstance(n, ast.Subscript) and isinstance(n.value, ast.Name)
                    and n.value.id == "slots"):
                sl = n.slice
                if isinstance(sl, ast.Constant) and isinstance(sl.value, str):
                    namen.add(sl.value)
            if isinstance(n, ast.DictComp):
                for gen in n.generators:
                    if isinstance(gen.iter, (ast.Tuple, ast.List)):
                        for elt in gen.iter.elts:
                            if isinstance(elt, ast.Constant) and isinstance(elt.value, str):
                                namen.add(elt.value)
            # _oepnv_eur(...) ist ein Modul-Helper AUSSERHALB jedes quantitaet-Blocks
            # (api.py:261), der intern "oepnv_kosten_jahr" liest — ein reiner
            # Literal-Scan innerhalb des Blocks würde diesen Namen sonst nicht sehen,
            # egal ob der Helper mit `slots` oder einem daraus abgeleiteten dict
            # (z.B. wk_input) aufgerufen wird.
            if isinstance(n, ast.Call) and isinstance(n.func, ast.Name) and n.func.id == "_oepnv_eur":
                namen.add("oepnv_kosten_jahr")
        return namen

    tree = ast.parse(open(path, encoding="utf-8").read())
    fn = next(n for n in ast.walk(tree)
              if isinstance(n, ast.FunctionDef) and n.name == "_bescheid_fn")
    out: dict[str, set[str]] = {}
    for stmt in fn.body:
        if (isinstance(stmt, ast.If) and isinstance(stmt.test, ast.Compare)
                and isinstance(stmt.test.left, ast.Name) and stmt.test.left.id == "quantitaet"
                and len(stmt.test.ops) == 1 and isinstance(stmt.test.ops[0], ast.Eq)
                and isinstance(stmt.test.comparators[0], ast.Constant)):
            out[stmt.test.comparators[0].value] = sammle(stmt)
    return out


def _signatur_slots_je_datei() -> dict[str, set[str]]:
    """dateiname -> Menge der signatur_slot-Werte darin (bindungen + luecken)."""
    out: dict[str, set[str]] = {}
    for fp in sorted(glob.glob(os.path.join(BIND_DIR, "bindung_*.yaml"))):
        with open(fp) as fh:
            doc = yaml.safe_load(fh)
        namen: set[str] = set()
        for b in doc.get("bindungen", []):
            slot = b.get("quelle", {}).get("signatur_slot")
            if slot:
                namen.add(slot)
        for l in doc.get("luecken", []):
            slot = l.get("signatur_slot")
            if slot:
                namen.add(slot)
        out[os.path.basename(fp)] = namen
    return out


def _signatur_slots_fuer_scheibe(scheibe: str) -> set[str]:
    """Die signatur_slot-Werte, die TR.lade_bindung() für diese Scheibe liefert — nachgebaut wie
    api.py:_scheibe_bindung() zur Laufzeit filtert: feld_id in SCHEIBEN[scheibe]['felder'] (bzw.
    aus felder_datei), dann quelle.signatur_slot je gebundenem Feld."""
    cfg = AC.SCHEIBEN[scheibe]
    if cfg["felder"] is not None:
        feld_ids = set(cfg["felder"])
    else:
        with open(os.path.join(BIND_DIR, cfg["felder_datei"])) as fh:
            d = yaml.safe_load(fh)
        feld_ids = {b["feld_id"] for b in d.get("bindungen", [])}

    alle_bindungen: dict[str, dict] = {}
    for fp in sorted(glob.glob(os.path.join(BIND_DIR, "bindung_*.yaml"))):
        with open(fp) as fh:
            doc = yaml.safe_load(fh)
        for b in doc.get("bindungen", []):
            alle_bindungen[b["feld_id"]] = b

    slots: set[str] = set()
    for fid in feld_ids:
        b = alle_bindungen.get(fid)
        if b is None:
            continue
        slot = b.get("quelle", {}).get("signatur_slot")
        if slot:
            slots.add(slot)
    return slots


def test_z_ast_scan_ist_vollstaendig():
    """Gegen-Assert: GELESENE_SLOT_NAMEN_JE_QUANTITAET muss exakt decken, was der AST-Scan in
    api.py heute je quantitaet-Zweig findet. Wird rot, wenn api.py einen neuen slots.get/
    slots[...]-Leser bekommt, der hier nicht nachgetragen wurde."""
    gefunden = _slot_reader_namen_je_quantitaet(API_PY)
    assert set(gefunden) == set(GELESENE_SLOT_NAMEN_JE_QUANTITAET), (
        f"quantitaet-Zweige in _bescheid_fn haben sich geändert: "
        f"{sorted(set(gefunden) ^ set(GELESENE_SLOT_NAMEN_JE_QUANTITAET))}"
    )
    for q, erwartet in GELESENE_SLOT_NAMEN_JE_QUANTITAET.items():
        assert gefunden[q] == erwartet, (
            f"quantitaet={q!r}: api.py liest Slots {sorted(gefunden[q] - erwartet)}, "
            f"die nicht in GELESENE_SLOT_NAMEN_JE_QUANTITAET[{q!r}] stehen — bewusst eintragen "
            f"(mit Prüfung, ob sie in DER Bindung dieser Scheibe existieren). Nicht mehr "
            f"gelesen: {sorted(erwartet - gefunden[q])} — hier entfernen."
        )


def test_gelesene_slot_namen_existieren_in_der_scheibe_die_sie_liest():
    """Kern-Gate, scheibengenau: für jede quantitaet muss jeder gelesene Slot-Name in DER
    Bindung existieren, die api.py._scheibe_bindung() für die zugehörige Scheibe tatsächlich an
    bescheid_via_slots übergibt — nicht nur irgendwo im Repo.

    Schärfer als eine repo-weite Existenzprüfung: `jahresrente` existiert z.B. gleichzeitig in
    bindung_an_gesamt.yaml UND bindung_rentner.yaml (p19_2 vs. p22_1, dokumentierte Kollision,
    s. p19_2-Report). Eine repo-weite Prüfung wäre blind, wenn genau EINER der beiden Träger
    umbenannt würde, weil der andere den Namen retten würde, obwohl der Rechenweg, der ihn
    wirklich liest, ihn verloren hat — Doppel-Mutationsprobe (main, 2026-08-08) belegt das:
    altes Gate 2 passed, /ergebnis trotzdem {"zahl_cent": 0, "grund": "bestaetigt"}."""
    for q, namen in GELESENE_SLOT_NAMEN_JE_QUANTITAET.items():
        if not namen:
            continue
        scheibe = QUANTITAET_ZU_SCHEIBE[q]
        vorhanden = _signatur_slots_fuer_scheibe(scheibe)
        fehlend = namen - vorhanden
        assert not fehlend, (
            f"quantitaet={q!r} (Scheibe {scheibe!r}): Slot-Name(n) {sorted(fehlend)} werden von "
            f"api.py per slots.get()/slots[...] gelesen, existieren aber NICHT in der Bindung "
            f"dieser Scheibe (nur ggf. in einer anderen bindung_*.yaml, die für {scheibe!r} "
            f"nicht geladen wird). Fail-open-Falle: bescheid_via_slots() liefert dann still den "
            f"Default (0/\"einzel\") statt eines Fehlers — siehe reports/adjudikation/"
            f"p2_festzusetzung_slot_verstoesse_2026-08-08.md."
        )


def test_scheibenfilter_ist_echt_und_nicht_leer():
    """Sperrt die Degradation, die das Kern-Gate oben still entwerten würde: liefert
    _signatur_slots_fuer_scheibe() eines Tages die repo-weite Menge (z.B. weil eine Scheibe auf
    felder=None/felder_datei umgestellt wird und der Fallback zu breit greift) oder eine leere
    Menge, dann prüft der Kern-Assert wieder nur 'existiert der Name irgendwo' bzw. gar nichts —
    grün, aber blind. Genau die Schwäche, die dieser Durchgang beseitigt hat.

    Warum überhaupt nötig: signatur_slot-Namen doppeln sich über Dateien hinweg real — gemessen
    2026-08-08 sind es 16 Kollisionspaare (u.a. `aufwendungen` in 4 Dateien, `jahresrente` in
    an_gesamt+rentner, s. p19_2-Report). Dass heute KEINER der sechs von api.py gelesenen Namen
    doppelt getragen wird (je genau ein Träger, nachgemessen), ist Glück, keine Invariante.
    Die scheibengenaue Prüfung ist deshalb Vorsorge, und dieser Test hält sie scharf."""
    repo_weit = set().union(*_signatur_slots_je_datei().values())
    for q in GELESENE_SLOT_NAMEN_JE_QUANTITAET:
        scheibe = QUANTITAET_ZU_SCHEIBE[q]
        vorhanden = _signatur_slots_fuer_scheibe(scheibe)
        assert vorhanden, (
            f"Scheibe {scheibe!r} (quantitaet={q!r}) liefert KEINE signatur_slots — der "
            f"Kern-Assert oben prüft dann nichts mehr und ist still grün."
        )
        assert vorhanden < repo_weit, (
            f"Scheibe {scheibe!r} (quantitaet={q!r}) liefert {len(vorhanden)} von "
            f"{len(repo_weit)} repo-weiten Slots — keine echte Teilmenge mehr. Der Kern-Assert "
            f"oben ist damit auf die alte, blinde 'existiert irgendwo'-Prüfung zurückgefallen."
        )
