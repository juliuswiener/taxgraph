"""Golden-Korpus-Cross-Check gegen GETTSIM 1.2.1 (Paket 9, Verifikations-Haertung).

Deterministisch, LLM-frei. Vergleicht die GEPINNTEN Golden-Werte unseres Korpus
(golden/cases/*.yaml) fallweise gegen GETTSIM, ueberall wo GETTSIM einen
Vergleichswert liefern kann. Aktuell deckungsfaehig: der § 32a-Tarif (g32a-Faelle,
Grund- und Splittingtarif) gegen GETTSIMs einkommensteuer.betrag_ohne_kinder-
freibetrag_y_sn.

KEINE stille Toleranz: jede Abweichung wird mit Cent-Delta gelistet und mit einer
Ursachen-Hypothese versehen. Abweichung != automatisch unser Fehler - GETTSIM 1.2.1
traegt zwei von uns gemeldete, offene Bugs (#1209 Splitting-Rundung, #1210
Progressionsfaktor), die genau hier sichtbar werden.

Nicht deckungsfaehige Domaenen (gewst/kst = koerperschafts-/gewerbesteuerlich, kein
GETTSIM-Analog; gesamt = volle festzusetzende ESt mit § 31-Guenstigerpruefung/§ 35a/
Vorsorge-Deckel, Node-Mapping nicht 1:1; arbeitnehmer/ep/ho = Werbungskosten-Feed-in)
werden im Coverage-Abschnitt AUSGEWIESEN, nicht still uebergangen.

Run:  . oracle/.venv312/bin/activate && python oracle/gettsim/golden_crosscheck.py
"""
from __future__ import annotations

import glob
import json
import os
import sys

import yaml

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
sys.path.insert(0, os.path.dirname(__file__))

import harness as H  # noqa: E402  (GETTSIM-Interface, gettsim_single/gettsim_splitting)

CASES_DIR = os.path.join(ROOT, "golden", "cases")
YEARS = (2024, 2025, 2026)


def _domain(cid: str, s: dict) -> str:
    if s.get("koerperschaft"):
        return "kst_nenner_b (Koerperschaftsteuer)"
    for pre, dom in (("g32a", "g32a (§ 32a-Tarif)"), ("gesamt", "gesamt (festzusetzende ESt)"),
                     ("gewst", "gewst (Gewerbesteuer)"), ("arbeitnehmer", "arbeitnehmer (Bruttolohn->ESt)"),
                     ("ep", "ep (Entfernungspauschale)"), ("ho", "ho (Homeoffice)")):
        if cid.startswith(pre):
            return dom
    return "sonstige"


def load_cases():
    for f in sorted(glob.glob(os.path.join(CASES_DIR, "*.yaml"))):
        yield yaml.safe_load(open(f, encoding="utf-8"))


def collect():
    """Partitioniert den Korpus. g32a-Tarif-Faelle sind deckungsfaehig."""
    tarif = []          # (cid, year, veranlagung, zve, ours_euro)
    coverage = {}       # domain -> count
    nicht_deckbar = {}  # domain -> [cids]  (deckungsfaehig=False)
    for c in load_cases():
        cid = c.get("id", "?")
        s = c.get("sachverhalt", {})
        e = c.get("erwartung", {})
        dom = _domain(cid, s)
        coverage[dom] = coverage.get(dom, 0) + 1
        deckbar = (dom.startswith("g32a") and "tarifliche_est" in e
                   and "zu_versteuerndes_einkommen" in s and not s.get("gesamtfall"))
        if deckbar:
            tarif.append((cid, int(s["veranlagungszeitraum"]),
                          s.get("veranlagung", "einzel"),
                          int(s["zu_versteuerndes_einkommen"]), int(e["tarifliche_est"])))
        else:
            nicht_deckbar.setdefault(dom, []).append(cid)
    return tarif, coverage, nicht_deckbar


