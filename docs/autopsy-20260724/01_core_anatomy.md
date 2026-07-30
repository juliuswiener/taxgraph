# 01 — Core Anatomy

## How api.py + runner.py Actually Work

### Architecture (from code, `api.py:587-1648`)

```
POST /event → Store (JSON filesystem)
  → _bescheid_fn() slot_fn
    → Read felder from Store
    → Guard check (_an_gesamt_sperrgrund)
    → Populate g/rentner_g dicts (EURO)
    → Call runner.catala_*() accessors
    → catala_gesamt / catala_est (Catala engine)
    → Post-processing (SolZ, KiSt, DBA, §35, §34)
    → extras dict (kist_cent, solz_cent, etc.)
  → Return zahl_cent + grund
```

**Revenue path**: `POST /fall/{id}/ergebnis` → `_feste_zahl()` → `ergebnis()` endpoint.

**Three scheiben** (calculation scopes):
1. `an_gesamt` — pure employment income via `catala_est` (no §2 Gesamt-Scope)
2. `gesamt` — multi-income-type via `catala_gesamt` (full §2 EStG)
3. `rentner_gesamt` — pensioner-specific via same `catala_gesamt` but different field set

### runner.py Accessor Catalog (61 functions)

| Category | Count | Examples |
|----------|-------|----------|
| Werbungskosten (§9) | 8 | catala_werbungskosten_n, catala_entfernungspauschale, catala_p7_linear_afa |
| Sonderausgaben (§10) | 8 | catala_p10_kv_pv, catala_p10_kist, catala_p10b_spenden |
| Altersvorsorge | 2 | catala_p24a_altersentlastung, catala_p24b_entlastung |
| Außergewöhnliche Belastungen (§33) | 3 | catala_p33_agb, catala_p33_zumutbar |
| Tarif (§32a) | 5 | catala_est, catala_est_zusammen, catala_est_einzel_zve |
| Gewinneinkünfte (§§13-18) | 3 | catala_euer_gewinn, catala_mitunternehmer_einkuenfte, catala_p16_4_freibetrag |
| Kapital (§20/§32d) | 5 | catala_kapital_verrechnung, catala_kapital_steuer, catala_sparer_pb |
| Steuerermäßigungen | 7 | catala_p35a_haushaltsnahe, catala_p35_anrechnung, catala_p34c_1 |
| KiSt/SolZ | 3 | catala_kist, catala_p10_kist, catala_solz_* |
| Sonstige | 17 | Kindergeld, Mobilitätsprämie, Verlustvortrag, Abschlusszahlung, etc. |

**15 of 61 accessors are Pure-Python** (not Catala-generated) — these are the ones whose behavior can be verified by reading source code alone.

### Data Flow (verified from code)

```
Store (CENT) → _c() / _cent() helper → // 100 → EURO → runner.catala_*() → EURO slots → catala_gesamt/est → CENT zahl_cent output
```

**Critical**: All money in Store = CENT. All runner accessors = EURO. Conversion via `// 100` (floor = over-tax-safe).

### The "Stufe-1 vs Stufe-2" Design Pattern

The codebase uses a consistent vocabulary:
- **"Stufe-1"** = MVP implementation, ring-fähig, guarded fail-closed
- **"Stufe-2"** = deferred feature, explicitly documented as backlog

**11 explicit Stufe-2 references** found in code (api.py + runner.py):
- §34 Abs.3 >5Mio-Excess (abs3_ueber_5mio_offen)
- §33 Härtefall-Prüfung
- §34c Mehrfach-DBA (multi-country = gesperrt)
- §23 Veräußerungsverlust-Vortrag
- §32b-Koinzidenz Post-Engine (Stufe-2: korrekte Post-§32b-Höchstbeträge)
- §31-Kinderfreibetrag-Freigrenze detaliiert
- §24a-24b Härte-Kappung
- §10 Abs.1 Nr.5 Kinderbetreuungskosten (Stufe-2-Backlog)
- §35-Mituveranlagung separable
- §10d-Rücktrag
- §10-KV/PV §4-Abzug Person-B Nachtrag
