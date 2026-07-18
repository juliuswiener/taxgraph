"""Gate für den Repeated-Instance-KERN (Store-Modell A, Klasse INSTANZ). Deterministisch, NULL LLM.

Der Kern ist die generische Instanz-MECHANIK in est_mapping (unabhängig freeze-bar, VOR dem ersten
Konsumenten Multi-Objekt-§21): ein wiederholbares Anlage-Feld trägt für Instanz 2..N das Suffix __<n> am
feld_id; est_mapping routet es in den anlage_instanzen-Bucket seiner Gruppe und verwendet je Instanz DIESELBE
Basis-Kz (Instanz-Reuse, kein neuer Kz — analog Person-B/Klasse g auf der Instanz-Achse).

Prüft mit einer SYNTHETISCHEN Bindung (kein reales Feld ist im Kern schon instanz-getaggt — das ist der
Konsument-Schritt): 1:1-Reuse über N Instanzen, Aggregat je Objekt (Klasse a je Instanz), Round-Trip,
fail-closed je Instanz (vorlaeufig -> unvollständig), Guard gegen __n auf nicht-instanz-fähiger Basis,
Kz-Reuse = kein Phantom (Drift-Awareness), und die STORE-INVARIANZ: base__<n> erfüllt das unveränderte
Store-feld_id-Pattern (Schema-Gate bleibt grün — bewusst kein '#').
"""
from __future__ import annotations

import copy
import json
import os
import sys

import pytest

jsonschema = pytest.importorskip("jsonschema")

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/mapping", "produkt/store"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import est_mapping as EM   # noqa: E402
import store as ST         # noqa: E402

TS = "2026-07-18T14:00:00+00:00"
H = {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}

# Synthetische Bindung: vv_einnahmen 1:1 (E0700201) + vv_gebaeude_afa/vv_schuldzinsen als Aggregat-Quellen
# (real in DOKUMENTIERT_AGGREGAT -> E0703838), alle drei der Gruppe 'vv_objekt' zugeordnet; solo_feld ist
# NICHT instanz-fähig (Guard-Fall). Nur die von deklariere gelesenen Schlüssel sind nötig (dict, keine Datei).
BINDUNG = {
    "vv_einnahmen":    {"elster_kz": "E0700201", "instanz_gruppe": "vv_objekt"},
    "vv_gebaeude_afa": {"elster_kz": None, "elster_kz_grund": "Anlage-V-Aggregat", "instanz_gruppe": "vv_objekt"},
    "vv_schuldzinsen": {"elster_kz": None, "elster_kz_grund": "Anlage-V-Aggregat", "instanz_gruppe": "vv_objekt"},
    "solo_feld":       {"elster_kz": "E0100082"},   # kein instanz_gruppe -> nicht instanz-fähig
}


def _snap(felder: dict, vorlaeufig: tuple = ()) -> dict:
    out = {}
    for fid, wert in felder.items():
        out[fid] = {"wert": wert, "zustand": "vorlaeufig" if fid in vorlaeufig else "bestaetigt",
                    "herkunft": H}
    return out


# ---- 1:1-Reuse über N Instanzen ---------------------------------------------

def test_instanz_1zu1_reuse_ueber_n():
    """Instanz 1 = Basis (deklaration, unverändert); __2/__3 = anlage_instanzen mit DERSELBEN Kz (Reuse)."""
    r = EM.deklariere(_snap({"vv_einnahmen": 100000, "vv_einnahmen__2": 200000, "vv_einnahmen__3": 300000}),
                      BINDUNG)
    assert r["deklaration"]["E0700201"] == 100000              # Instanz 1 unverändert in der Haupt-Deklaration
    inst = r["anlage_instanzen"]["vv_objekt"]
    assert [e["index"] for e in inst] == [2, 3]                # index-sortiert
    assert inst[0]["felder"]["E0700201"] == 200000             # Reuse: dieselbe Kz je Instanz
    assert inst[1]["felder"]["E0700201"] == 300000
    assert "E0700201" in r["deklaration"] and r["vollstaendig"] is True


def test_instanz_1_nicht_in_anlage_instanzen():
    """Ohne __n-Feld ist anlage_instanzen leer (reiner Einzel-Objekt-Fall = Null-Regression)."""
    r = EM.deklariere(_snap({"vv_einnahmen": 100000}), BINDUNG)
    assert r["anlage_instanzen"] == {}
    assert r["deklaration"]["E0700201"] == 100000


