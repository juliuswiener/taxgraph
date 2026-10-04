"""Rechenweg-Kette aus dem ENDSTAND (p24a-bau, 2026-10-04) — rot-vorher auf 0aa91677.

Der Defekt (vault [[rechenweg-zeigt-andere-steuer-als-der-bescheid]], Auftrag p24a-kette-end):
`_festzusetzende` rechnet die Endsteuer aus dem Vor-Korrektur-Rohstand `g` und setzt die Korrekturen
DANEBEN — § 32d-Abgeltung (`result = est_raw + kap_st_k`, bescheid_zweige.py:968), § 35-Anrechnung
(`p35_credit` in `g2["steuerermaessigungen"]`), § 34 (Fünftel/Abs.3, `tarifliche_est_modifiziert`),
§ 32b (Post-Engine-Wrapper). Die Kette aber speist `runner.catala_gesamt_kette(g)` aus dem
VOR-Korrektur-Wert. Der Wächter `_setze_kette` (bescheid_zweige.py:115) vergleicht die letzte Stufe
mit der gezahlten Steuer und verwirft die Kette still, wo sie abweichen — dort fehlt unter dem
Bescheid die Tabelle, der Nutzer sieht nur den Hinweis. In den sieben Sonderfaellen ist die Kette
heute `kette=None` (gemessen, s. bericht), nicht falsch, aber unsichtbar; wo sie gruen durchrauscht,
koennte sie eine andere Steuer zeigen als die Zahl darueber.

Dieser Test haelt die BESTELLTE Korrektur: die letzte Kette-Stufe MUSS die gezahlte Steuer treffen —
bei einem der sieben Faelle (`p32d-kap30k`, 30.000 EUR Kapitalertraege, § 32d-Abgeltung + 7.250 EUR).
Er ist auf dem heutigen Stand ROT (Kette=None, keine Stufe trifft), nach dem Fix GRUEN.

Reihenfolge der Stufen nach dem Fix (nur die letzte Stufe aendert sich, die Zwischenstufen bleiben):
  GdE=58.770, zvE=58.734, tarifliche=13.924 — unveraendert, denn § 32d-Kapital laeuft ueber
  § 2 Abs. 5b EStG nicht in den tariflichen zvE. festzusetzende=21.174 (war 13.924) = gezahlte Steuer.
"""
from __future__ import annotations

import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/haut", "produkt/store", "golden"):
    sys.path.insert(0, os.path.join(ROOT, sub))

# Dieselben Helfer und Feld-Listen wie der Existing-Rechenweg-Test: die Konstellation des § 32d-Falls
# wird nicht erfunden, sie stammt aus tests/test_ui_rechenweg.py::test_kette_endet_bei_der_zahl.
from test_ui_rechenweg import (  # noqa: E402,F401
    _LOHN_20K_EINZEL, _LOHN_300K_ZUSAMMEN, _LOHN_60K_EINZEL, _RENTE_20K, _VG_A, _fall_mit, base)


# § 32d, 30.000 EUR Kapitalertraege — genau die Konstellation des Existing-Tests (id p32d-kap30k).
_FELDER_P32D = _LOHN_60K_EINZEL + [
    ("kein_gewinn", True), ("kein_kap", False), ("kap_kapitalertraege", 3_000_000),
    ("kap_gewinn_aktien", 0), ("kap_gewinn_sonstige", 0),
    ("kap_verlust_aktien", 0), ("kap_verlust_sonstige", 0)]


def test_endstand_kette_trifft_die_gezaehlte_steuer_p32d(base):
    """Der § 32d-Fall: die Kette endet bei der gezahlten Steuer, nicht an einer stillen None.

    Heute: `kette is None` (der Wächter verwirft die Vor-Korrektur-Kette, die bei 13.924 EUR endete,
    gegen 21.174 EUR Bescheid). Nach dem Fix: Kette vorhanden, letzte Stufe = zahl_cent."""
    ergebnis = _fall_mit(base, "rw-end-p32d", "gesamt", _FELDER_P32D)
    k = ergebnis["kette"]
    assert k is not None, (
        "Rechenweg-Kette fehlt im § 32d-Fall (kette=None) — sie muesste aus dem Endstand "
        f"kommen und bei {ergebnis['zahl_cent']} ct enden; Bescheid: {ergebnis['zahl_cent']} ct")
    assert k["festzusetzende_est"] * 100 == ergebnis["zahl_cent"], (
        f"Rechenweg endet bei {k['festzusetzende_est']} EUR, die Zahl darueber ist "
        f"{ergebnis['zahl_cent']} ct — zwei Steuern unter demselben Label")


