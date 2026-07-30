# 02 — Guard System Analysis

## The 12-Guard Fail-Closed Safety Net

**Source**: `api.py:1694-1853` (`_an_gesamt_sperrgrund()`)

### Guard Inventory

| # | Guard Name | Trigger Condition | Scheiben | Severity if bypassed |
|---|-----------|-------------------|----------|---------------------|
| 1 | ausland_dhf | dhf_im_inland=False + dhf>0 | all | Over-tax (unrechenbare Auslands-dHf) |
| 2 | dhf_tatbestand_offen | dHf-Bedingungen unbestätigt | all | Over-tax |
| 3 | verpflegung_reduktion_offen | Verpflegungstage >0 ohne safe-Guard | all | Under-tax (falsche Pauschale) |
| 4 | ausland_uebernachtung | Übernachtung nicht Inland | all | Over-tax |
| 5 | uebernachtung_tatbestand_offen | Übernachtungs-Bedingungen unbestätigt | all | Over-tax |
| 6 | uebernachtung_zeitraum_offen | 48-Monats-Schwelle überschritten | all | Over-tax |
| 7 | arbeitsmittel_afa_ueber_gwg_offen | AM>800 ohne Nutzungsdauer, oder ≤800 ohne Wahlrecht | all | Under-tax (kein Abzug) |
| 8 | partner_konsistenz_offen | Partner-§33b ohne Zusammen | all (cross-scheibe) | Under-tax |
| 9 | alleinerziehend_konsistenz_offen | §24b+Zusammen | all (cross-scheibe) | Under-tax |
| 10 | abs3_ueber_5mio_offen | VÄ-Gewinn>5Mio + §34 Abs.3 | both | Over-tax (falscher Satz) |
| 11 | dba_freistellung_offen | dba_methode==freistellung | gesamt/rentner | **CRITICAL BUG (see 03)** |
| 12 | dba_multi_country_offen | dba_mehrere_staaten=True | gesamt/rentner | Over-tax (nur single-country) |

**Plus 3 an_gesamt-only guards** (kinder_gehoeren_in_gesamt, verlustvortrag_gehoert_in_gesamt, progression_gehoert_in_gesamt).

### Architecture Assessment

**What works**: The guard system is the strongest part of the codebase. Every unrechenbare code path is caught BEFORE the calculation runs. The `is True` / `is not True` pattern distinguishes explicit user choice from absent values correctly. The `_positiv()` helper correctly identifies numeric positive values (excluding bools, excluding 0/None/absent).

**Critical design decision (correct)**: Guards fire on ALL values (including `vorläufig`), not just `bestätigt`. This prevents the most dangerous failure mode: a `vorläufig` value quietly entering the calculation as 0 (silent over-tax). The user must confirm their answers before the Ring calculates.

### Coverage Gap: No Guard for Progressionsvorbehalt + DBA-Freistellung Combination

**Code reference**: `api.py:1792` — `dba_freistellung_offen` blocks ALL freistellung cases, even though the calculation layer (`api.py:1142-1148`) now handles them via p32b_progressionseinkuenfte.

This is a **contradiction**: the calculation code says "Freistellung is routable" but the guard says "Freistellung is forbidden." The net effect is that Freistellung DBAs (AT, US) can never actually calculate — they always return `grund="dba_freistellung_offen"` with `zahl_cent=null`.
