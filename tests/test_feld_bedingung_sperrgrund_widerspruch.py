"""Ein Feld darf nur dann per Kreuz entfallen, wenn kein Sperrgrund es als bestaetigt verlangt.

Dieser Satz stand bisher nur als Kommentar im Code. Hier wird er maschinell gepruefft, ueber
BEIDE Scheiben, die den gemeinsamen Guard nutzen (gesamt, rentner_gesamt).

Menge A: alle Felder mit einer `feld_bedingung` in den bindung_*.yaml (ein Kreuz schliesst sie bei
    einem abweichenden bestaetigten Wert aus der Warteschlange aus, fail-closed -- unbestaetigt
    schliesst nie aus, s. traverser.py::_feld_ausgeschlossen).
Menge B: alle Felder, die `_an_gesamt_sperrgrund` (bescheid_deklaration.py) tatsaechlich als
    `zustand == "bestaetigt"` prueft, bevor es einen Bescheid zulaesst.
Der Schnitt A ∩ B ist der Verdachtsraum: ein Feld, das per Kreuz verschwinden kann, waehrend
derselbe Bescheid es weiter verlangt. Beide Mengen werden bei jedem Lauf frisch aus YAML und
Quelltext gebaut, nicht von Hand gepflegt -- ein neuer Treffer faellt beim naechsten Lauf auf.

## Nicht jeder Treffer ist ein Fund: das strukturelle Muster

Ein Guard kann im Code stehen und trotzdem nie in genau dem Zustand feuern, den der Schnitt
meldet. Live geprueft (nicht nur aus dem Code geschlossen, s. Anlass unten) fuer drei der zwoelf
mechanischen Treffer: der Guard verlangt `_positiv(X)` als Vorbedingung, und X traegt SELBST
dieselbe `feld_bedingung` wie das bewachte Feld. Wird das bewachte Feld ausgeschlossen (Kreuz
bestaetigt, abweichender Wert), ist X unter derselben Bedingung IMMER MIT ausgeschlossen -- X kann
in diesem Zustand nie positiv sein, der Guard also nie feuern, waehrend das bewachte Feld fehlt.
Das ist eine Aussage ueber die Bauart, nicht drei Einzelfaelle: `_strukturell_unerreichbar()`
unten prueft das Muster maschinell (Vorbedingungen aus allen umschliessenden `if`-Tests, auch bei
Verschachtelung statt flachem `and`) statt es als drei Namen mit "ist ok" zu listen -- eine
Ausnahme aus einem Muster laesst sich beim naechsten Lauf nachrechnen, eine aus Namen altert ohne
Fehlermeldung.

Die drei bekannten Instanzen (belegt in `test_muster_erklaert_genau_die_drei_bekannten_...`):
  - gewst_hebesatz (Kreuz kein_gewinn) -- Vorbedingung `_positiv("gewst_messbetrag")`, dasselbe
    Kreuz (bindung_an_gesamt.yaml).
  - behinderungsbedingte_aufwendungen_wahlrecht_pb (Kreuz keine_behinderung_pflege) -- Vorbedingung
    `_positiv("behinderungsbedingte_aufwendungen")`, dasselbe Kreuz (bindung_sonder_agb_35a.yaml
    Z.166 vs. Z.193).
  - behinderungsbedingte_aufwendungen_wahlrecht_pb_partner (Kreuz veranlagung=zusammen) --
    Vorbedingung `_positiv("behinderungsbedingte_aufwendungen_partner")`, dieselbe feld_bedingung
    (Z.237 vs. Z.257).
Waere der mechanische Schnitt ungeprueft als Fundliste gemeldet worden, waeren 3 von 12 Eintraegen
(25 %) erfunden gewesen -- eine Landkarte, die nach dem ersten Fehlalarm nicht mehr gelesen wird.

## Die zwei echten Treffer -- und ihre gemeinsame Wurzel

kap_kapitalertraege_partner/-gewinn_aktien_partner/-verlust_aktien_partner/-gewinn_sonstige_partner/
-verlust_sonstige_partner (Kreuz kein_kap_partner, Guard partner_kegel_offen) und
rentner_renten_art_partner/-jahresrente_partner/-renten_beginn_jahr_partner/
-alter_bei_rentenbeginn_partner (Kreuz kein_sonstige_partner, Guard rente_instanz_offen) matchen
das Muster NICHT (belegt: keine `_positiv(X)`-Vorbedingung mit geteilter feld_bedingung) und sind
live bestaetigte, ungeloeste Treffer -- Namenseintraege in BEKANNTE_TREFFER, mit Beleg, nicht mit
"ist ok".

Beide haben dieselbe Wurzel: EIN KREUZ, DAS NICHT ZUR SCHEIBE GEHOERT, WIRKT TROTZDEM AUF SIE.
  - kein_kap_partner steht in "gesamt".felder, GESAMT_PARTNER_KAP (die 5 Felder oben) aber in
    BEIDEN Scheiben. Der Guard, der sie verlangt (`cfg.get("partner_19")`), feuert nur auf
    "gesamt" (partner_19 ist nur dort True, api_constants.py:729) -- auf rentner_gesamt sind die
    5 Felder erzwungene, unabschaltbare FRAGEN ohne Sperrgrund-Wirkung (laestig, nicht
    gefaehrlich). Auf "gesamt" ist es der Blocker: Kreuz frueh bestaetigt (jeder Weg, der ein
    `ui:laie`-Event ohne Katalogpruefung schreibt, s. test_verstanden_bestaetigt_ohne_
    katalog_pruefung.py) -> 5 Felder aus der Warteschlange, Guard verlangt sie weiter,
    sperr="partner_kegel_offen". SPERRE MIT AUSWEG, KEINE SACKGASSE: ein anderer Worker hat den
    Rueckweg aus genau diesem Zustand gemessen (nicht nur gelesen) -- ein Korrektur-Event
    (`ersetzt=<event_id>`, app.js::korrigiereBestaetigt) wird angenommen, die fuenf Felder kommen
    zurueck in die Warteschlange, das Ergebnis wird berechnet, und der Weg dahin ist auf dem
    Ergebnis-Screen sichtbar und anklickbar -- nicht nur technisch moeglich, sondern im Produkt
    auffindbar.
  - kein_sonstige_partner ist Teil von PARTNER_SCREENING (api_constants.py:129-134), das NUR in
    "gesamt".felder eingemischt wird (Z.712) -- in rentner_gesamt steht das Kreuz selbst gar
    nicht in der Warteschlange (live gemessen: Position "NIE GEFRAGT"), nur per Seitenkanal
    setzbar. RENTNER_22_PARTNER (die 4 Felder oben) wird aber per `+ RENTNER_22_PARTNER`
    (Z.359) in RENTNER_FELDER aufgenommen. Live gemessen (2026-08-30): natuerlicher Klick-Durch-
    lauf, nach dem ersten der vier Felder das Kreuz per Seitenkanal gesetzt -> die restlichen
    drei bleiben aus der Warteschlange, sperr="rente_instanz_offen". Gefaehrlicher als der
    kap_partner-Fall, weil hier nicht einmal die Reihenfolge im Normalfall schuetzt -- das Kreuz
    existiert in dieser Scheibe formal nicht.

## Was dieses Gate NICHT prueft (Grenze, nicht Luecke)

  - Die Scheiben-Erreichbarkeit einzelner cfg-Zweige (partner_19/rentner/gesamt_guard) wird NICHT
    strukturell nachgebaut -- das waere eine vierte, fragile Repraesentation derselben
    Verzweigung. Die beiden echten Treffer oben sind per Hand+Live-Lauf auf ihre jeweils EINE
    Scheibe zugeordnet (s. Kommentare bei BEKANNTE_TREFFER); das Gate selbst prueft nur A ∩ B
    global (scheiben-unabhaengig) gegen Muster+Ausnahmeliste.
  - "Ohne Ausgang" (gibt es eine wahrheitsgemaesse Antwort, die das Gate oeffnet?): fuer beide
    echten Treffer JA -- kein_kap_partner/kein_sonstige_partner sind normale, situationsabhaengige
    Fragen nach dem Partner (bindung_screening_partner.yaml Z.73-81, 129-137), keine, deren
    Antwort durch die Scheiben-Zugehoerigkeit selbst feststeht (anders als kein_sonstige fuer
    einen Rentner -- s. u.). Nicht geprueft: der Randfall, dass BEIDE Partner Rentner sind
    (kein_sonstige_partner koennte dann strukturell dieselbe Falle sein wie kein_sonstige fuer
    den Nutzer selbst) -- nicht gemessen, absichtlich offen gelassen statt geraten.
  - Der §23-Fall (rentner_gesamt, p23_veraeusserungspreis, fremd_arten kennt kein_sonstige nicht
    fuer diese Scheibe) faellt NICHT in Menge A -- das Feld hat keine feld_bedingung
    (bindung_p23_gesamt.yaml Z.12-17). Anderer Fehlerschuh (fehlendes Gate auf einem ungegateten
    Feld, nicht ein falsch feuerndes Gate auf einem gegateten) -- ausserhalb dieses Gates,
    absichtlich nicht mit hineingezogen.

Anlass: Auftrag ueber den orch-Bus, 2026-08-30 -- den verlorenen Kommentar maschinell pruefbar
machen, ueber gesamt UND rentner_gesamt.
"""
from __future__ import annotations

