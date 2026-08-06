# api.py Split-Messung — Baseline vor Schnittplan

**Datum:** 2026-08-07  
**Datei:** produkt/haut/api.py  
**Messmethod:** grep + Python Parsing (siehe `/tmp/measure_api.py`)

## Gesamtstruktur

| Metrik | Wert |
|--------|------|
| Dateigröße | 2606 Zeilen |
| _bescheid_fn (L256-L1612) | 1357 Zeilen = 52% |
| Endpunkte (nach _bescheid_fn) | 17 Funktionen |
| Modul-Level Globals | 6 Variablen |

## Modul-Level Globals (6)

```
L25: HERE = os.path.dirname(os.path.abspath(__file__))
L26: PRODUKT = os.path.dirname(HERE)
L27: ROOT = os.path.dirname(PRODUKT)
L2397: CHAT_501 = { ... }
L2404: AMPEL_503 = { ... }
L2414: ENTFERNUNG_FALLBACK = { ... }
```

**Kategorisierung:**
- **PATH-Setup (L25-27):** Pfadberechnungen, von beiden Modulen genutzt → gehören in gemeinsamen Import-Bereich
- **ERROR-Constants (L2397, L2404, L2414):** nur in Endpunkten genutzt → gehören zu api_endpunkte.py

## _bescheid_fn (L256-L1612)

**1357 Zeilen, davon:**
- Helpers vor _bescheid_fn (L56-L206): ~150 Zeilen
  - _fall_owner_check, _fall_pfad, lade_fall, speichere_fall, _cfg, _datei_felder, _scheibe_felder, _scheibe_bindung, _abs3_eligible, _gwg_sofortabzug_summe, _laufender_gewinn, _p23_ansonsten_einkuenfte, _oepnv_eur
- Nested Helpers in _bescheid_fn (L256-L1612): _gemeinsame_abzuege, _an_gesamt_sperrgrund, weitere (~40+ Funktionen)

**Gesamtbetrag gehört zu api_ring.py — _bescheid_fn + alle seine Abhängigkeiten**

## Endpunkte (L1612-L2606, 17 Funktionen)

```
L1984: fall_loeschen (gehört zu fall-Management)
L2019: fall_anlegen
L2073: fragen
L2098: stand
L2141: event
L2185: warum
L2195: ergebnis
L2239: preflight_check
L2296: deklaration
L2307: einreichen
L2370: graph
L2421: entfernung
L2461: vorjahr
L2484: kontoauszug
L2545: chat
L2597: health
L2602: ready
```

**Gehört zu api_endpunkte.py**

## Abweichung zum Backlog

| Quelle | api.py | _bescheid_fn | Endpunkte |
|--------|--------|-------------|-----------|
| Backlog | 2539 | 1301 | 14 |
| Messung 2026-08-07 | 2606 | 1357 | 17 |
| **Delta** | +67 | +56 | +3 |

**Erklärung:**
- +67 Zeilen api.py: VPF-Naht vom 2026-08-06 (~40 Zeilen) + 3 zusätzliche Endpunkte (health, ready, fall_loeschen)
- +56 Zeilen _bescheid_fn: VPF-Guard erweitert (kategorie-weise Prüfung, ~20 Zeilen) + andere Refactorings
- +3 Endpunkte: health, ready, fall_loeschen nicht im alten Backlog erfasst

**Backlog-Korrektion nötig:** Stand 2026-08-07 ist die Messung authoritative.

## Messmethode

```bash
# Gesamtzahlen
wc -l produkt/haut/api.py

# Globals
grep -n "^[A-Z_][A-Z_0-9]* = " produkt/haut/api.py

# _bescheid_fn Bereich
grep -n "^def _bescheid_fn\(" produkt/haut/api.py
grep -n "^def " produkt/haut/api.py | grep -A1 _bescheid_fn

# Endpunkte (Zeile 1612 bis Ende)
sed -n '1612,2606p' produkt/haut/api.py | grep -n "^def [a-z]"
```

Alle Befehle reproduzierbar, Ausgabe in `/tmp/measure_api.py` und Bash-Log.
