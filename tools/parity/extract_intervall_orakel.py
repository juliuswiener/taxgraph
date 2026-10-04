#!/usr/bin/env python3
"""Erzeugt rust/fixtures/intervall_orakel.json: Szenarien samt Antworten des PYTHON-Orakels
(`produkt/unsicherheit/intervall.py`, ueber `tools/parity/oracle_konsistenz.py`). Der Rust-Test
`rust/intervall/tests/orakel_werte.rs` spielt dieselben Szenarien gegen die Crate `intervall` und
vergleicht -- hermetisch, ohne Python zur Laufzeit.

Alle drei Orakel-Eintraege rechnen ueber dieselbe SYNTHETISCHE Engine (`oracle_konsistenz._synth`):
Summe ueber `gewicht(name) * zahl(wert)`, `gewicht = (Summe der UTF-8-Bytes) mod 7 - 3`, `zahl` = int,
bool 0/1, String-Laenge in Codepoints, sonst 0. Die Rust-Seite baut dieselbe Funktion im Test nach.

  * `sicht`    Pythons Rohlesung aller echten Bindungen (`intervall.sicht`) in Reihenfolge der Rust-Registry
               (Dateien alphabetisch, in der Datei wie geschrieben) -- Gegenprobe zu `AchsenBindung::from`.
  * `iv`       `intervall(...)`: synthetische Bindung (Python-Form), Snapshot `{feld: [wert, "b"|"v"]}`, Deckel
               (`"default"` = `CAP_DEFAULT`), `snapshot_id`; Antwort `{"ok": ..}` oder `{"err": ..}`.
  * `slots`    `bescheid_via_slots(...)`: Bindung, Quantitaet, Feldwerte `[[feld, wert], ..]` in Einfuegereihenfolge;
               Antwort Cent oder `{"err": ..}`.

Neu erzeugen:   python3 tools/parity/extract_intervall_orakel.py
"""
from __future__ import annotations

import glob
import json
import os
import random
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
if os.environ.get("KO_ROOT"):
    ROOT = os.environ["KO_ROOT"]
sys.path.insert(0, os.path.join(ROOT, "tools", "parity"))
sys.path.insert(0, ROOT)

import yaml  # noqa: E402

import oracle_konsistenz as O  # noqa: E402

OUT = os.environ.get("IVL_OUT_JSON", os.path.join(ROOT, "rust", "fixtures", "intervall_orakel.json"))
SEED = 20261004
CAP_DEFAULT = 256


def gew(name: str) -> int:
    return sum(name.encode()) % 7 - 3


def namen_mit_gewicht(w: int, anzahl: int, praefix: str = "") -> list[str]:
    """Kurze Feldnamen mit dem Synth-Gewicht `w` (deterministisch)."""
    out, i = [], 0
    while len(out) < anzahl:
        n = f"{praefix}{chr(97 + i % 26)}{'' if i < 26 else i // 26}"
        if gew(n) == w:
            out.append(n)
        i += 1
    return out


def ax(fid, typ, askable=True, enum_werte=None, bereich=None, slot="s2", summand=False):
    """Eintrag in Python-Form (`TR.lade_bindung()`): das liest `intervall.py`."""
    e = {"feld_id": fid, "typ": typ, "askable": askable,
         "quelle": {"regel_id": "r", "signatur_slot": slot} if slot else {"regel_id": "r", "geltungsbedingung": "g"}}
    if enum_werte is not None:
        e["enum_werte"] = enum_werte
    if bereich is not None:
        e["bereich"] = {"min": bereich[0], "max": bereich[1]}
    if summand:
        e["slot_beitrag"] = "summand"
    return e


def b(w):
    return [w, "b"]


def v(w):
    return [w, "v"]


# ------------------------------------------------------------------------------------ Sicht
def voll_sortiert() -> list[str]:
    reihe = []
    for f in sorted(glob.glob(os.path.join(ROOT, "produkt", "bindung", "bindung_*.yaml"))):
        d = yaml.safe_load(open(f, encoding="utf-8")) or {}
        reihe += [x["feld_id"] for x in d.get("bindungen", [])]
    return reihe


def sicht():
    ids = voll_sortiert()
    antwort = O.antwort({"fn": "intervall.sicht", "bindung_ids": ids})
    return antwort["ok"]


# ------------------------------------------------------------------------------------ intervall()
IV: list[dict] = []
SL: list[dict] = []


