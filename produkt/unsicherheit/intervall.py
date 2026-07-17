"""Unsicherheits-Derivat — [min,max]-Bescheid-Intervall + Beitrag je Feld (Julius #6).

Rein deterministisch, NULL LLM. Liest einen Store-Snapshot (materialisierte Felder), rechnet über
eine INJIZIERTE reine Funktion `bescheid_fn(werte) -> steuer_cent` (in Produktion golger/runner.py
catala_*, im Test eine reine Modellfunktion — die Kombinatorik hier ist engine-agnostisch).

Zwei Stufen (KONZEPT.md):
  1. One-at-a-time: Beitrag je unsicherer Achse (Frage-Reihenfolge). Ranking-Heuristik — kann
     Interaktionen unterschätzen; das Intervall selbst bleibt ehrlich via gedeckelt-Flag.
  2. Gedeckelter Kartesischer über die Top-K-Treiber (Cap): exaktes Intervall bzgl. der stärksten
     Felder; gedeckelt-Flag + rest_felder, wenn K < n.

Ehrlichkeit (Falsch-Grün-Analog):
  - unbounded/nicht-Achse-Feld = offene Achse -> Intervall auf beiden Seiten offen markiert.
  - unbounded Feld OHNE Vorschlag = NICHT fixierbar -> kein numerischer Bescheid, beide Seiten offen,
    KEIN Ersatzwert erfunden.
"""
from __future__ import annotations

from itertools import product

CAP_DEFAULT = 256
NICHT_FIXIERBAR = "__nicht_fixierbar__"


def _extremwerte(b: dict):
    """Extremwerte je typ. None = keine numerische Achse (datum/text) oder unbounded (kein bereich)."""
    typ = b["typ"]
    if typ == "bool":
        return [False, True]
    if typ == "enum":
        return list(b.get("enum_werte") or [])
    if typ in ("cent", "int"):
        ber = b.get("bereich")
        return sorted({ber["min"], ber["max"]}) if ber else None
    return None  # datum/text


def _fixwert(feld_id: str, snapshot: dict, b: dict):
    """Fixierwert für die 'anderen' Felder: vorlaeufig-Vorschlag, sonst bereich-Mittelpunkt,
    sonst NICHT_FIXIERBAR (kein Ersatzwert erfinden)."""
    cur = snapshot.get(feld_id)
    if cur is not None:
        return cur["wert"]
    ber = b.get("bereich")
    if ber and b["typ"] in ("cent", "int"):
        return (ber["min"] + ber["max"]) // 2
    return NICHT_FIXIERBAR


def _unsicher(feld_id: str, snapshot: dict) -> bool:
    cur = snapshot.get(feld_id)
    return cur is None or cur.get("zustand") == "vorlaeufig"


def intervall(snapshot: dict, bindung: dict, bescheid_fn, *, cap: int = CAP_DEFAULT,
              snapshot_id: str | None = None) -> dict:
    """snapshot: {feld_id -> {wert, zustand, herkunft}}. bindung: {feld_id -> bindungs-Eintrag}.
    bescheid_fn: {feld_id -> wert} -> steuer_cent (rein, deterministisch)."""
    askable = {fid: b for fid, b in bindung.items() if b.get("askable")}

    fest, achse, offen, nicht_fix = {}, {}, [], []
    base: dict = {}
    for fid, b in askable.items():
        cur = snapshot.get(fid)
        if cur is not None and cur.get("zustand") == "bestaetigt":
            fest[fid] = cur["wert"]
            base[fid] = cur["wert"]
            continue
        # unsicher (offen oder vorlaeufig)
        ex = _extremwerte(b)
        fw = _fixwert(fid, snapshot, b)
        if fw is NICHT_FIXIERBAR:
            nicht_fix.append(fid)
            offen.append(fid)
            continue
        base[fid] = fw
        if ex is None:
            offen.append(fid)          # fixierbar (Vorschlag), aber unbounded -> offene Achse
        else:
            achse[fid] = ex

    iv = {"min_cent": None, "max_cent": None, "min_offen": False, "max_offen": False,
          "gedeckelt": False, "exakt_bzgl_top_k": 0, "rest_felder": [],
          "offene_achsen": sorted(offen), "nicht_fixierbar": sorted(nicht_fix)}
    res = {"basis_snapshot": snapshot_id, "intervall": iv, "beitraege": []}

    if nicht_fix:
        # kein numerischer Bescheid ohne erfundenen Ersatzwert
        iv["min_offen"] = iv["max_offen"] = True
        return res

    # -- Stufe 1: Beitrag je Achse (One-at-a-time) --
    for fid in sorted(achse):
        werte = []
        for v in achse[fid]:
            w = dict(base)
            w[fid] = v
            werte.append(bescheid_fn(w))
        lo, hi = min(werte), max(werte)
        res["beitraege"].append({"feld_id": fid, "spanne_cent": hi - lo, "min_cent": lo, "max_cent": hi})
    res["beitraege"].sort(key=lambda x: (-x["spanne_cent"], x["feld_id"]))

    # -- Stufe 2: gedeckelter Kartesischer über die Top-K-Treiber --
    ranked = [x["feld_id"] for x in res["beitraege"]]
    top, raum = [], 1
    for fid in ranked:
        n = len(achse[fid])
        if raum * n > cap:
            break
        top.append(fid)
        raum *= n

    if top:
        smin = smax = None
        for combo in product(*[achse[fid] for fid in top]):
            w = dict(base)
            for fid, v in zip(top, combo):
                w[fid] = v
            s = bescheid_fn(w)
            smin = s if smin is None else min(smin, s)
            smax = s if smax is None else max(smax, s)
        iv["min_cent"], iv["max_cent"] = smin, smax
    else:
        s = bescheid_fn(base)
        iv["min_cent"] = iv["max_cent"] = s

    iv["exakt_bzgl_top_k"] = len(top)
    rest = [fid for fid in ranked if fid not in top]
    iv["gedeckelt"] = bool(rest)
    iv["rest_felder"] = sorted(rest)
    if offen:                          # fixierbare offene Achsen -> wahre Spanne reicht darüber hinaus
        iv["min_offen"] = iv["max_offen"] = True
    return res


# ---------------------------------------------------------------- Engine-Adapter (Produktion)

def bescheid_via_slots(bindung: dict, slot_fn):
    """Baut ein bescheid_fn(feld_werte) aus einer slot-basierten Engine-Funktion
    (z.B. golden/runner.catala_*). Übersetzt feld_id -> signatur_slot über die Bindungstabelle und
    addiert Summanden-Slots (Summen-Konvention). Reine Funktion, kein Zustand."""
    def _fn(feld_werte: dict) -> int:
        slots: dict = {}
        for fid, wert in feld_werte.items():
            q = bindung[fid]["quelle"]
            slot = q.get("signatur_slot")
            if slot is None:
                continue                        # geltungsbedingung-Bindung -> kein numerischer Slot
            if bindung[fid].get("slot_beitrag") == "summand":
                slots[slot] = slots.get(slot, 0) + wert
            else:
                slots[slot] = wert
        return slot_fn(slots)
    return _fn
