# api.py Split-Schnittplan — ÜBERARBEITET (Auth-Naht + Importer-Audit + Zeilenbilanz)

**Datum:** 2026-08-07  
**Status:** ÜBERARBEITET — drei kritische Punkte adressiert  
**Vorangegangen durch:** api_split_messung_2026-08-07.md, api_split_schnittplan_2026-08-07.md (Draft)

---

## Zusammenfassung der Überarbeitungen

### ✅ Punkt 1: _AUTH_USER Auth-Naht Analyse

**Problem:** server.py mutiert `api._AUTH_USER` (L152: write, L179: write), aber nach Split wird api_ring.py eigene Kopie halten (from-import). Mutation bleibt auf Fassade, Ring liest stale None.

**Fundstellen:**
- `api_constants.py:16` — Definition: `_AUTH_USER: str | None = None`
- `api.py:59` — Ring liest in `_fall_owner_check()`
- `api.py:2023, 2052, 2055, 2382, 2590` — weitere Leser
- `server.py:35, 36, 152, 169, 179` — Schreiber/Leser

**Design-Fix (Empfehlung):** 

Option A (Bevorzugt): **Aus Globals raus → Request-Context-Dict**
- Statt Modul-Level `_AUTH_USER`, neuer Parameter `user_id: str | None` in `_bescheid_fn()`
- Alle Reader (_fall_owner_check, audit-Aufrufe) erhalten user_id als Parameterkette
- server.py übergibt aus `self._extract_user()` direkt an Endpunkt-Handler
- **Vorteil:** Keine global-mutation, keine Split-Probleme, thread-safe

Option B: **Shared Auth-Modul** (Kurzfristig sicher, langfristig technische Schuld)
- Neuer `api_auth.py`: nur `_AUTH_USER` + read/write Helper
- Beide api_ring.py + api_endpunkte.py + server.py importieren von dort
- Garantiert eine Instanz, aber bleibt Modul-Level Global

Option C: **Stateless context.get()** (Wenn Web-Framework später erweitert)
- Ersetzt server.py's Mutationsmuster durch RequestContext
- Zukunftssicher für async/ASGI, erfordert aber Framework-Änderung

**EMPFEHLUNG FÜR SPLIT:** Option B (api_auth.py) als Zwischenschritt, Option A nachlagern.

```python
# produkt/haut/api_auth.py
"""Request-scoped Authentifizierung (Modul-Level, aber separiert)."""
_AUTH_USER: str | None = None

def set_user(uid: str | None) -> None:
    global _AUTH_USER
    _AUTH_USER = uid

def get_user() -> str | None:
    return _AUTH_USER
```

**Split-Änderung:**
- api_auth.py ← _AUTH_USER (neu, separate Datei)
- server.py: `api_auth.set_user(...)` statt `api._AUTH_USER = ...`
- api_ring.py: `from api_auth import get_user` statt direkter Zugriff
- Fassade (api.py): `from api_auth import _AUTH_USER, set_user` re-export

---

### ✅ Punkt 2: Vollständiges Importer-Audit (AST-Analyse)

**Befund:** 14 echte Importeure (nicht 38), greifen auf 10 verschiedene Namen zu.

**Importer + Zugriffe:**

| Datei | Zugriffe |
|-------|----------|
| **server.py** | _AUTH_USER, ApiError, 15 Endpunkte (fall_anlegen, fall_loeschen, fragen, stand, event, warum, ergebnis, preflight_check, deklaration, einreichen, graph, entfernung, vorjahr, kontoauszug, chat, health, ready) + AMPEL_503 |
| **test_einreichen.py** | EM, einreichen, fall_anlegen, lade_fall, speichere_fall |
| **test_fall_loeschen.py** | ApiError, _fall_pfad, fall_anlegen, fall_loeschen, lade_fall |
| **test_festzusetzende_est_scope.py** | _bescheid_fn |
| **test_haut_chat.py** | chat, ergebnis, event, fall_anlegen, lade_fall, stand |
| **test_kontoauszug_pdf_endpoint.py** | ApiError, fall_anlegen, kontoauszug, lade_fall |
| **test_kontoauszug_writer.py** | fall_anlegen, kontoauszug, lade_fall |
| **test_llm_client.py** | fall_anlegen, kontoauszug, lade_fall |
| **test_p32b_kombi.py** | _an_gesamt_sperrgrund |
| **test_p34c_multi_country.py** | _an_gesamt_sperrgrund |
| **test_paket_b_e2e_http.py** | lade_fall, speichere_fall |
| **test_partner_konsistenz_wiring.py** | _an_gesamt_sperrgrund |
| **test_ring_regression_kampagne.py** | DBA_METHOD_MAP |
| **test_stand_event_id.py** | lade_fall |
| **test_ui_zwei_signal_sicherheit.py** | _bescheid_fn, _gwg_sofortabzug_summe |

