"""§ 10c Sonderausgaben-Pauschbetrag im Gesamt-Scope (Aufrufer-Floor).

Der einzel/zusammen-Tarif buendelt § 10c intern (Engine-Groesse sonderausgaben_pauschbetrag);
der Gesamt-Scope (festzusetzende_est_gesamt) nimmt die FINALEN Sonderausgaben und subtrahiert nur.
Ohne Aufrufer-Floor wurde derselbe § 19-Lohn pfad-abhaengig verschieden besteuert (an_gesamt via
est_einzel MIT § 10c vs. gesamt-Ringe OHNE) — ein K2-Fidelity-Bruch. _sonderausgaben_final schliesst
ihn: mindestens der Pauschbetrag (36 je Person / 72 zusammen), Wert aus params/<vz>.
"""
import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, os.path.join(ROOT, "golden"))

VZ = (2024, 2025, 2026)


def _runner():
    try:
        import runner
        return runner
    except Exception as e:  # Catala-Toolchain fehlt
        pytest.skip(f"Catala-Toolchain nicht verfügbar: {type(e).__name__}: {e}")


def test_konsistenz_gate_engine_groesse():
    """KONSISTENZ-GATE: _sonderausgaben_final(0, ...) reproduziert die Engine-Groesse
    sonderausgaben_pauschbetrag (§ 10c) — einzel = Engine-Wert, zusammen = x2 (§ 10c Satz 2).
    Divergenz params ↔ Engine-Konstante → ROT."""
    runner = _runner()
    for vz in VZ:
        engine = int(runner.E.sonderausgaben_pauschbetrag(runner.VZ_ENUM[vz])) // 100
        assert runner._sonderausgaben_final(0, vz, "einzel") == engine, vz
        assert runner._sonderausgaben_final(0, vz, "zusammen") == engine * 2, vz


def test_floor_bindet_nur_unter_pauschbetrag():
    """Guenstigervergleich: tatsaechliche SA >= Pauschbetrag bleiben unveraendert; darunter greift
    der Pauschbetrag."""
    runner = _runner()
    assert runner._sonderausgaben_final(0, 2025, "einzel") == 36
    assert runner._sonderausgaben_final(20, 2025, "einzel") == 36
    assert runner._sonderausgaben_final(36, 2025, "einzel") == 36
    assert runner._sonderausgaben_final(500, 2025, "einzel") == 500
    assert runner._sonderausgaben_final(50, 2025, "zusammen") == 72
    assert runner._sonderausgaben_final(5000, 2025, "zusammen") == 5000


def _gesamt(runner, **kw):
    s = {"gesamtfall": True, "veranlagungszeitraum": 2025, "veranlagung": "einzel"}
    s.update(kw)
    return runner.catala_est(s)


def test_gesamt_zieht_p10c_ab():
    """catala_gesamt subtrahiert den § 10c-Pauschbetrag: reiner Vermieter vv=30000 -> zvE 29964
    -> 4293 (statt 4303 ohne Floor)."""
    runner = _runner()
    assert _gesamt(runner, einkuenfte_vermietung=30000) == 4293


def test_pfad_fidelitaet_einzel_gleich_gesamt():
    """K2-KERN: derselbe § 19-Lohn muss pfad-unabhaengig gleich besteuert werden. est_einzel-Pfad
    (an_gesamt-Ring) und gesamt-Pfad (gesamt/kombiniert-Ring, gespeist mit summe_der_einkuenfte)
    liefern jetzt identische festzusetzende ESt — vor dem § 10c-Floor 11 EUR auseinander."""
    runner = _runner()
    from catala_runtime import Money
    for bl in (30000, 40000, 55000):
        r = runner.E.festzusetzende_est_einzel(runner.E.FestzusetzendeEstEinzelIn(
            bruttoarbeitslohn_in=Money(f"{bl}.00"), werbungskosten_in=Money("0.00"),
            sonderausgaben_in=Money("0.00"), veranlagungszeitraum_in=runner.VZ_ENUM[2025]))
        einzel_est = int(r.festzusetzende_est) // 100
        sde = int(r.summe_der_einkuenfte) // 100
        gesamt_est = _gesamt(runner, einkuenfte_nichtselbststaendig=sde)
        assert einzel_est == gesamt_est, (bl, einzel_est, gesamt_est)