import ast
import glob
import os
import sys

import yaml

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/haut", "produkt/bescheid"):
    _p = os.path.join(ROOT, _sub)
    if _p not in sys.path:
        sys.path.insert(0, _p)

import api_constants as AC  # noqa: E402
import bescheid_deklaration as BD  # noqa: E402

# Bekannte, ungeloeste Treffer -- NICHT vom Muster erklaerbar (s. test_muster_erklaert_genau_...).
# Live gemessen, nicht geraten. Reparatur liegt bei Julius/team-lead, s. Modul-Docstring.
BEKANNTE_TREFFER = {
    "kap_kapitalertraege_partner": "gesamt (partner_kegel_offen, Kreuz kein_kap_partner)",
    "kap_gewinn_aktien_partner": "gesamt (partner_kegel_offen, Kreuz kein_kap_partner)",
    "kap_verlust_aktien_partner": "gesamt (partner_kegel_offen, Kreuz kein_kap_partner)",
    "kap_gewinn_sonstige_partner": "gesamt (partner_kegel_offen, Kreuz kein_kap_partner)",
    "kap_verlust_sonstige_partner": "gesamt (partner_kegel_offen, Kreuz kein_kap_partner)",
    "rentner_renten_art_partner": "rentner_gesamt (rente_instanz_offen, Kreuz kein_sonstige_partner)",
    "rentner_jahresrente_partner": "rentner_gesamt (rente_instanz_offen, Kreuz kein_sonstige_partner)",
    "rentner_renten_beginn_jahr_partner": "rentner_gesamt (rente_instanz_offen, Kreuz kein_sonstige_partner)",
    "rentner_alter_bei_rentenbeginn_partner": "rentner_gesamt (rente_instanz_offen, Kreuz kein_sonstige_partner)",
}


