"""Registry der Dauersplitter (objektiv ambige Items).

Protokolldekret 2026-07-10, Punkt 4: Items - vor allem `wirkt_hinein`-Norm-Teile -,
die ueber Kampagnen hinweg wiederholt 2:1 splitten, werden nicht in jeder Messung
neu ausgewuerfelt. Sie gelten als objektiv ambig (haengen von einer
Signatur-Interpretation ab) und routen fest in die Review-Queue.

Die Registry liegt in `pipeline/grenzfaelle.yaml`: rule_id -> Liste von
String-Schluesseln (der `schluessel` aus judge.py, JSON-serialisierter Anker).

    from grenzfaelle import dauersplitter
    keys = dauersplitter("p9_4a_verpflegungsmehraufwand")

Aufbau/Aktualisierung aus Messdaten:
    python pipeline/grenzfaelle.py bauen reports/nachtschicht/judge-stabilitaet-*.json
"""

from __future__ import annotations

import glob
import json
import os
import sys
from collections import defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
REGISTRY = os.path.join(HERE, "grenzfaelle.yaml")

sys.path.insert(0, ROOT)
from yamlstrict import load_yaml   # noqa: E402

# Ein Item gilt als Dauersplitter, wenn es in mindestens so vielen getrennten
# Laeufen gesplittet hat. "ueber Kampagnen hinweg wiederholt" -> mindestens zwei.
MIN_SPLIT_LAEUFE = 2


def _registry() -> dict:
    if not os.path.exists(REGISTRY):
        return {}
    return load_yaml(REGISTRY) or {}


def dauersplitter(rule_id: str) -> set[str]:
    eintrag = _registry().get(rule_id) or []
    return {e["schluessel"] if isinstance(e, dict) else e for e in eintrag}


def _sammle(messungen: list[str]) -> dict[str, dict[str, dict]]:
    """rule_id -> schluessel -> {split_laeufe, laeufe, beispiel}.

    Liest die per-Lauf gespeicherten Split-Schluessel (`item_split_keys`) aus den
    Stabilitaets-Messdateien.
    """
    agg: dict[str, dict[str, dict]] = defaultdict(lambda: defaultdict(
        lambda: {"split_laeufe": 0, "laeufe": 0, "beispiel": ""}))
    for pfad in messungen:
        d = json.load(open(pfad, encoding="utf-8"))
        for rid, laeufe in d.get("laeufe", {}).items():
            for lauf in laeufe:
                if lauf.get("parse_error"):
                    continue
                fuer_lauf = {k["schluessel"]: k for k in lauf.get("item_split_keys", [])}
                for sk, info in fuer_lauf.items():
                    e = agg[rid][sk]
                    e["split_laeufe"] += 1
                    e["beispiel"] = info.get("item", "")
    return agg


def bauen(messungen: list[str]) -> dict:
    agg = _sammle(messungen)
    neu = {}
    for rid, items in sorted(agg.items()):
        dauer = [{"schluessel": sk, "split_laeufe": e["split_laeufe"],
                  "beispiel": e["beispiel"]}
                 for sk, e in items.items() if e["split_laeufe"] >= MIN_SPLIT_LAEUFE]
        if dauer:
            neu[rid] = sorted(dauer, key=lambda x: -x["split_laeufe"])
    return neu


def main() -> int:
    if len(sys.argv) < 3 or sys.argv[1] != "bauen":
        print(__doc__)
        return 2
    muster = sys.argv[2:]
    dateien = [f for m in muster for f in glob.glob(m)]
    if not dateien:
        raise SystemExit(f"keine Messdateien zu {muster}")
    neu = bauen(dateien)
    import yaml
    kopf = ("# Dauersplitter-Registry (Protokolldekret 2026-07-10, Punkt 4).\n"
            "# Objektiv ambige Items, die ueber Kampagnen hinweg wiederholt 2:1\n"
            "# splitten. Sie routen fest in die Review-Queue statt neu ausgewuerfelt\n"
            f"# zu werden. Erzeugt aus: {', '.join(os.path.basename(d) for d in dateien)}\n\n")
    with open(REGISTRY, "w", encoding="utf-8") as f:
        f.write(kopf + yaml.safe_dump(neu, allow_unicode=True, sort_keys=True))
    n = sum(len(v) for v in neu.values())
    print(f"{n} Dauersplitter ueber {len(neu)} Regel(n) -> {REGISTRY}")
    for rid, items in neu.items():
        print(f"  {rid}: {len(items)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
