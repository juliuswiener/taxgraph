"""Gate fuer ebilanz/feldmapping.yaml (Paket 8).

Prueft je gemapptem Eintrag: das taxonomie_konzept existiert im Muss-Feld-Katalog
JEDER gelisteten wj_version (6.7/6.8) UND die kategorie stimmt mit dem Katalog
ueberein; kein Konzept doppelt gemappt; status:sicher hat rule_id+output, die in
der Registry existieren (signature.output stimmt); status:unklar hat begruendung.
Tamper-Negativtest: erfundenes Konzept / falsche Kategorie -> Verletzung.

Catala-frei, laeuft in `make unit`. Katalog-Quelle: ebilanz/katalog_{6.7,6.8}.json
(regenerierbar: python ebilanz/katalog.py). Registry nur READ.
"""

from __future__ import annotations

import json
import os
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
sys.path.insert(0, ROOT)
sys.path.insert(0, os.path.join(ROOT, "pipeline"))

from yamlstrict import load_yaml  # noqa: E402

FELDMAPPING = os.path.join(ROOT, "ebilanz", "feldmapping.yaml")
RULES = os.path.join(ROOT, "pipeline", "produktion", "rules.yaml")


def _katalog(version: str) -> dict:
    with open(os.path.join(ROOT, "ebilanz", f"katalog_{version}.json"), encoding="utf-8") as fh:
        k = json.load(fh)
    return {e["concept"]: e["kategorie"] for e in k["muss_felder"]}


def _kataloge() -> dict:
    return {v: _katalog(v) for v in ("6.7", "6.8")}


def _mappings() -> list:
    return load_yaml(FELDMAPPING)["mappings"]


def _rule_outputs() -> dict:
    out = {}
    for r in load_yaml(RULES)["regeln"]:
        sig = r.get("signature") or {}
        out[r["rule_id"]] = sig.get("output")
    return out


def konzept_verletzungen(mappings: list, kataloge: dict) -> list:
    """Konzept existiert + kategorie stimmt in JEDER gelisteten wj_version.

    Rueckgabe: Liste (idx, konzept, grund). Leer = ok.
    """
    viol = []
    for i, m in enumerate(mappings):
        konzept = m.get("taxonomie_konzept")
        kat = m.get("kategorie")
        if not konzept:
            viol.append((i, konzept, "kein taxonomie_konzept"))
            continue
        for v in m.get("wj_versionen") or []:
            if v not in kataloge:
                viol.append((i, konzept, f"unbekannte wj_version {v}"))
                continue
            ist = kataloge[v].get(konzept)
            if ist is None:
                viol.append((i, konzept, f"Konzept fehlt im Katalog {v}"))
            elif ist != kat:
                viol.append((i, konzept, f"kategorie {v}: '{kat}' != Katalog '{ist}'"))
    return viol


# -- Gate: alle gemappten Konzepte existieren + kategorie stimmt (beide WJ) ----

def test_konzepte_existieren_und_kategorie_stimmt():
    viol = konzept_verletzungen(_mappings(), _kataloge())
    assert not viol, f"{len(viol)} Konzept-Verletzung(en); erste: {viol[0]}"


# -- Gate: kein Konzept doppelt gemappt ---------------------------------------

def test_keine_konzept_duplikate():
    konzepte = [m["taxonomie_konzept"] for m in _mappings() if m.get("taxonomie_konzept")]
    dupe = sorted({c for c in konzepte if konzepte.count(c) > 1})
    assert not dupe, f"doppelt gemappte Konzepte: {dupe}"


# -- Gate: status:sicher hat echten rule_id + output aus der Registry ---------

def test_sicher_hat_registry_output():
    outs = _rule_outputs()
    bad = []
    for m in _mappings():
        if m.get("status") != "sicher":
            continue
        rid, o = m.get("rule_id"), m.get("output")
        if not rid or not o:
            bad.append((rid, o, "rule_id/output fehlt bei status:sicher"))
        elif rid not in outs:
            bad.append((rid, o, "rule_id nicht in Registry"))
        elif outs[rid] != o:
            bad.append((rid, o, f"signature.output ist '{outs[rid]}'"))
    assert not bad, f"sichere Mappings ohne gueltigen Registry-Output: {bad}"


# -- Gate: status:unklar traegt eine Begruendung ------------------------------

def test_unklar_hat_begruendung():
    ohne = [m.get("taxonomie_konzept") for m in _mappings()
            if m.get("status") == "unklar" and not (m.get("begruendung") or "").strip()]
    assert not ohne, f"unklare Mappings ohne Begruendung: {ohne}"


# -- Selbst-Konsistenz: Bestand > 0, jeder status bekannt ---------------------

def test_bestand_und_status():
    ms = _mappings()
    assert len(ms) >= 8, f"unerwartet wenige Mappings: {len(ms)}"
    unbekannt = [m.get("status") for m in ms if m.get("status") not in ("sicher", "unklar")]
    assert not unbekannt, f"unbekannte status-Werte: {unbekannt}"


# -- Negativtest: erfundenes Konzept MUSS fallen ------------------------------

def test_negativ_erfundenes_konzept():
    kataloge = _kataloge()
    boese = [{"taxonomie_konzept": "bs.ass.ERFUNDEN.nichtImKatalog",
              "kategorie": "Mussfeld", "wj_versionen": ["6.7", "6.8"]}]
    viol = konzept_verletzungen(boese, kataloge)
    assert viol and "fehlt im Katalog" in viol[0][2], viol


# -- Negativtest: falsche Kategorie MUSS fallen -------------------------------

def test_negativ_falsche_kategorie():
    kataloge = _kataloge()
    # bs.ass.prepaidExp ist Mussfeld -> als Summenmussfeld deklariert = Verletzung
    boese = [{"taxonomie_konzept": "bs.ass.prepaidExp",
              "kategorie": "Summenmussfeld", "wj_versionen": ["6.7", "6.8"]}]
    viol = konzept_verletzungen(boese, kataloge)
    assert viol and "kategorie" in viol[0][2], viol
