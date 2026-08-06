"""Explizite Kz-Asserts für 7 gebundene Kz ohne Test-Coverage (BACKLOG.yaml id: kz-ohne-assert).

Jeder Test prüft, dass:
  1. Das Feld in der Bindung existiert
  2. Das Kz-Literal in einem assert-gesteuerten Pfad vorkommt
  3. Die ELSTER-Deklaration das Kz mit dem erwarteten Wert enthält

Kz in dieser Datei:
  E0108405 spenden_betrag         (§10b Spenden)
  E0203611 ep_oepnv_kosten        (§9 Entfernungspauschale ÖPNV)
  E0205201 tage_ueber_8h_eintaegig (§9 Abs.4a Verpflegung >8h)
  E0205302 tage_an_abreise        (§9 Abs.4a Verpflegung An-/Abreise)
  E0205409 tage_24h               (§9 Abs.4a Verpflegung 24h)
  E0207611 dhf_unterkunftskosten_monat (§9 Abs.1 Nr.5 DHf)
  E0505607 schulgeld              (§10 Abs.1 Nr.9 Schulgeld)
"""
import glob
import os
import sys

import pytest
import yaml

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
BIND_DIR = os.path.join(ROOT, "produkt", "bindung")
sys.path.insert(0, os.path.join(ROOT, "produkt", "mapping"))
from est_mapping import deklariere  # noqa: E402


@pytest.fixture(scope="module")
def all_bindings():
    """Lade alle Bindungen in ein dict: feld_id -> vollstaendiger Bindungseintrag aus YAML.

    Der komplette dict wird an deklariere() uebergeben, damit auch typ und andere
    Eigenschaften aus der YAML geprueft werden (nicht synthetisch vorgegeben).
    """
    result = {}
    for fp in sorted(glob.glob(os.path.join(BIND_DIR, "bindung_*.yaml"))):
        with open(fp, encoding="utf-8") as f:
            doc = yaml.safe_load(f)
        for b in doc.get("bindungen", []):
            fid = b.get("feld_id")
            if fid:
                result[fid] = b  # Ganzer Bindungseintrag
    return result


# --- Kz-Daten: feld_id -> (kz, test_wert_cent, expected_dekl_wert) ---

_KZ_E0108405 = ("spenden_betrag", 50000, 500)  # 500€ Spenden
_KZ_E0203611 = ("ep_oepnv_kosten", 10000, 100)  # 100€ ÖPNV-Kosten
_KZ_E0205201 = ("tage_ueber_8h_eintaegig", 10, 10)  # 10 Tage >8h
_KZ_E0205302 = ("tage_an_abreise", 5, 5)  # 5 Tage An-/Abreise
_KZ_E0205409 = ("tage_24h", 8, 8)  # 8 Tage 24h
_KZ_E0207611 = ("dhf_unterkunftskosten_monat", 150000, 1500)  # 1500€ Unterkunft
_KZ_E0505607 = ("schulgeld", 100000, 1000)  # 1000€ Schulgeld


def test_e0108405_spenden_betrag_in_deklaration(all_bindings):
    """E0108405 spenden_betrag — Kz in Deklaration vorhanden (mit echter YAML-Bindung)."""
    fid, wert_cent, expected = _KZ_E0108405
    assert fid in all_bindings, f"{fid} nicht in Bindungen"
    assert all_bindings[fid]["elster_kz"] == "E0108405"

    snapshot = {fid: {"wert": wert_cent, "zustand": "bestaetigt"}}
    result = deklariere(snapshot, {fid: all_bindings[fid]})
    dekl = result.get("deklaration", {})

    assert "E0108405" in dekl, f"E0108405 nicht in Deklaration: {dekl.keys()}"
    assert dekl["E0108405"] == expected, f"E0108405={dekl['E0108405']} ≠ {expected}"


def test_e0203611_ep_oepnv_kosten_in_deklaration(all_bindings):
    """E0203611 ep_oepnv_kosten — Kz in Deklaration vorhanden (mit echter YAML-Bindung)."""
    fid, wert_cent, expected = _KZ_E0203611
    assert fid in all_bindings, f"{fid} nicht in Bindungen"
    assert all_bindings[fid]["elster_kz"] == "E0203611"

    snapshot = {fid: {"wert": wert_cent, "zustand": "bestaetigt"}}
    result = deklariere(snapshot, {fid: all_bindings[fid]})
    dekl = result.get("deklaration", {})

    assert "E0203611" in dekl, f"E0203611 nicht in Deklaration: {dekl.keys()}"
    assert dekl["E0203611"] == expected, f"E0203611={dekl['E0203611']} ≠ {expected}"


