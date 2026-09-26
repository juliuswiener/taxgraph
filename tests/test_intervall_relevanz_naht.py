"""Die Kopfzeile (/stand → intervall) muss traverser.relevanz() respektieren wie /ergebnis
(Vault backlog/taxgraph/intervall-ignoriert-relevanz, Weg 5).

Befund, an 49d18e5 noch rot: wer dhf_beruflich_veranlasst=False bestätigt, schließt die doppelte
Haushaltsführung aus. Der Traverser fragt deren Felder folgerichtig nie, /ergebnis rechnet ohne sie
(_relevante_kegel_felder) — aber IV.intervall() bekam über _ring_bindung() den vollen Kegel und
führte die vier nie gefragten dhf_*-Felder als nicht_fixierbar. Folge: /ergebnis nennt eine
bestätigte Zahl, die Kopfzeile daneben „Noch keine Zahl" (min/max None).

Zwei Richtungen, in die der Fix falsch laufen kann:
  1. er wirkt nicht               -> test_abbestellte_regel_haelt_das_intervall_nicht_offen
  2. er wirkt zu früh (fail-open) -> test_unbeantwortetes_gate_haelt_das_intervall_offen

Der tragende Test ist test_abbestellte_regel_haelt_das_intervall_nicht_offen. Er prüft nur die
Kopfzeilen-Zahl, nicht den Ring, und enthält den Normalfall: bei Gate „ja" bleibt die Regel
relevant, ihre unbeantworteten Felder fehlen weiter — der Fix darf dort nichts ändern.

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

from test_bescheid_fn_collector import AN_KEGEL   # noqa: E402
from test_checkest_durchstich import _b           # noqa: E402

# Das Gate, das p9_1_3_nr5_doppelte_haushaltsfuehrung ausschließt. Alle übrigen dhf_*-Felder bleiben
# ungesetzt — genau die Felder, die der Nutzer nach „nein" nie zu sehen bekommt.
GATE = "dhf_beruflich_veranlasst"


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


def _fall(fid: str, gate: bool | None) -> str:
    """an_gesamt-Kegel ohne dhf_*. `gate`: der Wert des Gates dhf_beruflich_veranlasst — True oder
    False bestätigt (Regel bleibt relevant / wird ausgeschlossen), None = gar nicht beantwortet."""
    API.fall_anlegen({"fall_id": fid, "scheibe": "an_gesamt", "veranlagungszeitraum": 2025})
    store = API.lade_fall(fid)
    for feld, wert in AN_KEGEL:
        if feld.startswith("dhf_") and feld != GATE:
            continue
        if feld == GATE:
            if gate is None:
                continue
            wert = gate
        _b(store, feld, wert)
    API.speichere_fall(fid, store)
    return fid


def test_abbestellte_regel_haelt_das_intervall_nicht_offen():
    """Der tragende Test: Gate bestätigt „nein" -> die Kopfzeile nennt die bestätigte Zahl, statt zu
    verschwinden. Normalfall (Gate „ja") steht mit drin: dort bleibt die Regel relevant und der Fix
    darf nichts ändern."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")

    # 1. Gate „nein": die dhf-Regel ist abbestellt, ihre Felder fragt der Traverser nie.
    fid = _fall("iv_relevanz_aus", gate=False)
    _, erg = API.ergebnis(fid)
    assert erg["grund"] == "bestaetigt" and erg["zahl_cent"], (
        f"Vorbedingung: /ergebnis muss eine bestätigte Zahl liefern, sonst misst der Test nichts: {erg}")
    iv = API.stand(fid)[1]["intervall"]
    assert iv["nicht_fixierbar"] == [], (
        f"{GATE}=False schließt die doppelte Haushaltsführung aus, trotzdem hält die Kopfzeile "
        f"{iv['nicht_fixierbar']} offen — Felder, die der Nutzer nie zu sehen bekommt. "
        f"Sie zeigt damit 'Noch keine Zahl' neben der bestätigten Zahl {erg['zahl_cent']} von /ergebnis.")
    assert iv["min_cent"] == iv["max_cent"] == erg["zahl_cent"], (
        f"Kopfzeile {iv['min_cent']}–{iv['max_cent']} neben bestätigter Zahl {erg['zahl_cent']}")

    # 2. Normalfall Gate „ja": die Regel bleibt relevant, ihre unbeantworteten Felder halten das
    #    Intervall weiter offen — hier muss alles so bleiben, wie es ohne den Fix war.
    fid_ja = _fall("iv_relevanz_ja", gate=True)
    iv_ja = API.stand(fid_ja)[1]["intervall"]
    assert {"dhf_unterkunftskosten_monat", "dhf_eigener_hausstand"} <= set(iv_ja["nicht_fixierbar"]), (
        f"{GATE}=True bestellt die doppelte Haushaltsführung NICHT ab; ihre offenen Felder müssen das "
        f"Intervall weiter offen halten, sonst zeigt die Kopfzeile eine Zahl vor der Frage: {iv_ja}")
    assert iv_ja["min_cent"] is None and iv_ja["max_cent"] is None, iv_ja


def test_unbeantwortetes_gate_haelt_das_intervall_offen():
    """Gegenrichtung (fail-closed): ohne Antwort auf das Gate ist die Regel NICHT ausgeschlossen. Ihre
    offenen Felder müssen das Intervall offen halten — sonst zeigte die Kopfzeile eine Zahl, bevor
    der Nutzer gefragt wurde. relevanz() schließt nur bei bestätigtem False aus."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    fid = _fall("iv_relevanz_offen", gate=None)
    iv = API.stand(fid)[1]["intervall"]
    assert {GATE, "dhf_eigener_hausstand"} <= set(iv["nicht_fixierbar"]), iv["nicht_fixierbar"]
    assert iv["min_cent"] is None and iv["max_cent"] is None, iv
