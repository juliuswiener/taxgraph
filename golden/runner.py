"""Golden-corpus runner: check the Catala § 32a formalisation against the
curated cases in golden/cases/, and verify each case's citation anchor against
the frozen source text (hard gate).

For every case:
  1. the `zitatanker` must occur (after normalisation) in the referenced
     sources/ document;
  2. the Catala-computed tarifliche ESt must equal `erwartung.tarifliche_est`.

Exit code 0 only if all cases pass. Requires the assembled Catala package
(bash oracle/gettsim/assemble_catala.sh) and PyYAML.

Run: python golden/runner.py   (or: make golden)
"""

from __future__ import annotations

import glob
import os
import re
import sys

import yaml

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
_CAT = os.path.join(ROOT, "oracle", "gettsim", "_catala")
sys.path.insert(0, os.path.join(_CAT, "rt"))
sys.path.insert(0, _CAT)

from pkg import Einkommensteuertarif as E  # noqa: E402  (Catala-generated)
from pkg import Entfernungspauschale as EP  # noqa: E402
from pkg import Arbeitszimmer_homeoffice as AZ  # noqa: E402
from catala_runtime import Money, Decimal, Bool  # noqa: E402


def _az_params(year: int) -> dict:
    p = yaml.safe_load(open(os.path.join(
        ROOT, "params", str(year), "arbeitszimmer_homeoffice.yaml"), encoding="utf-8"))
    return {k: p[k]["wert"] for k in
            ("jahrespauschale", "tagespauschale_pro_tag", "tagespauschale_hoechstbetrag")}


def catala_raumkosten(s: dict) -> int:
    r = _az_params(s["veranlagungszeitraum"])
    out = AZ.raumkostenabzug(AZ.RaumkostenabzugIn(
        arbeitszimmer_vorhanden_in=Bool(s.get("arbeitszimmer_vorhanden", False)),
        ist_mittelpunkt_in=Bool(s.get("ist_mittelpunkt", False)),
        tatsaechliche_aufwendungen_in=Money(f"{int(s.get('tatsaechliche_aufwendungen', 0))}.00"),
        jahrespauschale_gewaehlt_in=Bool(s.get("jahrespauschale_gewaehlt", False)),
        monate_ohne_mittelpunkt_in=int(s.get("monate_ohne_mittelpunkt", 0)),
        homeoffice_tage_in=int(s.get("homeoffice_tage", 0)),
        jahrespauschale_in=Money(f"{int(r['jahrespauschale'])}.00"),
        tagespauschale_pro_tag_in=Money(f"{int(r['tagespauschale_pro_tag'])}.00"),
        tagespauschale_hoechstbetrag_in=Money(f"{int(r['tagespauschale_hoechstbetrag'])}.00")))
    return int(out.abzug_gesamt) // 100


def _ep_saetze(year: int) -> dict:
    """Read the Entfernungspauschale rates for a VZ from params/."""
    p = yaml.safe_load(open(os.path.join(
        ROOT, "params", str(year), "entfernungspauschale.yaml"), encoding="utf-8"))
    return {k: p[k]["wert"] for k in
            ("satz_bis_20_km", "satz_ab_21_km", "staffelgrenze_km", "hoechstbetrag_ohne_kfz")}


def catala_entfernungspauschale(s: dict) -> int:
    year = s["veranlagungszeitraum"]
    r = _ep_saetze(year)
    out = EP.berechnung(EP.BerechnungIn(
        entfernung_km_roh_in=Decimal(str(s["entfernung_km_roh"])),
        arbeitstage_in=int(s["arbeitstage"]),
        eigenes_oder_ueberlassenes_kfz_in=Bool(s.get("eigenes_oder_ueberlassenes_kfz", False)),
        oepnv_kosten_jahr_in=Money(f"{int(s.get('oepnv_kosten_jahr', 0))}.00"),
        satz_bis_20_km_in=Money(f"{r['satz_bis_20_km']:.2f}"),
        satz_ab_21_km_in=Money(f"{r['satz_ab_21_km']:.2f}"),
        staffelgrenze_km_in=int(r["staffelgrenze_km"]),
        hoechstbetrag_in=Money(f"{int(r['hoechstbetrag_ohne_kfz'])}.00")))
    return int(out.abziehbarer_betrag) // 100

