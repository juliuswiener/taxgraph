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

import functools
import glob
import os
import re
import sys

import yaml  # noqa: F401

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
sys.path.insert(0, ROOT)
# Der Rechenkern liegt seit 2026-08-19 in produkt/engine/ — dieses Skript muss den Ort
# SELBST auf den Pfad legen. Fehlte beim Umzug (c6380cd) und machte `python
# golden/golden_lauf.py` sofort unstartbar; gemeldet vom Abnahme-Audit, nicht von der Suite.
# Warum die Suite es nicht sah: tests/conftest.py legt produkt/engine für jeden pytest-Lauf
# hin, und mein "135/135"-Beleg kam aus einem Aufruf, der den Pfad ebenfalls mitbrachte —
# das Gate bekam seine Vorbedingung von mir geliefert. Deshalb steht die Gegenprobe jetzt
# in tests/test_ci_konfiguration.py und ruft das Skript als eigenen Prozess auf.
sys.path.insert(0, os.path.join(ROOT, "produkt", "engine"))

from yamlstrict import load_str  # noqa: E402

# Die Rechen-Accessoren liegen seit 2026-08-19 in produkt/engine/runner.py.
# Dieses Skript ist der GOLDEN-LAUF darüber: Fälle laden, rechnen, vergleichen,
# berichten. Es importiert den Kern — der Kern kennt es nicht (Richtung geprüft:
# er ruft keine dieser fünf Funktionen).
import runner  # noqa: E402  (produkt/engine/runner.py)
from runner import (  # noqa: E402
    _kinderfreibetrag,
    _verpflegung_kuerzung_cent,
    _verpflegung_pauschale,
    normalize,
    _UMLAUT,
    _load_yaml_path,
    _verpflegung_params,
    _verpflegung_roh_cent,
    catala_est,
    load_yaml_fh,
)





def main() -> int:
    cases = sorted(glob.glob(os.path.join(ROOT, "golden", "cases", "*.yaml")))
    if not cases:
        print("no golden cases found")
        return 1

    source_cache: dict[str, str] = {}
    failures = []

    for path in cases:
        c = load_yaml_fh(open(path, encoding="utf-8"))
        cid = c["id"]
        s = c["sachverhalt"]
        erw = c["erwartung"]
        exp = erw.get("tarifliche_est",
                      erw.get("festzusetzende_est",
                              erw.get("abziehbarer_betrag",
                                      erw.get("abzug_gesamt",
                                              erw.get("gewst_cent",
                                                      erw.get("nenner_b_cent",
                                                              erw.get("sanierung_ermaessigung_cent",
                                                                      erw.get("nutzungswert_monat_cent"))))))))
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


# Der Einstiegspunkt gehört hierher, nicht in den Kern: dieses Skript IST der
# Golden-Lauf, produkt/engine/runner.py ist eine Bibliothek. Beim Umzug am 2026-08-19 blieb
# er versehentlich dort zurück (er ist kein `def`, also hat ihn die Funktions-Extraktion
# nicht mitgenommen) — mit der Folge, dass `python golden/golden_lauf.py` mit Exit 0
# zurückkam, ohne einen einzigen Fall zu rechnen. Ein grünes Gate, das nichts prüft.
if __name__ == "__main__":
    sys.exit(main())
