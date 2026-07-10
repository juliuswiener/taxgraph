"""Regressionstests des dekomponierten Judge mit Mehrheitsentscheid.

Jeder Test steht fuer eine Regel des Protokolldekrets 2026-07-10:

  * ein Parse-Fehler ist KEINE Stimme -> es wird nachgelaufen,
  * ohne Mehrheit gilt das Item konservativ (Schweigen winkt nichts durch),
  * ein 2:1-Split wird vermerkt und eskaliert auf blockierenden Gates,
  * ein Item, das nur ein Inventarlauf sah, wird trotzdem beurteilt (es
    wegzulassen waere stilles Gruen).

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
import gates as G          # noqa: E402
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
    """Inventar-Antwort im inventar@2-Format. Strings werden zu Struktur-Items
    mit Default-Anker (betrifft='x', kategorie='interpretation', referenz='§ 1')."""
    def wrap(xs, f):
        out = []
        for x in xs:
            if isinstance(x, dict):
                out.append(x)
            elif f == "abweichungen":
                out.append({"betrifft": "x", "aussage": x})
            elif f == "annahmen":
                out.append({"betrifft": "x", "kategorie": "interpretation", "aussage": x})
            else:
                out.append({"referenz": "§ 1", "zitat": x})
        return out
    return json.dumps({"abweichungen": wrap(abw, "abweichungen"),
                       "annahmen": wrap(ann, "annahmen"),
                       "norm_teile": wrap(teile, "norm_teile")}, ensure_ascii=False)


def lauf(antworten):
    c = FakeClient(antworten)
    v, prov, kosten = J.judge_regel(c, ROLE, "norm", "src", SIG, BED, "hash")
    return v, c


# -- Inventar: Vereinigung, kein Weglassen ------------------------------------

def test_item_aus_nur_einem_inventar_wird_trotzdem_beurteilt():
    """Ein Item wegzulassen, weil es nur ein Inventarlauf sah, waere stilles Gruen.

    Die Vereinigung nimmt es auf; die Item-Abstimmung filtert Rauschen ohnehin.
    Wie oft es gesehen wurde, steht als `inventar_streuung` im Report.
    """
    dreimal_gleich = inv(ann=["Die Eingabe x wird als Nettobetrag gelesen"])
    einmal_extra = inv(ann=["Die Eingabe x wird als Nettobetrag gelesen",
                            "Ein voellig anderer Gedanke ueber Fristen"])
    v, _ = lauf({"inventar@2": [dreimal_gleich, einmal_extra, dreimal_gleich],
                 "item_annahme@1": ['{"mapping": "nur_inland"}'] * 6})
    assert len(v["stille_zusatzannahmen"]) == 2
    streuung = v["judge_instability"]["inventar_streuung"]["annahmen"]
    assert len(streuung) == 1 and streuung[0]["in_laeufen"] == 1


def test_umformulierung_ist_dasselbe_item():
    """Derselbe Befund, einmal knapp und einmal ausfuehrlich. Jaccard haette zwei
    Items daraus gemacht; gemessen wird die Ueberdeckung der kleineren Wortmenge."""
    kurz = inv(ann=["Die Eingabe x ist ein Nettobetrag"])
    lang = inv(ann=["Die Formalisierung nimmt an, dass die Eingabe x als Nettobetrag zu lesen ist"])
    v, _ = lauf({"inventar@2": [kurz, lang, kurz],
                 "item_annahme@1": ['{"mapping": "nur_inland"}'] * 3})
    assert len(v["stille_zusatzannahmen"]) == 1


def test_item_in_zwei_von_drei_inventaren_zaehlt():
    mit = inv(teile=["Bei einer Taetigkeit im Ausland gelten andere Betraege"])
    ohne = inv()
    v, _ = lauf({"inventar@2": [mit, mit, ohne],
                 "item_normteil@1": ['{"klasse": "unabhaengig", "abgedeckt_von": "none"}'] * 3})
    assert len(v["scope_gap"]) == 1


# -- Stimmen: Parse-Fehler zaehlt nicht, es wird nachgelaufen ------------------

def test_parse_fehler_ist_keine_stimme():
    i = inv(ann=["Die Eingabe x ist ein Nettobetrag"])
    c = FakeClient({"inventar@2": [i, i, i],
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
    c = FakeClient({"inventar@2": [i, i, i],
                    "item_abweichung@1": ["TRUNCATED", '{"ist_echt": true}',
                                          '{"ist_echt": true}', '{"ist_echt": true}']})
    v, _, _ = J.judge_regel(c, ROLE, "norm", "src", SIG, BED, "hash")
    assert v["abweichungen"] == ["Der Cap betraegt 1500 statt 1200 Euro"]
    assert c.calls.count("item_abweichung@1") == 4


def test_ohne_gueltige_stimme_gilt_konservativ():
    """Annahme -> undeclared, Norm-Teil -> wirkt_hinein, Abweichung -> echt."""
    i = inv(abw=["Ein Befund"], ann=["Eine Annahme"], teile=["Ein Norm-Teil"])
    unlesbar = ["Prosa"] * J.MAX_VERSUCHE
    v, _ = lauf({"inventar@2": [i, i, i],
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
    v, _ = lauf({"inventar@2": [i, i, i],
                 "item_annahme@1": ['{"mapping": "nur_inland"}',
                                    '{"mapping": "undeclared"}',
                                    '{"mapping": "nur_inland"}']})
    assert v["stille_zusatzannahmen"][0]["bedingung_id"] == "nur_inland"
    splits = v["judge_instability"]["item_splits"]
    assert len(splits) == 1 and splits[0]["art"] == "annahme"


def test_split_auf_blockierendem_gate_eskaliert():
    i = inv(ann=["Die Eingabe x ist ein Nettobetrag"])
    v, _ = lauf({"inventar@2": [i, i, i],
                 "item_annahme@1": ['{"mapping": "nur_inland"}',
                                    '{"mapping": "undeclared"}',
                                    '{"mapping": "nur_inland"}']})
    assert J.hat_split_auf_blockierendem_gate(v) is True


def test_einstimmig_eskaliert_nicht():
    i = inv(ann=["Die Eingabe x ist ein Nettobetrag"])
    v, _ = lauf({"inventar@2": [i, i, i],
                 "item_annahme@1": ['{"mapping": "nur_inland"}'] * 3})
    assert J.hat_split_auf_blockierendem_gate(v) is False
    assert v["faithful"] is True


def test_erfundene_bedingungs_id_wird_zu_undeclared():
    i = inv(ann=["Die Eingabe x ist ein Nettobetrag"])
    v, _ = lauf({"inventar@2": [i, i, i],
                 "item_annahme@1": ['{"mapping": "gibt_es_nicht"}'] * 3})
    assert v["stille_zusatzannahmen"][0]["bedingung_id"] is None
    assert v["faithful"] is False


def test_erfundene_abdeckung_wird_zu_none():
    i = inv(teile=["Bei einer Taetigkeit im Ausland"])
    v, _ = lauf({"inventar@2": [i, i, i],
                 "item_normteil@1": ['{"klasse": "wirkt_hinein", "abgedeckt_von": "phantasie"}'] * 3})
    assert v["scope_gap"][0]["abgedeckt_von"] == "none"


# -- Verdikt traegt seine Herkunft --------------------------------------------

def test_verdikt_traegt_lauf_id_und_timestamp():
    i = inv()
    v, _ = lauf({"inventar@2": [i, i, i]})
    assert v["lauf_id"] and v["timestamp"]
    assert v["judge_protokoll"].startswith("dekomponiert")


def test_kein_gueltiges_inventar_ist_parse_error():
    v, _ = lauf({"inventar@2": ["Prosa"] * J.MAX_VERSUCHE})
    assert v["parse_error"] is True
    assert v["lauf_id"]


# -- Ueber-Merge-Schutz: Anker + Kategorie ------------------------------------

def test_gleicher_anker_andere_kategorie_bleibt_getrennt():
    """Zwei Annahmen ueber dieselbe Eingabe, aber verschiedene Kategorie, sind
    verschiedene Items. Der Anker allein wuerde sie ueber-mergen."""
    interp = json.dumps({"abweichungen": [], "norm_teile": [],
        "annahmen": [{"betrifft": "x", "kategorie": "interpretation",
                      "aussage": "x erfuellt die Legaldefinition"}]}, ensure_ascii=False)
    zeit = json.dumps({"abweichungen": [], "norm_teile": [],
        "annahmen": [{"betrifft": "x", "kategorie": "zeitbezug",
                      "aussage": "x gilt ganzjaehrig"}]}, ensure_ascii=False)
    v, _ = lauf({"inventar@2": [interp, zeit, interp],
                 "item_annahme@1": ['{"mapping": "nur_inland"}'] * 6})
    assert len(v["stille_zusatzannahmen"]) == 2


def test_gleicher_schluessel_verschiedener_text_bleibt_getrennt():
    """Innerhalb eines Buckets (gleicher Anker+Kategorie) trennt der Textabgleich
    zwei inhaltlich verschiedene Befunde."""
    a = json.dumps({"abweichungen": [], "norm_teile": [],
        "annahmen": [{"betrifft": "x", "kategorie": "interpretation",
                      "aussage": "x wird als Bruttobetrag vor Abzuegen gelesen"}]}, ensure_ascii=False)
    b = json.dumps({"abweichungen": [], "norm_teile": [],
        "annahmen": [{"betrifft": "x", "kategorie": "interpretation",
                      "aussage": "x umfasst ausschliesslich inlaendische Sachverhalte"}]}, ensure_ascii=False)
    v, _ = lauf({"inventar@2": [a, b, a],
                 "item_annahme@1": ['{"mapping": "nur_inland"}'] * 6})
    assert len(v["stille_zusatzannahmen"]) == 2


def test_erfundener_anker_faellt_auf_sammelbucket():
    """Ein betrifft, das keine Signatur-Eingabe ist, wird nicht als Anker
    verwendet - das Item verschwindet nicht, es clustert per Text."""
    erfunden = json.dumps({"abweichungen": [], "norm_teile": [],
        "annahmen": [{"betrifft": "gibt_es_nicht", "kategorie": "interpretation",
                      "aussage": "Eine Annahme ueber etwas Erfundenes"}]}, ensure_ascii=False)
    v, _ = lauf({"inventar@2": [erfunden, erfunden, erfunden],
                 "item_annahme@1": ['{"mapping": "undeclared"}'] * 3})
    assert len(v["stille_zusatzannahmen"]) == 1
    assert v["stille_zusatzannahmen"][0]["bedingung_id"] is None


def test_norm_teil_traegt_referenz():
    teil = json.dumps({"abweichungen": [], "annahmen": [],
        "norm_teile": [{"referenz": "§ 9 Abs. 4a", "zitat": "Auslandsbetraege"}]}, ensure_ascii=False)
    v, _ = lauf({"inventar@2": [teil, teil, teil],
                 "item_normteil@1": ['{"klasse": "wirkt_hinein", "abgedeckt_von": "none"}'] * 3})
    assert v["scope_gap"][0]["referenz"] == "§ 9 Abs. 4a"
    assert v["scope_gap"][0]["klasse"] == "wirkt_hinein"


def test_annahme_traegt_anker():
    v, _ = lauf({"inventar@2": [inv(ann=["Die Eingabe x ist ein Nettobetrag"])] * 3,
                 "item_annahme@1": ['{"mapping": "nur_inland"}'] * 3})
    a = v["stille_zusatzannahmen"][0]
    assert a["betrifft"] == "x" and a["kategorie"] == "interpretation"


# -- Union-until-Saturation ---------------------------------------------------

def test_inventar_stoppt_bei_saettigung():
    """Zwei identische Inventare -> Lauf 2 bringt nichts Neues -> Stopp nach 2."""
    i = inv(ann=["Die Eingabe x ist ein Nettobetrag"])
    c = FakeClient({"inventar@2": [i, i, i, i, i],
                    "item_annahme@1": ['{"mapping": "nur_inland"}'] * 3})
    v, prov, _ = J.judge_regel(c, ROLE, "norm", "src", SIG, BED, "hash")
    assert c.calls.count("inventar@2") == 2
    assert v["judge_instability"]["gesaettigt"] is True
    assert v["judge_instability"]["saettigungskurve"][-1] == 0


def test_inventar_deckelt_bei_fuenf():
    """Jeder Lauf bringt etwas Neues -> Deckel greift bei 5."""
    verschieden = [json.dumps({"abweichungen": [], "annahmen": [],
        "norm_teile": [{"referenz": f"§ {k}", "zitat": f"Klausel {k}"}]}, ensure_ascii=False)
        for k in range(6)]
    c = FakeClient({"inventar@2": verschieden,
                    "item_normteil@1": ['{"klasse": "unabhaengig", "abgedeckt_von": "none"}'] * 20})
    v, _, _ = J.judge_regel(c, ROLE, "norm", "src", SIG, BED, "hash")
    assert c.calls.count("inventar@2") == J.INVENTAR_MAX == 5
    assert v["judge_instability"]["gesaettigt"] is False


# -- Grenzfall / Dauersplitter ------------------------------------------------

def test_dauersplitter_wird_als_grenzfall_markiert():
    teil = json.dumps({"abweichungen": [], "annahmen": [],
        "norm_teile": [{"referenz": "§ 9 Abs. 4a", "zitat": "Auslandsbetraege"}]}, ensure_ascii=False)
    key = J._kkey("norm_teile", {"key": ("ref", "§ 9 abs. 4a")})
    c = FakeClient({"inventar@2": [teil, teil],
                    "item_normteil@1": ['{"klasse": "wirkt_hinein", "abgedeckt_von": "none"}'] * 3})
    v, _, _ = J.judge_regel(c, ROLE, "norm", "src", SIG, BED, "hash",
                            dauersplitter={key})
    assert v["scope_gap"][0]["grenzfall"] is True
    assert len(v["grenzfaelle"]) == 1


def test_grenzfall_gate_faellt_bei_registriertem_dauersplitter():
    v = {"grenzfaelle": [{"schluessel": "k", "norm_teil": "x"}],
         "scope_gap": [], "abweichungen": [], "stille_zusatzannahmen": []}
    assert G.grenzfall_gate(v).status == G.FAIL


def test_grenzfall_gate_pass_ohne_dauersplitter():
    v = {"grenzfaelle": [], "scope_gap": [], "abweichungen": [], "stille_zusatzannahmen": []}
    assert G.grenzfall_gate(v).status == G.PASS


# -- Registry-Builder ---------------------------------------------------------

def test_grenzfall_builder_erkennt_dauersplitter(tmp_path):
    import grenzfaelle as GF
    mess = {"laeufe": {"r1": [
        {"parse_error": False, "item_split_keys": [
            {"schluessel": "k_dauer", "art": "norm_teil", "item": "ambig"},
            {"schluessel": "k_einmal", "art": "norm_teil", "item": "einmalig"}]},
        {"parse_error": False, "item_split_keys": [
            {"schluessel": "k_dauer", "art": "norm_teil", "item": "ambig"}]},
        {"parse_error": True},
    ]}}
    f = tmp_path / "mess.json"
    f.write_text(json.dumps(mess), encoding="utf-8")
    neu = GF.bauen([str(f)])
    # k_dauer split in 2 Laeufen -> Dauersplitter; k_einmal nur 1 -> nicht
    assert "r1" in neu
    keys = {e["schluessel"] for e in neu["r1"]}
    assert keys == {"k_dauer"}
