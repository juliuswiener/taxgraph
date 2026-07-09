# Gate G2: Bake-off-Auswertung

Entscheid: niedrigste Eskalationsrate gewinnt; Kosten nur als Tiebreaker.


## Protokollaenderung gegenueber dem ersten G2-Lauf

Der erste Lauf lieferte keinen Entscheid: die Eskalationsrate war gesaettigt
(Minimum 0.929) und 19 von 25 inhaltlichen FAILs waren derselbe Befund - der
Norm-Ausschnitt war breiter als die vorgegebene Scope-Signatur. Gemessen wurde
der Task-Zuschnitt, nicht das Modell. Vier Aenderungen, von Julius freigegeben:

1. **Judge kennt die Scope-Grenze.** Er bewertet `faithful` nur noch innerhalb
   der vorgegebenen Signatur. Norm-Teile ausserhalb gehen als `scope_gap` in eine
   eigene Metrik und fallen kein Gate. `scope_gap` ist Rueckmeldung zur
   Task-Spezifikation, nicht zur Modellqualitaet. (`roundtrip_diff@2`)
2. **Norm-Konstanten raus aus den Signaturen.** `p09` und `p04` reichten Betraege,
   Saetze und Caps als Eingaben herein und verletzten damit das eigene Prinzip aus
   dem Kopf von `tasks.yaml`; bei `p09` erzwang das zusaetzlich eine Staffel, die
   die eingefrorene Fassung 2026 gar nicht mehr kennt (StAendG 2025: einheitlich
   0,38 Euro ab km 1). Die Konstanten stehen jetzt in `ref.fixed_inputs` und
   erreichen nur die Referenz. Signaturen sind VZ-agnostisch.
3. **Genau eine Reparaturrunde**, nur bei Syntax- oder Typecheck-Fehler, Eingabe =
   eigener Quelltext plus woertliche Compiler-Meldung, symmetrisch fuer A und B.
   Der Report weist `syntaxvaliditaet_first_pass` und `syntaxvaliditaet`
   (post-repair) getrennt aus; die Kaskade rechnet mit dem reparierten Quelltext
   weiter. (`formalisierung_repair@1`)
4. **Dritte Paarung `A-glm_B-deepseek`**, Judge Sonnet. Sie beantwortet die Frage,
   die die ersten beiden Paarungen nicht stellen konnten: gehoert Sonnet ueberhaupt
   ins Formalisierer-Paar? Der Judge ist in jeder Paarung die dritte Modellfamilie;
   `check_pairings()` bricht ab, wenn eine Paarung das verletzt.

Vorab-Diagnose (Punkt 5 der Freigabe): Sonnets drei Typecheck-Fails hatten eine
gemeinsame, deterministische Ursache - den Numeric Tower. `p32a` und `p09` fielen
in beiden Paarungen an derselben Stelle (`decimal of (Decimal.truncate of ...)`;
`Decimal.truncate` liefert bereits ein `decimal`). Die Drift stammte nicht aus den
Few-Shots (die enthalten kein `decimal of`), sondern aus dem zu duennen
Syntax-Primer in `formalisierung@2`. `formalisierung@3` schreibt den Numeric Tower
explizit aus; die Regeln sind gegen den Compiler gegengeprueft. Das hebt alle
Paarungen gleichmaessig und verhindert, dass G2 ein Prompt-Artefakt misst.

| Paarung (Formalisierer B) | n | syntaxvaliditaet_first_pass | syntaxvaliditaet | repair_rate | aequivalenz_divergenzrate | roundtrip_abweichungsrate | annahme_benannt | annahme_verpasst | blind_reproduktion | blind_build_error_rate | eskalationsrate | kosten_pro_approved_usd |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| A-glm_B-deepseek | 14 | 0.857 | 1.000 | 0.143 | 0.429 | 0.429 | 0.286 | 0.286 | 0.750 | 0.000 | 0.571 | 0.082 |
| A-sonnet_B-deepseek | 14 | 0.857 | 0.929 | 0.071 | 0.357 | 0.357 | 0.214 | 0.143 | 0.500 | 0.000 | 0.429 | 0.058 |
| A-sonnet_B-glm | 14 | 0.929 | 1.000 | 0.071 | 0.429 | 0.500 | 0.214 | 0.000 | 0.750 | 0.000 | 0.500 | 0.070 |

