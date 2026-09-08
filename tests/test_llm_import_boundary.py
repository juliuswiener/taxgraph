"""Guard-Test: LLM-Client ist auf api_llm.py isoliert. Keine anderen Haut-Module dürfen llm_client importieren."""

import os
import ast

HAUT_DIR = os.path.join(
    os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
    "produkt", "haut"
)

# Module, die llm_client importieren DUERFEN:
#   api_llm.py    -- ist die vorgesehene Isolationsschicht, MUSS llm_client importieren.
#   llm_client.py -- ist der Client selbst; ein Treffer auf sich selbst waere kein Verstoss.
AUSNAHMEN = {"api_llm.py", "llm_client.py"}


def _parse_imports(source: str) -> tuple[set[str], set[str]]:
    """Parse Python source, return (top-level imports, from-imports)."""
    top_level = set()
    from_imports = set()
    try:
        tree = ast.parse(source)
    except SyntaxError:
        return top_level, from_imports

    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            for alias in node.names:
                top_level.add(alias.name)
        elif isinstance(node, ast.ImportFrom):
            if node.module:
                from_imports.add(node.module)
    return top_level, from_imports


def _importiert_llm_client(pfad: str) -> bool:
    with open(pfad, "r") as f:
        source = f.read()
    top_level, from_imports = _parse_imports(source)
    return "llm_client" in top_level or "llm_client" in from_imports


def _verstoesse(haut_dir: str, ausnahmen: set[str]) -> list[str]:
    """Modulnamen (nicht Pfade) unter haut_dir, die llm_client importieren und nicht in
    ausnahmen stehen. Durchlaeuft haut_dir per Verzeichnislisting -- KEINE hartkodierte
    Namensliste, damit ein neues Modul automatisch erfasst wird, sobald es existiert."""
    treffer = []
    for name in sorted(os.listdir(haut_dir)):
        if not name.endswith(".py") or name in ausnahmen:
            continue
        pfad = os.path.join(haut_dir, name)
        if not os.path.isfile(pfad):
            continue
        if _importiert_llm_client(pfad):
            treffer.append(name)
    return treffer


def test_llm_client_not_in_api():
    """api.py darf NICHT llm_client importieren."""
    api_path = os.path.join(
        os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
        "produkt", "haut", "api.py"
    )
    with open(api_path, 'r') as f:
        source = f.read()
    top_level, from_imports = _parse_imports(source)
    assert "llm_client" not in top_level, f"api.py importiert llm_client (sollte nur in api_llm.py sein)"
    assert "llm_client" not in from_imports, f"api.py hat 'from llm_client' (sollte nur in api_llm.py sein)"


def test_llm_client_only_in_api_llm():
    """NUR api_llm.py darf llm_client importieren."""
    api_llm_path = os.path.join(
        os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
        "produkt", "haut", "api_llm.py"
    )
    with open(api_llm_path, 'r') as f:
        source = f.read()
    top_level, from_imports = _parse_imports(source)
    # api_llm.py MUSS llm_client haben (oder lazy importieren)
    assert "llm_client" in top_level or "llm_client" in from_imports, \
        f"api_llm.py sollte llm_client importieren"


def test_no_llm_client_in_other_haut_modules():
    """Keine anderen Haut-Module (ausser den AUSNAHMEN) duerfen llm_client importieren.

    Frueher stand hier eine hartkodierte Liste dreier Namen (api_engines.py, api_guards.py,
    api_helpers.py), von denen KEINER je existierte -- jede Iteration traf sofort auf
    `continue`, der Schleifenrumpf mit den eigentlichen Zusicherungen lief nie. Der Test war
    seit seiner Erstellung gruen, unabhaengig davon, was irgendein Haut-Modul importierte.
    Jetzt wird produkt/haut/ per Verzeichnislisting durchlaufen (siehe _verstoesse); die
    Selbstpruefung unten stellt sicher, dass diese Menge nicht wieder leerlaufen kann, ohne
    dass es auffaellt.
    """
    treffer = _verstoesse(HAUT_DIR, AUSNAHMEN)
    assert treffer == [], (
        "Module unter produkt/haut/ importieren llm_client, obwohl sie nicht in AUSNAHMEN "
        f"stehen (sollte nur in api_llm.py sein): {treffer}"
    )


def test_scan_menge_ist_nicht_leer_und_enthaelt_reale_module():
    """Selbstpruefung des Pruefers: die Menge, die _verstoesse tatsaechlich durchlaeuft, darf
    nicht leer sein und muss mindestens die zum Zeitpunkt dieses Tests bekannten realen
    Haut-Module enthalten. Genau diese Zusicherung fehlte beim alten Test -- eine leere
    Scan-Menge (dort: drei tote Namen) macht die Zusicherung darueber wertlos, ohne dass die
    Suite es zeigt."""
    gescannt = {n for n in os.listdir(HAUT_DIR) if n.endswith(".py")}
    UNTERGRENZE = 8  # gemessen 2026-09-08: 9 reale .py-Dateien unter produkt/haut/
    assert len(gescannt) >= UNTERGRENZE, (
        f"Nur {len(gescannt)} .py-Module unter produkt/haut/ gefunden, erwartet mindestens "
        f"{UNTERGRENZE} -- die Scan-Menge ist unerwartet klein geworden"
    )
    bekannte_module = {"api.py", "api_llm.py", "llm_client.py", "server.py"}
    fehlend = bekannte_module - gescannt
    assert not fehlend, f"Bekannte Haut-Module fehlen in der Scan-Menge: {sorted(fehlend)}"


def test_ausnahmen_zeigen_auf_existierende_dateien():
    """Jede AUSNAHME muss auf eine tatsaechlich existierende Datei zeigen. Eine Ausnahme fuer
    eine verschwundene Datei ist derselbe Fehler wie vorher (tote Namen in einer Liste), nur
    kleiner -- sie wuerde niemanden mehr von der Pruefung ausnehmen, aber auch niemand merkt,
    dass die Ausnahme selbst ins Leere zeigt."""
    fehlend = sorted(n for n in AUSNAHMEN if not os.path.isfile(os.path.join(HAUT_DIR, n)))
    assert not fehlend, f"AUSNAHMEN-Eintraege ohne existierende Datei: {fehlend}"


def test_verstoesse_findet_neuen_import_unter_tmp_path(tmp_path):
    """Rot-Gruen ohne Eingriff in den geteilten Baum: _verstoesse bekommt ein FRISCHES
    Verzeichnis unter tmp_path (nicht produkt/haut/) mit einem Modul, das llm_client
    importiert, und muss es finden. Ein sauberes Nachbarmodul im selben Verzeichnis bleibt
    unauffaellig -- Gegenprobe, dass der Fund nicht aus dem Verzeichnis an sich kommt."""
    (tmp_path / "boese_datei.py").write_text("import llm_client\n")
    (tmp_path / "saubere_datei.py").write_text("import os\n")
    treffer = _verstoesse(str(tmp_path), ausnahmen=set())
    assert treffer == ["boese_datei.py"], (
        f"_verstoesse haette genau ['boese_datei.py'] finden muessen, fand {treffer}"
    )