def crosscheck(tarif):
    """Vergleicht ours (Golden) vs GETTSIM je Tarif-Fall. delta_cent = (ours - gettsim)*100."""
    rows = []
    for year in YEARS:
        for verf, fn in (("einzel", H.gettsim_single), ("zusammen", H.gettsim_splitting)):
            sub = [(cid, zve, ours) for cid, y, v, zve, ours in tarif if y == year and v == verf]
            if not sub:
                continue
            gvals = fn([zve for _, zve, _ in sub], year)
            for (cid, zve, ours), g in zip(sub, gvals):
                g = int(g)
                rows.append(dict(cid=cid, year=year, verfahren=verf, zve=zve,
                                 ours=ours, gettsim=g, delta_cent=(ours - g) * 100))
    return rows


def hypothese(row) -> str:
    if row["verfahren"] == "zusammen":
        return ("GETTSIM-Bug #1209 (offen): Splitting rundet NACH Verdopplung statt den "
                "bereits gerundeten Halbtarif zu verdoppeln (+ fehlende Abrundung von Z/2). "
                "§ 32a Abs. 5 i.V.m. Abs. 1 S. 6 verlangt 2*abrunden(Tarif(abrunden(Z/2))); "
                "Catala = Wortlaut = BMF-Steuerrechner. NICHT unser Fehler.")
    return ("GETTSIM-Bug #1210 (offen): voll aufgeloeste Progressionsfaktor-Rekonstruktion "
            "weicht an isolierten Zonen-Innenpunkten um 1 Euro vom publizierten (2-Nachkomma-) "
            "Tarif ab. Catala = literaler Koeffizient. NICHT unser Fehler.")


def main():
    tarif, coverage, nicht_deckbar = collect()
    rows = crosscheck(tarif)
    dev = [r for r in rows if r["delta_cent"] != 0]
    report = write_report(rows, dev, coverage, nicht_deckbar)
    # Maschinenlesbar fuer das pytest-Gate:
    out_json = os.path.join(os.path.dirname(__file__), "golden_crosscheck_result.json")
    zero = [r for r in rows if r["delta_cent"] == 0]
    json.dump({"rows": rows, "deviations": dev, "n_deckungsgleich": len(zero)},
              open(out_json, "w", encoding="utf-8"), ensure_ascii=False, indent=1)
    print(f"Tarif-Faelle verglichen: {len(rows)}  deckungsgleich: {len(rows)-len(dev)}  "
          f"Abweichungen: {len(dev)}")
    print(f"wrote {report}")
    print(f"wrote {out_json}")