# --------------------------------------------------------------- Menge A: ausschliessbare Felder

def _menge_a() -> dict[str, dict]:
    """feld_id -> feld_bedingung (feld/wert/wert_nicht), aus allen bindung_*.yaml. Quelle ist die
    YAML selbst -- eine neue feld_bedingung ist beim naechsten Lauf automatisch mit drin."""
    out: dict[str, dict] = {}
    for pfad in sorted(glob.glob(os.path.join(ROOT, "produkt/bindung/bindung_*.yaml"))):
        with open(pfad, encoding="utf-8") as f:
            daten = yaml.safe_load(f)
        for b in (daten or {}).get("bindungen", []) or []:
            fb = b.get("feld_bedingung")
            if fb:
                out[b["feld_id"]] = {"feld": fb.get("feld"), "wert": fb.get("wert"),
                                      "wert_nicht": fb.get("wert_nicht")}
    return out


# --------------------------------------------------------------- Menge B: vom Guard verlangte Felder

def _funktion(modul, name: str) -> ast.FunctionDef:
    quelle_pfad = modul.__file__
    baum = ast.parse(open(quelle_pfad, encoding="utf-8").read(), filename=quelle_pfad)
    for n in ast.walk(baum):
        if isinstance(n, (ast.FunctionDef, ast.AsyncFunctionDef)) and n.name == name:
            return n
    raise AssertionError(f"{name} nicht in {quelle_pfad} gefunden -- umbenannt oder verschoben?")


