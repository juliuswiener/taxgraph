"""Tests der Praezedenz-Ratsche (M-UI.3).

Harte Eigenschaften: Auto-Apply NUR bei EXAKTER Anker-Gleichheit (kein Fuzzy);
Ziel muss in der Zielregel gueltig sein; Widerruf setzt das Item zurueck UND
sperrt den Praezedenzfall; Auto-Apply schreibt ueber aufnehmen.
"""

from __future__ import annotations

import os
import sys

import pytest

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
sys.path.insert(0, ROOT)
sys.path.insert(0, os.path.join(ROOT, "pipeline"))

from pipeline.ui import praezedenz, service    # noqa: E402
from pipeline.ui.service import IR              # noqa: E402


@pytest.fixture
def wire(tmp_path, monkeypatch):
    reg = tmp_path / "reg"
    reg.mkdir()
    monkeypatch.setattr(IR, "REG_DIR", str(reg))
    monkeypatch.setattr(service, "ENTSCHEIDUNGS_LOG",
                        str(reg / "entscheidungs_log.yaml"))
    monkeypatch.setattr(praezedenz, "PRAEZ_YAML", str(reg / "praezedenz.yaml"))
    monkeypatch.setattr(praezedenz, "PRAEZ_LOG", str(reg / "praezedenz_log.yaml"))
    # kontrollierte Zielregel: Bedingung b1 deklariert, b_fremd nicht
    monkeypatch.setattr(service, "get_rule", lambda rid: {
        "rule_id": rid, "geltungsbedingungen": [{"bedingung": "b1"}]})
    return reg


def item(anker2="x", kat="interpretation", text="t"):
    return {"art": "annahme", "anker": ["betrifft_kat", anker2, kat],
            "text": text, "kategorie": kat}


def test_exakter_treffer_wendet_an__abweichender_anker_nicht(wire):
    it = item("anzahl_kinder")
    praezedenz.record_precedent("rA", it, "bedingung_neu", bedingung="b1",
                                now="t0")
    # gleiches Item, ANDERE Regel -> Treffer (Anker exakt gleich, b1 deklariert)
    assert praezedenz.treffer("rB", item("anzahl_kinder")) is not None
    # abweichender Anker -> KEIN Treffer (kein Fuzzy)
    assert praezedenz.treffer("rB", item("anzahl_der_kinder")) is None
    # abweichende Kategorie -> anderer Anker -> kein Treffer
    assert praezedenz.treffer("rB", item("anzahl_kinder", kat="zeitbezug")) is None


def test_ziel_ungueltig_in_zielregel__kein_apply(wire):
    it = item("splitting")
    praezedenz.record_precedent("rA", it, "bedingung_neu", bedingung="b_fremd",
                                now="t0")
    # b_fremd ist in der Zielregel NICHT deklariert -> Queue, kein Treffer
    assert praezedenz.treffer("rB", item("splitting")) is None


def test_mehrdeutig__kein_apply(wire):
    it = item("k")
    praezedenz.record_precedent("rA", it, "bedingung_neu", bedingung="b1", now="t0")
    # zweiter Fall gleicher Anker, andere Entscheidung -> 2 Kandidaten -> Queue
    praezedenz.record_precedent("rA", it, "nicht_material", now="t0")
    assert praezedenz.treffer("rB", item("k")) is None


def test_auto_apply_schreibt_durch_aufnehmen_und_metrik(wire, monkeypatch):
    praezedenz.record_precedent("rA", item("kinder"), "bedingung_neu",
                                bedingung="b1", now="t0")
    draft = {"rule_id": "rB", "items": [item("kinder"), item("fremd_ohne_praez")]}
    monkeypatch.setattr(service, "open_draft", lambda rid: draft)

    calls = {"n": 0}
    echt = IR.aufnehmen
    monkeypatch.setattr(IR, "aufnehmen",
                        lambda d: (calls.__setitem__("n", calls["n"] + 1), echt(d))[1])

    charge = praezedenz.auto_apply("rB", now="t1")
    assert charge["charge_groesse"] == 2
    assert charge["angewendet"] == 1
    assert charge["quote"] == 0.5
    assert calls["n"] == 1                       # nur das getroffene Item geschrieben
    reg = IR.load("rB")
    assert len(reg["items"]) == 1
    assert reg["items"][0]["bedingung"] == "b1"


def test_widerruf_setzt_zurueck_und_sperrt(wire, monkeypatch):
    praezedenz.record_precedent("rA", item("kinder"), "bedingung_neu",
                                bedingung="b1", now="t0")
    draft = {"rule_id": "rB", "items": [item("kinder")]}
    monkeypatch.setattr(service, "open_draft", lambda rid: draft)

    charge = praezedenz.auto_apply("rB", now="t1")
    aid = charge["angewendete"][0]["id"]
    assert len(IR.load("rB")["items"]) == 1

    res = praezedenz.widerruf(aid, now="t2")
    assert res["item_entfernt"] is True
    assert res["praezedenz_gesperrt"] is True
    # Item zurueck auf offen (aus Registry entfernt)
    assert IR.load("rB")["items"] == []
    # Praezedenzfall gesperrt -> kein erneutes Auto-Apply
    assert praezedenz.treffer("rB", item("kinder")) is None
    charge2 = praezedenz.auto_apply("rB", now="t3")
    assert charge2["angewendet"] == 0


def test_record_ist_idempotent_und_reaktiviert_nicht(wire):
    it = item("kinder")
    f1 = praezedenz.record_precedent("rA", it, "bedingung_neu", bedingung="b1",
                                     now="t0")
    # manuell sperren
    d = praezedenz.load_praez()
    d["faelle"][0]["gesperrt"] = True
    praezedenz.save_praez(d)
    # erneutes record darf NICHT entsperren
    f2 = praezedenz.record_precedent("rA", it, "bedingung_neu", bedingung="b1",
                                     now="t1")
    assert f2["schluessel"] == f1["schluessel"]
    assert praezedenz.load_praez()["faelle"][0]["gesperrt"] is True
