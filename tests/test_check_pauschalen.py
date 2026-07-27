"""Gate für den Pauschal-Hinweis-Guard (produkt/konsistenz/check_pauschalen.py). Deterministisch, NULL LLM.

Prüft: Einkunfts-Quelle aktiv (bestätigt > 0) + zugehöriges Pauschal-Feld fehlt = Hinweis (soft).
Negativtests: kein Auslöser = kein Hinweis, Pauschal-Feld gesetzt = kein Hinweis.
"""
from __future__ import annotations

import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
                                "produkt", "konsistenz"))
import check_pauschalen as CP   # noqa: E402


def _snap(**felder):
    """{feld_id: (wert, zustand)} -> Snapshot-felder-Ebene."""
    return {fid: {"wert": w, "zustand": z} for fid, (w, z) in felder.items()}


# ---- Sparer-PB ----------------------------------------------------------------

def test_sparer_pb_hinweis_kapitalertraege():
    h = CP.pauschal_hinweise(_snap(kap_kapitalertraege=(300000, "bestaetigt")))
    ids = {x["check_id"] for x in h}
    assert "sparer_pb" in ids


def test_sparer_pb_hinweis_gewinn_aktien():
    h = CP.pauschal_hinweise(_snap(kap_gewinn_aktien=(100000, "bestaetigt")))
    ids = {x["check_id"] for x in h}
    assert "sparer_pb" in ids


def test_sparer_pb_hinweis_hat_hinweistext():
    h = CP.pauschal_hinweise(_snap(kap_kapitalertraege=(300000, "bestaetigt")))
    assert len(h) == 1
    assert "Sparer-Pauschbetrag" in h[0]["hinweis"]
    assert h[0]["ausloeser_felder"][0]["feld_id"] == "kap_kapitalertraege"
    assert h[0]["fehlende_felder"] == ["kap_zusammenveranlagung"]


def test_sparer_pb_konsistent_kein_kapital():
    # keine Kapitaleinkünfte -> kein Hinweis
    assert CP.pauschal_hinweise(_snap()) == []


def test_sparer_pb_konsistent_zusammen_gesetzt():
    # Kapital vorhanden + Zusammenveranlagung bestätigt -> kein Hinweis mehr
    assert CP.pauschal_hinweise(_snap(kap_kapitalertraege=(300000, "bestaetigt"),
                                       kap_zusammenveranlagung=(True, "bestaetigt"))) == []


def test_sparer_pb_konsistent_nur_vorlaeufig():
    # Kapital nur vorlaeufig -> kein sicherer Auslöser -> kein Hinweis
    assert CP.pauschal_hinweise(_snap(kap_kapitalertraege=(300000, "vorlaeufig"))) == []


def test_sparer_pb_kapital_null_kein_hinweis():
    # Kapital 0 -> kein Auslöser -> kein Hinweis
    assert CP.pauschal_hinweise(_snap(kap_kapitalertraege=(0, "bestaetigt"))) == []


# ---- EP-Arbeitstage -----------------------------------------------------------

def test_ep_arbeitstage_hinweis():
    h = CP.pauschal_hinweise(_snap(bruttoarbeitslohn=(4000000, "bestaetigt")))
    ids = {x["check_id"] for x in h}
    assert "ep_arbeitstage" in ids


def test_ep_arbeitstage_hinweis_text():
    h = CP.pauschal_hinweise(_snap(bruttoarbeitslohn=(4000000, "bestaetigt")))
    assert len(h) == 1
    assert "Entfernungspauschale" in h[0]["hinweis"]
    assert h[0]["fehlende_felder"] == ["ep_arbeitstage"]


def test_ep_arbeitstage_konsistent_arbeitstage_gesetzt():
    assert CP.pauschal_hinweise(_snap(bruttoarbeitslohn=(4000000, "bestaetigt"),
                                       ep_arbeitstage=(220, "bestaetigt"))) == []


def test_ep_arbeitstage_kein_lohn_kein_hinweis():
    assert CP.pauschal_hinweise(_snap()) == []


