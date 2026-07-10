"""Auswertung der Judge-Stabilitaetsmessung nach dem vorregistrierten Plan.

Die Entscheidungskriterien stehen hier im Code, weil sie VOR der Messung
festgelegt wurden (Protokolldekret 2026-07-10, Messplan Punkt 2). Sie werden nicht
nachtraeglich an das Ergebnis angepasst - genau dagegen schuetzt eine
Vorregistrierung.

    Item-Splitrate <= 10 %   -> Protokoll steht, kein Zweit-Judge, keine zweite Messung
    10 % < Rate <= 20 %      -> Spot-Replikation auf den zwei instabilsten Regeln
    Rate > 20 %              -> Zweit-Judge anderer Familie als naechste Stufe

    python pipeline/judge_stabilitaet_report.py reports/nachtschicht/judge-stabilitaet-dekomponiert.json
"""

from __future__ import annotations

import json
import os
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "reports", "judge-stabilitaet.md")

SCHWELLE_OK = 0.10
SCHWELLE_ZWEIT_JUDGE = 0.20


def entscheid(rate: float) -> tuple[str, str]:
    if rate <= SCHWELLE_OK:
        return ("protokoll_steht",
                "Das Protokoll steht. Kein Zweit-Judge, kein zweiter Messdurchgang.")
    if rate <= SCHWELLE_ZWEIT_JUDGE:
        return ("spot_replikation",
                "Spot-Replikation auf den zwei instabilsten Regeln, dann Entscheid.")
    return ("zweit_judge",
            "Zweit-Judge einer anderen Modellfamilie als naechste Stufe.")


def main() -> int:
    pfad = sys.argv[1] if len(sys.argv) > 1 else os.path.join(
        ROOT, "reports", "nachtschicht", "judge-stabilitaet-dekomponiert.json")
    d = json.load(open(pfad, encoding="utf-8"))
    zus, laeufe = d["zusammenfassung"], d["laeufe"]

    items = sum(z["items_beurteilt"] for z in zus.values())
    splits = sum(z["item_splits"] for z in zus.values())
    rate = splits / items if items else 0.0

    je_gate = {}
    for g in ("roundtrip", "geltungsbereich"):
        s = sum(sum(l["splits_je_gate"].get(g, 0) for l in laeufe[r] if not l.get("parse_error"))
                for r in laeufe)
        i = sum(sum(l["items_je_gate"].get(g, 0) for l in laeufe[r] if not l.get("parse_error"))
                for r in laeufe)
        je_gate[g] = (s, i, (s / i if i else None))

    schluessel, satz = entscheid(rate)
    kosten = sum(z["kosten_usd"] for z in zus.values())
    parse = sum(z["parse_fehler"] for z in zus.values())
    gesamt_laeufe = sum(z["laeufe"] for z in zus.values())
    merges = sum(z.get("merges", 0) for z in zus.values())
    streuung = sum(z.get("inventar_streuung_items", 0) for z in zus.values())

    L = ["# Judge-Stabilitaet nach der Dekomposition\n",
         "Messplan vorregistriert (Protokolldekret 2026-07-10). Die Kriterien standen "
         "vor der Messung fest und liegen in `pipeline/judge_stabilitaet_report.py`.\n",
         f"Ein Durchgang, {len(zus)} Regeln, je 3 Laeufe. Kosten {kosten:.4f} USD.\n",
         "## Kennzahlen\n",
         f"- **Item-Splitrate gesamt: {rate:.1%}** ({splits} Splits auf {items} beurteilte Items)",
         f"- Parse-Fehler: {parse} von {gesamt_laeufe} Laeufen"
         f" ({parse / gesamt_laeufe:.0%})" if gesamt_laeufe else "",
         f"- Inventar-Streuung: {streuung} Items nicht in allen drei Inventarlaeufen gesehen",
         f"- Merges des Aehnlichkeitsabgleichs: {merges}\n",
         "### Splitrate je blockierendem Gate\n",
         "| Gate | Splits | Items | Rate |", "|---|---|---|---|"]
    for g, (s, i, r) in je_gate.items():
        L.append(f"| `{g}` | {s} | {i} | {'-' if r is None else f'{r:.1%}'} |")

    L += ["\n### Je Regel\n",
          "| Regel | Items | Splits | Rate | Gate-Urteile | Parse-Fehler |",
          "|---|---|---|---|---|---|"]
    for r, z in sorted(zus.items(), key=lambda kv: -(kv[1]["item_splitrate"] or 0)):
        sr = z["item_splitrate"]
        L.append(f"| `{r}` | {z['items_beurteilt']} | {z['item_splits']} | "
                 f"{'-' if sr is None else f'{sr:.1%}'} | "
                 f"{', '.join(z['verschiedene_gate_urteile'])} | {z['parse_fehler']} |")

    L += [f"\n## Entscheid nach dem vorregistrierten Kriterium\n",
          f"Splitrate {rate:.1%} -> **{schluessel}**\n", satz, ""]

    if schluessel == "spot_replikation":
        instabil = sorted(zus.items(), key=lambda kv: -(kv[1]["item_splitrate"] or 0))[:2]
        L.append("Instabilste Regeln fuer die Spot-Replikation: "
                 + ", ".join(f"`{r}`" for r, _ in instabil))

    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, "w", encoding="utf-8") as f:
        f.write("\n".join(x for x in L if x is not None))
    print("\n".join(x for x in L if x is not None))
    print(f"\ngeschrieben: {OUT}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
