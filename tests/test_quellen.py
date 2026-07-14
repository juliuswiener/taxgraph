"""Tests der Quellen-Typ-Hinweise (Charge 14: verwaltung-Nachrang gehaertet).

Der verwaltung-Hinweis wurde um den Nachrang-Satz "geht dem Gesetzeswortlaut aber
NICHT vor" ergaenzt (Instructor-Ruling 2026-07-14). Auflage: der Prompt reiner
gesetz-Regeln bleibt byte-identisch - der Hinweis wird NUR je verwendeter Quelle
injiziert. Diese Tests pinnen den gesetz-Hinweis byte-genau und beweisen die
Per-Quelle-Injektion. Kein Netz, kein Modell.
"""

from __future__ import annotations

import os
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
sys.path.insert(0, ROOT)
sys.path.insert(0, os.path.join(ROOT, "pipeline"))

import quellen as Q   # noqa: E402

NACHRANG = "geht dem Gesetzeswortlaut aber NICHT vor"


def test_gesetz_hinweis_byte_unveraendert():
    # Der gesetz-Hinweis darf sich NICHT aendern (Auflage: reine gesetz-Prompts
    # byte-identisch). Aendert jemand diesen String, driften alle gesetz-Regeln.
    assert Q._TYP_HINWEIS["gesetz"] == "Gesetzestext. Massgeblich."


def test_rang_verwaltung_nachrangig():
    assert Q._RANG == {"gesetz": 0, "rechtsprechung": 1, "verwaltung": 2}


def test_verwaltung_hinweis_nachrang_gehaertet():
    assert NACHRANG in Q._TYP_HINWEIS["verwaltung"]


def _rule(quellen):
    return {"rule_id": "t", "quellen": quellen}


def test_verwaltung_hinweis_nur_bei_verwaltungsquelle(tmp_path):
    (tmp_path / "g.txt").write_text("Der Gesetzestext sagt X.\n", encoding="utf-8")
    (tmp_path / "v.txt").write_text("Die Verwaltung sagt Y.\n", encoding="utf-8")

    # gesetz-only -> Nachrang-Satz NICHT im Prompt (Injektion nur je verwendeter Quelle)
    nt_g, _ = Q.build_norm_text(
        _rule([{"typ": "gesetz", "datei": "g.txt", "auszug": "Der Gesetzestext sagt X."}]),
        str(tmp_path))
    assert NACHRANG not in nt_g

    # verwaltung dabei -> Nachrang-Satz erscheint (und nur dann)
    nt_v, _ = Q.build_norm_text(
        _rule([{"typ": "gesetz", "datei": "g.txt", "auszug": "Der Gesetzestext sagt X."},
                {"typ": "verwaltung", "datei": "v.txt", "auszug": "Die Verwaltung sagt Y."}]),
        str(tmp_path))
    assert NACHRANG in nt_v
    # gesetz-Block bleibt unveraendert enthalten
    assert "Gesetzestext. Massgeblich." in nt_v
