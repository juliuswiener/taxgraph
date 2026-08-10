"""Gate für Option A (Julius-Entscheidung 2026-08-10, BACKLOG anlage-kap-folgeangabe): bestätigte
KAP-Nullen werden NICHT mehr deklariert. Ausgangslage (gemessen 2026-08-09, Kommentar in
tests/test_checkest_durchstich.py): kap_kapitalertraege/kap_gewinn_aktien/kap_verlust_aktien/
kap_gewinn_sonstige/kap_verlust_sonstige sind Pflicht-Kegelfelder der "gesamt"-Scheibe — jeder
Arbeitnehmer ohne Kapitalvermögen beantwortet sie mit 0. Deklariert als Kz, liest ELSTER daraus
"Kapitalerträge erklärt" und verlangt einen Angabegrund, den wir nicht liefern (checkESt-Meldung).

Deckt: alle 5 Felder 0 (keine E19*-Kz mehr), ein Feld >0 (ALLE Kz bleiben, auch die Nullen —
Gegenrichtung), Zusammenveranlagung mit beiden Personen atomar (echter Bug, gemessen 2026-08-10:
nur Person A unterdrückt + Person B behält ihre Nullen erzeugt eine NEUE checkESt-Fehlerklasse,
weil der Writer für Person-B-Instanz zuerst eine leere Person-A-Instanz anlegt)."""
from __future__ import annotations

import os
import sys

import pytest

yaml = pytest.importorskip("yaml")

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/mapping", "produkt/store", "produkt/traverser"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import est_mapping as EM   # noqa: E402
import store as ST         # noqa: E402
import traverser as TR     # noqa: E402

TS = "2026-08-10T10:00:00+00:00"
H = {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}

_KAP_KZ_A = ("E1900701", "E1900901", "E1901301", "E1901201")   # kap_gewinn_sonstige hat kein eigenes Kz


@pytest.fixture(scope="module")
def bindung():
    return TR.lade_bindung()


def _b(s, feld_id, wert, zustand="bestaetigt"):
    sig = {"signal_1": None, "signal_2": f"ok@{feld_id}"} if zustand == "bestaetigt" else {"signal_1": None, "signal_2": None}
    ST.append_event(s, feld_id=feld_id, wert=wert, zustand=zustand, herkunft=H, schreiber="ui:laie",
                    signal=sig, ts=TS)


def _store_mit(felder: dict, vz=2025):
    s = ST.leerer_store(vz, fall_id="kap-null")
    for fid, wert in felder.items():
        _b(s, fid, wert)
    return s


# ---- 1) alle fünf Felder 0 -> keine E19*-Kz --------------------------------

def test_alle_fuenf_null_keine_kap_kz(bindung):
    felder = {f: 0 for f in EM.KAP_FELDER_A}
    snap, _ = ST.materialisiere(_store_mit(felder))
    r = EM.deklariere(snap, bindung)
    for kz in _KAP_KZ_A:
        assert kz not in r["deklaration"], f"{kz} haette unterdrueckt werden muessen"
    ndf = {x["feld_id"] for x in r["nicht_deklariert"]}
    assert set(EM.KAP_FELDER_A) <= ndf                          # Auflage C: maschinenlesbar begründet
    for x in r["nicht_deklariert"]:
        if x["feld_id"] in EM.KAP_FELDER_A:
            assert x["grund"]                                   # jeder Eintrag mit Grund


def test_alle_fuenf_null_bleibt_vollstaendig(bindung):
    """Die Unterdrückung ist kein fail-closed-Fall — die Erklärung bleibt vollständig."""
    felder = {f: 0 for f in EM.KAP_FELDER_A}
    snap, _ = ST.materialisiere(_store_mit(felder))
    r = EM.deklariere(snap, bindung)
    assert r["vollstaendig"] is True
    assert not r["unvollstaendig"]


# ---- 2) Gegenrichtung: ein Feld > 0 -> ALLE Kz bleiben, auch die Nullen ----

def test_ein_feld_positiv_alle_kz_bleiben(bindung):
    """Reale Kapitalerträge (Gewinn 5000, Verlust 0 im selben Topf) dürfen NICHT beschädigt werden —
    die Anlage KAP muss vollständig entstehen, inklusive der echten Nullen."""
    felder = {"kap_kapitalertraege": 500000, "kap_gewinn_aktien": 500000,
              "kap_verlust_aktien": 0, "kap_gewinn_sonstige": 0, "kap_verlust_sonstige": 0}
    snap, _ = ST.materialisiere(_store_mit(felder))
    r = EM.deklariere(snap, bindung)
    assert r["deklaration"]["E1900701"] == 5000
    assert r["deklaration"]["E1900901"] == 5000
    assert r["deklaration"]["E1901301"] == 0                    # echte Null bleibt deklariert
    assert r["deklaration"]["E1901201"] == 0
    ndf = {x["feld_id"] for x in r["nicht_deklariert"]}
    assert "kap_verlust_aktien" not in ndf
    assert "kap_verlust_sonstige" not in ndf


def test_nur_ein_feld_ungleich_null_reicht_zum_erhalt(bindung):
    """Auch wenn NUR kap_gewinn_sonstige (kein eigenes Kz) ungleich 0 ist, gilt die Person nicht als
    'garantiert kapitalertragsfrei' — die anderen vier Kz bleiben deklariert."""
    felder = {"kap_kapitalertraege": 0, "kap_gewinn_aktien": 0, "kap_verlust_aktien": 0,
              "kap_gewinn_sonstige": 12300, "kap_verlust_sonstige": 0}
    snap, _ = ST.materialisiere(_store_mit(felder))
    r = EM.deklariere(snap, bindung)
    for kz in _KAP_KZ_A:
        assert kz in r["deklaration"]
        assert r["deklaration"][kz] == 0


