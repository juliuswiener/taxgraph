# B1 — Vorregistrierter Messplan: Bedingungen/Hinweise → Formalisierer (VOR dem Lauf)

Instructor-Auftrag B1 (nach Julius-Freigabe). **Prompt-Änderung → braucht Julius-OK + diesen
vorregistrierten Plan.** Dieser Plan ist vor dem Lauf fixiert; nachträgliche Kriterien-Änderung
= ungültig.

## Frage

Der Formalisierer-Prompt sieht heute NUR den quellen-**auszug** (+ worker-claims + bare-signature);
Geltungsbedingungen erreichen ihn NIE (Code-Fakt, [[formalisierer-kontext-kanal]]). Die Nacht löste
Kontext-Hunger durch auszug-Weitung + Prominenz — arbeitsintensiv je Regel. **Reduziert es den
Hunger STRUKTURELL, wenn deklarierte Bedingungen/Hinweise den Formalisierer erreichen?** Falls ja,
sparen wir künftig Weitungs-Handarbeit; falls nein, bleibt die auszug-Leitlinie der Weg.

## Arme (roles.py-Varianten, hinter Flag; identische Prompts sonst)

- **BASELINE (Kontrolle)**: aktueller Pipeline-Stand mit GEWEITETEN auszügen (die Nacht-Fixes) —
  bekannt grün. Dient als Obergrenze + Regressions-Referenz.
- **ARM A — Bedingungs-beschreibungen**: die `geltungsbedingungen[].beschreibung` gehen als eigener
  Prompt-Block an den Formalisierer. Getestet mit den URSPRÜNGLICHEN NARROW auszügen (Weitung
  zurückgenommen) → schließt die beschreibung die Hunger-Lücke, die der enge auszug lässt?
- **ARM B — dediziertes `hinweis`-Feld**: neues optionales `hinweis`-Feld je Regel (kuratierter
  Spec-Hinweis, KEIN Gesetzestext-Ersatz) im Prompt. Ebenfalls mit NARROW auszügen. Enthält für
  den Präzisions-Fall zusätzlich ein **Numeric-Idiom** ("Prozentrechnung in decimal, Cent-Schnitt
  am Ende") → Klasse-5-Test (solzg).

Prompt-Template-ID + models.yaml-Hash je Arm gestempelt. A/Judge unverändert, B=glm.

## Testset (Regressionskorpus = Nacht-Hunger-Fälle, mit NARROW auszug)

| Fall | Hunger-Klasse | Wächter-Seed | Bedingung/Hinweis, der es tragen soll |
|---|---|---|---|
| nr5a (Monolith, narrow) | 1 Kontext-Hunger | 47→16800 / 48→12000 | zeitraum_ohne_schwellenuebertritt-beschreibung (48-Trigger) |
| nr6_7_afa_laufend (narrow §7) | 1 | Jahr0→200 | § 7-Verteilungs-Hinweis |
| solzg (narrow, 2 Phrasen) | 1 + 5 | 25000→553,35 (+ 20351→0,11) | Milderungs-beschreibung + Numeric-Idiom (Arm B) |
| p10_1_3_3a (narrow, kein Durchbruch) | 1 (Prominenz) | 4000→4000 | Basis-Durchbruch-Hinweis |
| p33_1_2 (narrow) | 1 | 5000/1408,7→3591,3 | zumutbare-Belastung-Subtraktions-Hinweis |
| KONTROLLE p10_1_4 (narrow) | — | 1200/200→1000 | (darf NICHT regredieren) |

Alle Fälle bekommen ihren NARROW auszug zurück (nur fürs Experiment, in einer Experiment-Kopie der
rules.yaml — die grüne Produktions-Version bleibt unberührt). Wächter-Seeds + clerk wie etabliert.

## Messung je (Arm × Fall), B-spezifisch wie im Bake-off

- **clerk (A)** auf den Wächter-Seeds: löst der Arm den Hunger-Fall?
- **catala_B** per-Seed (glm) — trägt der Kanal auch B?
- **first-pass** (kompiliert ohne Repair), **Kosten/Lauf**.
- **2 Läufe je Zelle** (Run-Varianz ist real).

## Vorregistrierte Kriterien (gewichtet, fix)

| Kriterium | Gewicht | Messung |
|---|---|---|
| Hunger-Fix-Rate (clerk-Wächter grün, narrow auszug) | **0,50** | Anteil der 5 Hunger-Fälle, die der Arm mit NARROW auszug löst (über 2 Läufe) |
| Kontrolle p10_1_4 nicht regrediert | 0,20 | bleibt grün |
| first-pass ohne Repair | 0,15 | kompiliert direkt |
| B trägt den Kanal (catala_B grün) | 0,10 | glm ebenfalls gefixt |
| Kosten | 0,05 | invers, Tiebreaker |

## Entscheidungsregel (fix)

- **Ein Arm ≥ 4/5 Hunger-Fälle mit narrow auszug gelöst UND Kontrolle grün** → strukturell wirksam,
  Empfehlung an Julius: adoptieren (Prompt-Change offiziell). Bei zwei wirksamen Armen: höherer
  Gesamtscore; Gleichstand <0,05 → Eskalation.
- **Kein Arm ≥ 3/5** → Bedingungen/Hinweise ersetzen die auszug-Leitlinie NICHT; auszug-Weitung
  bleibt der Weg (dokumentieren). Kein Prompt-Change.
- **Numeric-Idiom (Arm B) löst solzg-Präzision (20351→0,11)** → separat vermerken (Klasse-5-Signal
  via Hinweis funktioniert, unabhängig vom Haupt-Ergebnis).

## Budget

2 Arme × 6 Fälle × 2 Läufe = 24 Läufe × ~0,08 ≈ **~2 USD** (Baseline-Zellen sind $0-Regate aus den
grünen Reports). Rahmen ≤3. Nacht-Rest ~6,4.

## Ablauf nach Freigabe

1. roles.py: `_bedingungen_block`-Variante (Arm A) + `hinweis`-Block (Arm B) hinter Flag, Prompt-
   Template-Bump + Hash-Stempel. 2. Experiment-rules.yaml mit narrow auszügen + hinweis-Feldern.
   3. Messung fahren, Rohdaten (per-Zelle, per-Seed) an Instructor. 4. Besetzungs-analoger
   Entscheid → Julius-Freigabe für den Prompt-Change, falls wirksam.

## Fragen an Instructor

(a) Arme so (A beschreibungen / B hinweis+numeric) oder dritten Arm (A+B kombiniert)? (b) Testset
vollständig, oder Fall streichen/ergänzen? (c) Gewichte ok (Hunger-Fix 0,50 dominant)? (d) Numeric-
Idiom nur in Arm B oder auch als eigener Mini-Arm auf solzg isoliert?
