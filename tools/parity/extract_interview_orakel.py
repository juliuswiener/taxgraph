#!/usr/bin/env python3
"""Erzeugt rust/fixtures/interview_orakel.json: Szenarien samt Antworten des PYTHON-Orakels
(`produkt/traverser/traverser.py`, `produkt/haut/bindung_rollen.py`). Der Rust-Test
`rust/interview/tests/orakel_werte.rs` spielt dieselben Szenarien gegen die Crate `interview` und
vergleicht -- hermetisch, ohne Python zur Laufzeit.

Szenario = Events (Reihenfolge ist Semantik) + Sicht:
  * `null`                      die volle Bindung in Reihenfolge der Rust-Registry,
  * `["feld", ...]`             eine Teil-Bindung wie `api._scheibe_bindung`,
  * `{"synth": [spec, ...]}`    selbstgebaute Felder (Vorlage: eine echte Bindung), damit Faelle
                                erreichbar sind, die die echte Bindung nicht enthaelt (Ring, Gleichstand).
Antworten: `queue` (Indizes in die Sicht-Reihenfolge), `queue_b` (mit Beitrag), `relevanz`,
`instanz_anzahl`, `fehlende`, `gate_gewicht`, ggf. `justification`/`trace`.

Neu erzeugen:   python3 tools/parity/extract_interview_orakel.py
"""
from __future__ import annotations

import copy
import glob
import json
import os
import random
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
if os.environ.get("IV_ROOT"):
    ROOT = os.environ["IV_ROOT"]
sys.path.insert(0, ROOT)

import yaml  # noqa: E402

from produkt.traverser import traverser as T  # noqa: E402

OUT = os.path.join(ROOT, "rust", "fixtures", "interview_orakel.json")
if os.environ.get("IV_OUT_JSON"):
    OUT = os.environ["IV_OUT_JSON"]
VORLAGE = "veranlagung"          # Feld der echten Bindung, aus dem `synth`-Felder geklont werden
SEED = 20261004

BIND = T.lade_bindung()


def voll_sortiert() -> dict:
    """`lade_bindung()` in der Reihenfolge der Rust-Registry: Dateien alphabetisch, in der Datei wie
    geschrieben (so macht es auch `tools/parity/oracle.py:_voll_sortiert`)."""
    reihe = []
    for f in sorted(glob.glob(os.path.join(ROOT, "produkt", "bindung", "bindung_*.yaml"))):
        d = yaml.safe_load(open(f, encoding="utf-8")) or {}
        reihe += [b["feld_id"] for b in d.get("bindungen", [])]
    return {f: BIND[f] for f in reihe}


VOLL = voll_sortiert()
assert set(VOLL) == set(BIND)

H_LAIE = {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}
H_VORJAHR = {"herkunft": "vorjahr", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}


# ------------------------------------------------------------------ Szenario -> Python-Store
def py_store(events: list) -> dict:
    evs = []
    for i, e in enumerate(events):
        feld, wert, zu = e[0], e[1], e[2]
        her = e[3] if len(e) > 3 else "laie"
        evs.append({"event_id": f"ev{i:04d}", "feld_id": feld, "wert": wert,
                    "zustand": "bestaetigt" if zu == "b" else "vorlaeufig",
                    "herkunft": H_VORJAHR if her == "vorjahr" else H_LAIE,
                    "ersetzt": None,
                    "signal": {"signal_1": None, "signal_2": f"ok@{feld}"} if len(e) > 4 and e[4] else None})
    return {"version": 1, "veranlagungszeitraum": 2025, "events": evs, "snapshots": []}


def py_felder(store: dict) -> dict:
    """Snapshot `feld -> {wert, zustand}` (letztes Event je Feld; kein Szenario ersetzt Events)."""
    return {e["feld_id"]: {"wert": e["wert"], "zustand": e["zustand"]} for e in store["events"]}


