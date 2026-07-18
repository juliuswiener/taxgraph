"""Gate für den K2-Partner-Konsistenz-Guard (produkt/konsistenz/partner_check.py). Deterministisch, NULL LLM.

Prüft: ein gesetztes Partner-Behinderungsfeld (§ 33b Person B) + veranlagung != zusammen = Widerspruch
(fail-closed, kein Partner-Pauschbetrag ohne gemeinsam veranlagten Ehegatten). Plus die Feinheiten
(unbestätigte veranlagung = noch kein Widerspruch, vorlaeufig zählt nicht, False/0 ist nicht gesetzt) +
Negativtest (der Guard feuert wirklich).
"""
from __future__ import annotations

import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
                                "produkt", "konsistenz"))
import partner_check as PC   # noqa: E402


def _snap(**felder):
    """{feld_id: (wert, zustand)} -> Snapshot-felder-Ebene."""
    return {fid: {"wert": w, "zustand": z} for fid, (w, z) in felder.items()}


# ---- der Widerspruch (Partner-Feld gesetzt, aber einzel) ---------------------

def test_gdb_partner_einzel_widerspruch():
    w = PC.partner_ohne_zusammen(_snap(rentner_grad_der_behinderung_partner=(50, "bestaetigt"),
                                       veranlagung=("einzel", "bestaetigt")))
    assert len(w) == 1 and w[0]["feld_id"] == "rentner_grad_der_behinderung_partner"


def test_hilflos_partner_einzel_widerspruch():
    w = PC.partner_ohne_zusammen(_snap(rentner_hilflos_blind_taubblind_partner=(True, "bestaetigt"),
                                       veranlagung=("einzel", "bestaetigt")))
    assert len(w) == 1 and w[0]["feld_id"] == "rentner_hilflos_blind_taubblind_partner"


def test_beide_partnerfelder_einzel_zwei_widersprueche():
    w = PC.partner_ohne_zusammen(_snap(rentner_grad_der_behinderung_partner=(60, "bestaetigt"),
                                       rentner_hilflos_blind_taubblind_partner=(True, "bestaetigt"),
                                       veranlagung=("einzel", "bestaetigt")))
    assert {x["feld_id"] for x in w} == {"rentner_grad_der_behinderung_partner",
                                         "rentner_hilflos_blind_taubblind_partner"}


# ---- die konsistente Lage ----------------------------------------------------

def test_partner_zusammen_konsistent():
    assert PC.partner_ohne_zusammen(_snap(rentner_grad_der_behinderung_partner=(50, "bestaetigt"),
                                          veranlagung=("zusammen", "bestaetigt"))) == []


# ---- fail-closed-Feinheiten --------------------------------------------------

def test_veranlagung_unbestaetigt_noch_kein_widerspruch():
    # veranlagung noch nicht bestätigt -> Unvollständigkeit, nicht Widerspruch
    assert PC.partner_ohne_zusammen(_snap(rentner_grad_der_behinderung_partner=(50, "bestaetigt"),
                                          veranlagung=("einzel", "vorlaeufig"))) == []


def test_partner_nur_vorlaeufig_zaehlt_nicht():
    # Partner-Feld nur vorlaeufig -> noch kein belegter Wert -> kein Widerspruch
    assert PC.partner_ohne_zusammen(_snap(rentner_grad_der_behinderung_partner=(50, "vorlaeufig"),
                                          veranlagung=("einzel", "bestaetigt"))) == []


def test_hilflos_false_nicht_gesetzt():
    # False (kein Merkzeichen) ist NICHT gesetzt -> kein Widerspruch bei einzel
    assert PC.partner_ohne_zusammen(_snap(rentner_hilflos_blind_taubblind_partner=(False, "bestaetigt"),
                                          veranlagung=("einzel", "bestaetigt"))) == []


# ---- Negativtest: der Guard ist nicht wirkungslos ----------------------------

def test_neg_guard_feuert_wirklich():
    ok = _snap(rentner_grad_der_behinderung_partner=(50, "bestaetigt"), veranlagung=("zusammen", "bestaetigt"))
    kaputt = _snap(rentner_grad_der_behinderung_partner=(50, "bestaetigt"), veranlagung=("einzel", "bestaetigt"))
    assert PC.partner_ohne_zusammen(ok) == [] and PC.partner_ohne_zusammen(kaputt), \
        "Guard muss den Partner-ohne-Zusammenveranlagung-Widerspruch fangen und die konsistente Lage durchlassen"


# ---- §20 Kapital Person-B (Anlage-KAP-Instanz B) ----------------------------

def test_kap_partner_einzel_widerspruch():
    w = PC.partner_ohne_zusammen(_snap(kap_kapitalertraege_partner=(500000, "bestaetigt"),
                                       veranlagung=("einzel", "bestaetigt")))
    assert len(w) == 1 and w[0]["feld_id"] == "kap_kapitalertraege_partner"


def test_kap_partner_zusammen_konsistent():
    assert PC.partner_ohne_zusammen(_snap(kap_kapitalertraege_partner=(500000, "bestaetigt"),
                                          kap_verlust_aktien_partner=(50000, "bestaetigt"),
                                          veranlagung=("zusammen", "bestaetigt"))) == []


# ---- §22 Rente Person-B (Anlage-R-Instanz B) --------------------------------

def test_renten_partner_einzel_widerspruch():
    w = PC.partner_ohne_zusammen(_snap(rentner_jahresrente_partner=(1800000, "bestaetigt"),
                                       veranlagung=("einzel", "bestaetigt")))
    assert len(w) == 1 and w[0]["feld_id"] == "rentner_jahresrente_partner"


def test_renten_partner_zusammen_konsistent():
    assert PC.partner_ohne_zusammen(_snap(rentner_jahresrente_partner=(1800000, "bestaetigt"),
                                          veranlagung=("zusammen", "bestaetigt"))) == []