`annahme_benannt` = der Judge hat eine stille Zusatzannahme selbst benannt (gut). `annahme_verpasst` = der Judge hielt die Formalisierung fuer treu, aber Aequivalenz oder die Blind-Referenz widersprechen (schlecht). Ein Judge, der nie etwas meldet, sieht in Spalte 1 gut aus und faellt in Spalte 2 durch.


`blind_reproduktion` zaehlt nur Laeufe, die ueberhaupt vergleichbar waren. Ein Kandidat, der nicht baut, ist kein 'nicht reproduziert', sondern gar kein Vergleich - er steht in `blind_build_error_rate`.


`syntaxvaliditaet_first_pass` ist der Erstversuch, `syntaxvaliditaet` der Stand nach genau einer Reparaturrunde (Eingabe: eigener Quelltext plus woertliche Compiler-Meldung, symmetrisch fuer A und B). `repair_rate` = Anteil der Laeufe, in denen ueberhaupt repariert wurde. Die Kaskade arbeitet mit dem reparierten Quelltext weiter.


## scope_gap: Rueckmeldung zur Task-Spezifikation

Norm-Teile, die ausserhalb der vorgegebenen Scope-Signatur liegen. Der Judge meldet sie getrennt; sie sind KEIN Modellfehler und fallen kein Gate. Ein hoher Wert heisst: der Norm-Ausschnitt ist breiter geschnitten als die Signatur - ein Befund ueber den Task-Zuschnitt.

| Paarung | scope_gap_anteil | scope_gap_je_task |
|---|---|---|
| A-glm_B-deepseek | 1.000 | 3.929 |
| A-sonnet_B-deepseek | 0.857 | 2.714 |
| A-sonnet_B-glm | 0.714 | 2.071 |

## Eskalation getrennt nach ausloesendem Gate

| Paarung | equivalence | roundtrip | syntax_b | typecheck_b |
|---|---|---|---|---|
| A-glm_B-deepseek | 0.429 | 0.429 | 0.000 | 0.000 |
| A-sonnet_B-deepseek | 0.357 | 0.357 | 0.071 | 0.071 |
| A-sonnet_B-glm | 0.429 | 0.500 | 0.000 | 0.000 |

Clerk-Gate n/a-Anteil (kein EStH/BMF-Rechenbeispiel; Aequivalenz + Round-Trip tragen):

- `A-glm_B-deepseek`: 1.000
- `A-sonnet_B-deepseek`: 1.000
- `A-sonnet_B-glm`: 1.000

## Provider-Flakiness (Transport, nicht Modellqualitaet)

Timeouts, Retries und Rate-Limits sagen etwas ueber den Hoster, nicht ueber das Modell. Sie fliessen deshalb nicht in die Entscheidung ein.

| Paarung | role_timeout_rate | retries | timeouts | rate_limits | errors |
|---|---|---|---|---|---|
| A-glm_B-deepseek | 0.000 | 8 | 0 | 8 | 0 |
| A-sonnet_B-deepseek | 0.000 | 3 | 0 | 3 | 3 |
| A-sonnet_B-glm | 0.000 | 5 | 0 | 5 | 4 |

`A-glm_B-deepseek` je Provider:
- together: retries=8 timeouts=0 rate_limits=8 errors=0

`A-sonnet_B-deepseek` je Provider:
- together: retries=3 timeouts=0 rate_limits=3 errors=3

`A-sonnet_B-glm` je Provider:
- together: retries=5 timeouts=0 rate_limits=5 errors=4

## Empfehlung

**`A-sonnet_B-deepseek`** (Eskalationsrate 0.429; Kosten nur Tiebreaker). Entscheidung trifft Julius.


Vorsprung in absoluten Zahlen: 6/14 eskalierte Laeufe gegenueber 7/14 beim Zweiten (`A-sonnet_B-glm`) - ein Unterschied von 1 Task.

**Schwacher Vorsprung.** 1 Task bei n=14 kann ein einzelner anders geschnittener Norm-Ausschnitt drehen. Die Rangfolge ist ein Hinweis, kein belastbarer Befund; wer sie als Entscheidung liest, ueberdehnt die Datenlage.