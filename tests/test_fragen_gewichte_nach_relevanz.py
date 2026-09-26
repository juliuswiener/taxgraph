"""Zweite Flaeche desselben Fixes (Vault backlog/taxgraph/intervall-ignoriert-relevanz, Weg 5).

test_intervall_relevanz_naht.py misst die Kopfzeilen-Zahl ueber API.stand(). Diese Datei misst die
andere Aufrufstelle derselben Funktion: _gesamt_beitrag() reicht IV.intervall() dieselbe Bindung und
baut aus deren beitraegen die Frage-Gewichte, mit denen naechste_fragen() die Queue sortiert.

Ohne den store an dieser Stelle ist ein Feld der abbestellten Regel nicht_fixierbar. intervall.py
bricht dann bei `if nicht_fix` VOR jedem beitrag ab — alle Gewichte waeren leer, und der Nutzer
bekaeme die wichtigen Fragen spaeter gestellt.

Der Test erreicht die Aufrufstelle direkt, weil /stand sie nicht beruehrt: nur /fragen geht ueber
_gesamt_beitrag. Ein Kopfzeilen-Test kann sie deshalb nicht abdecken.

NULL LLM.
"""
from __future__ import annotations

import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/haut", "golden", "produkt/unsicherheit", "produkt/store",
            "produkt/traverser", "produkt/mapping"):
    sys.path.insert(0, os.path.join(ROOT, sub))
sys.path.insert(0, HERE)

import api as API              # noqa: E402
import audit                   # noqa: E402
import store as ST             # noqa: E402
import traverser as TR         # noqa: E402

from test_bescheid_fn_collector import AN_KEGEL               # noqa: E402
from test_checkest_durchstich import _b, _H, TS               # noqa: E402

# Das Gate, das p9_1_3_nr5_doppelte_haushaltsfuehrung ausschliesst.
GATE = "dhf_beruflich_veranlasst"

# Diese vier Felder bleiben vorlaeufig statt bestaetigt: der einzige Weg, auf dem IV.intervall()
# ueberhaupt eine Spanne rechnet und damit ein Gewicht > 0 entsteht. Nur ep_arbeitstage ist
# beschraenkt (bereich 0..366), ep_eigenes_kfz ist bool — beide liefern echte Extremwerte.
VORLAEUFIG = {"ep_arbeitstage": 220, "ep_entfernung_km": 100, "ep_oepnv_kosten": 0,
              "ep_eigenes_kfz": True}


def _catala_da() -> bool:
    try:
        import runner  # noqa: F401
        return True
    except Exception:
        return False


@pytest.fixture(autouse=True)
def _isoliert(tmp_path, monkeypatch):
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))


def _fall(fid: str) -> str:
    """an_gesamt-Kegel 2025: dhf_* abbestellt (Gate bestaetigt-False), die vier ep_* vorlaeufig."""
    API.fall_anlegen({"fall_id": fid, "scheibe": "an_gesamt", "veranlagungszeitraum": 2025})
    store = API.lade_fall(fid)
    for feld, wert in AN_KEGEL:
        if feld.startswith("dhf_") and feld != GATE:
            continue
        if feld in VORLAEUFIG:
            ST.append_event(store=store, feld_id=feld, wert=VORLAEUFIG[feld], zustand="vorlaeufig",
                            herkunft=_H, schreiber="ui:laie",
                            signal={"signal_1": None, "signal_2": f"ok@{feld}"}, ts=TS)
        else:
            _b(store, feld, wert)
    API.speichere_fall(fid, store)
    return fid


def test_abbestellte_regel_laesst_die_gewichte_nicht_leer():
    """Kern: mit dem Filter liefert _gesamt_beitrag() Gewichte > 0 fuer die pendler-relevanten Felder.
    Ohne ihn waeren es keine — ein einziges nicht_fixierbar aus der abbestellten Regel bricht
    intervall.py vor jedem beitrag ab."""
    if not _catala_da():
        pytest.skip("catala nicht verfuegbar")
    fid = _fall("gew_relevanz")
    store = API.lade_fall(fid)
    cfg = API._cfg(store)
    bindung = API._scheibe_bindung(store)
    felder, sid = ST.materialisiere(store)
    beitrag = API._gesamt_beitrag(store, cfg, bindung, felder, sid, 2025)

    assert beitrag, (
        "Keine Gewichte: dhf_beruflich_veranlasst=False schliesst die doppelte Haushaltsfuehrung aus, "
        "ihre Felder sind aber noch in der Bindung an IV.intervall() — eines davon nicht_fixierbar "
        "genuegt, und intervall.py bricht vor jedem beitrag ab.")
    assert beitrag.get("ep_arbeitstage", 0) > 0, beitrag
    assert beitrag.get("ep_eigenes_kfz", 0) > 0, beitrag


def test_gewichte_ziehen_die_fragen_nach_vorn():
    """Und sie werden benutzt: dieselbe Queue, einmal mit den Gewichten und einmal ohne, ist
    verschieden. Sonst waere der Fix folgenlos."""
    if not _catala_da():
        pytest.skip("catala nicht verfuegbar")
    fid = _fall("gew_reihenfolge")
    store = API.lade_fall(fid)
    cfg = API._cfg(store)
    bindung = API._scheibe_bindung(store)
    felder, sid = ST.materialisiere(store)
    beitrag = API._gesamt_beitrag(store, cfg, bindung, felder, sid, 2025)
    assert beitrag, "Vorbedingung: ohne Gewichte misst dieser Test nichts"

    mit = TR.naechste_fragen(store, bindung, beitrag)
    ohne = TR.naechste_fragen(store, bindung, None)
    assert mit != ohne, (
        "Die Gewichte aendern die Reihenfolge nicht — dann liest sie niemand. "
        f"mit={mit[:8]} ohne={ohne[:8]}")
