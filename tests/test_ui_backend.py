"""Tests der Triage-UI-Service-Schicht (Workstream 1).

Kern: der EINE Schreibpfad. Jede Submission geht durch item_registry.aufnehmen;
Merge-Erkennung; Auth der Schreib-Endpoints.
"""

from __future__ import annotations

import os
import sys

import pytest

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
sys.path.insert(0, ROOT)
sys.path.insert(0, os.path.join(ROOT, "pipeline"))

from pipeline.ui import praezedenz         # noqa: E402
from pipeline.ui import service           # noqa: E402
from pipeline.ui.service import IR         # noqa: E402


@pytest.fixture
def wire(tmp_path, monkeypatch):
    reg = tmp_path / "reg"
    reg.mkdir()
    monkeypatch.setattr(IR, "REG_DIR", str(reg))
    monkeypatch.setattr(service, "ENTSCHEIDUNGS_LOG",
                        str(reg / "entscheidungs_log.yaml"))
    # Auch die Praezedenz-Pfade umbiegen: der Submit-Endpoint registriert einen
    # Praezedenzfall, der sonst in die ECHTE Registry schriebe.
    monkeypatch.setattr(praezedenz, "PRAEZ_YAML", str(reg / "praezedenz.yaml"))
    monkeypatch.setattr(praezedenz, "PRAEZ_LOG", str(reg / "praezedenz_log.yaml"))
    return reg


# p33 hat die Bedingung `kinder_sind_anspruchskinder` deklariert.
RID = "p33_3_zumutbare_belastung"
ITEM = {"art": "annahme", "anker": ["betrifft_kat", "anzahl_kinder", "interpretation"],
        "text": "Die Anzahl der Kinder wird als Anspruchskinder interpretiert.",
        "kategorie": "interpretation"}


def test_submit_geht_durch_aufnehmen(wire, monkeypatch):
    calls = {"n": 0}
    echt = IR.aufnehmen

    def spy(draft):
        calls["n"] += 1
        return echt(draft)

    monkeypatch.setattr(IR, "aufnehmen", spy)
    res = service.submit(RID, ITEM, "bedingung_neu",
                         bedingung="kinder_sind_anspruchskinder",
                         now="2026-07-11T00:00:00Z")
    assert calls["n"] == 1                      # aufnehmen TATSAECHLICH aufgerufen
    assert res["aufgenommen"] == 1
    reg = IR.load(RID)
    keys = {IR._key(it["art"], it["anker"]) for it in reg["items"]}
    assert IR._key(ITEM["art"], ITEM["anker"]) in keys
    log = service.entscheidungs_log()
    assert log[-1]["entschieden_via"] == "ui"
    assert log[-1]["timestamp"] == "2026-07-11T00:00:00Z"
    assert log[-1]["item_snapshot"] == ITEM


def test_submit_bedingung_neu_ohne_bedingung_faellt(wire):
    with pytest.raises(ValueError):
        service.submit(RID, ITEM, "bedingung_neu")


def test_submit_undeklarierte_bedingung_wird_geschrieben_aber_geflaggt(wire):
    # bedingung_neu mit noch nicht deklarierter Bedingung ist der Normalfall,
    # der den geltungsbereich_gate im Review haelt - es wird geschrieben, nur
    # als nicht-deklariert markiert (kein hartes Reject).
    res = service.submit(RID, ITEM, "bedingung_neu", bedingung="gibt_es_nicht")
    assert res["aufgenommen"] == 1
    assert res["bedingung_deklariert"] is False
    reg = IR.load(RID)
    assert reg["items"][0]["bedingung"] == "gibt_es_nicht"


def test_submit_defekt_formalisierer_ohne_bedingung(wire):
    it = {"art": "annahme", "anker": ["betrifft_kat", "ergebnis", "rundung"],
          "text": "rundet ohne Normbefehl"}
    res = service.submit(RID, it, "defekt_formalisierer")
    assert res["aufgenommen"] == 1
    reg = IR.load(RID)
    e = reg["items"][0]
    assert e["triage"] == "defekt_formalisierer"
    assert "bedingung" not in e            # defekt darf NIE eine bedingung tragen


def test_merge_erkennung_und_merge_log(wire):
    service.submit(RID, ITEM, "bedingung_neu",
                   bedingung="kinder_sind_anspruchskinder")
    zweite = dict(ITEM, text="Andere Formulierung derselben Annahme.")
    res = service.submit(RID, zweite, "bedingung_neu",
                         bedingung="kinder_sind_anspruchskinder")
    assert res["gemerged"] is True
    ml = service.merge_log()
    treffer = [m for m in ml if m["rule_id"] == RID]
    assert treffer and len(treffer[0]["formulierungen"]) == 2


def test_norm_excerpt_aus_eingefrorenem_normtext(wire):
    rule = service.get_rule(RID)
    # eine woertliche Formulierung aus § 33 Abs. 3
    ex = service.norm_excerpt(rule, "Als Kinder des Steuerpflichtigen zählen")
    assert ex["gefunden"] is True
    assert "Kinder" in ex["ausschnitt"]


# -- App / Auth ---------------------------------------------------------------

def test_schreiben_ohne_token_gesperrt(wire, monkeypatch):
    from fastapi.testclient import TestClient
    from pipeline.ui import app as appmod
    monkeypatch.delenv("TAXGRAPH_UI_TOKEN", raising=False)
    c = TestClient(appmod.app)
    r = c.post(f"/api/rule/{RID}/submit",
               json={"item": ITEM, "triage": "nicht_echt"})
    assert r.status_code == 503


def test_schreiben_mit_token(wire, monkeypatch):
    from fastapi.testclient import TestClient
    from pipeline.ui import app as appmod
    monkeypatch.setenv("TAXGRAPH_UI_TOKEN", "geheim")
    c = TestClient(appmod.app)
    r = c.post(f"/api/rule/{RID}/submit",
               json={"item": ITEM, "triage": "bedingung_neu",
                     "bedingung": "kinder_sind_anspruchskinder"},
               headers={"X-Taxgraph-Token": "geheim"})
    assert r.status_code == 200, r.text
    assert r.json()["aufgenommen"] == 1
    r2 = c.post(f"/api/rule/{RID}/submit",
                json={"item": ITEM, "triage": "nicht_echt"},
                headers={"X-Taxgraph-Token": "falsch"})
    assert r2.status_code == 401


def test_queue_und_read_endpoints(wire):
    from fastapi.testclient import TestClient
    from pipeline.ui import app as appmod
    c = TestClient(appmod.app)
    assert c.get("/api/queue").status_code == 200
    assert c.get("/api/merge-log").status_code == 200
    assert c.get("/api/praezedenz").status_code == 200
