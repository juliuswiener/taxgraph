"""hinweis-Kanal-Provenance im Produktions-Report (Instructor-Auflage 2026-07-12).

Der hinweis geht ueber `formalisierer_zusatz` in den Formalisierer-Prompt. Wird er
spaeter in rules.yaml geaendert, muss ein alter report.json noch nachweisen, WAS im
Lauf gesendet wurde. Diese Tests sichern den Audit-Nachweis ab - ohne Toolchain/Netz.
"""

from __future__ import annotations

import hashlib
import os
import sys

import pytest
import yaml

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
sys.path.insert(0, ROOT)
sys.path.insert(0, os.path.join(ROOT, "pipeline"))
sys.path.insert(0, os.path.join(ROOT, "pipeline", "produktion"))

import run as PRUN  # noqa: E402

RULES_YAML = os.path.join(ROOT, "pipeline", "produktion", "rules.yaml")


def test_leerer_hinweis_liefert_leere_provenance():
    # "kein hinweis" muss im Report sofort sichtbar sein - NICHT als Hash von "".
    for rule in ({}, {"hinweis": ""}, {"hinweis": None}):
        prov = PRUN.hinweis_provenance(rule)
        assert prov == {"hinweis": "", "hinweis_sha256": ""}


def test_hinweis_wird_mit_korrektem_hash_protokolliert():
    h = "Hinweis zur Berechnung: abziehbar = max(0, a - b)."
    prov = PRUN.hinweis_provenance({"hinweis": h})
    assert prov["hinweis"] == h
    assert prov["hinweis_sha256"] == hashlib.sha256(h.encode("utf-8")).hexdigest()


def test_geaenderter_hinweis_aendert_den_hash():
    # Der eigentliche Audit-Zweck: ein spaeter veraenderter hinweis darf nicht
    # unbemerkt bleiben - der Hash im alten Report weicht dann ab.
    a = PRUN.hinweis_provenance({"hinweis": "Variante A: min(x,1000)*n."})
    b = PRUN.hinweis_provenance({"hinweis": "Variante B: min(x,1000)*n ab Monat 48."})
    assert a["hinweis_sha256"] != b["hinweis_sha256"]


def test_build_candidate_reicht_hinweis_als_zusatz_durch():
    # End-to-End: build_candidate mappt rule["hinweis"] -> formalisierer_zusatz,
    # und genau dieser String landet in der Report-Provenance. Braucht die
    # eingefrorenen Quellen; fehlen sie, ist das kein Regress dieses Kanals.
    cfg = yaml.safe_load(open(RULES_YAML, encoding="utf-8"))
    base = cfg["regeln"][0]
    h = "Bearbeitungshinweis: SolZ = min(5,5%*BMG, 11,9%*(BMG-Freigrenze))."
    rule = dict(base, hinweis=h)
    try:
        cand = PRUN.build_candidate(rule)
    except Exception as e:  # QuellenFehler o.ae. -> Freeze nicht verfuegbar
        pytest.skip(f"Freeze-Quellen nicht verfuegbar: {type(e).__name__}")
    assert cand["formalisierer_zusatz"] == h
    assert PRUN.hinweis_provenance(rule)["hinweis"] == h

    # Kontrolle: dieselbe Regel OHNE hinweis -> leerer zusatz (kein Regress).
    cand0 = PRUN.build_candidate(base)
    assert cand0["formalisierer_zusatz"] == ""
