# B-Bake-off — Rohdaten + Verdikt (Lauf abgeschlossen)

Vorregistrierter Messplan: commit e0f2ed6 + Instructor-Freigabe msg 1232 (Gewichts-
Korrektur: equivalence NUR @p24b gewertet). 3 Kandidaten × 3 Regeln × 2 Läufe = 18 Zellen.
A (claude-sonnet-4.6) + Judge (deepseek-v4-pro@deepinfra) + Prompts unverändert. models.yaml
NUR B-Rolle je Kandidat gepinnt, danach `git restore` (sauber zurückgesetzt, verifiziert).
Ist-Kosten Bake-off ≈ **1,00 USD** (report-cost-sum 0,999; OpenRouter-key kumuliert 1,54).

## Rohdaten je Zelle

| cand | rule | run | catala_b | syntax_b | typecheck_b | clerk_b | **waechter** | equiv | $ |
|---|---|---|---|---|---|---|---|---|---|
| gemini | nr6_7 | 1 | ja | PASS | PASS | **PASS** | **True** | FAIL | 0.114 |
| gemini | nr6_7 | 2 | nein | – | – | FAIL | False | – | 0.131 |
| gemini | nr5a | 1 | nein | – | – | FAIL | False | – | 0.000\* |
| gemini | nr5a | 2 | ja | PASS | PASS | FAIL | False | PASS\*\* | 0.051 |
| gemini | p24b | 1 | ja | PASS | PASS | PASS | – | PASS | 0.071 |
| gemini | p24b | 2 | ja | PASS | PASS | PASS | – | PASS | 0.071 |
| mistral | nr6_7 | 1 | ja | PASS | FAIL | FAIL | False | FAIL | 0.055 |
| mistral | nr6_7 | 2 | ja | PASS | PASS | FAIL | False | FAIL | 0.065 |
| mistral | nr5a | 1 | ja | PASS | FAIL | FAIL | False | FAIL | 0.035 |
| mistral | nr5a | 2 | ja | PASS | FAIL | FAIL | False | FAIL | 0.040 |
| mistral | p24b | 1 | ja | PASS | FAIL | **FAIL** | – | **FAIL** | 0.053 |
| mistral | p24b | 2 | ja | PASS | PASS | PASS | – | PASS | 0.066 |
| llama | nr6_7 | 1 | ja | PASS | PASS | FAIL | False | FAIL | 0.047 |
| llama | nr6_7 | 2 | ja | PASS | PASS | FAIL | False | FAIL | 0.047 |
| llama | nr5a | 1 | ja | PASS | PASS | FAIL | False | PASS\*\* | 0.021 |
| llama | nr5a | 2 | ja | PASS | PASS | FAIL | False | PASS\*\* | 0.023 |
| llama | p24b | 1 | ja | PASS | PASS | PASS | – | PASS | 0.051 |
| llama | p24b | 2 | ja | PASS | PASS | PASS | – | PASS | 0.059 |

\* $0,000/55s = role_timeout/Call-Fehler (kein Billing), Provider-Status war 0 → transient.
\*\* equiv=PASS auf defekt-Regel = **B reproduziert A's Bug** (Anti-Signal, per Korrektur ungewertet).

## Aggregat + vorregistrierter Score

| Kandidat | eligibel nr6_7 | eligibel nr5a | Wächter (0,40) | catala-clean (0,30) | eq@p24b (0,10) | Kontrolle (0,15) | $ | **SCORE** |
|---|---|---|---|---|---|---|---|---|
| gemini | **ja** (1/2) | nein (0/2) | 0,25 | 0,67 | 1,00 | 1,00 | 0,437 | **0,550** |
| mistral | nein | nein | 0,00 | 0,33 | 0,50 | 0,50 | 0,314 | 0,225 |
| llama | nein | nein | 0,00 | 1,00 | 1,00 | 1,00 | 0,248 | **0,550** |

Eligibilität = Wächter in ≥1 von 2 Läufen. **Kein Kandidat für BEIDE defekt-Regeln eligibel.**

## Verdikt: ESKALATION (per vorregistrierter Regel)

„Kein Kandidat eligibel für beide defekt-Regeln → Eskalation." Erfüllt. Zusätzlich Gleich-
stand gemini=llama (0,550, Δ<0,05) → auch das eskaliert. **Der Score-Gleichstand ist ein
Artefakt:** gemini hat als EINZIGER Kandidat überhaupt Wächter-Treffer (das 0,40-Gewicht =
der eigentliche Zweck); llama = makelloser Compiler (catala-clean 1,00) mit **null** Problem-
lösung (0 Wächter, reproduziert nur A). Eligibilität, nicht Rohscore, trägt die Entscheidung.

## Der eigentliche Befund (wichtiger als die Besetzung)

**nr5a ist KEIN Besetzungsproblem.** Jedes emittierte nr5a-catala (gemini run2, llama ×2)
liefert für seed47 (monate_bisher=47, SOLL 16.800 ungekappt) den Wert **12.000** — kappt also
UNBEDINGT, genau wie A und wie glm. seed48 (SOLL 12.000 gekappt) stimmt nur zufällig, weil
alles gekappt wird. **0/6 Läufe** produzieren die ungekappte 47er-Verzweigung. Ein besseres B
fixt das nicht — alle Modelle bilden dasselbe falsche mentale Modell („immer kappen"). Fix-Pfad:
**Spec/Zuschnitt** — expliziter 48-Monats-Guard im Zuschnitt, oder ein Seed/Rechenbeispiel das
die ungekappte Verzweigung erzwingt, oder Signatur überdenken. → Julius/Instructor.

**nr6_7 ist besetzbar, aber ohne stabilen Sieger.** Nur gemini löste (run1, alle 6 Seeds inkl.
Wächter 200/200); run2 lieferte kein extrahierbares Catala (gemini-2.5-pro ist verbose →
Trunkierungs-Verdacht bei fixem max_tokens=8192). llama/mistral kompilieren, rechnen aber
falsch (eigene Fehler ≠ A). → Ein besseres/anderes B KANN nr6_7 knacken, aber gemini @ 8192
ist instabil.

## Empfehlung (dev, Julius-Widerrufsvorbehalt)

1. **B NICHT auf dieser Evidenz tauschen.** Kein Kandidat eligibel für beide; Gleichstand
   Artefakt; keiner schlägt glm auf dem eigentlichen Ziel (defekt-Regeln lösen) eindeutig.
2. **Problem splitten:** nr5a → Spec/Zuschnitt-Fix (nicht B). nr6_7 → entweder gemini mit
   höherem max_tokens (Token-Budget-Änderung = Julius) oder ebenfalls Spec-Fix.
3. **Gezielter Mikro-Bake-off denkbar:** gemini vs glm NUR auf nr6_7, max_tokens angehoben,
   um zu prüfen ob gemini stabil wird — falls Julius die Token-Änderung freigibt.

## Frage an Instructor

(a) nr5a: bestätige „Spec-Fix statt Besetzung" — und ist das ein Re-Zuschnitt (expliziter
48-Gate-Guard / erzwingende Seed) oder Julius-Entscheid? (b) nr6_7: gemini-mit-mehr-Tokens
Mikro-Bake-off lohnend, oder auch Spec-Fix? (c) Besetzung: glm halten bis Spec-Fixe stehen —
einverstanden?
