"""Gate für den Traverser (produkt/traverser/, K1, Task #11). Deterministisch, NULL LLM.

Prüft Rückwärts (relevanz/naechste_fragen inkl. Günstiger-nicht-geschnitten + ausgeschlossene-Regel-
fragt-nicht) und Vorwärts (justification/trace, Anker = Bindungstabelle-Anker). Plus Sweep-Netz:
jede rules.yaml-günstiger-Regel steht in der Günstiger-Liste ODER als benannte Ausnahme.
"""
from __future__ import annotations

import os
import sys

import pytest

yaml = pytest.importorskip("yaml")

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, os.path.join(ROOT, "produkt", "traverser"))
sys.path.insert(0, os.path.join(ROOT, "produkt", "store"))
sys.path.insert(0, ROOT)
import traverser as T  # noqa: E402
import store as ST     # noqa: E402
from pipeline.gates import _normalize  # noqa: E402

TS = "2026-07-17T14:00:00+00:00"
H = {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}


def _bestaetigt(s, feld_id, wert):
    ST.append_event(s, feld_id=feld_id, wert=wert, zustand="bestaetigt", herkunft=H,
                    schreiber="ui:laie", signal={"signal_1": None, "signal_2": f"ok@{feld_id}"}, ts=TS)


def _vorlaeufig(s, feld_id, wert):
    ST.append_event(s, feld_id=feld_id, wert=wert, zustand="vorlaeufig", herkunft=H,
                    schreiber="ui:laie", signal={"signal_1": None, "signal_2": None}, ts=TS)


@pytest.fixture(scope="module")
def bindung():
    return T.lade_bindung()


@pytest.fixture(scope="module")
def guenstiger():
    return T.lade_guenstiger()


# ---- (a) Rückwärts -------------------------------------------------------------

def test_relevanz_ausgeschlossen_bei_gate_false(bindung):
    s = ST.leerer_store(2025)
    _bestaetigt(s, "dhf_beruflich_veranlasst", False)      # Gating-Bedingung verneint
    rel = T.relevanz(s, bindung)
    assert rel["p9_1_3_nr5_doppelte_haushaltsfuehrung"]["status"] == "ausgeschlossen"


def test_relevanz_unentschieden_bei_offenem_gate(bindung):
    s = ST.leerer_store(2025)
    rel = T.relevanz(s, bindung)
    assert rel["p9_1_3_nr5_doppelte_haushaltsfuehrung"]["status"] == "unentschieden"
    assert "dhf_beruflich_veranlasst" in rel["p9_1_3_nr5_doppelte_haushaltsfuehrung"]["gates_offen"]


def test_relevanz_annahmen_nie_still(bindung):
    # p10_1_2 hat askable:false-Bedingungen (voller_abzug etc.) -> als annahmen_offen geführt
    s = ST.leerer_store(2025)
    rel = T.relevanz(s, bindung)
    ann = rel["p10_1_2_altersvorsorge"]["annahmen_offen"]
    assert "voller_abzug_100_prozent" in ann


# ---- regel_bedingungen: p2_festzusetzung_zusammen nur bei veranlagung=="zusammen" ----------
#
# p2_festzusetzung_zusammen trägt ausschließlich nicht-bool Gates (text/enum/datum: person_b_idnr,
# stammdaten_*_partner, kist_konfession_partner, steuerklasse_partner, ...) — der alte Gate-Loop in
# relevanz() prüft nur `ev.get("wert") is False`, was für diese Typen strukturell nie zutrifft. Die
# Regel konnte also nie ausgeschlossen werden, egal was der Nutzer antwortet (Dialog-Überangebot,
# reports/adjudikation/relevanz_nicht_bool_blast_radius_2026-08-10.md). Fix: regel_bedingungen
# (bindung_regel_bedingungen.yaml) wertet veranlagung regel-weit aus, unabhängig von den eigenen
# Gate-Feldern der Regel.

def test_relevanz_p2_zusammen_ausgeschlossen_bei_einzel(bindung):
    s = ST.leerer_store(2025)
    _bestaetigt(s, "veranlagung", "einzel")
    rel = T.relevanz(s, bindung)
    assert rel["p2_festzusetzung_zusammen"]["status"] == "ausgeschlossen"
    fragen = T.naechste_fragen(s, bindung)
    partner_felder = [f for f in fragen if bindung[f]["quelle"]["regel_id"] == "p2_festzusetzung_zusammen"]
    assert not partner_felder, f"Partner-Felder trotz veranlagung=einzel im Angebot: {partner_felder}"


def test_relevanz_p2_zusammen_bleibt_relevant_bei_zusammen(bindung):
    """Kontrolle: bei echter Zusammenveranlagung bleiben die Partner-Felder im Angebot (kein
    Overshoot des Fixes — die Regel gilt hier wirklich)."""
    s = ST.leerer_store(2025)
    _bestaetigt(s, "veranlagung", "zusammen")
    rel = T.relevanz(s, bindung)
    assert rel["p2_festzusetzung_zusammen"]["status"] != "ausgeschlossen"
    fragen = T.naechste_fragen(s, bindung)
    partner_felder = [f for f in fragen if bindung[f]["quelle"]["regel_id"] == "p2_festzusetzung_zusammen"]
    assert partner_felder, "Zusammenveranlagung: Partner-Felder fälschlich weggefallen"


