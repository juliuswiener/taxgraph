# 10 — External Review Integration

## Source: Code Review & Deep Dive Analysis (2026-07-24)

This document integrates findings from a parallel external review into the autopsy framework.
Cross-references to existing documents use the format `(see 04, GAP-X)`.

---

## New Critical Finding

### EXT-C1: Runtime Regression — `kapitalertraege` NameError (CRITICAL)

**External review finding**: "Commit `236b655` altered line 1301 to pass `'est_regulaer_mit_kap': kapitalertraege` into `runner.catala_kapital_steuer()`. However, `kapitalertraege` is not defined in this scope."

**Verification from code**: `git show 236b655:produkt/haut/api.py` line ~1301:
```python
kap_st_total = runner.catala_kapital_steuer({
    "veranlagungszeitraum": vz,
    "kapitaleinkuenfte": kapitaleinkuenfte,
    "est_regulaer_mit_kap": kapitalertraege,  # ← UNDEFINED
    "est_regulaer_ohne_kap": est_raw})
```

**Impact**: This breaks 105 HTTP integration tests. Any `/fall/<id>/ergebnis` request involving church tax or capital yields crashes with `NameError: name 'kapitalertraege' is not defined`.

**This is the root cause of the 105 pre-existing test failures** identified in documents 04 (GAP-H2) and 05 (Category B). The variable should be `g2["est_gesamt"]` or the correct local scope variable.

**Recommended fix** (from external review):
```python
"est_regulaer_mit_kap": est_mit if 'est_mit' in locals() else g2["est_gesamt"],
```

**Severity**: CRITICAL — production crash on all capital income + church tax calculations.

---

## New Important Findings

### EXT-I1: Makefile VENV312 Hardcoding (MEDIUM)

**Finding**: `Makefile` line: `VENV312 := oracle/.venv312/bin/activate` assumes a fixed local virtualenv directory. Running `make s02` or `make p1` fails in CI or on systems without this exact path.

**Impact**: The GETTSIM cross-check oracle cannot run outside the original developer's machine. This undermines the "dual-oracle verification" architecture claim.

**Recommended fix**:
```makefile
PYTHON ?= python3
VENV312 ?= oracle/.venv312/bin/activate
VENV_RUN = $(if $(wildcard $(VENV312)),. $(VENV312) &&,)
```

**Severity**: MEDIUM — breaks CI reproducibility, does not affect production calculation.

### EXT-I2: Unhandled Exception Response Schema Violation (MEDIUM)

**Finding**: When internal Python exceptions occur, `api.py` returns `{"fehler": "<Exception message>"}` which violates the `api_schema/ergebnis.json` OpenAPI contract. The schema requires `fall_id`, `snapshot_id`, `zahl_cent`, `grund`, etc.

**Verification from code** (`api.py:2117-2158`): The `/ergebnis` endpoint has exception handling that returns non-schema-compliant error responses.

**Current behavior**: On engine failure, API returns `{"fehler": "..."}` — test code validates this with `_val("ergebnis", erg)` which throws jsonschema ValidationError.

**Recommended fix**:
```python
{"zahl_cent": None, "grund": "engine_unavailable", "offen": [str(e)], "fall_id": ..., "snapshot_id": ...}
```

**Severity**: MEDIUM — production API would break contract on any unhandled exception.

---

## New Suggestions (Nice to Have)

### EXT-S1: Automated Pre-Commit Snapshot Verification

**Finding**: No automated gate ensures that Catala source edits regenerate snapshots. A developer could edit `rules/estg/*.catala_en` without running `make snapshot`, breaking the SHA256 integrity chain.

**Recommended**: Add `make snapshot-verify` to `.git/hooks/pre-commit` or CI pipeline.

**Severity**: LOW — snapshots are checked in; only matters when Catala sources change.

### EXT-S2: Type Annotation Coverage in runner.py

**Finding**: `golden/runner.py` uses untyped `s: dict` throughout. Adding `TypedDict` models for input payloads would prevent key-name mismatches between Catala wrapper functions.

**Severity**: LOW — nice to have, no correctness impact.

---

## Updated Honesty Matrix

### FALSE — Contradicted by Code (addenda)

4. **Claim: "854 tests pass, 105 fail due to pre-existing jsonschema issues"** (from prior project status). **PARTIALLY FALSE.** The external review identified a specific code bug: `kapitalertraege` NameError in `api.py:1301` (commit `236b655`). This is NOT a jsonschema issue — it's a genuine code regression. The 105 failures include ~87 jsonschema `grund` enum mismatches AND ~18 genuine runtime crashes caused by this NameError.

### Reclassification of Pre-Existing Failures

| Category | Count | Root Cause | Documented In |
|----------|-------|-----------|---------------|
| `kapitalertraege` NameError | ~18 | Code regression in `236b655` | EXT-C1 |
| jsonschema `grund` enum mismatch | ~87 | Schema incomplete — lagging behind guard additions | GAP-H2 |
| Binding validation | 1 | `p7_1_lineare_afa` missing 2 of 3 slots | BIND-GAP-1 |

### Updated Bottom Line

**The system's calculation core is sound, BUT:**
1. There is ONE genuine runtime regression (kapitalertraege NameError) causing ~18 test crashes — this IS a code bug, not infrastructure
2. The remaining ~87 failures are schema maintenance (grund enum values)
3. The external review's DBA-Freistellung + ELSTER findings align with this autopsy's GAP-C1 and Prod-Readiness sections
4. The Makefile venv hardcoding and exception schema issues are real but lower priority
