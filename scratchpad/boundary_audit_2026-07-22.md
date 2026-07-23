# Boundary-Werte-Audit 2026-07-22

Geprüft: JEDER `catala_*`-Accessor mit Schwelle/Freigrenze/Deckel/Floor.  
Flag-Kategorien: FLAG-1 (Code ≠ Docstring), FLAG-2 (kein Boundary-Test +-1), FLAG-3 (Cent/Euro an Schwelle).

## Boundary-Tabelle

| # | Accessor | Schwelle | Operator | Docstring | Boundary-Test ±1? | Flags |
|---|----------|----------|----------|-----------|-------------------|-------|
| 1 | `p22_nr3_einkuenfte` L1504 | 25600 Cent | `betrag_cent < 25600` | "≥ 25600 Cent → voller Betrag" | **JA** (L818-819: 25599→0, 25600→25600) | OK |
| 2 | `p23_freigrenze` L1195 | 1000€ | `gesamt >= 1000` | "≥ 1000 → VOLL" + Wächter 999→0/1000→1000 | **JA** (4 seeds pipeline-verified) | OK |
| 3 | `p6_2_gwg` L554 (Catala) | 800€ netto | `≤ 800` in Catala (api.py Guard L480: `netto > 80000 → 0`) | "≤ 800 → netto, > 800 → 0" | **JA** (test 400/600/800 exact L1147; 801 fehlt) | FLAG-2 (kein >800 1ct Grenze im Ring-Test) |
| 4 | `p16_4_freibetrag` L510 (Catala) | 45000€ / 136000€ / 181000€ | Catala-modul-intern | "45000 − max(0, vg − 136000), 0 bei vg ≤ 0, voll abgeschmolzen ab 181000" | **Teil-JA** (181000→3065000 im Ring-Test L1168) | FLAG-2 (kein 45000/136000 ±1 Cent, kein vg≤0-Test) |
| 5 | `behinderten_pb` L698 | 20 GdB | `if gdb < 20: return 0` | "GdB-Staffel, ab 20" | **JA** (test: 19→0, 45→860 L53-62) | OK |
| 6 | `p33a_unterhalt` L737 | 624€ | `max(0, andere - 624)` | "Schonbetrag 624€" | **JA** (test: 500→0, 2000→1376 L31-45) | OK |
| 7 | `p10_1a_realsplitting` L772 | 13805€ | `min(unterhalt, 13805 + kv_pv)` | "13.805 + kv_pv" | **JA** (test: 15000→13805 L22-38) | OK |
| 8 | `p10_1_5_kinderbetreuung` L763 | 4800€ | `min(aufw × 0.8, 4800)` | "80%, capped 4800€ je Kind" | **JA** (test: 6000×0.8=4800 L767) | OK |
| 9 | `p10_1_7_berufsausbildung` L501 | 6000€ | `min(aufw, 6000)` | "min(aufwendungen, 6000)" | **JA** (test 5999/6000/6001 L27-29) | OK |
| 10 | `p10_kv_pv` L492 | 1900/2800€ HB | Catala-Modul (params) | "1900/2800" | **Teil-JA** (params-basiert) | FLAG-2 (kein ±1 Cent über HB) |
| 11 | `p35c_sanierung` L1467 | 14000/12000€ | `min(roh, 14000/12000)` | "7% bis 14k, 6% bis 12k" | **JA** (Seeds: 20000×0.07=1400 capped, 200000×0.07=14000 exact) | OK |
| 12 | `uebernachtung_abzug` L278 | 48 Monate | `bisher >= 48` | ">= 48 → Kappung auf 1000" | **NEIN** (Guard prüft Überspannung, aber ±1 bei 47/48/49 ungetestet) | **FLAG-2** |
| 13 | `p24a_altersentlastung` L435 | geburtsjahr+65 | `geburtsjahr+65 > vz` | "64+-Gate: geb+65 ≤ VZ" | **Teil-JA** (1958→2023≤2025 L2156) | FLAG-2 (kein ±1 am Geburtsjahr-Gate) |
| 14 | `sparer_pb` L589 | 1000/2000€ | `max(0, kapital - pb)` | "Sparer-PB aus params" | **Teil-JA** (params-basiert) | OK (params-getrieben) |
| 15 | `p10d_2` L1155 | 1M/2M + 70% | Catala-Modul (params) | "Sockel + 70% ab Sockel" | **Teil-JA** (Ring-Test L1570: 1.5M; ±1 am Sockel fehlt) | FLAG-2 (kein 999999/1000000/1000001) |
| 16 | `p34c_1` L1214 | zvE ≤ 0 | `if ausl <= 0 or zve <= 0: return 0` | "zve ≤ 0 → 0" | **JA** (Seed: zve=60000-ausl=30000-30000→5000) | OK |
| 17 | `p32b_1` L792 | erhoehte ≤ 0 | `if erhoehte <= 0: return 0` | — | **NEIN** | FLAG-2 (kein erhoehte≈0-Test) |
| 18 | `p34_abs3` L1242 | 5 Mio | `min(ao, 5Mio)` im Modul | "≤ 5Mio, Excess fail-closed" | **JA** (Guard test L1313: >5Mio→offen; ±1 fehlt) | FLAG-2 (4.999.999/5.000.000/5.000.001) |
| 19 | `p35a_haushaltsnahe` L322 | 510/4000/1200€ | Catala-Modul | "3×20%-Töpfe" | **Teil-JA** (params-basiert) | OK (params) |
| 20 | `p33_zumutbar` L347 | 1-7% Staffel | Catala-Modul | "Tranchen" | **Teil-JA** (params) | OK (params) |