def vorschlag(e):
    """Ein Vorschlag, der die Achse fixierbar macht (`_fixwert`: nur Bereich-Mittelpunkt geht ohne)."""
    return {"bool": False, "enum": (e.get("enum_werte") or ["x"])[0], "text": "t", "datum": "2025-01-01"}.get(e["typ"], 5)


def iv(name, bindung, snap=None, cap="default", sid=None, roh=False):
    """`roh=False`: jede askable Achse ohne Mittelpunkt und ohne Snapshot-Eintrag bekommt einen
    vorlaeufigen Vorschlag -- sonst ist das ganze Szenario `nicht fixierbar` und rechnet nichts."""
    snap = dict(snap or {})
    if not roh:
        for e in bindung:
            hat_mitte = e["typ"] in ("int", "cent") and "bereich" in e
            if e["askable"] and e["feld_id"] not in snap and not hat_mitte:
                snap[e["feld_id"]] = v(vorschlag(e))
    IV.append({"n": name, "bindung": bindung, "snap": snap, "cap": cap, "sid": sid})


def slots(name, bindung, q, werte):
    SL.append({"n": name, "bindung": bindung, "q": q, "werte": werte})


def iv_gezielt():
    iv("leer", [])
    iv("leer_sid", [], sid="sid-1")
    iv("bool_unsicher", [ax("a", "bool")], roh=True)
    iv("bool_vorlaeufig_true", [ax("a", "bool")], {"a": v(True)})
    iv("bool_bestaetigt", [ax("a", "bool")], {"a": b(True)})
    iv("bool_nicht_askable", [ax("a", "bool", askable=False)])
    iv("bool_nicht_askable_bestaetigt", [ax("a", "bool", askable=False)], {"a": b(True)})
    iv("bool_geltungsbedingung", [ax("a", "bool", slot=None)])
    # Deckel: k Bool-Achsen mit Gewicht != 0, Deckel an 2^k.
    pos = namen_mit_gewicht(3, 10) + namen_mit_gewicht(-3, 10)
    for k in (1, 2, 3, 7, 8, 9, 10):
        bind = [ax(pos[i], "bool") for i in range(k)]
        for cap in dict.fromkeys(("default", 0, 1, 2, 3, 4, 2 ** k - 1, 2 ** k, 2 ** k + 1, 255, 257)):
            if cap == "default" and k not in (7, 8, 9, 10):
                continue
            iv(f"bool{k}_cap{cap}", bind, cap=cap)
    # Bruch statt Abbruch: grosse Achse zuerst (4 Werte, Gewicht 3), kleine danach.
    gross = ax("a", "enum", enum_werte=["x", "yy", "zzz", "wwww"])
    klein = ax(namen_mit_gewicht(3, 1)[0] if namen_mit_gewicht(3, 1)[0] != "a" else "h", "bool")
    iv("bruch_gross_zuerst", [gross, klein], cap=3)
    iv("bruch_gross_zuerst_cap4", [gross, klein], cap=4)
    iv("bruch_gross_zuerst_cap8", [gross, klein], cap=8)
    iv("bruch_gross_zuerst_cap7", [gross, klein], cap=7)
    # Gleichstand der Spanne: feld_id entscheidet; Deckel laesst nur eine zu.
    zw = namen_mit_gewicht(3, 3)
    iv("gleichstand_cap2", [ax(zw[1], "bool"), ax(zw[0], "bool"), ax(zw[2], "bool")], cap=2)
    iv("gleichstand_cap4", [ax(zw[1], "bool"), ax(zw[0], "bool"), ax(zw[2], "bool")], cap=4)
    iv("gleichstand_cap1", [ax(zw[1], "bool"), ax(zw[0], "bool"), ax(zw[2], "bool")], cap=1)
    # Spanne 0 (Gewicht 0): Rangfolge nur ueber feld_id.
    nul = namen_mit_gewicht(0, 3)
    iv("spanne_null_cap2", [ax(n, "bool") for n in reversed(nul)], cap=2)
    # Reihenfolge der Rest-Felder (sortiert), Beitraege (absteigend).
    mix = [ax("m1", "enum", enum_werte=["a", "bb", "ccc"]), ax("m2", "bool"), ax("m3", "int", bereich=(0, 10)),
           ax("m4", "cent", bereich=(-5, 5)), ax("m5", "enum", enum_werte=["q", "rr"])]
    for cap in (0, 1, 2, 3, 5, 6, 11, 12, 13, 22, 24, 100, 300):
        iv(f"mix_cap{cap}", mix, cap=cap)
    iv("mix_default", mix)
    # Mittelpunkt: lo+hi floor/2, Achse Y mit grossem Gewicht bestimmt die Spanne; X steht nicht in Top.
    for lo, hi in ((-5, 0), (-3, 0), (-1, 0), (0, 1), (-7, -2), (3, 3), (0, 0), (-100, 101), (2 ** 40 + 1, 2 ** 40 + 4),
                   (7, 2), (-9, -9)):
        for typ in ("int", "cent"):
            x = namen_mit_gewicht(1, 1, "x")[0]
            iv(f"mitte_{typ}_{lo}_{hi}", [ax("y", "enum", enum_werte=["a", "bbbbbbbbbb"]), ax(x, typ, bereich=(lo, hi))], cap=2)
    # Vorschlag statt Mittelpunkt, auch ohne Bereich.
    x = namen_mit_gewicht(1, 1, "x")[0]
    iv("vorschlag_ueberschreibt_mitte", [ax("y", "enum", enum_werte=["a", "bbbbbbbbbb"]), ax(x, "int", bereich=(0, 10))],
       {x: v(9)}, cap=2)
    iv("vorschlag_ohne_bereich_offen", [ax("y", "enum", enum_werte=["a", "bbbbbbbbbb"]), ax(x, "int")], {x: v(9)}, cap=2)
    iv("vorschlag_text_offen", [ax("y", "bool"), ax("t", "text")], {"t": v("abc")})
    iv("vorschlag_datum_offen", [ax("y", "bool"), ax("d", "datum")], {"d": v("2025-01-01")})
    iv("vorschlag_null", [ax("y", "bool"), ax("t", "text")], {"t": v(None)})
    # Nicht fixierbar.
    iv("nicht_fixierbar_text", [ax("t", "text")], sid="s", roh=True)
    iv("nicht_fixierbar_int_ohne_bereich", [ax("i", "int"), ax("y", "bool")], roh=True)
    iv("nicht_fixierbar_mehrere", [ax("z", "datum"), ax("a", "text"), ax("y", "bool")], roh=True)
    iv("nicht_fixierbar_cent_ohne_bereich", [ax("c", "cent")], roh=True)
    iv("enum_ohne_werte_nicht_fixierbar", [ax("e", "enum", enum_werte=[])], roh=True)
    # Leere Enum-Achse (vorlaeufiger Wert, keine Werte) -> ValueError.
    iv("enum_leer_vorschlag", [ax("e", "enum", enum_werte=[])], {"e": v("x")})
    iv("enum_ohne_schluessel_vorschlag", [ax("e", "enum")], {"e": v("x")})
    iv("enum_ein_wert", [ax("e", "enum", enum_werte=["x"]), ax("y", "bool")])
    # Bestaetigte Werte gehen in die Basis (Gewicht).
    iv("basis_bestaetigt_int", [ax("a", "int", bereich=(0, 5)), ax("y", "bool")], {"a": b(4)})
    iv("basis_bestaetigt_text", [ax("a", "text"), ax("y", "bool")], {"a": b("abcd")})
    iv("basis_bestaetigt_bool_enum", [ax("a", "bool"), ax("e", "enum", enum_werte=["q", "rr"]), ax("y", "bool")],
       {"a": b(True), "e": b("rr")})
    iv("snapshot_fremdes_feld", [ax("y", "bool")], {"unbekannt": b(5)})
    iv("vorlaeufig_ausserhalb_bereich", [ax("a", "int", bereich=(0, 5)), ax("y", "bool")], {"a": v(100)})
    iv("bereich_min_gleich_max", [ax("a", "int", bereich=(4, 4)), ax("y", "bool")])
    iv("bereich_umgekehrt", [ax("a", "int", bereich=(9, 2)), ax("y", "bool")])
    iv("bereich_negativ", [ax("a", "cent", bereich=(-9, -2)), ax("y", "bool")])
    # Beitraege: Spanne, min, max je Achse; mehrere Achsen mit Gewicht < 0.
    neg = namen_mit_gewicht(-3, 2)
    iv("gewicht_negativ", [ax(neg[0], "int", bereich=(0, 7)), ax(neg[1], "bool"), ax("y", "enum", enum_werte=["a", "bb"])])
    # Deckel an einer einzelnen grossen Achse: 256 passt, 257 nicht (CAP_DEFAULT = 256).
    for n_werte in (255, 256, 257):
        iv(f"enum{n_werte}_default", [ax("y", "enum", enum_werte=[f"w{i:03d}" for i in range(n_werte)])])
    iv("snapshot_id_gesetzt", [ax("y", "bool")], sid="abc")
    iv("sid_leer", [ax("y", "bool")], sid="")
    # Zustand: genau "bestaetigt" fixiert; "vorlaeufig" nicht.
    iv("zustand_vorlaeufig_achse_bleibt", [ax("a", "int", bereich=(0, 5)), ax("y", "bool")], {"a": v(2)})