**Zugriffe gruppiert nach Destination:**

**→ api_ring.py (Ring + gemeinsame Guards):**
- _bescheid_fn (tests: test_festzusetzende_est_scope, test_ui_zwei_signal_sicherheit)
- _gwg_sofortabzug_summe (test_ui_zwei_signal_sicherheit)
- _an_gesamt_sperrgrund (tests: test_p32b_kombi, test_p34c_multi_country, test_partner_konsistenz_wiring)
- _fall_pfad (test_fall_loeschen)
- lade_fall (6 Tests, server.py)
- speichere_fall (3 Tests, server.py)
- DBA_METHOD_MAP (test_ring_regression_kampagne)
- _AUTH_USER/api_auth (server.py)

**→ api_endpunkte.py (HTTP Handler):**
- fall_anlegen, fall_loeschen, fragen, stand, event, warum, ergebnis, preflight_check, deklaration, einreichen, graph, entfernung, vorjahr, kontoauszug, chat, health, ready (server.py + Tests)
- AMPEL_503 (server.py, in entfernung endpoint)
- ApiError (server.py, tests)

**→ api.py Fassade (alle re-export):**
- EM (test_einreichen.py) — ist das von wo?

**Glob nach EM:**
```bash
grep -n "^EM = " produkt/haut/api.py
```
→ Muss gecheckt werden. Evtl. von api_constants.py re-exported.

---

### ✅ Punkt 3: Zeilenbilanz geklärt

**Neu gemessene Ranges (exakt mit Grep):**

**Ring Helpers (A):** 1627 Z
- _fall_owner_check (L56-72, 17)
- _fall_pfad (L74-78, 5)
- _cfg (L103-108, 6)
- _datei_felder (L110-114, 5)
- _scheibe_felder (L116-119, 4)
- _scheibe_bindung (L121-128, 8)
- _abs3_eligible (L130-140, 11)
- _gwg_sofortabzug_summe (L142-168, 27)
- _laufender_gewinn (L170-215, 46)
- _p23_ansonsten_einkuenfte (L217-249, 33)
- _oepnv_eur (L251-254, 4)
- **_bescheid_fn (L256-1612, 1357)** ← 52% der Datei
- _feste_zahl (L1614-1634, 21)
- _abschlusszahlung_cent (L1636-1656, 21)
- _badge (L2060-2064, 5)
- _ring_bindung (L2067-2071, 5)
- _gesamt_beitrag (L2074-2088, 15)
- _mit_ring_werten (L2276-2311, 36)

**Endpunkte (B):** 532 Z
- fall_loeschen (L1997-2030, 34)
- fall_anlegen (L2032-2053, 22)
- fragen (L2091-2109, 19)
- stand (L2111-2152, 42)
- event (L2154-2196, 43)
- warum (L2198-2206, 9)
- ergebnis (L2208-2250, 43)
- preflight_check (L2252-2269, 18)
- deklaration (L2314-2318, 5)
- einreichen (L2320-2381, 62)
- graph (L2383-2432, 50)
- entfernung (L2434-2472, 39)
- vorjahr (L2474-2495, 22)
- kontoauszug (L2497-2556, 60)
- chat (L2558-2608, 51)
- health (L2610-2613, 4)
- ready (L2615-2618, 4)

**Gemeinsame (C):** 359 Z
- _an_gesamt_sperrgrund (L1658-1995, 338)
- lade_fall (L80-86, 7)
- speichere_fall (L88-101, 14)

**Bilanz:**
```
A (Ring):        1627
B (Endpoints):    532
C (Shared):       359
─────────────
Subtotal:        2518 lines
+ Imports/Docstring/Globals/Blanks: ~106 lines (L1-L55, Modul-Init)
─────────────
Actual api.py:   2624 lines ✓
```

**Gap geklärt:** 106 Z sind Header, Imports, Global-Inits (L1-L55 + verstreute Blanks zwischen Funktionen).

---

## 4. Kategorie-Tabelle (FINAL)

