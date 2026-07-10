# Judge-Stabilitaet nach der Dekomposition

Messplan vorregistriert (Protokolldekret 2026-07-10). Die Kriterien standen vor der Messung fest und liegen in `pipeline/judge_stabilitaet_report.py`.

Ein Durchgang, 7 Regeln, je 3 Laeufe. Kosten 4.9373 USD.

## Kennzahlen

- **Item-Splitrate gesamt: 19.3%** (59 Splits auf 305 beurteilte Items)
- Parse-Fehler: 0 von 21 Laeufen (0%)
- Inventar-Streuung: 249 Items nicht in allen drei Inventarlaeufen gesehen
- Merges des Aehnlichkeitsabgleichs: 132

### Splitrate je blockierendem Gate

| Gate | Splits | Items | Rate |
|---|---|---|---|
| `roundtrip` | 11 | 155 | 7.1% |
| `geltungsbereich` | 48 | 150 | 32.0% |

### Je Regel

| Regel | Items je Lauf | Splits | Rate | Gate-Urteile | stabil? |
|---|---|---|---|---|---|
| `p9_1_3_nr5_doppelte_haushaltsfuehrung` | 12-14 | 9 | 23.7% | FAIL/FAIL, FAIL/PASS | **nein** |
| `p9_4a_verpflegungsmehraufwand` | 19-24 | 15 | 22.7% | FAIL/FAIL, FAIL/PASS | **nein** |
| `p24b_entlastungsbetrag` | 10-12 | 7 | 21.2% | FAIL/PASS, PASS/FAIL | **nein** |
| `p10_1_7_berufsausbildung` | 5-14 | 6 | 20.7% | FAIL/FAIL | ja |
| `p9_6_erstausbildung_abgrenzung` | 8-14 | 6 | 19.4% | FAIL/FAIL | ja |
| `p35a_2_3_haushaltsnahe` | 24-31 | 13 | 15.1% | FAIL/FAIL, FAIL/PASS | **nein** |
| `p33_3_zumutbare_belastung` | 6-9 | 3 | 13.6% | FAIL/FAIL, FAIL/PASS | **nein** |

### Was die Splitrate NICHT misst

Die vorregistrierte Splitrate misst die Uneinigkeit der drei Stimmen ueber EIN Item. Sie sagt nichts darueber, ob in zwei Laeufen dieselben Items ueberhaupt gefunden wurden. Genau dort sitzt die verbliebene Streuung: das Inventar findet mal mehr, mal weniger Norm-Teile, und ein zusaetzlich gefundener `wirkt_hinein`-Teil kippt das `geltungsbereich`-Gate.

Regeln mit wechselnden Gate-Urteilen trotz stabiler Item-Urteile: `p9_1_3_nr5_doppelte_haushaltsfuehrung`, `p9_4a_verpflegungsmehraufwand`, `p24b_entlastungsbetrag`, `p35a_2_3_haushaltsnahe`, `p33_3_zumutbare_belastung`.

Spanne der beurteilten Items je Lauf steht in der Tabelle oben. Diese Groesse war nicht vorregistriert; sie wird berichtet, weil sie das Kriterium unterlaeuft, nicht weil sie es bestaetigt.


## Entscheid nach dem vorregistrierten Kriterium

Splitrate 19.3% -> **spot_replikation**

Spot-Replikation auf den zwei instabilsten Regeln, dann Entscheid.

Instabilste Regeln fuer die Spot-Replikation: `p9_1_3_nr5_doppelte_haushaltsfuehrung`, `p9_4a_verpflegungsmehraufwand`