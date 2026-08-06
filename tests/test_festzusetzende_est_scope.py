"""Test: _bescheid_fn-Zweig 'festzusetzende_est' wird AUSGEFÜHRT (nicht nur gebaut).
NameError: _kind_kv_pv_summe war außerhalb des Scopes definiert, Slot starb beim Aufruf.
Dieser Test zwingt den Zweig zur Ausführung — rote beim Ausbau des Fixes.
NULL LLM.
"""
import os
import sys
import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
for sub in ("produkt/haut", "golden", "produkt/unsicherheit", "produkt/store",
            "produkt/traverser", "produkt/mapping"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import api as API
import traverser as TR


def _catala_da() -> bool:
    try:
        import runner  # noqa: F401
        return True
    except Exception:
        return False


def test_festzusetzende_est_wird_ausgefuehrt():
    """_bescheid_fn('festzusetzende_est', ...) mit Werten -> Ergebnis 1024500 ct.
    Bricht mit NameError, wenn _kind_kv_pv_summe im Scope fehlt (2026-08-06 Befund)."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")

    bindung = TR.lade_bindung()
    felder = {
        'bruttoarbeitslohn': {'wert': 5000000, 'zustand': 'bestaetigt'},
        'veranlagung': {'wert': 'einzel', 'zustand': 'bestaetigt'},
        'ep_arbeitstage': {'wert': 0, 'zustand': 'bestaetigt'},
        'ep_entfernung_km': {'wert': 0, 'zustand': 'bestaetigt'},
        'ep_oepnv_kosten': {'wert': 0, 'zustand': 'bestaetigt'},
        'ep_eigenes_kfz': {'wert': False, 'zustand': 'bestaetigt'},
        'vor_an_anteil_rv': {'wert': 0, 'zustand': 'bestaetigt'},
        'vor_ag_anteil_rv': {'wert': 0, 'zustand': 'bestaetigt'},
        'vor_rv_ausserhalb_lstb': {'wert': 0, 'zustand': 'bestaetigt'},
        'basis_kv': {'wert': 0, 'zustand': 'bestaetigt'},
        'basis_pv': {'wert': 0, 'zustand': 'bestaetigt'},
        'versicherungsart': {'wert': 'gesetzlich_an', 'zustand': 'bestaetigt'},
        'mit_anspruch_auf_zuschuss': {'wert': False, 'zustand': 'bestaetigt'},
        'vorsorge_arbeitslosenversicherung': {'wert': 0, 'zustand': 'bestaetigt'},
        'vorsorge_erwerbsunfaehigkeit': {'wert': 0, 'zustand': 'bestaetigt'},
        'vorsorge_unfall_haftpflicht': {'wert': 0, 'zustand': 'bestaetigt'},
        'vorsorge_rv_alt_mit_ueberschuss': {'wert': 0, 'zustand': 'bestaetigt'},
        'vorsorge_rv_alt_ohne_ueberschuss': {'wert': 0, 'zustand': 'bestaetigt'},
        'fam_anzahl_kinder': {'wert': 0, 'zustand': 'bestaetigt'},
        'verlustvortrag_bestand': {'wert': 0, 'zustand': 'bestaetigt'},
        'dhf_im_inland': {'wert': True, 'zustand': 'bestaetigt'},
        'dhf_monate': {'wert': 0, 'zustand': 'bestaetigt'},
        'dhf_unterkunftskosten': {'wert': 0, 'zustand': 'bestaetigt'},
        'dhf_zweitmietvertrag': {'wert': False, 'zustand': 'bestaetigt'},
        'dhf_eigener_haushalt': {'wert': False, 'zustand': 'bestaetigt'},
        'dhf_keine_berufliche_veranlassung': {'wert': False, 'zustand': 'bestaetigt'},
        'vpf_tage_ganz': {'wert': 0, 'zustand': 'bestaetigt'},
        'vpf_tage_abwesend': {'wert': 0, 'zustand': 'bestaetigt'},
        'vpf_tage_abwesend_14h': {'wert': 0, 'zustand': 'bestaetigt'},
        'vpf_tage_abwesend_8h': {'wert': 0, 'zustand': 'bestaetigt'},
        'vpf_monate_am_ort': {'wert': 0, 'zustand': 'bestaetigt'},
    }

    slot_fn = API._bescheid_fn('festzusetzende_est', 2025, bindung, felder, store=None, nur_bestaetigt=True)
    assert slot_fn is not None, "bescheid_fn gab None (catala/cases?)"

    result = slot_fn({
        'bruttoarbeitslohn': 5000000, 'veranlagung': 'einzel',
        'ep_arbeitstage': 0, 'ep_entfernung_km': 0, 'ep_oepnv_kosten': 0, 'ep_eigenes_kfz': False})
    assert result == 1024500, f"festzusetzende_est(50000€, einzel, keine Vorsorge): {result} ct, erwartet 1024500 ct"