# ---- 3) Zusammenveranlagung, beide Personen ---------------------------------

def test_zusammen_beide_null_beide_unterdrueckt(bindung):
    felder = {f: 0 for f in EM.KAP_FELDER_A} | {f: 0 for f in EM.KAP_FELDER_B}
    snap, _ = ST.materialisiere(_store_mit(felder))
    r = EM.deklariere(snap, bindung)
    for kz in _KAP_KZ_A:
        assert kz not in r["deklaration"]
        assert kz not in r["person_b"]
    ndf = {x["feld_id"] for x in r["nicht_deklariert"]}
    assert set(EM.KAP_FELDER_A) <= ndf
    assert set(EM.KAP_FELDER_B) <= ndf


def test_zusammen_atomar_a_null_b_wert_beide_bleiben(bindung):
    """Der eigentliche Bug (gemessen 2026-08-10): Person A ist garantiert kapitalertragsfrei, Person B
    hat echte Kapitalerträge. Trotzdem darf A NICHT unterdrückt werden — sonst legt der Writer beim
    Schreiben der Person-B-Instanz eine leere Person-A-Instanz an und checkESt meldet eine NEUE
    Fehlerklasse statt weniger Fehler. Atomar heißt: nur wenn BEIDE Seiten 0 sind, fällt auch nur
    eine Seite weg."""
    felder = {f: 0 for f in EM.KAP_FELDER_A}
    felder["kap_kapitalertraege_partner"] = 800000
    felder["kap_gewinn_aktien_partner"] = 800000
    felder["kap_verlust_aktien_partner"] = 0
    felder["kap_verlust_sonstige_partner"] = 0
    snap, _ = ST.materialisiere(_store_mit(felder))
    r = EM.deklariere(snap, bindung)
    for kz in _KAP_KZ_A:                                        # Person A NICHT unterdrückt
        assert kz in r["deklaration"], f"{kz} haette wegen Person B (Wert) NICHT unterdrueckt werden duerfen"
        assert r["deklaration"][kz] == 0
    assert r["person_b"]["E1900701"] == 8000
    assert r["person_b"]["E1900901"] == 8000
    assert r["person_b"]["E1901301"] == 0
    assert r["person_b"]["E1901201"] == 0


def test_zusammen_atomar_a_wert_b_null_beide_bleiben(bindung):
    """Spiegelfall: Person A hat echte Kapitalerträge, Person B ist garantiert kapitalertragsfrei —
    auch hier bleibt B NICHT unterdrückt (dieselbe Atomaritäts-Naht in der anderen Richtung)."""
    felder = {"kap_kapitalertraege": 300000, "kap_gewinn_aktien": 300000,
              "kap_verlust_aktien": 0, "kap_gewinn_sonstige": 0, "kap_verlust_sonstige": 0}
    for f in EM.KAP_FELDER_B:
        felder[f] = 0
    snap, _ = ST.materialisiere(_store_mit(felder))
    r = EM.deklariere(snap, bindung)
    assert r["deklaration"]["E1900701"] == 3000
    assert r["deklaration"]["E1900901"] == 3000
    for kz in _KAP_KZ_A:
        assert kz in r["person_b"], f"{kz} haette wegen Person A (Wert) NICHT unterdrueckt werden duerfen"
        assert r["person_b"][kz] == 0


# ---- Sonderfall: fail-closed hat Vorrang -----------------------------------

def test_ein_feld_vorlaeufig_unterdrueckt_nichts(bindung):
    """Ein vorläufiges KAP-Feld macht die 'garantiert 0'-Entscheidung unklar — nichts wird
    unterdrückt, die bestehende fail-closed-Logik (unvollständig) bleibt zuständig."""
    s = ST.leerer_store(2025, fall_id="kap-vorl")
    for f in EM.KAP_FELDER_A:
        if f == "kap_gewinn_aktien":
            _b(s, f, 0, zustand="vorlaeufig")
        else:
            _b(s, f, 0)
    snap, _ = ST.materialisiere(s)
    r = EM.deklariere(snap, bindung)
    assert r["vollstaendig"] is False
    for kz in ("E1900701", "E1901301", "E1901201"):             # die bestätigten Nullen bleiben deklariert
        assert kz in r["deklaration"]
    assert "E1900901" not in r["deklaration"]                   # das vorläufige Feld selbst nie deklariert


def test_kap_gewinn_sonstige_bleibt_immer_unabhaengig_deklariert_grund(bindung):
    """kap_gewinn_sonstige hat kein eigenes elster_kz (Bindung: 'ENDGUELTIG: Modell-Mismatch') und
    landet unabhängig vom Options-A-Filter mit seinem EIGENEN Grund in nicht_deklariert — keine
    doppelte oder überschriebene Begründung."""
    felder = {f: 0 for f in EM.KAP_FELDER_A}
    snap, _ = ST.materialisiere(_store_mit(felder))
    r = EM.deklariere(snap, bindung)
    treffer = [x for x in r["nicht_deklariert"] if x["feld_id"] == "kap_gewinn_sonstige"]
    assert len(treffer) == 1
    assert "Modell-Mismatch" in treffer[0]["grund"]
    assert treffer[0]["grund"] != EM._KAP_NULL_GRUND
