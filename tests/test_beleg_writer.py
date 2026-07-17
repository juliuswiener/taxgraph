"""Gate für den Beleg-Upload-Writer (produkt/import/, Stufe 1 LStB). Deterministisch, NULL LLM.

Prüft: (a) Beleg-Typ-Erkennung + Anker-Extraktion aus dem synthetischen Muster-LStB gegen die
erwarteten Kandidatenwerte; (b) fail-closed — der Writer schreibt NUR vorlaeufig (herkunft=beleg_import,
schreiber=import:beleg, signal_2=null), signal_1 trägt Beleg-Ref+confidence; (c) Confidence ändert NIE
den Zustand (keine Auto-Bestätigung); (d) unlesbar/nicht gefunden → benannte Lücke (kein Rate-Wert);
(e) Guard-Tamper — import:beleg + bestaetigt wirft (Runtime + Schema). Test-LStB ist SYNTHETISCH.
"""
from __future__ import annotations

import os
import sys

import pytest

yaml = pytest.importorskip("yaml")
jsonschema = pytest.importorskip("jsonschema")

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/import", "produkt/store", "produkt/traverser"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import beleg_writer as BW   # noqa: E402
import store as ST         # noqa: E402
import traverser as TR     # noqa: E402

TS = "2026-07-18T09:00:00+00:00"
MUSTER = open(os.path.join(HERE, "fixtures", "muster_lstb_2025.txt"), encoding="utf-8").read()


@pytest.fixture(scope="module")
def bindung():
    return TR.lade_bindung()


@pytest.fixture(scope="module")
def store_schema():
    import json
    return json.load(open(os.path.join(ROOT, "produkt", "store", "schema.json")))


# ---- (a) Erkennung + Extraktion ----------------------------------------------

def test_erkenne_lstb():
    assert BW.erkenne_lstb(MUSTER) is True
    assert BW.erkenne_lstb("Irgendein anderer Beleg, Rechnung Nr. 5") is False


def test_lstb_felder_aus_herkunft_slots(bindung):
    felder = BW.lstb_felder(bindung)
    assert felder.get("bruttoarbeitslohn") == "3"
    assert felder.get("vor_ag_anteil_rv") == "22"
    assert felder.get("vor_an_anteil_rv") == "23"


def test_extrahiere_kandidaten(bindung):
    k = {x["feld_id"]: x for x in BW.extrahiere(MUSTER, bindung)}
    assert k["bruttoarbeitslohn"]["wert"] == 4500000        # 45.000,00 -> Cent
    assert k["vor_ag_anteil_rv"]["wert"] == 418500          # Nr. 22
    assert k["vor_an_anteil_rv"]["wert"] == 418500          # Nr. 23
    assert all(x["confidence"] == 1.0 for x in k.values())  # Textlayer-Default


def test_kein_lstb_keine_extraktion(bindung):
    assert BW.extrahiere("Rechnung Handwerker, Arbeitskosten 1.000,00", bindung) == []


# ---- (b) fail-closed: Writer schreibt nur vorlaeufig --------------------------

def test_schreibe_vorlaeufig_beleg_import(bindung):
    s = ST.leerer_store(2025, fall_id="beleg-test")
    kand = BW.extrahiere(MUSTER, bindung)
    events = BW.schreibe_kandidaten(s, kand, beleg_ref="beleg:muster_lstb_2025", ts=TS)
    assert len(events) == 3
    for ev in events:
        assert ev["zustand"] == "vorlaeufig"
        assert ev["herkunft"]["herkunft"] == "beleg_import"
        assert ev["schreiber"] == "import:beleg"
        assert ev["signal"]["signal_2"] is None
        assert ev["signal"]["signal_1"]["typ"] == "beleg"
        assert "confidence" in ev["signal"]["signal_1"]
    # materialisiert -> die Werte stehen (als vorlaeufig) im Snapshot
    snap, _ = ST.materialisiere(s)
    assert snap["bruttoarbeitslohn"]["wert"] == 4500000
    assert snap["bruttoarbeitslohn"]["zustand"] == "vorlaeufig"


# ---- (c) Confidence ändert nie den Zustand -----------------------------------

