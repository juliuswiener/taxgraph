"""produkt/-Gesamt-Gate: Bindungstabelle × est_mapping Deklarations-Abdeckung (Task #11). NULL LLM.

Drift-Wächter (Falsch-Grün-Klasse Bindung↔est_mapping). Deckt genau die stille Über-/Unter-Deklaration:
  1. jedes eingetragene bindung.elster_kz wird von est_mapping deklariert (kein stiller Wegfall);
  2. jedes null-Kz-Feld ist bewusst GAP (nicht_deklariert mit Grund) ODER Transform-Quelle;
  3. kein Phantom-Kz in der Deklaration (jede E-Nr rückführbar auf Bindung ODER Transform-Ziel);
  4. est_mapping-Transform-Konfig konsistent (Quellfelder existieren, keine Kz-Kollision);
  5. feld_id GLOBAL eindeutig über alle Scheiben (der Bindungs-Gate prüft nur pro Datei).
Kein Catala/Toolchain (reiner Store→deklaration-Transform). Negativtests: Tamper selbst getriggert.
"""
from __future__ import annotations

import glob
import os
import sys

import pytest

yaml = pytest.importorskip("yaml")

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, os.path.join(ROOT, "produkt", "mapping"))

import est_mapping as EM   # noqa: E402

BINDUNG_GLOB = os.path.join(ROOT, "produkt", "bindung", "bindung_*.yaml")
H = {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}


def _eintraege_je_datei() -> dict:
    return {f: (yaml.safe_load(open(f, encoding="utf-8")).get("bindungen") or [])
            for f in sorted(glob.glob(BINDUNG_GLOB))}


@pytest.fixture(scope="module")
def merged() -> dict:
    """{feld_id -> eintrag} über ALLE Scheiben (nach dem Eindeutigkeits-Check gefahrlos)."""
    d: dict = {}
    for eintraege in _eintraege_je_datei().values():
        for b in eintraege:
            d[b["feld_id"]] = b
    return d


def _snapshot(merged: dict) -> dict:
    """felder-Ebene: alle askable Felder bestätigt mit beispielwert."""
    return {fid: {"wert": b["beispielwert"], "zustand": "bestaetigt", "herkunft": H}
            for fid, b in merged.items() if b.get("askable")}


def _verzweigung_ziel_kz() -> set:
    return {kz for cfg in EM.VERZWEIGUNG.values() for kz in cfg["kz"].values()}


def _transform_quellen() -> set:
    # Klasse f: Wert-Felder (VERZWEIGUNG-Keys) + Art-Felder; Klasse g: die _partner-Felder.
    verzweigung = set(EM.VERZWEIGUNG) | {cfg["art_feld"] for cfg in EM.VERZWEIGUNG.values()}
    return (set(EM.NEGATION) | set(EM.MULTIPLIKATION) | EM._aggregation_quellen()
            | verzweigung | set(EM.PARTNER_INSTANZ))


def _erlaubte_kz(merged: dict) -> set:
    """1:1-Kz aus der Bindung + Transform-Ziel-Kz (Negation + Verzweigung), die deklariert werden dürfen.
    Die Person-B-Instanz-Kz (Klasse g) sind KEINE neuen Kz — sie sind identisch mit den Person-A-1:1-Kz
    und erscheinen im person_b-Bucket, nicht in der Haupt-Deklaration (kein Kollisions-/Phantom-Fall)."""
    return ({b["elster_kz"] for b in merged.values() if b.get("elster_kz")}
            | set(EM.NEGATION.values()) | _verzweigung_ziel_kz())


# ---- Assertion 5: globale feld_id-Eindeutigkeit ------------------------------

def test_feld_id_global_eindeutig():
    gesehen: dict = {}
    for f, eintraege in _eintraege_je_datei().items():
        for b in eintraege:
            fid = b["feld_id"]
            assert fid not in gesehen, (f"feld_id '{fid}' doppelt über Scheiben: "
                                        f"{os.path.basename(gesehen[fid])} + {os.path.basename(f)}")
            gesehen[fid] = f


# ---- Assertion 1: jedes eingetragene Kz wird deklariert ----------------------

def test_jedes_eingetragene_kz_wird_deklariert(merged):
    r = EM.deklariere(_snapshot(merged), merged)
    unter = [fid for fid, b in merged.items()
             if b.get("askable") and b.get("elster_kz") and b["elster_kz"] not in r["deklaration"]]
    assert not unter, f"eingetragene Kz fallen still aus est_mapping (Unter-Deklaration): {unter}"