# ---- Aggregat je Objekt (Klasse a je Instanz) --------------------------------

def test_instanz_aggregat_je_objekt():
    """Die WK-Aggregat-Quellen summieren PRO Instanz auf E0703838 (dokumentiert, nicht deklariert)."""
    r = EM.deklariere(_snap({"vv_gebaeude_afa__2": 30000, "vv_schuldzinsen__2": 20000,
                             "vv_gebaeude_afa__3": 40000}), BINDUNG)
    inst = {e["index"]: e for e in r["anlage_instanzen"]["vv_objekt"]}
    assert inst[2]["dokumentiert"]["E0703838"]["summe"] == 50000           # je Objekt getrennt
    assert inst[2]["dokumentiert"]["E0703838"]["quell_felder"] == ["vv_gebaeude_afa__2", "vv_schuldzinsen__2"]
    assert inst[3]["dokumentiert"]["E0703838"]["summe"] == 40000
    assert "E0703838" not in r["deklaration"]                             # Aggregat NIE deklariert (Anlage-V-Ruling)


# ---- Round-Trip --------------------------------------------------------------

def test_instanz_roundtrip_1zu1_und_aggregat():
    r = EM.deklariere(_snap({"vv_einnahmen": 100000, "vv_einnahmen__2": 200000,
                             "vv_gebaeude_afa__2": 30000, "vv_schuldzinsen__2": 20000}), BINDUNG)
    rt = EM.zuruecklesen(r, BINDUNG)
    assert rt["felder"]["vv_einnahmen"] == 100000              # Instanz 1 (Basis)
    assert rt["felder"]["vv_einnahmen__2"] == 200000           # Instanz 2 invertierbar (1:1)
    assert rt["aggregat"]["E0703838__2"] == 50000              # Aggregat je Instanz nur Summe (verlustbehaftet)
    assert "vv_gebaeude_afa__2" not in rt["felder"]            # kein Detail-Verlust vorgetäuscht


# ---- fail-closed je Instanz --------------------------------------------------

def test_instanz_fail_closed_vorlaeufig():
    """Ein vorläufiges Instanz-Feld -> unvollständig, NICHT im Bucket, Gesamt vollstaendig=False."""
    r = EM.deklariere(_snap({"vv_einnahmen__2": 200000}, vorlaeufig=("vv_einnahmen__2",)), BINDUNG)
    assert r["vollstaendig"] is False
    assert "vv_einnahmen__2" in {x["feld_id"] for x in r["unvollstaendig"]}
    assert r["anlage_instanzen"] == {}                         # vorläufige Instanz nie im Bucket


# ---- Guard: __n auf nicht-instanz-fähiger Basis ------------------------------

def test_instanz_basis_nicht_instanzfaehig_fail_closed():
    """solo_feld__2 (Basis ohne instanz_gruppe) wird NICHT still als Instanz deklariert -> fail-closed."""
    r = EM.deklariere(_snap({"solo_feld": "x", "solo_feld__2": "y"}), BINDUNG)
    assert r["anlage_instanzen"] == {}                         # nicht als Instanz behandelt
    assert "solo_feld__2" in {x["feld_id"] for x in r["nicht_deklariert"]}   # bewusst nicht deklariert
    assert r["deklaration"]["E0100082"] == "x"                # Basis (Instanz 1) unverändert


# ---- Drift-Awareness: Instanz-Kz = Basis-Kz (kein Phantom) -------------------

def test_instanz_kz_reuse_kein_phantom():
    """Alle in anlage_instanzen verwendeten Kz sind Basis-1:1-Kz (Instanz-Reuse), kein neues/Phantom-Kz —
    die Drift-Phantom-Prüfung (Assertion 3) bleibt gültig, weil Instanz-Kz aus der erlaubten Menge stammen."""
    r = EM.deklariere(_snap({"vv_einnahmen__2": 200000, "vv_einnahmen__3": 300000}), BINDUNG)
    basis_kz = {b["elster_kz"] for b in BINDUNG.values() if b.get("elster_kz")}
    inst_kz = {kz for grp in r["anlage_instanzen"].values() for e in grp for kz in e["felder"]}
    assert inst_kz and inst_kz <= basis_kz, f"Instanz-Kz ohne Basis-Entsprechung (Phantom): {inst_kz - basis_kz}"


