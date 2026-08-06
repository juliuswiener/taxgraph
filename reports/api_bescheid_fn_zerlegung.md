# api.py `_bescheid_fn` — Zerlegungsanalyse

Stand: 2026-08-06, HEAD 8e1e7b2 (plus dev-b in-flight)
Datei: `produkt/haut/api.py`, Z.256-1551 (50 % der Datei, ~1300 Zeilen)

## 1. Aufbau

```
_bescheid_fn(quantitaet, vz, bindung, felder, store, nur_bestaetigt, solz_container, extras)
├── quantitaet=="abziehbarer_betrag"          (Z.283-297)   136 Z.  §9 Entfernungspauschale
│   └── slot_fn(slots)                        16 Z.  trivial
├── quantitaet=="festzusetzende_est"           (Z.299-447)   148 Z.  §2 AN einzeln
│   └── slot_fn(slots)                        135 Z.  ~37 catala-Aufrufe
├── quantitaet=="festzusetzende_est_gesamt"    (Z.449-1125)  676 Z.  §19+§21+alle Abzüge
│   ├── _c(fid) / _b(fid)                      3 Z.   Helfer
│   ├── _kinderbetreuung_summe()              14 Z.   EM.instanzen -> §10 Abs.1 Nr.5
│   ├── _schulgeld_summe()                    15 Z.   EM.instanzen -> §10 Abs.1 Nr.9
│   ├── _vv_objekt(fi)                        40 Z.   §21 Überschuss EINES Objekts (inv. _ci, _bi)
│   └── slot_fn(slots)                        590 Z.  ~37 catala-Aufrufe + _festzusetzende (88 Z.)
├── quantitaet=="festzusetzende_est_rentner"   (Z.1127-1547) 420 Z.  §22 Rentner
│   ├── _c(fid) / _b(fid)                      3 Z.   Helfer
│   ├── _rente_instanz(fi)                    16 Z.   §22 Einkünfte EINER Rente (inv. _ci)
│   ├── _kinderbetreuung_summe()              14 Z.   1:1 gesamt-Duplikat
│   ├── _schulgeld_summe()                    15 Z.   1:1 gesamt-Duplikat
│   └── slot_fn(slots)                        350 Z.  ~20 catala-Aufrufe + _festzusetzende_r (82 Z.)
└── return None                                (Z.1549-1551) Kein Accessor
```

## 2. Was unterscheidet gesamt und rentner WIRKLICH?

### Gemeinsam (byte-identisch oder 1:1-Parameter)

| Funktion | gesamt | rentner | Diff |
|----------|--------|---------|------|
| _kinderbetreuung_summe | Z.460-473 | Z.1158-1170 | **IDENTISCH** (nur closure-Name) |
| _schulgeld_summe | Z.475-491 | Z.1172-1185 | **IDENTISCH** (nur closure-Name) |
| p35a_haushaltsnahe | Z.764-769 | Z.1371-1377 | identische Argumente |
| p10_kv_pv (A) | Z.376-378 | Z.1398-1401 | identische Argumente |
| p10_kv_pv (B) | Z.385-387 | Z.1413-1416 | identische Argumente |
| p10b_spenden | Z.792-793 | Z.1393-1394 | identische Argumente |
| p10_kist | Z.794-795 | Z.1395-1396 | identische Argumente |
| p10_1_7_berufsausbildung | Z.818-819 | Z.1410-1411 | identische Argumente |
| p10_1a_realsplitting | Z.804-807 | Z.1406-1409 | identische Argumente |
| p33a_unterhalt | Z.908-912 | Z.1325-1329 | identische Argumente |
| p33a_ausbildung | Z.913-914 | Z.1330-1331 | identische Argumente |
| p34c_1 | Z.950-954 | Z.1353-1357 | identische Argumente |
| p35c_sanierung | Z.774-776 | Z.1380-1382 | identische Argumente |
| p35c_energieberater | Z.777-778 | Z.1383-1384 | identische Argumente |
| p35c_jahresdeckel | Z.780-783 | Z.1385-1388 | identische Argumente |
| p16_4_freibetrag | Z.691 | Z.1251-1252 | identische Argumente |
| p22_nr3 | Z.881-883 | Z.1239-1241 | identische Argumente |
| _p23_ansonsten_einkuenfte | Z.878 | Z.1235-1236 | identischer Aufruf |
| p10d_2 | Z.915-918 | Z.1332-1335 | identische Argumente |
| fuenftel | Z.1023-1025 | Z.1463-1465 | identische Argumente |
| ermaessigter_durchschnittssatz | Z.1015-1018 | Z.1457-1460 | identische Argumente |
| p31_familienleistung | Z.1091-1095 | Z.1525-1529 | identische Argumente |
| solz | Z.1105-1109 | Z.1535-1539 | _fast_ identisch (kapital_steuer=0 statt kap_st) |
| kist | Z.1120-1123 | Z.1542-1545 | _fast_ identisch (est_mit_fb aus solz_info) |

