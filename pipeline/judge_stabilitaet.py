"""Streuung der Judge-Verdikte bei Temperatur 0 messen.

Bei `p9_4a` hat der Judge in zwei Laeufen mit identischem Input unterschiedlich
geurteilt: einmal eine `abweichung` und ein gruener Geltungsbereich, einmal keine
Abweichung und ein zusaetzlicher `wirkt_hinein`. Beide Verdikte sind vertretbar -
aber ein Gate, das ueber Rechtsregeln entscheidet, darf bei gleichem Input nicht
verschiedene Antworten geben.

Dieses Skript misst die Streuung, statt sie zu vermuten: dieselbe Regel, derselbe
Quelltext, N Judge-Laeufe. Ausgabe nach reports/nachtschicht/judge-stabilitaet.json.

Es aendert nichts an der Pipeline und schreibt keinen Report um.

    python pipeline/judge_stabilitaet.py --n 3 p9_4a_verpflegungsmehraufwand
"""

from __future__ import annotations

import argparse
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, ROOT)
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, "produktion"))

from yamlstrict import load_yaml          # noqa: E402
from client import OpenRouterClient       # noqa: E402
from provenance import load_roles         # noqa: E402
import roles as R                          # noqa: E402
import gates as G                          # noqa: E402
from run import build_candidate            # noqa: E402

OUT = os.path.join(ROOT, "reports", "nachtschicht", "judge-stabilitaet.json")


def messe(rid: str, n: int, cfg: dict, roles: dict, models_hash: str,
          client: OpenRouterClient) -> list[dict]:
    rule = next(x for x in cfg["regeln"] if x["rule_id"] == rid)
    cand = build_candidate(rule)
    rep = json.load(open(os.path.join(ROOT, "pipeline", "runs", "produktion", rid,
                                      "report.json"), encoding="utf-8"))
    src = rep["catala_a"]
    laeufe = []
    for i in range(n):
        text, prov = R.roundtrip(client, roles["judge"], cand["norm_text"], src,
                                 models_hash, signature=cand["signature"],
                                 geltungsbedingungen=cand["geltungsbedingungen"])
        v = G.roundtrip_parse(text)
        if v.get("parse_error") or prov.truncated:
            # Ein unlesbares oder abgeschnittenes Verdikt ist selbst ein Messwert:
            # es zeigt, wie stabil der Judge antwortet. Es darf die Messung nicht
            # abbrechen.
            laeufe.append({"lauf": i + 1, "parse_error": True,
                           "truncated": bool(prov.truncated),
                           "completion_tokens": prov.completion_tokens,
                           "roundtrip": G.FAIL, "geltungsbereich": G.FAIL,
                           "abweichungen": -1, "annahmen": -1, "unmapped": -1,
                           "scope_gaps": -1, "wirkt_hinein": -1,
                           "kosten_usd": round(prov.cost_usd, 5)})
            print(f"  {rid} Lauf {i+1}: PARSE-FEHLER "
                  f"(truncated={prov.truncated}, tokens={prov.completion_tokens})",
                  flush=True)
            continue
        unmapped = sum(1 for a in v["stille_zusatzannahmen"] if not a["bedingung_id"])
        laeufe.append({
            "lauf": i + 1,
            "parse_error": False,
            "faithful": v["faithful"],
            "abweichungen": len(v["abweichungen"]),
            "abweichungen_text": [a[:120] for a in v["abweichungen"]],
            "annahmen": len(v["stille_zusatzannahmen"]),
            "unmapped": unmapped,
            "scope_gaps": len(v["scope_gap"]),
            "wirkt_hinein": sum(1 for g in v["scope_gap"] if g["klasse"] != "unabhaengig"),
            "roundtrip": G.roundtrip_gate(v, cand).status,
            "geltungsbereich": G.geltungsbereich_gate(v, cand).status,
            "kosten_usd": round(prov.cost_usd, 5),
        })
        print(f"  {rid} Lauf {i+1}: abw={laeufe[-1]['abweichungen']} "
              f"annahmen={laeufe[-1]['annahmen']} (unmapped {unmapped}) "
              f"gaps={laeufe[-1]['scope_gaps']}/{laeufe[-1]['wirkt_hinein']} "
              f"roundtrip={laeufe[-1]['roundtrip']} "
              f"geltungsbereich={laeufe[-1]['geltungsbereich']}", flush=True)
    return laeufe


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("rule_ids", nargs="+")
    ap.add_argument("--n", type=int, default=3)
    args = ap.parse_args()

    cfg = load_yaml(os.path.join(HERE, "produktion", "rules.yaml"))
    roles, models_hash = load_roles()
    client = OpenRouterClient(dry_run=False)

    ergebnis = {}
    for rid in args.rule_ids:
        ergebnis[rid] = messe(rid, args.n, cfg, roles, models_hash, client)

    # Stabilitaet: sind die Gate-Urteile ueber alle Laeufe identisch?
    zusammenfassung = {}
    for rid, laeufe in ergebnis.items():
        gates = {(l["roundtrip"], l["geltungsbereich"]) for l in laeufe}
        gut = [l for l in laeufe if not l.get("parse_error")]
        zusammenfassung[rid] = {
            "laeufe": len(laeufe),
            "parse_fehler": len(laeufe) - len(gut),
            "gate_urteile_stabil": len(gates) == 1,
            "verschiedene_gate_urteile": sorted(f"{a}/{b}" for a, b in gates),
            "abweichungen_min_max": ([min(l["abweichungen"] for l in gut),
                                      max(l["abweichungen"] for l in gut)] if gut else None),
            "wirkt_hinein_min_max": ([min(l["wirkt_hinein"] for l in gut),
                                      max(l["wirkt_hinein"] for l in gut)] if gut else None),
            "kosten_usd": round(sum(l["kosten_usd"] for l in laeufe), 5),
        }

    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, "w", encoding="utf-8") as f:
        json.dump({"laeufe": ergebnis, "zusammenfassung": zusammenfassung}, f,
                  ensure_ascii=False, indent=2)
    print("\n" + json.dumps(zusammenfassung, ensure_ascii=False, indent=2))
    print(f"\ngeschrieben: {OUT}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
