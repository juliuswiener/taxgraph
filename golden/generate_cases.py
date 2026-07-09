"""Generate the § 32a golden-corpus case files (golden/cases/*.yaml).

Expected tarifliche ESt is computed from the published closed-form § 32a tariff
(literally confirmed coefficients, see params/), independently of the Catala
implementation. The generated cases are the reviewed corpus artifact; the runner
(golden/runner.py) checks the Catala formalisation against them.

Run: python golden/generate_cases.py
"""

from __future__ import annotations

import math
import os

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
CASES = os.path.join(ROOT, "golden", "cases")

# Published, literally confirmed § 32a Abs. 1 coefficients per VZ (see params/).
COEF = {
    2024: dict(gfb=11784, e2=17005, e3=66760, top=277825,
               a2=954.80, a3=181.19, c3=991.21, d4=10636.31, d5=18971.06,
               quelle="§ 32a Abs. 1 EStG, VZ 2024, BGBl 2024 I Nr. 386 (recht.bund.de)"),
    2025: dict(gfb=12096, e2=17443, e3=68480, top=277825,
               a2=932.30, a3=176.64, c3=1015.13, d4=10911.92, d5=19246.67,
               quelle="§ 32a Abs. 1 EStG, VZ 2025, EStH/LStH 2025 (esth.bundesfinanzministerium.de)"),
    2026: dict(gfb=12348, e2=17799, e3=69878, top=277825,
               a2=914.51, a3=173.10, c3=1034.87, d4=11135.63, d5=19470.38,
               quelle="§ 32a Abs. 1 EStG, VZ 2026, gesetze-im-internet.de (Fassung ab VZ 2026)"),
}

SOURCE_FILE = "sources/gesetze-im-internet/estg_p32a_2026-07-09.txt"
# Structural, VZ-independent verbatim anchors present in the frozen § 32a text.
ANKER_ABRUNDUNG = "auf den naechsten vollen Euro-Betrag abzurunden"
ANKER_SPLITTING = "das Zweifache des Steuerbetrags"


def tarif(x: int, p: dict) -> int:
    """Published § 32a Abs. 1 tariff, x = floored taxable income (euro)."""
    x = math.floor(x)
    if x <= p["gfb"]:
        t = 0.0
    elif x <= p["e2"] - 1:
        y = (x - p["gfb"]) / 10000.0
        t = (p["a2"] * y + 1400.0) * y
    elif x <= p["e3"] - 1:
        z = (x - p["e2"]) / 10000.0
        t = (p["a3"] * z + 2397.0) * z + p["c3"]
    elif x <= p["top"] - 1:
        t = 0.42 * x - p["d4"]
    else:
        t = 0.45 * x - p["d5"]
    return math.floor(t)


def splitting(z_total: int, p: dict) -> int:
    """§ 32a Abs. 5: 2 * floor-tariff(floor(Z / 2))."""
    return 2 * tarif(math.floor(z_total / 2), p)


def esc(s: str) -> str:
    return s.replace('"', '\\"')


def emit(fid, beschreibung, year, veranlagung, zve, expected, anker, fundstelle):
    yaml = f"""id: {fid}
beschreibung: "{esc(beschreibung)}"

sachverhalt:
  veranlagungszeitraum: {year}
  veranlagung: {veranlagung}
  zu_versteuerndes_einkommen: {zve}

erwartung:
  tarifliche_est: {expected}

quelle:
  authority: gesetz
  redistributable: true
  fundstelle: "{esc(fundstelle)}"
  datei: "{SOURCE_FILE}"
  zitatanker: "{esc(anker)}"
"""
    with open(os.path.join(CASES, fid + ".yaml"), "w") as f:
        f.write(yaml)