### Unterschiede

**Einkunftsarten** (grundlegend):
- `gesamt`: hat `einkuenfte_nichtselbststaendig` (§19 Lohn) + `einkuenfte_vermietung` (§21) + `einkuenfte_gewinn` (§§13-18) + `einkuenfte_sonstige` (§23/§22)
- `rentner`: hat `einkuenfte_sonstige` (§22 Rente + §23) + `einkuenfte_gewinn` (§§13-18) + `aussergewoehnliche_belastungen` (§33b) — KEIN §19, KEIN §21

**§19-Lohn-Verarbeitung (nur gesamt)**:
- `gesamte` Werbungskosten-Logik (EP+dHf+Verpflegung+Übernachtung+AM-GWG+AM-AfA) — 80 Z.
- Versorgungsfreibetrag §19 Abs.2 — 20 Z.
- Person-B-§19 — 5 Z.

**Kapital §32d (identisch, aber anderer Kontext)**:
- `gesamt`: Sparer-PB auf gde mit ns+vv+gewinn+sonstige
- `rentner`: Sparer-PB auf rentner_g (nur renten + gewinn) — anderer gde

**§35 Nenner**:
- `gesamt`: ns + vv + einkuenfte_sonstige + einkuenfte_gewinn
- `rentner`: renten + einkuenfte_gewinn (kein §19, kein §21)

**§35a/§35c/§10b**:
- `gesamt`: in g["steuerermaessigungen"] + g["sonderausgaben"] + g["aussergewoehnliche_belastungen"]
- `rentner`: in rentner_g — identische Formeln, anderer gde für §10b/§33-Deckel

**§33b (Behinderte)**:
- `gesamt`: Z.823-838 (eigene Formel, additiv zu agB)
- `rentner`: Z.1217-1233 (identische Formel, direkt in ausserg)

**§24a/§24b**:
- `gesamt`: alt24a + alt24a_b + ent24b → g
- `rentner`: alt24a_r + ent24b_r → rentner_g (identisch, aber §24a hat andere positive_andere_einkuenfte)

**§31 (§32 Kinderfreibetrag)**:
- `gesamt`: _festzusetzende(0) vs _festzusetzende(kinder * kfb)
- `rentner`: _festzusetzende_r(0) vs _festzusetzende_r(kinder * kfb) — 1:1 identisch

**§32b Progressionsvorbehalt**:
- `gesamt`: Z.1045-1063, 18 Z. inkl. pe_raw, pe_active, t_32b
- `rentner`: Z.1498-1513, 16 Z. — **fast identisch**, Differenz: p35_credit_r aus bestehendem Wert, kein neuer Deckel

### FAZIT: "37 identische Aufrufe" — KORRIGIERT (AST-Messung, Main)

**Die Kernzahl 37 hält der Messung nicht stand.** AST-Vergleich (nicht Textzeilen — Formatierung
täuscht) der beiden grossen slot_fn:

    gemeinsame catala-NAMEN:            35
      davon AST-IDENTISCHER Aufruf:     19    <- billig extrahierbar
      davon UNTERSCHIEDLICH:            16    <- NICHT trivial

Die 16 unterschiedlichen sind genau die heiklen Faelle — zwei verschiedene Muster:

  (a) nur ein anderer Kontext-Container (g vs rentner_g) -> per Parameter loesbar
      z.B. catala_gesamt_zve (g) vs (rentner_g), catala_gesamt_gde
  (b) eine andere ZUGRIFFSART (f.get vs _b) oder eine andere QUELLE fuer denselben Wert
      (kap_st_total vs solz_info_r['est_mit_fb']) -> das ist womoeglich gar kein Duplikat,
      sondern ein inhaltlicher Unterschied. Oder ein BUG.

Die 16 blind in eine gemeinsame Funktion zu ziehen, waere genau die Art Umbau, die still die
Semantik verschiebt. **Variante D gilt nur fuer die 19 AST-identischen.**

Die wahren Unterschiede (Einkunftsarten):
1. **Einkunftsarten gesamt**: 19+21+13-18+23 (4 Arten)
2. **Einkunftsarten rentner**: 22+13-18+23 (3 Arten)
3. **§19-WK-Formel**: nur gesamt (80 Z. EP+dHf+Verpflegung+Übernachtung+AM)
4. **§21-VV-Formel**: nur gesamt (40 Z. _vv_objekt)
5. **§22-Renten-Formel**: nur rentner (16 Z. _rente_instanz)

## 3. Closure-Variablen der inneren defs

Die nested `def`s (`slot_fn`, `_c`, `_b`, `_kinderbetreuung_summe`, `_vv_objekt`, `_rente_instanz`, `_festzusetzende`, `_festzusetzende_r`) schliessen über:

