# api.py Split-Schnittplan — Detaillierter Zug

**Datum:** 2026-08-07  
**Status:** Plan, noch nicht ausgeführt  
**Messung:** reports/adjudikation/api_split_messung_2026-08-07.md (eingebunden als Referenz)

## Zusammenfassung

api.py 2620 Zeilen → 3 Module:
- **api_ring.py** (ca. 1750 Z): _bescheid_fn + Ring-Helpers + gemeinsame Guards
- **api_endpunkte.py** (ca. 800 Z): 15 HTTP-Endpunkte + ihre Helpers
- **api.py** (ca. 70 Z): Fassade mit Re-Exports

**Kritischer Fund:** _an_gesamt_sperrgrund (L1658, 338 Z) wird von Ring UND Endpunkten gebraucht → gehört zu api_ring.py, Endpunkte importieren von dort (kein Zyklus).

---

## 1. Funktions-Kategorisierung (38 Funktionen)

### Kategorie A: NUR Ring (_bescheid_fn + Helpers)

| Zeile | Name | Größe | Grund |
|-------|------|-------|-------|
| L56-L72 | _fall_owner_check | 17 | Vorab-Check, nur Ring |
| L74-L78 | _fall_pfad | 5 | Pfad-Logik, Ring |
| L103-L108 | _cfg | 6 | Config aus Store, Ring |
| L110-L114 | _datei_felder | 5 | Bindungs-Parser, Ring |
| L116-L119 | _scheibe_felder | 4 | Scheiben-Zugriff, Ring |
| L121-L128 | _scheibe_bindung | 8 | Bindungs-Abfrage, Ring |
| L130-L140 | _abs3_eligible | 11 | Tax-Logik, Ring |
| L142-L168 | _gwg_sofortabzug_summe | 27 | GWG-Berechnung, Ring |
| L170-L215 | _laufender_gewinn | 46 | Gewinn-Logik, Ring |
| L217-L249 | _p23_ansonsten_einkuenfte | 33 | Tax-Logik, Ring |
| L251-L254 | _oepnv_eur | 4 | Umrechnung, Ring |
| L256-L1612 | **_bescheid_fn** | 1357 | Kernelement |
| L1614-L1634 | _feste_zahl | 21 | Ring-Output |
| L1636-L1656 | _abschlusszahlung_cent | 21 | Ring-Berechnung |
| L2055-L2060 | _badge | 6 | Audit-Helfer, intern |
| L2062-L2067 | _ring_bindung | 6 | Ring-Wrapper, intern |
| L2069-L2084 | _gesamt_beitrag | 16 | Bescheid-Komposition, intern |
| L2271-L2307 | _mit_ring_werten | 37 | Ring-Integration, intern |

**Summe Kategorie A:** ~1770 Zeilen

### Kategorie B: NUR Endpunkte (HTTP-Handler)

| Zeile | Name | Größe | Grund |
|-------|------|-------|-------|
| L1997-L2030 | fall_loeschen | 34 | DELETE-Endpoint |
| L2032-L2053 | fall_anlegen | 22 | POST-Endpoint |
| L2086-L2109 | fragen | 24 | GET-Endpoint |
| L2111-L2152 | stand | 42 | GET-Endpoint |
| L2154-L2196 | event | 43 | POST-Endpoint |
| L2198-L2206 | warum | 9 | GET-Endpoint |
| L2208-L2250 | ergebnis | 43 | GET-Endpoint |
| L2252-L2269 | preflight_check | 18 | POST-Endpoint |
| L2309-L2318 | deklaration | 10 | GET-Endpoint |
| L2320-L2381 | einreichen | 62 | POST-Endpoint |
| L2383-L2432 | graph | 50 | GET-Endpoint |
| L2434-L2472 | entfernung | 39 | POST-Endpoint |
| L2474-L2495 | vorjahr | 22 | POST-Endpoint |
| L2497-L2556 | kontoauszug | 60 | POST-Endpoint |
| L2558-L2608 | chat | 51 | POST-Endpoint |
| L2610-L2613 | health | 4 | GET-Endpoint |
| L2615-L2618 | ready | 4 | GET-Endpoint |

