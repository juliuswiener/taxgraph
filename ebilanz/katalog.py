"""Baut den maschinenlesbaren E-Bilanz-Muss-Feld-Katalog je Taxonomie-Version
(WJ 2024 = 6.7, WJ 2025 = 6.8) aus den committeten XBRL-Linkbases + das
Muss ∩ W2-Mapping der 6 Andock-Cluster.

Deterministisch, reine stdlib, kein LLM, keine Cascade/Formalisierer, Registry
unberuehrt. Output: ebilanz/katalog_6.7.json + ebilanz/katalog_6.8.json.

Regenerieren:  python ebilanz/katalog.py        (schreibt die JSONs)
Der Gate (tests/test_ebilanz_katalog.py) parst frisch und prueft die JSONs +
die gepinnten Kennzahlen gegen Drift.
"""

from __future__ import annotations

import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import linkbase as L  # noqa: E402

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))

# Taxonomie-Version × Wirtschaftsjahr (dev-2-Befund: Version folgt dem WJ, nicht
# dem VZ; verpflichtend fuer WJ die nach dem 31.12.N beginnen).
VERSIONS = {
    "6.7": {"datum": "2023-04-01", "wj": "WJ 2024",
            "bmf_schreiben": "09.06.2023, IV C 6-S 2133-b/22/10002:002",
            "verpflichtend_wj_beginn_nach": "2023-12-31"},
    "6.8": {"datum": "2024-04-01", "wj": "WJ 2025",
            "bmf_schreiben": "27.05.2024, IV C 6-S 2133-b/24/10001:002, BStBl I S. 928",
            "verpflichtend_wj_beginn_nach": "2024-12-31"},
}

# Muss ∩ W2 — die 6 Andock-Cluster (dev-2 Report Abschn. 3, auf 6.7/6.8 re-verifiziert;
# 3 Scoping-Pfade korrigiert). Je Concept die ERWARTETE fiscalRequirement-Kategorie -
# der Gate prueft Existenz + Kategorie in BEIDEN Versionen (kein WJ-Drift zulaessig).
# hbst.transfer.* ist eine Tupel-Tabelle (kein Einzel-Muss, fiscalRequirement=None) -
# separat ueber die Concept-Zahl gefuehrt (20 xsd / 19 reference-fiscal, s. Gate).
W2_CLUSTER = [
    {"w2_regel": "p4_1_bv_vergleich", "norm": "§ 4 Abs. 1 EStG",
     "concepts": {
         "bs.eqLiab.equity.netIncome.taxBalanceGenerally": "Summenmussfeld",
         "bs.eqLiab.equity.netIncome.taxBalanceGenerally.transferDiffTaxAccounts": "Mussfeld",
     },
     "w2_fuellt": "steuerbilanzieller Gewinn + Ueberleitungswert"},
    {"w2_regel": "b2_massgeblichkeit", "norm": "§ 5 Abs. 1 EStG",
     "concepts": {},  # hbst.transfer.* Tupel, ueber tupel_prefix gefuehrt
     "tupel_prefix": "hbst.transfer",
     "w2_fuellt": "HandelsBil->SteuerBil-Ueberleitungsrechnung (Tupel)"},
    {"w2_regel": "p6_1_1_bewertung_av", "norm": "§ 6 Abs. 1 Nr. 1 EStG",
     "concepts": {"bs.ass.fixAss": "Summenmussfeld"},
     "w2_fuellt": "Anlagevermoegen-Wert"},
    {"w2_regel": "p6_1_3a_abzinsung", "norm": "§ 6 Abs. 1 Nr. 3a EStG",
     "concepts": {
         "bs.eqLiab.accruals": "Summenmussfeld",
         "bs.eqLiab.accruals.other": "Mussfeld, Kontennachweis erwünscht",
     },
     "w2_fuellt": "Rueckstellungswert"},
    {"w2_regel": "p6a_pension", "norm": "§ 6a EStG",
     "concepts": {
         "bs.eqLiab.accruals.pensions": "Summenmussfeld",
         "bs.eqLiab.accruals.pensions.direct": "Mussfeld",
         "bs.eqLiab.accruals.pensions.externalFunds": "Mussfeld",
         "bs.eqLiab.accruals.pensions.shareholder": "Mussfeld",
     },
     "w2_fuellt": "Pensionsrueckstellung"},
    {"w2_regel": "p5_5_aktiver_rap", "norm": "§ 5 Abs. 5 EStG",
     "concepts": {"bs.ass.prepaidExp": "Mussfeld"},
     "w2_fuellt": "aktiver Rechnungsabgrenzungsposten"},
]