def test_confidence_bleibt_vorlaeufig(bindung):
    s = ST.leerer_store(2025, fall_id="beleg-conf")
    kand = BW.extrahiere(MUSTER, bindung, confidence_map={"3": 0.99, "22": 0.20, "23": 0.55})
    events = BW.schreibe_kandidaten(s, kand, beleg_ref="beleg:x", ts=TS)
    for ev in events:                                       # egal wie hoch/niedrig die Confidence
        assert ev["zustand"] == "vorlaeufig"                # NIE Auto-Bestätigung (K2)
    conf = {ev["feld_id"]: ev["signal"]["signal_1"]["confidence"] for ev in events}
    assert conf["bruttoarbeitslohn"] == 0.99 and conf["vor_ag_anteil_rv"] == 0.20


# ---- (d) unlesbar -> benannte Lücke (kein Rate-Wert) -------------------------

def test_unlesbar_feld_wird_luecke(bindung):
    ohne_nr22 = "\n".join(z for z in MUSTER.splitlines() if "Nr. 22" not in z)
    fids = {x["feld_id"] for x in BW.extrahiere(ohne_nr22, bindung)}
    assert "vor_ag_anteil_rv" not in fids                   # nicht gefunden -> Lücke, kein Wert
    assert "bruttoarbeitslohn" in fids                      # der Rest bleibt extrahiert


# ---- (e) Guard-Tamper: import:beleg kann nicht bestätigen --------------------

def test_neg_import_beleg_kann_nicht_bestaetigen_runtime():
    s = ST.leerer_store(2025, fall_id="beleg-guard")
    with pytest.raises(ValueError, match="import:beleg"):
        ST.append_event(s, feld_id="bruttoarbeitslohn", wert=4500000, zustand="bestaetigt",
                        herkunft={"herkunft": "beleg_import", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                        schreiber="import:beleg",
                        signal={"signal_1": {"typ": "beleg"}, "signal_2": "klick@beleg"}, ts=TS)


def _store_mit_event(ev: dict) -> dict:
    return {"version": 1, "veranlagungszeitraum": 2025, "events": [ev], "snapshots": []}


def _fehler(store_schema, ev: dict) -> list:
    """Validiert ein Event im vollen Store gegen das GESAMTE Schema (die $ref lösen nur so auf)."""
    V = jsonschema.Draft202012Validator(store_schema)
    return [e for e in V.iter_errors(_store_mit_event(ev)) if "events" in list(e.path)]


def test_neg_import_beleg_bestaetigt_schema_invalid(store_schema):
    """Schema-Guard (schreiber-scoped): import:beleg + bestaetigt ist ungültig (allOf-Regel)."""
    ev = {"event_id": "a" * 64, "ts": TS, "feld_id": "bruttoarbeitslohn", "wert": 4500000,
          "zustand": "bestaetigt",
          "herkunft": {"herkunft": "beleg_import", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
          "schreiber": "import:beleg", "signal": {"signal_1": None, "signal_2": "klick"}, "ersetzt": None}
    assert _fehler(store_schema, ev), "import:beleg+bestaetigt müsste das Schema brechen"


def test_import_elster_beleg_import_bestaetigt_bleibt_gueltig(store_schema):
    """Regression: die Provenance beleg_import bleibt nach K3-Bestätigung erhalten — ein ANDERER
    Importer (import:elster-Vorfüllung) darf beleg_import+bestaetigt schreiben (Guard ist schreiber-,
    nicht herkunft-scoped)."""
    ev = {"event_id": "c" * 64, "ts": TS, "feld_id": "vor_an_anteil_rv", "wert": 3500000,
          "zustand": "bestaetigt",
          "herkunft": {"herkunft": "beleg_import", "pruef_tiefe": "plausibilisiert", "haftung": "nutzer"},
          "schreiber": "import:elster", "signal": {"signal_1": None, "signal_2": "lstb_z23"}, "ersetzt": None}
    assert not _fehler(store_schema, ev), "import:elster+beleg_import+bestaetigt sollte gültig bleiben"


def test_signal1_objekt_schema_gueltig(store_schema):
    """signal_1 darf ein Beleg-Herkunfts-Objekt sein (Schema-Erweiterung)."""
    ev = {"event_id": "b" * 64, "ts": TS, "feld_id": "bruttoarbeitslohn", "wert": 4500000,
          "zustand": "vorlaeufig",
          "herkunft": {"herkunft": "beleg_import", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
          "schreiber": "import:beleg",
          "signal": {"signal_1": {"typ": "beleg", "ref": "beleg:x#lstb_nr=3", "confidence": 1.0},
                     "signal_2": None}, "ersetzt": None}
    assert not _fehler(store_schema, ev), "signal_1-Objekt sollte gültig sein"
