"""Auswertung der Judge-Stabilitaetsmessung nach dem vorregistrierten Plan.

Die Entscheidungskriterien stehen hier im Code, weil sie VOR der Messung
festgelegt wurden (Protokolldekret 2026-07-10, Messplan Punkt 2). Sie werden nicht
nachtraeglich an das Ergebnis angepasst - genau dagegen schuetzt eine
Vorregistrierung.

    a) alle Regeln identisches Gate-Ergebnis ueber die Laeufe
    b) geltungsbereich-Splitrate <= 15 %
    c) undeclared-Annahmen je Regel schwanken um <= 1
    alle drei -> Protokoll steht; b > 20 % -> Zweit-Judge; sonst Spot-Diagnose

    python pipeline/judge_stabilitaet_report.py reports/nachtschicht/judge-stabilitaet-dekomponiert.json
"""

from __future__ import annotations

import json
import os
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "reports", "judge-stabilitaet.md")

# Vorregistrierte Erfolgskriterien nach der Vervollstaendigung (Protokolldekret
# 2026-07-10, Stufe 3, Punkt 3). Sie stehen VOR der Messung fest.
#   a) Gate-Ergebnis-Replikation: alle Regeln identisches Gesamtverdikt ueber die
#      3 Laeufe (das ist die Groesse, die zaehlt).
#   b) geltungsbereich-Splitrate <= 15 %.
#   c) Anzahl undeclared-Annahmen je Regel schwankt um <= 1 zwischen Laeufen.
GELTUNGSBEREICH_SPLIT_MAX = 0.15
SPLIT_TRIGGER = 0.20
UNDECLARED_SCHWANKUNG_MAX = 1


def entscheid(a_alle_stabil: bool, gb_split: float, c_ok: bool) -> tuple[str, str]:
    ok_b = gb_split <= GELTUNGSBEREICH_SPLIT_MAX
    if a_alle_stabil and ok_b and c_ok:
        return ("protokoll_steht",
                "Alle drei Kriterien erfuellt. Das Protokoll steht, kein Zweit-Judge.")
    if gb_split > SPLIT_TRIGGER:
        return ("zweit_judge",
                "geltungsbereich-Splitrate ueber 20 %. Der Zweit-Judge-Trigger gilt "
                "als sauber gemessen und wird gezogen.")
    if ok_b and not a_alle_stabil:
        return ("spot_diagnose",
                "geltungsbereich stabil (<= 15 %), aber nicht alle Regeln replizieren "
                "ihr Gesamtverdikt. Die Restinstabilitaet liegt im Einzelfall - "
                "Spot-Diagnose, dann Entscheid bei Julius.")
    return ("spot_diagnose",
            "Kriterien nicht erfuellt, Trigger nicht sauber ausgeloest. "
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

    gb_split = je_gate["geltungsbereich"][2] or 0.0

    # a) Alle Regeln stabil (identisches Gate-Ergebnis ueber die 3 Laeufe)
    a_alle_stabil = all(z["gate_urteile_stabil"] for z in zus.values())
    instabile = [r for r, z in zus.items() if not z["gate_urteile_stabil"]]
    # c) undeclared-Annahmen schwanken je Regel um <= 1
    c_je_regel = {}
    for r, ls in laeufe.items():
        werte = [l["unmapped"] for l in ls if not l.get("parse_error")]
        c_je_regel[r] = (max(werte) - min(werte)) if werte else 0
    c_ok = all(v <= UNDECLARED_SCHWANKUNG_MAX for v in c_je_regel.values())
    # Saettigung (Punkt 2): Anteil Verdikte, deren Inventar konvergiert ist
    ges = [l.get("gesaettigt") for r in laeufe for l in laeufe[r] if not l.get("parse_error")]
    ges_anteil = (sum(1 for x in ges if x) / len(ges)) if ges else None

    schluessel, satz = entscheid(a_alle_stabil, gb_split, c_ok)
    kosten = sum(z["kosten_usd"] for z in zus.values())
    parse = sum(z["parse_fehler"] for z in zus.values())
    gesamt_laeufe = sum(z["laeufe"] for z in zus.values())
    merges = sum(z.get("merges", 0) for z in zus.values())
    streuung = sum(z.get("inventar_streuung_items", 0) for z in zus.values())

    L = ["# Judge-Stabilitaet nach der Dekomposition\n",
         "Messplan vorregistriert (Protokolldekret 2026-07-10). Die Kriterien standen "
         "vor der Messung fest und liegen in `pipeline/judge_stabilitaet_report.py`.\n",
         f"Ein Durchgang, {len(zus)} Regeln, je 3 Laeufe. Kosten {kosten:.4f} USD.\n",
         "## Vorregistrierte Kriterien (Stufe 3)\n",
         f"- **a) Gate-Ergebnis-Replikation: {'ERFUELLT' if a_alle_stabil else 'VERFEHLT'}** "
         f"-- alle Regeln identisches Gesamtverdikt ueber die 3 Laeufe"
         + (f" (instabil: {', '.join('`'+r+'`' for r in instabile)})" if instabile else ""),
         f"- **b) geltungsbereich-Splitrate: {gb_split:.1%}** -- Ziel <= {GELTUNGSBEREICH_SPLIT_MAX:.0%}",
         f"- **c) undeclared-Annahmen-Schwankung je Regel: max {max(c_je_regel.values()) if c_je_regel else 0}** "
         f"-- Ziel <= {UNDECLARED_SCHWANKUNG_MAX}"
         + (" (ERFUELLT)" if c_ok else " (VERFEHLT)") + "\n",
         "## Kennzahlen\n",
         f"- Item-Splitrate gesamt: {rate:.1%} ({splits} Splits auf {items} beurteilte Items)",
         f"- Inventar gesaettigt: {ges_anteil:.0%} der Verdikte konvergiert (Union-until-Saturation)"
         if ges_anteil is not None else "- Inventar-Saettigung: keine Daten",
         f"- Parse-Fehler: {parse} von {gesamt_laeufe} Laeufen"
         f" ({parse / gesamt_laeufe:.0%})" if gesamt_laeufe else "",
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
          f"a) Gate-Replikation {'erfuellt' if a_alle_stabil else 'verfehlt'}, "
          f"b) geltungsbereich-Split {gb_split:.1%} (Ziel <= {GELTUNGSBEREICH_SPLIT_MAX:.0%}), "
          f"c) undeclared-Schwankung max {max(c_je_regel.values()) if c_je_regel else 0} "
          f"(Ziel <= {UNDECLARED_SCHWANKUNG_MAX}) -> **{schluessel}**\n", satz, ""]

    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, "w", encoding="utf-8") as f:
        f.write("\n".join(x for x in L if x is not None))
    print("\n".join(x for x in L if x is not None))
    print(f"\ngeschrieben: {OUT}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
