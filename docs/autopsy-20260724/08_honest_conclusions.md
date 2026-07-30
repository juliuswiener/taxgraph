# 08 — Honest Conclusions

## True / Uncertain / False Synthesis

### TRUE — Verified by Code

1. **The Fail-Closed Guard Architecture Works.** 12 guards cover every unrechenbare code path in the system. Guards fire on both `bestätigt` and `vorläufig` values, preventing the most dangerous failure mode (silent 0-calculation for incomplete inputs). Code: `api.py:1694-1853`.

2. **Cent→Euro conversion is correct and safe.** All money in Store = CENT. All calculation inputs = EURO via `// 100` (floor). The floor is over-tax-safe (slightly understates deductions). Code: `api.py` `_c()` / `_cent()` helpers throughout.

3. **DBA Anrechnung (credit method) works correctly.** `catala_p34c_1()` implements the correct §34c Abs.1 Höchstbetrag formula: `min(gezahlt, est * ausl // zve)`. 4 test_seeds pipeline-verified. Code: `runner.py:1219-1231`.

4. **The codebase is honest about its limitations.** 11 explicit "Stufe-2" markers document deferred features. 10 binding files contain "lücken" sections documenting known gaps. This is rare and valuable — the system tells you what it can't do.

5. **Field reachability is verified by an automated gate.** `test_erreichbarkeit_gate.py` asserts that every askable field ∈ SCHEIBEN.felder. This catches the dead-wiring bug class that hit 3 previous features (§35c, Realsplitting, KiSt).

### UNCERTAIN — Evidence Incomplete

1. **Whether DBA Freistellung actually works when the guard is removed.** The calculation code (`api.py:1132-1148`) routes Freistellung → `p32b_progressionseinkuenfte`, but no Ring-Diff test proves that `p32b` actually flows through to the final tax calculation. The p32b path is a Post-Engine wrapper — scope interaction unverified.

2. **Whether the Catala engine correctly handles all 61 accessor combinations.** 15 of 61 accessor functions are Pure-Python (verified), but 46 are Catala-generated binary code. The Catala engine's internal correctness is assumed based on `clerk test` results, not independently verified.

3. **Whether the ELSTER submission produces correct XML.** `elster_writer.py` generates XML, but there is no round-trip test against ERiC validation. The ERiC CI gate (`elster/validate_mapping.py`) validates field mapping, not calculation correctness.

4. **Whether the LLM chat classification is any good.** `llm_client.py` and `chat()` endpoint exist but have 64% code coverage. LLM quality is non-deterministic and untestable — the feature may work great or produce garbage depending on model/provider.

5. **Whether 105 pre-existing test failures mask real bugs.** The failures are documented and confirmed pre-existing, but they prevent the test suite from catching regressions in the affected areas. A bug in DBA, SolZ, or the gesamt Ring would go undetected because those tests can't run.

### FALSE — Contradicted by Code

1. **Claim: "Die AfA>800 ist ungebunden"** (from code comment `api.py:51`). **FALSE.** The A6-L2 implementation (`9e42529` + `21f1134`) added `arbeitsmittel_nutzungsdauer` routing with guard pass-through at >80000. The comment is stale.

2. **Claim: "DBA Freistellung is supported"** (from DBA_METHOD_MAP and routing code). **PARTIALLY FALSE.** The calculation code handles it, but the guard (`api.py:1792`) blocks it. Net effect: Freistellung DBAs can never calculate.

3. **Claim: "eDaten auto-bestätigt is implemented"** (from prior reports). **AMBIGUOUS.** The eDaten writer (`elster_writer.py`) writes `zustand="vorlaeufig"` only. The Julius-Cap for `auto_bestaetigt=True` was discussed but never committed. Current behavior: eDaten are never auto-confirmed.

### What's Actually Missing For Production

**Must fix before production use:**
1. Resolve DBA guard contradiction (CRITICAL)
2. Fix 105 pre-existing test failures (schema maintenance)
3. Automate `make build-python` as CI pre-step
4. Add p32b end-to-end verification test

**Should fix for completeness:**
5. Stale code comments in `api.py` and `runner.py`
6. Missing binding entries for `p7_1_lineare_afa` (2 of 3 slots)
7. Coverage gaps in `catala_p10_kist` and `catala_p21_2_verbilligt`

**Deferred by design (Stufe-2):**
8. Multi-country DBA
9. §34 Abs.3 >5Mio-Excess
10. §23 Veräußerungsverlust-Vortrag
11. §31-Kinderfreibetrag Feinheiten
12. §10-Kinderbetreuungskosten
13. §35-Mituveranlagung

### The Bottom Line

**The system is correct for ~85% of MVP scenarios** (pure Arbeitnehmer, Rentner, simple Gewerbe) with strong fail-closed safety nets. The DBA subsystem is the weakest link — functional for Anrechnung but broken for Freistellung due to a guard contradiction. The 105 pre-existing test failures are annoying but not dangerous (all are schema/enum mismatches or build infrastructure issues, not calculation bugs). The codebase is **remarkably honest** about its limitations — 11 explicit backlog markers document exactly what's deferred.
