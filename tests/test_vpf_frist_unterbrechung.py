"""
Tests für vpf_frist_unterbrochen Sperrgrund-Logik.

§ 9 Abs. 4a S. 6-7: Verpflegungspauschale auf 3 Monate beschränkt, außer wenn
Unterbrechung ≥4 Wochen Frist neu setzt.

Sperrgrund verpflegung_dreimonatsfrist_unterbrechung_offen:
- Bedingung: monate > 3 + Tage_gesamt > 0 + Tage_nach_frist_gesamt == 0 + Frage unbeantwortet
- Effekt: _an_gesamt_sperrgrund() gibt Sperrgrund-Name zurück (not None)
"""
from __future__ import annotations

import sys
import os

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "produkt/haut"))

import api


def test_vpf_frist_unterbrechung_unbeantwortet_sperrgrund():
    """
    Fall 1: monate=5, tage_24h=60, NACH_FRIST alle=0 bestaetigt, vpf_frist_unterbrochen NICHT gesetzt
    -> grund == "verpflegung_dreimonatsfrist_unterbrechung_offen"
    """
    felder = {
        "tage_24h": {"wert": 60, "zustand": "bestaetigt"},
        "tage_an_abreise": {"wert": 0, "zustand": "bestaetigt"},
        "tage_ueber_8h_eintaegig": {"wert": 0, "zustand": "bestaetigt"},
        "vpf_monate_am_ort": {"wert": 5, "zustand": "bestaetigt"},
        "vpf_keine_mahlzeitengestellung": {"wert": True, "zustand": "bestaetigt"},
        "vpf_tage_24h_nach_drei_monaten": {"wert": 0, "zustand": "bestaetigt"},
        "vpf_tage_an_abreise_nach_drei_monaten": {"wert": 0, "zustand": "bestaetigt"},
        "vpf_tage_ueber_8h_nach_drei_monaten": {"wert": 0, "zustand": "bestaetigt"},
        # vpf_frist_unterbrochen: NICHT gesetzt (zustand != "bestaetigt")
    }
    scheibe = {"guard": True}
    vz = 2025

    grund = api._an_gesamt_sperrgrund(felder, scheibe, vz, None, None)

    assert grund == "verpflegung_dreimonatsfrist_unterbrechung_offen", \
        f"Expected Sperrgrund, got grund={grund}"


def test_vpf_frist_unterbrechung_bestaetigt_rechnet():
    """
    Fall 2: monate=5, tage_24h=60, NACH_FRIST alle=0 bestaetigt, vpf_frist_unterbrochen bestaetigt=True
    -> grund is None (Ring rechnet)
    """
    felder = {
        "tage_24h": {"wert": 60, "zustand": "bestaetigt"},
        "tage_an_abreise": {"wert": 0, "zustand": "bestaetigt"},
        "tage_ueber_8h_eintaegig": {"wert": 0, "zustand": "bestaetigt"},
        "vpf_monate_am_ort": {"wert": 5, "zustand": "bestaetigt"},
        "vpf_keine_mahlzeitengestellung": {"wert": True, "zustand": "bestaetigt"},
        "vpf_tage_24h_nach_drei_monaten": {"wert": 0, "zustand": "bestaetigt"},
        "vpf_tage_an_abreise_nach_drei_monaten": {"wert": 0, "zustand": "bestaetigt"},
        "vpf_tage_ueber_8h_nach_drei_monaten": {"wert": 0, "zustand": "bestaetigt"},
        # vpf_frist_unterbrochen: BESTAETIGT
        "vpf_frist_unterbrochen": {"wert": True, "zustand": "bestaetigt"},
    }
    scheibe = {"guard": True}
    vz = 2025

    grund = api._an_gesamt_sperrgrund(felder, scheibe, vz, None, None)

    assert grund is None, \
        f"Expected no Sperrgrund (rechnet), got grund={grund}"


def test_vpf_normal_path_mit_nach_frist_nicht_blockiert():
    """
    Fall 3 (Regression): monate=5, tage_24h=60, vpf_tage_24h_nach_drei_monaten=15 bestaetigt
    -> grund is None (Sperrgrund darf NICHT feuern, normaler Weg nicht blockiert)

    Wichtigste Regression: die neue Sperrgrund-Bedingung "tage_nach_frist_gesamt == 0"
    darf NICHT feuern, wenn Nutzer die Aufteilung korrekt angegeben hat.
    """
    felder = {
        "tage_24h": {"wert": 60, "zustand": "bestaetigt"},
        "tage_an_abreise": {"wert": 0, "zustand": "bestaetigt"},
        "tage_ueber_8h_eintaegig": {"wert": 0, "zustand": "bestaetigt"},
        "vpf_monate_am_ort": {"wert": 5, "zustand": "bestaetigt"},
        "vpf_keine_mahlzeitengestellung": {"wert": True, "zustand": "bestaetigt"},
        # NACH_FRIST: 15 Tage (nicht 0) → Sperrgrund darf NICHT feuern
        "vpf_tage_24h_nach_drei_monaten": {"wert": 15, "zustand": "bestaetigt"},
        "vpf_tage_an_abreise_nach_drei_monaten": {"wert": 0, "zustand": "bestaetigt"},
        "vpf_tage_ueber_8h_nach_drei_monaten": {"wert": 0, "zustand": "bestaetigt"},
        # vpf_frist_unterbrochen: NICHT gesetzt (nicht nötig, Aufteilung ist korrekt)
    }
    scheibe = {"guard": True}
    vz = 2025

    grund = api._an_gesamt_sperrgrund(felder, scheibe, vz, None, None)

    assert grund is None, \
        f"Expected no Sperrgrund (normal path), got grund={grund}. " \
        "Sperrgrund darf NICHT feuern wenn nach_frist > 0"
