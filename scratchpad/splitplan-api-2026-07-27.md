# Split-Plan api.py / runner.py — 2026-07-27 (Task 4)

Von dev-2 analysiert, Instructor-gebilligt mit Auflagen. AUSFÜHRUNG erst NACH Bug 2
committed + volle Suite grün. INKREMENTELL, jeder Schritt einzeln durch volle Suite gegatet.

## Reihenfolge (risiko-aufsteigend, je Schritt eigener Commit + volle Suite grün)

1. **api_constants.py** (~290 LOC) — reine Tupel/Pfad-Konstanten (HAUSHALT_35A, GESAMT_ABZUEGE,
   EP_FELDER, RENTNER_*, KIST_*, ABS3_*, FAELLE, SCHEMA_DIR ...). Niedrigstes Risiko (reine Daten).
   api.py: `from api_constants import *` ODER `import api_constants as CONST`. Gate: volle Suite grün.
2. **api_llm.py** (~130 LOC) — BRIGHT-LINE: chat(), _chat_prompt/_parse, _llm_vorschlaege,
   _kontoauszug_llm_klassifikator. llm_client NUR hier (lazy import intra-function).
   HÖCHSTER WERT (legalHelper-Grenze). Danach Guard-Test schreiben:
   "Berechnungspfad importiert keinen llm_client" (grep api_engines/api_helpers/api_guards).
3. **api_guards.py** (~280 LOC) — ApiError, _abs3_eligible, _an_gesamt_sperrgrund. Self-contained.
4. **api_helpers.py** (~135 LOC) — lade_fall/speichere_fall/_cfg/_scheibe_*/_badge/_abschlusszahlung_cent.
5. **api_engines.py** — _bescheid_fn (1078 LOC Monolith) + _laufender_gewinn/_p23/_gwg. Höchstes Risiko.
   OPTIONAL/zuletzt; kann auch monolithisch in api.py bleiben wenn Risiko/Nutzen schlecht.
6. **api_endpoints.py** — HTTP-Handler (fall_anlegen/fragen/stand/event/ergebnis/graph/...).

## Auflagen (Instructor)
- **Zirkuläre Imports vermeiden**: strikte Schichtung constants ← helpers ← guards/engines ← endpoints ← api.
  Kein Modul importiert ein „höheres". Bei Zirkel: STOP, an Instructor.
- **Verhaltensneutral**: keine Logikänderung, nur Verschiebung. Gate = volle Suite identisch grün vor/nach JEDEM Schritt.
- **server.py-Imports**: server.py importiert `api` — der Public-Name muss stabil bleiben
  (api.py re-exportiert die Endpunkte: `from api_endpoints import *`), sonst bricht server.py.
- **Ein Commit pro Schritt**, reviewbar. KEIN push.
- Wenn ein Schritt Nutzen < Risiko (v.a. Schritt 5 _bescheid_fn): auslassen, an Instructor melden.

## runner.py (1521 LOC) — zweite Phase, nach api.py
Accessor-Cluster nach §-Gruppen (p10*, p35*, dba/p34c, gewinn §§13-18, tarif). Gleiche Regeln.
Niedrigeres Risiko (Accessoren sind großteils unabhängige Funktionen). Separat planen wenn api.py steht.
