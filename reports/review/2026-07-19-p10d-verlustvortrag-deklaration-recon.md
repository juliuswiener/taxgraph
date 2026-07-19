# §10d-Verlustvortrag Deklarations-Recon (strategische #1-Front-Prep) — Read-only dev-2, 2026-07-19

Parallel zu dev-1s §10d-Ring-Recon. ⚠ ALLE Werte source-verankert gegen estg_p10d_2026-07-13.txt
(„Fassung: geltende Fassung 2026"), KEIN Gedächtnis — die §35-4×-Lehre gelebt.

## ⭐ SOURCE-FINDING: 70 % (NICHT 60 %)

§10d Abs.2 S.1 (VZ 2026): Verlustvortrag bis **1 Million Euro unbeschränkt, darüber hinaus bis zu 70 Prozent**
des übersteigenden Gesamtbetrags der Einkünfte. Abs.2 S.2: bei Zusammenveranlagten „tritt an die Stelle des
Betrags von 1 Million … 2 Millionen Euro" → **Sockel 1 Mio einzel / 2 Mio zusammen**.
⚠ Die **70 %** ist die Wachstumschancengesetz-Anhebung (60→70 %, VZ 2024-2027 temporär; §52-Anwendung → danach
evtl. wieder 60 %). Wer 60 % aus Gedächtnis nimmt = Fehler (exakt wie §35 4× statt 3,8×). Für VZ 2026 = 70 %.

## Snapshot p10d_2 = JUDGE-ARTEFAKT (nicht Defekt) → PROMOTBAR

module VerlustvortragAbzug, verified_bedingt, faithful=**FALSE** — ABER die EINZIGE abweichung ist FALSCH:
„Der Sockelbetrag wird bei Zusammenveranlagung verdoppelt, was die Norm nicht vorsieht." → §10d Abs.2 S.2
(Quelle 2026) SIEHT die 2-Mio-Verdopplung VOR; der Judge übersah S.2. Die catala_a ist source-korrekt:
```
sockel = if zusammenveranlagung then $2,000,000.00 else $1,000,000.00
hoechstbetrag = sockel + ueberstieg * 0.70   ← 70 % korrekt (VZ 2026)
verlustabzug = min(verlustvortrag_bestand, hoechstbetrag)
```
→ **PROMOTBAR nach nicht_echt-Adjudikation der falschen abweichung** (Muster [[inerte-bindung-verified-bedingt-snapshot]] /
p6_2a-auflösung-Judge-Artefakt). catala_a byte-ready, 70 %/1-Mio/2-Mio source-verankert.

## Naht: GdE-Abzug (vorrangig vor Sonderausgaben)

§10d Abs.2: „vom Gesamtbetrag der Einkünfte … vorrangig vor Sonderausgaben … abzuziehen". → § 2-Stufe:
GdE − §10d-Verlustabzug − Sonderausgaben − agB = Einkommen. Der §10d-Fold sitzt auf dem GdE (post-GdE,
PRE-Sonderausgaben) — der Ring-GdE-Twin (schon berechnet) speist gesamtbetrag_einkuenfte. dev-1-Ring-Zone.

## Deklarations-Felder (KLEIN: 1 neu + 2 reuse/derived)

| Feld | Herkunft | Kz | Pflicht |
|---|---|---|---|
| **verlustvortrag_bestand** | NEU — der zum 31.12.VZ-1 festgestellte verbleibende Verlustvortrag (§10d-Feststellungsbescheid); User-Input/Bescheid, cent | null-MVP: E0190701 [Vortrag] ist nur der „Vortrag festgestellt"-FLAG, der BETRAG ist FA-festgestellt (wie gewst_hebesatz FA-bekannt) → null-Kz-MVP, Vorschau-Feld. optional (absent → kein Abzug, over-tax-safe) | optional |
| zusammenveranlagung | DERIVE aus veranlagung-Enum (schon gebunden, einzel/zusammen) | — | — |
| gesamtbetrag_einkuenfte | DERIVED (Ring-GdE-Twin, schon berechnet) | — | — |

## Fazit

§10d ist NAH promotbar: catala source-korrekt (70 %/1-2-Mio) + faithful=FALSE = reines Judge-Artefakt
(nicht_echt-Adjudikation, kein Re-Formalisieren) + kleine Deklaration (1 neues Feld). Aufwand LOW-MODERATE.
⚠ Multi-VZ-Falle: die 70 % ist VZ-2024-2027-temporär (§52) — für Multi-VZ müsste sie param-VZ-abhängig
(VZ 2028+ → 60 %?); MVP VZ 2026 = 70 % (in der catala hart, bei Promotion so lassen + §52-Temporal dokumentieren).
