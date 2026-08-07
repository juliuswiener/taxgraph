# §33b Stufe 2 — Bauanleitung Behinderungsbedingte Aufwendungen

**Stand:** 2026-08-07  
**Status:** Analysiert, bereit für Bau  
**Aufwand:** ~1.5h (Bindung eintrag + Ring-Kürzungslogik + Tests, keine EM.instanzen-Schleife nötig)

---

## Frage A: Ist § 33b Abs. 5 S. 4 heute implementiert?

**Antwort:** NEIN. Fehlt komplett.

**Fundstellen:**
- **api.py:306-311**: Kommentar nennt explizit "OHNE S.4-Ausschluss → benannte Lücke"
- **api.py:309-311** (Zitat):
  ```
  # S.4: "In diesen Fällen besteht für Aufwendungen, für die der Behinderten-Pauschbetrag
  # gilt, kein Anspruch auf eine Steuerermäßigung nach § 33" — NICHT implementiert
  # (agb_aufwendungen ist nicht nach Aufwandsart getrennt, pauschale Kürzung = Over-tax).
  ```

**Betroffene Fälle:**
- Jeder Fall mit Kind-PB-Übertragung (GdB ≥ 50, antrag=True, nicht_selbst_genutzt=True)
- **Messung (BACKLOG, gemessen):** 40k Lohn, einzel, GdB 100, 3k behinderungsbedingte agB 
  - Nur PB: ESt 880 EUR Wirkung
  - Nur agB: ESt 259 EUR Wirkung
  - Beides (fälschlich): ESt 1.131 EUR Wirkung
  - **Fehler (Doppelabzug): 251 EUR Under-tax** (gemessen, nicht geschätzt)

**Reihenfolge:**
Abs. 5 S. 4 (automatischer Ausschluss) kommt VOR Abs. 1 S. 1 (Wahlrecht). Der Ring erzwingt S.4 ohne zu fragen.

---

## Frage B: Präzedenz — wie sind kind_kv/kind_pv gebaut?

**Antwort:** Vollständig implementiert, additiv zu basis_kv/basis_pv.

### Bindung (produkt/bindung/bindung_p10_1_3_kind_kv_pv.yaml)
- **Zeile 15-32:** `kind_kv` (cent, instanz_gruppe: kind, askable, Kz E0503110)
- **Zeile 34-51:** `kind_pv` (cent, instanz_gruppe: kind, askable, Kz E0503310)
- **Zeile 54-59:** Lücke Kind-Beiträge gehen in denselben Deckel wie Elternteil-Beiträge

### Scheibe
- In **api_constants.py**: `KIND_KV_PV = ("kind_kv", "kind_pv", "kind_idnr")`
- Teil von `GESAMT_ABZUEGE`, daher in `SCHEIBEN["gesamt"]["felder"]` und `SCHEIBEN["rentner_gesamt"]["felder"]`

### Ring-Verdrahtung (produkt/haut/api.py)

#### Hilfsfunktion (api.py:288-301, zweite Variante 303-304):
```python
def _kind_kv_pv_summe() -> int:
    total = 0
    for inst in EM.instanzen(store, bindung, "kind"):
        if not nur_bestaetigt or inst["zustand"] == "bestaetigt":
            idnr = inst["felder"].get("kind_idnr", {}).get("wert")
            if not idnr or not isinstance(idnr, str) or len(idnr) < 11:
                continue
            # Zeile 295-297: kind_kv lesen, Typcheck, summieren
            kv = inst["felder"].get("kind_kv", {}).get("wert")
            if isinstance(kv, (int, float)) and not isinstance(kv, bool) and kv > 0:
                total += int(kv)
            # Zeile 298-300: kind_pv lesen, Typcheck, summieren
            pv = inst["felder"].get("kind_pv", {}).get("wert")
            if isinstance(pv, (int, float)) and not isinstance(pv, bool) and pv > 0:
                total += int(pv)
    return total  # CENT
```

**Guard:** `if store is not None` (Zeile 286-287, Fallback zur leeren Funktion Zeile 303-304)

#### Aufruf-Stellen (basis_kv/basis_pv addieren):
- **gesamt-Ring, Zeile 370:**
  ```python
  "basis_kv_pv": (_c("basis_kv") + _c("basis_pv") + _kind_kv_pv_summe()) // 100,
  ```
- **rentner-Ring, Zeile 540:**
  ```python
  "basis_kv_pv": (_cent("basis_kv") + _cent("basis_pv") + _kind_kv_pv_summe()) // 100,
  ```

