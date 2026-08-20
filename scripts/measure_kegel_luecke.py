#!/usr/bin/env python3
"""Welche Felder ausserhalb des Pflicht-Kegels aendern die Steuer — und in welche Richtung?

DIE FRAGE. Der Kegel ist die Menge Felder, die bestaetigt sein muessen, bevor der Ring eine
feste Zahl herausgibt (api._feste_zahl, sonst Sperrgrund input_kegel_nicht_bestaetigt). Er ist
aber KLEINER als die Feldmenge der Scheibe: `gesamt` fuehrt 313 Felder und 33 im Kegel. Und
api.py:233 uebergibt an die Rechenfunktion NUR die Kegel-Felder:

    zahl = bf({f: felder[f]["wert"] for f in scheibe_felder})

Ein rechenwirksames Feld ausserhalb des Kegels erreicht die slot_fn also gar nicht. Dort steht
es dann auf dem Default — meist 0 (BACKLOG slot-fail-open-13568-eur). Die Zahl entsteht, als
haette der Nutzer "nichts" geantwortet, ohne dass er je gefragt wurde.

BEIDE RICHTUNGEN ZAEHLEN, und das ist der Grund, warum dieses Skript nicht nach under-tax
filtert:

  Steuer STEIGT, wenn man das Feld setzt   -> fehlend war UNDER-TAX. Die Erklaerung ist
                                              unvollstaendig zugunsten des Nutzers; Haftung
                                              liegt bei ihm (§ 378 AO auch fahrlaessig), und
                                              eDaten/Kontrollmitteilungen finden es oft.
  Steuer SINKT, wenn man das Feld setzt    -> fehlend war OVER-TAX. Der Nutzer zahlt zu viel,
                                              und "es reklamiert niemand" (so steht es im
                                              Repo-Kommentar). Das Finanzamt prueft, ob GENUG
                                              erklaert wurde, nicht ob zu viel — es korrigiert
                                              keinen vergessenen Abzug. Nach Bestandskraft ist
                                              das Geld endgueltig weg.
  Steuer BLEIBT GLEICH                     -> das Feld wirkt nicht. Entweder eine tote Bindung
                                              (Slot wird von keiner slot_fn gelesen) oder eine
                                              Regel, die im Basisfall nicht greift. Beides ist
                                              ein eigener Befund.

Under-tax ist das rechtlich groessere Risiko, over-tax das fuer den Nutzer teurere, weil
unsichtbar. Wer nur nach einer Richtung sucht, findet die Haelfte.

NUR MESSEN. Aendert keine Repo-Datei, schreibt keinen Store, kein Netz, kein LLM.

    python3 scripts/measure_kegel_luecke.py --scheibe gesamt
    python3 scripts/measure_kegel_luecke.py --scheibe gesamt --nur betriebseinnahmen
"""
from __future__ import annotations

