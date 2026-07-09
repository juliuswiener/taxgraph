"""Formalisierungs-Backlog aus scope_gaps der Klasse `unabhaengig`.

Der Judge klassifiziert jeden Norm-Teil ausserhalb der Signatur (roundtrip_diff@3):

  (a) `unabhaengig`   -> der formalisierte Ausschnitt bleibt ohne ihn korrekt.
                         Er ist damit ein Kandidat fuer eine eigene Regel: dieses
                         Skript sammelt ihn als Backlog-Item ein.
  (b) `wirkt_hinein`  -> Eskalation im Gate, kein Backlog-Item. Ohne ihn ist der
                         Ausschnitt falsch, nicht bloss unvollstaendig.

Der Backlog ist damit ein Nebenprodukt der Formalisierung: jede Regel, die durch
die Kaskade laeuft, liefert den Zuschnitt fuer die naechsten mit.

    python pipeline/backlog.py pipeline/runs/produktion
"""

from __future__ import annotations

import glob
import json
import os
import sys
from collections import defaultdict

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def collect(runs_dir: str) -> dict[str, list[dict]]:
    """rule_id -> list of independent scope_gaps (deduplicated by norm_teil)."""
    by_rule: dict[str, dict[str, dict]] = defaultdict(dict)
    for p in sorted(glob.glob(os.path.join(runs_dir, "**", "report.json"),
                              recursive=True)):
        try:
            r = json.load(open(p, encoding="utf-8"))
        except json.JSONDecodeError:
            print(f"skip unreadable report: {p}", file=sys.stderr)
            continue
        for g in r.get("backlog") or []:
            key = " ".join(g.get("norm_teil", "").lower().split())
            if key:
                by_rule[r["candidate_id"]][key] = g
    return {rid: list(d.values()) for rid, d in sorted(by_rule.items())}


def to_markdown(by_rule: dict[str, list[dict]]) -> str:
    total = sum(len(v) for v in by_rule.values())
    L = ["# Formalisierungs-Backlog (aus scope_gaps)\n",
         "Automatisch erzeugt aus den Judge-Verdikten der Gate-Kaskade. Enthalten "
         "sind ausschliesslich Norm-Teile der Klasse `unabhaengig`: sie liegen "
         "ausserhalb der formalisierten Signatur und aendern deren Ergebnis nicht. "
         "Norm-Teile, die in den Signatur-Scope hineinwirken, stehen NICHT hier - "
         "sie eskalieren im `scope_gap`-Gate, weil der Ausschnitt ohne sie falsch "
         "waere.\n",
         f"{total} Item(s) aus {len(by_rule)} Regel(n).\n"]
    if not total:
        L.append("Kein Backlog. Entweder liefen noch keine Regeln durch die "
                 "Kaskade, oder der Judge fand keine unabhaengigen Norm-Teile.\n")
        return "\n".join(L)
    for rid, gaps in by_rule.items():
        L.append(f"\n## aus `{rid}`\n")
        for g in gaps:
            L.append(f"- **{g['norm_teil']}**")
            if g.get("begruendung"):
                L.append(f"  - {g['begruendung']}")
    return "\n".join(L)


def main() -> int:
    runs = sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, "pipeline", "runs")
    by_rule = collect(runs)
    out = os.path.join(ROOT, "reports", "formalisierungs-backlog.md")
    os.makedirs(os.path.dirname(out), exist_ok=True)
    with open(out, "w", encoding="utf-8") as f:
        f.write(to_markdown(by_rule))
    print(f"{sum(len(v) for v in by_rule.values())} Backlog-Item(s) -> {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