def zufall_ax(rng, i):
    typ = rng.choices(["bool", "enum", "int", "cent", "text", "datum"], [25, 20, 20, 20, 8, 7])[0]
    ew = rng.sample(["a", "bb", "ccc", "dddd"], rng.randint(0, 3)) if typ == "enum" or rng.random() < 0.1 else None
    ber = (rng.randint(-60, 60), rng.randint(-60, 60)) if rng.random() < 0.85 else None
    if ber and rng.random() < 0.3:
        ber = (min(ber), max(ber))
    fid = f"f{i}{'z' * rng.randint(0, 3)}"
    slot = rng.choice(["s2", "s3", "t1", "t2", None])
    return ax(fid, typ, askable=rng.random() < 0.85, enum_werte=ew, bereich=ber if typ in ("int", "cent") or rng.random() < 0.2 else None,
              slot=slot, summand=slot in ("t1", "t2"))


def zufall_wert(rng):
    r = rng.random()
    if r < 0.1:
        return None
    if r < 0.3:
        return rng.random() < 0.5
    if r < 0.7:
        return rng.randint(-60, 60)
    if r < 0.85:
        return rng.randint(-10 ** 12, 10 ** 12)
    return rng.choice(["", "a", "bbb", "zusammen"])


def iv_zufall(n=140):
    rng = random.Random(SEED)
    for k in range(n):
        anz = rng.randint(0, 8)
        bind = [zufall_ax(rng, i) for i in range(anz)]
        snap = {}
        for e in bind:
            fix_noetig = not (e["typ"] in ("int", "cent") and "bereich" in e)
            if rng.random() < (0.85 if fix_noetig else 0.4):
                snap[e["feld_id"]] = [zufall_wert(rng), "b" if rng.random() < 0.4 else "v"]
        iv(f"zufall_{k:03d}", bind, snap, cap=rng.randint(0, 300) if rng.random() < 0.9 else "default",
           sid=rng.choice([None, "sid"]), roh=True)