def test_ep_arbeitstage_lohn_vorlaeufig():
    assert CP.pauschal_hinweise(_snap(bruttoarbeitslohn=(4000000, "vorlaeufig"))) == []


def test_ep_arbeitstage_lohn_null():
    assert CP.pauschal_hinweise(_snap(bruttoarbeitslohn=(0, "bestaetigt"))) == []


# ---- V+V-Werbungskosten -------------------------------------------------------

def test_vv_wk_hinweis():
    h = CP.pauschal_hinweise(_snap(vv_einnahmen=(1200000, "bestaetigt")))
    ids = {x["check_id"] for x in h}
    assert "vv_wk" in ids


def test_vv_wk_hinweis_text():
    h = CP.pauschal_hinweise(_snap(vv_einnahmen=(1200000, "bestaetigt")))
    assert len(h) == 1
    assert "Werbungskosten bei Vermietung" in h[0]["label"]
    assert "vv_schuldzinsen" in h[0]["fehlende_felder"]
    assert "vv_erhaltungsaufwand" in h[0]["fehlende_felder"]
    assert "vv_sonstige_wk" in h[0]["fehlende_felder"]


def test_vv_wk_ein_wk_feld_reicht():
    # Mindestens ein WK-Feld gesetzt -> kein Hinweis (das schützt vor False-Positives
    # wenn der Nutzer z.B. nur Schuldzinsen hat aber keine Erhaltungsaufwendungen).
    assert CP.pauschal_hinweise(_snap(vv_einnahmen=(1200000, "bestaetigt"),
                                       vv_schuldzinsen=(200000, "bestaetigt"))) == []


def test_vv_wk_alle_drei_leer_hinweis():
    h = CP.pauschal_hinweise(_snap(vv_einnahmen=(1200000, "bestaetigt"),
                                    vv_schuldzinsen=(0, "bestaetigt"),
                                    vv_erhaltungsaufwand=(0, "bestaetigt"),
                                    vv_sonstige_wk=(0, "bestaetigt")))
    assert len(h) == 1 and h[0]["check_id"] == "vv_wk"


def test_vv_wk_keine_einnahmen():
    assert CP.pauschal_hinweise(_snap(vv_schuldzinsen=(0, "bestaetigt"))) == []


def test_vv_wk_einnahmen_vorlaeufig():
    assert CP.pauschal_hinweise(_snap(vv_einnahmen=(1200000, "vorlaeufig"))) == []


# ---- Gemischter Snapshot ------------------------------------------------------

def test_gemischter_snapshot_alle_hinweise():
    snap = _snap(
        kap_kapitalertraege=(500000, "bestaetigt"),
        bruttoarbeitslohn=(4000000, "bestaetigt"),
        vv_einnahmen=(1200000, "bestaetigt"),
    )
    h = CP.pauschal_hinweise(snap)
    ids = {x["check_id"] for x in h}
    assert ids == {"sparer_pb", "ep_arbeitstage", "vv_wk"}


def test_gemischter_snapshot_teilweise_gefuellt():
    snap = _snap(
        kap_kapitalertraege=(500000, "bestaetigt"),
        kap_zusammenveranlagung=(True, "bestaetigt"),   # Sparer-PB ok
        bruttoarbeitslohn=(4000000, "bestaetigt"),       # EP fehlt
        vv_einnahmen=(1200000, "bestaetigt"),
        vv_schuldzinsen=(200000, "bestaetigt"),          # V+V ok
    )
    h = CP.pauschal_hinweise(snap)
    ids = {x["check_id"] for x in h}
    assert ids == {"ep_arbeitstage"}


# ---- Negativtest: Guard feuert nicht grundlos ---------------------------------

def test_neg_guard_feuert_nur_bei_ausloeser():
    leer = _snap()
    assert CP.pauschal_hinweise(leer) == [], \
        "Ohne Einkunfts-Quelle darf kein Pauschal-Hinweis erscheinen"