# ---- Assertion 2: jedes null-Kz-Feld ist bewusst GAP oder Transform-Quelle ---

def test_jedes_null_kz_feld_ist_gap_oder_transform(merged):
    r = EM.deklariere(_snapshot(merged), merged)
    nd = {x["feld_id"] for x in r["nicht_deklariert"]}
    tq = _transform_quellen()
    verschwunden = [fid for fid, b in merged.items()
                    if b.get("askable") and not b.get("elster_kz") and fid not in nd and fid not in tq]
    assert not verschwunden, (f"null-Kz-Felder weder GAP (nicht_deklariert) noch Transform-Quelle "
                              f"— still verschwunden: {verschwunden}")


# ---- Assertion 3: kein Phantom-Kz in der Deklaration -------------------------

def test_kein_phantom_kz_in_deklaration(merged):
    r = EM.deklariere(_snapshot(merged), merged)
    erlaubt = _erlaubte_kz(merged)
    phantome = [kz for kz in r["deklaration"] if kz not in erlaubt]
    assert not phantome, f"Phantom-Kz in deklaration ohne Bindungs-/Transform-Herkunft: {phantome}"


def test_person_b_instanz_kz_sind_person_a_kz(merged):
    """Klasse g: die Person-B-Instanz-Kz (person_b-Bucket) sind IDENTISCH mit Person-A-1:1-Kz — kein
    neues/Phantom-Kz (Person B ist ein zweites Sub-Dokument mit denselben Kz)."""
    a_kz = {b["elster_kz"] for b in merged.values() if b.get("elster_kz")}
    fremd = set(EM.PARTNER_INSTANZ.values()) - a_kz
    assert not fremd, f"Person-B-Instanz-Kz ohne Person-A-1:1-Entsprechung (Phantom): {fremd}"


# ---- Assertion 4: est_mapping-Transform-Konfig konsistent --------------------

def test_transform_konfig_konsistent(merged):
    for src in _transform_quellen():
        assert src in merged, f"est_mapping-Transform-Quelle {src} fehlt in der Bindungstabelle"
    kz_felder: dict = {}
    for fid, b in merged.items():
        if b.get("elster_kz"):
            kz_felder.setdefault(b["elster_kz"], []).append(fid)
    kollisionen = {kz: fs for kz, fs in kz_felder.items() if len(fs) > 1}
    assert not kollisionen, f"1:1-Kz-Kollision (mehrere Felder je Kz): {kollisionen}"
    ziel_kz = set(EM.NEGATION.values()) | set(EM.DOKUMENTIERT_AGGREGAT) | _verzweigung_ziel_kz()
    kollidiert = ziel_kz & set(kz_felder)
    assert not kollidiert, f"Transform-Ziel-Kz kollidiert mit 1:1-Kz: {kollidiert}"


# ---- Negativtests (Tamper selbst getriggert; in-memory-Ergebnis, isoliert) ---

def test_neg_kz_wegnahme_wird_rot(merged):
    """(a) Ein eingetragenes Kz aus der Deklaration entfernt -> Abdeckungs-Prüfung (Assertion 1)
    findet die Unter-Deklaration. Tamper am lokalen Ergebnis, keine Quelldatei berührt."""
    r = EM.deklariere(_snapshot(merged), merged)
    opfer = next(b["elster_kz"] for b in merged.values()
                 if b.get("elster_kz") and b["elster_kz"] in r["deklaration"])
    r["deklaration"].pop(opfer)                                    # TAMPER
    unter = [fid for fid, b in merged.items()
             if b.get("askable") and b.get("elster_kz") and b["elster_kz"] not in r["deklaration"]]
    assert unter, "Kz-Wegnahme müsste als Unter-Deklaration auffallen (Gate wäre sonst wirkungslos)"


def test_neg_null_kz_als_enr_wird_rot(merged):
    """(b) Ein erfundenes E-Nr in der Deklaration -> Phantom-Prüfung (Assertion 3) schlägt an.
    Steht für den Über-Deklarations-/null→E-Nr-Fall. Tamper am lokalen Ergebnis."""
    r = EM.deklariere(_snapshot(merged), merged)
    erlaubt = _erlaubte_kz(merged)
    r["deklaration"]["E9999999"] = 1                              # TAMPER (erfundenes Kz)
    phantome = [kz for kz in r["deklaration"] if kz not in erlaubt]
    assert phantome == ["E9999999"], "Phantom-Kz müsste als nicht-rückführbar auffallen"
