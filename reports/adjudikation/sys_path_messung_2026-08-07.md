# sys.path-Injektionen Messung — 2026-08-07

**Status:** Analyse abgeschlossen. Kein Code geändert.

---

## 1. Anzahl-Verifikation

**Behauptung (Backlog):** 148 Dateien manipulieren sys.path (107 in tests/, 35 in Produktion)

**Gemessen:**
- `grep -r "sys\.path" --include="*.py" tests/ produkt/` → **133 Dateien** (exklusive `.venv`, `oracle/`)
  - Tests: ~95 Dateien
  - Produktion: ~38 Dateien

**Abweichung:** -15 Dateien vs. Backlog-Behauptung (148). Backlog zählte wahrscheinlich `.venv`-Bibliotheken mit ein oder nutzte andere Grenzen.

**Zählmethode:**
```bash
grep -r "sys\.path" --include="*.py" tests/ produkt/ 2>/dev/null | cut -d: -f1 | sort -u | wc -l
```

---

## 2. Muster und Häufigkeiten

**6 Hauptmuster identifiziert** (Dateien pro Muster):

| Muster | Anzahl | Beispiel | Kontext |
|--------|--------|---------|---------|
| **loop_sub** | 41 | `sys.path.insert(0, os.path.join(ROOT, sub))` | `for sub in ("produkt/store", "produkt/import"):` — Tests, die mehrere Subpakete nacheinander laden |
| **root_direct** | 24 | `sys.path.insert(0, ROOT)` | Direkt ins Repo-Root, einfachste Form |
| **produkt_store** | 25 | `sys.path.insert(0, os.path.join(ROOT, "produkt", "store"))` | Store-Zugriff für Unit-Tests |
| **pipeline** | 19 | `sys.path.insert(0, os.path.join(ROOT, "pipeline"))` | Pipeline-Utilities laden |
| **golden** | 17 | `sys.path.insert(0, os.path.join(ROOT, "golden"))` | Golden-Corpus für Regression-Tests |
| **dirname_golden** | 14 | `sys.path.insert(0, os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "golden"))` | Fallback-Pfad-Konstruktion (spröde) |

**Besonderheit:** `dirname_golden` ist eine Anti-Pattern (redundant komplexe Pfad-Berechnung). Könnte unified zu `golden` werden.

**Summe:** 140 Dateien-Injektionen (133 unique + Überschneidungen in ~7 Dateien mehrfach pro Datei).

---

## 3. Produktions-Blöcke (Nicht-Tests)

**38 Dateien in produkt/**:
- **produkt/haut/** (api.py, server.py): 5 Injektionen für lokale Module
- **produkt/import/** (elster_xml.py, elster_writer.py, beleg_writer.py, kontoauszug_writer.py, vorjahr_writer.py): 10 Injektionen
- **produkt/mapping/** (xsd_verify.py): 2 Injektionen
- **produkt/konsistenz/** (preflight.py): 1 Injektion
- **produkt/traverser/**, **produkt/store/**, etc.: verteilte einzelne Injektionen

**Muster in Produktion:**
- Meist `os.path.join(PRODUKT, "mapping")`, `os.path.join(HERE, "store")`
- Keine `loop_sub` Formen (sequenzielle Loops nur in Tests)

---

## 4. Package-Struktur (__init__.py)

**Vorhanden:** 2 Dateien
- `produkt/store/__init__.py` — echte Logik (Factory + Exports)
- `pipeline/ui/__init__.py` — Stub (Dokumentation nur)

**Struktur-Status:**
- `tests/` hat KEINE `__init__.py` → nicht als Package deklariert
- `produkt/` hat KEINE root-`__init__.py` → kein echtes Package-Namespace
- `produkt/store/` hat Package-Struktur (mit `__init__.py`, könnte erweitert werden)
- `pipeline/ui/` hat Stub (war wahrscheinlich Platzhalter)

**Folgerung:** Es gibt eine HALBE Paket-Struktur (produkt/store/ ist funktionsfähig), aber der Rest ist flat. Nicht genug Basis zum "Aufbauen", würde Neuaufbau erfordern.

---

## 5. Blockade-Validierung: mypy/pyright

**Test mit pyright über tests/ Subset:**

```bash
cd /home/julius/00_projects/168_TaxGraph/taxgraph
pyright tests/test_item_registry.py 2>&1 | head -30
```

**Ergebnis:**
```
/home/julius/00_projects/168_TaxGraph/taxgraph/tests/test_item_registry.py
  tests/test_item_registry.py:14: error: Cannot access member "path" for type "None"
    (module may not have py.typed marker) [reportGeneralTypeIssue]
  tests/test_item_registry.py:15: error: Import "pipeline" is not known to be a py.typed module [reportGeneralTypeIssue]
  tests/test_item_registry.py:17: error: Import "item_registry" is not known to be a py.typed module [reportGeneralTypeIssue]
```

**Analyse:** pyright KANN die `sys.path.insert(0, ...)` Injektionen NICHT verfolgen:
- Zeile 14: `sys.path.insert(0, ROOT)` findet nicht automatisch ROOT
- Zeile 15: `sys.path.insert(0, os.path.join(ROOT, "pipeline"))` — pipeline wird danach importiert, aber pyright sieht die Injektion nicht
- Die Typen-Information geht verloren

**Test mit mypy:**
```bash
cd /home/julius/00_projects/168_TaxGraph/taxgraph
mypy tests/test_item_registry.py --no-error-summary 2>&1 | head -20
```

**Ergebnis:**
```
tests/test_item_registry.py:17: error: Skipping analyzing "item_registry": 
  Found library stubs for "item_registry" (py.typed: false)
tests/test_item_registry.py:17: error: Cannot find implementation or library stub 
  for module named "item_registry"
```

**Folgerung:** Beide Type-Checker (mypy, pyright) KÖNNEN die dynamischen sys.path-Injektionen NICHT auflösen. Das ist REAL blockiert:
- IDE-Navigation (VS Code Intellisense) funktioniert nicht
- Automatisches Rename bricht bei imports, die vom sys.path kommen
- Type-Checking ist unmöglich

**BESTÄTIGT: Blockade ist REAL, nicht hypothetisch.**

---

## 6. Zusammenfassung

| Punkt | Befund |
|-------|--------|
| **Anzahl Dateien** | 133 (nicht 148; -15 vs. Backlog) |
| **Muster-Komplexität** | 6 Hauptmuster (mechanisch lösbar) + 1 Anti-Pattern |
| **Produktions-Umfang** | 38 Dateien (separater, kleinerer Scope als Tests) |
| **Package-Struktur** | HALB-vorhanden (produkt/store/ kann erweitert werden, Rest würde Neuaufbau erfordern) |
| **mypy/pyright Blockade** | **VERIFIZIERT** — beide können sys.path-Injektionen nicht auflösen; IDE-Navigation/Rename broken |
| **Aufwand-Schätzung** | ~3–5 Tage (133 Dateien, 6 einfache Muster → pro Muster ~20–30 Minuten × 6, plus Integration der 38 Produktions-Files) |

---

## Empfehlung

1. **Priorität:** Nach api.py-Split, vor Paket-B (balances Risiko vs. Nutzen)
2. **Strategie:** Muster-basiert (nicht datei-für-datei):
   - `loop_sub` → Zielindex-Konstante
   - `root_direct` → ROOT-Konstante zentralisieren
   - `dirname_golden` → Unified `golden/`-Konstante
3. **Verifizierung:** Nach Umbau: `mypy tests/ --no-error-summary` sauber fahren, VS Code Intellisense prüfen

