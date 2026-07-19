# § 34 Abs. 3 Ermäßigter Durchschnittssatz — Deklarations-Recon (Stufe-2-Prep) — dev-2, 2026-07-19

Read-only Recon (parallel zum Mitunternehmer-Gate). Deklarations-Felder + Snapshot-Status + BOUNDARY-ANALYSE.
Alle Werte source-verifiziert gegen estg_p34_2026-07-13.txt (Fassung 2026), NICHT Gedächtnis.

## 1. Source § 34 Abs. 3 (verbatim, estg_p34_2026-07-13)
- S.1: außerordentliche Einkünfte iSd Abs.2 Nr.1 (VÄ-Gewinne §§ 14, 14a Abs.1, §§ 16, 18 Abs.3) → **auf Antrag**,
  abweichend von Abs.1, der Teil ≤ **5 Millionen Euro** → ermäßigter Steuersatz, **wenn 55. Lebensjahr vollendet
  ODER im sozialversicherungsrechtlichen Sinne dauernd berufsunfähig**.
- S.2: ermäßigter Satz = **56 %** des durchschnittlichen Steuersatzes (tarifliche ESt nach gesamtem zvE **zzgl.
  Progressionsvorbehalt-Einkünfte**), **mindestens 14 %**.
- S.3: verbleibendes zvE (zvE − Abs.3-Einkünfte) → allgemeine Tarifvorschriften.
- S.4: **einmal im Leben**. S.5: bei mehreren VÄ-/Aufgabegewinnen im VZ → nur für EINEN. S.6: Abs.1 S.4 (§ 6b/6c-Ausnahme).

## 2. Snapshot p34_3_ermaessigter_durchschnittssatz.json
queue=verified_bedingt, faithful=**FALSE**, module=ErmaessigterDurchschnittssatz. catala_a (20 Z.):
```
est_ao =
  cap = 5,000,000; ao_gekappt = min(ao_einkuenfte, cap)
  durchschnittssatz = est_gesamt_zzgl_progression / bemessungsgrundlage_durchschnitt
  ermaessigter_satz = max(0.56 * durchschnittssatz, 0.14)
  est_ao = ao_gekappt * ermaessigter_satz
```
Inputs: ao_einkuenfte, est_gesamt_zzgl_progression (tarifl. ESt auf Basis), bemessungsgrundlage_durchschnitt
(gesamt-zvE zzgl. Progression). Output: est_ao.

### faithful=FALSE = JUDGE-ARTEFAKT (nicht Defekt) → wahrscheinlich promotbar
Judge-Abweichung: „Der durchschnittliche Steuersatz wird als Quotient aus est_gesamt_zzgl_progression und
bemessungsgrundlage_durchschnitt berechnet, nicht als tarifliche Einkommensteuer." → **MISREAD**: der
durchschnittliche *Steuersatz* IST per Definition ein Quotient (Steuer/Basis = Durchschnitts-*Rate*). § 34 Abs.3
S.2 „56 Prozent des durchschnittlichen Steuersatzes, der sich ergäbe, wenn die tarifliche Einkommensteuer nach
dem gesamten zvE … zu bemessen wäre" = (tarifl. ESt) / (Basis). est_gesamt_zzgl_progression = der tarifliche-ESt-
Zähler, korrekt. Muster wie p10d_2 (2-Mio-Falschflag) / p15_1_2 (Judge übersah S.2) → **nicht_echt-Adjudikation
durch Instructor**, kein Re-Formalisieren. ABER Boundary-Analyse trotzdem pflicht (faithful≠Correctness).

