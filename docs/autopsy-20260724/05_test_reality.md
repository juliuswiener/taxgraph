# 05 — Test Reality

## What the 105 Test Failures Actually Mean

### Current State

```
854 passed, 105 failed, 2 skipped
```

### Failure Categories

**Category A — jsonschema `grund` enum mismatch (87 tests)**
- Files: `test_paket_b_e2e_http.py`, `test_solz_ring.py`, `test_ring_regression_kampagne.py`
- Root cause: `produkt/haut/api_schema/ergebnis.json` `grund` enum is incomplete
- Missing entries: `dba_freistellung_offen`, `dba_multi_country_offen`, `dba_kapital_offen`, `p32b_kombi_offen`, `progression_gehoert_in_gesamt`, `p16_4_gate_offen`
- These are ALL valid guard responses — the schema is simply lagging behind the code
- **Not a code bug** — purely a schema maintenance issue

**Category B — g2 NameError in gesamt scheibe (15 tests)**
- File: `test_ring_regression_kampagne.py`
- Root cause: `api.py:1301` references `g2` variable which doesn't exist in that scope
- This was partially fixed in `be28ec7` but the fix was incomplete
- The KiSt calculation block uses `kap_st_total` and `g2["est_gesamt"]` — both undefined
- **Confirmed pre-existing**: same failures on `git stash` clean base

**Category C — Binding table validation (1 test)**
- File: `test_bindungstabelle.py::test_b_vollstaendigkeit`
- Root cause: `p7_1_lineare_afa` accessor expects 3 slots (`anschaffungs_herstellungskosten`, `nutzungsdauer_jahre`, `anzurechnende_monate`) that are not in the binding
- Partially fixed in `2468a6e` and `236b655`

**Category D — Catala engine unavailable (14 collection errors)**
- Files: `test_kapital_accessoren.py`, `test_kist_accessor.py`, `test_solz_accessor.py`, etc.
- Root cause: `from pkg import ...` fails — `pkg` module not built
- These tests NEVER EXECUTE — they fail during collection
- **Not a code bug** — build infrastructure issue

### Test Coverage Honest Assessment

| Domain | Tested? | How? |
|--------|---------|------|
| §32a Tarif (single) | YES | catala_est via an_gesamt Ring |
| §32a Tarif (splitting) | YES | catala_est_zusammen via gesamt Ring |
| Werbungskosten §9 | YES | catala_werbungskosten_n via gesamt Ring |
| Sonderausgaben §10 | MOSTLY | catala_p10_kv_pv, catala_p10b_spenden |
| Außergewöhnliche Belastungen §33 | YES | catala_p33_agb, catala_p33_zumutbar |
| Vorsorge §10 | YES | catala_p10_kv_pv via gesamt Ring |
| Altersentlastung §24a | MINIMAL | Only via catala_gesamt scope |
| Kinder §31/§32 | NO | "gehören in gesamt" — an_gesamt Ring doesn't test this |
| DBA §34c Anrechnung | PARTIAL | Unit test for catala_p34c_1 exists; Ring-Diff for Freistellung is MISSING |
| DBA §34c Freistellung | NO | Guard blocks it (GAP-C1), no Ring-Diff test possible |
| GewSt §35 | PARTIAL | catala_p35_anrechnung tested; Mitu §35 NOT tested |
| KiSt §51a | YES | via extras kist_cent in Ring tests |
| Verlustvortrag §10d | MINIMAL | Only via catala_gesamt scope |
| Progressionsvorbehalt §32b | NO | No Ring-Diff test proving p32b actually affects tax |
