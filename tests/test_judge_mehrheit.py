"""Regressionstests des dekomponierten Judge mit Mehrheitsentscheid.

Jeder Test steht fuer eine Regel des Protokolldekrets 2026-07-10:

  * ein Parse-Fehler ist KEINE Stimme -> es wird nachgelaufen,
  * ohne Mehrheit gilt das Item konservativ (Schweigen winkt nichts durch),
  * ein 2:1-Split wird vermerkt und eskaliert auf blockierenden Gates,
  * ein Item, das nur in einem von drei Inventaren auftaucht, ist Rauschen.

Kein Netz, kein Modell: der Client ist ein Skript aus vorgegebenen Antworten.
"""

from __future__ import annotations

import json
import os
import sys

import pytest

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
sys.path.insert(0, ROOT)
sys.path.insert(0, os.path.join(ROOT, "pipeline"))

import judge as J          # noqa: E402
from client import RoleConfig, Completion   # noqa: E402


class FakeClient:
    """Gibt je Template die vorgegebenen Antworten der Reihe nach zurueck."""

    def __init__(self, antworten: dict[str, list[str | None]]):
        self.antworten = {k: list(v) for k, v in antworten.items()}
        self.calls: list[str] = []

    def complete(self, role, messages, fixture_id=None):
        tpl = role.prompt_template_id
        self.calls.append(tpl)
        rest = self.antworten.get(tpl, [])
        text = rest.pop(0) if rest else None
        return Completion(text=text or "", role=role.role, slug=role.slug,
                          provider="fake", prompt_tokens=1, completion_tokens=1,
                          cost_usd=0.0, truncated=(text == "TRUNCATED"))


ROLE = RoleConfig(role="judge", slug="fake/judge", providers=["fake"],
                  prompt_template_id="dekomponiert@1")
SIG = {"scope": "S", "inputs": {"x": "money"}, "output": "y"}
BED = [{"bedingung": "nur_inland", "beschreibung": "nur Inland", "quelle": "§ x"}]


def inv(abw=(), ann=(), teile=()):
    return json.dumps({"abweichungen": list(abw), "annahmen": list(ann),
                       "norm_teile": list(teile)}, ensure_ascii=False)


def lauf(antworten):
    c = FakeClient(antworten)
    v, prov, kosten = J.judge_regel(c, ROLE, "norm", "src", SIG, BED, "hash")
    return v, c


# -- Inventar: Mehrheits-Mitgliedschaft ---------------------------------------

def test_item_nur_in_einem_inventar_ist_rauschen():
    dreimal_gleich = inv(ann=["Die Eingabe x wird als Nettobetrag gelesen"])
    einmal_extra = inv(ann=["Die Eingabe x wird als Nettobetrag gelesen",
                            "Ein voellig anderer Gedanke ueber Fristen"])
    v, _ = lauf({"inventar@1": [dreimal_gleich, einmal_extra, dreimal_gleich],
                 "item_annahme@1": ['{"mapping": "nur_inland"}'] * 3})
    assert len(v["stille_zusatzannahmen"]) == 1
    streuung = v["judge_instability"]["inventar_streuung"]["annahmen"]
    assert len(streuung) == 1 and streuung[0]["in_laeufen"] == 1


def test_item_in_zwei_von_drei_inventaren_zaehlt():
    mit = inv(teile=["Bei einer Taetigkeit im Ausland gelten andere Betraege"])
    ohne = inv()
    v, _ = lauf({"inventar@1": [mit, mit, ohne],
                 "item_normteil@1": ['{"klasse": "unabhaengig", "abgedeckt_von": "none"}'] * 3})
    assert len(v["scope_gap"]) == 1


# -- Stimmen: Parse-Fehler zaehlt nicht, es wird nachgelaufen ------------------

def test_parse_fehler_ist_keine_stimme():
    i = inv(ann=["Die Eingabe x ist ein Nettobetrag"])
    c = FakeClient({"inventar@1": [i, i, i],
                    "item_annahme@1": ["kein JSON", '{"mapping": "nur_inland"}',
                                       '{"mapping": "nur_inland"}',
                                       '{"mapping": "nur_inland"}']})
    v, prov, _ = J.judge_regel(c, ROLE, "norm", "src", SIG, BED, "hash")
    assert v["stille_zusatzannahmen"][0]["bedingung_id"] == "nur_inland"
    # vier Versuche fuer drei gueltige Stimmen
    assert c.calls.count("item_annahme@1") == 4
    assert not v["judge_instability"]["item_splits"]