def _resolve_feldnamen(expr: ast.AST, local_assigns: dict, stack: tuple = ()) -> set[str]:
    """Ausdruck -> Menge von Feldnamen. Versteht String-Konstante, Tuple/List/Set, `a + b`,
    frozenset(...)/tuple(...)/set(...)/list(...), lokale Zuweisung, api_constants-Attribut."""
    if isinstance(expr, ast.Constant) and isinstance(expr.value, str):
        return {expr.value}
    if isinstance(expr, (ast.Tuple, ast.List, ast.Set)):
        out: set[str] = set()
        for e in expr.elts:
            out |= _resolve_feldnamen(e, local_assigns, stack)
        return out
    if isinstance(expr, ast.BinOp) and isinstance(expr.op, ast.Add):
        return (_resolve_feldnamen(expr.left, local_assigns, stack)
                | _resolve_feldnamen(expr.right, local_assigns, stack))
    if (isinstance(expr, ast.Call) and isinstance(expr.func, ast.Name)
            and expr.func.id in ("frozenset", "set", "tuple", "list")):
        return _resolve_feldnamen(expr.args[0], local_assigns, stack) if expr.args else set()
    if isinstance(expr, ast.Name):
        if expr.id in stack:
            raise AssertionError(f"Zirkelbezug bei {expr.id!r}")
        if expr.id in local_assigns:
            return _resolve_feldnamen(local_assigns[expr.id], local_assigns, stack + (expr.id,))
        if hasattr(AC, expr.id):
            val = getattr(AC, expr.id)
            if isinstance(val, (tuple, list, set, frozenset)) and all(isinstance(x, str) for x in val):
                return set(val)
        raise AssertionError(f"Unbekannter Name {expr.id!r} (weder lokale Zuweisung noch api_constants)")
    raise AssertionError(f"Nicht aufloesbarer Ausdruck: {ast.dump(expr)[:150]}")


def _bind_target(target: ast.AST, iter_expr: ast.AST) -> dict[str, ast.AST]:
    """target = for-Ziel, iter_expr = durchlaufenes Iterable. Versteht Name sowie
    zip(...)/enumerate(...) elementweise (fuer die vpf_*-Doppelschleife im Guard)."""
    if (isinstance(iter_expr, ast.Call) and isinstance(iter_expr.func, ast.Name)
            and iter_expr.func.id == "enumerate" and iter_expr.args):
        if isinstance(target, ast.Tuple) and len(target.elts) == 2:
            return _bind_target(target.elts[1], iter_expr.args[0])
        return {}
    if (isinstance(iter_expr, ast.Call) and isinstance(iter_expr.func, ast.Name)
            and iter_expr.func.id == "zip"):
        if isinstance(target, ast.Tuple) and len(target.elts) == len(iter_expr.args):
            out: dict[str, ast.AST] = {}
            for sub_t, sub_i in zip(target.elts, iter_expr.args):
                out.update(_bind_target(sub_t, sub_i))
            return out
        return {}
    if isinstance(target, ast.Name):
        return {target.id: iter_expr}
    return {}


