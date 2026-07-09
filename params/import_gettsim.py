"""Import scalar parameters from GETTSIM parameter files into the TaxGraph
parameter layer (params/<vz>/<name>.yaml), with full provenance.

GETTSIM is a data source and cross-check oracle, never a legal source: the
`rechtsquelle` field always points to the statute, the GETTSIM origin goes into
`datenquelle` (version, path, the date-keyed entry used, import date).

For each configured parameter and Veranlagungszeitraum, the effective GETTSIM
entry (latest date <= <vz>-01-01) is selected and written out.

Run: python params/import_gettsim.py   (or: make params-import)
"""

from __future__ import annotations

import datetime
import os

import gettsim
import yaml

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
GETTSIM_BASE = os.path.dirname(gettsim.__file__)
GETTSIM_VERSION = getattr(gettsim, "__version__", "?")
# Import date. Kept explicit (not datetime.now) so re-runs are reproducible;
# update when re-importing from a new GETTSIM version.
IMPORT_DATE = "2026-07-09"

VZ_LIST = (2024, 2025, 2026)

# Parameters to import. `gettsim_rel` is the path within the GETTSIM package,
# `key` the top-level parameter, `rechtsquelle` the statutory citation (NOT from
# GETTSIM), `out` the output file name.
IMPORTS = [
    dict(
        out="arbeitnehmerpauschbetrag",
        gettsim_rel="germany/einkommensteuer/einkünfte/aus_nichtselbstständiger_arbeit/werbungskostenpauschale.yaml",
        key="arbeitnehmerpauschbetrag",
        einheit="euro",
        rechtsquelle={"gesetz": "EStG", "paragraph": "9a", "absatz": "1", "nummer": "1a"},
        # Primaerquelle: eingefrorene Gesetzesfassung. GETTSIM dient nur als Prueinstanz.
        gesetzesquelle="§ 9a Satz 1 Nr. 1a EStG, sources/gesetze-im-internet/estg_p9a_2026-07-09.txt",
        kommentar="Arbeitnehmer-Pauschbetrag (Werbungskostenpauschbetrag) nach § 9a Satz 1 Nr. 1a EStG.",
    ),
    dict(
        out="sonderausgabenpauschbetrag",
        gettsim_rel="germany/einkommensteuer/abzüge/sonderausgaben.yaml",
        key="sonderausgabenpauschbetrag",
        einheit="euro",
        rechtsquelle={"gesetz": "EStG", "paragraph": "10c", "absatz": "", "satz": "1"},
        gesetzesquelle="§ 10c Satz 1 EStG, sources/gesetze-im-internet/estg_p10c_2026-07-09.txt",
        kommentar="Sonderausgaben-Pauschbetrag nach § 10c EStG (je Person; bei Zusammenveranlagung verdoppelt, § 10c Satz 2).",
    ),
]


def effective_entry(param: dict, vz: int):
    """Return (date_str, entry) of the latest date-keyed entry <= <vz>-01-01.

    PyYAML parses `2023-01-01:` keys as datetime.date objects.
    """
    cutoff = datetime.date(vz, 1, 1)
    dated = []
    for k, v in param.items():
        if not isinstance(v, dict):
            continue
        d = k if isinstance(k, datetime.date) else None
        if d is None and isinstance(k, str):
            try:
                d = datetime.date.fromisoformat(k)
            except ValueError:
                d = None
        if d is not None and d <= cutoff:
            dated.append((d, v))
    if not dated:
        raise ValueError(f"no GETTSIM entry <= {cutoff.isoformat()}")
    d, entry = sorted(dated)[-1]
    return d.isoformat(), entry


def rq_inline(rq: dict) -> str:
    parts = ", ".join(f'{k}: "{v}"' for k, v in rq.items() if v != "")
    return "{" + parts + "}"


def emit(spec: dict, vz: int, date_str: str, entry: dict):
    value = entry["value"]
    ref = entry.get("reference") or entry.get("note") or ""
    ref = f", Eintrag {date_str} ({ref})" if ref else f", Eintrag {date_str}"
    datenquelle = (f"{spec['gesetzesquelle']}; Wert bestaetigt durch GETTSIM "
                   f"{GETTSIM_VERSION} ({spec['gettsim_rel']}{ref}) als Prueinstanz; "
                   f"Stand {IMPORT_DATE}")
    out = f"""# {spec['kommentar']}
# Veranlagungszeitraum {vz}. Wert aus der eingefrorenen Gesetzesfassung
# (sources/), gegengeprueft mit GETTSIM (params/import_gettsim.py). GETTSIM ist
# Prueinstanz, nicht Rechtsquelle.

parameter: {spec['out']}
veranlagungszeitraum: {vz}
authority: gesetz
redistributable: true
gueltig_ab: "{vz}-01-01"

wert:
  wert: {value}
  einheit: {spec['einheit']}
  veranlagungszeitraum: {vz}
  rechtsquelle: {rq_inline(spec['rechtsquelle'])}
  datenquelle: "{datenquelle}"
"""
    out_dir = os.path.join(ROOT, "params", str(vz))
    os.makedirs(out_dir, exist_ok=True)
    with open(os.path.join(out_dir, spec["out"] + ".yaml"), "w") as f:
        f.write(out)


def main():
    n = 0
    for spec in IMPORTS:
        path = os.path.join(GETTSIM_BASE, spec["gettsim_rel"])
        data = yaml.safe_load(open(path, encoding="utf-8"))
        param = data[spec["key"]]
        for vz in VZ_LIST:
            date_str, entry = effective_entry(param, vz)
            emit(spec, vz, date_str, entry)
            n += 1
            print(f"{spec['out']} VZ {vz}: {entry['value']} (GETTSIM entry {date_str})")
    print(f"\nwrote {n} parameter files from GETTSIM {GETTSIM_VERSION}")


if __name__ == "__main__":
    main()
