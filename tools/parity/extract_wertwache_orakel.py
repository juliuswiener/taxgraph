#!/usr/bin/env python3
"""Erzeugt rust/fixtures/wertwache_orakel.json: feste Grenzfaelle der engine-Accessoren samt den Antworten des
LAUFENDEN Python-Orakels (`tools/parity/oracle.py`, Aufruf `runner.catala_*` mit dem rohen Sachverhalt-dict, wie
`rust/parity/tests/zugriff_teil{1,2}_paritaet.rs` es tun). Der Rust-Test `rust/engine/tests/wertwache_orakel_werte.rs`
spielt dieselben dicts ueber dieselben Adapter gegen `engine::zugriff` und vergleicht -- hermetisch, ohne Python und
ohne `PARITY=1`.

Warum es das gibt: der Zufallsgenerator der Parity-Suiten zieht Grenzwerte (0, -1, genau die Schwelle) nur selten und
nie gemeinsam; ein Mutant an einer Schwelle ueberlebt dort mit 1000 Faellen je Funktion (Messung 2026-10-04,
`berichte/wertwache-engine.md`). Hier stehen die Grenzen fest im Fixture, je Stelle mit beiden Nachbarn.

  * Solz        §§ 3, 4 SolzG: Freigrenze je VZ (einzel/zusammen) und +-1/2/3, Schnittpunkt Regel/Milderung, Kapitalsteuer
  * Fuenftel    § 34 Abs. 1: verbleibendes zvE 0 / -1 / positiv, zvE 0 / -1 / positiv
  * P32b_1      Progressionsvorbehalt: zvE + Progressionseinkuenfte 0 / +-1 / negativ
  * P34c_1      Anrechnung auslaendischer Steuer: zvE und ausl 0 / +-1
  * KStG        Nenner B: Beteiligungsquote 9 / 10 / 11 (§ 8b Abs. 4)
  * Behinderten-Pauschbetrag: GdB 19 / 20 / 21 ... 100 / 101
  * § 33a       Unterhalt: Schonbetrag 623 / 624 / 625 und Grundfreibetrag je VZ
  * § 22        Renten (bb): Alter 96 / 97 / 98
  * § 3 Nr. 72  Photovoltaik: 30 kWp je Einheit, 100 kWp insgesamt, Leistung 0 / 1
  * § 101       Mobilitaetspraemie: Arbeitnehmer-Zweig und Unterschreitung des Grundfreibetrags

Alles deterministisch: feste Gitter, kein Zufall, sortierte Schluessel; zweimal erzeugt (auch unter PYTHONHASHSEED=1
und 777) ergibt dieselben Bytes. Der Aufrufer braucht `make build-python` (Catala-Paket fuer `oracle.py`).

Neu erzeugen:   python3 tools/parity/extract_wertwache_orakel.py
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
OUT = os.environ.get("WW_OUT_JSON", os.path.join(ROOT, "rust", "fixtures", "wertwache_orakel.json"))

VZS = (2024, 2025, 2026)
# Freigrenzen/Grundfreibetraege stehen nur als Messpunkte hier; die Antwort kommt immer vom Orakel.
SOLZ_FREIGRENZE = {2024: (18130, 36260), 2025: (19950, 39900), 2026: (20350, 40700)}
GFB_33A = {2024: 11784, 2025: 12096, 2026: 12348}

FAELLE: list[tuple[str, dict]] = []


def fall(fn: str, d: dict) -> None:
    FAELLE.append((fn, d))


def solz() -> None:
    for vz in VZS:
        for splitting in (False, True):
            f = SOLZ_FREIGRENZE[vz][1 if splitting else 0]
            basis = {"veranlagungszeitraum": vz, "splitting": splitting}
            punkte = [f + d for d in (-1, 0, 1, 2, 3, 10, 100)]
            schnitt = f * 119 // 64  # 5,5 % * b == 11,9 % * (b - f)
            punkte += [schnitt + d for d in range(-3, 4)]
            punkte += [2 * f, 3 * f, 100_000, 250_000, 1_000_003]
            for b in punkte:
                fall("catala_solz", {**basis, "bemessungsgrundlage": b, "kapital_steuer": 0})
            for b in (f + 1, 2 * f, 100_000):
                for kap in (0, 1, 2, 3, 5, 7, 10, 11, 19, 20, 100, 999, 12345):
                    fall("catala_solz", {**basis, "bemessungsgrundlage": b, "kapital_steuer": kap})
            fall("catala_solz", {**basis, "bemessungsgrundlage": 1000, "kapital_steuer": 5000})
            fall("catala_solz", {**basis, "bemessungsgrundlage": f + 5})  # kapital_steuer fehlt = 0


def fuenftel() -> None:
    paare = [(0, 0), (-1, -1), (-5, -5), (1, 1), (1000, 1000), (0, 1), (0, -1), (-1, 0), (1, 2), (1, 5), (5, 1),
             (3, 5), (5, 5), (6, 5), (100, 101), (100, 99), (10_000, 9_999), (10_000, 10_000), (10_000, 10_001),
             (30_000, 10_000), (60_000, 20_000), (200_000, 50_000), (200_000, 200_000), (200_000, 200_001)]
    for vz in VZS:
        for ver in ("einzel", "zusammen"):
            for zve, ao in paare:
                fall("catala_fuenftel", {"veranlagungszeitraum": vz, "veranlagung": ver,
                                         "zu_versteuerndes_einkommen": zve, "ausserordentliche_einkuenfte": ao})


def p32b_1() -> None:
    for zve in (-7, -1, 0, 1, 5, 30_000):
        for prog in (-30_001, -7, -5, -1, 0, 1, 7, 10_000):
            for est in (0, 5, 7209):
                fall("catala_p32b_1", {"zu_versteuerndes_einkommen": zve, "progressionseinkuenfte": prog,
                                       "est_auf_erhoehte_bemessung": est})


def p34c_1() -> None:
    for zve in (-1, 0, 1, 2, 60_000):
        for ausl in (-1, 0, 1, 2, 30_000, 60_000, 70_000):
            for est in (0, 5, 10_000):
                for gezahlt in (1, 5_000):
                    fall("catala_p34c_1", {"gezahlte_auslaendische_steuer": gezahlt, "deutsche_est_inkl_ausl": est,
                                           "zu_versteuerndes_einkommen": zve, "auslaendische_einkuenfte_staat": ausl})


def kst() -> None:
    for quote in (0, 1, 9, 10, 11, 100):
        for div in (0, 1000, 123_456):
            for gewinn in (0, 100_000):
                fall("catala_kst_nenner_b", {"gewst_hebesatz": 400, "beteiligung_prozent": quote,
                                             "dividende_bezuege": div, "gewinn_estg": gewinn,
                                             "veraeusserungsgewinn": 5000})
    fall("catala_kst_nenner_b", {"gewst_hebesatz": 400, "dividende_bezuege": 1000, "gewinn_estg": 100_000})


def behinderten() -> None:
    for vz in VZS:
        for gdb in (-1, 0, 10, 19, 20, 21, 29, 30, 39, 40, 50, 59, 60, 70, 80, 90, 99, 100, 101, 110, 120):
            fall("catala_behinderten_pb", {"veranlagungszeitraum": vz, "grad_der_behinderung": gdb})
        for gdb in (0, 19, 20, 50, 100):
            fall("catala_behinderten_pb", {"veranlagungszeitraum": vz, "grad_der_behinderung": gdb,
                                           "ist_hilflos_blind_taubblind": True})


def unterhalt() -> None:
    for vz in VZS:
        gfb = GFB_33A[vz]
        for aufw in (100, 12_000, 20_000, 1_000_000):
            for kv in (0, 700):
                for andere in (0, 623, 624, 625, 626, 1000, 5000):
                    fall("catala_p33a_unterhalt", {"veranlagungszeitraum": vz, "aufwendungen": aufw,
                                                   "kv_pv_beitraege": kv, "andere_einkuenfte_bezuege": andere})
        for aufw in (gfb - 1, gfb, gfb + 1):
            fall("catala_p33a_unterhalt", {"veranlagungszeitraum": vz, "aufwendungen": aufw})


def renten() -> None:
    for vz in VZS:
        for art in ("private_leibrente", "sonstige_leibrente"):
            for alter in (0, 1, 30, 65, 66, 94, 95, 96, 97, 98, 99, 100, 150):
                for rente in (12_000, 100_000):
                    fall("catala_renten_einkuenfte", {"veranlagungszeitraum": vz, "renten_art": art,
                                                      "alter_bei_rentenbeginn": alter, "jahresrente": rente})


def photovoltaik() -> None:
    g = {"wert": True}
    for leistung in (-1, 0, 1, 29, 30, 31, 59, 60, 61, 99, 100, 101, 120, 121, 150):
        for einheiten in (-1, 0, 1, 2, 3, 4, 5):
            fall("catala_p3_nr72_photovoltaik", {"pv_einnahmen": 5000, "pv_auf_gebaeude": g,
                                                 "pv_bruttoleistung_kwp": leistung, "pv_anzahl_einheiten": einheiten})
    for einnahmen in (-1, 0, 1):
        fall("catala_p3_nr72_photovoltaik", {"pv_einnahmen": einnahmen, "pv_auf_gebaeude": g,
                                             "pv_bruttoleistung_kwp": 10, "pv_anzahl_einheiten": 1})
    fall("catala_p3_nr72_photovoltaik", {"pv_einnahmen": 5000, "pv_auf_gebaeude": {"wert": False},
                                         "pv_bruttoleistung_kwp": 10, "pv_anzahl_einheiten": 1})
    fall("catala_p3_nr72_photovoltaik", {"pv_einnahmen": 5000, "pv_bruttoleistung_kwp": 10, "pv_anzahl_einheiten": 1})


def mobilitaet() -> None:
    for fn in ("catala_p101_mobilitaetspraemie", "catala_p101_mobilitaetspraemie_cent"):
        for ep in (0, 100, 1000):
            for zve in (0, 5000, 12_000, 20_000):
                for an in (False, True):
                    for wk in (0, 1000, 1230, 1500, 2230, 5000):
                        fall(fn, {"entfernungspauschale_ab_21km": ep, "zu_versteuerndes_einkommen": zve,
                                  "grundfreibetrag": 12_096, "ist_arbeitnehmer": an,
                                  "werbungskosten_gesamt": wk, "arbeitnehmer_pauschbetrag": 1230})


class Orakel:
    """`tools/parity/oracle.py` als Unterprozess (JSON-Zeilen), wie `parity::Oracle::spawn`."""

    def __init__(self) -> None:
        env = dict(os.environ)
        # Das Orakel importiert Produktmodule; die Akten-Ablage darf nie die echte sein.
        env.setdefault("TAXGRAPH_DATEN", tempfile.mkdtemp(prefix="wertwache-daten-"))
        self.p = subprocess.Popen(["python3", "tools/parity/oracle.py"], cwd=ROOT, env=env, text=True,
                                  stdin=subprocess.PIPE, stdout=subprocess.PIPE)

    def runner(self, name: str, args: list) -> dict:
        self.p.stdin.write(json.dumps({"fn": f"runner.{name}", "args": args}) + "\n")
        self.p.stdin.flush()
        zeile = self.p.stdout.readline()
        if not zeile:
            raise SystemExit("oracle.py hat die Verbindung geschlossen")
        return json.loads(zeile)

    def ende(self) -> None:
        self.p.stdin.close()
        self.p.wait(timeout=30)


def main() -> None:
    for f in (solz, fuenftel, p32b_1, p34c_1, kst, behinderten, unterhalt, renten, photovoltaik, mobilitaet):
        f()
    gesehen: set[str] = set()
    einmalig = []
    for fn, d in FAELLE:  # Gitter ueberlappen (z. B. kapital_steuer 0): jeder Fall einmal, erste Stelle gilt
        k = json.dumps([fn, d], sort_keys=True)
        if k not in gesehen:
            gesehen.add(k)
            einmalig.append((fn, d))
    o = Orakel()
    faelle = []
    for fn, d in einmalig:
        antwort = o.runner(fn, [d])
        faelle.append({"fn": fn, "args": [d], "py": antwort})
    o.ende()
    zaehl: dict[str, list[int]] = {}
    for f in faelle:
        z = zaehl.setdefault(f["fn"], [0, 0])
        z[0] += 1
        z[1] += "err" in f["py"]
    doc = {"erzeugt_von": "tools/parity/extract_wertwache_orakel.py", "orakel": "tools/parity/oracle.py -> produkt/engine/runner.py",
           "faelle": faelle}
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, "w", encoding="utf-8") as f:
        json.dump(doc, f, ensure_ascii=False, separators=(",", ":"), sort_keys=True)
        f.write("\n")
    print(f"{OUT}: {len(faelle)} Faelle, {os.path.getsize(OUT)} Bytes")
    for fn, (n, e) in sorted(zaehl.items()):
        print(f"  {fn:40s} {n:5d} Faelle, davon {e:4d} Fehlerantworten")


if __name__ == "__main__":
    main()