def test_truncation_ist_keine_stimme():
    i = inv(abw=["Der Cap betraegt 1500 statt 1200 Euro"])
    c = FakeClient({"inventar@1": [i, i, i],
                    "item_abweichung@1": ["TRUNCATED", '{"ist_echt": true}',
                                          '{"ist_echt": true}', '{"ist_echt": true}']})
    v, _, _ = J.judge_regel(c, ROLE, "norm", "src", SIG, BED, "hash")
    assert v["abweichungen"] == ["Der Cap betraegt 1500 statt 1200 Euro"]
    assert c.calls.count("item_abweichung@1") == 4


def test_ohne_gueltige_stimme_gilt_konservativ():
    """Annahme -> undeclared, Norm-Teil -> wirkt_hinein, Abweichung -> echt."""
    i = inv(abw=["Ein Befund"], ann=["Eine Annahme"], teile=["Ein Norm-Teil"])
    unlesbar = ["Prosa"] * J.MAX_VERSUCHE
    v, _ = lauf({"inventar@1": [i, i, i],
                 "item_abweichung@1": list(unlesbar),
                 "item_annahme@1": list(unlesbar),
                 "item_normteil@1": list(unlesbar)})
    assert v["abweichungen"] == ["Ein Befund"]
    assert v["stille_zusatzannahmen"][0]["bedingung_id"] is None
    assert v["scope_gap"][0]["klasse"] == "wirkt_hinein"
    assert len(v["judge_instability"]["items_ohne_mehrheit"]) == 3


# -- Mehrheit und Split -------------------------------------------------------

def test_zwei_zu_eins_mehrheit_entscheidet_und_wird_vermerkt():
    i = inv(ann=["Die Eingabe x ist ein Nettobetrag"])
    v, _ = lauf({"inventar@1": [i, i, i],
                 "item_annahme@1": ['{"mapping": "nur_inland"}',
                                    '{"mapping": "undeclared"}',
                                    '{"mapping": "nur_inland"}']})
    assert v["stille_zusatzannahmen"][0]["bedingung_id"] == "nur_inland"
    splits = v["judge_instability"]["item_splits"]
    assert len(splits) == 1 and splits[0]["art"] == "annahme"


def test_split_auf_blockierendem_gate_eskaliert():
    i = inv(ann=["Die Eingabe x ist ein Nettobetrag"])
    v, _ = lauf({"inventar@1": [i, i, i],
                 "item_annahme@1": ['{"mapping": "nur_inland"}',
                                    '{"mapping": "undeclared"}',
                                    '{"mapping": "nur_inland"}']})
    assert J.hat_split_auf_blockierendem_gate(v) is True


def test_einstimmig_eskaliert_nicht():
    i = inv(ann=["Die Eingabe x ist ein Nettobetrag"])
    v, _ = lauf({"inventar@1": [i, i, i],
                 "item_annahme@1": ['{"mapping": "nur_inland"}'] * 3})
    assert J.hat_split_auf_blockierendem_gate(v) is False
    assert v["faithful"] is True


def test_erfundene_bedingungs_id_wird_zu_undeclared():
    i = inv(ann=["Die Eingabe x ist ein Nettobetrag"])
    v, _ = lauf({"inventar@1": [i, i, i],
                 "item_annahme@1": ['{"mapping": "gibt_es_nicht"}'] * 3})
    assert v["stille_zusatzannahmen"][0]["bedingung_id"] is None
    assert v["faithful"] is False


def test_erfundene_abdeckung_wird_zu_none():
    i = inv(teile=["Bei einer Taetigkeit im Ausland"])
    v, _ = lauf({"inventar@1": [i, i, i],
                 "item_normteil@1": ['{"klasse": "wirkt_hinein", "abgedeckt_von": "phantasie"}'] * 3})
    assert v["scope_gap"][0]["abgedeckt_von"] == "none"


# -- Verdikt traegt seine Herkunft --------------------------------------------

def test_verdikt_traegt_lauf_id_und_timestamp():
    i = inv()
    v, _ = lauf({"inventar@1": [i, i, i]})
    assert v["lauf_id"] and v["timestamp"]
    assert v["judge_protokoll"].startswith("dekomponiert")


def test_kein_gueltiges_inventar_ist_parse_error():
    v, _ = lauf({"inventar@1": ["Prosa"] * J.MAX_VERSUCHE})
    assert v["parse_error"] is True
    assert v["lauf_id"]
