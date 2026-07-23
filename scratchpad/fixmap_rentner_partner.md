# Fix-Map: Rentner Person-B KV/PV (P2-#2) + VOR (P2-#3)

Target: dev-1, serielle api.py-Zone. Für §16-After-KiSt-Queue.
Read-only Map (kein Commit). Beide Over-tax (Partner-Abzug fehlt bei Zusammenveranlagung).

---

## P2-#2: Rentner Person-B KV/PV

**Exposure**: Bs KV/PV-Abzug (§10 Abs.1 Nr.3/3a) fehlt im Rentner-Ring bei zusammen.
Over-tax ~2800€ HB × ~14-40% = 400-1100€ Steuer, mittlere Häufigkeit (Rentner-Ehepaare).

### (a) Gesamt-Pattern (copy-template)

`api.py:984-990`:
```python
# Person-B-KV/PV (§ 10 Abs. 4, A.2): eigener Höchstbetrag JE PERSON → separater Accessor-Aufruf,
# additiv (kein gemeinsamer Deckel, kein Doppelzählen — B liest die _partner-Read-Keys).
+ (runner.catala_p10_kv_pv({
    "basis_kv_pv": _c("basis_kv_pv_partner") // 100,
    "weitere_vorsorgeaufwendungen": _c("weitere_vorsorgeaufwendungen_partner") // 100,
    "mit_anspruch_auf_zuschuss": f.get("mit_anspruch_auf_zuschuss_partner", {}).get("wert") is True})
   if g["veranlagung"] == "zusammen" else 0)
```

Modell: separater `catala_p10_kv_pv`-Aufruf mit `_partner`-Read-Keys, unter `if veranlagung==zusammen`, ADDITIV (eigener HB je Person, kein gemeinsamer Deckel). Person A ist bereits in der gleichen Expression enthalten (L1453-1456).

### (b) Rentner Insertion-Point

`api.py:1448-1467` — die `rentner_g["sonderausgaben"] = (...)`-Expression.

**Vorher** (L1465-1467, Expressionende):
```python
                + runner.catala_p10_1_7_berufsausbildung({
                    "berufsausbildung_aufwendungen": _c("berufsausbildung_aufwendungen") // 100}))
```

**Nachher** (letzter Term VOR dem schließenden `))`):
```python
                + runner.catala_p10_1_7_berufsausbildung({
                    "berufsausbildung_aufwendungen": _c("berufsausbildung_aufwendungen") // 100})
                # Person-B-KV/PV (§ 10 Abs. 4, A.2): eigener HB je Person, additiv (1:1 gesamt-Z.984-990)
                + (runner.catala_p10_kv_pv({
                    "basis_kv_pv": _c("basis_kv_pv_partner") // 100,
                    "weitere_vorsorgeaufwendungen": _c("weitere_vorsorgeaufwendungen_partner") // 100,
                    "mit_anspruch_auf_zuschuss": f.get("mit_anspruch_auf_zuschuss_partner", {}).get("wert") is True})
                   if f.get("veranlagung", {}).get("wert") == "zusammen" else 0))
```

Hinweis: rentner slot_fn nutzt `_c()` und `_b()/f.get()` (definiert L1265-1270). Der `f`-Bezug ist identisch zu gesamt. Bool-Felder via `f.get(...).get("wert") is True`.

### (c) SCHEIBEN.felder Delta

Folgende Partner-Felder fehlen für KV/PV-Fix in `RENTNER_FELDER` (aktuell 0 VORSORGE_PARTNER_FELDER in rentner_gesamt):

- `basis_kv_pv_partner` (cent, ASKABLE)
- `weitere_vorsorgeaufwendungen_partner` (cent, ASKABLE)
- `mit_anspruch_auf_zuschuss_partner` (bool, ASKABLE)

**Einbau**: Append `KV_PV_PARTNER_FELDER` (schon definiert als Tuple L99-100) zum `RENTNER_FELDER`-Tuple.
Das Tupel ist eine Kette aus Verkettungen (L184 + L213 + L260). Append z.B. am Ende der L213-ähnlichen Fortsetzung oder als + `KV_PV_PARTNER_FELDER` am Ende von L260.

Als optional (NICHT im kegel-Eintrag) — wie in gesamt (optional, absent→0→over-tax-safe).

### (d) Guard-Check

KEIN rentner-Guard in `_an_gesamt_sperrgrund` (L1623-1856) blockiert Partner-KV/PV. Der Guard prüft ausschließlich rentenfreibetrag_fixierung, rechnung_unbar, erstattungsueberhang, fremd_arten, dHf/Vpf — nichts von Vorsorge-Inputs. Der einzige Grund der B-Vorsorge nicht wirkt: fehlende SCHEIBEN.felder-Einträge → `_c()` gibt immer 0.

### (e) Tests (Pflicht)