# Grid of taxable incomes per VZ for single tariff (boundaries + spread).
SINGLE = {
    2026: [0, 12348, 12349, 17799, 17800, 20000, 30000, 69878, 69879,
           100000, 277825, 277826, 300000, 500000],
    2025: [0, 12096, 17443, 30000, 60000, 100000, 300000],
    2024: [0, 11784, 17005, 30000, 60000, 100000, 300000],
}
# Joint taxable incomes for splitting. Odd values (43139, 55555, 77777) exercise
# the floor(Z/2) path (statute floors the half income; a case where the literal
# reading diverges from GETTSIM by 2 euro).
SPLIT = {
    2026: [0, 24696, 60000, 77777, 100000, 120000, 200000],
    2025: [24342, 55555, 60000, 100000],   # 24342: BMF-Rechner-Spot-Check
    2024: [23634, 43139, 60000, 100000],   # 23634: BMF-Spot-Check; 43139: odd-Z (Diff -2)
}

# Splitting cases confirmed against the official BMF calculator (bmf-steuerrechner.de,
# 2026-07-09). Value = expected festzusetzende ESt in euro.
BMF_CONFIRMED = {(2024, 23634): 8, (2025, 24342): 20}


AN_PAUSCHBETRAG = 1230   # § 9a Satz 1 Nr. 1a EStG (GETTSIM, VZ 2024-2026)
SA_PAUSCHBETRAG = 36     # § 10c EStG (GETTSIM, VZ 2024-2026)

# End-to-end Arbeitnehmerfall (Einzelveranlagung): Bruttoarbeitslohn -> ESt.
ARBEITNEHMER = {
    2026: [30000, 60000, 100000],
    2025: [80000],
    2024: [50000],
}


def emit_arbeitnehmer(fid, beschreibung, year, brutto, expected):
    yaml = f"""id: {fid}
beschreibung: "{esc(beschreibung)}"

sachverhalt:
  veranlagungszeitraum: {year}
  veranlagung: einzel
  bruttoarbeitslohn: {brutto}
  werbungskosten: 0
  sonderausgaben: 0

erwartung:
  tarifliche_est: {expected}

quelle:
  authority: gesetz
  redistributable: true
  fundstelle: "Kette § 9a Satz 1 Nr. 1a + § 10c + § 32a EStG, VZ {year}"
  datei: "{SOURCE_FILE}"
  zitatanker: "{esc(ANKER_ABRUNDUNG)}"
"""
    with open(os.path.join(CASES, fid + ".yaml"), "w") as f:
        f.write(yaml)


def main():
    os.makedirs(CASES, exist_ok=True)
    for f in os.listdir(CASES):
        if f.endswith(".yaml"):
            os.remove(os.path.join(CASES, f))

    n = 0
    for year, xs in SINGLE.items():
        p = COEF[year]
        for x in xs:
            fid = f"g32a_{year}_single_{x}"
            emit(fid, f"Grundtarif, zvE {x} Euro, VZ {year}", year, "einzel",
                 x, tarif(x, p), ANKER_ABRUNDUNG, p["quelle"])
            n += 1
    for year, zs in SPLIT.items():
        p = COEF[year]
        for z in zs:
            fid = f"g32a_{year}_split_{z}"
            expected = splitting(z, p)
            fundstelle = "§ 32a Abs. 5 EStG i.V.m. Abs. 1; " + p["quelle"]
            if (year, z) in BMF_CONFIRMED:
                assert expected == BMF_CONFIRMED[(year, z)], \
                    f"BMF-confirmed value mismatch for {year}/{z}"
                fundstelle += "; amtlich bestaetigt BMF-Steuerrechner (bmf-steuerrechner.de, 2026-07-09)"
            emit(fid, f"Splitting, gemeinsames zvE {z} Euro, VZ {year}", year,
                 "zusammen", z, expected, ANKER_SPLITTING, fundstelle)
            n += 1
    for year, bs in ARBEITNEHMER.items():
        p = COEF[year]
        for b in bs:
            zve = max(0, b - AN_PAUSCHBETRAG - SA_PAUSCHBETRAG)
            fid = f"arbeitnehmer_{year}_einzel_{b}"
            emit_arbeitnehmer(fid, f"Arbeitnehmerfall Einzel, Bruttolohn {b} Euro, VZ {year}",
                              year, b, tarif(zve, p))
            n += 1
    print(f"generated {n} golden cases in golden/cases/")


if __name__ == "__main__":
    main()