# ------------------------------------------------------------------------------------ bescheid_via_slots
def sl_gezielt():
    summ = lambda f, s="t1": ax(f, "cent", slot=s, summand=True)
    exakt = lambda f, s="s2", typ="int": ax(f, typ, slot=s)
    for q in ("festzusetzende_est", "gewst_cent"):
        slots(f"{q}_leer", [], q, [])
        slots(f"{q}_summanden", [summ("an"), summ("ag"), summ("ak")], q, [["an", 100], ["ag", 40], ["ak", -7]])
        slots(f"{q}_summand_bool", [summ("an"), summ("ag")], q, [["an", True], ["ag", True]])
        slots(f"{q}_exakt_spaeter_gewinnt", [exakt("p"), exakt("q")], q, [["p", 5], ["q", 9]])
        slots(f"{q}_exakt_spaeter_gewinnt_umgekehrt", [exakt("p"), exakt("q")], q, [["q", 9], ["p", 5]])
        slots(f"{q}_exakt_wiederholt", [exakt("p"), exakt("q")], q, [["p", 5], ["q", 9], ["p", 11]])
        slots(f"{q}_geltungsbedingung_uebersprungen", [exakt("p"), ax("g", "bool", slot=None)], q, [["p", 5], ["g", True]])
        slots(f"{q}_unbekanntes_feld", [exakt("p")], q, [["p", 5], ["fremd", 1]])
        slots(f"{q}_unbekanntes_feld_zuerst", [exakt("p")], q, [["fremd", 1], ["p", 5]])
        slots(f"{q}_summand_text", [summ("an")], q, [["an", "abc"]])
        slots(f"{q}_summand_null", [summ("an")], q, [["an", None]])
        slots(f"{q}_summand_text_zweiter", [summ("an"), summ("ag")], q, [["an", 5], ["ag", "x"]])
        slots(f"{q}_exakt_text_null", [exakt("p", typ="text"), exakt("q")], q, [["p", "abcd"], ["q", None]])
        slots(f"{q}_summand_und_exakt_gemischt", [summ("an"), summ("ag", "t2"), exakt("p"), exakt("q", "s3")], q,
              [["an", 3], ["ag", 4], ["p", 5], ["q", 6], ["an", 10]])
        slots(f"{q}_summand_gross", [summ("an"), summ("ag")], q, [["an", 10 ** 12], ["ag", -10 ** 11]])
        slots(f"{q}_summand_negativ_summe", [summ("an"), summ("ag")], q, [["an", -50], ["ag", -70]])
        slots(f"{q}_summand_dreimal", [summ("an")], q, [["an", 1], ["an", 2]])
        slots(f"{q}_zwei_summand_slots", [summ("an", "t1"), summ("ag", "t2")], q, [["an", 1], ["ag", 2]])