VZ_ENUM = {
    2024: E.Veranlagungszeitraum(E.Veranlagungszeitraum.Code.VZ2024, None),
    2025: E.Veranlagungszeitraum(E.Veranlagungszeitraum.Code.VZ2025, None),
    2026: E.Veranlagungszeitraum(E.Veranlagungszeitraum.Code.VZ2026, None),
}

_UMLAUT = str.maketrans({"ä": "ae", "ö": "oe", "ü": "ue", "ß": "ss",
                         "Ä": "ae", "Ö": "oe", "Ü": "ue"})


def normalize(text: str) -> str:
    return re.sub(r"\s+", " ", text.translate(_UMLAUT).lower()).strip()


def catala_est(sachverhalt: dict) -> int:
    year = sachverhalt["veranlagungszeitraum"]
    veranlagung = sachverhalt.get("veranlagung")
    # Entfernungspauschale (§ 9): abziehbarer Betrag.
    if "entfernung_km_roh" in sachverhalt:
        return catala_entfernungspauschale(sachverhalt)
    # Arbeitszimmer/Homeoffice (§ 4 Abs. 5 Nr. 6b/6c): abzug_gesamt.
    if "arbeitszimmer_vorhanden" in sachverhalt:
        return catala_raumkosten(sachverhalt)
    # End-to-end Arbeitnehmerfall (Bruttolohn -> festzusetzende ESt).
    if "bruttoarbeitslohn" in sachverhalt:
        out = E.festzusetzende_est_einzel(E.FestzusetzendeEstEinzelIn(
            bruttoarbeitslohn_in=Money(f"{int(sachverhalt['bruttoarbeitslohn'])}.00"),
            werbungskosten_in=Money(f"{int(sachverhalt.get('werbungskosten', 0))}.00"),
            sonderausgaben_in=Money(f"{int(sachverhalt.get('sonderausgaben', 0))}.00"),
            veranlagungszeitraum_in=VZ_ENUM[year]))
        return int(out.festzusetzende_est) // 100
    # Tariff-level case (zvE -> tarifliche ESt).
    m = Money(f"{int(sachverhalt['zu_versteuerndes_einkommen'])}.00")
    if veranlagung == "einzel":
        out = E.grundtarif(E.GrundtarifIn(
            zu_versteuerndes_einkommen_in=m, veranlagungszeitraum_in=VZ_ENUM[year]))
    elif veranlagung == "zusammen":
        out = E.splittingtarif(E.SplittingtarifIn(
            zu_versteuerndes_einkommen_gemeinsam_in=m, veranlagungszeitraum_in=VZ_ENUM[year]))
    else:
        raise ValueError(f"unknown veranlagung: {veranlagung}")
    return int(out.tarifliche_steuer) // 100


def main() -> int:
    cases = sorted(glob.glob(os.path.join(ROOT, "golden", "cases", "*.yaml")))
    if not cases:
        print("no golden cases found")
        return 1

    source_cache: dict[str, str] = {}
    failures = []

    for path in cases:
        c = yaml.safe_load(open(path, encoding="utf-8"))
        cid = c["id"]
        s = c["sachverhalt"]
        erw = c["erwartung"]
        exp = erw.get("tarifliche_est",
                      erw.get("abziehbarer_betrag", erw.get("abzug_gesamt")))
        q = c["quelle"]

        # 1. citation-anchor gate
        src_path = os.path.join(ROOT, q["datei"])
        if src_path not in source_cache:
            source_cache[src_path] = normalize(open(src_path, encoding="utf-8").read())
        anchor_ok = normalize(q["zitatanker"]) in source_cache[src_path]

        # 2. value check against Catala
        got = catala_est(s)
        value_ok = got == exp

        if anchor_ok and value_ok:
            print(f"OK       {cid}  (est={got})")
        else:
            reason = []
            if not anchor_ok:
                reason.append("Zitatanker nicht im Quelltext")
            if not value_ok:
                reason.append(f"Wert Catala={got} != erwartet={exp}")
            print(f"FAIL     {cid}  -> {'; '.join(reason)}")
            failures.append(cid)

    print(f"\n{len(cases) - len(failures)}/{len(cases)} Faelle bestanden.")
    return 0 if not failures else 1


if __name__ == "__main__":
    sys.exit(main())