### Für neues Feld `behinderungsbedingte_aufwendungen` analog:

**WICHTIG:** agb_aufwendungen ist fallweit (KEINE instanz_gruppe). Die Übertragung des Kind-PB ist per-Kind, aber die ausgeschlossenen Aufwendungen sind des Steuerpflichtigen (Abs. 1 S. 1). Daher: `behinderungsbedingte_aufwendungen` AUCH fallweit (Option a). Keine EM.instanzen-Schleife nötig — einfaches zusätzliches Feld wie agb_aufwendungen.

| Punkt | Ort | Was tun |
|-------|-----|---------|
| **Bindung** | bindung_sonder_agb_35a.yaml (nach agb_aufwendungen, Zeile 74+) | Neuer Eintrag: `behinderungsbedingte_aufwendungen` (cent, KEIN instanz_gruppe, askable, kein Kz, fragetext: "Davon behinderungsbedingt (Hilfe tägliche Verrichtungen, Pflege, Wäschebedarf):") |
| **Scheibe** | api_constants.py | Neue Konstante `BEHINDERUNGSBEDINGTE_AUFWENDUNGEN = ("behinderungsbedingte_aufwendungen",)` |
| **Scheibe** | api_constants.py | In `GESAMT_ABZUEGE` aufnehmen (nach AGB_KIST oder am Ende) |
| **Ring** | api.py:387-388, in `_shared_steuer_sonder_agb` | **EINE Stelle, nicht zwei.** Der Helfer wurde am 2026-08-06 extrahiert und wird von beiden Zweigen gerufen: L971 (gesamt) und L1459 (rentner). `_c("agb_aufwendungen") // 100` bei L388 ist die einzige agB-Senke. Bei Kind-PB-Übertragung: `agb_bereinigt_cent = _c("agb_aufwendungen") - _c("behinderungsbedingte_aufwendungen")` (beide CENT), dann `max(0, agb_bereinigt_cent) // 100` |
| **Closure** | api.py:314 | `_kind_behinderten_pb_daten()` liegt im selben `_bescheid_fn`-Scope, ist also in `_shared_steuer_sonder_agb` sichtbar. Leere Liste = keine Übertragung = keine Kürzung. |
| **Guard** | api.py (beide Zweige) | `if _kind_pb_uebertragen():` prüft Übertragung; fallweit agb-Kürzung macht Sinn nur, wenn Kind-PB tatsächlich übertragen wurde |

---

## Frage C: Kein über-tax-sicherer Default für Wahlrecht (Abs. 1 S. 1)?

**Antwort:** BESTÄTIGT. Keine pauschale Wahl möglich.

### Backlog-Messung (40k Lohn, einzel, GdB 100):
```
Nur PB (2840 EUR):         ESt Δ = 880 EUR
Nur agB (3000 EUR):        ESt Δ = 259 EUR
Beides (fälschlich):       ESt Δ = 1.131 EUR
Fehler (Doppelabzug):      251 EUR zu wenig Steuer (GEMESSEN)
```

### Zwei Randfälle:
1. **Kleine agB (259 EUR Wirkung):** PB 880 > agB 259 → **PB wäre zu hoch**, Nutzer wählt Einzelnachweis für die 3 Aufwandsarten
2. **Große agB (3157 EUR Wirkung):** PB 880 < agB 3157 → **PB wäre zu niedrig**, Nutzer wählt PB für bessere Steuerwirkung

**Fazit:** Kein Pauschal-Default über beide Fälle hinweg möglich. **Der Nutzer MUSS gefragt werden.**

### Sperrgrund-Name (Empfehlung):
**`behinderungsbedingte_aufwendungen_wahlrecht_offen`**

Muster: `{feld}_{aspekt}_offen` (siehe api_schema/ergebnis.json Zeile 15-29, z.B. `verpflegung_dreimonatsfrist_aufteilung_offen`).

---

## Zusammenfassung: Bauanleitung

### Stufe 2a (Abs. 5 S. 4 — automatischer Ausschluss):

**Neues Feld:** `behinderungsbedingte_aufwendungen`
- **Typ:** CENT (Integer)
- **Instanz:** KEINE — fallweit, wie `agb_aufwendungen` (siehe Frage B, Absatz "WICHTIG"). Keine `instanz_gruppe`, keine `EM.instanzen`-Schleife.
- **askable:** true
- **Kz:** keines (dokumentiert, Grund: "reines Rechenfeld, keine ELSTER-Bindung")
- **Bedeutung:** "Davon"-Teilmenge von agb_aufwendungen; welcher Anteil ist behinderungsbedingt i.S.v. §33b Abs.1 S.1 (Hilfe tägliche Verrichtungen, Pflege, Wäschebedarf)?