def _positiv_felder_in(node: ast.AST) -> set[str]:
    """Alle Feldnamen, die als `_positiv("...")` in `node` gelesen werden."""
    out: set[str] = set()
    for n in ast.walk(node):
        if (isinstance(n, ast.Call) and isinstance(n.func, ast.Name) and n.func.id == "_positiv"
                and n.args and isinstance(n.args[0], ast.Constant) and isinstance(n.args[0].value, str)):
            out.add(n.args[0].value)
    return out


def _eltern_karte(fn: ast.AST) -> dict[ast.AST, ast.AST]:
    eltern: dict[ast.AST, ast.AST] = {}
    for node in ast.walk(fn):
        for kind in ast.iter_child_nodes(node):
            eltern[kind] = node
    return eltern


def _vorbedingung_positiv_felder(node: ast.AST, eltern: dict[ast.AST, ast.AST]) -> set[str]:
    """Alle `_positiv(...)`-Felder in JEDEM umschliessenden `if`-Test -- nicht nur dem naechsten.
    Deckt sowohl `if A and B:` (flache BoolOp, gewst_hebesatz) als auch `if A:\\n    if B:`
    (Verschachtelung, behinderung-Wahlrecht) ab, beide kommen im Guard vor."""
    out: set[str] = set()
    cur = node
    while cur in eltern:
        p = eltern[cur]
        if isinstance(p, ast.If):
            out |= _positiv_felder_in(p.test)
        cur = p
    return out


def _menge_b_mit_vorbedingungen() -> tuple[set[str], dict[str, set[str]]]:
    """Menge B (Felder, die der Guard als bestaetigt verlangt) UND, je Feld, die Menge der
    `_positiv(...)`-Felder in einem umschliessenden if -- Rohmaterial fuer die Musterpruefung."""
    fn = _funktion(BD, "_an_gesamt_sperrgrund")
    eltern = _eltern_karte(fn)
    local_assigns: dict[str, ast.AST] = {}
    for n in ast.walk(fn):
        if isinstance(n, ast.Assign) and len(n.targets) == 1 and isinstance(n.targets[0], ast.Name):
            local_assigns.setdefault(n.targets[0].id, n.value)

    gefunden: set[str] = set()
    vorbedingungen: dict[str, set[str]] = {}

    def _walk(node: ast.AST, scope: dict[str, ast.AST]) -> None:
        if isinstance(node, ast.For):
            neu = dict(scope)
            neu.update(_bind_target(node.target, node.iter))
            for kind in node.body:
                _walk(kind, neu)
            for kind in node.orelse:
                _walk(kind, neu)
            return
        if isinstance(node, (ast.GeneratorExp, ast.ListComp, ast.SetComp, ast.DictComp)):
            neu = dict(scope)
            for comp in node.generators:
                neu.update(_bind_target(comp.target, comp.iter))
                for cond in comp.ifs:
                    _walk(cond, neu)
            if isinstance(node, ast.DictComp):
                _walk(node.key, neu)
                _walk(node.value, neu)
            else:
                _walk(node.elt, neu)
            return
        if (isinstance(node, ast.Compare) and len(node.ops) == 1
                and isinstance(node.ops[0], (ast.Eq, ast.NotEq))):
            seiten = [node.left] + node.comparators
            konst = next((s for s in seiten if isinstance(s, ast.Constant) and s.value == "bestaetigt"), None)
            andere = next((s for s in seiten if s is not konst), None)
            if konst is not None and andere is not None:
                arg = None
                for n in ast.walk(andere):
                    if (isinstance(n, ast.Call) and isinstance(n.func, ast.Attribute)
                            and n.func.attr == "get" and isinstance(n.func.value, ast.Name)
                            and n.func.value.id == "felder" and n.args):
                        arg = n.args[0]
                        break
                if arg is not None:
                    if isinstance(arg, ast.Name) and arg.id in scope:
                        namen = _resolve_feldnamen(scope[arg.id], local_assigns)
                    else:
                        namen = _resolve_feldnamen(arg, local_assigns)
                    vb = _vorbedingung_positiv_felder(node, eltern)
                    for f in namen:
                        gefunden.add(f)
                        vorbedingungen.setdefault(f, set()).update(vb)
        for kind in ast.iter_child_nodes(node):
            _walk(kind, scope)

    _walk(fn, {})
    return gefunden, vorbedingungen