def test_relevanz_p2_zusammen_faengt_nicht_bei_unbeantwortet(bindung):
    """Fail-closed: solange veranlagung nicht gesetzt ist, wird NICHTS ausgeschlossen — sonst
    verschwindet die Frage, bevor der Nutzer sie beantwortet hat, und am Ende fehlt eine Angabe
    in der Erklärung."""
    s = ST.leerer_store(2025)
    rel = T.relevanz(s, bindung)
    assert rel["p2_festzusetzung_zusammen"]["status"] != "ausgeschlossen"


def test_relevanz_p2_zusammen_faengt_nicht_bei_vorlaeufig(bindung):
    """Fail-closed auch bei vorläufigem (noch nicht bestätigtem) veranlagung-Wert, z.B. aus einer
    Vorjahres-Übernahme (Zwei-Signal) — ein vorläufiger Wert darf ebensowenig ausschließen wie ein
    fehlender."""
    s = ST.leerer_store(2025)
    _vorlaeufig(s, "veranlagung", "einzel")
    rel = T.relevanz(s, bindung)
    assert rel["p2_festzusetzung_zusammen"]["status"] != "ausgeschlossen"


def test_relevanz_p2_zusammen_mutation_ohne_regel_bedingung(bindung, monkeypatch):
    """Gegenprobe: OHNE die regel_bedingung wäre p2_festzusetzung_zusammen bei veranlagung=einzel
    NICHT ausschließbar (das ist exakt der gefundene Bug). Beweist, dass der obige Test nicht
    vakuos grün ist, sondern die regel_bedingungen-Auswertung wirklich der Grund ist."""
    monkeypatch.setattr(T, "lade_regel_bedingungen", lambda: {})
    s = ST.leerer_store(2025)
    _bestaetigt(s, "veranlagung", "einzel")
    rel = T.relevanz(s, bindung)
    assert rel["p2_festzusetzung_zusammen"]["status"] != "ausgeschlossen", (
        "Gegenprobe fehlgeschlagen: ohne regel_bedingungen dürfte die Regel nicht ausschließbar sein")


def test_ausgeschlossene_regel_wird_nicht_gefragt(bindung):
    s = ST.leerer_store(2025)
    _bestaetigt(s, "dhf_beruflich_veranlasst", False)
    fragen = T.naechste_fragen(s, bindung)
    assert not any(bindung[f]["quelle"]["regel_id"] == "p9_1_3_nr5_doppelte_haushaltsfuehrung"
                   for f in fragen), "ausgeschlossene Regel taucht im Interview auf"


def test_gating_vor_slots(bindung):
    s = ST.leerer_store(2025)
    fragen = T.naechste_fragen(s, bindung)
    gate_pos = [i for i, f in enumerate(fragen) if "geltungsbedingung" in bindung[f]["quelle"]]
    slot_pos = [i for i, f in enumerate(fragen) if "signatur_slot" in bindung[f]["quelle"]]
    if gate_pos and slot_pos:
        assert max(gate_pos) < min(slot_pos), "Gating-Fragen nicht vor Slot-Fragen"


def test_beitrag_sortiert_slots(bindung):
    s = ST.leerer_store(2025)
    beitrag = {"ep_entfernung_km": 100, "ep_oepnv_kosten": 5}   # entfernung stärker
    fragen = T.naechste_fragen(s, bindung, beitrag=beitrag)
    slots = [f for f in fragen if "signatur_slot" in bindung[f]["quelle"]]
    assert slots.index("ep_entfernung_km") < slots.index("ep_oepnv_kosten")


# ---- Günstiger: beide Zweige gefragt (naives Lazy bricht) ----------------------

def test_guenstiger_beide_zweige_nicht_geschnitten(bindung, guenstiger):
    s = ST.leerer_store(2025)
    _bestaetigt(s, "ep_entfernung_km", 30)                  # ein Zweig FIX -> naiv würde ÖPNV wegfallen
    fragen = T.naechste_fragen(s, bindung)
    eintrag = next(g for g in guenstiger["guenstiger_regeln"] if g["regel_id"] == "p09_entfernungspauschale")
    for zf in eintrag["zweig_felder"]:
        aktiv = T._aktive_events(s).get(zf)
        if T._unbeantwortet(aktiv):                         # unbeantworteter Zweig MUSS gefragt werden
            assert zf in fragen, f"Günstiger-Zweig {zf} wurde weggeschnitten"


# ---- (b) Vorwärts --------------------------------------------------------------

def test_justification_anker_gleich_bindung(bindung):
    s = ST.leerer_store(2025)
    _bestaetigt(s, "ep_arbeitstage", 220)
    j = T.justification(s, "ep_arbeitstage", bindung)
    assert j["regel_id"] == "p09_entfernungspauschale"
    assert j["signatur_slot"] == "arbeitstage"
    assert j["anker_ref"] == bindung["ep_arbeitstage"]["anker_ref"]   # Trace-Anker = Bindungstabelle-Anker
    assert j["event_id"] and j["zustand"] == "bestaetigt"