# Die zwoelf Faelle des Rust-Gegenstuecks rust/api/tests/kette_endstand_hermetisch.rs, mit DENSELBEN
# Konstanten (Messung am Python-Server, 2026-10-04): (zahl_cent, [GdE, zvE, tariflich, festzusetzende]).
# Beide Aufrufstellen sind abgedeckt: gesamt (g0-g5) UND der Rentner-Zweig (r0-r5). Der Rentner-Zweig
# hat sonst keinen roten Test, der seine Setzstelle haelt. g4 und r2 halten den § 31-Zweig MIT
# Korrektur (§ 32d): ohne sie liesse sich die Speisung des Kinderfalls auf das Vor-Korrektur-`g`
# zurueckdrehen, ohne dass ein Test rot wird (Mutanten P3/P4). Drei Luecken fanden erst die Mutanten
# des Instructors: (1) r5 — Rentner-Kind, in dem der FREIBETRAG gewinnt (bei r2 gewinnt das Kindergeld,
# `_kette_p31` laesst den Freibetrag-Lauf dann unbenutzt); (2) g5/r3 — § 34 (die `tarifliche` Stufe
# traegt `tarif_modifiziert` aus dem finalen `g2`, nicht aus `g`); (3) r4 — § 35 im Rentner-Zweig
# (haelt Kette = Zahl, sieht aber `g2` vs. `g` NICHT: `steuerermaessigungen` beruehrt nur die letzte
# Stufe, die der Endwert ersetzt; aequivalent, s. Bericht).
_FELDER_P35 = _LOHN_60K_EINZEL + [
    ("kein_kap", True), ("kein_gewinn", False), ("einkuenfte_gewinn", 5_000_000),
    ("gewinn_betriebsart", "gewerbe"), ("gewst_messbetrag", 150_000), ("gewst_hebesatz", 400)]
_FELDER_KIND = _LOHN_20K_EINZEL + [("fam_anzahl_kinder", 1), ("kein_gewinn", True)]
_FELDER_KIND_FREIBETRAG_P32D = _LOHN_300K_ZUSAMMEN + [
    ("fam_anzahl_kinder", 1), ("kein_gewinn", True), ("kein_kap", False),
    ("kap_kapitalertraege", 3_000_000), ("kap_gewinn_aktien", 0), ("kap_gewinn_sonstige", 0),
    ("kap_verlust_aktien", 0), ("kap_verlust_sonstige", 0)]
_FELDER_RENTNER_P32D = _RENTE_20K + [
    ("kein_gewinn", True), ("kein_kap", False), ("kap_kapitalertraege", 3_000_000),
    ("kap_gewinn_aktien", 0), ("kap_gewinn_sonstige", 0),
    ("kap_verlust_aktien", 0), ("kap_verlust_sonstige", 0)]
# § 34: 120.000 EUR Veraeusserungsgewinn (Konstellationen vg-einzel120 / rente20k-vg120 des Existing-Tests).
_FELDER_P34_GESAMT = _LOHN_60K_EINZEL + [("kein_kap", True)] + _VG_A + [
    ("rentner_veraeusserungsgewinn", 12_000_000)]
_FELDER_P34_RENTNER = _RENTE_20K + _VG_A + [("rentner_veraeusserungsgewinn", 12_000_000)]
_FELDER_P35_RENTNER = _RENTE_20K + [
    ("kein_gewinn", False), ("einkuenfte_gewinn", 5_000_000), ("gewinn_betriebsart", "gewerbe"),
    ("gewst_messbetrag", 150_000), ("gewst_hebesatz", 400)]