# --------------------------------------------------------------- Musterpruefung + Gate

def _teilt_feld_bedingung(a: dict, b: dict) -> bool:
    return a.get("feld") == b.get("feld") and a.get("wert") == b.get("wert") and a.get("wert_nicht") == b.get("wert_nicht")


def _strukturell_unerreichbar(feld: str, menge_a: dict[str, dict], vorbedingungen: dict[str, set[str]]) -> str | None:
    """Muster (s. Modul-Docstring): eine `_positiv(X)`-Vorbedingung im Guard, wobei X dieselbe
    feld_bedingung traegt wie `feld` selbst -- X kann dann nie positiv sein, waehrend `feld`
    ausgeschlossen ist. Gibt den Namen von X als Beleg zurueck, sonst None."""
    fb = menge_a.get(feld)
    if not fb:
        return None
    for x in vorbedingungen.get(feld, ()):
        fb_x = menge_a.get(x)
        if fb_x and _teilt_feld_bedingung(fb, fb_x):
            return x
    return None


def _ungeklaerte_treffer(menge_a: dict[str, dict], menge_b: set[str], vorbedingungen: dict[str, set[str]]) -> list[str]:
    schnitt = sorted(set(menge_a) & menge_b)
    out = []
    for feld in schnitt:
        if _strukturell_unerreichbar(feld, menge_a, vorbedingungen):
            continue
        if feld in BEKANNTE_TREFFER:
            continue
        out.append(feld)
    return out


def test_mengen_nicht_leer():
    """Waechter gegen eine Extraktion, die mangels Fundstellen (umbenannte Funktion, geaenderte
    YAML-Struktur) still leer wird und den Gate-Test unten sinnlos gruen macht."""
    menge_a = _menge_a()
    menge_b, _ = _menge_b_mit_vorbedingungen()
    assert len(menge_a) > 30, f"Menge A verdaechtig klein ({len(menge_a)}) -- YAML-Scan kaputt?"
    assert len(menge_b) > 20, f"Menge B verdaechtig klein ({len(menge_b)}) -- Guard-Extraktion kaputt?"


def test_muster_erklaert_genau_die_drei_bekannten_falsch_positiven():
    """Haelt fest, WAS die Musterpruefung heute erklaert -- nicht mehr, nicht weniger -- damit eine
    kuenftige Verschiebung sichtbar wird, statt sich in den Gate-Treffern zu verstecken."""
    menge_a = _menge_a()
    menge_b, vorbedingungen = _menge_b_mit_vorbedingungen()
    schnitt = set(menge_a) & menge_b

    erklaert = {f for f in schnitt if _strukturell_unerreichbar(f, menge_a, vorbedingungen)}
    erwartet = {
        "gewst_hebesatz",
        "behinderungsbedingte_aufwendungen_wahlrecht_pb",
        "behinderungsbedingte_aufwendungen_wahlrecht_pb_partner",
    }
    assert erklaert == erwartet, (
        f"Musterpruefung erklaert {sorted(erklaert)}, erwartet {sorted(erwartet)} -- neuer Fall "
        "dazugekommen (pruefen, ob echt oder Muster) oder Muster hat sich verschoben.")


