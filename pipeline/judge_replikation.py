"""Aggregat-Reproduzierbarkeit: liefert derselbe Input zweimal dasselbe Verdikt?

Messplan Punkt 3 (vorregistriert): statt eines vollen zweiten Durchgangs werden
zwei Regeln einmal repliziert - die vorher instabilste (§ 9 Abs. 4a) und eine
stabile als Kontrolle (§ 24b). Frage: identisches GESAMTVERDIKT, ja oder nein.

Verglichen wird eine kanonische Form, nicht der Rohtext. Zwei Verdikte sind
identisch, wenn sie
  * dieselben Abweichungen als echt beurteilen,
  * jede Annahme auf dieselbe Bedingung (oder auf keine) abbilden,
  * jeden Norm-Teil derselben Klasse und derselben abdeckenden Bedingung zuordnen.

Die Item-TEXTE koennen abweichen (das Inventar formuliert frei); verglichen werden
die Item-Cluster ueber denselben Aehnlichkeitsabgleich, den der Judge benutzt. Ein
Item, das nur ein Lauf sieht, macht die Verdikte verschieden - und genau das ist
die Frage.

    python pipeline/judge_replikation.py p9_4a_verpflegungsmehraufwand p24b_entlastungsbetrag
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
from run import build_candidate            # noqa: E402

OUT = os.path.join(ROOT, "reports", "nachtschicht", "judge-replikation.json")


def _kanonisch(v: dict) -> dict:
    return {
        "abweichungen": sorted(v.get("abweichungen", [])),
        "annahmen": sorted((a["annahme"], a["bedingung_id"])
                           for a in v.get("stille_zusatzannahmen", [])),
        "norm_teile": sorted((g["norm_teil"], g["klasse"], g.get("abgedeckt_von"))
                             for g in v.get("scope_gap", [])),
    }


def _vergleiche(a: dict, b: dict) -> dict:
    """Vergleicht zwei kanonische Verdikte item-weise ueber den Aehnlichkeitsabgleich."""
    bericht = {}
    for feld in ("abweichungen", "annahmen", "norm_teile"):
        xa, xb = a[feld], b[feld]
        text = (lambda x: x) if feld == "abweichungen" else (lambda x: x[0])
        rest_b = list(xb)
        gleich, nur_a = [], []
        for ia in xa:
            treffer = next((ib for ib in rest_b if J._gleich(text(ia), text(ib))), None)
            if treffer is None:
                nur_a.append(ia)
                continue
            rest_b.remove(treffer)
            # gleiches Item: stimmen auch die Urteile ueberein?
            gleich.append({"item": text(ia)[:110],
                           "urteil_gleich": (ia[1:] == treffer[1:]) if feld != "abweichungen" else True,
                           "a": ia[1:] if feld != "abweichungen" else None,
                           "b": treffer[1:] if feld != "abweichungen" else None})
        bericht[feld] = {
            "gemeinsam": len(gleich),
            "urteil_abweichend": [g for g in gleich if not g["urteil_gleich"]],
            "nur_in_lauf_a": [text(x)[:110] for x in nur_a],
            "nur_in_lauf_b": [text(x)[:110] for x in rest_b],
        }
    identisch = all(not b["urteil_abweichend"] and not b["nur_in_lauf_a"]
                    and not b["nur_in_lauf_b"] for b in bericht.values())
    return {"identisch": identisch, "je_feld": bericht}


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("rule_ids", nargs="+")
    ap.add_argument("--out", default=OUT)
    ap.add_argument("--force", action="store_true")
    args = ap.parse_args()

    if os.path.exists(args.out) and not args.force:
        raise SystemExit(f"{args.out} existiert bereits. --out oder --force.")

    cfg = load_yaml(os.path.join(HERE, "produktion", "rules.yaml"))
    roles, models_hash = load_roles()
    client = OpenRouterClient(dry_run=False)

    ergebnis = {}
    for rid in args.rule_ids:
        rule = next(x for x in cfg["regeln"] if x["rule_id"] == rid)
        cand = build_candidate(rule)
        rep = json.load(open(os.path.join(ROOT, "pipeline", "runs", "produktion", rid,
                                          "report.json"), encoding="utf-8"))
        src = rep["catala_a"]
        verdikte, kosten = [], 0.0
        for _ in range(2):
            v, prov, k = J.judge_regel(client, roles["judge"], cand["norm_text"], src,
                                       cand["signature"], cand["geltungsbedingungen"],
                                       models_hash)
            verdikte.append(v)
            kosten += k
        if any(v.get("parse_error") for v in verdikte):
            ergebnis[rid] = {"parse_error": True, "kosten_usd": round(kosten, 5)}
            print(f"  {rid}: PARSE-FEHLER"); continue
        v = _vergleiche(_kanonisch(verdikte[0]), _kanonisch(verdikte[1]))
        v["kosten_usd"] = round(kosten, 5)
        v["lauf_ids"] = [x["lauf_id"] for x in verdikte]
        ergebnis[rid] = v
        print(f"  {rid}: identisch={v['identisch']} | ${kosten:.4f}", flush=True)
        for feld, b in v["je_feld"].items():
            if b["urteil_abweichend"] or b["nur_in_lauf_a"] or b["nur_in_lauf_b"]:
                print(f"      {feld}: {b['gemeinsam']} gemeinsam, "
                      f"{len(b['urteil_abweichend'])} anderes Urteil, "
                      f"{len(b['nur_in_lauf_a'])} nur A, {len(b['nur_in_lauf_b'])} nur B")

    os.makedirs(os.path.dirname(args.out), exist_ok=True)
    with open(args.out, "w", encoding="utf-8") as f:
        json.dump(ergebnis, f, ensure_ascii=False, indent=2)
    print(f"\ngeschrieben: {args.out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