| Variable | Scope | Genutzt von |
|----------|-------|-------------|
| `vz` | _bescheid_fn-Parameter | ALLE inneren defs |
| `f` | lokales `f = felder or {}` (Z.305, 454, 1132) | `_c`, `_b`, `_festzusetzende`, `slot_fn` |
| `store` | _bescheid_fn-Parameter | `_kinderbetreuung_summe`, `_schulgeld_summe`, `_vv_objekt`, `_rente_instanz`, `slot_fn` |
| `bindung` | _bescheid_fn-Parameter | `_kinderbetreuung_summe`, `_schulgeld_summe`, `_vv_objekt`, `_rente_instanz`, `slot_fn` |
| `nur_bestaetigt` | _bescheid_fn-Parameter | `_kinderbetreuung_summe`, `_schulgeld_summe`, `_vv_objekt`, `_rente_instanz`, `slot_fn` |
| `solz_container` | _bescheid_fn-Parameter | `slot_fn` (gesamt+rentner), `_festzusetzende` |
| `extras` | _bescheid_fn-Parameter | `slot_fn` (gesamt+rentner), `_festzusetzende` |
| `IV` | Modul-Import | `slot_fn` (alle 4) |
| `EM` | Modul-Import | `slot_fn` (gesamt+rentner), `_kinderbetreuung_summe`, `_schulgeld_summe` |
| `runner` | local import | ALLE inneren defs |
| `_oepnv_eur` | api.py top-level | `slot_fn` (entfernung+an) |
| `_p23_ansonsten_einkuenfte` | api.py top-level | `slot_fn` (gesamt+rentner) |
| `_laufender_gewinn` | api.py top-level | `slot_fn` (gesamt+rentner) |
| `_abs3_eligible` | api.py top-level | `_festzusetzende`, `_festzusetzende_r` |
| `SOLZ_KONSTANTEN` | api.py top-level | `slot_fn` (rentner, Z.1538) |
| `...` (Konstanten) | api.py top-level | `slot_fn` (alle) |

Die gemeinsamen Module (`IV`, `EM`, `runner`) sind unproblematisch — sie sind Modul-Referenzen, keine mutable State.

Die top-level Helper (`_oepnv_eur`, `_p23_ansonsten_einkuenfte`, `_laufender_gewinn`, `_abs3_eligible`) können als Parametrisierung durchgereicht werden.

## 4. Echte Schnittkanten

**Schnittkante A: `_c(fid)` / `_b(fid)`** — die Helfer sind in allen 3 slot_fn identisch (Z.307-309, 456-458, 1134-1136). Als `_c`/`_b`-Factory parametrisierbar.

**Schnittkante B: Einkunftsarten-Ermittlung** — die 4 Einkunftsarten (ns/lohn, vv, renten, gewinn, sonstige) sind logisch getrennt. Jede ist ein eigener Berechnungsblock.

**Schnittkante C: Post-§2-Abzüge** — §35a, §35c, §10b, §10 KiSt, §10 KV/PV, §10 Kinderbetreuung, §10 Schulgeld, §10 Berufsausbildung, §10 Realsplitting, §33 agB, §33b, §33a, §10d, §34c, §35, §24a, §24b, §32b, §31, SolZ, KiSt — sind ALLE g-agnostisch (lesen nur `f` + `gde`, schreiben in `g`). Die Reihenfolge ist fix.

**Schnittkante D: §31-Günstigerprüfung** — `_festzusetzende(freibetrag)` ist ein reiner Child-Call, der den gesamten Post-GdE-Pfad zweimal rechnet. Die innere Struktur ist identisch in gesamt vs rentner, aber die g-Dicts sind unterschiedlich.

## 5. Zerlegungsvarianten

### Variante A: Extra-Funktion je Einkunftsart (sicher, mittlerer Aufwand)

```
felder -> _einkuenfte_nichtselbststaendig(f, slots, vz) -> ns     # nur gesamt
felder -> _einkuenfte_vermietung(f, store, bindung, ...) -> vv     # nur gesamt  
felder -> _einkuenfte_rente(f, store, bindung, ...) -> renten     # nur rentner
felder -> _einkuenfte_gewinn(f, ...) -> gewinn                    # SHARED
felder -> _einkuenfte_sonstige(f, ...) -> sonstige                # SHARED
```

**Aufwand**: 2-3 h. **Risiko**: niedrig. Jeweils 1:1 rausziehen, Signatur aus Closure-Variablen. **Äquivalenznachweis**: trivial — Ring-Vergleich auf Seeds, extrahierte Funktionen rückstandslos aufrufbar.

