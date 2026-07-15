# VZ-Golden-Ketten F3 (DBA-Freistellung/§32b) + F4 (DBA-Anrechnung/§34c) 2024/2025 (taxgraph-dev-2, 2026-07-15)

Instructor-Auftrag: komplette Erwartungswert-Ketten F3/F4 für VZ 2024+2025 hand-rechnen,
gleiche Methodik wie F1/F2, trianguliert gegen die frozen 2026-Assertions, als unabhängige
Referenz für dev-1s M5-Golden-Bau. Read-only, meine Zone (reports/). Ketten repliziert aus den
handgeschriebenen Integrations-Goldens `rules/estg/integration/familie3_dba_freistellung.catala_en`
+ `familie4_dba_anrechnung.catala_en` (nur READ, dev-1-Zone). SolZ gegen Stufe-2-solzg-params
(dev-1 `1d8282e`): Freigrenze einzel 2024 18130, 2025 19950, 2026 20350; Milderung 11,9 % konstant.

## ⚠ PROMINENTER FUND — SolZ-Drift ist im Modul da, an der Call-Site aber abgeklemmt

Kein Größen-Drift jenseits tarif+SolZ (Erwartung bestätigt, s. u.). ABER ein **Golden-Bau-Blocker**
für die SolZ-Zeile: die drei Integrations-Scopes (familie1/2/4) rufen `Solzg.Solidaritaetszuschlag`
mit **hart verdrahtetem `veranlagungszeitraum: 2026`** auf (familie4 Zeile 181), NICHT mit dem
Ketten-VZ. Bewusste dev-1-Stufe-2-Entscheidung (Enum→INT-Konversion vermieden, „safe weil alle
Testfälle VZ2026"; Catala-1.2.0-cross-module-Enum-Grenze). Das solzg-Modul SELBST ist voll VZ-fähig
(if-then-else auf int, Freigrenze-Drift materialisiert). **Folge: ein naiver VZ-Klon der F1b/F4b-Fälle
liefert die FALSCHE SolZ-Zeile**, weil die Call-Site die Freigrenze bei 2026 (20350) festnagelt statt
2024 (18130) / 2025 (19950) zu nehmen.

| Fall | VZ | fest.ESt | SolZ **korrekt** (Freigrenze VZ) | SolZ **frozen-Scope** (2026-Pin) | Divergenz |
|---|---|---|---|---|---|
| F4b | 2024 | 28.363 | **1.217,72** | 953,54 | 264,18 |
| F4b | 2025 | 28.088 | **968,42** | 920,82 | 47,60 |
| F1b | 2024 | 23.363 | **622,72** | 358,54 | 264,18 |
| F1b | 2025 | 23.088 | **373,42** | 325,82 | 47,60 |

→ **dev-1 muss beim VZ2024/2025-Golden-Bau die Enum→INT-Konversion an der soli-Call-Site nachrüsten**
(familie1 + familie4; familie2 unbetroffen, alle Fälle SolZ 0; familie3 hat keinen SolZ). Sonst
materialisiert die Freigrenze-Drift nicht in der Kette. Meine SolZ-Spalten unten = die INTENDIERTEN
(korrekten) Zielwerte; die frozen-Scope-Werte sind das, was der Code OHNE Fix heute ausgibt.

## TRIANGULATION (Falsch-Grün-Härte)
Meine Ketten-Replik reproduziert JEDE frozen assertion der bestehenden 2026-Goldens exakt:
F3a tarif_ohne 3862 / est_erhöht 13747 / fest 6725,34 ✓ · F3b tarif_ohne=est_erhöht=fest 3862 ✓ ·
F4a dt.ESt 13747 / höchst 4681,10 / anr 4681,10 / fest 9065,90 / SolZ 0 ✓ · F4b dt.ESt 30864 /
anr 3000 / fest 27864 / SolZ 894,16 ✓ · F4c dt.ESt 3862 / anr 3862 / fest 0 / SolZ 0 ✓. tarif-
Closed-Form = BMF-Corpus-Replik; §32b-Bruch exakt (Fraction, round-half-away), §34c-Höchstbetrag
exakt, SolZ Cent-Schnitt.

## F3 — DbaFreistellungKette (§2 → §32a → §32b Progressionsvorbehalt → mod. Tarif) — KEIN SolZ
Kette: zvE = inländische Einkünfte − SA · tarif_ohne = tarif(zvE,VZ) · erhöhte_bmg = zvE + Progr.-
Einkünfte · est_erhöht = tarif(erhöhte_bmg,VZ) · besonderer_steuersatz = est_erhöht / erhöhte_bmg
(exakter money/money-Bruch, keine Rundung) · fest = round-half-away(steuersatz × zvE).

### F3a Progression hebt Satz (inländisch 28.734, SA 0, Progr.-Eink. 30.000)
| VZ | zvE | tarif_ohne | erhöhte_bmg | est_erhöht | fest = progr_est |
|---|---|---|---|---|---|
| 2024 | 28.734 | 4.051 | 58.734 | 14.148 | **6.921,52** |
| 2025 | 28.734 | 3.946 | 58.734 | 13.924 | **6.811,94** |
| 2026 | 28.734 | 3.862 | 58.734 | 13.747 | 6.725,34 (Pin ✓) |
Rechenweg 2024: Satz = 14148/58734 (exakt); fest = round-half-away(1414800 · 2873400 / 5873400 Cent)
= 692152 Cent = 6.921,52. Deutlich über tarif_ohne 4.051 → Progressions-Anhebung.

### F3b Regressions-Wächter (Progr.-Eink. = 0 → kein Progressionseffekt)
| VZ | zvE | tarif_ohne | erhöhte_bmg | est_erhöht | fest = progr_est |
|---|---|---|---|---|---|
| 2024 | 28.734 | 4.051 | 28.734 | 4.051 | **4.051,00** |
| 2025 | 28.734 | 3.946 | 28.734 | 3.946 | **3.946,00** |
| 2026 | 28.734 | 3.862 | 28.734 | 3.862 | 3.862,00 (Pin ✓) |
Progr. 0 → erhöhte_bmg = zvE → Satz × zvE kürzt sich glatt auf tarif(zvE) → fest ≡ Grundtarif ohne
Progression (alle vier Größen identisch). Wächter greift jeden VZ.

## F4 — DbaAnrechnungKette (§2 → §32a → §34c Höchstbetrag-Anrechnung → §3/4 SolzG)
Kette: zvE = Einkünfte − SA · dt.ESt = tarif(zvE,VZ) · durchschnittssatz = ausl. Einkünfte / zvE
(exakt) · höchstbetrag = round-half-away(dt.ESt × durchschnittssatz) · anrechnung = min(gezahlt;
höchstbetrag) [0 falls ausl≤0 ∨ zvE≤0] · fest = dt.ESt − anrechnung · SolZ = solzg(fest, Freigrenze[VZ]).

### F4a Höchstbetrag bindet (Eink. 58.770, ausl. 20.000, gezahlt 6.000, SA 36)
| VZ | zvE | dt.ESt | durchschn.satz | höchstbetrag | anrechnung | fest.ESt | SolZ |
|---|---|---|---|---|---|---|---|
| 2024 | 58.734 | 14.148 | 20000/58734 | 4.817,65 | 4.817,65 | 9.330,35 | 0 |
| 2025 | 58.734 | 13.924 | 20000/58734 | 4.741,38 | 4.741,38 | 9.182,62 | 0 |
| 2026 | 58.734 | 13.747 | 20000/58734 | 4.681,10 | 4.681,10 | 9.065,90 | 0 (Pin ✓) |
gezahlt 6.000 > höchstbetrag alle VZ → höchstbetrag bindet (driftet via tarif). fest < Freigrenze
alle VZ → SolZ 0 (keine SolZ-Drift hier).

### F4b Gezahlte Steuer bindet (Eink. 100.036, ausl. 20.000, gezahlt 3.000, SA 36)
| VZ | zvE | dt.ESt | durchschn.satz | höchstbetrag | anrechnung | fest.ESt | SolZ **korrekt** |
|---|---|---|---|---|---|---|---|
| 2024 | 100.000 | 31.363 | 0,2 | 6.272,60 | 3.000,00 | 28.363,00 | **1.217,72** |
| 2025 | 100.000 | 31.088 | 0,2 | 6.217,60 | 3.000,00 | 28.088,00 | **968,42** |
| 2026 | 100.000 | 30.864 | 0,2 | 6.172,80 | 3.000,00 | 27.864,00 | 894,16 (Pin ✓) |
gezahlt 3.000 < höchstbetrag alle VZ → gezahlt bindet (VZ-stabil). **Einziger F3/F4-Fall mit SolZ-
Drift** — Rechenweg SolZ 2024: fest 28.363 > FG 18.130 → min(5,5 %·28363 = 1.559,96; 11,9 %·(28363−18130)
= 11,9 %·10233 = 1.217,727) → Cent-Schnitt 1.217,72. ⚠ Frozen-Scope (2026-Pin) gäbe hier 953,54 —
s. prominenter Fund.

### F4c Quote 1 — ausl. Einkünfte = ganzes zvE (Eink. 28.770, ausl. 28.734, gezahlt 5.000, SA 36)
| VZ | zvE | dt.ESt | durchschn.satz | höchstbetrag | anrechnung | fest.ESt | SolZ |
|---|---|---|---|---|---|---|---|
| 2024 | 28.734 | 4.051 | 1,0 | 4.051,00 | 4.051,00 | 0,00 | 0 |
| 2025 | 28.734 | 3.946 | 1,0 | 3.946,00 | 3.946,00 | 0,00 | 0 |
| 2026 | 28.734 | 3.862 | 1,0 | 3.862,00 | 3.862,00 | 0,00 | 0 (Pin ✓) |
Quote 1 → höchstbetrag = volle dt.ESt; gezahlt 5.000 > dt.ESt → anrechnung = dt.ESt → fest ≡ 0 jeden
VZ. dt.ESt driftet via tarif, netzt aber immer auf fest 0. SolZ 0 (BMG 0).

## Was driftet, was nicht (M5-Hinweis für dev-1) — Erwartung BESTÄTIGT, kein Scope-Signal
- **Driftet je VZ:** nur tarifl.ESt (§32a-params) und SolZ (Freigrenze-params). KEINE weitere
  driftende Konstante — die §32b-Progressions-Mechanik (Addition, Quotient, Multiplikation) und die
  §34c-Höchstbetrags-Mechanik (Durchschnittssatz, min, Differenz) haben keinen Jahres-Wert. Alle
  Zwischengrößen (zvE, erhöhte_bmg, durchschnittssatz, Quote, Anrechnungs-Bindung) sind VZ-stabil.
  Damit gilt der F1/F2-Befund unverändert auch für F3/F4 → **kein M5-Scope-Erweiterungs-Signal**.
- **Abgeleiteter Drift (kein eigener Param):** F3 `besonderer_steuersatz`, F4 `höchstbetrag`/
  `anrechnung`(A,C) driften NUR weil ihr tarif-Faktor driftet — keine eigene Kohorten-/Jahres-Konstante.
- **Golden-Bau:** dev-1 kann die 2026-Testfälle auf VZ2024/2025 klonen (`veranlagungszeitraum` +
  Erwartungswerte tauschen); Zwischenwert-Assertions (zvE, durchschnittssatz, Anrechnungs-Bindung)
  bleiben identisch. **ABER F1b + F4b SolZ-Zeile braucht zusätzlich den Enum→INT-Call-Site-Fix**
  (s. prominenter Fund) — ohne ihn ist die SolZ-Assertion still falsch (falsch-grün-Risiko).
- Compute-Skript (deterministisch, $0): `scratchpad/e2e_f3f4.py` — repliziert beide Scopes inkl.
  frozen-vs-matched-SolZ-Kontrast, alle 2026-Pins bestehen.