# 300.000 EUR Rente, Zusammenveranlagung, 1 Kind: hier gewinnt der Kinderfreibetrag (gemessen, s. Bericht).
_FELDER_RENTNER_KIND_FREIBETRAG = [
    (f, {"veranlagung": "zusammen", "rentner_jahresrente": 30_000_000}.get(f, w))
    for f, w in _RENTE_20K] + [("fam_anzahl_kinder", 1), ("kein_gewinn", True)]


@pytest.mark.parametrize("scheibe,felder,zahl_cent,stufen,p31", [
    ("gesamt", _LOHN_60K_EINZEL + [("kein_kap", True), ("kein_gewinn", True)],
     1_392_400, [58_770, 58_734, 13_924, 13_924], None),
    ("gesamt", _FELDER_P32D, 2_117_400, [58_770, 58_734, 13_924, 21_174], None),
    ("gesamt", _FELDER_P35, 2_875_600, [108_770, 108_734, 34_756, 28_756], None),
    ("gesamt", _FELDER_KIND, 132_700, [18_770, 18_734, 1_327, 1_327], "kindergeld"),
    ("gesamt", _FELDER_KIND_FREIBETRAG_P32D, 10_965_600, [298_770, 289_098, 99_596, 109_656],
     "freibetraege"),
    ("rentner_gesamt", _FELDER_RENTNER_P32D, 806_100, [16_598, 16_562, 811, 8_061], None),
    ("rentner_gesamt", _FELDER_RENTNER_P32D + [("fam_anzahl_kinder", 1)], 806_100,
     [16_598, 16_562, 811, 8_061], "kindergeld"),
    ("rentner_gesamt", _RENTE_20K + [("kein_gewinn", True)], 81_100, [16_598, 16_562, 811, 811], None),
    ("gesamt", _FELDER_P34_GESAMT, 4_458_400, [133_770, 133_734, 44_584, 44_584], None),
    ("rentner_gesamt", _FELDER_P34_RENTNER, 2_051_100, [91_598, 91_562, 20_511, 20_511], None),
    ("rentner_gesamt", _FELDER_P35_RENTNER, 1_105_000, [66_598, 66_562, 17_050, 11_050], None),
    ("rentner_gesamt", _FELDER_RENTNER_KIND_FREIBETRAG, 8_234_000, [250_398, 240_726, 79_280, 82_340],
     "freibetraege"),
], ids=["g0-gegenprobe", "g1-p32d", "g2-p35", "g3-kind-kindergeld", "g4-kind-freibetrag-p32d",
        "r1-rentner-p32d", "r2-rentner-kind-p32d", "r0-rentner-gegenprobe", "g5-p34",
        "r3-rentner-p34", "r4-rentner-p35", "r5-rentner-kind-freibetrag"])
def test_endstand_kette_stufen_wie_in_rust(base, scheibe, felder, zahl_cent, stufen, p31, request):
    """Kette da, letzte Stufe = gezahlte Steuer, alle vier Stufen wie im Rust-Test (selbe Konstanten).

    GdE und zvE aendern sich durch den Fix nicht. Die `tarifliche` Stufe kommt aus dem finalen `g2`
    (§ 34 `tarif_modifiziert`, § 31 Freibetrag); die letzte Stufe traegt die Terme ausserhalb der
    Engine (§ 32d, § 32b) und die Anrechnung (§ 35)."""
    e = _fall_mit(base, request.node.callspec.id, scheibe, felder)
    assert e["zahl_cent"] == zahl_cent, e
    k = e["kette"]
    assert k is not None, f"kette fehlt (null) bei {e['zahl_cent']} ct -- der Waechter hat sie verworfen"
    assert [k["gesamtbetrag_der_einkuenfte"], k["zu_versteuerndes_einkommen"],
            k["tarifliche_est"], k["festzusetzende_est"]] == stufen, k
    assert k["festzusetzende_est"] * 100 == e["zahl_cent"]
    assert (k.get("p31") or {}).get("guenstiger") == p31, k
