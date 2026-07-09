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
from catala_runtime import Money  # noqa: E402

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
    veranlagung = sachverhalt["veranlagung"]
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
        exp = c["erwartung"]["tarifliche_est"]
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