# ---- STORE-INVARIANZ: base__<n> erfüllt das unveränderte Store-Pattern -------

def test_store_feld_id_pattern_unveraendert_akzeptiert_instanz():
    """Der KERN-Beweis: __<n> liegt in [a-z0-9_] -> der Store-feld_id-Pattern ^[a-z][a-z0-9_]*$ bleibt
    UNVERÄNDERT und akzeptiert Instanz-feld_ids. Store-Kern (append_event/materialisiere + Schema-Gate)
    behandelt vv_einnahmen__2 wie jedes andere feld_id — keine Instanz-Kenntnis, kein Schema-Umbau."""
    schema = json.load(open(os.path.join(ROOT, "produkt", "store", "schema.json"), encoding="utf-8"))
    s = ST.leerer_store(2025, fall_id="inst")
    for fid, wert in [("vv_einnahmen", 100000), ("vv_einnahmen__2", 200000), ("vv_gebaeude_afa__2", 30000)]:
        ST.append_event(s, feld_id=fid, wert=wert, zustand="bestaetigt", herkunft=H,
                        schreiber="ui:laie", signal={"signal_1": None, "signal_2": f"ok@{fid}"}, ts=TS)
    # Store-Schema-Gate MUSS grün bleiben (kein Pattern-Verstoß durch __<n>)
    errs = list(jsonschema.Draft202012Validator(schema).iter_errors(s))
    assert errs == [], f"Store-Schema-Gate rot durch Instanz-feld_id: {[e.message for e in errs]}"
    felder, _ = ST.materialisiere(s)
    assert felder["vv_einnahmen__2"]["wert"] == 200000        # der Store speichert/materialisiert es normal
    # und der Roundtrip durch est_mapping findet die Instanz wieder
    r = EM.deklariere(felder, BINDUNG)
    assert r["anlage_instanzen"]["vv_objekt"][0]["felder"]["E0700201"] == 200000


def test_hash_separator_wuerde_store_gate_brechen():
    """Gegenprobe (warum __<n> und nicht '#'): ein '#'-Suffix VERLETZT das Store-feld_id-Pattern -> das
    Schema-Gate würde rot. Belegt die Design-Wahl (Store-Invarianz statt Pattern-Aufweichung)."""
    schema = json.load(open(os.path.join(ROOT, "produkt", "store", "schema.json"), encoding="utf-8"))
    kaputt = ST.leerer_store(2025, fall_id="hash")
    # bewusst am append_event-Guard vorbei direkt ins Event-Log (append_event validiert den Pattern nicht,
    # das Schema-Gate schon) — zeigt: '#' käme durch den Writer, würde aber am Gate scheitern.
    kaputt["events"].append({"event_id": "0" * 64, "ts": TS, "feld_id": "vv_einnahmen#2", "wert": 1,
                             "zustand": "bestaetigt", "herkunft": H, "schreiber": "ui:laie",
                             "signal": {"signal_1": None, "signal_2": "ok"}, "ersetzt": None})
    errs = list(jsonschema.Draft202012Validator(schema).iter_errors(kaputt))
    assert any("vv_einnahmen#2" in str(e.instance) or "pattern" in e.message for e in errs), \
        "'#'-feld_id müsste das Store-Schema-Gate rot färben (Beleg der Separator-Wahl)"


# ---- Determinismus -----------------------------------------------------------

def test_instanz_determinismus():
    snap = _snap({"vv_einnahmen": 100000, "vv_einnahmen__2": 200000, "vv_gebaeude_afa__2": 30000})
    assert EM.deklariere(snap, BINDUNG) == EM.deklariere(snap, BINDUNG)


def test_neg_instanz_verfaelscht_bricht_roundtrip():
    """Manipulierter Instanz-Wert -> Round-Trip weicht ab (kein stiller Durchlauf)."""
    r = EM.deklariere(_snap({"vv_einnahmen__2": 200000}), BINDUNG)
    r2 = copy.deepcopy(r)
    r2["anlage_instanzen"]["vv_objekt"][0]["felder"]["E0700201"] += 1
    rt = EM.zuruecklesen(r2, BINDUNG)
    assert rt["felder"]["vv_einnahmen__2"] != 200000
