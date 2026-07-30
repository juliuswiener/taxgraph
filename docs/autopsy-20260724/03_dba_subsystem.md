# 03 — DBA Subsystem

## The 11-Country Routing: What Works, What Doesn't

### DBA_METHOD_MAP (from code, `api.py:238-252`)

```python
DBA_METHOD_MAP = {
    "at": "freistellung",  "ch": "anrechnung",   "dk": "anrechnung",
    "es": "anrechnung",    "fr": "anrechnung",   "gb": "anrechnung",
    "lu": "anrechnung",    "nl": "anrechnung",   "pl": "anrechnung",
    "tr": "anrechnung",    "us": "freistellung",
}
```

**Status**: **TRUE** — the map exists and is used by the calculation layer.

### Routing Logic (from code, `api.py:1132-1150`, `api.py:1458-1472`)

```python
dba_method_auto = DBA_METHOD_MAP.get(dba_staat_raw.lower() if dba_staat_raw else "") or "anrechnung"
if dba_method_auto == "freistellung":
    dba_anrechnung = 0
    g["p32b_progressionseinkuenfte"] = dba_ausl
else:
    dba_anrechnung = runner.catala_p34c_1({...})
```

**Status**: **TRUE** — the routing correctly distinguishes Freistellung (→0 anrechnung, p32b) vs Anrechnung (→catala_p34c_1).

### The Guard Contradiction (CRITICAL, `api.py:1792`)

```python
if dba_methode == "dba_freistellung":
    return "dba_freistellung_offen"
```

This guard fires BEFORE the calculation routing can execute. It blocks ALL freistellung cases, including AT and US which are mapped as freistellung in DBA_METHOD_MAP.

**The contradiction**: 
- Calculation layer (`api.py:1132`): "Freistellung is routable via p32b_progressionseinkuenfte"
- Guard layer (`api.py:1792`): "Freistellung is forbidden, return error"

**Actual behavior**: AT and US DBAs always return `grund="dba_freistellung_offen"` with `zahl_cent=null`.

**Resolution options**:
1. Remove the guard (allow Freistellung to calculate) — needs p32b_progression to work correctly in the catala_gesamt scope
2. Keep the guard (Freistellung = MVP limitation) — remove the routing code (dead code)
3. Gate behind a confirmed flag — user must confirm they understand Freistellung consequences

### What's Missing From the DBA Subsystem

**GAP-001 (HIGH)**: No per-einkunftsart routing. DBA treaties distinguish between Einkunftsarten (Art. 6 Immobilien, Art. 7 Betriebsstätte, Art. 10 Dividenden, Art. 15 Arbeit). The current implementation uses a single `dba_staat` → `dba_methode` lookup — all Einkunftsarten for a country get the same treatment.

**GAP-002 (HIGH)**: No DBA-Progressionseinkuenfte actual calculation. The Freistellung path sets `g["p32b_progressionseinkuenfte"] = dba_ausl`, but `p32b_progressionseinkuenfte` is defined as an OPTIONAL Aggregat-Feld. The p32b catala function (`runner.py:catala_p32b_1`) is a separate Post-Engine wrapper — it's unclear whether `dba_ausl` actually flows through to the final tax calculation.

**GAP-003 (MEDIUM)**: No DBA-Art.23 Abs.1 lit. c territoriality/Rückfallklausel. Some treaties (AT, GB) have an Aktivitätsklausel that falls back to Anrechnung instead of Freistellung if no active business is present. The simplified `DBA_METHOD_MAP` cannot express this.

**GAP-004 (LOW)**: `dba_multi_country_offen` blocks ALL multi-country cases. Real tax returns often involve income from multiple DBA countries. The guard correctly fails-closed (zahl_cent=null rather than silent zero), but the feature is completely unavailable.

### Field Coverage

All DBA-related fields (`dba_staat`, `dba_methode`, `dba_mehrere_staaten`, `dba_gezahlte_auslaendische_steuer`, `dba_auslaendische_einkuenfte`, `dba_abzug_statt_anrechnung`) are registered in `GESAMT_DBA` and included in both `RENTNER_FELDER` (Z.280) and `gesamt.felder` (Z.354).

**Status**: **TRUE** — field reachability verified in code and schemas.

### catala_p34c_1 Accessor (from code, `runner.py:1219-1231`)

```python
def catala_p34c_1(s: dict) -> int:
    gezahlt = int(s["gezahlte_auslaendische_steuer"])
    est = int(s["deutsche_est_inkl_ausl"])
    zve = int(s["zu_versteuerndes_einkommen"])
    ausl = int(s["auslaendische_einkuenfte_staat"])
    if ausl <= 0 or zve <= 0: return 0
    hoechstbetrag = est * ausl // zve
    return min(gezahlt, hoechstbetrag)
```

**Status**: **TRUE** — correctly implements §34c Abs.1 Höchstbetrag formula. 4 test_seeds pipeline-verified.
