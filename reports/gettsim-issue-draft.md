# GETTSIM Issue-Entwurf (NICHT abgesendet)

Entwurf fuer ein oder zwei Issues an das GETTSIM-Projekt. Vor dem Absenden von
Julius zu pruefen. Beide Punkte stammen aus dem TaxGraph-Differentialtest
(`oracle/gettsim/harness.py`) gegen GETTSIM 1.2.1 und sind vollstaendig in
`reports/s02-divergenzen.md` belegt.

Sprache im Entwurf: Englisch (GETTSIM-Repo-Konvention). Rechtsbezuege deutsch.

---

## Issue 1: § 32a Abs. 5 splitting rounds after doubling instead of before

**Summary.** For jointly assessed couples (Splitting-Verfahren), GETTSIM computes
the income tax as `floor_to_euro(2 * tariff(zvE / 2))`. The statute (§ 32a Abs. 5
in conjunction with Abs. 1 sentence 6 EStG) defines it as *twice the tax amount
determined for half of the joint taxable income under Abs. 1*, and the Abs. 1 tax
amount is itself already rounded down to full euro (sentence 6). The faithful
reading is therefore `2 * floor_to_euro(tariff(floor_to_euro(zvE / 2)))`, which is
always an **even** euro amount. GETTSIM's variant can produce odd amounts and
deviates by exactly 1 euro in a large share of cases.

**Legal basis.** § 32a Abs. 5 EStG: "das Zweifache des Steuerbetrags, der sich
fuer die Haelfte ihres gemeinsam zu versteuernden Einkommens nach Absatz 1
ergibt". § 32a Abs. 1 Satz 6 EStG: "Der sich ergebende Steuerbetrag ist auf den
naechsten vollen Euro-Betrag abzurunden." The amtliche Splittingtabelle lists
even amounts throughout, consistent with rounding before doubling.

**Reproduction (VZ 2024).** Joint taxable income 23 634 EUR.
- Statute / TaxGraph-Catala: `2 * floor(tariff(floor(23634 / 2)))` =
  `2 * floor(tariff(11817))` = `2 * 4` = **8 EUR**.
- GETTSIM `einkommensteuer.betrag_ohne_kinderfreibetrag_y_sn`: **9 EUR**.

Across a grid of ~1000 joint incomes per VZ, roughly 57-60 % of splitting cases
differ by exactly 1 EUR, and GETTSIM's result is odd in every diverging case.

**Confirmed against the official BMF calculator** (bmf-steuerrechner.de,
2026-07-09), which implements the statutory reading:
- VZ 2024, joint zvE 23 634: BMF = 8 EUR (matches statute/Catala), GETTSIM = 9.
- VZ 2025, joint zvE 24 342: BMF = 20 EUR (matches statute/Catala), GETTSIM = 21.

**Affected function.** `betrag_..._y_sn = anzahl_personen_sn * piecewise_polynomial(x / anzahl_personen_sn)`
with the `RoundingSpec(base=1, direction="down", "§ 32a Abs. 1 S. 6 EStG")` applied
to the already-doubled result. A faithful implementation would round the
per-half tariff amount before multiplying by `anzahl_personen_sn` (and floor the
half-income to full euro before the tariff).

**Note.** This may be an intentional modelling simplification in GETTSIM. Raising
it mainly to confirm the interpretation and to document the 1-euro band.

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

**Note.** Low practical impact (isolated points, 1 euro). Documented for
completeness; GETTSIM may prefer the continuous reconstruction by design.

---

## Interne Notiz (nicht Teil des Issues)

Beide Punkte sind fuer TaxGraph geklaert: Catala folgt dem Gesetzeswortlaut,
die Divergenzen sind als GETTSIM-Vereinfachung bzw. GETTSIM-Approximation
eingeordnet (siehe `reports/s02-divergenzen.md`, `reports/gate-g0.md`). Ein
Absenden ist optional und dient der Community, nicht dem MVP.
