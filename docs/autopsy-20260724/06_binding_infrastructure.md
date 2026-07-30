# 06 — Binding Infrastructure

## 17 YAML Files, 10 With Documented Gaps

### File Inventory

| File | Fields | Lücken | Description |
|------|--------|--------|-------------|
| `bindung_an_gesamt.yaml` | ~45 | 0 | Main employee case bindings |
| `bindung_p51a_kirchensteuer.yaml` | 2 | 0 | KiSt confession/bundesland |
| `bindung_p22_nr3.yaml` | 1 | 0 | §22 Nr.3 sonstige Leistungen |
| `bindung_p101_mobilitaetspraemie.yaml` | 0 | 1 | §101 mobility premium (lücken only) |
| `bindung_p10_1_5_gesamt.yaml` | 4 | 2 | §10 Abs.1 Nr.5 Kinderbetreuung |
| `bindung_p23_gesamt.yaml` | 4 | 1 | §23 private Veräußerungsgeschäfte |
| `bindung_p10_1a_realsplitting_gesamt.yaml` | 3 | 0 | §10 Abs.1a Realsplitting |
| `bindung_p32b_gesamt.yaml` | 1 | 1 | §32b Progressionsvorbehalt |
| `bindung_sonder_agb_35a.yaml` | 6 | 2 | §33/§35a exceptions |
| `bindung_p33a_gesamt.yaml` | 4 | 0 | §33a Unterhalt/Ausbildung |
| `bindung_p34c_gesamt.yaml` | 6 | 1 | §34c DBA Anrechnung |
| `bindung_p9_1_nr7_arbeitsmittel.yaml` | 3 | 0 | §9 Arbeitsmittel |
| `bindung_n_vor_gwg.yaml` | 50+ | 16 | Base labor bindings (largest file) |
| Others (4 files) | ~20 | ~5 | Various smaller bindings |

**Total**: ~150 bindungs + ~30 lücken entries across 17 files.

### Binding Schema (from code, `test_bindungstabelle.py`)

Each binding entry requires:
- `feld_id` — the field identifier
- `quelle` — {`regel_id`, `geltungsbedingung`, `signatur_slot`}
- `typ` — data type (int, bool, string, etc.)
- `anker_ref` — source reference
- `elster_kz` — ELSTER Kennzahl mapping (may be null)
- `hilfe_kurz` — short help text
- `beispielwert` — example value
- `askable` — whether the field appears in /fragen
- `vz_gueltigkeit` — validity period

### The "Lücke" Pattern

"Lücken" document gaps where the engine expects a slot but no binding exists. These are explicit concessions:

**Example** (`bindung_p32b_gesamt.yaml`):
```yaml
luecken:
  - regel_id: p32b_progressionseinkuenfte
    signatur_slot: anrechenbare_auslaendische_steuern  # known gap
```

This is honest — the system documents what it knows it doesn't handle. 10 of 17 files use this pattern.

### Known Binding Gaps That Matter

**BIND-GAP-1**: `p7_1_lineare_afa` expects `anschaffungs_herstellungskosten`, `nutzungsdauer_jahre`, `anzurechnende_monate` but only `nutzungsdauer_jahre` is bound. The other 2 slots are in lücken.

**BIND-GAP-2**: `dba_staat` enum was missing DK, LU, TR until `1ec4f5d` — now fixed. Enum now contains 14 entries.

**BIND-GAP-3**: `am_gwg_sofortabzug_gewaehlt` binding was missing `anker_ref` — this caused 1 binding validation failure. Fixed in `2468a6e`.