# ------------------------------------------------------------------ Sicht
def synth_dict(spec: dict) -> dict:
    """Spec -> Binding-Dict wie `traverser.py` es liest."""
    punkt = spec.get("punkt", ["slot", "s_" + spec["id"]])
    q = {"regel_id": spec["regel"]}
    q["signatur_slot" if punkt[0] == "slot" else "geltungsbedingung"] = punkt[1]
    b = {"feld_id": spec["id"], "quelle": q, "askable": spec.get("askable", True),
         "eingangsfrage": spec.get("eingang", False), "typ": spec.get("typ", "bool")}
    if spec.get("gate") is not None:
        b["gate"] = spec["gate"]
    if spec.get("kz"):
        b["elster_kz"] = spec["kz"]
    if spec.get("ableitung"):
        b["ableitung"] = {"aus": spec["ableitung"]["aus"]}
        if spec["ableitung"].get("und"):
            b["ableitung"]["und_feld"] = spec["ableitung"]["und"]
    if spec.get("vorjahr"):
        b["vorjahr"] = spec["vorjahr"]
    return b


def sicht_dict(sicht) -> dict:
    if sicht is None:
        return VOLL
    if isinstance(sicht, dict):
        return {s["id"]: synth_dict(s) for s in sicht["synth"]}
    return {f: BIND[f] for f in sicht}


def beitrag_b(felder: list) -> dict:
    """Deterministischer Unsicherheits-Beitrag nach Sicht-Position: `(n % 7) * 1000`; jede vierte Position
    (n % 4 == 3) fehlt in der Map -- fehlend und 0 sind gleich, das prueft die Rust-Seite mit."""
    return {f: (n % 7) * 1000 for n, f in enumerate(felder) if n % 4 != 3}


def relevanz_kompakt(rel: dict) -> dict:
    return {r: [v["status"], v["gates_offen"], v["annahmen_offen"]] for r, v in sorted(rel.items())}


SICHTEN: dict = {}


def sicht_eintrag(sicht) -> dict:
    """Je Sicht EINMAL: Felder, Gate-Gewicht (haengt nicht vom Store ab) und Relevanz des leeren
    Stores (Szenarien nennen nur die Abweichung davon)."""
    schluessel = json.dumps(sicht, sort_keys=True)
    if schluessel not in SICHTEN:
        sd = sicht_dict(sicht)
        SICHTEN[schluessel] = {
            "id": "voll" if sicht is None else f"sicht{len(SICHTEN)}",
            "felder": None if sicht is None else sicht,
            "gate_gewicht": {f: n for f, n in sorted(T.gate_gewicht(sd).items())},
            "relevanz_leer": relevanz_kompakt(T.relevanz(py_store([]), sd)),
        }
    return SICHTEN[schluessel]


def szenario(name: str, events: list, sicht=None, beitraege=True, justif=None, leerer_beitrag=False) -> dict:
    sd = sicht_dict(sicht)
    felder = list(sd)
    idx = {f: i for i, f in enumerate(felder)}
    store = py_store(events)
    s = {"name": name, "sicht": sicht_eintrag(sicht)["id"], "events": events}
    s["queue"] = [idx[f] for f in T.naechste_fragen(store, sd)]
    if beitraege:
        # Der Beitrag steht NICHT im Fixture: `(n % 7) * 1000` je Sicht-Position, der Rust-Test rechnet gleich.
        s["queue_b"] = [idx[f] for f in T.naechste_fragen(store, sd, beitrag_b(felder))]
    if leerer_beitrag:                     # `if beitrag:` -- eine leere Map gilt wie keine
        s["queue_e"] = [idx[f] for f in T.naechste_fragen(store, sd, {})]
    rel = relevanz_kompakt(T.relevanz(store, sd))
    basis = sicht_eintrag(sicht)["relevanz_leer"]
    s["relevanz_abweichung"] = {r: v for r, v in rel.items() if basis[r] != v}
    erste = {}
    for f in felder:                       # eine Auskunft je Gruppe genuegt: sie haengt nur an der Gruppe
        if sd[f].get("instanz_gruppe"):
            erste.setdefault(sd[f]["instanz_gruppe"], f)
    s["instanz_anzahl"] = {f: list(T.instanz_anzahl(store, sd, f)) for f in erste.values()}
    s["fehlende"] = T.fehlende_instanzen(py_felder(store), sd)
    if justif:
        s["justification"] = {}
        for f in justif:
            j = T.justification(store, f, sd)
            if j is not None:
                j = {k: v for k, v in j.items() if k != "event_id"}
            s["justification"][f] = j
        t = T.trace_ergebnis(store, sd, "snap")
        s["trace"] = {"basis_snapshot": t["basis_snapshot"],
                      "regeln": {r: [x["feld_id"] for x in js] for r, js in sorted(t["regeln"].items())}}
    return s


