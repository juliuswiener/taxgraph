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

# Vorregistrierte Erfolgskriterien der Nachmessung (Protokolldekret 2026-07-10,
# Auflage 2). Sie standen VOR der Messung fest und werden nicht angepasst.
DECKUNG_MIN = 0.80              # >= 80 % der Items in allen 3 Laeufen gesehen
GELTUNGSBEREICH_SPLIT_MAX = 0.15
SPLIT_TRIGGER = 0.20           # geltungsbereich-Splitrate darueber -> Trigger


def entscheid(deckung: float, gb_split: float, repliziert: bool | None) -> tuple[str, str]:
    ok_deckung = deckung >= DECKUNG_MIN
    ok_split = gb_split <= GELTUNGSBEREICH_SPLIT_MAX
    ok_repl = repliziert is True
    if ok_deckung and ok_split and ok_repl:
        return ("protokoll_steht",
                "Alle drei Kriterien erfuellt. Das Protokoll steht, kein Zweit-Judge.")
    if gb_split > SPLIT_TRIGGER and ok_deckung:
        return ("zweit_judge",
                "Inventar-Deckung stabil, aber geltungsbereich-Splitrate ueber 20 %. "
                "Der Trigger ist sauber gemessen und wird gezogen: Zweit-Judge einer "
                "anderen Familie.")
    return ("spot_diagnose",
            "Weder alle Kriterien erfuellt noch der Trigger sauber ausgeloest. "
            "Spot-Diagnose, dann Entscheid bei Julius.")


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

    inv_items = sum(z.get("inventar_items", 0) for z in zus.values())
    inv_deckung_n = sum(z.get("inventar_in_allen_laeufen", 0) for z in zus.values())
    deckung = inv_deckung_n / inv_items if inv_items else 0.0
    gb_split = je_gate["geltungsbereich"][2] or 0.0

    repl = None
    rpfad = os.path.join(ROOT, "reports", "nachtschicht", "judge-replikation.json")
    if os.path.exists(rpfad):
        rd = json.load(open(rpfad, encoding="utf-8"))
        vals = [v.get("identisch") for v in rd.values() if "identisch" in v]
        repl = all(vals) if vals else None

    schluessel, satz = entscheid(deckung, gb_split, repl)
    kosten = sum(z["kosten_usd"] for z in zus.values())
    parse = sum(z["parse_fehler"] for z in zus.values())
    gesamt_laeufe = sum(z["laeufe"] for z in zus.values())
    merges = sum(z.get("merges", 0) for z in zus.values())
    streuung = sum(z.get("inventar_streuung_items", 0) for z in zus.values())

    L = ["# Judge-Stabilitaet nach der Dekomposition\n",
         "Messplan vorregistriert (Protokolldekret 2026-07-10). Die Kriterien standen "
         "vor der Messung fest und liegen in `pipeline/judge_stabilitaet_report.py`.\n",
         f"Ein Durchgang, {len(zus)} Regeln, je 3 Laeufe. Kosten {kosten:.4f} USD.\n",
         "## Vorregistrierte Kriterien\n",
         f"- **Inventar-Deckung: {deckung:.1%}** ({inv_deckung_n} von {inv_items} "
         f"Items in allen 3 Laeufen) -- Ziel >= {DECKUNG_MIN:.0%}",
         f"- **geltungsbereich-Splitrate: {gb_split:.1%}** -- Ziel <= {GELTUNGSBEREICH_SPLIT_MAX:.0%}",
         f"- **Spot-Replikation identisch: {repl}** -- Ziel: True\n",
         "## Kennzahlen\n",
         f"- Item-Splitrate gesamt: {rate:.1%} ({splits} Splits auf {items} beurteilte Items)",
         f"- Parse-Fehler: {parse} von {gesamt_laeufe} Laeufen"
         f" ({parse / gesamt_laeufe:.0%})" if gesamt_laeufe else "",
         f"- Inventar-Streuung: {streuung} Items nicht in allen drei Inventarlaeufen gesehen",
         f"- Merges des Aehnlichkeitsabgleichs: {merges}\n",
         "### Splitrate je blockierendem Gate\n",
         "| Gate | Splits | Items | Rate |", "|---|---|---|---|"]
    for g, (s, i, r) in je_gate.items():
        L.append(f"| `{g}` | {s} | {i} | {'-' if r is None else f'{r:.1%}'} |")

    L += ["\n### Je Regel\n",
          "| Regel | Items je Lauf | Splits | Rate | Gate-Urteile | stabil? |",
          "|---|---|---|---|---|---|"]
    instabile_gates = []
    for r, z in sorted(zus.items(), key=lambda kv: -(kv[1]["item_splitrate"] or 0)):
        sr = z["item_splitrate"]
        gut = [l for l in laeufe[r] if not l.get("parse_error")]
        spanne = [l["items_beurteilt"] for l in gut]
        stabil = z["gate_urteile_stabil"]
        if not stabil:
            instabile_gates.append(r)
        L.append(f"| `{r}` | {min(spanne) if spanne else '-'}-{max(spanne) if spanne else '-'} | "
                 f"{z['item_splits']} | {'-' if sr is None else f'{sr:.1%}'} | "
                 f"{', '.join(z['verschiedene_gate_urteile'])} | "
                 f"{'ja' if stabil else '**nein**'} |")

    L += ["\n### Was die Splitrate NICHT misst\n",
          "Die vorregistrierte Splitrate misst die Uneinigkeit der drei Stimmen ueber "
          "EIN Item. Sie sagt nichts darueber, ob in zwei Laeufen dieselben Items "
          "ueberhaupt gefunden wurden. Genau dort sitzt die verbliebene Streuung: das "
          "Inventar findet mal mehr, mal weniger Norm-Teile, und ein zusaetzlich "
          "gefundener `wirkt_hinein`-Teil kippt das `geltungsbereich`-Gate.\n",
          f"Regeln mit wechselnden Gate-Urteilen trotz stabiler Item-Urteile: "
          f"{', '.join(f'`{r}`' for r in instabile_gates) if instabile_gates else 'keine'}.",
          f"\nSpanne der beurteilten Items je Lauf steht in der Tabelle oben. Diese "
          f"Groesse war nicht vorregistriert; sie wird berichtet, weil sie das "
          f"Kriterium unterlaeuft, nicht weil sie es bestaetigt.\n"]

    L += [f"\n## Entscheid nach den vorregistrierten Kriterien\n",
          f"Inventar-Deckung {deckung:.1%} (Ziel >= {DECKUNG_MIN:.0%}), "
          f"geltungsbereich-Splitrate {gb_split:.1%} (Ziel <= {GELTUNGSBEREICH_SPLIT_MAX:.0%}), "
          f"Replikation identisch {repl} (Ziel True) -> **{schluessel}**\n", satz, ""]

    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, "w", encoding="utf-8") as f:
        f.write("\n".join(x for x in L if x is not None))
    print("\n".join(x for x in L if x is not None))
    print(f"\ngeschrieben: {OUT}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