def test_feld_bedingung_schnitt_sperrgrund_nur_bekannte_oder_strukturell_unerreichbare_treffer():
    """Das eigentliche Gate. A ∩ B, frisch aus YAML+Guard gebaut. Jeder Treffer, der weder ins
    Muster (strukturell unerreichbar) noch in BEKANNTE_TREFFER faellt, ist ein NEUER Fund."""
    menge_a = _menge_a()
    menge_b, vorbedingungen = _menge_b_mit_vorbedingungen()
    ungeklaert = _ungeklaerte_treffer(menge_a, menge_b, vorbedingungen)
    assert not ungeklaert, (
        "Neue(r) Treffer in Menge A ∩ Menge B, weder strukturell unerreichbar noch bekannt: "
        f"{ungeklaert}. Ein Feld darf nur per Kreuz entfallen, wenn kein Sperrgrund es als "
        "bestaetigt verlangt -- pruefen, ob ein Sperrgrund oder eine feld_bedingung neu "
        "hinzugekommen ist, und das Ergebnis in BEKANNTE_TREFFER oder als Musterfall eintragen.")


def test_gate_reagiert_auf_neue_feld_bedingung_mutationsprobe():
    """Beweist, dass das Gate auf einen NEUEN Treffer reagiert -- nicht nur, dass es heute gruen
    ist. hh_rechnung_unbar ist in Menge B (bescheid_deklaration.py: `_hh_instanz_positiv(...) and
    ... hh_rechnung_unbar != bestaetigt -> rechnung_unbar_offen`) und traegt HEUTE keine
    feld_bedingung (bindung_sonder_agb_35a.yaml Z.522-528) -- sauberes, kollisionsfreies Ziel;
    seine einzige Vorbedingung ist `_hh_instanz_positiv`, kein `_positiv`, matcht das
    strukturelle Muster also nicht, egal was injiziert wird.

    Zwei getrennte Behauptungen, nicht eine: (1) die Mutation ist tatsaechlich in der
    Datenstruktur angekommen, (2) ERST DANACH aendert sie das Gate-Ergebnis. Eine Probe, die nur
    (2) zeigt, koennte gruen bleiben, weil die Mutation gar nicht ankam, und saehe trotzdem wie
    ein bestandener Test aus."""
    menge_a = _menge_a()
    menge_b, vorbedingungen = _menge_b_mit_vorbedingungen()

    assert "hh_rechnung_unbar" in menge_b, "Praemisse verletzt: hh_rechnung_unbar nicht mehr in Menge B."
    assert "hh_rechnung_unbar" not in menge_a, (
        "Praemisse verletzt: hh_rechnung_unbar traegt inzwischen eine echte feld_bedingung -- "
        "Mutationsziel neu waehlen.")
    assert "hh_rechnung_unbar" not in _ungeklaerte_treffer(menge_a, menge_b, vorbedingungen), (
        "Vor der Mutation sollte hh_rechnung_unbar nicht auftauchen.")

    # (1) Mutation einspielen und BELEGEN, dass sie ankam -- bevor irgendeine Gate-Logik laeuft.
    mutation = {"feld": "kein_gewinn", "wert": False, "wert_nicht": None}
    mutiert = dict(menge_a)
    mutiert["hh_rechnung_unbar"] = mutation
    assert mutiert["hh_rechnung_unbar"] == mutation
    assert "hh_rechnung_unbar" not in menge_a, "Mutation hat das Original veraendert statt einer Kopie."

    # (2) ERST DANACH: dieselbe Gate-Logik auf den mutierten Zustand losgelassen -- muss rot werden.
    treffer_nach_mutation = _ungeklaerte_treffer(mutiert, menge_b, vorbedingungen)
    assert "hh_rechnung_unbar" in treffer_nach_mutation, (
        "Mutation kam an (siehe (1)), aber das Gate hat sie nicht bemerkt -- die Gate-Logik "
        "selbst ist blind fuer einen neuen Treffer.")
