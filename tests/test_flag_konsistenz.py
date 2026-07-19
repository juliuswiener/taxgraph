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


# ---- §§ 13-18 Gewinneinkünfte (Stufe 1): kein_gewinn ↔ einkuenfte_gewinn (vorher leer, jetzt scharf) ----

def test_kein_gewinn_widerspruch():
    # kein_gewinn=true behauptet keine Gewinneinkünfte, aber ein Gewinn ist belegt -> Widerspruch
    w = FC.flag_widersprueche(_snap(kein_gewinn=(True, "bestaetigt"), einkuenfte_gewinn=(3000000, "bestaetigt")))
    assert len(w) == 1 and w[0]["flag"] == "kein_gewinn" and w[0]["feld_id"] == "einkuenfte_gewinn"


def test_kein_gewinn_konsistent_echter_gewinn():
    # echter Gewinn-Fall: Flag korrekt false -> KEIN Widerspruch (der einkuenfte_gewinn-Slot greift)
    assert FC.flag_widersprueche(_snap(kein_gewinn=(False, "bestaetigt"),
                                       einkuenfte_gewinn=(3000000, "bestaetigt"))) == []


def test_kein_gewinn_wahre_abwesenheit():
    # Flag true, kein Gewinn belegt -> KEIN Widerspruch (reiner-AN-Fall)
    assert FC.flag_widersprueche(_snap(kein_gewinn=(True, "bestaetigt"))) == []


def test_kein_gewinn_veraeusserungsgewinn_widerspruch():
    # § 16-vg = Gewinneinkunft (§ 2 Abs. 1 Nr. 2): kein_gewinn=true + rentner_veraeusserungsgewinn>0
    # -> Widerspruch (Stufe 2-I: der §16-vg fließt jetzt live in den Ring, darf nicht still übergangen werden).
    w = FC.flag_widersprueche(_snap(kein_gewinn=(True, "bestaetigt"),
                                    rentner_veraeusserungsgewinn=(15000000, "bestaetigt")))
    assert len(w) == 1 and w[0]["flag"] == "kein_gewinn" and w[0]["feld_id"] == "rentner_veraeusserungsgewinn"


# ---- EÜR-Komponenten (§ 4 Abs. 3, Stufe 2-II): alle 3 negieren kein_gewinn ----

def test_kein_gewinn_betriebseinnahmen_widerspruch():
    # EÜR-Betrieb: kein_gewinn=true + betriebseinnahmen>0 → Widerspruch.
    w = FC.flag_widersprueche(_snap(kein_gewinn=(True, "bestaetigt"),
                                    betriebseinnahmen=(8000000, "bestaetigt")))
    assert len(w) == 1 and w[0]["flag"] == "kein_gewinn" and w[0]["feld_id"] == "betriebseinnahmen"


def test_kein_gewinn_verlustjahr_afa_widerspruch():
    # Verlustjahr: einnahmen=0, aber afa>0 → Betrieb existiert → kein_gewinn=true widersprüchlich
    # (Instructor-K2-Vollständigkeit: betriebseinnahmen-only ließe das durchrutschen). betriebseinnahmen=0
    # ist kein Beleg (Wert 0), afa_jahresbetrag>0 triggert allein.
    w = FC.flag_widersprueche(_snap(kein_gewinn=(True, "bestaetigt"),
                                    betriebseinnahmen=(0, "bestaetigt"),
                                    afa_jahresbetrag=(500000, "bestaetigt")))
    assert len(w) == 1 and w[0]["feld_id"] == "afa_jahresbetrag"


def test_kein_gewinn_sonstige_ba_widerspruch():
    # Nur laufende Ausgaben (einnahmen/afa=0) → Betrieb existiert → Widerspruch.
    w = FC.flag_widersprueche(_snap(kein_gewinn=(True, "bestaetigt"),
                                    sonstige_betriebsausgaben=(3000000, "bestaetigt")))
    assert len(w) == 1 and w[0]["feld_id"] == "sonstige_betriebsausgaben"


def test_kein_gewinn_euer_konsistent():
    # echter EÜR-Betrieb: Flag korrekt false → KEIN Widerspruch (der EÜR-Ring greift).
    assert FC.flag_widersprueche(_snap(kein_gewinn=(False, "bestaetigt"),
                                       betriebseinnahmen=(8000000, "bestaetigt"),
                                       sonstige_betriebsausgaben=(3000000, "bestaetigt"),
                                       afa_jahresbetrag=(500000, "bestaetigt"))) == []


def test_kein_gewinn_gwg_widerspruch():
    # GWG = Betriebsvermögen (§ 6 Abs. 2): kein_gewinn=true + gwg_anschaffungskosten_netto>0 → Widerspruch (2-III).
    # Nur GWG-Anschaffungen (kein sonstiger Gewinn-Indikator) belegen schon einen Betrieb.
    w = FC.flag_widersprueche(_snap(kein_gewinn=(True, "bestaetigt"),
                                    gwg_anschaffungskosten_netto=(60000, "bestaetigt")))
    assert len(w) == 1 and w[0]["flag"] == "kein_gewinn" and w[0]["feld_id"] == "gwg_anschaffungskosten_netto"


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
