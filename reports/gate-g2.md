# Gate G2: Bake-off-Auswertung

Entscheid: niedrigste Eskalationsrate gewinnt; Kosten nur als Tiebreaker.

| Paarung (Formalisierer B) | n | syntaxvaliditaet | aequivalenz_divergenzrate | roundtrip_abweichungsrate | annahme_benannt | annahme_verpasst | blind_reproduktion | blind_build_error_rate | eskalationsrate | kosten_pro_approved_usd |
|---|---|---|---|---|---|---|---|---|---|---|
| A-sonnet_B-deepseek | 14 | 0.571 | 0.714 | 0.929 | 0.786 | 0.000 | 0.000 | 0.750 | 0.929 | n/a |
| A-sonnet_B-glm | 14 | 0.786 | 0.500 | 1.000 | 0.929 | 0.000 | 1.000 | 0.750 | 1.000 | n/a |

`annahme_benannt` = der Judge hat eine stille Zusatzannahme selbst benannt (gut). `annahme_verpasst` = der Judge hielt die Formalisierung fuer treu, aber Aequivalenz oder die Blind-Referenz widersprechen (schlecht). Ein Judge, der nie etwas meldet, sieht in Spalte 1 gut aus und faellt in Spalte 2 durch.


`blind_reproduktion` zaehlt nur Laeufe, die ueberhaupt vergleichbar waren. Ein Kandidat, der nicht baut, ist kein 'nicht reproduziert', sondern gar kein Vergleich - er steht in `blind_build_error_rate`.


## Eskalation getrennt nach ausloesendem Gate

| Paarung | equivalence | roundtrip | syntax_b | typecheck_a | typecheck_b |
|---|---|---|---|---|---|
| A-sonnet_B-deepseek | 0.714 | 0.929 | 0.071 | 0.214 | 0.214 |
| A-sonnet_B-glm | 0.500 | 1.000 | 0.143 | 0.214 | 0.214 |

Clerk-Gate n/a-Anteil (kein EStH/BMF-Rechenbeispiel; Aequivalenz + Round-Trip tragen):

- `A-sonnet_B-deepseek`: 0.929
- `A-sonnet_B-glm`: 1.000

## Provider-Flakiness (Transport, nicht Modellqualitaet)

Timeouts, Retries und Rate-Limits sagen etwas ueber den Hoster, nicht ueber das Modell. Sie fliessen deshalb nicht in die Entscheidung ein.

| Paarung | role_timeout_rate | retries | timeouts | rate_limits | errors |
|---|---|---|---|---|---|
| A-sonnet_B-deepseek | 0.071 | 4 | 0 | 4 | 1 |
| A-sonnet_B-glm | 0.000 | 0 | 0 | 0 | 1 |

`A-sonnet_B-deepseek` je Provider:
- fireworks: retries=3 timeouts=0 rate_limits=3 errors=0
- together: retries=1 timeouts=0 rate_limits=1 errors=1

`A-sonnet_B-deepseek` abgebrochene Rollen: judge (z-ai/glm-5.2) x1

`A-sonnet_B-glm` je Provider:
- together: retries=0 timeouts=0 rate_limits=0 errors=1

## Empfehlung

**Kein Entscheid moeglich.** Die Entscheidungsmetrik traegt nicht:

- Die Eskalationsrate ist gesaettigt (Minimum 0.929). Praktisch jeder Lauf wird eskaliert, also trennt sie die Paarungen nicht.
- Der Abstand betraegt nur 0.071 und liegt bei n=14 Tasks je Paarung im Rauschen.

Ein Sieger waere hier ein Artefakt der Metrik, kein Befund ueber die Modelle. Vor einer Entscheidung muss die Saettigungsursache behoben und der Bake-off wiederholt werden.

Zur Orientierung, ohne Entscheidungscharakter:

- `A-sonnet_B-deepseek`: eskalation 0.929, syntax 0.571, aequivalenz-divergenz 0.714, kosten $0.3315
- `A-sonnet_B-glm`: eskalation 1.000, syntax 0.786, aequivalenz-divergenz 0.500, kosten $0.3893