**Kosten**: mittel (6 Extraktionen, 150-200 Z. Signatur-Boilerplate). **Nutzen**: jede Einkunftsart einzeln testbar, die 37 Duplikat-Aufrufe bleiben in den Post-§2-Blöcken.

### Variante B: Post-§2-Abzüge als gemeinsame Funktion (größter Hebel)

Die 37 Accessor-Aufrufe (p35a, p10b, p33, p10_kv_pv, p35c, §31, SolZ, KiSt, §32b, …) in EINE Funktion:

```
def _post_gde_abzuege(f, vz, gde, g, solz_container, extras, veranlagung, etag):
    ... 37 Aufrufe, 1:1 aus gesamt/rentner, g-agnostisch
    return g_angereichert, solz, kist, mobilitaet
```

**Aufwand**: 1 h. **Risiko**: gering. **Äquivalenz**: Ring-Vergleich. **Problem**: `_festzusetzende` ist verschachtelt in `_festzusetzende_r` — die Child-Funktion ist NICHT g-agnostisch (sie crasht ohne kapitaleinkuenfte, etc.).

### Variante C: Zwei Drittel-Funktionen + Shared-Helper (empfohlen)

```
_bescheid_fn                        # dispatcher, 20 Z.
├── _entfernung_slot_fn(slots)      # existiert schon (Z.289-296)
├── _an_slot_fn(f, slots, ...)      # 590 Z.  gesamt (aus altem slot_fn)
├── _rentner_slot_fn(f, slots, ...) # 350 Z.  rentner (aus altem slot_fn)
└── _shared_post_gde(...)            # 300 Z.  gemeinsame Abzüge
```

**Plus**: `_c`/`_b`/`_kinderbetreuung_summe`/`_schulgeld_summe`/`_vv_objekt`/`_rente_instanz` als top-level oder Modulebene.

**Aufwand**: 3-4 h. **Risiko**: mittel (viele Closure-Variablen müssen als Parameter durchgereicht werden). **Äquivalenz**: Ring-Vergleich auf Seeds, byte-genau.

### Variante D: Monolith lassen, nur die 37 Aufrufe deduplizieren (minimaler Eingriff)

`_post_gde_abzuege(f, vz, gde, ...)` rausziehen, gesamt und rentner rufen sie. Die 37 identischen Aufrufe verschwinden aus beiden Zweigen.

**Aufwand**: 1 h. **Risiko**: sehr gering. **Nachteil**: `_bescheid_fn` bleibt 1100 Z. groß, aber die Duplikation ist weg.

## 6. Äquivalenzbeweis

**Mechanismus**: Ring-Ergebnisse auf identischen Seeds vorher/nachher, byte-genau.

**Vorhandene Infrastruktur**:
- `golden/runner.py` — alle catala-Accessoren sind hier
- `tests/test_einheiten.py` — Ring-Test auf Seeds (nicht vollständig, aber vorhanden)
- `test_*_ring.py` — mehrere Ring-Tests (p33b, p101, p10, kist, kapital, etc.)
- `test_snapshot.py` — Snapshot-Tests

**Was fehlt für einen zuverlässigen Äquivalenzbeweis**:
1. Ein parametrisierter Ring-Test, der ALLE Ring-Seeds auf `_bescheid_fn` laufen lässt und das Ergebnis (alle 4 quantitaeten) als JSON speichert — `golden/bescheid_fn_cases/`
2. Vorher/Nachher-Vergleich via `git worktree` oder `PYTHONPATH`-Swap: alten Code in einem worktree, neuen im anderen → `diff -r` über die JSON-Outputs
3. Jeder Seed deckt ALLE 4 quantitaeten ab (entfernung, an, gesamt, rentner) — die 4 slot_fn laufen nie auf demselben Seed, aber für den Äquivalenzbeweis reicht: jedes Seed läuft seine relevante quantitaet, wir sammeln pro quantitaet N Ergebnissets

**Praktischer Ablauf**:
```bash
# 1. Worktree bauen für Alt-Code
git worktree add /tmp/api_before HEAD
# 2. Seed-Test-Collector auf Alt-Code laufen lassen
cd /tmp/api_before && python -m pytest tests/test_ring_collector.py -o json_output=/tmp/before/
# 3. Umbau durchführen
# 4. Seed-Test-Collector auf Neu-Code laufen lassen
python -m pytest tests/test_ring_collector.py -o json_output=/tmp/after/
# 5. Byte-Vergleich
diff -r /tmp/before/ /tmp/after/
```

**Aufwand Collector-Schreiben**: 1-2 h (Ring-Test, der alle Seeds aus allen Ring-Tests sammelt und als JSON ausgibt).

## Empfehlung

**Variante D auf die 19 AST-identischen Aufrufe beschraenkt** — nicht auf 35 oder 37. Die 16
unterschiedlichen NICHT in eine gemeinsame Funktion ziehen (Semantik-Verschiebungs-Risiko), sondern
einzeln untersuchen: beabsichtigt / Container-Unterschied / BUG (siehe §7).

