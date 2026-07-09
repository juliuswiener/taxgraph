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
# 800: Kleinstlohn (Einkuenfte 0 nach § 9a Satz 2, zvE -36 nach § 10c, ESt 0).
ARBEITNEHMER = {
    2026: [800, 30000, 60000, 100000],
    2025: [80000],
    2024: [50000],
}


def arbeitnehmer_zve(brutto: int) -> int:
    """§ 9a Satz 2 (Pauschbetrag bis Einnahmen) + § 10c (ohne Untergrenze)."""
    einkuenfte = max(0, brutto - AN_PAUSCHBETRAG)   # § 9a Satz 2
    return einkuenfte - SA_PAUSCHBETRAG              # § 10c, negativ zulaessig


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


BMF_FILE = "sources/bmf/bmf_entfernungspauschalen_2021-11-18.txt"
P9_2026_FILE = "sources/gesetze-im-internet/estg_p9_abs1nr4_abs2_2026-07-09.txt"

# Entfernungspauschale-Faelle. (fid, VZ, sachverhalt, erwartung, anker, datei, fundstelle)
EP_CASES = [
    ("ep_2024_beispiel1_oepnv", 2024,
     dict(km=20.0, at=220, kfz=False, oepnv=1380), 1380,
     "220 x 20 x 0,30", BMF_FILE,
     "BMF-Schreiben 18.11.2021 Rz. 2, Beispiel 1 (OePNV-Guenstigerpruefung); VZ 2024 (km <= 20, jahresunabhaengig)"),
    ("ep_2024_rz12_volle_km", 2024,
     dict(km=10.6, at=200, kfz=False, oepnv=0), 600,
     "nur volle Kilometer der Entfernung anzusetzen", BMF_FILE,
     "BMF-Schreiben 18.11.2021 Rz. 12 (volle km); 10,6 km -> 10 km, 200 AT x 10 x 0,30 = 600"),
    ("ep_2024_staffel_30km", 2024,
     dict(km=30.0, at=220, kfz=True, oepnv=0), 2156,
     "restliche Entfernungskilometer x 0,38", BMF_FILE,
     "BMF-Schreiben 18.11.2021 Rz. 9 (Staffel 2024-2026); 20 x 0,30 + 10 x 0,38, 220 AT, eigenes Kfz"),
    ("ep_2026_flach_30km", 2026,
     dict(km=30.0, at=220, kfz=True, oepnv=0), 2508,
     "von 0,38 Euro anzusetzen", P9_2026_FILE,
     "§ 9 Abs. 1 S. 3 Nr. 4 Satz 2 EStG i.d.F. 2026 (StAendG 2025); ABGELEITET (0,38 ab km 1), nicht aus BMF-Beispiel zitiert"),
]


def emit_ep(fid, year, sv, expected, anker, datei, fundstelle):
    yaml = f"""id: {fid}
beschreibung: "Entfernungspauschale {sv['km']} km, {sv['at']} Arbeitstage, VZ {year}"

sachverhalt:
  veranlagungszeitraum: {year}
  entfernung_km_roh: {sv['km']}
  arbeitstage: {sv['at']}
  eigenes_oder_ueberlassenes_kfz: {str(sv['kfz']).lower()}
  oepnv_kosten_jahr: {sv['oepnv']}

erwartung:
  abziehbarer_betrag: {expected}

quelle:
  authority: {"verwaltung" if datei == BMF_FILE else "gesetz"}
  redistributable: true
  fundstelle: "{esc(fundstelle)}"
  datei: "{datei}"
  zitatanker: "{esc(anker)}"
"""
    with open(os.path.join(CASES, fid + ".yaml"), "w") as f:
        f.write(yaml)


P04_FILE = "sources/gesetze-im-internet/estg_p04_abs5_2026-07-09.txt"

# Arbeitszimmer/Homeoffice-Faelle (§ 4 Abs. 5 Nr. 6b/6c). (fid, VZ, sv, abzug_gesamt, anker, fundstelle)
HO_CASES = [
    ("ho_2024_tagespauschale_cap",
     dict(az=False, mp=False, aufw=0, jp=False, monate=0, tage=250), 1260,
     "6 Euro (Tagespauschale)",
     "§ 4 Abs. 5 S. 1 Nr. 6c Satz 1 EStG (6 Euro/Tag, hoechstens 1 260 Euro); 250 Tage -> Cap 1260"),
    ("ho_2024_jahrespauschale",
     dict(az=True, mp=True, aufw=0, jp=True, monate=0, tage=0), 1260,
     "1 260 Euro (Jahrespauschale)",
     "§ 4 Abs. 5 S. 1 Nr. 6b Satz 3 EStG (Jahrespauschale 1 260 Euro)"),
    ("ho_2024_ausschluss",
     dict(az=True, mp=True, aufw=2000, jp=False, monate=0, tage=100), 2000,
     "soweit ein Abzug nach Nummer 6b vorgenommen wird",
     "§ 4 Abs. 5 S. 1 Nr. 6c Satz 3 EStG (Ausschluss bei Abzug nach Nr. 6b); Arbeitszimmer 2000, Homeoffice 0"),
]


def emit_raumkosten(fid, sv, expected, anker, fundstelle):
    yaml = f"""id: {fid}
beschreibung: "Arbeitszimmer/Homeoffice, VZ 2024"

sachverhalt:
  veranlagungszeitraum: 2024
  arbeitszimmer_vorhanden: {str(sv['az']).lower()}
  ist_mittelpunkt: {str(sv['mp']).lower()}
  tatsaechliche_aufwendungen: {sv['aufw']}
  jahrespauschale_gewaehlt: {str(sv['jp']).lower()}
  monate_ohne_mittelpunkt: {sv['monate']}
  homeoffice_tage: {sv['tage']}

erwartung:
  abzug_gesamt: {expected}

quelle:
  authority: gesetz
  redistributable: true
  fundstelle: "{esc(fundstelle)}"
  datei: "{P04_FILE}"
  zitatanker: "{esc(anker)}"
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
            zve = arbeitnehmer_zve(b)
            fid = f"arbeitnehmer_{year}_einzel_{b}"
            emit_arbeitnehmer(fid, f"Arbeitnehmerfall Einzel, Bruttolohn {b} Euro, VZ {year}",
                              year, b, tarif(zve, p))
            n += 1
    # Splitting mit negativem gemeinsamem zvE (2 x Kleinstlohn 800 -> zvE -72):
    # sichert die korrekte Verarbeitung negativer Tarif-Inputs (truncate/floor).
    p = COEF[2026]
    emit("g32a_2026_split_negativ_72",
         "Splitting, gemeinsames zvE -72 Euro (2x Kleinstlohn), VZ 2026",
         2026, "zusammen", -72, splitting(-72, p), ANKER_SPLITTING,
         "§ 32a Abs. 5 EStG i.V.m. Abs. 1; " + p["quelle"])
    n += 1
    for fid, year, sv, expected, anker, datei, fundstelle in EP_CASES:
        emit_ep(fid, year, sv, expected, anker, datei, fundstelle)
        n += 1
    for fid, sv, expected, anker, fundstelle in HO_CASES:
        emit_raumkosten(fid, sv, expected, anker, fundstelle)
        n += 1
    print(f"generated {n} golden cases in golden/cases/")


if __name__ == "__main__":
    main()