# Kontennachweis-UEBERMITTLUNGSPFLICHT (JStG 2024) - Submission-Pflicht-KOMPONENTE
# auf Konten-Ebene, NICHT als de-gaap-ci-Muss-Feld fuehrbar (Auflage c). Die 52
# Concepts "Mussfeld, Kontennachweis erwünscht" sind das Positions-Flag, nicht die
# JStG-Pflicht.
KONTENNACHWEIS_PFLICHT = {
    "typ": "submission_pflicht_komponente",
    "kein_muss_feld": True,
    "norm": "§ 5b Abs. 1 Satz 1 EStG i.d.F. JStG 2024",
    "gesetz": "Jahressteuergesetz 2024 v. 02.12.2024, BGBl. 2024 I Nr. 387",
    "rechtlich_pflicht_ab": "WJ 2025 (WJ beginnt nach 31.12.2024)",
    "taxonomisch_umgesetzt_ab": "6.9 (WJ 2026)",
    "ueberbrueckung": "BMF-Nichtbeanstandung 10.06.2025, BStBl 2025 I S. 1450 (fuer 6.8-Uebermittlungen)",
    "w2_bezug": "Nicht-Gegenstand (W2-Regeln arbeiten positions-, nicht kontenweise); "
                "vom Extraktor als Pflicht-KOMPONENTE zu vermerken, nicht als Muss-Feld.",
}


def _paths(version: str) -> dict:
    v = VERSIONS[version]
    d = v["datum"]
    base = os.path.join(ROOT, "sources", "ebilanz", version, "xbrl")
    return {
        "reference_fiscal": os.path.join(base, f"de-gaap-ci-{d}-reference-fiscal.xml"),
        "label_de": os.path.join(base, f"de-gaap-ci-{d}-label-de.xml"),
        "xsd": os.path.join(base, f"de-gaap-ci-{d}.xsd"),
        "gcd_reference_fiscal": os.path.join(base, f"de-gcd-{d}-reference-fiscal.xml"),
        "presentation_bs": os.path.join(base, f"de-gaap-ci-{d}-presentation-balanceSheet.xml"),
    }


def parse_version(version: str) -> dict:
    """Roh-Parse einer Version: refmap, labels, concepts, cat, gcd-refmap."""
    p = _paths(version)
    refmap = L.parse_reference_fiscal(p["reference_fiscal"])
    labels = L.parse_labels(p["label_de"])
    concepts = L.parse_concepts_xsd(p["xsd"])
    gcd_ref = L.parse_reference_fiscal(p["gcd_reference_fiscal"], prefix="de-gcd_")
    return {"refmap": refmap, "labels": labels, "concepts": concepts,
            "cat": L.categorize(refmap), "gcd_ref": gcd_ref, "paths": p}


def _tupel_counts(version: str, prefix: str) -> dict:
    """Concept-Zahl eines Tupel-Prefix: xsd (alle) vs reference-fiscal (mit Eintrag)."""
    parsed = parse_version(version)
    xsd = {c for c in parsed["concepts"] if c.startswith(prefix)}
    ref = {c for c in parsed["refmap"] if c.startswith(prefix)}
    return {"xsd": sorted(xsd), "reference_fiscal": sorted(ref),
            "nur_xsd": sorted(xsd - ref)}


