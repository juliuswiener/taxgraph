"""D0 deckt_ab-Freeze-Gate + D1 Multi-Fragment/per-datei + D4 Diskriminanz-Lint.

Vor diesem Gate hatte `deckt_ab` KEINE Freeze-Pruefung: quellen.build_norm_text
pruefte nur zitatanker/auszug, gates.geltungsbereich_gate matchte deckt_ab nur gegen
das Judge-norm_teil. Volllaengen-Disziplin war reine Handarbeit - ein Quell-Umbau
kippte kein Gate (dev-2-Design-Studie, Instructor-Ruling 2026-07-15). D0 schliesst
das Loch: jedes deckt_ab-Fragment muss - nach _normalize - woertlich im aufgeloesten
Freeze stehen (Bedingungs-`datei`-Override, sonst Union der Regel-Quellen).

rundung.deckt_ab (eigenes Feld mit eigenem freeze-geprueftem zitatanker) ist NICHT
Teil dieser Menge - deckt_ab_* iteriert nur `geltungsbedingungen`.
"""

from __future__ import annotations

import os
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
sys.path.insert(0, ROOT)
sys.path.insert(0, os.path.join(ROOT, "pipeline"))

import quellen as Q   # noqa: E402
from yamlstrict import load_yaml   # noqa: E402

MANIFEST = os.path.join(ROOT, "pipeline", "produktion", "rules.yaml")


def _regeln():
    return load_yaml(MANIFEST)["regeln"]


# -- D0: jedes deckt_ab-Fragment steht woertlich im Freeze ---------------------

def test_alle_deckt_ab_im_freeze():
    cache: dict = {}
    verletzungen = []
    for rule in _regeln():
        verletzungen += Q.deckt_ab_freeze_verletzungen(rule, ROOT, cache)
    assert not verletzungen, (
        f"{len(verletzungen)} deckt_ab-Fragment(e) nicht im Freeze; erstes: "
        f"{verletzungen[0]}")


# -- Bestandszahl (Doku; nicht magisch gepinnt, nur Selbst-Konsistenz) --------

def test_bestandszahl_konsistent():
    n_bed = n_frag = 0
    for rule in _regeln():
        for b in rule.get("geltungsbedingungen") or []:
            frags = Q.deckt_ab_fragmente(b)
            if frags:
                n_bed += 1
                n_frag += len(frags)
    # Jede Bedingung mit deckt_ab hat >= 1 Fragment; Fragmente >= Bedingungen.
    assert n_frag >= n_bed >= 260, (n_bed, n_frag)


# -- D0-Negativtest: manipulierter Anker MUSS eine Verletzung erzeugen ---------

def test_negativtest_synthetischer_bad_anker(tmp_path):
    (tmp_path / "f.txt").write_text("Der Freeze enthaelt genau diesen Satz.\n",
                                    encoding="utf-8")
    rule = {
        "rule_id": "neg",
        "quellen": [{"typ": "gesetz", "datei": "f.txt"}],
        "geltungsbedingungen": [
            {"bedingung": "gut", "deckt_ab": "genau diesen Satz", "quelle": "x"},
            {"bedingung": "boese", "deckt_ab": "DIESER SATZ FEHLT IM FREEZE",
             "quelle": "x"},
        ],
    }
    viol = Q.deckt_ab_freeze_verletzungen(rule, str(tmp_path))
    ids = {v[1] for v in viol}
    assert ids == {"boese"}, viol   # nur der bad Anker faellt, der gute nicht


def test_negativtest_reales_fragment_manipuliert():
    # Ein reales, freeze-gedecktes Fragment + angehaengter Fremdtext MUSS fallen.
    cache: dict = {}
    rule = next(r for r in _regeln()
                if r["rule_id"] == "p4_1_bv_vergleich")
    kaputt = {"rule_id": rule["rule_id"], "quellen": rule.get("quellen"),
              "geltungsbedingungen": []}
    for b in rule.get("geltungsbedingungen") or []:
        b2 = dict(b)
        if b2.get("bedingung") == "massgeblichkeit_handelsbilanz_gob":
            b2["deckt_ab"] = b2["deckt_ab"] + " ZZZ_NICHT_IM_FREEZE"
        kaputt["geltungsbedingungen"].append(b2)
    viol = Q.deckt_ab_freeze_verletzungen(kaputt, ROOT, cache)
    assert any(v[1] == "massgeblichkeit_handelsbilanz_gob" for v in viol), viol


def test_build_norm_text_bricht_bei_bad_deckt_ab(tmp_path):
    (tmp_path / "f.txt").write_text("Der Freeze enthaelt genau diesen Satz.\n",
                                    encoding="utf-8")
    rule = {
        "rule_id": "neg2",
        "quellen": [{"typ": "gesetz", "datei": "f.txt",
                     "auszug": "genau diesen Satz"}],
        "geltungsbedingungen": [
            {"bedingung": "boese", "deckt_ab": "FEHLT", "quelle": "x"}],
    }
    try:
        Q.build_norm_text(rule, str(tmp_path))
    except Q.QuellenFehler as e:
        assert "deckt_ab" in str(e)
        return
    assert False, "build_norm_text haette bei bad deckt_ab abbrechen muessen"


# -- D1: Multi-Fragment + per-datei-Override ----------------------------------

def test_multi_fragment_und_datei_override(tmp_path):
    (tmp_path / "a.txt").write_text("Absatz eins mit Fragment A.\n", encoding="utf-8")
    (tmp_path / "b.txt").write_text("Ganz andere Datei mit Fragment B.\n",
                                    encoding="utf-8")
    # Multi-Fragment gegen a.txt (Regelquelle) + per-datei-Override auf b.txt.
    rule = {
        "rule_id": "d1", "quellen": [{"typ": "gesetz", "datei": "a.txt"}],
        "geltungsbedingungen": [
            {"bedingung": "multi", "deckt_ab": ["Fragment A", "Absatz eins"],
             "quelle": "x"},
            {"bedingung": "cross", "deckt_ab": "Fragment B", "datei": "b.txt",
             "quelle": "x"},
        ],
    }
    assert Q.deckt_ab_freeze_verletzungen(rule, str(tmp_path)) == []
    # String-deckt_ab == 1-Fragment-Liste (Rueckwaertskompatibilitaet)
    assert Q.deckt_ab_fragmente({"deckt_ab": "x"}) == ["x"]
    assert Q.deckt_ab_fragmente({"deckt_ab": ["x", "y"]}) == ["x", "y"]


# -- D4: kein kurzes UND mehrdeutiges Fragment (INFO-Baseline sauber) ----------

def test_diskriminanz_keine_kurz_mehrdeutig():
    cache: dict = {}
    flags = []
    for rule in _regeln():
        flags += Q.deckt_ab_diskriminanz(rule, ROOT, cache=cache)
    assert not flags, (
        f"{len(flags)} kurze UND mehrdeutige deckt_ab-Fragment(e) (D4); erstes: "
        f"{flags[0]}")
