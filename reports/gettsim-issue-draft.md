# GETTSIM Issues (ABGESENDET 2026-07-09)

Status: beide Issues sind von Julius im Repo `ttsim-dev/gettsim` eroeffnet.

| Hier | GitHub | Titel |
|------|--------|-------|
| Issue 1 | [#1209](https://github.com/ttsim-dev/gettsim/issues/1209) | BUG: § 32a Abs. 5 splitting rounds after doubling instead of before |
| Issue 2 | [#1210](https://github.com/ttsim-dev/gettsim/issues/1210) | BUG: § 32a Abs. 1 Progressionsfaktor reconstruction deviates by 1 euro from the published coefficients |

Dieses Dokument bleibt die Arbeitsfassung (Belege, Verteilung, Reproduktion).
Die Regression-Umrahmung (PR #803) und der Code-Beleg unten sind ggf. im Issue
nachzureichen, falls dort noch die urspruengliche Fassung steht.

Beide Punkte stammen aus dem TaxGraph-Differentialtest
(`oracle/gettsim/harness.py`) gegen GETTSIM 1.2.1 und sind vollstaendig in
`reports/s02-divergenzen.md` belegt.

Sprache im Entwurf: Englisch (GETTSIM-Repo-Konvention). Rechtsbezuege deutsch.

---

## Issue 1: Regression: splitting rounding fix from #803 lost in TTSIM refactoring (rounds after doubling again)

**Summary.** For jointly assessed couples (Splitting-Verfahren) GETTSIM 1.2.1 again
computes the income tax as `floor_to_euro(2 * tariff(zvE / 2))` instead of doubling
the already-rounded per-half amount. This reintroduces the pre-#803 behaviour: PR
#803 (Dec 2024, changelog entry "Apply correct rounding rules for Ehegatten-
splitting") corrected exactly this rounding order; in 1.2.1 the fix appears to have
been lost in the TTSIM refactoring. The statute (§ 32a Abs. 5 i.V.m. Abs. 1 Satz 6
EStG) requires *twice the tax amount determined for half of the joint taxable
income under Abs. 1*, where the Abs. 1 amount is itself already floored to full euro
(sentence 6): `2 * floor_to_euro(tariff(floor_to_euro(zvE / 2)))`, always an **even**
euro amount. The regression produces 1-2 euro deviations (see distribution below).

**Regression evidence (1.2.1).** The `RoundingSpec(base=1, direction="down",
"§ 32a Abs. 1 S. 6 EStG")` sits on `betrag_ohne_kinderfreibetrag_y_sn`, whose body
returns `anzahl_personen_sn * piecewise_polynomial(gesamteinkommen_y / anzahl_personen_sn)`
(`gettsim/germany/einkommensteuer/einkommensteuer.py`, ~L148-169). Because the
RoundingSpec is applied to the already-doubled result, the per-half tariff amount is
no longer rounded before doubling - the exact behaviour #803 corrected.

**Legal basis.** § 32a Abs. 5 EStG: "das Zweifache des Steuerbetrags, der sich
fuer die Haelfte ihres gemeinsam zu versteuernden Einkommens nach Absatz 1
ergibt". § 32a Abs. 1 Satz 6 EStG: "Der sich ergebende Steuerbetrag ist auf den
naechsten vollen Euro-Betrag abzurunden." The amtliche Splittingtabelle lists
even amounts throughout, consistent with rounding before doubling.

**Reproduction (VZ 2024).** Joint taxable income 23 634 EUR.
- Statute / TaxGraph-Catala: `2 * floor(tariff(floor(23634 / 2)))` =
  `2 * floor(tariff(11817))` = `2 * 4` = **8 EUR**.
- GETTSIM `einkommensteuer.betrag_ohne_kinderfreibetrag_y_sn`: **9 EUR**.

**Two effects.**
1. *Rounding order (the regression, #803).* GETTSIM rounds after doubling
   (`floor(2 * tariff(Z/2))`) instead of doubling the already-rounded per-half
   amount. When the per-half tariff has a fractional part >= 0.5, GETTSIM is 1 euro
   higher; its result is then odd. This is the dominant effect (Diff -1) and is
   exactly what #803 fixed.
2. *Missing floor of Z/2 (separate, never addressed by #803).* GETTSIM halves the
   joint income without rounding it down to full euro (§ 32a Abs. 1 Satz 1), so for
   odd Z it evaluates the tariff at x.5 instead of x. Combined with effect 1 this
   yields a 2-euro deviation (Diff -2), with an even GETTSIM result; it occurs
   (essentially) only for odd joint income. #803 addressed only the rounding order,
   not this floor.

A third, unrelated effect: at some Z/2 the full-precision coefficient
reconstruction (Issue 2) makes the statutory result 1 euro higher (Diff +1).

**Distribution of (statute/Catala - GETTSIM)** over ~1000 joint incomes per VZ
(identical grid, independently reproduced):

| VZ   | Diff -2 | Diff -1 | Diff +1 |
|------|---------|---------|---------|
| 2024 | 102     | 480     | 1       |
| 2025 | 89      | 475     | 5       |
| 2026 | 102     | 496     | 0       |

**Reproduction of a 2-euro case (VZ 2024).** Joint taxable income 43 139 (odd):
- Statute / Catala: `2 * floor(tariff(floor(43139 / 2)))` = `2 * floor(tariff(21569))`
  = `2 * 2122` = **4 244 EUR**.
- GETTSIM: **4 246 EUR** (halves to 21 569.5, no floor; rounds after doubling).

**Confirmed against the official BMF calculator** (bmf-steuerrechner.de,
2026-07-09), which implements the statutory reading:
- VZ 2024, joint zvE 23 634: BMF = 8 EUR (matches statute/Catala), GETTSIM = 9.
- VZ 2025, joint zvE 24 342: BMF = 20 EUR (matches statute/Catala), GETTSIM = 21.

**Fix.**
1. Restore #803: apply the `RoundingSpec` (§ 32a Abs. 1 S. 6) to the per-person
   tariff amount before multiplying by `anzahl_personen_sn`, not to the doubled
   result.
2. Additionally floor `gesamteinkommen_y / anzahl_personen_sn` to full euro before
   the tariff (§ 32a Abs. 1 S. 1) - the separate point above, independent of #803.

Both together reproduce the statutory result (even euro amounts), confirmed by the
official BMF calculator (above). Happy to open a PR restoring the #803 rounding and
adding the missing floor if that helps.

---

## Issue 2: § 32a Abs. 1 Progressionsfaktor reconstruction deviates by 1 euro from the published coefficients

**Summary.** GETTSIM reconstructs the quadratic tariff coefficients from the zone
boundaries and marginal rates via the Progressionsfaktor at full precision. The
statute publishes the closed-form coefficients rounded to two decimals
(e.g. VZ 2026: `(173,10 * z + 2 397) * z + 1 034,87`). At isolated interior points
the full-precision reconstruction and the published-coefficient tariff differ by
1 euro after the final rounding-down.

**Reproduction (VZ 2026).** Taxable income 58 832 EUR, single tariff.
- Published § 32a Abs. 1 coefficients (gesetze-im-internet.de): **13 784 EUR**.
- GETTSIM: **13 785 EUR**.

Over ~1000 grid points per VZ this affects 1-3 points per year (VZ 2024: 1,
VZ 2025: 3, VZ 2026: 1). All deviations are exactly 1 euro and occur at zone
interior points, not at zone boundaries.

**Legal basis.** The tax administration applies the published, two-decimal
coefficients of § 32a Abs. 1. Where GETTSIM's reconstruction rounds to a
different full euro, it deviates from the statutory tariff at that point.

**Related:** #300 (interface issue on the quadratic tariff parametrisation). Not
the same problem - #300 is about how the schedule is parametrised, this is about a
1-euro rounding deviation of the reconstruction against the published coefficients.

**Note.** Low practical impact (isolated points, 1 euro). Documented for
completeness; GETTSIM may prefer the continuous reconstruction by design.

---

## Interne Notiz (nicht Teil des Issues)

Beide Punkte sind fuer TaxGraph geklaert: Catala folgt dem Gesetzeswortlaut,
die Divergenzen sind als GETTSIM-Vereinfachung bzw. GETTSIM-Approximation
eingeordnet (siehe `reports/s02-divergenzen.md`, `reports/gate-g0.md`).
Abgesendet als #1209 (Splitting-Rundung) und #1210 (Progressionsfaktor);
Antworten des Projekts hier nachtragen.