def build_katalog(version: str, parsed: dict | None = None,
                  parsed_prev: dict | None = None) -> dict:
    """Maschinenlesbarer Muss-Feld-Katalog fuer eine Version."""
    v = VERSIONS[version]
    parsed = parsed or parse_version(version)
    refmap, labels, cat = parsed["refmap"], parsed["labels"], parsed["cat"]
    weit = L.muss_weit(cat)
    eng = L.muss_eng(cat)

    def entry(c):
        f = refmap[c]
        return {"concept": c, "kategorie": f["fiscalRequirement"],
                "label_de": labels.get(c, ""),
                "legalFormEU": f["legalFormEU"], "legalFormPG": f["legalFormPG"],
                "legalFormKSt": f["legalFormKSt"]}

    # W2-Cluster mit aufgeloester Kategorie + Label (Existenz gegen xsd geprueft)
    cluster_out = []
    for cl in W2_CLUSTER:
        concepts = []
        for c, erwartet in cl["concepts"].items():
            ist = refmap.get(c, {}).get("fiscalRequirement")
            concepts.append({
                "concept": c, "erwartete_kategorie": erwartet, "ist_kategorie": ist,
                "label_de": labels.get(c, ""),
                "existiert_xsd": c in parsed["concepts"],
                "ok": (c in parsed["concepts"]) and (ist == erwartet),
            })
        row = {"w2_regel": cl["w2_regel"], "norm": cl["norm"],
               "w2_fuellt": cl["w2_fuellt"], "concepts": concepts}
        if "tupel_prefix" in cl:
            tc = _tupel_counts(version, cl["tupel_prefix"])
            row["tupel"] = {"prefix": cl["tupel_prefix"],
                            "anzahl_xsd": len(tc["xsd"]),
                            "anzahl_reference_fiscal": len(tc["reference_fiscal"]),
                            "nur_xsd": tc["nur_xsd"],
                            "hinweis": "Tupel-Tabelle, kein Einzel-Muss (fiscalRequirement=None)"}
        cluster_out.append(row)

    gcd_muss = L.muss_weit(L.categorize(parsed["gcd_ref"]))

    kat = {
        "version": version,
        "wj": v["wj"],
        "taxonomie_datum": v["datum"],
        "bmf_schreiben": v["bmf_schreiben"],
        "verpflichtend_wj_beginn_nach": v["verpflichtend_wj_beginn_nach"],
        "quelle_reference_fiscal":
            os.path.relpath(parsed["paths"]["reference_fiscal"], ROOT),
        "counts": {
            "concepts_mit_reference": len(refmap),
            "concepts_xsd": len(parsed["concepts"]),
            "mussfeld": len(cat.get("Mussfeld", set())),
            "summenmussfeld": len(cat.get("Summenmussfeld", set())),
            "kontennachweis_erwuenscht":
                len(cat.get("Mussfeld, Kontennachweis erwünscht", set())),
            "rechnerisch_notwendig":
                len(cat.get("Rechnerisch notwendig, soweit vorhanden", set())),
            "muss_weit": len(weit),
            "muss_eng": len(eng),
        },
        "w2_nenner": {
            "muss_weit_eu_oder_pg": len(L.w2_nenner(refmap, weit)),
            "kst_only": len(weit) - len(L.w2_nenner(refmap, weit)),
        },
        "gcd_stammdaten_muss": len(gcd_muss),
        "w2_cluster": cluster_out,
        "kontennachweis_pflicht": KONTENNACHWEIS_PFLICHT,
        "muss_felder": [entry(c) for c in sorted(weit)],
    }

    # PersG-Delta gegenueber der Vorversion (Auflage d: je WJ getrennt, kein
    # stiller Merge) - nur wenn Vorversion vorliegt (6.8 gegen 6.7).
    if parsed_prev is not None:
        prev_weit = L.muss_weit(parsed_prev["cat"])
        prev_ref = parsed_prev["refmap"]
        neu = sorted(weit - prev_weit)
        hoeher, brandneu = [], []
        for c in neu:
            if c in prev_ref:
                hoeher.append({"concept": c, "von": prev_ref[c]["fiscalRequirement"],
                               "nach": refmap[c]["fiscalRequirement"],
                               "label_de": labels.get(c, "")})
            else:
                brandneu.append({"concept": c, "kategorie": refmap[c]["fiscalRequirement"],
                                 "label_de": labels.get(c, "")})
        kat["persg_delta_zur_vorversion"] = {
            "vorversion_muss_weit": len(prev_weit),
            "neu_muss": len(neu),
            "entfallen_muss": len(prev_weit - weit),
            "hoeherstufungen": hoeher,       # Concept existierte, Kategorie hochgestuft
            "brandneue_concepts": brandneu,  # in Vorversion nicht vorhanden
            "hinweis": "Alle betreffen PersG-/Mitunternehmer-Positionen; Katalog je WJ "
                       "getrennt fuehren, kein stiller Merge.",
        }
    return kat


def main() -> int:
    parsed = {ver: parse_version(ver) for ver in VERSIONS}
    prev = {"6.7": None, "6.8": parsed["6.7"]}
    for ver in VERSIONS:
        kat = build_katalog(ver, parsed[ver], prev[ver])
        out = os.path.join(os.path.dirname(__file__), f"katalog_{ver}.json")
        with open(out, "w", encoding="utf-8") as fh:
            json.dump(kat, fh, ensure_ascii=False, indent=2, sort_keys=False)
            fh.write("\n")
        c = kat["counts"]
        print(f"{ver} ({kat['wj']}): muss_weit={c['muss_weit']} muss_eng={c['muss_eng']} "
              f"(Mussfeld={c['mussfeld']} Summenmuss={c['summenmussfeld']} "
              f"KN={c['kontennachweis_erwuenscht']}) -> {os.path.relpath(out, ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