# ------------------------------------------------------------------ Hilfen fuer die Auswahl
GRUPPEN = T.lade_instanz_gruppen()
ZAEHLFELDER = {g["anzahl_feld"]: gn for gn, g in GRUPPEN.items()}
REGELB = T.lade_regel_bedingungen()


def gates_der_regel(rid: str) -> list:
    return [f for f, b in VOLL.items() if b["quelle"]["regel_id"] == rid and "geltungsbedingung" in b["quelle"]
            and b.get("askable") and b.get("gate", True)]


def wert_fuer(rng: random.Random, b: dict, f: str):
    if f in ZAEHLFELDER:
        return rng.choice([0, 1, 2, 2, 3, 4, True, 2.0, "2"])
    t = b.get("typ")
    if t == "bool":
        return rng.choice([True, False, False, True, False, 0, 1])
    if t in ("cent", "int"):
        return rng.choice([0, 0, 1, 2, 500, 12000])
    if t == "enum":
        return rng.choice(b.get("enum_werte") or ["x"])
    if t == "datum":
        return rng.choice(["2015-03-01", "2020-07-15"])
    return rng.choice(["abc", "zusammen", "keine"])


def zufall(rng: random.Random, n: int) -> list:
    """n Events auf Feldern, die etwas entscheiden: Gates, Regel-/Feld-Bedingungen, Zaehlfelder,
    Gruppenfelder (auch `__n`), Vorjahres-Felder -- plus beliebige Felder."""
    askable = [f for f, b in VOLL.items() if b.get("askable")]
    bedingt = sorted({c["feld"] for cs in REGELB.values() for c in cs}
                     | {b["feld_bedingung"]["feld"] for b in VOLL.values() if b.get("feld_bedingung")})
    gates = [f for f in askable if "geltungsbedingung" in VOLL[f]["quelle"]]
    gruppe = [f for f in askable if VOLL[f].get("instanz_gruppe") and f not in ZAEHLFELDER]
    vorj = [f for f in askable if VOLL[f].get("vorjahr") == "uebernehmbar"]
    zaehl = sorted(ZAEHLFELDER)
    out = []
    for _ in range(n):
        art = rng.random()
        if art < 0.22:
            f = rng.choice(bedingt)
        elif art < 0.42:
            f = rng.choice(gates)
        elif art < 0.52:
            f = rng.choice(zaehl)
        elif art < 0.66:
            f = rng.choice(gruppe)
            if rng.random() < 0.6:
                f = f"{f}__{rng.choice([2, 2, 3])}"
        elif art < 0.74:
            f = rng.choice(vorj)
        else:
            f = rng.choice(askable)
        basis = f.split("__")[0]
        zu = "b" if rng.random() < 0.78 else "v"
        her = "vorjahr" if (VOLL.get(basis, {}).get("vorjahr") and rng.random() < 0.5) else "laie"
        out.append([f, wert_fuer(rng, VOLL[basis], basis), zu, her])
    return out


def b_(f, w):
    return [f, w, "b"]


def v_(f, w):
    return [f, w, "v"]