**Ring-Logik (Abs. 5 S. 4):**

**RICHTIG** (nach Muster api.py:370 und 387-391):
```python
# Beide Eingaben in CENT, eine Division am Ende
if _kind_pb_uebertragen():
    agb_bereinigt_cent = _c("agb_aufwendungen") - _c("behinderungsbedingte_aufwendungen")
    agb_bereinigt_euro = max(0, agb_bereinigt_cent) // 100
else:
    agb_bereinigt_euro = _c("agb_aufwendungen") // 100

# agb_bereinigt_euro wird dann an catala_p33_agb übergeben
g_dict["aussergewoehnliche_belastungen"] = ... + runner.catala_p33_agb({
    "aussergewoehnliche_belastungen": agb_bereinigt_euro + fahrtkosten_pauschale_euro,
    ...
})
```

**FEHLER in meinem ursprünglichen Vorschlag:**
```python
# FALSCH: _c() liefert CENT, catala erwartet EURO
agb_bereinigt = max(0, _c("agb_aufwendungen") - _behinderungsbedingte_aufwendungen_summe() // 100)
                                                  ^ zieht EURO von CENT ab → Faktor 100 falsch
```

### Stufe 2b (Abs. 1 S. 1 — Wahlrecht, SPÄTER):

Neue Bool-Frage (z.B. `behinderungsbedingte_aufwendungen_wahlrecht_pb`):
```
"Möchtest du für die behinderungsbedingten Aufwendungen 
den Behinderten-Pauschbetrag nutzen oder die tatsächlichen 
Kosten als außergewöhnliche Belastung absetzen?"
```

**Ring-Logik:**
```python
# Bei eigenem GdB UND Wahlrecht-Frage beantwortet
if _eigener_pb_genutzt() and behinderungsbedingte_aufwendungen_wahlrecht_pb is True:
    # Beide CENT, eine Division am Ende
    agb_bereinigt_cent = _c("agb_aufwendungen") - _c("behinderungsbedingte_aufwendungen")
    agb_bereinigt_euro = max(0, agb_bereinigt_cent) // 100
else:
    agb_bereinigt_euro = _c("agb_aufwendungen") // 100
# (sonst: agb bleibt unberührt, Einzelnachweis mit vollem Betrag)
```

**Sperrgrund (Wahlrecht unbeantwortbar):**
`behinderungsbedingte_aufwendungen_wahlrecht_offen`

---

## Änderungsliste (Bau-Checklist)

- [ ] **bindung_sonder_agb_35a.yaml:** Neuer Eintrag nach agb_aufwendungen (Zeile ~86)
- [ ] **api_constants.py:** Neue Konstante + GESAMT_ABZUEGE erweitern
- [ ] **api.py::_shared_steuer_sonder_agb (387-394):** Kürzung genau HIER, eine Stelle. Beide Zweige (L971 gesamt, L1459 rentner) gehen durch diesen Helfer — ein zweiter Eingriff im rentner-Zweig wäre ein Doppelabzug.
- [ ] **tests/test_p10_1_3_kind_kv_pv_ring.py:** Regression (kind_kv/kind_pv addiert korrekt)
- [ ] **tests/test_p33b_abs5_s4_ring.py** (NEU): Test mit Kind-PB + behinderungsbedingte_aufwendungen
  - Fall 1: Kind-PB übertragen, behinderungsbedingte_aufwendungen=3000 → agB gekürzt
  - Fall 2: Kind-PB nicht übertragen → agB unverändert
- [ ] **Test-Gate:** `python -m pytest tests/ -q` must stay ≥ 1632 passed

---

## Fußnote: Abs. 1 S. 1 Wahlrecht

Abs. 5 S. 4 ist ein **Automatismus** (Ring erzwingt, kein User-Input). Abs. 1 S. 1 ist ein **Wahlrecht** (Nutzer entscheidet). Diesen Report behandelt Abs. 5 S. 4 komplett; Abs. 1 braucht eine zusätzliche Bool-Frage + Sperrgrund. Beide teilen dasselbe `behinderungsbedingte_aufwendungen`-Feld.
