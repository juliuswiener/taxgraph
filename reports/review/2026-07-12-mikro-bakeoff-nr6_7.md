# Mikro-Bake-off nr6_7 — Rohdaten + Verdikt (Token-Parität 16384)

Instructor msg 1236b: gemini vs glm, nr6_7 + p24b-Kontrolle, 2 Läufe, max_tokens 8192→16384
für BEIDE Rollen (Token-Parität, Messnotiz gesetzt, kein Prompt-Change). models.yaml je Zelle
gepinnt, danach `git restore` (verifiziert sauber). Kosten ≈ **0,74 USD** (report-sum; glm @16384
lief langsam, 150-280s). Kriterien geerbt (equivalence auf nr6_7 ungewertet, diagnostisch).

## Rohdaten je Zelle (@ max_tokens=16384)

| cand | rule | run | catala_b | clerk_b | waechter | equiv | $ | s |
|---|---|---|---|---|---|---|---|---|
| gemini | nr6_7 | 1 | ja | FAIL | False | FAIL | 0.090 | 111 |
| gemini | nr6_7 | 2 | ja | FAIL | False | FAIL | 0.138 | 150 |
| gemini | p24b | 1 | ja | PASS | – | PASS | 0.062 | 72 |
| gemini | p24b | 2 | ja | PASS | – | PASS | 0.074 | 76 |
| glm | nr6_7 | 1 | ja | FAIL | False | FAIL | 0.100 | 202 |
| glm | nr6_7 | 2 | **nein** | FAIL | False | – | 0.134 | 279 |
| glm | p24b | 1 | ja | PASS | – | PASS | 0.075 | 154 |
| glm | p24b | 2 | ja | PASS | – | PASS | 0.071 | 161 |

**nr6_7: gemini 0/2, glm 0/2.** p24b-Kontrolle: beide 2/2 sauber.

## Der Befund — nr6_7-Defekt ist NARROW (per-Seed identisch über beide Modelle)

Jedes emittierte nr6_7-catala (gemini ×2, glm run1) trifft **Seed 0-4** und verfehlt **NUR Seed 5**:

| Seed | Fall | soll | gemini/glm ist | |
|---|---|---|---|---|
| 0 | GWG 500 (sofort) | 500 | 500 | ✓ |
| 1 | GWG 800 (Grenze) | 800 | 800 | ✓ |
| 2 | AfA 801 Erstjahr voll | 267 | 267 | ✓ |
| 3 | **Wächter Jahr0 anteilig** (Monat 7) | 200 | 200 | ✓ |
| 4 | Jahr1 voll | 400 | 400 | ✓ |
| 5 | **Wächter Letztjahr-Rest** | 200 | **0** | ✗ |

Der Fehler ist EIN Zweig: das **Überhang-/Letztjahr** bei unterjährigem Beginn. Bei Anschaffung
Monat 7, Nutzungsdauer 3: AfA verteilt sich auf **4** Kalenderjahre (Jahr0=200 anteilig, J1=400,
J2=400, J3=Rest 200). Die Modelle rechnen `jahre_seit_anschaffung=3 >= nutzungsdauer=3 → fertig → 0`
und lassen den 200er-Rest fallen. **Kritisch: die anteilige Erstjahr-AfA (Seed 3) stimmt** — die
Modelle KÖNNEN pro-rata; sie tragen den Rest nur nicht ins Überhang-Jahr.

## Verdikt

1. **KEINE Besetzung für gemini.** gemini @16384 nr6_7 = 0/2 (schlechter als das glückliche 1/2
   @8192 im Haupt-Bake-off). Auf nr6_7 ist gemini == glm: beide verfehlen exakt und nur den
   Letztjahr-Rest. Kein Besetzungsvorteil. **glm gehalten** (bestätigt).
2. **Trunkierungs-Hypothese FALSIFIZIERT.** @16384 emittieren beide meist Catala (keine
   Trunkierung mehr), scheitern aber am SELBEN Einzel-Seed. gemini's 8192-Solve war Run-Varianz
   (Glücks-Sample), kein Token-Limit. Der Token-Raise brachte für nr6_7 nichts.
3. **nr6_7 ist ein NARROW Spec-Fix** — viel schmaler als nr5a. 5/6 Seeds sitzen über alle
   Modelle/Läufe; nur der Letztjahr-Rest fällt.

## Empfehlung nr6_7-Fix (dev, Instructor-Entscheid)

Der Defekt ist ein einzelner Zweig (Letztjahr-Rest bei unterjährigem Beginn), nicht die ganze
AfA-Mechanik. Zwei Optionen:

- **Option B (empfohlen, billiger): Zuschnitt-Anreicherung.** Explizite Geltungsbedingung +
  Rechenweg für den Überhang: „bei unterjährigem Beginn verteilt sich lineare AfA auf
  nutzungsdauer+1 Kalenderjahre; das letzte Jahr (jahre_seit_anschaffung == nutzungsdauer) trägt
  den Rest = anschaffungskosten − Summe der Vorjahres-AfA." Ein Wächter-Seed dafür existiert
  bereits (Seed 5). Danach glm-Neulauf. Da die Modelle pro-rata schon beherrschen (Seed 3),
  könnte der gezielte Hinweis reichen.
- **Option A (Fallback, wie nr5a): Teilregel-Split.** `_afa_laufend` (jahre_seit < nutzungsdauer)
  vs `_afa_letztjahr_rest` (jahre_seit == nutzungsdauer, nur bei unterjährigem Beginn). Sauberer,
  aber die Auswahl (jahre_seit vs nutzungsdauer + anschaffungsmonat) ist genau die Bedingung, die
  die Modelle fallenlassen — ein Split verlagert sie in die Geltungsbedingung/§ 2-Ebene.

Empfehlung: **erst B** (ein Neulauf, ~0,1 USD). Hält der Letztjahr-Rest nicht → **A**.

## Frage an Instructor

(a) nr6_7-Fix: Option B (Zuschnitt-Anreicherung + glm-Neulauf) zuerst, A als Fallback — ok?
(b) glm als B-Besetzung endgültig bestätigt (kein Modellwechsel), Julius-Widerruf morgens —
einverstanden? (c) nr6_7-Fix reiht sich wo ein (eigener Mini-Batch nach nr5a)?