# ------------------------------------------------------------------ gezielte Szenarien (echte Bindung)
def gezielt() -> list:
    sz = []
    sz.append(szenario("leer", []))
    sz.append(szenario("veranlagung_einzel", [b_("veranlagung", "einzel")]))
    sz.append(szenario("veranlagung_zusammen", [b_("veranlagung", "zusammen")]))
    sz.append(szenario("veranlagung_vorlaeufig", [v_("veranlagung", "zusammen")]))
    sz.append(szenario("kein_vuv_false", [b_("kein_vuv", False)]))
    sz.append(szenario("kein_vuv_null_statt_false", [b_("kein_vuv", 0), b_("kein_kap", 0)]))
    sz.append(szenario("kein_gewinn_false_feldbedingung", [b_("kein_gewinn", False)]))
    sz.append(szenario("kein_gewinn_true", [b_("kein_gewinn", True)]))
    sz.append(szenario("konfession_keine", [b_("kist_konfession", "keine")]))
    sz.append(szenario("konfession_ev", [b_("kist_konfession", "ev")]))
    sz.append(szenario("partner_lohn_null", [b_("bruttoarbeitslohn_partner", 0)]))
    sz.append(szenario("partner_lohn_5000", [b_("bruttoarbeitslohn_partner", 500000)]))
    # mehrere Gates einer Regel: das zweite verneint -> nur die offenen davor bleiben in gates_offen
    for rid in sorted({b["quelle"]["regel_id"] for b in VOLL.values()}):
        g = gates_der_regel(rid)
        if len(g) >= 3:
            sz.append(szenario(f"gate2_false_{rid}", [b_(g[1], False)]))
            sz.append(szenario(f"gate_null_statt_false_{rid}", [b_(g[0], 0)]))
            sz.append(szenario(f"gate_alle_true_{rid}", [b_(x, True) for x in g]))
            break
    # Vorjahr
    vorj = [f for f, b in VOLL.items() if b.get("askable") and b.get("vorjahr") == "uebernehmbar"]
    sz.append(szenario("vorjahr_uebernommen", [["rentner_renten_beginn_jahr", 2015, "v", "vorjahr"]]
                       if "rentner_renten_beginn_jahr" in VOLL else [[vorj[0], 1, "v", "vorjahr"]]))
    sz.append(szenario("vorjahr_von_laie", [["rentner_renten_beginn_jahr", 2015, "v", "laie"]]
                       if "rentner_renten_beginn_jahr" in VOLL else [[vorj[0], 1, "v", "laie"]]))
    vs = [f for f, b in VOLL.items() if b.get("askable") and b.get("vorjahr") == "vorschlag"]
    sz.append(szenario("vorjahr_vorschlag_bleibt_frage", [[vs[0], 1, "v", "vorjahr"]]))
    # Instanzen (Gruppe kind)
    kid = "kind_unter_14_haushaltszugehoerig"
    sz.append(szenario("kinder_2_nur_kind1", [b_("fam_anzahl_kinder", 2), b_(kid, False)]))
    sz.append(szenario("kinder_2_kind1_nein_kind2_ja", [b_("fam_anzahl_kinder", 2), b_(kid, False), b_(kid + "__2", True)]))
    sz.append(szenario("kinder_2_beide_nein", [b_("fam_anzahl_kinder", 2), b_(kid, False), b_(kid + "__2", False)]))
    sz.append(szenario("kinder_1_aber_instanz2_im_store", [b_("fam_anzahl_kinder", 1), b_(kid, False), b_(kid + "__2", True)]))
    sz.append(szenario("kinder_zaehlfeld_offen_instanz2_im_store", [b_(kid, False), b_(kid + "__2", True)]))
    sz.append(szenario("kinder_zaehlfeld_vorlaeufig", [v_("fam_anzahl_kinder", 3), b_(kid, False)]))
    sz.append(szenario("kinder_3_zwei_namen", [b_("fam_anzahl_kinder", 3), b_("kind_vorname", "A"), b_("kind_vorname__3", "C")]))
    sz.append(szenario("kinder_ueber_max", [b_("fam_anzahl_kinder", 50), b_("kind_vorname", "A")]))
    sz.append(szenario("kinder_zaehlfeld_bool_und_null", [b_("fam_anzahl_kinder", True), b_(kid, False)]))
    sz.append(szenario("objekte_und_handwerker", [b_("vv_anzahl_objekte", 2), b_("vv_objekt_strasse", "A"),
                                                  b_("hh_anzahl_handwerker", 3), b_("hh_handwerker_betrag", 100),
                                                  b_("hh_handwerker_betrag__2", 200)]))
    sz.append(szenario("instanz_13_nicht_02", [b_("fam_anzahl_kinder", 2), b_("kind_vorname", "A"),
                                               b_("kind_vorname__02", "x"), b_("kind_vorname__1", "y")]))
    return sz