def sl_zufall(n=70):
    rng = random.Random(SEED + 1)
    for k in range(n):
        anz = rng.randint(1, 6)
        bind = []
        for i in range(anz):
            slot = rng.choice([("s2", False), ("s3", False), ("t1", True), ("t2", True), None])
            bind.append(ax(f"f{i}{'z' * rng.randint(0, 2)}", rng.choice(["int", "cent", "bool", "text"]),
                           slot=slot[0] if slot else None, summand=bool(slot and slot[1])))
        werte = []
        for _ in range(rng.randint(0, 8)):
            e = rng.choice(bind) if rng.random() > 0.12 else {"feld_id": "unbekannt"}
            summ = e.get("slot_beitrag") == "summand"
            w = rng.choice([None, True, False, 0, 1, 7, -3, 100, 10 ** 12, "ab", ""]) if not summ or rng.random() < 0.15 \
                else rng.choice([True, False, 0, 1, 7, -3, 100, 10 ** 12])
            werte.append([e["feld_id"], w])
        slots(f"zufall_{k:03d}", bind, rng.choice(["festzusetzende_est", "gewst_cent"]), werte)


# ------------------------------------------------------------------------------------ Antworten
def python_snapshot(snap: dict) -> dict:
    return {k: {"wert": w, "zustand": "bestaetigt" if z == "b" else "vorlaeufig"} for k, (w, z) in snap.items()}


def antwort_iv(s: dict) -> dict:
    cap = CAP_DEFAULT if s["cap"] == "default" else s["cap"]
    req = {"fn": "intervall.intervall", "snapshot": python_snapshot(s["snap"]), "bindung": s["bindung"], "cap": cap}
    if s["sid"] is not None:
        req["snapshot_id"] = s["sid"]
    return O.antwort(req)


def antwort_sl(s: dict) -> dict:
    # Python-`dict` mit Einfuegereihenfolge; ein wiederholter Schluessel behaelt seine erste Position.
    werte: dict = {}
    for fid, w in s["werte"]:
        werte[fid] = w
    return O.antwort({"fn": "intervall.bescheid_via_slots", "bindung": s["bindung"], "quantitaet": s["q"],
                      "feld_werte": list(werte.items())})


def main():
    iv_gezielt()
    iv_zufall()
    sl_gezielt()
    sl_zufall()
    for liste in (IV, SL):
        namen = [s["n"] for s in liste]
        assert len(namen) == len(set(namen)), "doppelte Szenarionamen"
    for s in IV:
        s["erg"] = antwort_iv(s)
    for s in SL:
        s["erg"] = antwort_sl(s)
    doc = {
        "erzeugt_von": "tools/parity/extract_intervall_orakel.py",
        "orakel": "produkt/unsicherheit/intervall.py (Engine: oracle_konsistenz._synth)",
        "sicht": sicht(),
        "iv": IV,
        "slots": SL,
    }
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, "w", encoding="utf-8") as f:
        json.dump(doc, f, ensure_ascii=False, separators=(",", ":"))
        f.write("\n")
    fehler = sum(1 for s in IV if "err" in s["erg"])
    zahl = sum(1 for s in IV if "ok" in s["erg"] and s["erg"]["ok"]["intervall"]["min_cent"] is not None)
    sfehler = sum(1 for s in SL if "err" in s["erg"])
    print(f"{OUT}: {len(doc['sicht'])} Sichten, {len(IV)} iv ({zahl} mit Zahl, {fehler} Fehler), "
          f"{len(SL)} slots ({sfehler} Fehler), {os.path.getsize(OUT)} Bytes")


if __name__ == "__main__":
    main()