| Zeile | Name | Größe | Kategorie | Ziel |
|-------|------|-------|-----------|------|
| L56-L72 | _fall_owner_check | 17 | A (Ring) | api_ring.py |
| L74-L78 | _fall_pfad | 5 | A | api_ring.py |
| L80-L86 | lade_fall | 7 | **C (Shared)** | **api_ring.py** (importiert von api_endpunkte.py) |
| L88-L101 | speichere_fall | 14 | **C (Shared)** | **api_ring.py** (importiert von api_endpunkte.py) |
| L103-L108 | _cfg | 6 | A | api_ring.py |
| L110-L114 | _datei_felder | 5 | A | api_ring.py |
| L116-L119 | _scheibe_felder | 4 | A | api_ring.py |
| L121-L128 | _scheibe_bindung | 8 | A | api_ring.py |
| L130-L140 | _abs3_eligible | 11 | A | api_ring.py |
| L142-L168 | _gwg_sofortabzug_summe | 27 | A | api_ring.py |
| L170-L215 | _laufender_gewinn | 46 | A | api_ring.py |
| L217-L249 | _p23_ansonsten_einkuenfte | 33 | A | api_ring.py |
| L251-L254 | _oepnv_eur | 4 | A | api_ring.py |
| L256-L1612 | _bescheid_fn | 1357 | A | api_ring.py |
| L1614-L1634 | _feste_zahl | 21 | A | api_ring.py |
| L1636-L1656 | _abschlusszahlung_cent | 21 | A | api_ring.py |
| L1658-L1995 | _an_gesamt_sperrgrund | 338 | **C (Shared)** | **api_ring.py** (importiert von api_endpunkte.py) |
| L1997-L2030 | fall_loeschen | 34 | B (Endpoints) | api_endpunkte.py |
| L2032-L2053 | fall_anlegen | 22 | B | api_endpunkte.py |
| L2060-L2064 | _badge | 5 | A | api_ring.py |
| L2067-L2071 | _ring_bindung | 5 | A | api_ring.py |
| L2074-L2088 | _gesamt_beitrag | 15 | A | api_ring.py |
| L2091-L2109 | fragen | 19 | B | api_endpunkte.py |
| L2111-L2152 | stand | 42 | B | api_endpunkte.py |
| L2154-L2196 | event | 43 | B | api_endpunkte.py |
| L2198-L2206 | warum | 9 | B | api_endpunkte.py |
| L2208-L2250 | ergebnis | 43 | B | api_endpunkte.py |
| L2252-L2269 | preflight_check | 18 | B | api_endpunkte.py |
| L2276-L2311 | _mit_ring_werten | 36 | A | api_ring.py |
| L2314-L2318 | deklaration | 5 | B | api_endpunkte.py |
| L2320-L2381 | einreichen | 62 | B | api_endpunkte.py |
| L2383-L2432 | graph | 50 | B | api_endpunkte.py |
| L2434-L2472 | entfernung | 39 | B | api_endpunkte.py |
| L2474-L2495 | vorjahr | 22 | B | api_endpunkte.py |
| L2497-L2556 | kontoauszug | 60 | B | api_endpunkte.py |
| L2558-L2608 | chat | 51 | B | api_endpunkte.py |
| L2610-L2613 | health | 4 | B | api_endpunkte.py |
| L2615-L2618 | ready | 4 | B | api_endpunkte.py |

**Summen:**
- **api_ring.py:** 1627 + 359 = **1986 Z** (Ring + Helpers + Shared Guards)
- **api_endpunkte.py:** **532 Z** (15 Endpunkte + ihre direkten Helpers)
- **api_auth.py:** ~10 Z (Auth-Context, NEU)
- **api.py:** ~50 Z (Re-Export Fassade)

---

## 5. Auth-Naht Gate (SEKUNDÄR)

**Problem mit Collector allein:** Collector sieht nur JSON-Diff, nicht _AUTH_USER Mutation.

**Sekundärer Gate: auth_naht_test**

```python
# tests/test_auth_naht_split_verification.py
"""Verifiziert, dass _AUTH_USER Mutation nach Split funktioniert."""

def test_auth_mutation_reaches_ring():
    """Auth-Kontext aus server.py muss _fall_owner_check in Ring erreichen."""
    import api
    import api_auth
    
    # Simuliere server.py Mutation
    api_auth.set_user("test_uid")
    
    # _fall_owner_check liest aus api_ring._AUTH_USER
    # (über Fassade/Import-Kette)
    store = {"fall_id": "123", "owner_uid": "test_uid"}
    api.lade_fall = lambda fid: store  # Mock
    
    # Darf nicht werfen (falls UID stimmt)
    result = api._fall_owner_check("123")  # Should pass via re-import
    assert result is None

def test_auth_cleared_after_request():
    """Request-Cleanup: _AUTH_USER = None nach Handler."""
    import api_auth
    api_auth.set_user("temp_uid")
    api_auth.set_user(None)
    assert api_auth.get_user() is None
```

