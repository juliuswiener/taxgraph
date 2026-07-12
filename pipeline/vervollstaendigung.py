"""Vervollstaendigungs-Pakete: Kandidaten fuer fehlende Geltungsbedingungen.

Protokolldekret 2026-07-10, Stufe 3, Punkt 1: pro Regel ein Paket mit Vorschlaegen
fuer die Bedingungsliste, gegen die AKKUMULIERTE Item-Union aller vorhandenen
Verdikte. Julius segnet ab. Items, die objektiv ambig sind (haengen von einer
Signatur-Interpretation ab), werden NICHT in Bedingungen gezwungen, sondern als
grenzfall-Kandidat markiert -> eskaliert per Design.

Quelle der Union sind die gespeicherten Judge-Verdikte:
  * jeder ungedeckte `wirkt_hinein`-Norm-Teil (referenz + zitat) -> Bedingungs-
    Kandidat mit Zitatanker-Platzhalter;
  * jede undeklarierte Annahme (betrifft + kategorie) -> Input-Semantik-Kandidat.

Ausgabe: reports/review/<datum>-vervollstaendigung-<regel>.md je Regel.

    python pipeline/vervollstaendigung.py 2026-07-11 \
        pipeline/runs/produktion/*/report.json
"""

from __future__ import annotations

import glob
import json
import os
import sys
from collections import defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, ROOT)
sys.path.insert(0, HERE)

from yamlstrict import load_yaml   # noqa: E402


def _verdikte(pfade: list[str]) -> dict[str, list[dict]]:
    """rule_id -> Liste von Judge-Verdikten aus den Report-Dateien."""
    out: dict[str, list[dict]] = defaultdict(list)
    for p in pfade:
        try:
            r = json.load(open(p, encoding="utf-8"))
        except (json.JSONDecodeError, OSError):
            continue
        v = r.get("judge_verdict")
        # skipped-Verdikte sind kein Judge-Lauf (skip_judge) - nicht als Verdikt zaehlen,
        # sonst verfaelschen sie die Union-Zaehlung (tragen ohnehin null Items).
        if v and not v.get("parse_error") and not v.get("skipped"):
            out[r["candidate_id"]].append(v)
    return out


def _union(verdikte: list[dict]) -> dict:
    """Akkumulierte Union der Items ueber mehrere Verdikte, per Anker dedupliziert."""
    gaps: dict[str, dict] = {}      # referenz -> {zitat, wirkt_hinein_count, gedeckt, gesehen}
    ann: dict[tuple, dict] = {}     # (betrifft,kategorie) -> {aussage, undeclared_count, gesehen}
    for v in verdikte:
        for g in v.get("scope_gap", []):
            ref = str(g.get("referenz") or "?")
            e = gaps.setdefault(ref, {"zitat": g["norm_teil"][:200], "wirkt": 0,
                                      "gedeckt": 0, "gesehen": 0})
            e["gesehen"] += 1
            if g.get("klasse") == "wirkt_hinein":
                e["wirkt"] += 1
                if g.get("abgedeckt_von") and g["abgedeckt_von"] != "none":
                    e["gedeckt"] += 1
        for a in v.get("stille_zusatzannahmen", []):
            key = (str(a.get("betrifft")), str(a.get("kategorie")))
            e = ann.setdefault(key, {"aussage": a["annahme"][:200], "undeclared": 0,
                                     "gesehen": 0})
            e["gesehen"] += 1
            if not a.get("bedingung_id"):
                e["undeclared"] += 1
    return {"gaps": gaps, "annahmen": ann, "n": len(verdikte)}