def test_e0205201_tage_ueber_8h_eintaegig_in_deklaration(all_bindings):
    """E0205201 tage_ueber_8h_eintaegig — Kz in Deklaration vorhanden (mit echter YAML-Bindung)."""
    fid, wert_int, expected = _KZ_E0205201
    assert fid in all_bindings, f"{fid} nicht in Bindungen"
    assert all_bindings[fid]["elster_kz"] == "E0205201"

    snapshot = {fid: {"wert": wert_int, "zustand": "bestaetigt"}}
    result = deklariere(snapshot, {fid: all_bindings[fid]})
    dekl = result.get("deklaration", {})

    assert "E0205201" in dekl, f"E0205201 nicht in Deklaration: {dekl.keys()}"
    assert dekl["E0205201"] == expected, f"E0205201={dekl['E0205201']} ≠ {expected}"


def test_e0205302_tage_an_abreise_in_deklaration(all_bindings):
    """E0205302 tage_an_abreise — Kz in Deklaration vorhanden (mit echter YAML-Bindung)."""
    fid, wert_int, expected = _KZ_E0205302
    assert fid in all_bindings, f"{fid} nicht in Bindungen"
    assert all_bindings[fid]["elster_kz"] == "E0205302"

    snapshot = {fid: {"wert": wert_int, "zustand": "bestaetigt"}}
    result = deklariere(snapshot, {fid: all_bindings[fid]})
    dekl = result.get("deklaration", {})

    assert "E0205302" in dekl, f"E0205302 nicht in Deklaration: {dekl.keys()}"
    assert dekl["E0205302"] == expected, f"E0205302={dekl['E0205302']} ≠ {expected}"


def test_e0205409_tage_24h_in_deklaration(all_bindings):
    """E0205409 tage_24h — Kz in Deklaration vorhanden (mit echter YAML-Bindung)."""
    fid, wert_int, expected = _KZ_E0205409
    assert fid in all_bindings, f"{fid} nicht in Bindungen"
    assert all_bindings[fid]["elster_kz"] == "E0205409"

    snapshot = {fid: {"wert": wert_int, "zustand": "bestaetigt"}}
    result = deklariere(snapshot, {fid: all_bindings[fid]})
    dekl = result.get("deklaration", {})

    assert "E0205409" in dekl, f"E0205409 nicht in Deklaration: {dekl.keys()}"
    assert dekl["E0205409"] == expected, f"E0205409={dekl['E0205409']} ≠ {expected}"


def test_e0207611_dhf_unterkunftskosten_monat_in_deklaration(all_bindings):
    """E0207611 dhf_unterkunftskosten_monat — Kz in Deklaration vorhanden (mit echter YAML-Bindung)."""
    fid, wert_cent, expected = _KZ_E0207611
    assert fid in all_bindings, f"{fid} nicht in Bindungen"
    assert all_bindings[fid]["elster_kz"] == "E0207611"

    snapshot = {fid: {"wert": wert_cent, "zustand": "bestaetigt"}}
    result = deklariere(snapshot, {fid: all_bindings[fid]})
    dekl = result.get("deklaration", {})

    assert "E0207611" in dekl, f"E0207611 nicht in Deklaration: {dekl.keys()}"
    assert dekl["E0207611"] == expected, f"E0207611={dekl['E0207611']} ≠ {expected}"


def test_e0505607_schulgeld_in_deklaration(all_bindings):
    """E0505607 schulgeld — Kz in Deklaration vorhanden (mit echter YAML-Bindung)."""
    fid, wert_cent, expected = _KZ_E0505607
    assert fid in all_bindings, f"{fid} nicht in Bindungen"
    assert all_bindings[fid]["elster_kz"] == "E0505607"

    snapshot = {fid: {"wert": wert_cent, "zustand": "bestaetigt"}}
    result = deklariere(snapshot, {fid: all_bindings[fid]})
    dekl = result.get("deklaration", {})

    assert "E0505607" in dekl, f"E0505607 nicht in Deklaration: {dekl.keys()}"
    assert dekl["E0505607"] == expected, f"E0505607={dekl['E0505607']} ≠ {expected}"