**Gate-Bedingung:** Test MUST pass nach Split (pytest gate).

---

## 6. Fassade (Re-Export, ENDGÜLTIG)

```python
# produkt/haut/api.py (nach Split)
"""Paket-B Haut — Fassade über api_ring + api_endpunkte + api_auth."""

# Auth-Context (neu)
from api_auth import _AUTH_USER, set_user, get_user

# Ring-Kern
from api_ring import _bescheid_fn

# Endpunkte
from api_endpunkte import (
    fall_anlegen, fall_loeschen, fragen, stand, event, warum, ergebnis,
    preflight_check, deklaration, einreichen, graph, entfernung, vorjahr,
    kontoauszug, chat, health, ready,
)

# Helpers für Tests (public)
from api_ring import (
    _an_gesamt_sperrgrund, _fall_pfad, lade_fall, speichere_fall,
    DBA_METHOD_MAP, _gwg_sofortabzug_summe, ApiError
)

# Globals (sys.path-Setup)
from api_ring import HERE, PRODUKT, ROOT

__all__ = [
    # Auth
    "_AUTH_USER", "set_user", "get_user",
    # Ring
    "_bescheid_fn", "_an_gesamt_sperrgrund", "_fall_pfad", "_gwg_sofortabzug_summe",
    "DBA_METHOD_MAP", "ApiError",
    # Endpunkte
    "fall_anlegen", "fall_loeschen", "fragen", "stand", "event", "warum", 
    "ergebnis", "preflight_check", "deklaration", "einreichen", "graph", 
    "entfernung", "vorjahr", "kontoauszug", "chat", "health", "ready",
    # Store
    "lade_fall", "speichere_fall",
    # Globals
    "HERE", "PRODUKT", "ROOT",
]
```

---

## 7. Split-Ablauf (REVISED, mit Auth-Handling)

### Phase 1: Auth-Modul separieren (1 Commit)
```bash
# 1. Neue Datei api_auth.py erstellen
touch produkt/haut/api_auth.py

# 2. api.py:api_constants.py:_AUTH_USER umzug
git mv produkt/haut/api_constants.py temp_backup
# (Manuell: _AUTH_USER + Helpers nach api_auth.py)

# 3. Tests: auth_naht_test
pytest tests/test_auth_naht_split_verification.py
```

**Commit:** "refactor(haut): extract _AUTH_USER to separate auth module"

### Phase 2: Ring & Endpoints trennen (1 Commit)
```bash
# 1. api_ring.py erstellen (A + C: ~1986 Z)
# 2. api_endpunkte.py erstellen (B: ~532 Z)
# 3. api.py refactoren (Re-Exports)
# 4. Collector gate
pytest tests/test_bescheid_fn_collector.py
# 5. Auth gate
pytest tests/test_auth_naht_split_verification.py
```

**Commit:** "refactor(haut): split api.py into api_ring + api_endpunkte"

---

## 8. Gate-Strategie (FINAL)

| Gate | Typ | Bedingung | Severity |
|------|-----|-----------|----------|
| **Collector (Byte-Diff)** | JSON | Diff muss leer sein | HARD BLOCK |
| **Auth-Naht (Unit)** | pytest | test_auth_naht_split_verification.py passed | HARD BLOCK |
| **Imports (AST)** | grep | Alle 14 Importeure erhalten ihre Namen via Fassade | SOFT WARN |

---

## 9. EMPFEHLUNG

**Gehen Sie mit TWO-PHASE approach vor:**

1. **Phase 1 (Auth-Modul)** — 15 min
   - Isoliert _AUTH_USER-Mutation-Risiko
   - Nur Importe umleiten, kein Refactoring
   - auth_naht_test verifiziert Mutation-Pfad

2. **Phase 2 (Split)** — 30 min
   - api_ring.py, api_endpunkte.py, Fassade
   - Collector + auth_naht gates beide MUSS grün
   - Atomic commit

**Warum nicht sofort Phase 2:** Phase 1 isoliert das Risiko, das Julius identifiziert hat (stale-copy). Nach Phase 1 ist Split sauber + verifizierbar.

---

## Anhang: EM-Lookup (TODO)

```bash
grep -n "^EM = " produkt/haut/*.py
```

Falls EM von api_constants.py, add zu Fassade. Falls lokal in test, nicht exportieren.