## 3. ⚠ BOUNDARY-ANALYSE (p10d_2-Lehre)
| Prüfpunkt | Befund | Risiko |
|---|---|---|
| min 14 % | `max(0.56*satz, 0.14)` ✓ S.2 „mindestens 14 %" | sauber |
| 56 % Faktor | `0.56 * durchschnittssatz` ✓ S.2 | sauber |
| 5-Mio-Cap | `min(ao_einkuenfte, 5Mio)` ✓ S.1 „≤ 5 Mio" | sauber (für den ≤5Mio-Teil) |
| **5-Mio-EXCESS** | Modul rechnet NUR est auf ≤5Mio-Teil. Der Überschuss (ao−5Mio) ist NICHT im Modul | ⚠⚠ **RING-NAHT-PFLICHT**: Ring MUSS (ao−5Mio) separat besteuern (Abs.1-Fünftel/Normaltarif) — sonst >5Mio-Teil UNVERSTEUERT = K2-UNDER-TAX |
| **÷ durchschnittssatz** | `est / bemessungsgrundlage` — DIVISION | ⚠ Div-by-Zero wenn bemessungsgrundlage=0 (niedrige Prob: Basis ⊇ ao_einkuenfte, aber Guard defensiv sinnvoll) |
| Progression-Basis | Inputs müssen zvE **zzgl. Progressionsvorbehalt** tragen + est = tarifliche ESt | ⚠ Ring-Detail: korrekte Speisung (Progression-Inklusion) — sonst Satz-Verfälschung |
| S.3 verbleibendes zvE | allgemeiner Tarif auf (zvE − ao) — NICHT im Modul | Ring-Naht (wie Abs.1: verbleibendes-zvE-Tarif) |

**Schärfstes Risiko = 5-Mio-Excess** (K2-under-tax wenn Ring den Überschuss vergisst). Kein p10d_2-Cap-Bug im
Modul selbst (Cap korrekt), aber die Excess-Behandlung ist Ring-Naht-Pflicht.

## 4. Deklarations-Felder Abs.3
| Feld | Status | Herkunft |
|---|---|---|
| **antrag_ermaessigter_satz** | NEU (flag) | S.1 „auf Antrag" — Opt-in; ohne Antrag → Abs.1-Default (over-tax-safe) |
| **dauernd_berufsunfaehig** | NEU (flag) | S.1 Alternative zu Alter≥55 (sozialversicherungsrechtlich) |
| Alter ≥ 55 | DERIVE | aus geburtsjahr (schon gebunden, bindung_an_gesamt:34): VZ − geburtsjahr ≥ 55 |
| **ermaessigung_einmal_genutzt** | NEU (flag) | S.4 „einmal im Leben" — Selbst-Bestätigung (FA-tracked, App muss fragen) |
| ao_einkuenfte (VÄ-Gewinn) | REUSE | rentner_veraeusserungsgewinn (gebunden, bindung_rentner:362) / § 16-vg |
| est_gesamt_zzgl_progression / bemessungsgrundlage_durchschnitt | DERIVED (Ring) | tarifliche ESt + Basis (zvE zzgl. Progression) |

⚠ S.5 (mehrere VÄ-Gewinne → nur EINER): MVP-Scope = ein VÄ-Gewinn (rentner_veraeusserungsgewinn ist skalar);
Multi-VÄ = benannte Lücke. Antrag-Wahl welcher = out-of-MVP.

## 5. Aufwand / Sequenz
MODERATE-KOMPLEX (deutlich > Mitunternehmer): (a) faithful=FALSE nicht_echt-Adjudikation (Judge-Artefakt), (b) 3
neue Eligibility-Flags + Alter-Derive + vg-Reuse, (c) ⚠ komplexe Ring-Naht (5-Mio-Excess-Besteuerung +
Progression-Speisung + verbleibendes-zvE-Tarif + Abs.1-vs-Abs.3-Chooser auf Antrag). Braucht Abs.1
(Fünftelregelung, dev-1) ZUERST — Abs.3 ist die Antrags-Alternative (S.1 „abweichend von Absatz 1"), S.5
mutually-exclusive je VÄ-Gewinn.

## Fazit
p34_3-Modul ist wertmäßig korrekt (56%/14%/5Mio source-verifiziert), faithful=FALSE = Judge-Misread (Durchschnitts-
*Satz* = Quotient) → nicht_echt-adjudizierbar. Der EINE scharfe Boundary-Punkt = **5-Mio-Excess muss der Ring
besteuern** (sonst K2-under-tax) + Div-by-Zero-Guard. Deklaration: 3 neue Flags + Derive/Reuse. Ring-Naht komplex
(Chooser + Excess + Progression). → Stufe-2 nach § 34 Abs.1. Instructor-Boundary-Review + nicht_echt-Adjudikation
vor Materialisierung.
