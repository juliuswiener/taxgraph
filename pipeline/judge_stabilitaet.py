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
import judge as J                          # noqa: E402
import gates as G                          # noqa: E402
from run import build_candidate            # noqa: E402

OUT = os.path.join(ROOT, "reports", "nachtschicht", "judge-stabilitaet.json")

# Welches Item speist welches blockierende Gate? Splits werden danach getrennt
# ausgewiesen - eine Splitrate ohne Gate-Bezug sagt nicht, was auf dem Spiel steht.
GATE_VON_ART = {"abweichung": "roundtrip", "annahme": "roundtrip",
                "norm_teil": "geltungsbereich"}


def _itemzahlen(v: dict) -> dict:
    inst = v.get("judge_instability") or {}
    items = {"abweichung": len(v.get("abweichungen", [])),
             "annahme": len(v.get("stille_zusatzannahmen", [])),
             "norm_teil": len(v.get("scope_gap", []))}
    # Abweichungen: nur die als echt beurteilten stehen im Verdikt; die Zahl der
    # BEURTEILTEN Items steht im Inventar-Cluster.
    cluster = inst.get("cluster") or {}
    beurteilt = {"abweichung": cluster.get("abweichungen", items["abweichung"]),
                 "annahme": cluster.get("annahmen", items["annahme"]),
                 "norm_teil": cluster.get("norm_teile", items["norm_teil"])}
    splits_je_art = {a: 0 for a in beurteilt}
    for sp in inst.get("item_splits", []):
        splits_je_art[sp["art"]] = splits_je_art.get(sp["art"], 0) + 1
    splits_je_gate: dict[str, int] = {}
    items_je_gate: dict[str, int] = {}
    for art, gate in GATE_VON_ART.items():
        splits_je_gate[gate] = splits_je_gate.get(gate, 0) + splits_je_art.get(art, 0)
        items_je_gate[gate] = items_je_gate.get(gate, 0) + beurteilt.get(art, 0)
    return {"items_beurteilt": sum(beurteilt.values()),
            "items_je_art": beurteilt,
            "splits_je_art": splits_je_art,
            "splits_je_gate": splits_je_gate,
            "items_je_gate": items_je_gate,
            "roh_items": inst.get("roh_items", {}),
            "merges": {f: len(m) for f, m in (inst.get("merge_log") or {}).items()},
            "inventar_streuung": {f: len(x) for f, x in
                                  (inst.get("inventar_streuung") or {}).items()}}


def messe(rid: str, n: int, cfg: dict, roles: dict, models_hash: str,
          client: OpenRouterClient) -> list[dict]:
    rule = next(x for x in cfg["regeln"] if x["rule_id"] == rid)
    cand = build_candidate(rule)
    rep = json.load(open(os.path.join(ROOT, "pipeline", "runs", "produktion", rid,
                                      "report.json"), encoding="utf-8"))
    src = rep["catala_a"]
    laeufe = []
    for i in range(n):
        v, prov_liste, kosten = J.judge_regel(
            client, roles["judge"], cand["norm_text"], src, cand["signature"],
            cand["geltungsbedingungen"], models_hash)

        class _P:  # Kostentraeger, damit die Auswertung unveraendert bleibt
            cost_usd = kosten
            truncated = False
            completion_tokens = sum(p["completion_tokens"] for p in prov_liste)
        prov = _P()
        if v.get("parse_error"):
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
            "item_splits": len((v.get("judge_instability") or {}).get("item_splits", [])),
            "items_ohne_mehrheit": len((v.get("judge_instability") or {}).get("items_ohne_mehrheit", [])),
            "lauf_id": v.get("lauf_id"),
            **_itemzahlen(v),
        })
        print(f"  {rid} Lauf {i+1}: abw={laeufe[-1]['abweichungen']} "
              f"annahmen={laeufe[-1]['annahmen']} (unmapped {unmapped}) "
              f"gaps={laeufe[-1]['scope_gaps']}/{laeufe[-1]['wirkt_hinein']} "
              f"splits={laeufe[-1]['item_splits']} "
              f"roundtrip={laeufe[-1]['roundtrip']} "
              f"geltungsbereich={laeufe[-1]['geltungsbereich']} "
              f"${laeufe[-1]['kosten_usd']}", flush=True)
    return laeufe


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("rule_ids", nargs="+")
    ap.add_argument("--n", type=int, default=3)
    ap.add_argument("--out", default=OUT, help="Zieldatei")
    ap.add_argument("--force", action="store_true",
                    help="vorhandene Messung ueberschreiben")
    args = ap.parse_args()

    # Eine Messung ist ein Datum, kein Zwischenstand. Sie wird nicht beilaeufig
    # ueberschrieben - beim zweiten Lauf dieser Nacht waere die erste Messung sonst
    # verloren gewesen.
    if os.path.exists(args.out) and not args.force:
        raise SystemExit(f"{args.out} existiert bereits. --out <andere Datei> oder "
                         f"--force, wenn die alte Messung wirklich weg soll.")

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
        items = sum(l.get("items_beurteilt", 0) for l in gut)
        splits = sum(l.get("item_splits", 0) for l in gut)
        sg = {g: sum(l["splits_je_gate"].get(g, 0) for l in gut) for g in ("roundtrip", "geltungsbereich")}
        ig = {g: sum(l["items_je_gate"].get(g, 0) for l in gut) for g in ("roundtrip", "geltungsbereich")}
        zusammenfassung[rid] = {
            "laeufe": len(laeufe),
            "parse_fehler": len(laeufe) - len(gut),
            "items_beurteilt": items,
            "item_splits": splits,
            "item_splitrate": round(splits / items, 4) if items else None,
            "splitrate_je_gate": {g: (round(sg[g] / ig[g], 4) if ig[g] else None)
                                  for g in sg},
            "merges": sum(sum(l["merges"].values()) for l in gut),
            "inventar_streuung_items": sum(sum(l["inventar_streuung"].values()) for l in gut),
            "gate_urteile_stabil": len(gates) == 1,
            "verschiedene_gate_urteile": sorted(f"{a}/{b}" for a, b in gates),
            "abweichungen_min_max": ([min(l["abweichungen"] for l in gut),
                                      max(l["abweichungen"] for l in gut)] if gut else None),
            "wirkt_hinein_min_max": ([min(l["wirkt_hinein"] for l in gut),
                                      max(l["wirkt_hinein"] for l in gut)] if gut else None),
            "kosten_usd": round(sum(l["kosten_usd"] for l in laeufe), 5),
        }

    os.makedirs(os.path.dirname(args.out), exist_ok=True)
    with open(args.out, "w", encoding="utf-8") as f:
        json.dump({"laeufe": ergebnis, "zusammenfassung": zusammenfassung}, f,
                  ensure_ascii=False, indent=2)
    print("\n" + json.dumps(zusammenfassung, ensure_ascii=False, indent=2))
    print(f"\ngeschrieben: {args.out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
