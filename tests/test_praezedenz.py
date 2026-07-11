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
            "text": text, "kategorie": kat, "triage": "offen"}


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


# -- D0: Degenerate-Anchor-Guard ----------------------------------------------

def _abw(text="a"):
    return {"art": "abweichung", "anker": ["betrifft", "?"], "text": text,
            "triage": "offen"}


def test_D0_degenerierter_anker_wird_nie_praezedenz(wire):
    r = praezedenz.record_precedent("rA", _abw(), "bedingung_neu", bedingung="b1",
                                    now="t0")
    assert r.get("skipped")                          # nicht gespeichert
    assert praezedenz.load_praez()["faelle"] == []
    # Abweichung B, gleicher degenerierter Anker -> kein Kandidat, kein Treffer
    assert praezedenz._kandidaten(_abw("anders")) == []
    assert praezedenz.treffer("rB", _abw("anders")) is None


# -- C1: Widerruf loescht nie Human/Seeding-Eintraege -------------------------

def test_C1a_auto_apply_ueberspringt_bereits_registrierten_key(wire, monkeypatch):
    # Human-triagiertes Item steht schon in der Registry (andere Triage/Formulierung)
    service.submit("rB", item("kinder", text="Human-Formulierung"), "grenzfall")
    praezedenz.record_precedent("rA", item("kinder"), "bedingung_neu",
                                bedingung="b1", now="t0")
    draft = {"rule_id": "rB", "items": [item("kinder")]}
    monkeypatch.setattr(service, "open_draft", lambda rid: draft)
    charge = praezedenz.auto_apply("rB", now="t1")
    assert charge["angewendet"] == 0                 # Key steht schon -> kein Auto-Apply
    e = IR.load("rB")["items"][0]
    assert e["triage"] == "grenzfall"                # Human-Item unveraendert
    assert e["formulierungen"] == ["Human-Formulierung"]


def test_C1b_widerruf_schuetzt_veraenderten_eintrag(wire, monkeypatch):
    praezedenz.record_precedent("rA", item("kinder"), "bedingung_neu",
                                bedingung="b1", now="t0")
    draft = {"rule_id": "rB", "items": [item("kinder", text="auto-text")]}
    monkeypatch.setattr(service, "open_draft", lambda rid: draft)
    aid = praezedenz.auto_apply("rB", now="t1")["angewendete"][0]["id"]
    # Human ergaenzt eine zweite Formulierung auf demselben Key
    service.submit("rB", item("kinder", text="human-zusatz"), "bedingung_neu",
                   bedingung="b1")
    assert len(IR.load("rB")["items"][0]["formulierungen"]) == 2

    res = praezedenz.widerruf(aid, now="t2")
    assert res["item_status"] == "kein_match"        # veraendert -> nicht loeschen
    assert res["item_entfernt"] is False
    assert res["praezedenz_gesperrt"] is True         # Praezedenz trotzdem gesperrt
    assert len(IR.load("rB")["items"]) == 1           # Item bleibt


# -- M1: Audit-Log sofort je Item ---------------------------------------------

def test_M1_audit_sofort_wenn_zweites_item_wirft(wire, monkeypatch):
    praezedenz.record_precedent("rA", item("k1"), "bedingung_neu", bedingung="b1",
                                now="t0")
    praezedenz.record_precedent("rA", item("k2"), "bedingung_neu", bedingung="b1",
                                now="t0")
    draft = {"rule_id": "rB", "items": [item("k1"), item("k2")]}
    monkeypatch.setattr(service, "open_draft", lambda rid: draft)
    calls = {"n": 0}
    echt = service.submit

    def boom(*a, **k):
        calls["n"] += 1
        if calls["n"] == 2:
            raise RuntimeError("kaputt beim 2. Item")
        return echt(*a, **k)

    monkeypatch.setattr(service, "submit", boom)
    with pytest.raises(RuntimeError):
        praezedenz.auto_apply("rB", now="t1")
    aw = praezedenz.anwendungen()
    assert len(aw) == 1                               # 1. Item bereits protokolliert
    assert aw[0]["anker_schluessel"] == praezedenz.anker_schluessel(item("k1"))


# -- M2: Idempotenz -----------------------------------------------------------

def test_M2_zweiter_auto_apply_erzeugt_keine_duplikate(wire, monkeypatch):
    praezedenz.record_precedent("rA", item("kinder"), "bedingung_neu",
                                bedingung="b1", now="t0")
    draft = {"rule_id": "rB", "items": [item("kinder")]}
    monkeypatch.setattr(service, "open_draft", lambda rid: draft)
    assert praezedenz.auto_apply("rB", now="t1")["angewendet"] == 1
    n = len(praezedenz.anwendungen())
    c2 = praezedenz.auto_apply("rB", now="t2")        # Key jetzt registriert
    assert c2["angewendet"] == 0
    assert len(praezedenz.anwendungen()) == n         # 0 neue Anwendungen


# -- Whitelist: nur bedingung_neu / nicht_material auto-apply ------------------

def test_whitelist_defekt_nur_hinweis_kein_auto_apply(wire, monkeypatch):
    praezedenz.record_precedent("rA", item("dfk"), "defekt_formalisierer", now="t0")
    # Hinweis in der Queue bleibt (treffer zeigt ihn) ...
    assert praezedenz.treffer("rB", item("dfk")) is not None
    # ... aber Auto-Apply feuert nicht (nicht in der Whitelist)
    draft = {"rule_id": "rB", "items": [item("dfk")]}
    monkeypatch.setattr(service, "open_draft", lambda rid: draft)
    charge = praezedenz.auto_apply("rB", now="t1")
    assert charge["angewendet"] == 0
    assert IR.load("rB").get("items", []) == []


# -- m2: unbekannte rule_id -> KeyError vor Pfadkonstruktion -------------------

def test_m2_unbekannte_rule_id_wirft_vor_draft(wire, monkeypatch):
    monkeypatch.setattr(service, "get_rule", lambda rid: None)
    with pytest.raises(KeyError):
        praezedenz.auto_apply("gibt_es_nicht", now="t1")
