# Judge-Stabilitaet nach der Dekomposition

Messplan vorregistriert (Protokolldekret 2026-07-10). Die Kriterien standen vor der Messung fest und liegen in `pipeline/judge_stabilitaet_report.py`.

Ein Durchgang, 7 Regeln, je 3 Laeufe. Kosten 6.7906 USD.

## Vorregistrierte Kriterien

- **Inventar-Deckung: 19.7%** (80 von 406 Items in allen 3 Laeufen) -- Ziel >= 80%
- **geltungsbereich-Splitrate: 29.8%** -- Ziel <= 15%
- **Spot-Replikation identisch: False** -- Ziel: True

## Kennzahlen

- Item-Splitrate gesamt: 18.7% (76 Splits auf 406 beurteilte Items)
- Parse-Fehler: 0 von 21 Laeufen (0%)
- Inventar-Streuung: 326 Items nicht in allen drei Inventarlaeufen gesehen
- Merges des Aehnlichkeitsabgleichs: 126

### Splitrate je blockierendem Gate

| Gate | Splits | Items | Rate |
|---|---|---|---|
| `roundtrip` | 23 | 228 | 10.1% |
| `geltungsbereich` | 53 | 178 | 29.8% |

### Je Regel

| Regel | Items je Lauf | Splits | Rate | Gate-Urteile | stabil? |
|---|---|---|---|---|---|
| `p9_4a_verpflegungsmehraufwand` | 29-38 | 28 | 27.5% | FAIL/FAIL, FAIL/PASS | **nein** |
| `p24b_entlastungsbetrag` | 12-18 | 12 | 26.7% | FAIL/FAIL, FAIL/PASS | **nein** |
| `p9_1_3_nr5_doppelte_haushaltsfuehrung` | 13-17 | 8 | 18.2% | FAIL/FAIL, FAIL/PASS | **nein** |
| `p35a_2_3_haushaltsnahe` | 34-45 | 18 | 15.4% | FAIL/FAIL | ja |
| `p33_3_zumutbare_belastung` | 3-12 | 3 | 13.6% | FAIL/FAIL | ja |
| `p10_1_7_berufsausbildung` | 9-14 | 4 | 12.5% | FAIL/FAIL, FAIL/PASS | **nein** |
| `p9_6_erstausbildung_abgrenzung` | 13-16 | 3 | 6.8% | FAIL/FAIL | ja |

### Was die Splitrate NICHT misst

Die vorregistrierte Splitrate misst die Uneinigkeit der drei Stimmen ueber EIN Item. Sie sagt nichts darueber, ob in zwei Laeufen dieselben Items ueberhaupt gefunden wurden. Genau dort sitzt die verbliebene Streuung: das Inventar findet mal mehr, mal weniger Norm-Teile, und ein zusaetzlich gefundener `wirkt_hinein`-Teil kippt das `geltungsbereich`-Gate.

Regeln mit wechselnden Gate-Urteilen trotz stabiler Item-Urteile: `p9_4a_verpflegungsmehraufwand`, `p24b_entlastungsbetrag`, `p9_1_3_nr5_doppelte_haushaltsfuehrung`, `p10_1_7_berufsausbildung`.

Spanne der beurteilten Items je Lauf steht in der Tabelle oben. Diese Groesse war nicht vorregistriert; sie wird berichtet, weil sie das Kriterium unterlaeuft, nicht weil sie es bestaetigt.


## Entscheid nach den vorregistrierten Kriterien

Inventar-Deckung 19.7% (Ziel >= 80%), geltungsbereich-Splitrate 29.8% (Ziel <= 15%), Replikation identisch False (Ziel True) -> **spot_diagnose**

Weder alle Kriterien erfuellt noch der Trigger sauber ausgeloest. Spot-Diagnose, dann Entscheid bei Julius.