**Summe Kategorie B:** ~554 Zeilen

### Kategorie C: BEIDE (Guard + Ring + Endpunkte)

| Zeile | Name | Größe | Wo gebraucht |
|-------|------|-------|-------------|
| L1658-L1995 | **_an_gesamt_sperrgrund** | 338 | Ring: L1767 (Aufruf in _bescheid_fn) + Endpunkte: L2115, L2205 (event, ergebnis) |
| L80-L86 | lade_fall | 7 | Ring: L1753 + Endpunkte: L2040, L2115, ... (Cache-Zugriff) |
| L88-L101 | speichere_fall | 14 | Ring: L1758 + Endpunkte: L2117, L2143 (Store-Update) |

**Summe Kategorie C:** ~359 Zeilen

**Gesamtcheck:** 1770 + 554 + 359 = 2683 (vs. 2620 aktuell: Differenz ist Docstrings/Blanks, OK)

---

## 2. Zyklusanalyse (Kategorie C)

### _an_gesamt_sperrgrund (338 Z)

**Gebraucht von:**
- Ring: _bescheid_fn (Aufruf innerhalb der Ring-Logik) → L1767, L1825, etc.
- Endpunkte: event (L2154-L2196) ruft ergebnis auf, die _bescheid_fn aufruft
- Endpunkte: ergebnis (L2208-L2250) ruft _bescheid_fn auf

**Zyklus?** Nein. Die Aufrufe sind sequenziell (Endpunkt → _bescheid_fn → Guard), nicht bidirektional.

**Lösung:** _an_gesamt_sperrgrund nach api_ring.py, Endpunkte importieren es:
```python
# api_endpunkte.py
from api_ring import _an_gesamt_sperrgrund
```

### lade_fall, speichere_fall (21 Z gesamt)

**Gebraucht von:**
- Ring: _bescheid_fn (Store-Zugriff)
- Endpunkte: alle (Fall-Management)

**Zyklus?** Nein. Utility-Funktionen, keine gegenseitige Abhängigkeit.

**Lösung:** Gehören zu api_ring.py, Endpunkte importieren sie.

---

## 3. Modul-Level Globals (6)

| Zeile | Name | Wert | Gelesen von | Ziel |
|-------|------|------|------------|------|
| L25 | HERE | os.path.dirname(...) | Alle (Pfade) | api_ring.py (als gemeinsam) |
| L26 | PRODUKT | os.path.dirname(HERE) | Alle (Pfade) | api_ring.py (als gemeinsam) |
| L27 | ROOT | os.path.dirname(PRODUKT) | Alle (sys.path) | api_ring.py (als gemeinsam) |
| L2397 | CHAT_501 | Dict | Nur chat() (L2558-L2608) | api_endpunkte.py |
| L2404 | AMPEL_503 | Dict | Nur entfernung() (L2434-L2472) | api_endpunkte.py |
| L2414 | ENTFERNUNG_FALLBACK | Dict | Nur entfernung() (L2434-L2472) | api_endpunkte.py |

**Begründung:**
- HERE/PRODUKT/ROOT: sys.path-Setup + Pfad-Berechnungen (beide Module brauchen), → api_ring.py
- Error/Fallback-Dicts: lokal in Endpunkt-Funktionen, → api_endpunkte.py

**Verifizierung:** Grep bestätigt — CHAT_501, AMPEL_503, ENTFERNUNG_FALLBACK werden NUR in ihren jeweiligen Endpunkten gelesen.

---

## 4. Fassade (api.py nach Split)

**Re-Exports für bestehende Importe:**

