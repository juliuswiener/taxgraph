"""Gate für den Preflight-Orchestrator (produkt/konsistenz/preflight.py). Deterministisch, NULL LLM.

Prüft: RED bei harten Widersprüchen, AMBER bei nur soft warnings, GREEN bei sauberem Snapshot.
"""
from __future__ import annotations

import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
                                "produkt", "konsistenz"))
import preflight   # noqa: E402


def _snap(**felder):
    """{feld_id: (wert, zustand)} -> Snapshot-felder-Ebene."""
    return {fid: {"wert": w, "zustand": z} for fid, (w, z) in felder.items()}


def test_preflight_green():
    """Keine Widersprüche, keine vergessenen Pauschalen -> GREEN."""
    erg = preflight.preflight(_snap())
    assert erg["status"] == "GREEN"
    assert erg["widersprueche_flag"] == []
    assert erg["widersprueche_partner"] == []
    assert erg["widersprueche_alleinerziehend"] == []
    assert erg["hinweise_pauschalen"] == []


def test_preflight_red_flag():
    """Flag-Widerspruch -> RED."""
    erg = preflight.preflight(_snap(kein_vuv=(True, "bestaetigt"), vv_einnahmen=(1200000, "bestaetigt")))
    assert erg["status"] == "RED"
    assert len(erg["widersprueche_flag"]) >= 1


def test_preflight_red_partner():
    """Partner-ohne-Zusammen -> RED."""
    erg = preflight.preflight(_snap(rentner_grad_der_behinderung_partner=(50, "bestaetigt"),
                                     veranlagung=("einzel", "bestaetigt")))
    assert erg["status"] == "RED"
    assert len(erg["widersprueche_partner"]) >= 1


def test_preflight_red_alleinerziehend():
    """Alleinerziehend+Zusammen -> RED."""
    erg = preflight.preflight(_snap(fam_alleinstehend=(True, "bestaetigt"),
                                     veranlagung=("zusammen", "bestaetigt")))
    assert erg["status"] == "RED"
    assert len(erg["widersprueche_alleinerziehend"]) >= 1


def test_preflight_amber():
    """Keine harten Widersprüche, aber vergessene Pauschale -> AMBER."""
    erg = preflight.preflight(_snap(bruttoarbeitslohn=(4000000, "bestaetigt")))
    assert erg["status"] == "AMBER"
    assert erg["widersprueche_flag"] == []
    assert erg["widersprueche_partner"] == []
    assert erg["widersprueche_alleinerziehend"] == []
    assert len(erg["hinweise_pauschalen"]) >= 1


def test_preflight_red_beats_amber():
    """Harte Widersprüche + Pauschal-Hinweise -> RED (nicht AMBER)."""
    erg = preflight.preflight(_snap(kein_vuv=(True, "bestaetigt"),
                                     vv_einnahmen=(1200000, "bestaetigt"),
                                     bruttoarbeitslohn=(4000000, "bestaetigt")))
    assert erg["status"] == "RED", "Harte Widersprüche dominieren AMBER"
    assert len(erg["widersprueche_flag"]) >= 1
    assert len(erg["hinweise_pauschalen"]) >= 1


def test_preflight_green_mit_pauschal_gefuellt():
    """Alle Checks sauber wenn Pauschal-Felder gesetzt -> GREEN."""
    erg = preflight.preflight(_snap(kap_kapitalertraege=(300000, "bestaetigt"),
                                     veranlagung=("zusammen", "bestaetigt"),
                                     bruttoarbeitslohn=(4000000, "bestaetigt"),
                                     ep_arbeitstage=(220, "bestaetigt")))
    assert erg["status"] == "GREEN"


def test_preflight_ergebnis_struktur():
    """Ergebnis-Dict hat alle erwarteten Schlüssel."""
    erg = preflight.preflight(_snap())
    assert set(erg.keys()) == {"widersprueche_flag", "widersprueche_partner",
                                "widersprueche_alleinerziehend", "hinweise_pauschalen", "status"}