def themen_szenarien() -> list:
    """angefangene Themen: Reihenfolge der Antworten bestimmt die Themenfolge."""
    sz = []
    themen = {}
    for f, b in VOLL.items():
        if b.get("askable") and b["quelle"]["regel_id"] not in ("p2_festzusetzung_einzel", "p2_festzusetzung_zusammen"):
            themen.setdefault(b["quelle"]["regel_id"], []).append(f)
    ids = sorted(t for t, fs in themen.items() if len(fs) >= 3)
    a, b2, c = ids[3], ids[10], ids[17]
    ev = [b_(themen[a][0], "x" if VOLL[themen[a][0]]["typ"] == "text" else 1),
          b_(themen[c][0], "x" if VOLL[themen[c][0]]["typ"] == "text" else 1),
          v_(themen[b2][0], 1), b_(themen[b2][1], 1), b_(themen[a][1], 1)]
    sz.append(szenario("themen_angefangen_reihenfolge", ev))
    sz.append(szenario("themen_vorlaeufig_zaehlt_nicht", [v_(themen[a][0], 1), v_(themen[c][0], 1)]))
    sz.append(szenario("themen_gleiches_feld_zweimal", [b_(themen[a][0], 1), b_(themen[c][0], 1), b_(themen[a][0], 2)]))
    return sz


def zufaellig(n: int = 36) -> list:
    rng = random.Random(SEED)
    sz = []
    for i in range(n):
        ev = zufall(rng, rng.choice([2, 5, 9, 14, 22, 35]))
        sz.append(szenario(f"zufall_{i:02d}", ev))
    return sz


# ------------------------------------------------------------------ Teil-Bindungen
def teilsichten() -> list:
    sz = []
    sz.append(szenario("sicht_nur_veranlagung", [], sicht=["veranlagung"], beitraege=False))
    sz.append(szenario("sicht_veranlagung_und_partner", [b_("veranlagung", "zusammen")],
                       sicht=["veranlagung", "person_b_idnr", "stammdaten_vorname_partner", "bruttoarbeitslohn"]
                       if all(x in BIND for x in ("person_b_idnr", "stammdaten_vorname_partner", "bruttoarbeitslohn")) else ["veranlagung"]))
    kinder = [f for f, b in VOLL.items() if b.get("instanz_gruppe") == "kind"]
    sz.append(szenario("sicht_kinder_ohne_zaehlfeld", [b_("fam_anzahl_kinder", 2), b_(kinder[0], False)], sicht=kinder))
    sz.append(szenario("sicht_kinder_mit_zaehlfeld", [b_("fam_anzahl_kinder", 2), b_(kinder[0], False)],
                       sicht=["fam_anzahl_kinder"] + kinder))
    return sz


# ------------------------------------------------------------------ synthetische Sichten
def S(i, regel, **kw):
    d = {"id": i, "regel": regel}
    d.update(kw)
    return d


