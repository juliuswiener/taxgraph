# Engine-Kompositions-Goldens — dev-2, 2026-07-18

**Status: gebaut + grün, freeze-ready.** Beweist die Engine-Seite von Weg (ii): catala_gesamt komponiert
mehrere Einkunftsarten MIT mehreren Abzügen korrekt. Jeder Wert UNABHÄNGIG hand-verifiziert (Grundtarif/
Splittingtarif + § 2-Komposition + § 35a-Floor), nicht nur Engine-Output gelockt.

## 4 neue golden/cases (golden 121 -> 125, EXIT 0)
| Case | Mischung | zvE / Tarif | festzusetzende_est |
|---|---|---|---|
| gesamt_2025_einzel_mehrarten_35a_10b | § 19 40000 + § 21 18770 + § 10b 5000 + § 35a 1200(Floor) | 53770 / Grundtarif 12053 | **10853** |
| gesamt_2025_einzel_kapital_agb_kist | § 19 50000 + § 20 8000(Günstiger) + § 10-KiSt 1200 + § 33 agB 1604 | 55196 / Grundtarif 12582 | **12582** |
| gesamt_2025_zusammen_abzuege | zusammen § 19 80000 + § 10b 16000(20%-Cap gemeins. GdE) + § 33 agB 1864(Splitting-zumutbar) | 62136 / Splittingtarif 9216 | **9216** |
| gesamt_2025_einzel_35a_floor_komposition | § 19 15000 + § 35a-SUMME 3er Töpfe 5710 > Steuer 478 | 14964 / Grundtarif 478 | **0** (gefloort) |

## Unabhängige Verifikation (Methode)
Für jeden Fall: Abzugs-Werte via die bestehenden Stage-1-Accessoren berechnet (catala_p35a_haushaltsnahe,
catala_p10b_spenden, catala_p33_agb→ZumutbareBelastung, catala_p10_kist), dann hand-Referenz gerechnet:
GdE = Σ Einkunftsarten; Einkommen = GdE − max(SA+Vorsorge; § 10c-Pauschbetrag) − agB; zvE = Einkommen − FB;
Tarif = E.grundtarif / E.splittingtarif(zvE); festzusetzende = Tarif − min(§ 35a; max(0; Tarif − ausl. St.)).
**Alle 4: hand-Referenz == catala_gesamt-Output** (Script scratchpad/komposition_verify.py, MATCH ×4). Der
§ 35a-Floor greift bei summierten Ermäßigungen (k4). Abzüge komponieren korrekt auf die gemeinsame GdE (k3,
20%-Cap + zumutbar auf A+B, nicht je Person).

## Abgedeckte Kompositions-Lücke
Bestehende Goldens testeten Abzüge EINZELN (abzuege_ermaessigung, floor). Diese 4 testen den KOMBINIERTEN
Fall (2-3 Einkunftsarten × 2 Abzüge) — der Beweis, den Weg (ii) engine-seitig braucht. Stage-1-Accessoren
(§35a/§10b/§33/KiSt). §24a/§24b/§31 = Nachtrag-Goldens sobald dev-1s Stage-2-Accessoren committet.

## Belege
golden 125/125 EXIT 0 (inkl. Anker-Gate je Case), volle Suite 532 passed. Anker: § 2 "ist die festzusetzende
Einkommensteuer" (estg_p2) + § 35a "ermäßigt sich die tarifliche Einkommensteuer…" (estg_p35a, Floor-Case).
