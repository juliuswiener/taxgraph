"""§ 19 Abs. 2 Versorgungsfreibetrag + Zuschlag-Accessor (Weg A).

catala_p19_2_versorgungsfreibetrag: Kohorten-Lookup (Jahr Versorgungsbeginn),
Berechnung VFB = min(% × BG; HB) + Zuschlag (mit Deckel § 19 Abs. 2 S. 5).
Fail-closed ohne Bemessungsgrundlage oder Versorgungsbeginn-Jahr (keine stille Null).
"""
import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, os.path.join(ROOT, "golden"))


def _runner():
    try:
        import runner
        return runner
    except Exception as e:
        pytest.skip(f"Catala-Toolchain nicht verfügbar: {type(e).__name__}: {e}")


def _vfb(runner, **kw):
    return runner.catala_p19_2_versorgungsfreibetrag({
        "versorgungsbeginn_jahr": 2025,
        **kw
    })


# ---- Kohorten-Tests ----

def test_kohorte_2005_max_tabelle():
    """2005: 40% / HB 3000 / Zuschlag 900. BG=12000 → VFB=4800 aber HB 3000 → 3000,
    Zuschlag 900, Deckel = 12000−3000 = 9000 >> 900 → 3000+900 = 3900."""
    runner = _runner()
    assert _vfb(runner, versorgungsbezuege_bemessungsgrundlage=12000,
                versorgungsbeginn_jahr=2005) == 3900


def test_kohorte_2024():
    """2024: 13.6% / HB 1020 / Zuschlag 306. BG=12000 → VFB=1632 aber HB 1020 → 1020,
    Zuschlag 306, Deckel = 12000−1020 = 10980 >> 306 → 1020+306 = 1326."""
    runner = _runner()
    assert _vfb(runner, versorgungsbezuege_bemessungsgrundlage=12000,
                versorgungsbeginn_jahr=2024) == 1326


def test_kohorte_2025():
    """2025: 13.2% / HB 990 / Zuschlag 297. BG=12000 → VFB=1584 aber HB 990 → 990,
    Zuschlag 297, Deckel = 12000−990 = 11010 >> 297 → 990+297 = 1287."""
    runner = _runner()
    assert _vfb(runner, versorgungsbezuege_bemessungsgrundlage=12000,
                versorgungsbeginn_jahr=2025) == 1287


def test_kohorte_2058_null():
    """2058: 0% / HB 0 / Zuschlag 0 → alles 0."""
    runner = _runner()
    assert _vfb(runner, versorgungsbezuege_bemessungsgrundlage=12000,
                versorgungsbeginn_jahr=2058) == 0


# ---- Zuschlag-Deckel-Test (§ 19 Abs. 2 S. 5) ----

def test_zuschlag_deckel_kleine_bg():
    """Kleine BG, Zuschlag-Deckel greift: BG=1500, 2024 (13.6%/1020/306).
    VFB = 13.6% × 1500 ≈ 204, HB=1020 (nicht gedeckelt), Deckel = 1500−204 = 1296.
    Zuschlag 306 << Deckel → voll addiert → 204+306 = 510."""
    runner = _runner()
    assert _vfb(runner, versorgungsbezuege_bemessungsgrundlage=1500,
                versorgungsbeginn_jahr=2024) == 510


def test_zuschlag_deckel_sehr_kleine_bg():
    """BG < Zuschlag: BG=200, 2025 (13.2%/990/297).
    VFB = 200 × 13.2% ≈ 26, HB=990 >> 26 → 26 (nicht gedeckelt), Deckel = 200−26 = 174.
    Zuschlag 297 > Deckel 174 → gekürzt auf 174 → 26+174 = 200 (volle BG aufgebraucht)."""
    runner = _runner()
    assert _vfb(runner, versorgungsbezuege_bemessungsgrundlage=200,
                versorgungsbeginn_jahr=2025) == 200


def test_vfb_ohne_hoechstbetrag_deckelung():
    """BG << HB: BG=500, 2005 (40%/3000/900). VFB = 500 × 40% = 200 << HB 3000 → 200.
    Deckel = 500−200 = 300 < Zuschlag 900 → gekürzt auf 300 → 200+300 = 500."""
    runner = _runner()
    assert _vfb(runner, versorgungsbezuege_bemessungsgrundlage=500,
                versorgungsbeginn_jahr=2005) == 500


# ---- Fail-Closed Tests ----

def test_fehlende_bemessungsgrundlage():
    """Ohne BG → VersorgungsfreibetragOffen."""
    runner = _runner()
    with pytest.raises(runner.VersorgungsfreibetragOffen):
        _vfb(runner, versorgungsbezuege_bemessungsgrundlage=0,
             versorgungsbeginn_jahr=2025)


def test_fehlender_versorgungsbeginn_jahr():
    """Ohne Versorgungsbeginn-Jahr (Kohorte) → VersorgungsfreibetragOffen."""
    runner = _runner()
    with pytest.raises(runner.VersorgungsfreibetragOffen):
        _vfb(runner, versorgungsbezuege_bemessungsgrundlage=12000,
             versorgungsbeginn_jahr=0)


def test_fehlende_beide_inputs():
    """Ohne BG und Versorgungsbeginn-Jahr → VersorgungsfreibetragOffen (fail-closed)."""
    runner = _runner()
    with pytest.raises(runner.VersorgungsfreibetragOffen):
        runner.catala_p19_2_versorgungsfreibetrag({})


# ---- Arithmetik-Randfall: 1-Dezimal-Prozent ----

def test_prozentsatz_nicht_ganzzahlig_exakt():
    """Kohorte 2005 = 40.0%, 2025 = 13.2%, 2026 = 12.8% — die Multiplikation
    muss exakt sein (round(%×10)//1000, wie catala_renten_stpfl). Beispiel 2025:
    BG=10000, VFB = 13.2% × 10000 = 1320, HB=990 → 990, Zuschlag 297,
    Deckel = 10000−990 = 9010 >> 297 → 990+297 = 1287."""
    runner = _runner()
    assert _vfb(runner, versorgungsbezuege_bemessungsgrundlage=10000,
                versorgungsbeginn_jahr=2025) == 1287


def test_prozentsatz_1_dezimal_ab_2025():
    """2025 = 13.2% (1 Dezimal). 2026 = 12.8% (1 Dezimal). Verlangen volle Präzision,
    nicht Truncation. BG=36000, Jahr 2026 (12.8%/960/288):
    VFB = 36000 × 12.8% = 4608, HB=960 → 960, Deckel = 36000−960 = 35040 >> 288
    → 960+288 = 1248."""
    runner = _runner()
    assert _vfb(runner, versorgungsbezuege_bemessungsgrundlage=36000,
                versorgungsbeginn_jahr=2026) == 1248
