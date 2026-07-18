"""Gate für die § 22 bb-Ertragsanteil-Kohortentabelle (params/kohorten/rente_ertragsanteil_p22.yaml) +
den § 9a S.1 Nr.3 Renten-WK-Pauschbetrag (102). Deterministisch, NULL LLM.

Validiert Vollständigkeit (Alter 0..97 lückenlos, sonst fällt der Accessor-Lookup ins Leere), Monotonie
(Ertragsanteil fällt mit steigendem Alter bei Rentenbeginn) und Anker-Spot-Werte gegen § 22 Nr.1 S.3 a bb
(0→59, 64→19, 65→18, 97→1). bb ist exakt für alle Jahre (Ertragsanteil bei Rentenbeginn fix) — keine
Fixierung nötig, anders als aa (dort greift der Rentenfreibetrag-K2-Guard im Accessor).
"""
from __future__ import annotations

import os

import pytest

yaml = pytest.importorskip("yaml")

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def _bb():
    p = os.path.join(ROOT, "params", "kohorten", "rente_ertragsanteil_p22.yaml")
    return yaml.safe_load(open(p, encoding="utf-8"))["kohorten"]


def test_bb_alter_luckenlos_0_bis_97():
    assert sorted(_bb()) == list(range(0, 98))          # jeder Accessor-Lookup 0..97 trifft


def test_bb_monoton_fallend():
    werte = [_bb()[a]["ertragsanteil_prozent"] for a in range(0, 98)]
    assert all(werte[i] >= werte[i + 1] for i in range(len(werte) - 1))


def test_bb_anker_spotwerte():
    k = _bb()                                            # § 22 Nr.1 S.3 a bb Tabelle
    assert k[0]["ertragsanteil_prozent"] == 59.0
    assert k[64]["ertragsanteil_prozent"] == 19.0
    assert k[65]["ertragsanteil_prozent"] == 18.0        # Instructor-Spot 65→18
    assert k[80]["ertragsanteil_prozent"] == 8.0
    assert k[97]["ertragsanteil_prozent"] == 1.0


def test_9a_renten_wk_pauschbetrag_102_alle_vz():
    for vz in (2024, 2025, 2026):
        p = os.path.join(ROOT, "params", str(vz), "renten_werbungskostenpauschbetrag_p9a.yaml")
        d = yaml.safe_load(open(p, encoding="utf-8"))
        assert d["wert"]["wert"] == 102 and d["wert"]["einheit"] == "euro"