def test_trace_ergebnis_gruppiert_je_regel(bindung):
    s = ST.leerer_store(2025)
    _bestaetigt(s, "ep_arbeitstage", 220)
    _bestaetigt(s, "vor_an_anteil_rv", 3500000)
    tr = T.trace_ergebnis(s, bindung, snapshot_id="a" * 64)
    assert "p09_entfernungspauschale" in tr["regeln"]
    assert "p10_1_2_altersvorsorge" in tr["regeln"]
    assert tr["basis_snapshot"] == "a" * 64


# ---- Günstiger-Sweep-Netz: kein stilles Vergessen ------------------------------

def test_guenstiger_sweep_netz(guenstiger):
    rules = T.lade_rules()
    erfasst = ({g["regel_id"] for g in guenstiger.get("guenstiger_regeln", [])}
               | {a["regel_id"] for a in guenstiger.get("ausnahmen", [])})
    fehlend = []
    for rid, r in rules.items():
        blob = yaml.safe_dump(r, allow_unicode=True).lower()
        if ("günstiger" in blob or "guenstiger" in blob) and rid not in erfasst:
            fehlend.append(rid)
    assert not fehlend, f"günstiger-Regeln weder in Liste noch Ausnahmen: {fehlend}"


def test_guenstiger_anker_verifiziert(guenstiger):
    for g in guenstiger.get("guenstiger_regeln", []):
        a = g["anker_ref"]
        pfad = os.path.join(ROOT, a["datei"])
        norm = _normalize(open(pfad, encoding="utf-8").read())
        assert _normalize(a["zitatanker"]) in norm, f"{g['regel_id']}: Günstiger-Anker nicht in Quelle"


# ---- p35a_mitveranlagung: Slot, kein Gate (Screening-Sweep 2026-08-12) ----------
#
# p35a_mitveranlagung hing ueber geltungsbedingung: mitveranlagung_faktor an relevanz().
# "mitveranlagung_faktor" existiert nirgends in rules.yaml als bedingung — die Bindung war keine
# Ob-Bedingung, sondern eine reine Slot-Frage: golden/runner.py catala_p35a_haushaltsnahe() liest
# p35a_mitveranlagung direkt als Halbierungsfaktor, nicht als Eligibility-Check. Der Normalfall
# (Antwort "Nein") schloss dadurch die GESAMTE Regel p35a_2_3_haushaltsnahe aus — die
# haushaltsnahen Leistungen wurden nie erfragt, § 35a faktisch nie beantragt (over-tax). Fix:
# geltungsbedingung entfernt (produkt/bindung/bindung_sonder_agb_35a.yaml).

def test_p35a_mitveranlagung_ist_kein_gate(bindung):
    assert "geltungsbedingung" not in bindung["p35a_mitveranlagung"]["quelle"]


def test_relevanz_p35a_normalfall_bleibt_relevant(bindung):
    """Normalfall: Nutzer beantwortet die Mitveranlagungs-Frage mit 'Nein' (häufigster Fall).
    Die Regel darf NICHT ausgeschlossen werden — sonst verschwinden alle Fragen zu
    Handwerkerleistungen/Dienstleistungen und § 35a wird faktisch nie beantragt."""
    s = ST.leerer_store(2025)
    _bestaetigt(s, "p35a_mitveranlagung", False)
    rel = T.relevanz(s, bindung)
    assert rel["p35a_2_3_haushaltsnahe"]["status"] != "ausgeschlossen"
    fragen = T.naechste_fragen(s, bindung)
    p35a_felder = [f for f in fragen if bindung[f]["quelle"]["regel_id"] == "p35a_2_3_haushaltsnahe"]
    assert p35a_felder, "§35a-Folgefragen fehlen trotz Normalfall-Antwort"


def test_relevanz_p35a_mutation_mit_geltungsbedingung(bindung):
    """Gegenprobe: MIT der (falschen) geltungsbedingung wäre die Regel bei der Normalfall-Antwort
    ausgeschlossen gewesen — das ist exakt der gefundene Bug. Beweist, dass der obige Test nicht
    vakuos grün ist, sondern die entfernte geltungsbedingung wirklich der Grund war."""
    b2 = {k: (dict(v, quelle=dict(v["quelle"])) if k == "p35a_mitveranlagung" else v)
          for k, v in bindung.items()}
    b2["p35a_mitveranlagung"]["quelle"]["geltungsbedingung"] = "mitveranlagung_faktor"
    s = ST.leerer_store(2025)
    _bestaetigt(s, "p35a_mitveranlagung", False)
    rel = T.relevanz(s, b2)
    assert rel["p35a_2_3_haushaltsnahe"]["status"] == "ausgeschlossen", (
        "Gegenprobe fehlgeschlagen: mit geltungsbedingung müsste die Regel ausschließbar sein")