## Zusammenfassung

**FLAG-1 (Code ≠ Docstring)**: 0 — alle Operatoren konsistent mit Docstring-Behauptung.

**FLAG-3 (Cent/Euro an Schwelle)**: 0 — §22 Nr.3 25600 Cent vs 256€ korrekt (Cent-Vergleich in Cent), §16 45000€ korrekt in Euro, GWG 80000 Cent Guard korrekt.

**FLAG-2 (ungetestete Grenzen)** — Rangliste:

| Rang | Grenze | Accessor | Risiko |
|------|--------|----------|--------|
| 1 | 47/48/49 Monate in Übernachtung | `_uebernachtung_abzug` | 48-Monats-Schwelle ungetestet. ±1 könnte Kappung falsch aktivieren/deaktivieren → max 1000€×12 = 12000€ Over/Under-tax |
| 2 | 999999/1000000/1000001 sockel §10d | `catala_p10d_2` (Catala) | seltener Fall, aber 1M Sockel = Scharnier für 70%-Regel |
| 3 | 4.999.999/5.000.000/5.000.001 §34 Abs.3 | `catala_ermaessigter_durchschnittssatz` (Catala) | 5Mio-Cap; jenseits fail-closed → Schutz durch Guard |
| 4 | 44999/45000/45001 + 135999/136000/136001 §16 Abs.4 | `catala_p16_4_freibetrag` (Catala) | vg=0 ungetestet, Abschmelzung ±1 ungetestet |
| 5 | >800 1ct Grenze GWG | api.py L480 `> 80000` | Cent-Guard in api.py separat (L477-480) vor dem Catala-Aufruf — der Cent-Guard ist der RELEVANTE Schutz hier |
| 6 | geburtsjahr-Gate ±1 §24a | `catala_p24a_altersentlastung` | over-tax-safe (Geburtstag im Dezember vs Januar) |

## Gesamturteil

20 Schwellen identifiziert. 0 FLAG-1, 0 FLAG-3. 6 FLAG-2 (ungentestete Boundaries), alle niedrige Priorität außer Übernachtung 48-Monate (dort empirisch seltener Fall). Der A8-§22-Nr.3-Bug (Cent/Euro) wäre durch diesen Sweep nicht gefangen worden (das war eine fehlende Konversion, keine falsche Schwelle).