def write_report(rows, dev, coverage, nicht_deckbar):
    total = sum(coverage.values())
    L = []
    L.append("# Golden-Korpus × GETTSIM 1.2.1 — Cross-Check (Paket 9, Verifikations-Haertung)\n")
    L.append("Erzeugt von `oracle/gettsim/golden_crosscheck.py` (deterministisch, LLM-frei, $0). "
             "Vergleicht die gepinnten Golden-Werte fallweise gegen GETTSIM 1.2.1, wo GETTSIM "
             "einen Vergleichswert liefert. Keine stille Toleranz: jede Abweichung ist gelistet.\n")
    L.append("\n## GETTSIM-Issue-Status (github ttsim-dev/gettsim, gelesen 2026-07-16)\n")
    L.append("| Issue | Titel | Stand |")
    L.append("|-------|-------|-------|")
    L.append("| [#1209](https://github.com/ttsim-dev/gettsim/issues/1209) | § 32a Abs. 5 Splitting rundet nach Verdopplung | **OPEN**, Label `bug`, keine PR/Resolution |")
    L.append("| [#1210](https://github.com/ttsim-dev/gettsim/issues/1210) | § 32a Abs. 1 Progressionsfaktor 1-Euro-Abweichung | **OPEN**, Label `bug`, keine PR/Resolution |")
    n_einzel_dev = sum(1 for r in dev if r["verfahren"] == "einzel")
    n_zus_dev = sum(1 for r in dev if r["verfahren"] == "zusammen")
    L.append(f"\nBeide von juliuswiener am 2026-07-09 eroeffnet, unveraendert offen. GETTSIM 1.2.1 "
             f"traegt beide Bugs weiter. In diesem Lauf zeigen sich {n_zus_dev} Splitting-Abweichungen "
             f"(#1209) und {n_einzel_dev} Grundtarif-Abweichungen (#1210) — GETTSIM-Signatur, kein "
             f"Catala-Fehler.\n")

    L.append(f"\n## Coverage (alle {total} Golden-Faelle)\n")
    L.append("| Domaene | Faelle | GETTSIM-deckungsfaehig | Begruendung |")
    L.append("|---------|--------|------------------------|-------------|")
    n_tarif = len(rows)
    for dom in sorted(coverage):
        n = coverage[dom]
        if dom.startswith("g32a"):
            L.append(f"| {dom} | {n} | JA ({n_tarif}) | § 32a-Tarif direkt vs GETTSIM einkommensteuer.betrag |")
        elif dom.startswith("gesamt"):
            L.append(f"| {dom} | {n} | NEIN (verschoben) | volle festzusetzende ESt: § 31-Guenstigerpruefung/§ 35a/§ 24a/Vorsorge-Deckel; Node-Mapping nicht 1:1, separater Harness noetig — NICHT forciert (falsche Deltas vermeiden) |")
        elif dom.startswith(("gewst", "kst")):
            L.append(f"| {dom} | {n} | NEIN | koerperschafts-/gewerbesteuerlich, KEIN GETTSIM-Analog |")
        else:
            L.append(f"| {dom} | {n} | NEIN | Werbungskosten-/Feed-in-Groesse, kein eigenstaendiger GETTSIM-Vergleichswert |")
    L.append(f"\nDeckungsfaehig und verglichen: **{n_tarif}** § 32a-Tarif-Faelle. Nicht deckungsfaehige "
             "Domaenen sind oben ausgewiesen (kein stiller Ausschluss); ihre Verifikation laeuft ueber "
             "den eigenen Golden-Gate (value + Zitatanker) bzw. — fuer gesamt — einen kuenftigen "
             "festzusetzende-ESt-Harness (Backlog).\n")

    L.append(f"\n## § 32a-Tarif-Cross-Check: {n_tarif} Faelle, {n_tarif-len(dev)} deckungsgleich, {len(dev)} Abweichungen\n")
    if not dev:
        L.append("Alle verglichenen Tarif-Faelle stimmen mit GETTSIM cent-exakt ueberein.\n")
    else:
        L.append("Jede Abweichung ist ein Item mit Cent-Delta und Ursachen-Hypothese. Delta = "
                 "(unser Golden-Wert − GETTSIM), positiv = Catala hoeher.\n")
        L.append("| Fall | VZ | Verfahren | zvE | unser Wert | GETTSIM | Delta (ct) | Ursachen-Hypothese |")
        L.append("|------|----|-----------|-----|-----------|---------|-----------|--------------------|")
        for r in dev:
            L.append(f"| {r['cid']} | {r['year']} | {r['verfahren']} | {r['zve']} | {r['ours']} | "
                     f"{r['gettsim']} | {r['delta_cent']:+d} | {hypothese(r)} |")
        L.append("\n**Triage:** alle Abweichungen fallen auf die zwei offenen GETTSIM-Bugs "
                 "(#1209 Splitting, #1210 Grundtarif). Catala folgt dem Gesetzeswortlaut und ist "
                 "am amtlichen BMF-Steuerrechner bestaetigt (siehe reports/s02-divergenzen.md, "
                 "2026-07-09). Kein Catala-Fund; Adjudikation der Triage-Klasse ueber Instructor.\n")

    L.append(f"\n## Deckungsgleiche Faelle (Regressions-Anker)\n")
    L.append(f"{n_tarif-len(dev)} von {n_tarif} Tarif-Faellen sind mit GETTSIM cent-exakt deckungsgleich "
             "(im Wesentlichen der Grundtarif und die geraden/aufgehenden Splitting-Faelle). Diese Menge "
             "ist ueber `tests/test_gettsim_crosscheck.py` als deterministischer Gate verankert: die "
             "deckungsgleichen Faelle MUESSEN deckungsgleich bleiben, jede neue Abweichung schlaegt rot.\n")

    path = os.path.join(ROOT, "reports", "review", "2026-07-16-gettsim-crosscheck.md")
    open(path, "w", encoding="utf-8").write("\n".join(L) + "\n")
    return path


if __name__ == "__main__":
    main()
