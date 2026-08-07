# Importeur-Inventar api.py — Voraussetzung 2 für api-py-datei-split

**Datum:** 2026-08-07
**Auftrag:** BACKLOG.yaml `api-py-datei-split`, Blockade 2 ("Modul-Globals: 6 zählt Definitionen, nicht Importeure")
**Methode:** AST-Parse `produkt/haut/api.py` (ast.Assign/AnnAssign, top-level, dedupliziert gegen Reassignment
anywhere im File) + AST-Scan Repo-weit auf `ast.ImportFrom`/`ast.Import` mit Ziel `api`/`api_auth`/`api_constants`/
`produkt.haut.api*`. Gegen grep verifiziert (Aliasse `import api as API` mitgenommen).

## 1) Modul-Globals von api.py — Trennung neu zugewiesen vs. einmal definiert

AST fand 6 top-level `Assign`-Ziele in api.py:

| Name | Zeile | Neu zugewiesen irgendwo im File? |
|------|-------|-----------------------------------|
| `HERE` | 25 | Nein — genau 1 Assign im ganzen File |
| `PRODUKT` | 26 | Nein — genau 1 Assign |
| `ROOT` | 27 | Nein — genau 1 Assign |
| `CHAT_501` | 2430 | Nein — genau 1 Assign |
| `AMPEL_503` | 2437 | Nein — genau 1 Assign |
| `ENTFERNUNG_FALLBACK` | 2447 | Nein — genau 1 Assign |

Trennkriterium: AST-Walk über den ganzen Modulbody nach jedem `Assign`/`AugAssign`-Ziel mit diesem Namen — nicht
nur top-level. Kein `global`-Statement in irgendeiner Funktion von api.py (0 Treffer). Ergebnis: **alle 6
Modul-Globals in api.py selbst sind harmlos** — Konstanten, kein Laufzeit-Rebind.

Der gefährliche Fall (`_AUTH_USER`) liegt **nicht** in api.py, sondern in `api_auth.py:8`. api.py importiert
`api_auth` als Modul (`import api_auth`, Zeile 46) und liest/schreibt ausschließlich `api_auth._AUTH_USER` —
nie ein nackter Name. Die riskante Zuweisung passiert extern in `server.py:153,170,180` (`api_auth._AUTH_USER = …`).
Das ist der Musterfall aus fffd7c8: Modul-Attribut, kein Name-Import — und genau das macht ihn sicher.

## 2) + 3) Importeure je Name

### `_AUTH_USER` (api_auth.py:8) — die eine bekannte Bug-Bauart

| Importeur-Datei:Zeile | Form | Wird nach Import neu zugewiesen? | Risiko |
|---|---|---|---|
| `produkt/haut/server.py:30` | `import api_auth` (Modul-Attribut) | Ja — `api_auth._AUTH_USER = ...` (L153, L180) | **sicher** — Attribut-Zugriff löst jedes Mal neu auf |
| `produkt/haut/api.py:46` | `import api_auth` (Modul-Attribut) | Nein (nur Lesezugriff) | sicher |
| `tests/test_auth_naht_mutation.py:30,65,91,130` | `import api_auth` (Modul-Attribut, 4× im File) | Ja — Test setzt `api_auth._AUTH_USER = "..."` gezielt, um genau diese Naht zu prüfen | sicher (ist der Regressionstest für den Bug) |
| `tests/test_auth_integration.py`, `test_auth_comprehensive.py` (5×), `test_auth_security.py`, `test_fall_loeschen.py` (6×) | `monkeypatch.setattr(api_auth, "_AUTH_USER", …)` | Ja, kontrolliert über monkeypatch | sicher — monkeypatch.setattr ist Attribut-Ebene, kein Name-Bind |

Kein einziger Treffer für `from api_auth import _AUTH_USER` oder `from api import _AUTH_USER` im Repo (Tests
zitieren die Zeichenkette nur in Docstrings/Kommentaren als Negativ-Beispiel, das explizit NICHT verwendet wird).

### `HERE`, `PRODUKT`, `ROOT`, `CHAT_501`, `AMPEL_503`, `ENTFERNUNG_FALLBACK`

**Kein Treffer.** Repo-weiter AST-Scan (inkl. tests/) auf `from api import <Name>` oder
`from produkt.haut.api import <Name>` für diese 6 Namen: 0 Zeilen. Auch kein `import api as X; X.<Name> = ...`
gefunden, das einen dieser Namen neu zuweist. Alle Zugriffe (`server.py:79 api.AMPEL_503`) sind reiner
Lesezugriff über Modul-Attribut, nicht Re-Assignment.

## 4) Sichere Form: `import api` + `api.<name>`-Zugriff — Bestand

| Muster | Anzahl |
|---|---|
| `import api as API` (Tests, Modul-Attribut-Zugriff) | 37 Dateien |
| `import api` (server.py + api.py selbst + Tests, oft lokal in Testfunktion) | 8 weitere Stellen (Tests, lokal importiert) |
| `monkeypatch.setattr(API, "FAELLE", …)` — FAELLE kommt via `api_constants import *` in api.py-Namespace, Reassignment ausschließlich über Attribut | 32 Testdateien |
| `monkeypatch.setattr(api_auth, "_AUTH_USER", …)` | 8 Stellen (siehe oben) |

**Gesamt: 45 Importstellen von `api`/`api_auth`, alle als Modul-Objekt (`import X` + `X.attr`), 0 als
`from X import name`.** Die Codebase hat die aus fffd7c8 gelernte Regel bereits durchgängig angewendet — nicht
nur an der einen entschärften Stelle.

## Befund für die Fassade

Da kein `from api import <Name>` existiert, muss die künftige Fassade `api.py` (nach Split in `api_ring.py` +
`api_endpunkte.py`) **keinen** der 6 Globals per Re-Export exponieren, um bestehende Importeure nicht zu brechen
— alle Aufrufer gehen über `import api` + Attributzugriff, was bei jedem Zugriff neu auf das jeweils aktuelle
Zielmodul aufgelöst werden kann, solange die Fassade weiterhin ein Attribut mit dem Namen trägt (egal ob
Konstante direkt oder Re-Export-Referenz auf `api_ring`/`api_endpunkte`). Die eigentliche Auth-Gefahrenklasse
(`_AUTH_USER`) liegt außerhalb von api.py in `api_auth.py` und ist unabhängig vom Split bereits durchgängig
Modul-Attribut-only.

## Ergebnis in einem Satz

6 Modul-Globals in api.py, 0 davon werden im Repo per `from api import <Name>` gebunden — die einzige real
existierende Instanz der `_AUTH_USER`-Bug-Bauart ist die bereits bekannte und bereits entschärfte
(`api_auth._AUTH_USER`, ausschließlich Modul-Attribut-Zugriff, 45 Importstellen alle sicher).