import argparse
import glob
import json
import os
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
for sub in ("produkt/haut", "produkt/store", "produkt/traverser", "produkt/mapping",
            "produkt/import", "produkt/engine"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import yaml                             # noqa: E402
import api as API                       # noqa: E402
import store as ST                      # noqa: E402
import traverser as TR                  # noqa: E402


def _bindungsfelder() -> dict[str, dict]:
    """feld_id -> Bindungseintrag, ueber alle bindung_*.yaml."""
    aus = {}
    for p in glob.glob(os.path.join(ROOT, "produkt", "bindung", "bindung_*.yaml")):
        d = yaml.safe_load(open(p, encoding="utf-8")) or {}
        for b in (d.get("bindungen") or []):
            aus[b["feld_id"]] = b
    return aus


def _pseudo_slots() -> set[tuple[str, str]]:
    """(feld_id, slot) aus SIGNATUR_SLOT_ZEIGT_INS_LEERE — Felder OHNE Rechenwirkung.

    Ohne diese Ausnahme meldet der Sweep jedes Formalienfeld als Kandidaten: sie tragen einen
    signatur_slot, der bewusst auf keinen Catala-Input zeigt (Zieladresse, Beschaeftigungsort,
    Ausbildungsbezeichnung). Sie KOENNEN die Steuer nicht aendern — sie im Ergebnis zu fuehren
    hiesse, 20 Zeilen Rauschen ueber die echten Befunde zu legen.
    """
    pfad = os.path.join(ROOT, "tests", "test_bindungstabelle.py")
    quelle = open(pfad, encoding="utf-8").read()
    start = quelle.find("SIGNATUR_SLOT_ZEIGT_INS_LEERE = {")
    if start < 0:
        return set()
    ende = quelle.find("\n}", start)
    import ast
    import re
    roh = set()
    for m in re.finditer(r'\(\s*"([^"]+)"\s*,\s*"([^"]+)"\s*,\s*"([^"]+)"\s*,\s*"([^"]+)"\s*\)',
                         quelle[start:ende]):
        _, feld, regel, slot = m.groups()
        roh.add((feld, slot))
    del ast
    return roh


def _messbare_felder(scheibe: str) -> list[str]:
    """Felder der Scheibe, die rechenwirksam sein KOENNTEN und nicht im Kegel stehen."""
    cfg = API.SCHEIBEN[scheibe]
    felder = tuple(cfg.get("felder") or ())
    kegel = set(cfg.get("kegel") or felder)
    bind = _bindungsfelder()
    pseudo = _pseudo_slots()
    aus = []
    for f in felder:
        if f in kegel:
            continue
        b = bind.get(f)
        if not b or not b.get("askable"):
            continue
        q = b.get("quelle") or {}
        slot = q.get("signatur_slot")
        if not slot or (f, slot) in pseudo:
            continue
        if b.get("beispielwert") is None:
            continue          # ohne Probewert nicht messbar
        aus.append(f)
    return sorted(aus)


_HERKUNFT = {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}


def _setze(store: dict, feld_id: str, wert) -> None:
    """Ein bestaetigtes Laien-Event. Dieselbe Form wie die Test-Fixturen (_b dort)."""
    ST.append_event(store=store, feld_id=feld_id, wert=wert, zustand="bestaetigt",
                    herkunft=_HERKUNFT, schreiber="ui:laie",
                    signal={"signal_1": None, "signal_2": f"sweep@{feld_id}"},
                    ts="2026-01-01T00:00:00Z")


def _zahl(store: dict, scheibe: str, vz: int):
    """Die feste Zahl der Scheibe, oder None wenn der Kegel nicht steht."""
    bindung = API._scheibe_bindung(store)
    felder, _ = ST.materialisiere(store)
    felder = API._mit_ring_werten(felder, vz)
    cfg = API.SCHEIBEN[scheibe]
    kegel = tuple(cfg.get("kegel") or cfg.get("felder") or ())
    erg = API._feste_zahl(felder, bindung, cfg, vz, kegel, store)
    if erg is None:
        return None
    zahl, solz, extras = erg
    kist = (extras or {}).get("kist_cent") or 0
    return int(zahl) + int(solz or 0) + int(kist)


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--scheibe", default="gesamt", choices=sorted(API.SCHEIBEN))
    p.add_argument("--vz", type=int, default=2025)
    p.add_argument("--nur", nargs="*", default=None, help="nur diese feld_ids messen")
    p.add_argument("--json", help="Rohdaten hierhin schreiben")
    a = p.parse_args()

    bind_alle = _bindungsfelder()

    def _basisfall() -> dict:
        """Ein Store, dessen Kegel VOLLSTAENDIG bestaetigt ist — aus den Beispielwerten gebaut.

        Bewusst nicht aus einer Test-Fixtur: die Abgabe-Fixturen fuellen den Kegel der Scheibe
        `gesamt` nicht (dort stehen 33 Pflichtfelder), und eine fremde Fixtur zu erweitern
        hiesse, ihre Zahlen zu veraendern. Die beispielwerte der Bindung sind ohnehin die
        Probewerte, mit denen die Feldmatrix arbeitet.
        """
        s = ST.leerer_store(a.vz, fall_id="kegel_sweep")
        s["scheibe"] = a.scheibe
        cfg = API.SCHEIBEN[a.scheibe]
        for f in (cfg.get("kegel") or cfg.get("felder") or ()):
            b = bind_alle.get(f)
            if not b or b.get("beispielwert") is None:
                continue
            _setze(s, f, b["beispielwert"])
        return s

    basis_store = _basisfall()
    basis = _zahl(basis_store, a.scheibe, a.vz)
    if basis is None:
        cfg = API.SCHEIBEN[a.scheibe]
        kegel = tuple(cfg.get("kegel") or cfg.get("felder") or ())
        felder, _ = ST.materialisiere(basis_store)
        fehlt = [f for f in kegel if f not in felder]
        print(f"Basisfall liefert KEINE Zahl fuer Scheibe '{a.scheibe}' — der Kegel steht nicht.\n"
              f"Ohne Vergleichswert ist keine Differenz messbar.\n"
              f"Unbesetzt ({len(fehlt)} von {len(kegel)}): {fehlt[:12]}", file=sys.stderr)
        return 2
    print(f"Scheibe {a.scheibe}, VZ {a.vz}: Basis = {basis} ct\n")

    bind = _bindungsfelder()
    felder = a.nur if a.nur else _messbare_felder(a.scheibe)
    print(f"{len(felder)} Felder ausserhalb des Kegels mit moeglicher Rechenwirkung\n")

    ergebnisse = []
    for i, fid in enumerate(felder, 1):
        wert = bind[fid].get("beispielwert")
        s = _basisfall()
        try:
            _setze(s, fid, wert)
            neu = _zahl(s, a.scheibe, a.vz)
        except Exception as e:                      # noqa: BLE001 — Messung, kein Produktivpfad
            print(f"  {i:3}/{len(felder)} {fid:44} FEHLER {type(e).__name__}: {e}"[:150])
            ergebnisse.append({"feld": fid, "fehler": f"{type(e).__name__}: {e}"})
            continue
        if neu is None:
            print(f"  {i:3}/{len(felder)} {fid:44} keine Zahl (Kegel gesperrt)")
            ergebnisse.append({"feld": fid, "gesperrt": True})
            continue
        d = neu - basis
        richtung = "UNDER-TAX" if d > 0 else ("OVER-TAX" if d < 0 else "wirkungslos")
        print(f"  {i:3}/{len(felder)} {fid:44} {d:+12} ct  {richtung}")
        ergebnisse.append({"feld": fid, "delta_ct": d, "richtung": richtung,
                           "probewert": wert})

    wirkt = [e for e in ergebnisse if e.get("delta_ct")]
    under = [e for e in wirkt if e["delta_ct"] > 0]
    over = [e for e in wirkt if e["delta_ct"] < 0]
    tot = [e for e in ergebnisse if e.get("delta_ct") == 0]
    print(f"\n{'='*78}\n"
          f"UNDER-TAX (fehlend -> Steuer zu niedrig, Haftung beim Nutzer): {len(under)}\n"
          f"OVER-TAX  (fehlend -> Steuer zu hoch, es reklamiert niemand):  {len(over)}\n"
          f"wirkungslos (Feld aendert die Zahl nicht):                     {len(tot)}")
    if under:
        print(f"\ngroesste UNDER-TAX-Betraege:")
        for e in sorted(under, key=lambda x: -x["delta_ct"])[:8]:
            print(f"    {e['delta_ct']:+12} ct  {e['feld']}")
    if over:
        print(f"\ngroesste OVER-TAX-Betraege:")
        for e in sorted(over, key=lambda x: x["delta_ct"])[:8]:
            print(f"    {e['delta_ct']:+12} ct  {e['feld']}")

    if a.json:
        with open(a.json, "w", encoding="utf-8") as fh:
            json.dump({"scheibe": a.scheibe, "vz": a.vz, "basis_ct": basis,
                       "ergebnisse": ergebnisse}, fh, ensure_ascii=False, indent=2)
        print(f"\nRohdaten: {a.json}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
