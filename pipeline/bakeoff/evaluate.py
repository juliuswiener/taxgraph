"""G2 bake-off evaluation: aggregate run reports into reports/gate-g2.md.

Consumes the report.json files a bake-off run writes (one per candidate per
pairing) and computes the metrics from Nachtrag §5. Decision rule: lowest
escalation rate wins; cost is only a tiebreaker.

Run (after the bake-off, which Julius releases separately):
    pipeline/.venv/bin/python pipeline/bakeoff/evaluate.py pipeline/runs
"""

from __future__ import annotations

import glob
import json
import os
import sys
from collections import defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))


def load_reports(runs_dir: str) -> list[dict]:
    out = []
    for p in glob.glob(os.path.join(runs_dir, "**", "report.json"), recursive=True):
        try:
            out.append(json.load(open(p, encoding="utf-8")))
        except json.JSONDecodeError:
            print(f"skip unreadable report: {p}", file=sys.stderr)
    return out


def _gate(r: dict, name: str) -> str | None:
    for g in r.get("gates", []):
        if g["name"] == name:
            return g["status"]
    return None


def pairing_of(r: dict) -> str:
    """Identify the pairing by the formaliser-B slug in the provenance."""
    for p in r.get("provenance", []):
        if p["role"] == "formalisierer_b":
            return p["slug"].replace(" (dry-run)", "")
    return "unknown"


def rate(n: int, d: int) -> float:
    return (n / d) if d else 0.0


def evaluate(reports: list[dict]) -> dict:
    by = defaultdict(list)
    for r in reports:
        by[pairing_of(r)].append(r)

    result = {}
    for pairing, rs in by.items():
        n = len(rs)
        syntax_ok = sum(1 for r in rs
                        if _gate(r, "syntax_a") == "PASS" and _gate(r, "syntax_b") == "PASS"
                        and _gate(r, "typecheck_a") in ("PASS", None)
                        and _gate(r, "typecheck_b") in ("PASS", None))
        equiv_div = sum(1 for r in rs if _gate(r, "equivalence") == "FAIL")
        rt_dev = sum(1 for r in rs if _gate(r, "roundtrip") == "FAIL")
        silent = sum(1 for r in rs if "annahmen=[" in str(r.get("gates")) and
                     "annahmen=[]" not in str(r.get("gates")))
        escalated = sum(1 for r in rs if r.get("queue_status") == "flagged_for_review")
        blind = [r for r in rs if "blind_repro_match" in r]
        blind_ok = sum(1 for r in blind if r["blind_repro_match"])
        approved = [r for r in rs if r.get("queue_status", "").startswith("verified")]
        cost = sum(r.get("total_cost_usd", 0.0) for r in rs)

        result[pairing] = {
            "n": n,
            "syntaxvaliditaet": rate(syntax_ok, n),
            "aequivalenz_divergenzrate": rate(equiv_div, n),
            "roundtrip_abweichungsrate": rate(rt_dev, n),
            "stille_zusatzannahmen": rate(silent, n),
            "blind_reproduktion": rate(blind_ok, len(blind)) if blind else None,
            "eskalationsrate": rate(escalated, n),
            "kosten_gesamt_usd": round(cost, 6),
            "kosten_pro_approved_usd": round(cost / len(approved), 6) if approved else None,
        }
    return result


def to_markdown(res: dict) -> str:
    L = ["# Gate G2: Bake-off-Auswertung\n",
         "Entscheid: niedrigste Eskalationsrate gewinnt; Kosten nur als Tiebreaker.\n"]
    if not res:
        L.append("Keine Laeufe gefunden. Der Bake-off wurde noch nicht gestartet "
                 "(Freigabe durch Julius steht aus).\n")
        return "\n".join(L)

    cols = ["n", "syntaxvaliditaet", "aequivalenz_divergenzrate",
            "roundtrip_abweichungsrate", "stille_zusatzannahmen",
            "blind_reproduktion", "eskalationsrate", "kosten_pro_approved_usd"]
    L.append("| Paarung (Formalisierer B) | " + " | ".join(cols) + " |")
    L.append("|" + "---|" * (len(cols) + 1))
    for pairing, m in sorted(res.items()):
        cells = []
        for c in cols:
            v = m[c]
            cells.append("n/a" if v is None else (f"{v:.3f}" if isinstance(v, float) else str(v)))
        L.append(f"| {pairing} | " + " | ".join(cells) + " |")

    ranked = sorted(res.items(), key=lambda kv: (kv[1]["eskalationsrate"],
                                                 kv[1]["kosten_pro_approved_usd"] or 0.0))
    L.append(f"\n**Empfehlung:** `{ranked[0][0]}` "
             f"(Eskalationsrate {ranked[0][1]['eskalationsrate']:.3f}). "
             f"Entscheidung trifft Julius.\n")
    return "\n".join(L)


def main() -> int:
    runs_dir = sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, "pipeline", "runs")
    reports = load_reports(runs_dir)
    res = evaluate(reports)
    md = to_markdown(res)
    out = os.path.join(ROOT, "reports", "gate-g2.md")
    os.makedirs(os.path.dirname(out), exist_ok=True)
    with open(out, "w", encoding="utf-8") as f:
        f.write(md)
    print(md)
    print(f"wrote {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