def synthetisch() -> list:
    sz = []
    # 1. Gate-Reihenfolge: Gewicht absteigend, Gleichstand: Eingangsfrage vor Merkmal, dann Name.
    sz.append(szenario("synth_gate_reihenfolge", [], sicht={"synth": [
        S("b_leicht", "RA", punkt=["gelt", "ga"], typ="bool"), S("b_s1", "RA"),
        S("z_merkmal", "RB", punkt=["gelt", "gb"], typ="bool"), S("a_merkmal", "RB", punkt=["gelt", "gc"], typ="bool"),
        S("m_eingang", "RB", punkt=["gelt", "gd"], typ="bool", eingang=True), S("b_s2", "RB"), S("b_s3", "RB"),
        S("n_schwer", "RC", punkt=["gelt", "ge"], typ="bool"), S("c_s1", "RC"), S("c_s2", "RC"), S("c_s3", "RC"), S("c_s4", "RC"),
        S("k_int_gelt", "RC", punkt=["gelt", "gf"], typ="int"), S("k_bool_slot", "RC", typ="bool"),
        S("k_nicht_askable", "RC", askable=False), S("k_gate_false", "RC", punkt=["gelt", "gg"], typ="bool", gate=False),
    ]}))
    # 2. Vordruck-Ordnung: nur Felder MIT Kennzahl tauschen die Plaetze, je Klasse.
    vordruck = [
        S("v_gelt_c", "TV", punkt=["gelt", "x1"], typ="text", kz="E0000300"), S("v_ohne_kz", "TV", punkt=["gelt", "x2"], typ="text"),
        S("v_gelt_a", "TV", punkt=["gelt", "x3"], typ="text", kz="E0000100"), S("v_gelt_b", "TV", punkt=["gelt", "x4"], typ="text", kz="E0000200"),
        S("v_wert_z", "TV", kz="E0000700", typ="cent"), S("v_wert_y", "TV", kz="E0000800", typ="cent"), S("v_wert_x", "TV", kz="E0000900", typ="cent"),
        S("v_eingang", "TV", kz="E0000050", typ="cent", eingang=True),
        S("w_gate_kz", "TW", punkt=["gelt", "y1"], typ="bool", kz="E0000600"), S("w_slot", "TW", typ="cent"),
        S("w_formal_b", "TW", punkt=["gelt", "y2"], typ="text", kz="E0000500"), S("w_formal_a", "TW", punkt=["gelt", "y3"], typ="text", kz="E0000400"),
        S("p_formal_b", "TP", punkt=["gelt", "z1"], typ="text", kz="E0000020"), S("p_formal_a", "TP", punkt=["gelt", "z2"], typ="text", kz="E0000010"),
        S("p_wert_b", "TP", kz="E0000040", typ="cent"), S("p_wert_a", "TP", kz="E0000030", typ="cent"),
    ]
    sz.append(szenario("synth_vordruck", [], sicht={"synth": vordruck}, leerer_beitrag=True))
    sz.append(szenario("synth_vordruck_teilweise_beantwortet", [b_("v_gelt_a", "x"), b_("v_wert_z", 1)], sicht={"synth": vordruck}))
    # 2b. Gewicht genau 1 ohne Geltungsbedingung: `veranlagung` schaltet hier EIN Feld ab (p2_festzusetzung_zusammen
    #     hat in dieser Sicht nur `z_nur`) -- es ist ein Gate (vorn, nicht umsortiert), kein Slot.
    sz.append(szenario("synth_gewicht_eins", [], sicht={"synth": [
        S("a_slot", "TQ", kz="E0000100", typ="cent"), S("veranlagung", "TQ", kz="E0000900", typ="cent"),
        S("b_gelt", "TR", punkt=["gelt", "q1"], typ="text"), S("z_nur", "p2_festzusetzung_zusammen", typ="cent"),
    ]}))
    # 2c. Leere regel_id (Schema verlangt keine Mindestlaenge; die echte Bindung hat keine): ein Thema "" zaehlt NIE als
    #     angefangen (`if t:`), auch wenn eines seiner Felder beantwortet ist.
    sz.append(szenario("synth_leere_regel_id", [b_("x_eins", 1)], sicht={"synth": [
        S("a_f", "TA", typ="cent"), S("x_eins", "", typ="cent"), S("x_zwei", "", typ="cent"),
    ]}))
    # 3. Themenfolge: Voraussetzungen (ableitung.aus), Ring, Einstieg + angefangene Themen.
    folge = [
        S("e_f1", "p2_festzusetzung_einzel", typ="cent"), S("e_f2", "p2_festzusetzung_einzel", typ="cent"),
        S("z_f1", "p2_festzusetzung_zusammen", typ="cent"),
        S("t_a1", "TA", typ="cent", ableitung={"aus": "t_b1"}), S("t_b1", "TB", typ="cent"), S("t_b2", "TB", typ="cent"),
        S("t_c1", "TC", typ="cent", ableitung={"aus": "t_a1"}),
        S("r_x1", "RX", typ="cent", ableitung={"aus": "r_y1"}), S("r_y1", "RY", typ="cent", ableitung={"aus": "r_x1"}),
        S("t_d1", "TD", typ="cent"), S("t_d2", "TD", typ="cent"),
        S("t_e1", "TE", typ="cent"),
    ]
    sz.append(szenario("synth_themenfolge_leer", [], sicht={"synth": folge}))
    sz.append(szenario("synth_themenfolge_angefangen", [b_("t_d2", 1), b_("t_e1", 1)], sicht={"synth": folge}))
    sz.append(szenario("synth_themenfolge_nur_einzel_vorne", [], sicht={"synth": [f for f in folge if f["regel"] != "p2_festzusetzung_zusammen"]}))
    sz.append(szenario("synth_themenfolge_ohne_einzel", [b_("t_e1", 1)], sicht={"synth": [f for f in folge if not f["regel"].startswith("p2_")]}))
    # 4. Vorjahr-Uebernahme, Gate/Slot-Teilung
    sz.append(szenario("synth_vorjahr_und_gate", [["j_uebern", 5, "v", "vorjahr"], ["j_vorschlag", 5, "v", "vorjahr"]], sicht={"synth": [
        S("j_uebern", "TJ", typ="cent", vorjahr="uebernehmbar"), S("j_vorschlag", "TJ", typ="cent", vorjahr="vorschlag"),
        S("j_ohne", "TJ", typ="cent"), S("j_gate_weight1", "TK", punkt=["gelt", "k1"], typ="bool"), S("j_slot_k", "TK", typ="cent"),
    ]}))
    return sz


