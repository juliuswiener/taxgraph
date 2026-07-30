# 04 — Gap Analysis

## All Known Limitations, Ranked by Severity

### CRITICAL

**GAP-C1: DBA Freistellung Guard Contradiction** (see doc 03)
- Guard blocks what calculation layer supports
- Affects AT, US, and CH (via Protocol 2023) DBA scenarios
- **Fix**: Remove guard OR remove routing code — pick one

**GAP-C2: Missing Catala PKG module in test environment**
- `golden/runner.py:39` imports `from pkg import Einkommensteuertarif as E`
- The `pkg` module must be built via `make build-python` (opam/catala toolchain)
- Without this, 14 test files fail during COLLECTION (never reach execution)
- **Fix**: Automate `make build-python` as CI pre-step, document requirement

### HIGH

**GAP-H1: No per-Einkunftsart DBA routing** (see doc 03)
- Single `dba_staat` → method lookup
- DBA treaties have article-specific methods
- **Impact**: Overly simplified — all income for a country gets the same treatment
- **Fix**: Expand DBA_METHOD_MAP to `(staat, einkunftsart)` tuples

**GAP-H2: 105 pre-existing test failures**
- `test_ring_regression_kampagne.py`: 15 failed (g2-NameError in gesamt scheibe)
- `test_solz_ring.py`: 3 failed (jsonschema)
- Various e2e: 87 failed (jsonschema `grund` enum mismatch)
- All confirmed to exist on clean-base commit
- **Fix**: Systematic jsonschema update (grund enum values, ergebnis schema)

**GAP-H3: Einkunftsart-spezifische Fremdarten-Guard missing for several types**
- `kein_sonstige` ∈ `AN_GESAMT_FLAGS` but its negation only covers §22 Nr.1 (Renten)
- Other sonstige Einkünfte (§22 Nr.3, §23) are NOT caught by the flag
- **Impact**: A user with §23 Einkünfte could accidentally route through an_gesamt
- (Mitigated by fact that no §23 wiring exists in an_gesamt scheibe)

**GAP-H4: DBA p32b progression route not end-to-end verified**
- `g["p32b_progressionseinkuenfte"] = dba_ausl` is set (code exists)
- But no Ring-Diff test proves the value actually affects tax calculation
- p32b is a Post-Engine wrapper — timing/scope interaction unverified

### MEDIUM

**GAP-M1: Einkunftsart-spezifisches binding/validation missing**
- Fields that are mandatory for certain betriebsarten are not enforced
- `gewinn_betriebsart` determines which einkuenfte_gewinn fields are required
- Currently: all fields optional, absent→0 → no validation

**GAP-M2: Partner fields inconsistent across scheiben**
- Person B KV/PV, VOR, §33b fields exist in RENTNER_FELDER but not in AN_GESAMT_FELDER
- This is intentional (an_gesamt is pure-AN MVP) but undocumented
- Could cause confusion when comparing Scheiben output

**GAP-M3: No Ring-Diff tests for 6 of 61 accessor functions**
- 55 of 61 accessor functions have at least partial test coverage
- 6 functions are called from api.py but never tested in isolation
- Includes: `catala_p10_kist`, `catala_mitunternehmer_einkuenfte`, `catala_p21_2_verbilligt`

**GAP-M4: Erreichbarkeits-Gate covers askable→SCHEIBEN but not SCHEIBEN→reader**
- Test `test_erreichbarkeit_gate.py` verifies: every askable feld ∈ SCHEIBEN.felder
- But does NOT verify: every feld in SCHEIBEN.felder has a reader in the calculation
- A field could be POSTable but silently ignored by the Ring

### LOW

**GAP-L1: Code comment staleness**
- `api.py:51`: "AfA-Zweig (> 800... ) ist ungebunden → der Guard sperrt" — OUTDATED
- Stale since A6-L2 implementation (`9e42529` + `21f1134`) added Nutzungsdauer routing

**GAP-L2: 85% runner.py coverage with 88 missed lines**
- 15 lines are Catala engine unavailable fallbacks (valid)
- ~20 lines are edge cases (0/negative zvE, absence)
- ~30 lines are Stufe-2 deferred logic (expected)
- ~20 lines are genuinely uncovered (should have tests)

**GAP-L3: No load/performance tests**
- `lade_fall()` reads JSON from filesystem per request
- `_scheibe_felder()` processes binding YAML per request
- No LRU cache warming, no concurrent request testing

### NEGLIGIBLE

**GAP-N1: Scratchpad directory contains ~50 files of development artifacts**
- Commit messages, draft yamls, debug scripts
- Not harmful but untracked and growing

**GAP-N2: LLM chat client hardcoded to OpenRouter, no fallback provider**
- `llm_client.py` references specific OpenRouter endpoints
- If provider is unavailable, LLM chat feature is completely broken