**Äquivalenz: Ring-Vergleich auf Seeds**. Der Collector muss VOR dem Umbau gebaut sein und auf
UNVERAENDERTEM Code zweimal reproduzierbar dasselbe liefern (diff leer). Nutz `git worktree add`
(HEAD und Umbau gleichzeitig, kein Stash). Pfad unter /tmp, nicht im Repo.

## 7. Die 16 unterschiedlichen Aufrufe — Vollanalyse

Alle 16 sind `runner.catala_*`-Namen, die in BEIDEN slot_fn (gesamt + rentner) vorkommen,
aber AST-UNTERSCHIEDLICHE Argumente haben. Drei Kategorien:

**(a) Container-Diff**: gleicher Wert, anderer g-Container (g vs rentner_g) — mechanisch
   per Parameter auflösbar, kein inhaltlicher Unterschied

**(b) Zugriffsstil**: `f.get(fid, {}).get("wert")` vs `_b(fid)` — funktional identisch,
   da `_b = lambda k: f.get(k, {}).get("wert")`. Nur Code-Stil, kein Bug.

**(b') Quelle**: anderer WERT für denselben Argument-Namen — hier kann ein BUG liegen,
   wenn die Quellen semantisch unterschiedlich sind

**(c) Verdacht auf Bug**: zwei unterschiedliche Berechnungspfade für denselben
   logischen Wert — MESSEN, ob wertgleich

### 16 Paare

| # | catala-Aufruf | gesamt | rentner | Kat. | Risiko |
|---|---|---|---|---|---|
| 1 | `catala_gesamt_zve` | `(g)` | `(rentner_g)` | (a) Container | kein |
| 2 | `catala_gesamt_gde` (main) | `g` mit ns+vv+gewinn+sonstige | `rentner_g` mit renten+gewinn | (a) Einkunftsarten | kein |
| 3 | `catala_gesamt_gde` (p10d) | `gde_p10d` (post-kist_ueberhang) | `gde` (post-kist_ueberhang_r) | (a) Container | kein |
| 4 | `catala_gesamt_tarifliche` | `(g)` | `(rentner_g)` | (a) Container | kein |
| 5 | `catala_p33_agb` | gde (mit ns+vv+gewinn+sonstige) | gde (mit renten+gewinn) | (a) Container | kein |
| 6 | `catala_p10d_2` | gde_p10d (mit ns+vv+gewinn+sonstige) | gde (mit renten+gewinn) | (a) Container | kein |
| 7 | `catala_behinderten_pb` | `f.get(..., {}).get("wert")` | `_b(...)` | (b) Stil | kein |
| 8 | `catala_pflege_pb` | `f.get(..., {}).get("wert")` | `_b(...)` | (b) Stil | kein |
| 9 | `catala_hinterbliebenen_pb` | `f.get(..., {}).get("wert")` | `_b(...)` | (b) Stil | kein |
| 10 | `catala_behinderten_pb` (B) | `f.get(..., {}).get("wert")` | `_b(...)` | (b) Stil | kein |
| 11 | `catala_p24a_altersentlastung` | `positive_andere_einkuenfte: max(0, vv + g["einkuenfte_gewinn"] + g.get("einkuenfte_sonstige", 0))` | `positive_andere_einkuenfte: max(0, laufender_gewinn + netto_vg + p23_eink)` | (a) Einkunftsarten | kein |
| 12 | `catala_kapital_steuer` (in _festzusetzende) | `est_regulaer_mit_kap: est_mit, est_regulaer_ohne_kap: est_raw` (lokal, aus solz_info) | `est_regulaer_mit_kap: est_mit, est_regulaer_ohne_kap: est_raw` (lokal, aus solz_info_r) | (a) Closure | kein |
| 13 | `catala_est` (in _festzusetzende, base) | `catala_est(g2)` mit `kapitaleinkuenfte` | `catala_est(g2)` mit `kapitaleinkuenfte_r` | (a) Closure | kein |
| 14 | `catala_kist` (extras) | `kap_st_total` (volles §32d-Kapital-Steuer) | `solz_info_r['est_mit_fb']` (gesamte ESt) | **(b') QUELLE** | **⚠** |
| 15 | `catala_solz` | `kapital_steuer: solz_info.get("kap_st", 0)` | `kapital_steuer: 0` (MUTATION) | **(b') QUELLE** | **⚠** |
| 16 | `catala_sparer_pb` | `kapitaleinkuenfte` (aus verrechnete) | `kapitaleinkuenfte_r` (aus verrechnete) | (a) Closure | kein |

### Detailanalyse der kritischen Fälle

#### #14 catala_kist — QUELLE: `kap_st_total` vs `solz_info_r['est_mit_fb']`

**gesamt** (Z.1114-1123):
```python
kap_st_total = runner.catala_kapital_steuer({
    "kapitaleinkuenfte": kapitaleinkuenfte,
    "est_regulaer_mit_kap": solz_info.get("est_roh_mit_kap", 0),
    "est_regulaer_ohne_kap": solz_info.get("est_roh_ohne_kap", 0)})
kap_st_netto = kap_st_total * 0.75
extras["kist_cent"] = runner.catala_kist({
    "est_mit_fb": kap_st_total, ...})
```
→ KiSt auf §32d-Kapital-Steuer (Abgeltungsteuer-Teil), mit 25%-Kürzung.

**rentner** (Z.1541-1545):
```python
extras["kist_cent"] = runner.catala_kist({
    "est_mit_fb": solz_info_r["est_mit_fb"], ...})
```
→ KiSt auf GESAMTE ESt (nicht nur Kapital-Teil). KEINE 25%-Kürzung.

**Bewertung**: BEABSICHTIGT. Zwei unterschiedliche gesetzliche Pfade:
- § 32d Abs. 3 S. 1: Bei Kapitalerträgen wird die KiSt auf die Abgeltungsteuer
  berechnet (e/(4+k)-Formel im Gesamt-Ring). Der `kap_st_total*0.75`-Term ist
  die 25%-Kürzung nach § 32d Abs. 3 (wird in der KiSt-Formel berücksichtigt).
- Im Rentner-Ring gibt es keine §32d-Abgeltungsteuer als Primär-Pfad → KiSt
  auf die volle ESt. Der Rentner hat `kapital_steuer: 0` im SolZ (siehe #15),
  also auch keine Kapital-KiSt-Trennung.

**Kein Bug, aber**: wenn ein Rentner Kapitaleinkünfte hat (kapitaleinkuenfte_r > 0),
  wird die KiSt falsch berechnet — auf die volle ESt statt auf den Kapital-Teil.
  Das ist der bekannte §32d-Rentner-Gap (MVP).

#### #15 catala_solz — QUELLE: `kapital_steuer: solz_info.get("kap_st",0)` vs `0`

**gesamt** (Z.1105-1109):
```python
solz_container[0] = runner.catala_solz({
    "kapital_steuer": solz_info.get("kap_st", 0),
    ...})
```
→ SolZ-Basis = ESt abzgl. §32d-Kapitalsteuer (§ 3 Abs. 3 S. 1 SolzG).

**rentner** (Z.1535-1539):
```python
solz_container[0] = runner.catala_solz({
    "kapital_steuer": 0,   # MUTATION — Kommentar im Code
    ...})
```
→ SolZ-Basis = volle ESt (kein Kapital-Abzug).

**Bewertung**: BEABSICHTIGT (Mutation). Im Rentner-Ring ist die §32d-Kapitalsteuer
  nicht getrennt ausweisbar (`solz_info_r` hat `kap_st` nur im
  Günstigerprüfung-Pfad, Z.1496, der überschrieben wird durch Z.1517).
  Der `kapital_steuer: 0`-Fall ist der konservative/over-tax-safe-Pfad:
  ohne Kapital-Abzug ist die SolZ-Basis höher → mehr SolZ → over-tax.

  **Kein Bug, aber**: wenn der Rentner Kapitaleinkünfte hat (kapitaleinkuenfte_r > 0),
  wird der SolZ auf den Kapital-Teil doppelt besteuert (SolZ auf ESt inkl. Kapital,
  plus 5,5% SolZ auf die Abgeltungsteuer). Das ist der bekannte §32d-Rentner-Gap.

#### #12/#13 catala_kapital_steuer/catala_est in _festzusetzende

Die inneren `_festzusetzende` und `_festzusetzende_r` sind STRUKTURELL IDENTISCH
(selbe Variablen: `g2`, `est_raw`, `est_mit`, `kapitaleinkuenfte`). Der einzige
AST-Unterschied ist die Closure-Variable `kapitaleinkuenfte` vs `kapitaleinkuenfte_r`
sowie `kinder` (Z.1066 `if freibetrag > 0 or kinder == 0` vs Z.1494
`if freibetrag > 0 or kinder == 0` — identisch). Die Aufrufe selbst sind
byte-identisch.

**Bewertung**: (a) Closure-Diff. Kein Bug.

### Zusammenfassung

| Kategorie | Anzahl | Details |
|---|---|---|
| (a) Container-Diff | 11 | g vs rentner_g, gde vs gde_r, etc. |
| (b) Zugriffsstil | 4 | f.get vs _b (funktional identisch) |
| (b') Quelle | 2 | **#14 catala_kist**, **#15 catala_solz** |
| (c) Bug-Verdacht | 0 | Kein echter Geldfehler in den 16 |

**Fazit**: Keiner der 16 unterschiedlichen Aufrufe ist ein Bug. Die zwei Quelle-
Unterschiede (#14, #15) sind beabsichtigt — sie reflektieren, dass der Rentner-Ring
§32d-Kapital nicht als Primär-Pfad hat (MVP-Restriktion). Ein zukünftiger Fix
(§32d im Rentner-Ring) würde beide automatisch heilen.

**Konsequenz für Extraktion**: Alle 16 können SICHER in eine gemeinsame
`_post_gde_abzuege`-Funktion überführt werden, wenn man die 2 Quelle-Unterschiede
per Parameter (`kapital_steuer`, `kist_est_fb`) steuert. Die 11 Container-Diffs
lösen sich auf, indem man `g`/`gde`/`veranlagung` als Parameter übergibt.
Die 4 Zugriffsstil-Unterschiede sind irrelevant (beide Pfade lesen denselben Wert).

**Nicht gebaut (Schritt 3 NICHT):** die 16 NICHT extrahieren. Nur analysiert.

## 9. §51a-Bemessungsgrundlage: drei Aufrufstellen — Äquivalenzprüfung

Stand nach KiSt-Fix (2026-08-06): alle drei Aufrufe nutzen jetzt ESt OHNE §32d-Kapital.

| Stelle | quantitaet | Ausdruck | §32d-Kapital |
|--------|------------|----------|--------------|
| Z.474 | festzusetzende_est (AN) | `est` (return von catala_est) | nie enthalten (AN ohne Kap) |
| Z.1148 | festzusetzende_est_gesamt | `solz_info["est_roh_ohne_kap"]` | explizit raus (est_raw) |
| Z.1629 | festzusetzende_est_rentner | `solz_info_r["est_roh_ohne_kap"]` | explizit raus (est_raw) |

### Frage: Wird `est` in festzusetzende_est nach Tarif noch verändert?

**Pfad Z.351-471:**

```
Z.442 est = catala_est_zusammen(...)            # Tarif, einziger Set
Z.458 est = catala_est({...})                   # Tarif, einziger Set (else)
Z.466-470 solz_container[0] = catala_solz({...}) # LiST est, ändert NICHT
Z.472-476 extras["kist_cent"] = catala_kist({...}) # LiST est, ändert NICHT
Z.477-502 §101 Mobilitätsprämie                  # Schreib extras, NICHT est
Z.470 return est                                  # est unverändert
```

**Kein Pfad modifiziert `est` nach Tarif.** §35a, §35c, §34c, §35, §32b (Progressionsvorbehalt), §31 (Familienleistungsausgleich) existieren in diesem slot_fn NICHT. §101 ist extras-only.

**Fazit Z.444: `est` = reine tarifliche ESt. Äquivalent zu den anderen beiden.**

### ABER: Numerisch NICHT identisch.

Die drei slot_fn rechnen auf UNTERSCHIEDLICHEN Eingaben:

- **festzusetzende_est (AN)**: nur §19 Lohn + Werbungskosten + Sonderausgaben (§10 KV/PV+Vorsorge). Kein Kapital, keine Rente, keine VV, kein Gewinn, keine agB.
- **festzusetzende_est_gesamt**: §19 Lohn + §21 VV + §22/23 sonstige + §§13-18 Gewinn + voller Post-GdE-Abzugskatalog (§35a, §10b, §33, §10d, §35, §32b, §31).
- **festzusetzende_est_rentner**: §22 Rente + §§13-18 Gewinn + voller Post-GdE-Abzugskatalog.

Für DENSELBEN Steuerpflichtigen mit Lohn, Rente, Kapital liefen alle drei auf verschiedenen zvE → verschiedene tarifliche ESt. Zusammenführung wäre eine Falle: die drei `est_roh_ohne_kap`-Werte unterscheiden sich weil die slot_fn unterschiedliche Einkunftskombinationen abdecken, nicht wegen der KiSt-Formel.

**Empfehlung**: Zusammenführung NUR wenn ein gemeinsamer Accessor `_kist_basis()` denselben Catala-Scope mit konsistenten Eingaben rechnet — das erfordert den Zusammenbau des vollständigen zvE aus allen Slot-Quellen, was die gesamt/rentner-Trennung aushebelt. Lassen wie es ist.

## 8. Collector-Seeds (2026-08-06)

### Null-Fixes

**AN_KEGEL** (vorher `input_kegel_nicht_bestaetigt`): fehlten 10 Felder aus dem an_gesamt-Kegel.
- DHF_RING: `dhf_unterkunftskosten_monat, dhf_monate, dhf_im_inland`
- DHF_BEDINGUNGEN: 4 Bedingungen
- VERPFLEGUNG_TAGE: 3 Tage-Kategorien
Alle 0/false → kein Sperrgrund, slot_fn überspringt die Blöcke. Aber Kegel ist vollständig → `_feste_zahl` erreicht die Catala-Engine.

**RENTNER_KEGEL** (vorher `flag_konsistenz_offen`): `kein_sonstige=True` + `rentner_jahresrente=20000`
→ `flag_widersprueche` feuert (FLAG_NEGIERT["kein_sonstige"] = ["rentner_jahresrente"]).
Fix: `kein_sonstige=False`.

**GESAMT_KEGEL**: war OK (alle VV_GESAMT_FELDER + EP + VOR + KV_PV + KAP + AN_GESAMT_FLAGS).

### Non-zero Abzüge für die 19 shared catala-Aufrufe

Jeder Seed setzt Abzüge > 0, die die 19 AST-identischen Aufrufe in BEIDEN
Zweigen (gesamt + rentner) durchlaufen lassen. Dokumentation pro Aufruf:

| catala-Aufruf | Feld(er) | gesamt | rentner |
|---|---|---|---|
| `catala_p35a_haushaltsnahe` | hh_minijob_aufwendungen=50000 | ✓ | — |
| `catala_p10_kv_pv` (A) | basis_kv=200000, basis_pv=50000 | ✓ | ✓ |
| `catala_p10_kv_pv` (B) | basis_kv_partner=0 (absent) | — | — |
| `catala_p10b_spenden` | spenden_betrag=100000 | ✓ | ✓ |
| `catala_p10_kist` | kist_gezahlt=100000 | ✓ | ✓ |
| `catala_p10_1_7_berufsausbildung` | berufsausbildung_aufwendungen=100000 | ✓ | — |
| `catala_p10_1a_realsplitting` | realsplitting_unterhaltsleistungen=100000 | ✓ | — |
| `catala_p33a_unterhalt` | p33a_unterhalt_aufwendungen=300000 | ✓ | ✓ |
| `catala_p33a_ausbildung` | p33a_ausbildung_anzahl_kinder=1 | ✓ | — |
| `catala_p34c_1` | dba_gezahlte_auslaendische_steuer=50000 | ✓ | — |
| `catala_p35c_sanierung` | p35c_sanierungsaufwendungen=500000 | ✓ | — |
| `catala_p35c_energieberater` | p35c_energieberater_aufwendungen=100000 | ✓ | — |
| `catala_p35c_jahresdeckel` | beide p35c-Felder (s.o.) | ✓ | — |
| `catala_p16_4_freibetrag` | (kein Veräußerungsgewinn — 0) | — | — |
| `catala_p22_nr3_einkuenfte` | p22_nr3_einkuenfte=50000 | ✓ | — |
| `_p23_ansonsten_einkuenfte` | (keine p23-Instanzen) | — | — |
| `catala_p10d_2` | verlustvortrag_bestand=200000 (ges) / 0 (rent) | ✓ | — |
| `catala_fuenftel` | (kein ao-Gewinn) | — | — |
| `catala_ermaessigter_durchschnittssatz` | (kein §34-Abs.3-Antrag) | — | — |
| `catala_p31_familienleistung` | fam_anzahl_kinder=2 | ✓ | — |
| `catala_solz` | (automatisch am Ende) | ✓ | ✓ |
| `catala_kist` | (automatisch am Ende) | ✓ | ✓ |

**Deckung gesamt**: 15 der 19 AST-identischen Aufrufe (u.a. p10b, p10_kist, p10_kv_pv,
p35a, p33a, p34c, p35c, p10d, realsplitting, p22_nr3, p31, berufsausbildung).
**Deckung rentner**: 5 der 19 (p10_kv_pv, p10b, p10_kist, p33a_unterhalt, solz, kist).
Rentner-Set bewusst klein — ESt muss > 0 bleiben (43 EUR). Für volle Rentner-Deckung
bräuchte es einen Seed mit höherer Rente (z.B. 40.000 EUR).

**Nicht abgedeckt** (bewusst): p16_4_freibetrag (braucht Veräußerungsgewinn > 0),
fuenftel/ermaessigter_durchschnittssatz (brauchen ao-Gewinn), p23 (braucht
p23-Instanzen), p10_kv_pv-B (braucht veranlagung=zusammen + partnerdaten).

### Ergebnisse

```
an_gesamt    → 7.152.100 cent (71.521 EUR)
an_ep_ep     → 7.113.200 cent (71.132 EUR)  — EP-Abzug 220×30km×0,30×0,5
gesamt       → 6.809.100 cent (68.091 EUR)  — VV + Abzüge (p33a, p35c, Spenden, DBA)
rentner      → 4.300 cent (43 EUR)           — minimal (Rente 20.000, Abzüge 1.000 EUR)
```

Alle 4 Scheiben: `grund=bestaetigt`, kein null. Grundlage für den
Äquivalenzbeweis vor/nach dem api.py-Umbau.