# ------------------------------------------------------------------ Beweis (justification / trace)
def beweis() -> list:
    ev = [["veranlagung", "einzel", "b", "laie", True],        # mit signal
          v_("kein_vuv", False),                               # vorlaeufig
          b_("kein_kap", True), b_("kist_konfession", "ev"),
          b_("bruttoarbeitslohn", 500000), b_("dhf_beruflich_veranlasst", True)]
    ev = [e for e in ev if e[0] in BIND]
    return [szenario("beweis_justification_trace", ev, beitraege=False,
                     justif=[e[0] for e in ev] + ["fam_anzahl_kinder"])]


def rollen_szenarien() -> list:
    """`bindung_rollen.relevante_kegel_felder` / `ring_bindung` (Kegel = Pflichtfelder der Spanne)."""
    sys.path.insert(0, os.path.join(ROOT, "produkt", "traverser"))   # bindung_rollen: `import traverser`
    from produkt.haut import bindung_rollen as BR  # noqa: E402
    kegel = ["veranlagung", "kein_vuv", "vv_wohnzwecke", "vv_entgelt_quote_prozent", "gibt_es_nicht", "veranlagung"]
    out = []
    for name, ev, k, mit_store in (
            ("leerer_kegel_gilt_als_kein_kegel", [], [], True),
            ("kegel_ohne_store", [b_("kein_vuv", False)], kegel, False),
            ("kegel_ohne_ausschluss", [], kegel, True),
            ("kegel_mit_ausschluss", [b_("kein_vuv", False)], kegel, True),
            ("kegel_vorlaeufig_schliesst_nicht_aus", [v_("kein_vuv", False)], kegel, True)):
        store = py_store(ev) if mit_store else None
        out.append({"name": name, "events": ev, "kegel": k, "mit_store": mit_store,
                    "relevante": list(BR.relevante_kegel_felder(tuple(k), VOLL, store)),
                    "ring": list(BR.ring_bindung({"kegel": k}, VOLL, store))})
    return out


def main():
    szenarien = gezielt() + themen_szenarien() + zufaellig() + teilsichten() + synthetisch() + beweis()
    doc = {
        "erzeugt_von": "tools/parity/extract_interview_orakel.py",
        "orakel": "produkt/traverser/traverser.py (Python); Sicht-Reihenfolge der Rust-Registry",
        "universum": list(VOLL),
        "sichten": [v for v in SICHTEN.values()],
        "szenarien": szenarien,
        "rollen": rollen_szenarien(),
    }
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, "w", encoding="utf-8") as f:
        json.dump(doc, f, ensure_ascii=False, separators=(",", ":"), sort_keys=False)
        f.write("\n")
    print(f"{len(szenarien)} Szenarien -> {OUT} ({os.path.getsize(OUT)} Bytes)")


if __name__ == "__main__":
    main()