1. **Erreichbarkeit**: `POST /event` auf `basis_kv_pv_partner` → 201 (nicht 400). Nach fixture in rentner_gesamt-Scheibe. Betrifft auch `weitere_vorsorgeaufwendungen_partner` und `mit_anspruch_auf_zuschuss_partner`.
2. **Ring-Differential**: rentner zusammen MIT B-KV/PV-Wert vs OHNE → `zahl_cent` strikt niedriger (Δ = B-Vorsorge × Grenzsteuersatz). Over-tax entfernt. Muster: `tests/test_ring_regression_kampagne.py` (dev-1).

---

## P2-#3: Rentner Person-B VOR (Basisvorsorge RV)

**Exposure**: Bs RV-Vorsorge-Abzug (§10 Abs.1 Nr.2) fehlt im Rentner-Ring bei zusammen.
Over-tax ~27566€ HB (selten ausgeschöpft bei Rentnern ohne aktive RV-Beiträge).

### (a) Gesamt-Pattern

`api.py:921-924`:
```python
if g["veranlagung"] == "zusammen":
    g["vorsorge_gesamtbeitraege_inkl_ag"] += (_c("vor_an_anteil_rv_partner")
        + _c("vor_ag_anteil_rv_partner") + _c("vor_rv_ausserhalb_lstb_partner")) // 100
    g["vorsorge_ag_anteil_steuerfrei"] += _c("vor_ag_anteil_rv_partner") // 100
```

Modell: In die bestehenden Summen-Slots (`vorsorge_gesamtbeitraege_inkl_ag`, `vorsorge_ag_anteil_steuerfrei`) ADDITIV die _partner-Euro-Werte einfließen lassen, unter `if zusammen`. Der catala_gesamt-Accessor deckelt EINMAL (HB verdoppelt nicht → over-tax-residual dokumentiert L922).

### (b) Rentner Insertion-Point

`api.py:1424-1426` — die aktuelle Person-A-only VOR-Slot-Befüllung:
```python
rentner_g["vorsorge_gesamtbeitraege_inkl_ag"] = (_c("vor_an_anteil_rv") + _c("vor_ag_anteil_rv")
                                                  + _c("vor_rv_ausserhalb_lstb")) // 100
rentner_g["vorsorge_ag_anteil_steuerfrei"] = _c("vor_ag_anteil_rv") // 100
```

**Insert NACH L1426**, VOR `# § 35a Haushaltsnahe (L1427)` ein Block:
```python
# Person-B-Altersvorsorge (§ 10 Abs. 1 Nr. 2, A.2): bei zusammen die vor_*_rv_partner ADDITIV
# in dieselben Summen-Slots (catala_gesamt/_vorsorge_abzug deckelt EINMAL, 1:1 gesamt-Z.921-924).
if _b("veranlagung") == "zusammen":
    rentner_g["vorsorge_gesamtbeitraege_inkl_ag"] += (_c("vor_an_anteil_rv_partner")
        + _c("vor_ag_anteil_rv_partner") + _c("vor_rv_ausserhalb_lstb_partner")) // 100
    rentner_g["vorsorge_ag_anteil_steuerfrei"] += _c("vor_ag_anteil_rv_partner") // 100
```

Hinweis: Wiederverwende `_b()` oder `f.get("veranlagung", {}).get("wert")` für den Bool-Check (rentner-Konvention L1269). Slot-Zugriff über `_c()` — identisch Pattern.

### (c) SCHEIBEN.felder Delta

Zusätzlich zu den KV/PV-Feldern braucht VOR-Fix folgende in RENTNER_FELDER:
- `vor_an_anteil_rv_partner` (cent, ASKABLE)
- `vor_ag_anteil_rv_partner` (cent, ASKABLE)
- `vor_rv_ausserhalb_lstb_partner` (cent, ASKABLE)

Diese 3 Felder sind Teil von `VOR_PARTNER_FELDER` (L93-95). Append `VOR_PARTNER_FELDER` zu `RENTNER_FELDER` (gemeinsam mit KV/PV-Feldern möglich, alle optional).

### (d) Guard-Check

Identisch zu KV/PV: kein rentner-Guard blockiert VOR-Partner. `_c()` gibt aktuell 0 weil Felder nicht in SCHEIBEN.felder. Höchstbetrag-Residual (HB wird bei zusammen nicht verdoppelt) ist dokumentiert L922 und separat gemeldet — nicht Teil dieses Fix.

### (e) Tests (Pflicht)

1. **Erreichbarkeit**: POST `vor_an_anteil_rv_partner` + `vor_ag_anteil_rv_partner` + `vor_rv_ausserhalb_lstb_partner` → 201.
2. **Ring-Differential**: rentner zusammen MIT vs OHNE B-VOR → `zahl_cent` strikt niedriger.

---

## Abbau-Reihenfolge

1. KV/PV+P2-#2 (kleiner, 3 Felder, Copy-known-pattern) → dann P2-#3 (VOR, +3 Felder, Copy-known-pattern).
2. Beide können im selben Commit (alle 6 Partner-Felder + 2 Insert-Blöcke) oder getrennt.
3. Vorher: `git status` check (keine offenen Änderungen in api.py). Nachher: Vollsuite + Erreichbarkeits-Gate (grün).
