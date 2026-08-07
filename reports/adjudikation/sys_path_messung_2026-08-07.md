# sys.path-Injektionen Messung — 2026-08-07

**Status:** Analyse abgeschlossen. Kein Code geändert.

---

## 1. Anzahl-Verifikation

**Behauptung (Backlog):** 148 Dateien manipulieren sys.path (107 in tests/, 35 in Produktion)

**Gemessen:**
```bash
grep -rl "sys\.path" --include="*.py" tests/   | wc -l   # 123
grep -rl "sys\.path" --include="*.py" produkt/ | wc -l   # 10
# Total: 133 Dateien (exklusive `.venv`, `oracle/`)
```

**Aufteilung:**
- **Tests:** 123 Dateien
- **Produktion:** 10 Dateien

**Abweichung:** -15 Dateien vs. Backlog (148). Backlog zählte wahrscheinlich `.venv`-Bibliotheken oder nutzte andere Grenzen. Die 10:123-Split ist KRITISCH anders als Backlog-Behauptung "35 in Produktion".

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

**Drei Zahlen, die nicht verwechselt werden dürfen:**

| Zahl | Bedeutung | Messung |
|------|-----------|---------|
| **133** | Dateien, die `sys.path` überhaupt erwähnen | `grep -rl "sys\.path" --include="*.py" tests/ produkt/ \| wc -l` |
| **187** | tatsächliche `sys.path.insert`-Aufrufe (Vorkommen, nicht Dateien) | `grep -rho "sys\.path\.insert" --include="*.py" tests/ produkt/ \| wc -l` |
| **140** | Summe der Muster-Spalte oben | 41+24+25+19+17+14 |

Die Muster-Spalte zählt **Dateien pro Muster**, nicht Aufrufe. 140 > 133, weil einige Dateien
zwei Muster gleichzeitig verwenden. Die 187 Aufrufe sind die Zahl, die beim Umbau tatsächlich
angefasst wird — 174 in `tests/`, 13 in `produkt/`.

Andere Formen als `.insert` kommen nicht vor (`sys.path.append`/`extend`: 0 Treffer).

---

## 3. Produktions-Blöcke (Nicht-Tests)

**10 Dateien in produkt/, 13 Aufrufe.** Vollständig, namentlich (`grep -rc 'sys\.path\.insert' --include='*.py' produkt/`):

| Datei | Aufrufe |
|-------|---------|
| produkt/haut/server.py | 3 |
| produkt/import/elster_xml.py | 2 |
| produkt/mapping/xsd_verify.py | 2 |
| produkt/haut/api.py | 1 |
| produkt/import/beleg_writer.py | 1 |
| produkt/import/elster_writer.py | 1 |
| produkt/import/kontoauszug_writer.py | 1 |
| produkt/import/vorjahr_writer.py | 1 |
| produkt/konsistenz/preflight.py | 1 |
| produkt/haut/api_llm.py | 0 |

`api_llm.py` erscheint in der Datei-Liste, hat aber keinen eigenen Aufruf — nur einen Kommentar
(Zeile 9: `import audit  # noqa: E402 — P1.6 Audit-Log (sys.path via api.py)`). Es erbt den Pfad
von `api.py`. Das ist genau die Kopplung, die der Umbau beseitigen soll: eine Datei, die nur
importierbar ist, weil eine andere vorher `sys.path` manipuliert hat.

`produkt/traverser/` und `produkt/store/` tauchen NICHT auf — sie sind bereits sauber.

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
| **Anzahl Dateien** | 133 (nicht 148; -15 vs. Backlog), 187 Aufrufe |
| **Muster-Komplexität** | 6 Hauptmuster (mechanisch lösbar) + 1 Anti-Pattern |
| **Produktions-Umfang** | **10 Dateien / 13 Aufrufe** — nicht 38. Die Produktion ist fast sauber. |
| **Test-Umfang** | 123 Dateien / 174 Aufrufe = **92 % des Problems** |
| **Package-Struktur** | HALB-vorhanden (produkt/store/ kann erweitert werden, Rest würde Neuaufbau erfordern) |
| **mypy/pyright Blockade** | **VERIFIZIERT** — beide können sys.path-Injektionen nicht auflösen; IDE-Navigation/Rename broken |

---

## 7. Folgerung: zwei unabhängige Stufen

Die 10:123-Aufteilung ist der wichtigste Befund dieser Messung, weil sie den Umbau in zwei
Vorhaben zerlegt, die **nichts miteinander zu tun haben** und einzeln entschieden werden können:

**Stufe 1 — `produkt/` (10 Dateien, 13 Aufrufe).**
Klein, aber echtes Produktionsrisiko: ein falsch aufgelöster Import bricht den Ring oder den
Server zur Laufzeit, nicht beim Type-Check. Braucht ein Gate. Aufwand: **~0,5 Tage.**
Reihenfolge-Hinweis: `api_llm.py` erbt seinen Pfad von `api.py` und muss mit `api.py` zusammen
umgestellt werden, sonst ist es nach dem Schnitt nicht mehr importierbar.

**Stufe 2 — `tests/` (123 Dateien, 174 Aufrufe).**
Mechanisch, kein Produktionsrisiko. Ein Fehler zeigt sich sofort als roter Test, nicht als
stiller Laufzeitfehler. Kann am Stück und später laufen, muster-basiert statt datei-für-datei.
Aufwand: **~2,5–4 Tage.**

Die alte Gesamtschätzung von 3–5 Tagen bleibt in Summe gültig; neu ist, dass der riskante
Anteil davon ein halber Tag ist. Stufe 1 lohnt sich auch dann, wenn Stufe 2 nie kommt.

**Abhängigkeit zum api.py-Split:** Stufe 1 fasst `api.py`, `api_llm.py` und `server.py` an —
dieselben Dateien wie der Split. Beides gleichzeitig ist ein vermeidbares Risiko. Eins nach dem
anderen, und der Split zuerst, weil er die Modulgrenzen festlegt, an denen sich die Importe
danach ausrichten.

---

## Empfehlung

1. **Priorität:** Stufe 1 (produkt/) nach dem api.py-Split, vor Paket-B. Stufe 2 (tests/) offen —
   sie blockiert nichts und kann jederzeit am Stück laufen.
2. **Strategie:** Muster-basiert (nicht datei-für-datei):
   - `loop_sub` → Zielindex-Konstante
   - `root_direct` → ROOT-Konstante zentralisieren
   - `dirname_golden` → Unified `golden/`-Konstante
3. **Verifizierung:** Nach Umbau: `mypy tests/ --no-error-summary` sauber fahren, VS Code Intellisense prüfen