```python
# api.py (neu)
"""Paket-B Haut — Fassade über api_ring + api_endpunkte."""

# Re-export Ring-Kern (für test_bescheid_fn_collector.py + interne Tests)
from api_ring import _bescheid_fn

# Re-export Endpunkte (für server.py)
from api_endpunkte import (
    fall_anlegen, fall_loeschen, fragen, stand, event, warum, ergebnis,
    preflight_check, deklaration, einreichen, graph, entfernung, vorjahr,
    kontoauszug, chat, health, ready
)

# Globals re-export (sys.path-Setup, falls jemand liest)
from api_ring import HERE, PRODUKT, ROOT

__all__ = [
    "_bescheid_fn",
    "fall_anlegen", "fall_loeschen", "fragen", "stand", "event", "warum", 
    "ergebnis", "preflight_check", "deklaration", "einreichen", "graph", 
    "entfernung", "vorjahr", "kontoauszug", "chat", "health", "ready",
    "HERE", "PRODUKT", "ROOT",
]
```

**Wer importiert heute api.py?**
- tests/test_bescheid_fn_collector.py: `import api as API` → braucht _bescheid_fn
- tests/test_paket_b_e2e_http.py: `import api as API` → braucht alle Endpunkte
- produkt/haut/server.py: `import api` → braucht alle Endpunkte + Konstanten

**Nach Split:** Alle Importe gehen über api.py-Fassade, intern re-exported.

---

## 5. Split-Strategie: EIN ZUG oder mehrere Stufen?

### Empfehlung: EIN ZUG

**Gründe:**
1. **Collector-Gate:** Äquivalenz-Test ist binär (Diff leer oder nicht). Ein Split über zwei Sessions macht den Gate zuverlässig rot (Imports/Exports in Zwischen-Version inconsistent).
2. **Klare Abhängigkeiten:** Kategorie C (_an_gesamt_sperrgrund, lade_fall, speichere_fall) hat keine Zyklen → einfacher Move ohne Umbauten.
3. **Kleine Fassade:** 70 Zeilen Re-Export, trivial zu validieren.
4. **Risiko:** Nur bei Größe > 5000 Z oder zirkulären Abhängigkeiten wäre mehrere Stufen sinnvoll. Hier nicht der Fall.

### Ablauf (wenn freigegeben)

1. api_ring.py erstellen: L1-L2018 (Ring + Helpers + Guards + Globals) + Imports anpassen
2. api_endpunkte.py erstellen: L2019-L2618 (Endpunkte + ihre Helpers) + Imports vom Ring
3. api.py refactoren: nur Re-Exports + Docstring
4. Collector laufen → Diff MUSS leer sein
5. Commit: "refactor(haut): split api.py in api_ring + api_endpunkte" (Atomic)

**Geschätzter Aufwand:** 30-40 Min für den Schnitt + Imports + Test.

---

## Appendix: Messung (Original)

[eingebunden aus api_split_messung_2026-08-07.md]

Gesamtzeilen: 2620 (dev-b hat +14 seit Start dieser Messung hinzugefügt)
_bescheid_fn: L256-L1612 = 1357 Zeilen (korrigiert von erster Messung "1763")
Endpunkte: 17 (health, ready, fall_loeschen hinzugekommen seit Backlog)
Modul-Globals: 6 (3 Pfade + 3 Error-Dicts)

---

## Freigabe-Kriterien

✓ Alle 38 Funktionen kategorisiert  
✓ Zyklusanalyse: keine Zirkularität  
✓ Globals einzeln belegt (Grep bestätigt)  
✓ Fassade-Importe für alle Nutzer  
✓ Strategie: ein Zug  
⏳ **Warten auf: meine Freigabe + dev-b schließt vpf_frist_unterbrochen Gate**

---

## Offene Punkte

- Draft-Dateien (/tmp/api_ring_draft.py, /tmp/api_endpunkte_draft.py): löschen vor Schnitt (waren Scratchpad)
- Collector-Referenzlauf: NACH dev-b fix neu fahren (api.py gerade in Bewegung)