def _paket(rid: str, rule: dict, u: dict, datum: str) -> str:
    n = u["n"]
    L = [f"# Vervollständigung {rid} — zur Absegnung ({datum})\n",
         f"Akkumulierte Item-Union aus {n} gespeicherten Verdikt(en). Vorschläge für",
         "fehlende Geltungsbedingungen; nichts davon ist im Manifest aktiv.\n",
         "**Vorbehalt:** Diese Union stammt aus den vorhandenen Verdikten. Für ein",
         "belastbares Paket sollte sie aus einem frischen Union-until-Saturation-Lauf",
         "je Regel erzeugt werden (die Inventarstufe streut). Zitatanker sind",
         "Platzhalter und gegen den eingefrorenen Normtext zu prüfen.\n"]

    bestehende = {b["bedingung"] for b in rule.get("geltungsbedingungen", [])}
    L.append(f"Bereits deklariert: {len(bestehende)} Bedingung(en).\n")

    # A) wirkt_hinein-Norm-Teile ohne Abdeckung -> Bedingungs-Kandidaten
    offen = [(ref, e) for ref, e in u["gaps"].items()
             if e["wirkt"] > 0 and e["gedeckt"] < e["wirkt"]]
    L.append("## A) Ungedeckte `wirkt_hinein`-Norm-Teile → Bedingungs-Kandidaten\n")
    if not offen:
        L.append("Keine.\n")
    for ref, e in sorted(offen, key=lambda x: -x[1]["wirkt"]):
        dauer = e["wirkt"] >= 2 and e["gesehen"] >= 2 and e["wirkt"] == e["gesehen"]
        L.append(f"- **{ref}** (wirkt_hinein in {e['wirkt']}/{e['gesehen']} Verdikten)")
        L.append(f"  - Zitat: {e['zitat']}")
        if dauer:
            L.append("  - **Grenzfall-Kandidat**: konsistent wirkt_hinein → als "
                     "Bedingung deklarieren ODER, wenn objektiv ambig, in die "
                     "Dauersplitter-Registry.")
        L.append("  - Vorschlag:")
        L.append("    ```yaml")
        L.append("    - bedingung: <name>")
        L.append(f'      deckt_ab: "<wörtliche Passage aus {ref}>"')
        L.append(f'      quelle: "{ref}"')
        L.append('      beschreibung: "<was die Bedingung an/ausschaltet>"')
        L.append("    ```")

    # B) undeklarierte Annahmen -> Input-Semantik-Kandidaten
    undekl = [(k, e) for k, e in u["annahmen"].items() if e["undeclared"] > 0]
    L.append("\n## B) Undeklarierte Annahmen → Input-Semantik-Kandidaten\n")
    if not undekl:
        L.append("Keine.\n")
    for (betrifft, kat), e in sorted(undekl, key=lambda x: -x[1]["undeclared"]):
        L.append(f"- **{betrifft}** / `{kat}` (undeklariert in "
                 f"{e['undeclared']}/{e['gesehen']} Verdikten)")
        L.append(f"  - Annahme: {e['aussage']}")
        L.append("  - Vorschlag:")
        L.append("    ```yaml")
        L.append(f"    - bedingung: {betrifft}_{kat}")
        L.append('      deckt_ab: "<wörtliche Passage, die diese Lesart festlegt>"')
        L.append('      quelle: "<§ ...>"')
        L.append(f'      beschreibung: "Input-Semantik: {betrifft} ({kat}) ..."')
        L.append("    ```")

    L.append("\n## Absegnung\n")
    L.append("Pro Kandidat: übernehmen (dann trage ich die Bedingung mit geprüftem")
    L.append("Zitatanker ein), ändern, oder als Grenzfall in die Registry statt in")
    L.append("eine Bedingung.")
    return "\n".join(L)


def main() -> int:
    if len(sys.argv) < 3:
        print(__doc__)
        return 2
    datum = sys.argv[1]
    pfade = [f for m in sys.argv[2:] for f in glob.glob(m)]
    cfg = load_yaml(os.path.join(HERE, "produktion", "rules.yaml"))
    regeln = {r["rule_id"]: r for r in cfg["regeln"]}
    verd = _verdikte(pfade)
    outdir = os.path.join(ROOT, "reports", "review")
    os.makedirs(outdir, exist_ok=True)
    geschrieben = []
    for rid, vs in sorted(verd.items()):
        rule = regeln.get(rid, {})
        md = _paket(rid, rule, _union(vs), datum)
        out = os.path.join(outdir, f"{datum}-vervollstaendigung-{rid}.md")
        with open(out, "w", encoding="utf-8") as f:
            f.write(md)
        geschrieben.append(out)
        print(f"  {rid}: {len(vs)} Verdikt(e) -> {os.path.basename(out)}")
    print(f"\n{len(geschrieben)} Paket(e) unter reports/review/")
    return 0


if __name__ == "__main__":
    sys.exit(main())
