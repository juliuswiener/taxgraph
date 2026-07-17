"""Gate für den K2-Flag-Konsistenz-Guard (produkt/konsistenz/flag_check.py). Deterministisch, NULL LLM.

Prüft die Abwesenheits-Flag ↔ Einkunftsart-Inversion (Front V+V): ein gesetztes kein_X-Flag + ein
belegtes Einkunfts-Feld derselben Art = Widerspruch (K2, keine still übergangene Einkunftsart). Plus
Negativtests (der Guard feuert wirklich) + vorlaeufig-zählt-nicht.
"""
from __future__ import annotations

import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
                                "produkt", "konsistenz"))
import flag_check as FC   # noqa: E402


def _snap(**felder):
    """{feld_id: (wert, zustand)} -> Snapshot-felder-Ebene."""
    return {fid: {"wert": w, "zustand": z} for fid, (w, z) in felder.items()}


# ---- V+V-Inversion (die Ring-relevante) --------------------------------------

def test_kein_vuv_widerspruch():
    w = FC.flag_widersprueche(_snap(kein_vuv=(True, "bestaetigt"), vv_einnahmen=(1200000, "bestaetigt")))
    assert len(w) == 1 and w[0]["flag"] == "kein_vuv" and w[0]["feld_id"] == "vv_einnahmen"


def test_kein_vuv_konsistent_echte_vv():
    # echter V+V-Fall: Flag korrekt false -> KEIN Widerspruch
    assert FC.flag_widersprueche(_snap(kein_vuv=(False, "bestaetigt"),
                                       vv_einnahmen=(1200000, "bestaetigt"))) == []


def test_kein_vuv_wahre_abwesenheit():
    # Flag true, keine V+V-Einnahme -> KEIN Widerspruch (reiner-AN-Fall)
    assert FC.flag_widersprueche(_snap(kein_vuv=(True, "bestaetigt"))) == []


# ---- alle 4 Arten symmetrisch ------------------------------------------------

def test_kein_kap_und_sonstige_widerspruch():
    w = FC.flag_widersprueche(_snap(kein_kap=(True, "bestaetigt"), kap_kapitalertraege=(50000, "bestaetigt"),
                                    kein_sonstige=(True, "bestaetigt"), rentner_jahresrente=(1800000, "bestaetigt")))
    flags = {x["flag"] for x in w}
    assert flags == {"kein_kap", "kein_sonstige"}


# ---- fail-closed-Feinheiten --------------------------------------------------

def test_vorlaeufig_zaehlt_nicht():
    # ein nur vorlaeufiges Einkunfts-Feld ist noch kein Beleg -> kein Widerspruch (erst bestätigt)
    assert FC.flag_widersprueche(_snap(kein_vuv=(True, "bestaetigt"),
                                       vv_einnahmen=(1200000, "vorlaeufig"))) == []


def test_flag_nur_vorlaeufig_keine_behauptung():
    # das Flag selbst nur vorlaeufig -> es behauptet noch nichts -> kein Widerspruch
    assert FC.flag_widersprueche(_snap(kein_vuv=(True, "vorlaeufig"),
                                       vv_einnahmen=(1200000, "bestaetigt"))) == []


def test_wert_null_kein_widerspruch():
    # Betrag 0 (bestätigte Null) widerspricht der Abwesenheit nicht
    assert FC.flag_widersprueche(_snap(kein_vuv=(True, "bestaetigt"), vv_einnahmen=(0, "bestaetigt"))) == []


# ---- Negativtest: der Guard ist nicht wirkungslos ----------------------------

def test_neg_guard_feuert_wirklich():
    ok = _snap(kein_vuv=(True, "bestaetigt"))                      # konsistent
    kaputt = _snap(kein_vuv=(True, "bestaetigt"), vv_einnahmen=(1, "bestaetigt"))
    assert FC.flag_widersprueche(ok) == [] and FC.flag_widersprueche(kaputt), \
        "Guard muss den Widerspruch fangen und die konsistente Lage durchlassen"
